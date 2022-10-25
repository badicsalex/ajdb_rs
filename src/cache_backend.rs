// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::future::Future;
use std::hash::Hash;
use std::num::NonZeroUsize;

use async_once_cell::OnceCell;
use lru::LruCache;
use std::sync::{Arc, Mutex};

/*
   Reasons behind the 'data' field of this abomination:

   - Mutex is used to protect the LruCache, because the operations behind the
     lock is very fast, faster than even a tokio task switch. It cannot be an
     RwLock or similar, because we do modify the struct.
     Maybe sharding (or outright thread locals) can be implemented if contention
     becomes an issue.

   - LruCache is the simplest, most robust LRU cache implementation I could find.

   - The stored objects are Arcs, because (this) LruCache does not support pinning
     properly, and we don't want to lock the Mutex for long, we need something to
     break the lifetime dependence between it and the entry itself.
     Arc was chosen, because the worst that can happen is that it rotates out while
     OnceCell is doing its thing, and then gets destroyed.
     The proper solution would be an LruCache implementation that pins "in use"
     objects without requiring a write lock, or even a lifetime. That might not be
     possible without the LruCache containing the Mutex too.

   - Async OnceCell is used to make sure that if two tasks try to access the same
     key for the first time, the second one will asynchronously wait. "Regular"
     OnceCell would block. Also this leaves the window open for an async initializer
     function.

   One big problem is that Error results (which should be uncommon) leave empty
   OnceCells in the LruCache, crowding out useful entries. This is unfortunately
   inherent in the locking scheme, and removing them is not as easy as it sounds.
   Fortunately they get rotated out if not repeatedly accessed, or if the problem
   is intermittent and there is a successful run.
*/

pub struct CacheBackend<K: Hash + Eq, T> {
    data: Mutex<LruCache<K, Arc<OnceCell<T>>>>,
}

impl<K: Hash + Eq, T: Clone> CacheBackend<K, T> {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            data: Mutex::new(LruCache::new(capacity)),
        }
    }

    /// Get or init a single value.
    ///
    /// In case multiple tasks concurrently
    /// try to access a value for the first time, only one will actually
    /// run the init function, the rest will wait asynchronously.
    ///
    /// In case of an error coming from the init function, the error is
    /// forwarded, and no actual value is stored in the LRU.
    pub async fn get_or_try_init<E>(
        &self,
        k: K,
        init: impl Future<Output = Result<T, E>>,
    ) -> Result<T, E> {
        let cell_rc = {
            // It's important that we don't hold this lock for long
            // The code block is here to remind the reader of this
            let mut locked_data = self.data.lock().expect("Cache lock was poisoned");
            locked_data
                .get_or_insert(k, || Arc::new(OnceCell::new()))
                .clone()
        };
        cell_rc.get_or_try_init(init).await.cloned()
    }

    pub fn contains(&self, k: &K) -> bool {
        self.data
            .lock()
            .expect("Cache lock was poisoned")
            .contains(k)
    }

    // TODO: Synchronous get() and set()
}

// BIG TODO: Tests.
