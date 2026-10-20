//! Deterministic original-image features. Distances are not probabilities.
use crate::{capture::RawFrame, storage::StorageError};
use sha2::{Digest, Sha256};
pub const ALGORITHM_VERSION: i64 = 1;
pub struct Features {
    pub width_px: u32,
    pub height_px: u32,
    pub content_hash: String,
    pub perceptual_hash: String,
    pub difference_hash: String,
    pub descriptor: Vec<u8>,
}
fn rgb(frame: &RawFrame, x: u32, y: u32) -> [u8; 3] {
    let i = (y as usize * frame.width_px as usize + x as usize) * 4;
    let a = frame.rgba[i + 3] as u32;
    std::array::from_fn(|c| ((frame.rgba[i + c] as u32 * a + 255 * (255 - a) + 127) / 255) as u8)
}
// Area averaging over the complete image, with alpha composited on white.
fn resize(frame: &RawFrame, w: usize, h: usize) -> Vec<[u8; 3]> {
    (0..w * h)
        .map(|i| {
            let x = i % w;
            let y = i / w;
            let x0 = x as u32 * frame.width_px / w as u32;
            let x1 = (((x + 1) as u32 * frame.width_px) / w as u32)
                .max(x0 + 1)
                .min(frame.width_px);
            let y0 = y as u32 * frame.height_px / h as u32;
            let y1 = (((y + 1) as u32 * frame.height_px) / h as u32)
                .max(y0 + 1)
                .min(frame.height_px);
            let mut sum = [0u64; 3];
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let p = rgb(frame, xx, yy);
                    for c in 0..3 {
                        sum[c] += p[c] as u64;
                    }
                }
            }
            let n = ((x1 - x0) * (y1 - y0)) as u64;
            std::array::from_fn(|c| (sum[c] / n) as u8)
        })
        .collect()
}
fn gray(rgb: [u8; 3]) -> f64 {
    (299. * rgb[0] as f64 + 587. * rgb[1] as f64 + 114. * rgb[2] as f64) / 1000.
}
pub fn compute(png: &[u8]) -> Result<Features, StorageError> {
    let original = super::workflow::decode_png(png)
        .map_err(|_| StorageError::Invalid("原图损坏或尺寸超限；未删除素材。"))?;
    let (width_px, height_px) = (original.width_px, original.height_px);
    let frame = crate::storage::files::thumbnail_frame(&original);
    drop(original);
    let small = resize(&frame, 32, 32);
    let mut coefficients = [0.; 64];
    for v in 0..8 {
        for u in 0..8 {
            let mut sum = 0.;
            for y in 0..32 {
                for x in 0..32 {
                    sum += gray(small[y * 32 + x])
                        * ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 64.).cos()
                        * ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / 64.).cos();
                }
            }
            coefficients[v * 8 + u] = sum;
        }
    }
    let mut ac = coefficients[1..].to_vec();
    ac.sort_by(f64::total_cmp);
    let median = ac[31];
    let mut p = 0u64;
    for (i, c) in coefficients.iter().enumerate().skip(1) {
        if *c > median + 1e-7 {
            p |= 1 << i;
        }
    }
    let differences = resize(&frame, 9, 8);
    let mut d = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            if gray(differences[y * 9 + x]) > gray(differences[y * 9 + x + 1]) {
                d |= 1 << (y * 8 + x);
            }
        }
    }
    Ok(Features {
        width_px,
        height_px,
        content_hash: format!("{:x}", Sha256::digest(png)),
        perceptual_hash: format!("{p:016x}"),
        difference_hash: format!("{d:016x}"),
        descriptor: resize(&frame, 16, 16).into_iter().flatten().collect(),
    })
}
pub fn distance(a: &str, b: &str) -> Result<u32, StorageError> {
    let parse = |s: &str| {
        if s.len() == 16 {
            u64::from_str_radix(s, 16).ok()
        } else {
            None
        }
    };
    Ok(
        (parse(a).ok_or(StorageError::Invalid("特征索引损坏，请重建索引。"))?
            ^ parse(b).ok_or(StorageError::Invalid("特征索引损坏，请重建索引。"))?)
        .count_ones(),
    )
}
pub fn color_distance(a: &[u8], b: &[u8]) -> Result<u32, StorageError> {
    if a.len() != 768 || b.len() != 768 {
        return Err(StorageError::Invalid("颜色特征损坏，请重建索引。"));
    }
    Ok(a.iter()
        .zip(b)
        .map(|(a, b)| a.abs_diff(*b) as u32)
        .sum::<u32>()
        / 768)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::screen_capture::encode_png;
    #[test]
    fn jpeg_quality_changes_keep_visual_family_and_unrelated_images_are_filtered() {
        let original = png(240, 160, false);
        let image = image::load_from_memory(&original).unwrap().to_rgb8();
        let mut low = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut low, 45)
            .encode(image.as_raw(), 240, 160, image::ExtendedColorType::Rgb8)
            .unwrap();
        let decoded = image::load_from_memory(&low).unwrap().to_rgba8();
        let encoded = encode_png(RawFrame {
            width_px: 240,
            height_px: 160,
            rgba: decoded.into_raw(),
        })
        .unwrap();
        let a = compute(&original).unwrap();
        let b = compute(&encoded).unwrap();
        assert_ne!(a.content_hash, b.content_hash);
        assert!(distance(&a.perceptual_hash, &b.perceptual_hash).unwrap() <= 16);
        assert!(color_distance(&a.descriptor, &b.descriptor).unwrap() <= 15);
        let different = compute(
            &encode_png(RawFrame {
                width_px: 240,
                height_px: 160,
                rgba: [0, 240, 40, 255].repeat(240 * 160),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(
            distance(&a.perceptual_hash, &different.perceptual_hash).unwrap() > 10
                || color_distance(&a.descriptor, &different.descriptor).unwrap() > 35
        );
    }
    fn png(w: u32, h: u32, change: bool) -> Vec<u8> {
        let mut rgba = vec![];
        for y in 0..h {
            for x in 0..w {
                let c = if x * 2 < w && y * 3 < h {
                    [30, 90, 180]
                } else if x * 3 > w * 2 {
                    [180, 40, 60]
                } else {
                    [240, 245, 250]
                };
                rgba.extend_from_slice(&[
                    c[0],
                    c[1],
                    if change && x < w / 10 { 10 } else { c[2] },
                    255,
                ]);
            }
        }
        encode_png(RawFrame {
            width_px: w,
            height_px: h,
            rgba,
        })
        .unwrap()
    }
    #[test]
    fn exact_scaled_local_change_and_corruption() {
        let a = compute(&png(300, 210, false)).unwrap();
        let b = compute(&png(150, 105, false)).unwrap();
        assert_ne!(a.content_hash, b.content_hash);
        assert!(distance(&a.perceptual_hash, &b.perceptual_hash).unwrap() <= 10);
        assert!(color_distance(&a.descriptor, &b.descriptor).unwrap() <= 5);
        let c = compute(&png(300, 210, true)).unwrap();
        assert_ne!(a.content_hash, c.content_hash);
        assert!(distance(&a.perceptual_hash, &c.perceptual_hash).unwrap() <= 16);
        assert!(compute(b"broken").is_err());
        assert!(distance("x", &a.perceptual_hash).is_err());
        assert!(color_distance(&[], &[]).is_err());
    }
    #[test]
    fn alpha_and_solid_colors_do_not_make_probabilities() {
        let frame = |rgba| {
            encode_png(RawFrame {
                width_px: 4,
                height_px: 4,
                rgba,
            })
            .unwrap()
        };
        let white = compute(&frame([255, 255, 255, 255].repeat(16))).unwrap();
        let clear = compute(&frame([0, 0, 0, 0].repeat(16))).unwrap();
        let black = compute(&frame([0, 0, 0, 255].repeat(16))).unwrap();
        assert_eq!(white.descriptor, clear.descriptor);
        assert_ne!(white.content_hash, clear.content_hash);
        assert_eq!(
            color_distance(&white.descriptor, &black.descriptor).unwrap(),
            255
        );
    }
}
