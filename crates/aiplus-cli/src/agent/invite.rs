use crate::agent::core::is_stub;
use anyhow::Result;

pub fn handle_invite(role: &str) -> Result<()> {
    if is_stub(role) {
        return Err(anyhow::anyhow!(
            "STUB_NOT_INVITABLE: expert is v0.2 stub, not yet functional"
        ));
    }

    println!("Inviting {} to the active team...", role);
    Ok(())
}
