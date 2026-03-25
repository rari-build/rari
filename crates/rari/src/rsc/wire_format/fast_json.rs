use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde::Serialize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{SystemTime, UNIX_EPOCH};

static EMPTY_PROPS_JSON: LazyLock<Arc<String>> = LazyLock::new(|| Arc::new("{}".to_string()));

type CacheValue = (Arc<String>, FxHashMap<String, Value>, u64);

pub struct FastJson {
    prop_cache: Arc<DashMap<u64, CacheValue>>,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl Default for FastJson {
    fn default() -> Self {
        Self::new()
    }
}

impl FastJson {
    pub fn new() -> Self {
        Self {
            prop_cache: Arc::new(DashMap::with_capacity(256)),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn to_string<T: Serialize>(value: &T) -> Result<String, sonic_rs::Error> {
        sonic_rs::to_string(value)
    }

    #[inline]
    pub fn to_string_or<T: Serialize + ?Sized>(value: &T, fallback: &str) -> String {
        sonic_rs::to_string(value).unwrap_or_else(|_| fallback.to_string())
    }

    #[inline]
    fn current_timestamp() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    pub fn serialize_props_cached(&self, props: &FxHashMap<String, Value>) -> Arc<String> {
        if props.is_empty() {
            return Arc::clone(&EMPTY_PROPS_JSON);
        }

        let hash = Self::compute_props_hash(props);
        let now = Self::current_timestamp();

        let entry = self.prop_cache.entry(hash);

        match entry {
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let (cached_json, cached_props, _) = occupied.get();

                if Self::props_equal(props, cached_props) {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    let result = Arc::clone(cached_json);
                    occupied.get_mut().2 = now;
                    result
                } else {
                    drop(occupied);

                    let secondary_hash = hash.wrapping_mul(0x9e3779b97f4a7c15);
                    if let Some(mut entry) = self.prop_cache.get_mut(&secondary_hash) {
                        let (cached_json, cached_props, timestamp) = entry.value_mut();
                        if Self::props_equal(props, cached_props) {
                            self.cache_hits.fetch_add(1, Ordering::Relaxed);
                            *timestamp = now;
                            return Arc::clone(cached_json);
                        }
                    }

                    self.cache_misses.fetch_add(1, Ordering::Relaxed);

                    let mut buf = Vec::new();
                    let mut serializer = sonic_rs::Serializer::new(&mut buf).sort_map_keys();
                    let serialized = if serde::Serialize::serialize(props, &mut serializer).is_ok()
                    {
                        String::from_utf8(buf).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        "{}".to_string()
                    };
                    let arc_string = Arc::new(serialized);

                    if let dashmap::mapref::entry::Entry::Vacant(vacant) =
                        self.prop_cache.entry(secondary_hash)
                    {
                        vacant.insert((Arc::clone(&arc_string), props.clone(), now));
                        if self.prop_cache.len() > 10000 {
                            self.evict_cache_entries();
                        }
                    }

                    arc_string
                }
            }
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                self.cache_misses.fetch_add(1, Ordering::Relaxed);

                let mut buf = Vec::new();
                let mut serializer = sonic_rs::Serializer::new(&mut buf).sort_map_keys();
                let serialized = if serde::Serialize::serialize(props, &mut serializer).is_ok() {
                    String::from_utf8(buf).unwrap_or_else(|_| "{}".to_string())
                } else {
                    "{}".to_string()
                };
                let arc_string = Arc::new(serialized);

                vacant.insert((Arc::clone(&arc_string), props.clone(), now));

                if self.prop_cache.len() > 10000 {
                    self.evict_cache_entries();
                }

                arc_string
            }
        }
    }

    fn props_equal(a: &FxHashMap<String, Value>, b: &FxHashMap<String, Value>) -> bool {
        if a.len() != b.len() {
            return false;
        }

        a.iter().all(|(k, v)| b.get(k) == Some(v))
    }

    fn compute_props_hash(props: &FxHashMap<String, Value>) -> u64 {
        let mut hasher = DefaultHasher::new();

        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort_unstable();

        for key in keys {
            key.hash(&mut hasher);
            Self::hash_json_value(&props[key], &mut hasher, 64);
        }

        hasher.finish()
    }

    fn hash_json_value<H: Hasher>(value: &Value, hasher: &mut H, max_depth: u32) {
        if max_depth == 0 {
            255u8.hash(hasher);
            return;
        }

        match value {
            Value::Null => 0u8.hash(hasher),
            Value::Bool(b) => {
                1u8.hash(hasher);
                b.hash(hasher);
            }
            Value::Number(n) => {
                2u8.hash(hasher);
                n.to_string().hash(hasher);
            }
            Value::String(s) => {
                3u8.hash(hasher);
                s.hash(hasher);
            }
            Value::Array(arr) => {
                4u8.hash(hasher);
                arr.len().hash(hasher);
                for item in arr {
                    Self::hash_json_value(item, hasher, max_depth - 1);
                }
            }
            Value::Object(obj) => {
                5u8.hash(hasher);
                obj.len().hash(hasher);

                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort_unstable();

                for key in keys {
                    key.hash(hasher);
                    Self::hash_json_value(&obj[key], hasher, max_depth - 1);
                }
            }
        }
    }

