#[cfg(debug_assertions)]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    researchclip_lib::qa_revisions::run(std::path::PathBuf::from(&args[2]), &args[1]);
}
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("This isolated QA probe requires a debug build.");
    std::process::exit(1);
}
