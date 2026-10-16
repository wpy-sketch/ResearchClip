use super::StorageError;
use crate::{
    capture::{MAX_PNG_BYTES, RawFrame},
    services::{screen_capture::encode_png, workflow::decode_png},
};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub fn id_valid(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
pub fn ordinary(path: &Path) -> Result<(), StorageError> {
    let meta = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(StorageError::Invalid("工作区存在重解析路径，已拒绝访问。"));
        }
    }
    if meta.file_type().is_symlink() {
        return Err(StorageError::Invalid("工作区存在符号链接，已拒绝访问。"));
    }
    Ok(())
}
pub fn directory(path: &Path) -> Result<(), StorageError> {
    existing_ordinary(path)?;
    fs::create_dir_all(path)?;
    ordinary(path)?;
    if !path.is_dir() {
        return Err(StorageError::Invalid("工作区子目录不可用。"));
    }
    Ok(())
}
pub fn existing_ordinary(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(_) => ordinary(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
pub fn image_path(root: &Path, relative: &str) -> Result<PathBuf, StorageError> {
    let (folder, name) = relative
        .split_once('/')
        .ok_or(StorageError::Invalid("图片引用无效。"))?;
    if !["originals", "thumbnails"].contains(&folder)
        || !name.strip_suffix(".png").is_some_and(id_valid)
    {
        return Err(StorageError::Invalid("图片引用无效。"));
    }
    ordinary(root)?;
    ordinary(&root.join(folder))?;
    if root.join(folder).canonicalize()?.parent() != Some(root) {
        return Err(StorageError::Invalid("图片引用超出工作区。"));
    }
    let path = root.join(relative);
    existing_ordinary(&path)?;
    Ok(path)
}
pub fn read(root: &Path, relative: &str) -> Result<Vec<u8>, StorageError> {
    let path = image_path(root, relative)?;
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_PNG_BYTES as u64 {
        return Err(StorageError::Invalid("图片文件超过上限或不是普通文件。"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_PNG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PNG_BYTES {
        return Err(StorageError::Invalid("图片文件超过大小上限。"));
    }
    Ok(bytes)
}
pub fn write(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), StorageError> {
    crate::services::workflow::export_png_new(&image_path(root, relative)?, bytes).map_err(|_| {
        StorageError::Invalid("图片保存失败，已有文件未覆盖；未完成保存可在重启后恢复。")
    })
}
pub fn thumbnail_frame(source: &RawFrame) -> RawFrame {
    let scale = (512.0 / source.width_px.max(source.height_px) as f64).min(1.0);
    let width = (source.width_px as f64 * scale).round().max(1.0) as u32;
    let height = (source.height_px as f64 * scale).round().max(1.0) as u32;
    let mut rgba = vec![0; width as usize * height as usize * 4];
    // Bilinear interpolation in premultiplied alpha avoids dark fringes around transparent pixels.
    for y in 0..height {
        for x in 0..width {
            let sx = ((x as f64 + 0.5) * source.width_px as f64 / width as f64 - 0.5).max(0.0);
            let sy = ((y as f64 + 0.5) * source.height_px as f64 / height as f64 - 0.5).max(0.0);
            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let fx = sx - x0 as f64;
            let fy = sy - y0 as f64;
            let mut sum = [0.0; 4];
            for (xx, yy, weight) in [
                (x0, y0, (1.0 - fx) * (1.0 - fy)),
                ((x0 + 1).min(source.width_px - 1), y0, fx * (1.0 - fy)),
                (x0, (y0 + 1).min(source.height_px - 1), (1.0 - fx) * fy),
                (
                    (x0 + 1).min(source.width_px - 1),
                    (y0 + 1).min(source.height_px - 1),
                    fx * fy,
                ),
            ] {
                let i = (yy as usize * source.width_px as usize + xx as usize) * 4;
                let alpha = source.rgba[i + 3] as f64 / 255.0;
                for (c, s) in sum.iter_mut().enumerate().take(3) {
                    *s += source.rgba[i + c] as f64 * alpha * weight;
                }
                sum[3] += alpha * weight;
            }
            let i = (y as usize * width as usize + x as usize) * 4;
            if sum[3] > 0.0 {
                for c in 0..3 {
                    rgba[i + c] = (sum[c] / sum[3]).round().clamp(0.0, 255.0) as u8;
                }
            }
            rgba[i + 3] = (sum[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    RawFrame {
        width_px: width,
        height_px: height,
        rgba,
    }
}
pub fn thumbnail(source: &[u8]) -> Result<Vec<u8>, StorageError> {
    let frame = decode_png(source).map_err(|_| StorageError::Invalid("原始PNG无法解码。"))?;
    encode_png(thumbnail_frame(&frame)).map_err(|_| StorageError::Invalid("无法生成缩略图。"))
}