    fn evict_cache_entries(&self) {
        let target_size = 8_000;
        let current_size = self.prop_cache.len();

        if current_size <= target_size {
            return;
        }

        let to_remove = current_size - target_size;

        let mut entries: Vec<(u64, u64)> =
            self.prop_cache.iter().map(|entry| (*entry.key(), entry.value().2)).collect();

        entries.sort_by_key(|(_, timestamp)| *timestamp);

        for (key, _) in entries.iter().take(to_remove) {
            self.prop_cache.remove(key);
        }
    }

    pub fn clear_cache(&self) {
        self.prop_cache.clear();
    }

    pub fn cache_stats(&self) -> (u64, u64, f64) {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 { (hits as f64 / total as f64) * 100.0 } else { 0.0 };
        (hits, misses, hit_rate)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_serialization() {
        let value = serde_json::json!({"key": "value", "number": 42});
        let result = FastJson::to_string(&value).expect("serialization should succeed");
        assert!(result.contains("key"));
        assert!(result.contains("value"));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_serialization_with_fallback() {
        let value = serde_json::json!({"test": "data"});
        let result = FastJson::to_string_or(&value, "{}");
        assert!(result.contains("test"));
    }

    #[test]
    fn test_prop_caching() {
        let serializer = FastJson::new();

        let mut props = FxHashMap::default();
        props.insert("name".to_string(), Value::String("test".to_string()));
        props.insert("count".to_string(), Value::Number(42.into()));

        let result1 = serializer.serialize_props_cached(&props);
        let (hits, misses, _) = serializer.cache_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);

        let result2 = serializer.serialize_props_cached(&props);
        let (hits, misses, _) = serializer.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);

        assert_eq!(*result1, *result2);
    }

    #[test]
    fn test_empty_props() {
        let serializer = FastJson::new();
        let props = FxHashMap::default();
        let result = serializer.serialize_props_cached(&props);
        assert_eq!(*result, "{}");
    }

    #[test]
    fn test_consistent_hashing() {
        let serializer = FastJson::new();

        let mut props1 = FxHashMap::default();
        props1.insert("a".to_string(), Value::String("1".to_string()));
        props1.insert("b".to_string(), Value::String("2".to_string()));

        let mut props2 = FxHashMap::default();
        props2.insert("b".to_string(), Value::String("2".to_string()));
        props2.insert("a".to_string(), Value::String("1".to_string()));

        let result1 = serializer.serialize_props_cached(&props1);
        let result2 = serializer.serialize_props_cached(&props2);

        let (hits, misses, _) = serializer.cache_stats();
        assert_eq!(misses, 1);
        assert_eq!(hits, 1);
        assert_eq!(*result1, *result2);
    }

    #[test]
    fn test_clear_cache() {
        let serializer = FastJson::new();

        let mut props = FxHashMap::default();
        props.insert("key".to_string(), Value::String("value".to_string()));

        serializer.serialize_props_cached(&props);
        serializer.clear_cache();

        serializer.serialize_props_cached(&props);
        let (_, misses, _) = serializer.cache_stats();
        assert_eq!(misses, 2);
    }

