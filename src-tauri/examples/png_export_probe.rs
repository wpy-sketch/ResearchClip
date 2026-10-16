// Verify an explicitly selected controlled-image export. Never log pixel contents.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("expected controlled PNG path")?,
    );
    let root = std::env::current_dir()?.join("artifacts").canonicalize()?;
    if !path.canonicalize()?.starts_with(root) {
        return Err("export probe is restricted to test artifacts".into());
    }
    let frame = researchclip_lib::services::workflow::decode_png(&std::fs::read(path)?)?;
    if frame.width_px != 320 || frame.height_px != 200 {
        return Err("unexpected fixture image dimensions".into());
    }
    for y in 0..200 {
        for x in 0..320 {
            let i = (y * 320 + x) * 4;
            if frame.rgba[i..i + 4] != [(x % 256) as u8, (y % 256) as u8, 180, 255] {
                return Err("exported PNG pixels differ from the controlled original".into());
            }
        }
    }
    println!(
        "System dialog PNG export: 320x200; all controlled RGBA pixels matched; not a screen capture"
    );
    Ok(())
}
