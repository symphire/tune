use crate::common::SemanticKey;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct KeyProvider {
    counter: AtomicU64,
}

impl KeyProvider {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(1),
        } // reserve 0 for invalid keys
    }

    pub fn next(&self) -> SemanticKey {
        SemanticKey(self.counter.fetch_add(1, Ordering::Relaxed))
    }
}
