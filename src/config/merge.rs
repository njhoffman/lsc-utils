//! Shallow overlay of user YAML maps onto bundled defaults.
//!
//! Mirrors colorls's `lib/colorls/yaml.rb` semantics: a flat one-level merge
//! where user keys replace bundled keys and unmentioned bundled keys remain.
//! No deep merge — these YAMLs are flat string-to-string maps.

use super::YamlMap;

pub fn overlay(base: &mut YamlMap, overrides: YamlMap) {
    for (k, v) in overrides {
        base.insert(k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> YamlMap {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    #[test]
    fn overlay_replaces_existing_keys() {
        let mut base = map(&[("dir", "blue"), ("file", "yellow")]);
        overlay(&mut base, map(&[("dir", "hotpink")]));
        assert_eq!(base.get("dir").map(String::as_str), Some("hotpink"));
        assert_eq!(base.get("file").map(String::as_str), Some("yellow"));
    }

    #[test]
    fn overlay_adds_new_keys() {
        let mut base = map(&[("dir", "blue")]);
        overlay(&mut base, map(&[("git_branch", "green")]));
        assert_eq!(base.len(), 2);
        assert_eq!(base.get("git_branch").map(String::as_str), Some("green"));
    }

    #[test]
    fn empty_overrides_is_noop() {
        let mut base = map(&[("dir", "blue")]);
        let before = base.clone();
        overlay(&mut base, YamlMap::new());
        assert_eq!(base, before);
    }

    #[test]
    fn empty_base_takes_full_overlay() {
        let mut base = YamlMap::new();
        overlay(&mut base, map(&[("a", "1"), ("b", "2")]));
        assert_eq!(base.len(), 2);
    }
}
