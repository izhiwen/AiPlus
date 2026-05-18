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
                || rel == "aiplus-compact-reminder/package.json"
                || rel.starts_with("aiplus-compact-reminder/tests/")
            {
                continue;
            }
            // Silently skip binary blobs (images, video, archives, transient
            // captures). They have no business in an `include_bytes!`-style
            // embed and would bloat the binary. Unlike private-asset patterns
            // below, these are allowed in the assets tree (e.g. a public
            // module ships a demo.gif for its README); they just don't get
            // baked into aiplus-core.
            if is_skip_embed_asset(&rel) {
                continue;
            }
            enforce_public_asset_policy(&rel);
            files.push((rel, path));
        }
    }
    Ok(())
}

fn is_skip_embed_asset(rel: &str) -> bool {
    // File extensions that are public-allowed in the assets tree but should
    // never be embedded in the binary. Match by suffix only (so a file named
    // `foo.gif.txt` is NOT skipped — it would still hit the policy check).
    const SKIP_EXTS: &[&str] = &[
        ".gif", ".png", ".jpg", ".jpeg", ".mp4", ".mov", ".m4a", ".wav",
        ".zip", ".tar", ".tgz", ".har", ".webrtcdump", ".log",
    ];
    SKIP_EXTS.iter().any(|ext| rel.ends_with(ext))
}

fn enforce_public_asset_policy(rel: &str) {
    // Truly-private patterns: panic if any of these slip into the assets tree.
    // Binary-blob extensions are handled separately by `is_skip_embed_asset`
    // and never reach this function.
    let forbidden = [
        "aiplus-work-with-zhiwen",
        "work-with-zhiwen",
        "AGENTS.profile.md",
        "profile.toml",
        "secret-aliases.tsv",
        ".codex/compact/checkpoints",
    ];
    if forbidden.iter().any(|needle| rel.contains(needle)) {
        panic!("refusing to embed private or generated asset: {rel}");
    }
}
