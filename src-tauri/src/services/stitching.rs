//! Bounded, vertical-only stitching. No DOM, network access or raw-frame history.
use crate::capture::{CaptureError, ErrorCode, MAX_PIXELS, RawFrame, rgba_len};

pub const MAX_HEIGHT: u32 = 30_000;
pub const MAX_FRAMES: u32 = 64;
pub fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::CaptureFailed, message)
}
fn validate(frame: &RawFrame) -> Result<(), CaptureError> {
    if frame.width_px < 128
        || frame.height_px < 160
        || frame.height_px > MAX_HEIGHT
        || frame.width_px > 4096
        || u64::from(frame.width_px) * u64::from(frame.height_px) > 4_194_304
        || frame.rgba.len() != rgba_len(frame.width_px, frame.height_px)?
    {
        return Err(failure(
            "长截图选区须至少128×160，宽不超过4096且单帧不超过4,194,304像素。",
        ));
    }
    Ok(())
}

// Compare irregularly spaced samples, excluding browser scrollbars and a potential
// sticky header. A second independent validation pass guards sparse coincidences.
fn score(a: &RawFrame, b: &RawFrame, offset: u32, dense: bool) -> (f64, f64, f64) {
    let top = (a.height_px / 3).min(220);
    let bottom = a.height_px - offset;
    if bottom <= top + 32 {
        return (255.0, 0.0, 0.0);
    }
    let mut sum = 0u64;
    let mut good = 0u64;
    let mut count = 0u64;
    let mut values = [0u32; 16];
    let rows = if dense { 83 } else { 39 };
    let cols = if dense { 43 } else { 23 };
    for iy in 0..rows {
        let y = top + (iy * (bottom - top - 1) / rows);
        for ix in 0..cols {
            let x = 8 + ((ix * 977 + iy * 13) % (a.width_px - 28));
            let ia = (((y + offset) * a.width_px + x) * 4) as usize;
            let ib = ((y * b.width_px + x) * 4) as usize;
            let delta: u32 = (0..3)
                .map(|c| a.rgba[ia + c].abs_diff(b.rgba[ib + c]) as u32)
                .sum();
            sum += u64::from(delta);
            good += u64::from(delta <= 36);
            count += 1;
            values[a.rgba[ia] as usize / 16] += 1;
        }
    }
    let texture = 1.0 - f64::from(*values.iter().max().unwrap_or(&0)) / count as f64;
    (
        sum as f64 / (count * 3) as f64,
        good as f64 / count as f64,
        texture,
    )
}

pub fn frame_difference(a: &RawFrame, b: &RawFrame) -> f64 {
    if (a.width_px, a.height_px) != (b.width_px, b.height_px) {
        return 255.0;
    }
    let mut sum = 0u64;
    let mut count = 0u64;
    for y in (0..a.height_px).step_by(7) {
        for x in (4..a.width_px.saturating_sub(16)).step_by(13) {
            let i = ((y * a.width_px + x) * 4) as usize;
            for c in 0..3 {
                sum += u64::from(a.rgba[i + c].abs_diff(b.rgba[i + c]));
                count += 1;
            }
        }
    }
    sum as f64 / count.max(1) as f64
}

pub fn estimate_offset(a: &RawFrame, b: &RawFrame) -> Result<u32, CaptureError> {
    if (a.width_px, a.height_px) != (b.width_px, b.height_px) {
        return Err(failure("选区尺寸变化，无法继续拼接。"));
    }
    let max = a.height_px - (a.height_px / 3).max(96) - 34;
    let mut candidates = Vec::with_capacity(max as usize);
    for d in 1..=max {
        let (error, good, texture) = score(a, b, d, false);
        if good > 0.985 && error < 1.0 && texture > 0.015 {
            candidates.push((d, error));
        }
    }
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let Some(&(best, error)) = candidates.first() else {
        return Err(failure(
            "相邻画面重叠不足或内容变化过大。请每次只滚动约半屏，等待动画结束；可停止保留已有部分。",
        ));
    };
    if candidates
        .iter()
        .any(|(d, e)| d.abs_diff(best) > 4 && *e < error + 0.5)
    {
        return Err(failure(
            "页面重复纹理导致多个拼接位置，无法可靠判断。请缩小选区或使用更明显的正文区域。",
        ));
    }
    let (error, good, _) = score(a, b, best, true);
    if error >= 1.0 || good <= 0.985 {
        return Err(failure(
            "拼接质量校验失败，可能有悬浮工具栏、动画或动态加载。请排除遮挡并重新采集。",
        ));
    }
    Ok(best)
}

