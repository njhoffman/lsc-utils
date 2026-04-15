//! Bundled colorls YAML data + user override loading.
//!
//! Six YAML maps live under `assets/` (copied verbatim from the Ruby colorls
//! gem; see THIRD_PARTY.md). Each is `String -> String`. At runtime we parse
//! the bundled defaults and shallow-overlay user files from
//! `~/.config/lsc-utils/<name>.yaml` if present, mirroring colorls's behavior
//! in `lib/colorls/yaml.rb`.

pub mod icons;
pub mod merge;
pub mod theme;

pub use icons::{IconKind, Icons, Resolved};
pub use theme::{ActiveTheme, ColorMode, Rgb, Style, Theme};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub type YamlMap = BTreeMap<String, String>;

const DARK_DEFAULT: &str = include_str!("../../assets/dark_colors.yaml");
const LIGHT_DEFAULT: &str = include_str!("../../assets/light_colors.yaml");
const FILES_DEFAULT: &str = include_str!("../../assets/files.yaml");
const FOLDERS_DEFAULT: &str = include_str!("../../assets/folders.yaml");
const FILE_ALIASES_DEFAULT: &str = include_str!("../../assets/file_aliases.yaml");
const FOLDER_ALIASES_DEFAULT: &str = include_str!("../../assets/folder_aliases.yaml");

#[derive(Debug, Clone)]
pub struct Config {
    pub dark_colors: YamlMap,
    pub light_colors: YamlMap,
    pub files: YamlMap,
    pub folders: YamlMap,
    pub file_aliases: YamlMap,
    pub folder_aliases: YamlMap,
}

impl Config {
    /// Load bundled defaults and overlay user YAMLs from `user_dir` if present.
    /// Pass `None` to skip user overrides entirely (used by tests + `--no-config`).
    pub fn load(user_dir: Option<&Path>) -> Result<Self> {
        Ok(Self {
            dark_colors: load_one(DARK_DEFAULT, "dark_colors.yaml", user_dir)?,
            light_colors: load_one(LIGHT_DEFAULT, "light_colors.yaml", user_dir)?,
            files: load_one(FILES_DEFAULT, "files.yaml", user_dir)?,
            folders: load_one(FOLDERS_DEFAULT, "folders.yaml", user_dir)?,
            file_aliases: load_one(FILE_ALIASES_DEFAULT, "file_aliases.yaml", user_dir)?,
            folder_aliases: load_one(FOLDER_ALIASES_DEFAULT, "folder_aliases.yaml", user_dir)?,
        })
    }

    /// Standard user config dir: `$XDG_CONFIG_HOME/lsc-utils` or `~/.config/lsc-utils`.
    pub fn default_user_dir() -> Option<PathBuf> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            let p = PathBuf::from(xdg);
            if !p.as_os_str().is_empty() {
                return Some(p.join("lsc-utils"));
            }
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("lsc-utils"))
    }
}

fn load_one(default: &str, name: &str, user_dir: Option<&Path>) -> Result<YamlMap> {
    let mut map: YamlMap =
        serde_yml::from_str(default).with_context(|| format!("parse bundled {name}"))?;
    if let Some(dir) = user_dir {
        let user_path = dir.join(name);
        if user_path.exists() {
            let body = std::fs::read_to_string(&user_path)
                .with_context(|| format!("read {}", user_path.display()))?;
            let overrides: YamlMap = serde_yml::from_str(&body)
                .with_context(|| format!("parse {}", user_path.display()))?;
            merge::overlay(&mut map, overrides);
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn bundled_defaults_load_non_empty() {
        let cfg = Config::load(None).expect("load defaults");
        assert!(!cfg.dark_colors.is_empty(), "dark_colors empty");
        assert!(!cfg.light_colors.is_empty(), "light_colors empty");
        assert!(!cfg.files.is_empty(), "files empty");
        assert!(!cfg.folders.is_empty(), "folders empty");
        assert!(!cfg.file_aliases.is_empty(), "file_aliases empty");
        assert!(!cfg.folder_aliases.is_empty(), "folder_aliases empty");
    }

    #[test]
    fn dark_theme_includes_canonical_keys() {
        let cfg = Config::load(None).unwrap();
        assert_eq!(
            cfg.dark_colors.get("dir").map(String::as_str),
            Some("dodgerblue")
        );
        assert_eq!(
            cfg.dark_colors.get("recognized_file").map(String::as_str),
            Some("yellow")
        );
        assert_eq!(
            cfg.dark_colors.get("addition").map(String::as_str),
            Some("chartreuse")
        );
    }

    #[test]
    fn user_dir_missing_is_ok() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        let cfg = Config::load(Some(&nonexistent)).expect("missing dir tolerated");
        assert!(!cfg.dark_colors.is_empty());
    }

    #[test]
    fn user_overlay_replaces_specific_keys_only() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("dark_colors.yaml"),
            "dir: hotpink\nbrand_new: tomato\n",
        )
        .unwrap();
        let cfg = Config::load(Some(tmp.path())).unwrap();
        assert_eq!(
            cfg.dark_colors.get("dir").map(String::as_str),
            Some("hotpink")
        );
        assert_eq!(
            cfg.dark_colors.get("brand_new").map(String::as_str),
            Some("tomato")
        );
        // unrelated keys preserved from bundled defaults
        assert_eq!(
            cfg.dark_colors.get("recognized_file").map(String::as_str),
            Some("yellow"),
        );
    }

    #[test]
    fn wrong_shape_user_yaml_surfaces_path_in_error() {
        // A YAML sequence is structurally valid YAML but not a String->String map,
        // so serde_yml deserialization fails. We assert the offending file path
        // appears in the anyhow error chain.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("files.yaml"), "- a\n- b\n").unwrap();
        let err = Config::load(Some(tmp.path())).unwrap_err();
        let chain: String = err
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            chain.contains("files.yaml"),
            "error chain missing path: {chain}"
        );
    }

    #[test]
    fn xdg_config_home_takes_precedence() {
        let saved_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let saved_home = std::env::var_os("HOME");
        // SAFETY: tests are not executed in parallel within this module by default
        // for env mutation; we restore both vars in finally-style.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test-xyz");
            std::env::set_var("HOME", "/tmp/home-test-xyz");
        }
        let dir = Config::default_user_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/xdg-test-xyz/lsc-utils"));

        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let dir = Config::default_user_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/home-test-xyz/.config/lsc-utils"));

        unsafe {
            match saved_xdg {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match saved_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}