    #[test]
    fn test_forced_hash_collision() {
        let serializer = FastJson::new();

        let mut original_props = FxHashMap::default();
        original_props.insert("key1".to_string(), Value::String("value1".to_string()));

        let hash = FastJson::compute_props_hash(&original_props);

        let mut fake_props = FxHashMap::default();
        fake_props.insert("key2".to_string(), Value::String("fake_value".to_string()));

        let fake_json = Arc::new(r#"{"key2":"fake_value"}"#.to_string());
        serializer.prop_cache.insert(hash, (fake_json, fake_props, 0));

        let result = serializer.serialize_props_cached(&original_props);

        assert!(result.contains("value1"), "Should contain original value1, not fake_value");
        assert!(!result.contains("fake_value"), "Should not contain the fake cached value");
        assert!(result.contains("key1"), "Should contain original key1");

        let (hits, misses, _) = serializer.cache_stats();
        assert_eq!(hits, 0, "Should have 0 cache hits due to collision");
        assert_eq!(misses, 1, "Should have 1 cache miss due to collision detection");
    }

    #[test]
    fn test_props_equality_check() {
        let mut props1 = FxHashMap::default();
        props1.insert("key1".to_string(), Value::String("value1".to_string()));
        props1.insert("key2".to_string(), Value::Number(42.into()));

        let mut props2 = FxHashMap::default();
        props2.insert("key1".to_string(), Value::String("value1".to_string()));
        props2.insert("key2".to_string(), Value::Number(42.into()));

        let mut props3 = FxHashMap::default();
        props3.insert("key1".to_string(), Value::String("different".to_string()));
        props3.insert("key2".to_string(), Value::Number(42.into()));

        assert!(FastJson::props_equal(&props1, &props2));
        assert!(!FastJson::props_equal(&props1, &props3));
    }

    #[test]
    fn test_caching_multiple_distinct_props() {
        let serializer = FastJson::new();

        let mut props1 = FxHashMap::default();
        props1.insert("key1".to_string(), Value::String("value1".to_string()));

        let mut props2 = FxHashMap::default();
        props2.insert("key2".to_string(), Value::String("value2".to_string()));

        let result1 = serializer.serialize_props_cached(&props1);
        assert!(result1.contains("value1"));

        let result2 = serializer.serialize_props_cached(&props2);
        assert!(result2.contains("value2"));

        let result1_again = serializer.serialize_props_cached(&props1);
        assert_eq!(*result1, *result1_again);

        let result2_again = serializer.serialize_props_cached(&props2);
        assert_eq!(*result2, *result2_again);

        let (hits, misses, _) = serializer.cache_stats();
        assert_eq!(misses, 2, "Should have 2 cache misses (one for each unique props)");
        assert_eq!(hits, 2, "Should have 2 cache hits (one for each repeated call)");
    }

    #[test]
    fn test_deeply_nested_json_depth_limit() {
        let serializer = FastJson::new();

        let mut deeply_nested = Value::String("deep".to_string());
        for _ in 0..70 {
            let mut obj = serde_json::Map::new();
            obj.insert("nested".to_string(), deeply_nested);
            deeply_nested = Value::Object(obj);
        }

        let mut props = FxHashMap::default();
        props.insert("data".to_string(), deeply_nested);

        let result = serializer.serialize_props_cached(&props);
        assert!(!result.is_empty(), "Should serialize without stack overflow");

        let result2 = serializer.serialize_props_cached(&props);
        assert_eq!(*result, *result2, "Should produce consistent results");
    }

    #[test]
    fn test_depth_limit_affects_hash() {
        let mut shallow = Value::String("value".to_string());
        for _ in 0..60 {
            let mut obj = serde_json::Map::new();
            obj.insert("nested".to_string(), shallow);
            shallow = Value::Object(obj);
        }

        let mut deep1 = Value::String("value1".to_string());
        for _ in 0..70 {
            let mut obj = serde_json::Map::new();
            obj.insert("nested".to_string(), deep1);
            deep1 = Value::Object(obj);
        }

        let mut deep2 = Value::String("value2".to_string());
        for _ in 0..70 {
            let mut obj = serde_json::Map::new();
            obj.insert("nested".to_string(), deep2);
            deep2 = Value::Object(obj);
        }

        let mut props_shallow = FxHashMap::default();
        props_shallow.insert("data".to_string(), shallow);

        let mut props_deep1 = FxHashMap::default();
        props_deep1.insert("data".to_string(), deep1);

        let mut props_deep2 = FxHashMap::default();
        props_deep2.insert("data".to_string(), deep2);

        let hash_shallow = FastJson::compute_props_hash(&props_shallow);
        let hash_deep1 = FastJson::compute_props_hash(&props_deep1);
        let hash_deep2 = FastJson::compute_props_hash(&props_deep2);

        assert_ne!(hash_shallow, hash_deep1, "Shallow and deep should have different hashes");

        assert!(hash_deep1 > 0);
        assert!(hash_deep2 > 0);
    }

    #[test]
    fn test_lru_eviction() {
        let serializer = FastJson::new();

        for i in 0..11000 {
            let mut props = FxHashMap::default();
            props.insert("id".to_string(), Value::Number(i.into()));
            serializer.serialize_props_cached(&props);
        }

        let size = serializer.prop_cache.len();
        assert!(size < 11000, "Cache should have evicted some entries, size: {}", size);
        assert!(size >= 8000, "Cache should keep at least target size entries, size: {}", size);
    }

    #[test]
    fn test_deterministic_serialization() {
        let serializer = FastJson::new();

        let mut props = FxHashMap::default();
        props.insert("zebra".to_string(), Value::String("last".to_string()));
        props.insert("apple".to_string(), Value::String("first".to_string()));
        props.insert("middle".to_string(), Value::String("mid".to_string()));

        let result1 = serializer.serialize_props_cached(&props);
        let result2 = serializer.serialize_props_cached(&props);
        let result3 = serializer.serialize_props_cached(&props);

        assert_eq!(*result1, *result2, "Serialization should be deterministic");
        assert_eq!(*result2, *result3, "Serialization should be deterministic");

        let json_str = result1.as_str();
        let apple_pos = json_str.find("apple").expect("Should contain apple");
        let middle_pos = json_str.find("middle").expect("Should contain middle");
        let zebra_pos = json_str.find("zebra").expect("Should contain zebra");

        assert!(apple_pos < middle_pos, "apple should appear before middle");
        assert!(middle_pos < zebra_pos, "middle should appear before zebra");
    }
}
