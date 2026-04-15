//! Nerd-font icon resolution.
//!
//! Mirrors colorls's algorithm in `lib/colorls/core.rb#options`:
//! - Directories: lowercase basename -> `folders` map; if missing, alias chain
//!   via `folder_aliases`; final fallback `folder`.
//! - Files: extension (without dot, lowercase) -> `files` map; if missing,
//!   alias via `file_aliases`; final fallback `file`. Note this is
//!   extension-only — basename aliases like `gemfile` in `file_aliases.yaml`
//!   are dormant under colorls's logic and we preserve that for parity.

use std::collections::HashMap;

use anyhow::{anyhow, Result};

use super::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    /// Directory whose name (or alias) matched `folders`.
    Folder,
    /// Directory falling back to the generic folder glyph.
    DefaultFolder,
    /// File whose extension (or alias) matched `files`.
    File,
    /// File falling back to the generic file glyph.
    DefaultFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub glyph: char,
    pub kind: IconKind,
    /// The matched key in `files`/`folders` (or "file"/"folder" on fallback).
    /// Useful for downstream color decisions and debug output.
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct Icons {
    files: HashMap<String, char>,
    folders: HashMap<String, char>,
    file_aliases: HashMap<String, String>,
    folder_aliases: HashMap<String, String>,
    file_default: char,
    folder_default: char,
}

impl Icons {
    pub fn from_config(cfg: &Config) -> Result<Self> {
        let files = parse_glyph_map(&cfg.files, "files.yaml")?;
        let folders = parse_glyph_map(&cfg.folders, "folders.yaml")?;
        let file_default = *files
            .get("file")
            .ok_or_else(|| anyhow!("files.yaml missing required `file` default glyph"))?;
        let folder_default = *folders
            .get("folder")
            .ok_or_else(|| anyhow!("folders.yaml missing required `folder` default glyph"))?;
        Ok(Self {
            files,
            folders,
            file_aliases: cfg.file_aliases.clone().into_iter().collect(),
            folder_aliases: cfg.folder_aliases.clone().into_iter().collect(),
            file_default,
            folder_default,
        })
    }

    pub fn for_directory(&self, name: &str) -> Resolved {
        let key = name.to_lowercase();
        if let Some(&glyph) = self.folders.get(&key) {
            return Resolved {
                glyph,
                kind: IconKind::Folder,
                key,
            };
        }
        if let Some(canonical) = self.folder_aliases.get(&key) {
            if let Some(&glyph) = self.folders.get(canonical) {
                return Resolved {
                    glyph,
                    kind: IconKind::Folder,
                    key: canonical.clone(),
                };
            }
        }
        Resolved {
            glyph: self.folder_default,
            kind: IconKind::DefaultFolder,
            key: "folder".to_owned(),
        }
    }

    pub fn for_file(&self, name: &str) -> Resolved {
        let ext = file_extension_key(name);
        if let Some(&glyph) = self.files.get(&ext) {
            return Resolved {
                glyph,
                kind: IconKind::File,
                key: ext,
            };
        }
        if let Some(canonical) = self.file_aliases.get(&ext) {
            if let Some(&glyph) = self.files.get(canonical) {
                return Resolved {
                    glyph,
                    kind: IconKind::File,
                    key: canonical.clone(),
                };
            }
        }
        Resolved {
            glyph: self.file_default,
            kind: IconKind::DefaultFile,
            key: "file".to_owned(),
        }
    }
}

/// colorls uses `File.extname(name).delete_prefix('.').downcase`. Ruby's
/// `File.extname` returns the substring after the last dot when the dot is
/// neither leading nor trailing; otherwise empty. This matches Rust's
/// `Path::extension()` for normal cases.
fn file_extension_key(name: &str) -> String {
    std::path::Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default()
}

fn parse_glyph_map(map: &super::YamlMap, source: &str) -> Result<HashMap<String, char>> {
    let mut out = HashMap::with_capacity(map.len());
    for (k, v) in map {
        let mut chars = v.chars();
        let glyph = chars
            .next()
            .ok_or_else(|| anyhow!("{source}: empty glyph for `{k}`"))?;
        if chars.next().is_some() {
            return Err(anyhow!(
                "{source}: glyph for `{k}` must be a single codepoint, got `{v}`"
            ));
        }
        out.insert(k.clone(), glyph);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icons() -> Icons {
        let cfg = Config::load(None).unwrap();
        Icons::from_config(&cfg).unwrap()
    }

    #[test]
    fn directory_direct_match() {
        let i = icons();
        let r = i.for_directory(".git");
        assert_eq!(r.kind, IconKind::Folder);
        assert_eq!(r.key, ".git");
    }

    #[test]
    fn directory_alias_chain() {
        // `bin -> config` per folder_aliases.yaml.
        let i = icons();
        let r = i.for_directory("bin");
        assert_eq!(r.kind, IconKind::Folder);
        assert_eq!(r.key, "config");
        let direct = i.for_directory("config");
        assert_eq!(r.glyph, direct.glyph);
    }

    #[test]
    fn directory_unknown_falls_back() {
        let i = icons();
        let r = i.for_directory("some_random_name");
        assert_eq!(r.kind, IconKind::DefaultFolder);
        assert_eq!(r.key, "folder");
    }

    #[test]
    fn directory_match_is_case_insensitive() {
        let i = icons();
        let lo = i.for_directory(".git");
        let mixed = i.for_directory(".GIT");
        assert_eq!(lo.glyph, mixed.glyph);
    }

    #[test]
    fn file_direct_extension_match() {
        let i = icons();
        let r = i.for_file("script.py");
        assert_eq!(r.kind, IconKind::File);
        assert_eq!(r.key, "py");
    }

    #[test]
    fn file_alias_chain() {
        // `tsx -> jsx` per file_aliases.yaml.
        let i = icons();
        let r = i.for_file("Component.tsx");
        assert_eq!(r.kind, IconKind::File);
        assert_eq!(r.key, "jsx");
    }

    #[test]
    fn file_no_extension_falls_back() {
        let i = icons();
        let r = i.for_file("Gemfile");
        // colorls behavior: only extensions are matched. `Gemfile` has none,
        // so we land on the default file glyph despite the alias entry existing.
        assert_eq!(r.kind, IconKind::DefaultFile);
        assert_eq!(r.key, "file");
    }

    #[test]
    fn file_unknown_extension_falls_back() {
        let i = icons();
        let r = i.for_file("data.qwertyuiop");
        assert_eq!(r.kind, IconKind::DefaultFile);
    }

    #[test]
    fn file_extension_match_is_case_insensitive() {
        let i = icons();
        let a = i.for_file("photo.JPG");
        let b = i.for_file("photo.jpg");
        assert_eq!(a.glyph, b.glyph);
    }

    #[test]
    fn glyphs_are_single_char_or_load_fails() {
        // Sanity: ensure parse_glyph_map enforces single-codepoint.
        let mut bad = super::super::YamlMap::new();
        bad.insert("oops".into(), "ab".into());
        let err = parse_glyph_map(&bad, "test.yaml").unwrap_err();
        assert!(err.to_string().contains("single codepoint"));
    }
}
