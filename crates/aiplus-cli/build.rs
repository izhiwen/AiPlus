use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let asset_root = manifest_dir.join("../../assets").canonicalize().unwrap();
    println!("cargo:rerun-if-changed={}", asset_root.display());

    let mut files = Vec::new();
    collect_files(&asset_root, &asset_root, &mut files).unwrap();
    files.sort();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut output = fs::File::create(out_dir.join("asset_files.rs")).unwrap();
    writeln!(output, "static ASSET_FILES: &[(&str, &[u8])] = &[").unwrap();
    for (rel, abs) in files {
        writeln!(
            output,
            "    ({rel:?}, include_bytes!({abs:?}) as &[u8]),",
            rel = rel,
            abs = abs.display().to_string()
        )
        .unwrap();
    }
    writeln!(output, "];").unwrap();
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some(".git" | "node_modules" | "target" | ".DS_Store")
        ) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.file_name().and_then(|n| n.to_str()) == Some("compactctl.mjs")
                || rel == "aiplus-auto-compact/package.json"
                || rel.starts_with("aiplus-auto-compact/tests/")
            {
                continue;
            }
            enforce_public_asset_policy(&rel);
            files.push((rel, path));
        }
    }
    Ok(())
}

fn enforce_public_asset_policy(rel: &str) {
    let forbidden = [
        "aiplus-work-with-zhiwen",
        "work-with-zhiwen",
        "AGENTS.profile.md",
        "profile.toml",
        "secret-aliases.tsv",
        ".codex/compact/checkpoints",
        ".har",
        ".webrtcdump",
        ".png",
        ".jpg",
        ".jpeg",
        ".gif",
        ".log",
        ".mp4",
        ".mov",
        ".m4a",
        ".wav",
        ".zip",
        ".tar",
        ".tgz",
    ];
    if forbidden.iter().any(|needle| rel.contains(needle)) {
        panic!("refusing to embed private or generated asset: {rel}");
    }
}
