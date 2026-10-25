// Offline migration/relink acceptance for an explicitly isolated test workspace.
use researchclip_lib::research::ResearchRepository as Storage;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3);
    let allowed = std::env::current_dir()
        .unwrap()
        .join("artifacts")
        .canonicalize()
        .unwrap();
    let root = std::path::PathBuf::from(&args[0]).canonicalize().unwrap();
    let selected = std::path::PathBuf::from(&args[2]).canonicalize().unwrap();
    assert!(root.starts_with(&allowed) && selected.starts_with(&allowed));
    let mut storage = Storage::open(root).unwrap();
    let (_, hash) = storage.document_location(&args[1]).unwrap();
    storage
        .relink_markdown_resource(&args[1], &hash, &selected)
        .unwrap();
    assert!(
        storage
            .document_directory()
            .unwrap()
            .documents
            .iter()
            .any(|d| d.id == args[1] && d.available)
    );
    println!("PASS: isolated stable-ID relink; original file bytes and revisions retained");
}
