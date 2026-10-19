fn main() {
    #[cfg(debug_assertions)]
    {
        let args: Vec<String> = std::env::args().collect();
        assert_eq!(args.len(), 3);
        match researchclip_lib::qa_ocr::run(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
        ) {
            Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
