use std::sync::Mutex;

use gix::hashtable::HashMap;
use gix::ObjectId;

use crate::blob_id::BlobId;

/// A finite map with `BlobId` values as keys, designed for concurrent modification.
///
/// This implementation imposes an equivalence relation on blob IDs, assigning each to one of 256
/// classes (based on its first byte). Each class is represented by a standard `HashMap` protected
/// by a `Mutex`. Since blob IDs are SHA-1 digests, and hence effectively random, the odds that two
/// random blob IDs appear in the same class is 1/256.
///
/// We can model this as a generalized birthday problem. With 256 mutex-protected hash maps,
/// (i.e., "days in the year" or "possible birthdays"), you would need 20 threads (i.e., "people")
/// accessing the set simultaneously to exceed 50% probability of 2 threads contending.
///
/// Or in other words, there should be relatively little contention on that global data structure
/// even when using lots of threads.
pub struct BlobIdMap<V> {
    maps: [Mutex<HashMap<ObjectId, V>>; 256],
}

impl<V> BlobIdMap<V> {
    pub fn new() -> Self {
        BlobIdMap {
            // What's this weird initialization?
            // It's to get around the fact that `Mutex` is not `Copy`.
            // https://stackoverflow.com/a/69756635
            maps: [(); 256]
                .map(|_| Mutex::new(HashMap::with_capacity_and_hasher(1024, Default::default()))),
        }
    }

    /// Add the given `BlobId` to the map.
    ///
    /// Returns the old value mapped to the `BlobId`, if any.
    #[inline]
    pub fn insert(&self, blob_id: BlobId, v: V) -> Option<V> {
        let bucket: u8 = blob_id.as_bytes()[0];
        self.maps[bucket as usize]
            .lock()
            .unwrap()
            .insert(blob_id.into(), v)
    }

    /// Check if the given `BlobId` is in the map without modifying it.
    #[inline]
    pub fn contains_key(&self, blob_id: &BlobId) -> bool {
        let bucket: u8 = blob_id.as_bytes()[0];
        self.maps[bucket as usize]
            .lock()
            .unwrap()
            .contains_key(&ObjectId::from(blob_id))
    }

    /// Return the total number of blob IDs contained in the map.
    ///
    /// Note: this is not a cheap operation.
    pub fn len(&self) -> usize {
        self.maps.iter().map(|b| b.lock().unwrap().len()).sum()
    }

    /// Is the map empty?
    ///
    /// Note: this is not a cheap operation.
    pub fn is_empty(&self) -> bool {
        self.maps.iter().all(|b| b.lock().unwrap().is_empty())
    }
}

impl<V: Copy> BlobIdMap<V> {
    /// Get the value mapped to the given `BlobId`.
    #[inline]
    pub fn get(&self, blob_id: &BlobId) -> Option<V> {
        let bucket: u8 = blob_id.as_bytes()[0];
        self.maps[bucket as usize]
            .lock()
            .unwrap()
            .get(&ObjectId::from(blob_id))
            .copied()
    }
}

impl<V> Default for BlobIdMap<V> {
    fn default() -> Self {
        Self::new()
    }
}
