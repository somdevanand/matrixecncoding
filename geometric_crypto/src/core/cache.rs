// geometric_crypto/src/core/cache.rs
use crate::core::coordinates::Coordinate3D;
use super::cache_structs::{CoordinateMetadata, DiskBackedCoordinateMap};
use lru::LruCache;
use std::num::NonZeroUsize;

const L1_SIZE: usize = 1024; // Size of the L1 cache array

#[derive(Debug)] // Removed Clone
pub struct HierarchicalCoordinateCache {
    l1_cache: Vec<Option<(Coordinate3D, CoordinateMetadata)>>,
    l2_cache: LruCache<Coordinate3D, CoordinateMetadata>,
    cold_storage: Option<DiskBackedCoordinateMap>,
    // Basic hashing for L1: just use one component or a simple mix.
    // For simplicity in this example, (x as usize) % L1_SIZE will be used.
    // A better hash would combine x, y, z.
}

impl HierarchicalCoordinateCache {
    pub fn new(l2_capacity: NonZeroUsize, use_cold_storage: bool) -> Self {
        let mut l1_cache_vec = Vec::with_capacity(L1_SIZE);
        for _ in 0..L1_SIZE {
            l1_cache_vec.push(None);
        }

        Self {
            l1_cache: l1_cache_vec,
            l2_cache: LruCache::new(l2_capacity),
            cold_storage: if use_cold_storage {
                Some(DiskBackedCoordinateMap::new())
            } else {
                None
            },
        }
    }

    fn l1_hash(coord: &Coordinate3D) -> usize {
        // Simple hash for L1. Could be improved.
        // Using a combination of components to distribute better.
        (coord.x as usize + coord.y as usize * 17 + coord.z as usize * 31) % L1_SIZE
    }

    pub fn get(&mut self, coord: &Coordinate3D) -> Option<CoordinateMetadata> {
        let l1_idx = Self::l1_hash(coord);

        // Check L1
        if let Some((stored_coord, metadata)) = &self.l1_cache[l1_idx] {
            if stored_coord == coord {
                return Some(*metadata); // L1 Hit
            }
        }

        // Check L2
        if let Some(metadata) = self.l2_cache.get(coord) {
             // L2 Hit. Promote to L1.
             // Evict from L1 if necessary (current item at l1_idx) and place in L2.
             let meta_to_return = *metadata; // Copy metadata to return
             let evicted_l1 = self.l1_cache[l1_idx].replace((*coord, meta_to_return));
             if let Some((evicted_coord, evicted_meta)) = evicted_l1 {
                 if evicted_coord != *coord { // Don't put the same coord we are promoting back into L2 immediately
                     self.l2_cache.put(evicted_coord, evicted_meta);
                 }
             }
             return Some(meta_to_return);
        }

        // Check Cold Storage
        if let Some(storage) = &self.cold_storage {
            if let Some(metadata) = storage.get(coord) {
                // Cold Storage Hit. Promote to L2 and L1.
                // Similar promotion logic as L2 hit.
                let meta_to_return = metadata; // Assuming metadata is Copy
                let evicted_l1 = self.l1_cache[l1_idx].replace((*coord, meta_to_return));
                if let Some((evicted_coord_l1, evicted_meta_l1)) = evicted_l1 {
                     if evicted_coord_l1 != *coord {
                         // If L2 evicts, it's lost (or could go back to cold_storage if sophisticated)
                         self.l2_cache.put(evicted_coord_l1, evicted_meta_l1);
                     }
                }
                // Also put into L2 (original logic from L2 get might have already done this implicitly if L2 was checked first)
                // but explicitly putting it into L2 from cold storage makes sense.
                self.l2_cache.put(*coord, meta_to_return); 
                return Some(meta_to_return);
            }
        }
        None // Not found in any cache level
    }

