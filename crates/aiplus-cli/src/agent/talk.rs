use anyhow::Result;

pub fn handle_talk(role: &str) -> Result<()> {
    println!("Starting conversation with {}...", role);
    Ok(())
}