#[derive(Debug)]
pub enum Append {
    Added(u32),
    Repeated,
}
pub struct StitchingEngine {
    previous: RawFrame,
    output: Vec<u8>,
    pub height: u32,
    pub frames: u32,
    pub repeats: u32,
}
impl StitchingEngine {
    pub fn new(frame: RawFrame) -> Result<Self, CaptureError> {
        validate(&frame)?;
        Ok(Self {
            height: frame.height_px,
            output: frame.rgba.clone(),
            previous: frame,
            frames: 1,
            repeats: 0,
        })
    }
    pub fn append(&mut self, frame: RawFrame) -> Result<Append, CaptureError> {
        validate(&frame)?;
        if frame_difference(&self.previous, &frame) < 0.6 {
            self.repeats += 1;
            return Ok(Append::Repeated);
        }
        let offset = estimate_offset(&self.previous, &frame)?;
        let height = self
            .height
            .checked_add(offset)
            .ok_or_else(|| failure("长图高度溢出。"))?;
        if height > MAX_HEIGHT
            || u64::from(height) * u64::from(frame.width_px) > MAX_PIXELS
            || self.frames >= MAX_FRAMES
        {
            return Err(CaptureError::new(
                ErrorCode::ResourceLimit,
                "已达到长图上限（30,000高、16,777,216像素或64帧）。请停止保存已有部分并分段继续。",
            ));
        }
        let start = ((frame.height_px - offset) * frame.width_px * 4) as usize;
        self.output
            .try_reserve_exact(frame.rgba.len() - start)
            .map_err(|_| failure("无法分配长图内存，请停止并分段采集。"))?;
        self.output.extend_from_slice(&frame.rgba[start..]);
        self.height = height;
        self.previous = frame;
        self.frames += 1;
        self.repeats = 0;
        Ok(Append::Added(offset))
    }
    pub fn finish(self) -> RawFrame {
        RawFrame {
            width_px: self.previous.width_px,
            height_px: self.height,
            rgba: self.output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(offset: u32, header: bool) -> RawFrame {
        let (w, h) = (180, 360);
        let mut rgba = vec![0; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let yy = if header && y < 35 { y } else { y + offset };
                let mut seed = yy.wrapping_mul(928371).wrapping_add(x * 371 + 77);
                for c in 0..3 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    rgba[((y * w + x) * 4 + c) as usize] = (seed >> 24) as u8;
                }
                rgba[((y * w + x) * 4 + 3) as usize] = 255;
            }
        }
        RawFrame {
            width_px: w,
            height_px: h,
            rgba,
        }
    }
    #[test]
    fn three_and_ten_screens_keep_exact_pixels_and_fixed_header_once() {
        for count in [5, 19] {
            for header in [false, true] {
                let mut e = StitchingEngine::new(frame(0, header)).unwrap();
                for i in 1..count {
                    assert!(matches!(
                        e.append(frame(i * 180, header)).unwrap(),
                        Append::Added(180)
                    ));
                }
                let result = e.finish();
                assert_eq!(result.height_px, 360 + (count - 1) * 180);
                for y in 360..result.height_px {
                    let expected = frame(y - 359, header);
                    assert_eq!(
                        &result.rgba[(y * 180 * 4) as usize..((y + 1) * 180 * 4) as usize],
                        &expected.rgba[(359 * 180 * 4) as usize..]
                    );
                }
            }
        }
    }
    #[test]
    fn repeats_bottom_and_small_final_offset() {
        let mut e = StitchingEngine::new(frame(0, false)).unwrap();
        e.append(frame(180, false)).unwrap();
        assert!(matches!(
            e.append(frame(187, false)).unwrap(),
            Append::Added(7)
        ));
        for _ in 0..2 {
            assert!(matches!(
                e.append(frame(187, false)).unwrap(),
                Append::Repeated
            ));
        }
        assert_eq!(e.repeats, 2);
        assert_eq!(e.height, 547);
    }
    #[test]
    fn insufficient_overlap_and_dynamic_changes_preserve_previous_result() {
        let mut e = StitchingEngine::new(frame(0, false)).unwrap();
        assert!(e.append(frame(350, false)).is_err());
        assert_eq!(e.height, 360);
        assert_eq!(e.frames, 1);
        let mut dynamic = frame(180, false);
        dynamic.rgba.fill(32);
        assert!(e.append(dynamic).is_err());
        assert_eq!(e.frames, 1);
    }
    #[test]
    fn frame_and_output_budgets_are_checked() {
        assert!(
            StitchingEngine::new(RawFrame {
                width_px: 1,
                height_px: 1,
                rgba: vec![0; 4]
            })
            .is_err()
        );
        let mut e = StitchingEngine::new(frame(0, false)).unwrap();
        e.height = MAX_HEIGHT;
        e.output.clear();
        assert_eq!(
            e.append(frame(180, false)).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
    }

    #[test]
    fn stitched_png_roundtrip_preserves_size_and_all_rows() {
        let mut engine = StitchingEngine::new(frame(0, true)).unwrap();
        engine.append(frame(180, true)).unwrap();
        engine.append(frame(360, true)).unwrap();
        let result = engine.finish();
        let png = super::super::screen_capture::encode_png_ref(&result).unwrap();
        let reopened = super::super::workflow::decode_png(&png).unwrap();
        assert_eq!((reopened.width_px, reopened.height_px), (180, 720));
        assert_eq!(result.rgba, reopened.rgba);
        let mut capped = StitchingEngine::new(frame(0, false)).unwrap();
        capped.frames = MAX_FRAMES;
        assert_eq!(
            capped.append(frame(180, false)).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(capped.height, 360);
    }
}
