// Test-only failure injection. Never linked into ResearchClip; refuses paths outside this repo's artifacts.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).ok_or("expected block-save or unblock-save")?;
    let root = std::path::PathBuf::from(args.get(2).ok_or("expected isolated workspace")?)
        .canonicalize()?;
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("project root")?
        .join("artifacts")
        .canonicalize()?;
    if !root.starts_with(&artifacts) || root.file_name().is_none_or(|n| n != "workspace") {
        return Err("only an acceptance workspace is allowed".into());
    }
    let db = root.join("researchclip.sqlite3").canonicalize()?;
    if !db.starts_with(&root) {
        return Err("database outside acceptance workspace".into());
    }
    let c = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    if c.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))? != 3 {
        return Err("unexpected test schema".into());
    }
    match mode.as_str(){
        "block-save"=>c.execute_batch("CREATE TRIGGER researchclip_test_block_card BEFORE UPDATE ON screenshots BEGIN SELECT RAISE(ABORT,'acceptance failure'); END;")?,
        "unblock-save"=>c.execute_batch("DROP TRIGGER researchclip_test_block_card;")?,
        _=>return Err("unknown fixed test operation".into()),
    }
    Ok(())
}
