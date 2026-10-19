fn main() {
    #[cfg(debug_assertions)]
    {
        let a: Vec<String> = std::env::args().collect();
        assert_eq!(a.len(), 5);
        match researchclip_lib::qa_math::run(
            std::path::Path::new(&a[1]),
            std::path::Path::new(&a[2]),
            std::path::Path::new(&a[3]),
            std::path::Path::new(&a[4]),
        ) {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
