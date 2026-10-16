// Test-only process fixture: copy this executable into an owned installation directory
// as researchclip.exe to exercise Restart Manager without opening real app data.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    println!("INSTALLER_GUARD_READY");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(())
}
