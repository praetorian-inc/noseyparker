use std::sync::Mutex;

use gix::hashtable::HashSet;
use gix::ObjectId;

use crate::blob_id::BlobId;

/// A set of `BlobId` values, designed for concurrent modification.
///
/// This implementation imposes an equivalence relation on blob IDs, assigning each to one of 256
/// classes (based on its first byte). Each class is represented by a standard `HashMap` protected
/// by a `Mutex`. Since blob IDs are SHA-1 digests, and hence effectively random, the odds that two
/// random blob IDs appear in the same class is 1/256.
///
/// We can model this as a generalized birthday problem. With 256 mutex-protected hash sets,
/// (i.e., "days in the year" or "possible birthdays"), you would need 20 threads (i.e., "people")
/// accessing the set simultaneously to exceed 50% probability of 2 threads contending.
///
/// Or in other words, there should be relatively little contention on that global data structure
/// even when using lots of threads.
pub struct BlobIdSet {
    sets: [Mutex<HashSet<ObjectId>>; 256],
}

impl BlobIdSet {
    pub fn new() -> Self {
        BlobIdSet {
            // What's this weird initialization?
            // It's to get around the fact that `Mutex` is not `Copy`.
            // https://stackoverflow.com/a/69756635
            sets: [(); 256]
                .map(|_| Mutex::new(HashSet::with_capacity_and_hasher(1024, Default::default()))),
        }
    }

    /// Add the given `BlobId` to the set.
    ///
    /// Returns `true` if and only if the set was modified by this operation.
    #[inline]
    pub fn insert(&self, blob_id: BlobId) -> bool {
        let bucket: u8 = blob_id.as_bytes()[0];
        self.sets[bucket as usize]
            .lock()
            .unwrap()
            .insert(blob_id.into())
    }

    /// Check if the given `BlobId` is in the set without modifying it.
    #[inline]
    pub fn contains(&self, blob_id: &BlobId) -> bool {
        let bucket: u8 = blob_id.as_bytes()[0];
        self.sets[bucket as usize]
            .lock()
            .unwrap()
            .contains(&ObjectId::from(blob_id))
    }

    /// Return the total number of blob IDs contained in the set.
    ///
    /// Note: this is not a cheap operation.
    pub fn len(&self) -> usize {
        self.sets.iter().map(|b| b.lock().unwrap().len()).sum()
    }

    /// Is the set empty?
    ///
    /// Note: this is not a cheap operation.
    pub fn is_empty(&self) -> bool {
        self.sets.iter().all(|b| b.lock().unwrap().is_empty())
    }
}

impl Default for BlobIdSet {
    fn default() -> Self {
        Self::new()
    }
}
