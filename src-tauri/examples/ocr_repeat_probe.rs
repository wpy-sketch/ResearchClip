fn main() {
    for i in 0..20 {
        let s = researchclip_lib::services::ocr::OcrService::status().expect("native OCR status");
        println!("status {i}: {} languages", s.languages.len());
    }
}
