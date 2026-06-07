//! Versioned localStorage persistence and high-score tables, shared by all
//! games. Each game brings its own keys and (de)serializable types; a corrupt
//! or out-of-date save is silently discarded rather than crashing.

use gloo_storage::{LocalStorage, Storage};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Deserialize)]
struct Versioned<T> {
    version: u32,
    data: T,
}

#[derive(Serialize)]
struct VersionedRef<'a, T: Serialize> {
    version: u32,
    data: &'a T,
}

/// Load `key` if it exists and matches `version`; `None` otherwise.
pub fn load<T: DeserializeOwned>(key: &str, version: u32) -> Option<T> {
    LocalStorage::get::<Versioned<T>>(key)
        .ok()
        .filter(|v| v.version == version)
        .map(|v| v.data)
}

pub fn save<T: Serialize>(key: &str, version: u32, data: &T) {
    let _ = LocalStorage::set(key, &VersionedRef { version, data });
}

pub fn delete(key: &str) {
    LocalStorage::delete(key);
}

/// The stored high-score list for `key` (empty if absent/corrupt).
pub fn scores<T: DeserializeOwned>(key: &str) -> Vec<T> {
    LocalStorage::get(key).unwrap_or_default()
}

/// Insert `entry`, keep the list sorted descending by `score_of`, cap it.
pub fn record_score<T>(key: &str, entry: T, score_of: impl Fn(&T) -> i64, cap: usize)
where
    T: Serialize + DeserializeOwned,
{
    let mut list: Vec<T> = scores(key);
    list.push(entry);
    list.sort_by_key(|e| std::cmp::Reverse(score_of(e)));
    list.truncate(cap);
    let _ = LocalStorage::set(key, &list);
}
