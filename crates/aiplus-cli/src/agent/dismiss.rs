use anyhow::Result;

pub fn handle_dismiss(role: &str) -> Result<()> {
    println!("Dismissing {} from the active team...", role);
    Ok(())
}
