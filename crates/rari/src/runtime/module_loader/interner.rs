use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub struct StringInterner {
    cache: Arc<RwLock<FxHashMap<String, Arc<str>>>>,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(FxHashMap::default())),
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
        }
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.hit_count.load(Ordering::Relaxed), self.miss_count.load(Ordering::Relaxed))
    }

    pub fn intern(&self, s: &str) -> Arc<str> {
        let cache = self.cache.read();
        if let Some(existing) = cache.get(s) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            return existing.clone();
        }
        drop(cache);

        self.miss_count.fetch_add(1, Ordering::Relaxed);
        let arc_str: Arc<str> = Arc::from(s);

        let mut cache = self.cache.write();
        cache.insert(s.to_string(), arc_str.clone());

        arc_str
    }

    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }

    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

static STRING_INTERNER: OnceLock<StringInterner> = OnceLock::new();

pub fn get_string_interner() -> &'static StringInterner {
    STRING_INTERNER.get_or_init(StringInterner::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interner_deduplication() {
        let interner = StringInterner::new();

        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");

        assert!(Arc::ptr_eq(&s1, &s2));

        let (hits, misses) = interner.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_interner_different_strings() {
        let interner = StringInterner::new();

        let s1 = interner.intern("hello");
        let s2 = interner.intern("world");

        assert!(!Arc::ptr_eq(&s1, &s2));

        let (hits, misses) = interner.stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 2);
    }

    #[test]
    fn test_interner_cache_size() {
        let interner = StringInterner::new();

        interner.intern("one");
        interner.intern("two");
        interner.intern("one");

        assert_eq!(interner.cache_size(), 2);
    }

    #[test]
    fn test_interner_clear() {
        let interner = StringInterner::new();

        interner.intern("test");
        assert_eq!(interner.cache_size(), 1);

        interner.clear();
        assert_eq!(interner.cache_size(), 0);
    }
}
