use anyhow::Result;

pub fn handle_disable(role: &str) -> Result<()> {
    println!("Disabling {}...", role);
    Ok(())
}
