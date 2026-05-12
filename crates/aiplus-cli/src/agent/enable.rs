use anyhow::Result;

pub fn handle_enable(role: &str) -> Result<()> {
    println!("Enabling {}...", role);
    Ok(())
}
