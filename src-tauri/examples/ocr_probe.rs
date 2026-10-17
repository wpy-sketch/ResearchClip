#[cfg(target_os = "windows")]
fn main() -> windows::core::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 {
        let png = std::fs::read(&args[1]).expect("controlled test PNG");
        let header =
            serde_json::to_vec(&serde_json::json!({"language":args[2],"region":null})).unwrap();
        let mut bytes = (header.len() as u32).to_le_bytes().to_vec();
        bytes.extend(header);
        bytes.extend(png);
        match researchclip_lib::services::ocr::OcrService::recognize(&bytes) {
            Ok(result) => println!("{}", serde_json::to_string(&result).unwrap()),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    use windows::{Media::Ocr::OcrEngine, Win32::System::WinRT::*};
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }
    let result = (|| {
        let languages = OcrEngine::AvailableRecognizerLanguages()?;
        for language in languages {
            println!("{} {}", language.LanguageTag()?, language.DisplayName()?);
        }
        println!("maxDimension={}", OcrEngine::MaxImageDimension()?);
        Ok(())
    })();
    unsafe {
        RoUninitialize();
    }
    result
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows OCR probe is unavailable on this platform");
}
