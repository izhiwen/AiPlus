// TODO(v0.2): cross-platform binary alias substitution. Used by audit
// when a fixture references e.g. `stata` but the host machine has it
// under a different name. Not wired up yet — kept as scaffolding.
// Suppress dead_code at module level to keep `cargo build` warning-free.
#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
}

pub fn current_platform() -> Platform {
    match std::env::consts::OS {
        "linux" => Platform::Linux,
        "macos" => Platform::MacOS,
        _ => Platform::Linux,
    }
}

#[derive(Debug, Clone)]
pub struct PlatformCommand {
    pub linux: String,
    pub macos: String,
    pub fallback_macos: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BinAliases {
    aliases: HashMap<String, PlatformCommand>,
}

impl BinAliases {
    pub fn new(aliases: HashMap<String, PlatformCommand>) -> Self {
        Self { aliases }
    }

    pub fn from_schema(schema: HashMap<String, aiplus_core::agent_team::BinAlias>) -> Self {
        let aliases = schema
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    PlatformCommand {
                        linux: v.linux,
                        macos: v.macos,
                        fallback_macos: v.fallback_macos,
                    },
                )
            })
            .collect();
        Self { aliases }
    }

    pub fn resolve(&self, alias: &str, platform: Platform) -> Result<Vec<String>> {
        let cmd = self
            .aliases
            .get(alias)
            .ok_or_else(|| anyhow!("Unknown bin alias: {}", alias))?;

        let cmd_str = match platform {
            Platform::Linux => &cmd.linux,
            Platform::MacOS => {
                let tokens = shell_words::split(&cmd.macos).map_err(|e| {
                    anyhow!("Failed to tokenize macos command for '{}': {}", alias, e)
                })?;
                if let Some(program) = tokens.first() {
                    if !command_exists(program) {
                        if let Some(ref fallback) = cmd.fallback_macos {
                            return shell_words::split(fallback).map_err(|e| {
                                anyhow!(
                                    "Failed to tokenize fallback command for '{}': {}",
                                    alias,
                                    e
                                )
                            });
                        }
                    }
                }
                &cmd.macos
            }
        };

        shell_words::split(cmd_str)
            .map_err(|e| anyhow!("Failed to tokenize alias '{}': {}", alias, e))
    }

    pub fn substitute(&self, cmd: &str) -> Result<String> {
        substitute_in_command(cmd, self)
    }
}

