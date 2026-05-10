use anyhow::{anyhow, Context, Result};

include!(concat!(env!("OUT_DIR"), "/asset_files.rs"));

pub fn embedded_asset_bytes(rel: &str) -> Result<&'static [u8]> {
    ASSET_FILES
        .iter()
        .find_map(|(path, bytes)| (*path == rel).then_some(*bytes))
        .ok_or_else(|| anyhow!("missing embedded asset: {rel}"))
}

pub fn embedded_asset_text(rel: &str) -> Result<String> {
    let bytes = embedded_asset_bytes(rel)?;
    String::from_utf8(bytes.to_vec()).with_context(|| format!("decode embedded asset {rel}"))
}

pub fn embedded_asset_paths() -> impl Iterator<Item = &'static str> {
    ASSET_FILES.iter().map(|(path, _)| *path)
}

pub fn embedded_files_with_prefix(
    prefix: &str,
) -> impl Iterator<Item = (&'static str, &'static [u8])> + '_ {
    ASSET_FILES
        .iter()
        .filter_map(move |(rel, bytes)| rel.strip_prefix(prefix).map(|stripped| (stripped, *bytes)))
}
