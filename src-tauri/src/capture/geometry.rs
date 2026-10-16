use super::{CaptureError, ErrorCode, RawFrame, rgba_len};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub start: Point,
    pub end: Point,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct RectPx {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn selection_to_pixels(
    selection: Selection,
    width: u32,
    height: u32,
) -> Result<RectPx, CaptureError> {
    rgba_len(width, height)?;
    let values = [
        selection.start.x,
        selection.start.y,
        selection.end.x,
        selection.end.y,
        selection.viewport_width,
        selection.viewport_height,
    ];
    if values.iter().any(|v| !v.is_finite())
        || selection.viewport_width <= 0.0
        || selection.viewport_height <= 0.0
    {
        return Err(CaptureError::new(
            ErrorCode::InvalidSelection,
            "选区坐标或窗口尺寸无效。",
        ));
    }
    let x1 = selection.start.x.clamp(0.0, selection.viewport_width);
    let x2 = selection.end.x.clamp(0.0, selection.viewport_width);
    let y1 = selection.start.y.clamp(0.0, selection.viewport_height);
    let y2 = selection.end.y.clamp(0.0, selection.viewport_height);
    if x1 == x2 || y1 == y2 {
        return Err(CaptureError::new(
            ErrorCode::ZeroArea,
            "选区没有面积，请重新拖动。",
        ));
    }
    let sx = f64::from(width) / selection.viewport_width;
    let sy = f64::from(height) / selection.viewport_height;
    let left = (x1.min(x2) * sx).floor().clamp(0.0, f64::from(width)) as u32;
    let top = (y1.min(y2) * sy).floor().clamp(0.0, f64::from(height)) as u32;
    let right = (x1.max(x2) * sx).ceil().clamp(0.0, f64::from(width)) as u32;
    let bottom = (y1.max(y2) * sy).ceil().clamp(0.0, f64::from(height)) as u32;
    if right <= left || bottom <= top {
        return Err(CaptureError::new(
            ErrorCode::ZeroArea,
            "选区没有像素面积，请重新拖动。",
        ));
    }
    Ok(RectPx {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// Consume the frozen frame and pack the selected rows in-place. No second full frame allocation.
pub fn crop_frame(mut frame: RawFrame, rect: RectPx) -> Result<RawFrame, CaptureError> {
    let source_len = rgba_len(frame.width_px, frame.height_px)?;
    if frame.rgba.len() != source_len
        || rect.width == 0
        || rect.height == 0
        || rect
            .x
            .checked_add(rect.width)
            .is_none_or(|x| x > frame.width_px)
        || rect
            .y
            .checked_add(rect.height)
            .is_none_or(|y| y > frame.height_px)
    {
        return Err(CaptureError::new(
            ErrorCode::InvalidSelection,
            "裁剪范围超出冻结图像或像素缓冲不完整。",
        ));
    }
    let source_stride = frame.width_px as usize * 4;
    let target_stride = rect.width as usize * 4;
    for row in 0..rect.height as usize {
        let source = (rect.y as usize + row) * source_stride + rect.x as usize * 4;
        frame
            .rgba
            .copy_within(source..source + target_stride, row * target_stride);
    }
    frame.rgba.truncate(target_stride * rect.height as usize);
    frame.width_px = rect.width;
    frame.height_px = rect.height;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_scale_reverse_drag_rounding_and_edges() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let selection = Selection {
                start: Point {
                    x: 10.0 / scale,
                    y: 20.0 / scale,
                },
                end: Point {
                    x: 110.0 / scale,
                    y: 220.0 / scale,
                },
                viewport_width: 1920.0 / scale,
                viewport_height: 1080.0 / scale,
            };
            assert_eq!(
                selection_to_pixels(selection, 1920, 1080).unwrap(),
                RectPx {
                    x: 10,
                    y: 20,
                    width: 100,
                    height: 200
                }
            );
            let reversed = Selection {
                start: selection.end,
                end: selection.start,
                ..selection
            };
            assert_eq!(
                selection_to_pixels(reversed, 1920, 1080).unwrap(),
                selection_to_pixels(selection, 1920, 1080).unwrap()
            );
        }
        let rect = selection_to_pixels(
            Selection {
                start: Point { x: -3.0, y: 2.2 },
                end: Point { x: 2000.0, y: 30.1 },
                viewport_width: 800.0,
                viewport_height: 600.0,
            },
            1000,
            750,
        )
        .unwrap();
        assert_eq!(
            rect,
            RectPx {
                x: 0,
                y: 2,
                width: 1000,
                height: 36
            }
        );
    }
    #[test]
    fn rejects_zero_area_non_finite_or_invalid_viewport() {
        let selection = Selection {
            start: Point { x: 1.1, y: 2.1 },
            end: Point { x: 1.1, y: 5.0 },
            viewport_width: 800.0,
            viewport_height: 600.0,
        };
        assert_eq!(
            selection_to_pixels(selection, 800, 600).unwrap_err().code,
            ErrorCode::ZeroArea
        );
        assert_eq!(
            selection_to_pixels(
                Selection {
                    viewport_width: 0.0,
                    ..selection
                },
                800,
                600
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidSelection
        );
        assert_eq!(
            selection_to_pixels(
                Selection {
                    end: Point {
                        x: f64::NAN,
                        y: 2.0
                    },
                    ..selection
                },
                800,
                600
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidSelection
        );
        assert_eq!(
            selection_to_pixels(
                Selection {
                    start: Point { x: -10.0, y: 3.0 },
                    end: Point { x: -1.0, y: 20.0 },
                    ..selection
                },
                800,
                600
            )
            .unwrap_err()
            .code,
            ErrorCode::ZeroArea
        );
    }
    fn frame() -> RawFrame {
        RawFrame {
            width_px: 4,
            height_px: 3,
            rgba: (0..12).flat_map(|n| [n, 0, 255 - n, 255]).collect(),
        }
    }
    #[test]
    fn crop_preserves_exact_rows_and_png_dimensions() {
        let cropped = crop_frame(
            frame(),
            RectPx {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
        )
        .unwrap();
        let expected: Vec<u8> = [5, 6, 9, 10]
            .into_iter()
            .flat_map(|n| [n, 0, 255 - n, 255])
            .collect();
        assert_eq!(cropped.rgba, expected);
        let png = crate::services::screen_capture::encode_png(cropped).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buffer = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut buffer).unwrap();
        assert_eq!((info.width, info.height), (2, 2));
        assert_eq!(&buffer[..info.buffer_size()], expected);
    }
    #[test]
    fn crop_handles_full_frame_single_pixel_and_rejects_overflow() {
        assert_eq!(
            crop_frame(
                frame(),
                RectPx {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 3
                }
            )
            .unwrap()
            .rgba,
            frame().rgba
        );
        assert_eq!(
            crop_frame(
                frame(),
                RectPx {
                    x: 3,
                    y: 2,
                    width: 1,
                    height: 1
                }
            )
            .unwrap()
            .rgba,
            [11, 0, 244, 255]
        );
        for rect in [
            RectPx {
                x: 3,
                y: 0,
                width: 2,
                height: 1,
            },
            RectPx {
                x: u32::MAX,
                y: 0,
                width: 1,
                height: 1,
            },
            RectPx {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            RectPx {
                x: 0,
                y: 2,
                width: 1,
                height: 2,
            },
        ] {
            assert_eq!(
                crop_frame(frame(), rect).err().unwrap().code,
                ErrorCode::InvalidSelection
            );
        }
        let mut incomplete = frame();
        incomplete.rgba.pop();
        assert_eq!(
            crop_frame(
                incomplete,
                RectPx {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1
                }
            )
            .err()
            .unwrap()
            .code,
            ErrorCode::InvalidSelection
        );
    }
}