pub fn substitute_in_command(cmd: &str, aliases: &BinAliases) -> Result<String> {
    let mut result = String::new();
    let mut chars = cmd.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut alias_name = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                alias_name.push(next_ch);
                chars.next();
            }

            if !alias_name.is_empty() {
                let resolved = aliases.resolve(&alias_name, current_platform())?;
                result.push_str(&shell_words::join(&resolved));
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn command_exists(name: &str) -> bool {
    if name.contains('/') {
        std::path::Path::new(name).exists()
    } else {
        std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|path| {
                let full = path.join(name);
                full.exists()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_aliases() -> BinAliases {
        let mut map = HashMap::new();
        map.insert(
            "sha256".to_string(),
            PlatformCommand {
                linux: "sha256sum".to_string(),
                macos: "shasum -a 256".to_string(),
                fallback_macos: None,
            },
        );
        map.insert(
            "md5".to_string(),
            PlatformCommand {
                linux: "md5sum".to_string(),
                macos: "md5".to_string(),
                fallback_macos: None,
            },
        );
        map.insert(
            "timeout".to_string(),
            PlatformCommand {
                linux: "timeout 5".to_string(),
                macos: "gtimeout 5".to_string(),
                fallback_macos: Some("/bin/sleep 5".to_string()),
            },
        );
        BinAliases::new(map)
    }

    #[test]
    fn test_resolve_linux() {
        let aliases = test_aliases();
        let args = aliases.resolve("sha256", Platform::Linux).unwrap();
        assert_eq!(args, vec!["sha256sum"]);
    }

    #[test]
    fn test_resolve_macos() {
        let aliases = test_aliases();
        let args = aliases.resolve("sha256", Platform::MacOS).unwrap();
        assert_eq!(args, vec!["shasum", "-a", "256"]);
    }

    #[test]
    fn test_resolve_unknown_alias() {
        let aliases = test_aliases();
        let result = aliases.resolve("unknown", Platform::Linux);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown bin alias"));
    }

    #[test]
    fn test_substitute_single_alias() {
        let aliases = test_aliases();
        let cmd = aliases.substitute("{sha256} file.txt").unwrap();
        if current_platform() == Platform::MacOS {
            assert_eq!(cmd, "shasum -a 256 file.txt");
        } else {
            assert_eq!(cmd, "sha256sum file.txt");
        }
    }

    #[test]
    fn test_substitute_spaces_in_path() {
        let aliases = test_aliases();
        let cmd = aliases
            .substitute("{sha256} \"name with spaces.txt\"")
            .unwrap();
        if current_platform() == Platform::MacOS {
            assert_eq!(cmd, "shasum -a 256 \"name with spaces.txt\"");
        } else {
            assert_eq!(cmd, "sha256sum \"name with spaces.txt\"");
        }
    }

    #[test]
    fn test_substitute_multiple_aliases() {
        let aliases = test_aliases();
        let cmd = aliases
            .substitute("{sha256} file.txt && {md5} file.txt")
            .unwrap();
        if current_platform() == Platform::MacOS {
            assert_eq!(cmd, "shasum -a 256 file.txt && md5 file.txt");
        } else {
            assert_eq!(cmd, "sha256sum file.txt && md5sum file.txt");
        }
    }

    #[test]
    fn test_fallback_when_command_absent() {
        let mut map = HashMap::new();
        map.insert(
            "nonexistent".to_string(),
            PlatformCommand {
                linux: "echo linux".to_string(),
                macos: "__definitely_missing_cmd__ arg".to_string(),
                fallback_macos: Some("echo fallback".to_string()),
            },
        );
        let aliases = BinAliases::new(map);

        // On macOS, the missing command should trigger fallback
        let args = aliases.resolve("nonexistent", Platform::MacOS).unwrap();
        assert_eq!(args, vec!["echo", "fallback"]);

        // On Linux, it should use linux variant regardless
        let args = aliases.resolve("nonexistent", Platform::Linux).unwrap();
        assert_eq!(args, vec!["echo", "linux"]);
    }

    #[test]
    fn test_fallback_not_used_when_command_exists() {
        let mut map = HashMap::new();
        map.insert(
            "existing".to_string(),
            PlatformCommand {
                linux: "echo linux".to_string(),
                macos: "echo primary".to_string(),
                fallback_macos: Some("echo fallback".to_string()),
            },
        );
        let aliases = BinAliases::new(map);

        // "echo" definitely exists on all platforms
        let args = aliases.resolve("existing", Platform::MacOS).unwrap();
        assert_eq!(args, vec!["echo", "primary"]);
    }

    #[test]
    fn test_from_schema() {
        use aiplus_core::agent_team::BinAlias;
        let mut schema = HashMap::new();
        schema.insert(
            "stat_size".to_string(),
            BinAlias {
                linux: "stat -c %s".to_string(),
                macos: "stat -f %z".to_string(),
                fallback_macos: None,
            },
        );
        let aliases = BinAliases::from_schema(schema);
        let args = aliases.resolve("stat_size", Platform::Linux).unwrap();
        assert_eq!(args, vec!["stat", "-c", "%s"]);
    }

    #[test]
    fn test_current_platform() {
        let platform = current_platform();
        if cfg!(target_os = "macos") {
            assert_eq!(platform, Platform::MacOS);
        } else if cfg!(target_os = "linux") {
            assert_eq!(platform, Platform::Linux);
        }
    }
}
