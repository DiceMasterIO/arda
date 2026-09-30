//! A byte-bounded least-recently-used cache with deterministic eviction.
//!
//! Every server cache is one of these, so resident memory is the sum of the
//! configured budgets (goal-prompt §8: limits refuse, never silently grow).

use std::collections::BTreeMap;
use std::sync::Arc;

struct Entry<V> {
    value: Arc<V>,
    bytes: usize,
    tick: u64,
}

/// LRU keyed by `K`, holding shared values whose sizes sum to at most `capacity` bytes.
pub struct ByteLru<K: Ord + Clone, V> {
    capacity: usize,
    used: usize,
    tick: u64,
    entries: BTreeMap<K, Entry<V>>,
    order: BTreeMap<u64, K>,
}

impl<K: Ord + Clone, V> ByteLru<K, V> {
    /// An empty cache that never holds more than `capacity` bytes.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            used: 0,
            tick: 0,
            entries: BTreeMap::new(),
            order: BTreeMap::new(),
        }
    }

    /// Returns a hit and marks it most recently used.
    pub fn get(&mut self, key: &K) -> Option<Arc<V>> {
        self.tick += 1;
        let tick = self.tick;
        let entry = self.entries.get_mut(key)?;
        self.order.remove(&entry.tick);
        entry.tick = tick;
        self.order.insert(tick, key.clone());
        Some(Arc::clone(&entry.value))
    }

    /// Inserts `value`, evicting least-recently-used entries until it fits.
    ///
    /// A value larger than the whole budget is returned uncached.
    pub fn insert(&mut self, key: K, value: Arc<V>, bytes: usize) -> Arc<V> {
        if bytes > self.capacity {
            return value;
        }
        self.remove(&key);
        while self.used + bytes > self.capacity {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some(gone) = self.entries.remove(&oldest) {
                self.used -= gone.bytes;
            }
        }
        self.tick += 1;
        self.order.insert(self.tick, key.clone());
        self.entries.insert(
            key,
            Entry {
                value: Arc::clone(&value),
                bytes,
                tick: self.tick,
            },
        );
        self.used += bytes;
        value
    }

    fn remove(&mut self, key: &K) {
        if let Some(old) = self.entries.remove(key) {
            self.order.remove(&old.tick);
            self.used -= old.bytes;
        }
    }

    /// Bytes currently held.
    #[must_use]
    pub const fn used(&self) -> usize {
        self.used
    }

    /// Entries currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_within_the_byte_budget() {
        let mut lru = ByteLru::new(10);
        lru.insert(1, Arc::new("a"), 4);
        lru.insert(2, Arc::new("b"), 4);
        assert!(lru.get(&1).is_some());
        lru.insert(3, Arc::new("c"), 4);
        assert!(lru.get(&2).is_none(), "2 was least recently used");
        assert!(lru.get(&1).is_some() && lru.get(&3).is_some());
        assert_eq!(lru.used(), 8);
    }

    #[test]
    fn oversized_values_are_served_but_never_cached() {
        let mut lru = ByteLru::new(3);
        let v = lru.insert(1, Arc::new(7), 4);
        assert_eq!(*v, 7);
        assert!(lru.is_empty() && lru.used() == 0);
    }

    #[test]
    fn reinserting_a_key_replaces_its_accounting() {
        let mut lru = ByteLru::new(10);
        lru.insert(1, Arc::new(1), 6);
        lru.insert(1, Arc::new(2), 3);
        assert_eq!((lru.len(), lru.used()), (1, 3));
        assert_eq!(lru.get(&1).as_deref(), Some(&2));
    }
}
