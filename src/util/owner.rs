//! Cached uid/gid -> name lookup.
//!
//! Lookups go through libc (via `uzers`) and are cached per-process so a long
//! listing of N entries doesn't hammer NSS N times. On lookup failure we fall
//! back to the numeric id rendered as a decimal string.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

static USERS: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static GROUPS: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn user_name(uid: u32) -> String {
    let mut cache = USERS.lock().expect("user cache poisoned");
    if let Some(n) = cache.get(&uid) {
        return n.clone();
    }
    let name = uzers::get_user_by_uid(uid)
        .map(|u| u.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| uid.to_string());
    cache.insert(uid, name.clone());
    name
}

pub fn group_name(gid: u32) -> String {
    let mut cache = GROUPS.lock().expect("group cache poisoned");
    if let Some(n) = cache.get(&gid) {
        return n.clone();
    }
    let name = uzers::get_group_by_gid(gid)
        .map(|g| g.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| gid.to_string());
    cache.insert(gid, name.clone());
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_user_resolves() {
        // uid 0 is root on every POSIX system; cache repeat exercises the hit path.
        let a = user_name(0);
        let b = user_name(0);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn unknown_uid_falls_back_to_decimal() {
        let n = user_name(4_000_000_000);
        assert_eq!(n, "4000000000");
    }

    #[test]
    fn root_group_resolves() {
        let a = group_name(0);
        let b = group_name(0);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn unknown_gid_falls_back_to_decimal() {
        assert_eq!(group_name(3_999_999_999), "3999999999");
    }
}