    pub fn put(&mut self, coord: Coordinate3D, metadata: CoordinateMetadata) {
        let l1_idx = Self::l1_hash(&coord);

        // Evict current L1 item to L2 if the slot is occupied by a different coordinate
        if let Some((stored_coord, stored_meta)) = self.l1_cache[l1_idx].take() { // take() sets slot to None
             if stored_coord != coord {
                 // LruCache::put returns Option<V>, not Option<(K,V)>.
                 // This means we only get the evicted_value, not its key.
                 // The current DiskBackedCoordinateMap::put expects a key.
                 // For now, we'll skip putting to cold_storage if we don't have the key easily.
                 // A more complex solution might involve peeking at the LRU item before it's evicted.
                 let _evicted_value_option = self.l2_cache.put(stored_coord, stored_meta);
                 // if let Some(evicted_value) = evicted_value_option {
                 //     if let Some(cold_storage) = &mut self.cold_storage {
                 //         // cold_storage.put(evicted_key, evicted_value); // We don't have evicted_key here
                 //     }
                 // }
             }
        }
        // Place new item in L1
        self.l1_cache[l1_idx] = Some((coord, metadata));
        
        // Also update L2 (or put if not present).
        // This makes L2 a superset of L1's non-empty unique items, or close to it.
        // This policy can vary. Here, we ensure what's in L1 is also fresh in L2.
        let _evicted_value_option = self.l2_cache.put(coord, metadata);
        // if let Some(evicted_value) = evicted_value_option {
        //     if let Some(cold_storage) = &mut self.cold_storage {
        //         // cold_storage.put(evicted_key, evicted_value); // We don't have evicted_key here
        //         // And, if evicted_key was somehow available, the check `evicted_key != coord`
        //         // would be relevant.
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    fn new_cache(l2_cap: usize) -> HierarchicalCoordinateCache {
        HierarchicalCoordinateCache::new(NonZeroUsize::new(l2_cap).unwrap(), false)
    }

    #[test]
    fn test_cache_new() {
        let cache = new_cache(10);
        assert_eq!(cache.l1_cache.len(), L1_SIZE);
        assert!(cache.l1_cache.iter().all(|x| x.is_none()));
        assert_eq!(cache.l2_cache.cap().get(), 10);
        assert!(cache.cold_storage.is_none());
    }

    #[test]
    fn test_put_and_get_l1_hit() {
        let mut cache = new_cache(10);
        let coord1 = Coordinate3D::new(1, 2, 3);
        let meta1 = CoordinateMetadata { last_access_time: 1, frequency: 1 };
        cache.put(coord1, meta1);
        
        let retrieved_meta = cache.get(&coord1);
        assert_eq!(retrieved_meta, Some(meta1));
    }

    #[test]
    fn test_l1_eviction_to_l2() {
        let mut cache = new_cache(10);
        let meta = CoordinateMetadata::default();

        // Fill one L1 slot
        let coord1 = Coordinate3D::new(1,2,3); // Hashes to l1_idx
        cache.put(coord1, meta); 
        assert!(cache.get(&coord1).is_some(), "Coord1 should be in cache");

        // Find another coord that hashes to the same L1 slot
        let mut coord2 = Coordinate3D::new(0,0,0);
        let l1_idx_coord1 = HierarchicalCoordinateCache::l1_hash(&coord1);
        for i in 0..=255 {
            coord2.x = i; // Vary x to find a hash collision for L1
            if HierarchicalCoordinateCache::l1_hash(&coord2) == l1_idx_coord1 && coord2 != coord1 {
                break;
            }
            if i == 255 { panic!("Could not find a suitable L1 hash collision for test"); }
        }
        
        cache.put(coord2, meta); // This should evict coord1 from L1 to L2
        
        // coord2 should now be in L1
        assert!(cache.l1_cache[l1_idx_coord1].as_ref().unwrap().0 == coord2, "Coord2 should be in L1 slot");
        // coord1 should be in L2
        assert!(cache.l2_cache.contains(&coord1), "Coord1 should be in L2 after L1 eviction");
        assert!(cache.get(&coord1).is_some(), "Coord1 should still be retrievable (from L2)");
    }

    #[test]
    fn test_l2_hit_promotes_to_l1() {
        let mut cache = new_cache(10);
        let meta = CoordinateMetadata::default();
        let l1_idx_target: usize;

        // 1. Put coord1, it goes to L1 and L2.
        let coord1 = Coordinate3D::new(1,1,1);
        cache.put(coord1, meta);
        l1_idx_target = HierarchicalCoordinateCache::l1_hash(&coord1);

        // 2. Fill up the L1 slot for coord1 with other coordinates enough times
        //    such that coord1 is evicted from L1 but remains in L2.
        //    We need to find L1_SIZE distinct coordinates that hash to the same L1 slot.
        //    This is hard to guarantee quickly. A simpler way:
        //    Manually clear L1 for that slot after putting to L2.
        //    Or, ensure coord1 is in L2 but not L1 by specific eviction.
        
        // Simplified: Put coord1. Then find coord2 that evicts coord1 from its L1 slot.
        let mut coord2 = Coordinate3D::new(0,0,0);
         for i in 0..=255 {
             coord2.x = i;
             if HierarchicalCoordinateCache::l1_hash(&coord2) == l1_idx_target && coord2 != coord1 {
                 break;
             }
             if i == 255 { panic!("Could not find L1 hash collision for coord2"); }
         }
         cache.put(coord2, meta); // coord1 to L2, coord2 to L1

         // Verify coord1 is not in L1's target slot, but coord2 is
         assert_ne!(cache.l1_cache[l1_idx_target].as_ref().map(|(c,_)|c), Some(&coord1));
         assert_eq!(cache.l1_cache[l1_idx_target].as_ref().map(|(c,_)|c), Some(&coord2));
         assert!(cache.l2_cache.contains(&coord1), "Coord1 should be in L2");

        // 3. Get coord1. It should be an L2 hit and then promoted to L1.
        let retrieved_meta = cache.get(&coord1);
        assert_eq!(retrieved_meta, Some(meta));
        assert_eq!(cache.l1_cache[l1_idx_target].as_ref().unwrap().0, coord1, "Coord1 should be promoted back to L1");
    }
     // More tests: L2 eviction to cold storage, cold storage hit, etc.
}
