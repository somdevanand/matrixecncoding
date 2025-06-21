// geometric_crypto/src/core/memory_optimizer.rs
use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::GeometricOperation; // For LruCache value
use lru::LruCache;
use std::collections::VecDeque;
use std::hash::Hash; // For OperationKey
use std::num::NonZeroUsize;
use bincode; // For OperationKey hashing if needed

/// A simple object pool for reusing allocations.
#[derive(Debug)]
pub struct ObjectPool<T: Default> {
    items: Vec<T>,
    // Future: Could have multiple Vecs for different chunk sizes or thread locality.
}

impl<T: Default> ObjectPool<T> {
    pub fn new(initial_capacity: usize) -> Self {
        let mut items = Vec::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            items.push(T::default());
        }
        Self { items }
    }

    /// Retrieves an item from the pool. If pool is empty, allocates a new one.
    pub fn get(&mut self) -> T {
        self.items.pop().unwrap_or_else(T::default)
    }

    /// Returns an item to the pool.
    pub fn release(&mut self, item: T) {
        // Optional: Could check if pool is over capacity and drop item.
        self.items.push(item);
    }

    /// Returns the total capacity of the internal Vec<T>.
    pub fn main_buffer_capacity(&self) -> usize {
        self.items.capacity()
    }
}

/// Key for caching GeometricOperations.
/// For simplicity, a hash of the serialized operation can be used.
/// This requires GeometricOperation to be Serialize.
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct OperationKey(Vec<u8>); // Stores the serialized (hashed or full) representation

impl OperationKey {
    pub fn new(op: &GeometricOperation) -> Result<Self, String> {
        // Using bincode to serialize the op for a unique key.
        // This could be slow if ops are large. A more optimized key would hash specific fields.
        bincode::serialize(op)
            .map(OperationKey)
            .map_err(|e| format!("Failed to create OperationKey: {}", e))
    }
}


#[derive(Debug)] // LruCache and VecDeque are Debug
pub struct MemoryOptimizer {
    pub coordinate_pool: ObjectPool<Coordinate3D>, 
    pub operation_cache: LruCache<OperationKey, GeometricOperation>,
    pub streaming_buffer: VecDeque<u8>, 
    op_cache_hits: u64,
    op_cache_misses: u64,
}

impl MemoryOptimizer {
    pub fn new(op_cache_capacity: NonZeroUsize, buffer_capacity: usize, coord_pool_capacity: usize) -> Self {
        Self {
            coordinate_pool: ObjectPool::new(coord_pool_capacity),
            operation_cache: LruCache::new(op_cache_capacity),
            streaming_buffer: VecDeque::with_capacity(buffer_capacity),
            op_cache_hits: 0,
            op_cache_misses: 0,
        }
    }

    pub fn add_to_streaming_buffer(&mut self, data: &[u8]) {
        self.streaming_buffer.extend(data);
    }

   /// Parses Coordinate3D objects from the streaming_buffer using the coordinate_pool.
   pub fn process_streaming_data(&mut self, num_coords_to_process: Option<usize>) -> Vec<Coordinate3D> {
       let mut results = Vec::new();
       
       let bytes_per_coord = 3; // x, y, z each u8
       
       let mut coords_to_make = self.streaming_buffer.len() / bytes_per_coord;
       
       if let Some(limit) = num_coords_to_process {
           coords_to_make = coords_to_make.min(limit);
       }
       
       for _ in 0..coords_to_make {
           if self.streaming_buffer.len() < bytes_per_coord {
               // Should not happen if coords_to_make is calculated correctly based on buffer length
               break; 
           }
           
           // Dequeue 3 bytes
           let x = self.streaming_buffer.pop_front().unwrap_or_default(); // Default if empty, though guarded by len check
           let y = self.streaming_buffer.pop_front().unwrap_or_default();
           let z = self.streaming_buffer.pop_front().unwrap_or_default();
           
           // Get a Coordinate3D from the pool
           let mut coord_obj = self.coordinate_pool.get();
           
           // Populate its fields
           coord_obj.x = x;
           coord_obj.y = y;
           coord_obj.z = z;
           
           results.push(coord_obj);
           // Note: The PooledObject<T> wrapper for RAII release is not used here
           // as ObjectPool::get returns T directly. Coordinates are 'leased'.
           // They are not automatically returned to the pool unless explicitly done by the caller of this function.
           // For this subtask, we return Vec<Coordinate3D> as if they are owned by the caller.
           // A full pooled system might require returning a custom smart pointer.
       }
       
       results
   }

    // Method to get an operation from the cache, tracking hits/misses.
    // Returns a clone of the operation if found, to avoid lifetime issues with LruCache internal refs.
    // Consider returning Option<&GeometricOperation> if caller can handle lifetimes or if ops are large.
    // For simplicity now, returning a clone.
    pub fn get_from_op_cache(&mut self, key: &OperationKey) -> Option<GeometricOperation> {
        if let Some(op) = self.operation_cache.get(key) {
            self.op_cache_hits += 1;
            Some(op.clone()) // Return a clone
        } else {
            self.op_cache_misses += 1;
            None
        }
    }

    // Method to put an operation into the cache.
    // The LruCache put itself might evict an old item, which is returned as Option<V> (the evicted value).
    // This method just wraps the put.
    pub fn put_in_op_cache(&mut self, key: OperationKey, op: GeometricOperation) -> Option<GeometricOperation> {
        self.operation_cache.put(key, op)
    }

   pub fn profile_memory_usage(&self) -> MemoryProfile {
       let op_cache_len = self.operation_cache.len();
       // let op_cache_cap = self.operation_cache.cap().get(); // Not used for efficiency calculation with hits/misses

       let total_op_cache_accesses = self.op_cache_hits + self.op_cache_misses;
       let cache_efficiency = if total_op_cache_accesses > 0 {
           self.op_cache_hits as f64 / total_op_cache_accesses as f64
       } else {
           0.0 // No accesses yet, efficiency is undefined or 0
       };

       // Calculate individual component sizes
       let streaming_buffer_content_bytes = self.streaming_buffer.capacity() * std::mem::size_of::<u8>();
       let coordinate_pool_content_bytes = self.coordinate_pool.main_buffer_capacity() * std::mem::size_of::<Coordinate3D>();
       
       let mut operation_cache_content_bytes = 0;
       for (key, op) in self.operation_cache.iter() {
           operation_cache_content_bytes += key.0.len(); // Size of Vec<u8> in OperationKey
           operation_cache_content_bytes += std::mem::size_of_val(op); // Size of GeometricOperation enum
            // Note: std::mem::size_of_val(op) for enums can be tricky. It gives the size of the enum discriminant
            // plus the size of the largest variant's data if enums are not stored packed.
            // For more precise heap usage of ops (e.g. Vecs inside ops), deep serialization or specific sizing needed.
            // This is a common estimation challenge. For now, this is better than just op_cache_len * average.
       }

       let heap_usage_bytes = streaming_buffer_content_bytes + 
                              coordinate_pool_content_bytes + 
                              operation_cache_content_bytes +
                              std::mem::size_of_val(&self.operation_cache); // Add overhead of LruCache itself

       MemoryProfile {
           heap_usage_bytes,
           cache_efficiency, 
           memory_fragmentation: 0.0, // Still a placeholder
           op_cache_hits: self.op_cache_hits,
           op_cache_misses: self.op_cache_misses,
           streaming_buffer_content_bytes,
           coordinate_pool_content_bytes,
           operation_cache_content_bytes,
       }
   }
}

/// Placeholder for memory usage profiling data.
#[derive(Debug, Default, Clone, Copy, PartialEq)] 
pub struct MemoryProfile {
    pub heap_usage_bytes: usize, // Renamed
    pub cache_efficiency: f64, 
    pub memory_fragmentation: f32,
    pub op_cache_hits: u64,
    pub op_cache_misses: u64,
    pub streaming_buffer_content_bytes: usize, // New
    pub coordinate_pool_content_bytes: usize,  // New
    pub operation_cache_content_bytes: usize,   // New
}

impl MemoryProfile { 
    pub fn new(
        heap_usage_bytes: usize, 
        cache_efficiency: f64, 
        memory_fragmentation: f32, 
        op_cache_hits: u64, 
        op_cache_misses: u64,
        streaming_buffer_content_bytes: usize,
        coordinate_pool_content_bytes: usize,
        operation_cache_content_bytes: usize,
    ) -> Self {
        Self { 
            heap_usage_bytes, 
            cache_efficiency, 
            memory_fragmentation, 
            op_cache_hits, 
            op_cache_misses,
            streaming_buffer_content_bytes,
            coordinate_pool_content_bytes,
            operation_cache_content_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, CoordinateDelta, CompressedValues, Matrix3x3};
    use std::io::Cursor; 


    #[test]
    fn test_object_pool_basic() {
        let mut pool = ObjectPool::<Coordinate3D>::new(5);
        assert_eq!(pool.items.capacity(), 5); // Check capacity set by new()
        assert_eq!(pool.main_buffer_capacity(), 5); // Check new method

        let c1 = pool.get();
        assert_eq!(c1, Coordinate3D::default()); // Assuming T::default() behavior
        assert_eq!(pool.items.len(), 4);

        pool.release(c1);
        assert_eq!(pool.items.len(), 5);

        // Get all, then one more (new allocation)
        let _c2: Vec<Coordinate3D> = (0..6).map(|_| pool.get()).collect();
        assert_eq!(pool.items.len(), 0);
        
        let c_new = pool.get(); // Should be default, newly allocated
        assert_eq!(c_new, Coordinate3D::default());
        pool.release(c_new);
        assert_eq!(pool.items.len(), 1);
    }

    #[test]
    fn test_operation_key_creation() {
        let op = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(1,2,3)],
            interpolation: InterpolationType::Linear,
            data_encoding: EncodingScheme::Raw,
        };
        let key_result = OperationKey::new(&op);
        assert!(key_result.is_ok());
        let key1 = key_result.unwrap();
        assert!(!key1.0.is_empty());

        // Same op should produce same key
        let key2 = OperationKey::new(&op).unwrap();
        assert_eq!(key1, key2);

        let op_different = GeometricOperation::PathTrace {
            waypoints: vec![Coordinate3D::new(3,2,1)], // Different data
            interpolation: InterpolationType::Linear,
            data_encoding: EncodingScheme::Raw,
        };
        let key_different = OperationKey::new(&op_different).unwrap();
        assert_ne!(key1, key_different);
    }

    #[test]
    fn test_memory_optimizer_new() {
        let capacity = NonZeroUsize::new(100).unwrap();
        let buffer_cap = 1024;
        let coord_pool_cap = 50;
        let optimizer = MemoryOptimizer::new(capacity, buffer_cap, coord_pool_cap);
        
        assert_eq!(optimizer.coordinate_pool.items.capacity(), coord_pool_cap); 
        assert_eq!(optimizer.operation_cache.cap(), capacity);
        assert_eq!(optimizer.streaming_buffer.capacity(), buffer_cap);
    }

    #[test]
    // Test for the old signature of process_streaming_data, can be removed or adapted
    // fn test_process_streaming_data_stub_old() {
    //     let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 10);
    //     let data: Vec<u8> = vec![1,2,3,4,5];
    //     let mut cursor = Cursor::new(data);
        
    //     let result = optimizer.process_streaming_data_old_signature(&mut cursor); // Assuming you rename the old one
    //     assert!(result.is_err());
    //     assert_eq!(result.err().unwrap(), "process_streaming_data not implemented");
    // }

    #[test]
    fn test_profile_memory_usage_detailed() {
        let op_cache_cap = NonZeroUsize::new(10).unwrap();
        let buffer_cap_bytes = 1024; // streaming_buffer capacity in bytes
        let coord_pool_cap_items = 10; // coordinate_pool capacity in items

        let mut optimizer = MemoryOptimizer::new(op_cache_cap, buffer_cap_bytes, coord_pool_cap_items);
        
        // Expected initial sizes based on capacity
        let expected_streaming_buffer_bytes = buffer_cap_bytes * std::mem::size_of::<u8>();
        let expected_coord_pool_bytes = coord_pool_cap_items * std::mem::size_of::<Coordinate3D>();
        
        // Add one item to cache
        let op1 = GeometricOperation::PathTrace { waypoints: vec![Coordinate3D::new(1,1,1)], interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw };
        let key1_data = bincode::serialize(&op1).unwrap();
        let key1 = OperationKey(key1_data.clone());
        optimizer.put_in_op_cache(key1.clone(), op1.clone());
        
        let expected_op_cache_bytes = key1_data.len() + std::mem::size_of_val(&op1) + std::mem::size_of_val(&optimizer.operation_cache);


        let profile1 = optimizer.profile_memory_usage();
        assert_eq!(profile1.streaming_buffer_content_bytes, expected_streaming_buffer_bytes);
        assert_eq!(profile1.coordinate_pool_content_bytes, expected_coord_pool_bytes);
        assert_eq!(profile1.operation_cache_content_bytes + std::mem::size_of_val(&optimizer.operation_cache), expected_op_cache_bytes, "Operation cache content size mismatch");
        assert_eq!(profile1.heap_usage_bytes, expected_streaming_buffer_bytes + expected_coord_pool_bytes + expected_op_cache_bytes);

        assert_eq!(profile1.op_cache_hits, 0);
        assert_eq!(profile1.op_cache_misses, 0);
        assert_eq!(profile1.cache_efficiency, 0.0);

        // Access present item (hit)
        let _ = optimizer.get_from_op_cache(&key1);
        let profile2 = optimizer.profile_memory_usage();
        assert_eq!(profile2.op_cache_hits, 1);
        assert_eq!(profile2.op_cache_misses, 0);
        assert!((profile2.cache_efficiency - 1.0).abs() < f64::EPSILON);
        assert_eq!(profile2.heap_usage_bytes, profile1.heap_usage_bytes); // Heap usage should be similar

        // Access non-present item (miss)
        let op2 = GeometricOperation::RegionFill { start: Default::default(), end: Default::default(), fill_byte:0, compression_ratio:0.0 };
        let key2 = OperationKey::new(&op2).unwrap();
        let _ = optimizer.get_from_op_cache(&key2);
        let profile3 = optimizer.profile_memory_usage();
        assert_eq!(profile3.op_cache_hits, 1);
        assert_eq!(profile3.op_cache_misses, 1);
        assert!((profile3.cache_efficiency - 0.5).abs() < f64::EPSILON); 
    }
    
    #[test]
    fn test_memory_profile_constructor() { 
        let profile = MemoryProfile::new(100, 0.9, 0.1, 9, 1, 20, 30, 50);
        assert_eq!(profile.heap_usage_bytes, 100);
        assert!((profile.cache_efficiency - 0.9).abs() < f64::EPSILON);
        assert!((profile.memory_fragmentation - 0.1).abs() < f32::EPSILON);
    }

    // New tests for the implemented process_streaming_data
    // And new tests for cache hit/miss tracking

    #[test]
    fn op_cache_hit_miss_tracking() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(5).unwrap(), 256, 5);
        let op1 = GeometricOperation::PathTrace { waypoints: vec![Coordinate3D::new(1,2,3)], interpolation: InterpolationType::None, data_encoding: EncodingScheme::Raw };
        let key1 = OperationKey::new(&op1).unwrap();
        
        // Miss
        assert!(optimizer.get_from_op_cache(&key1).is_none());
        assert_eq!(optimizer.op_cache_misses, 1);
        assert_eq!(optimizer.op_cache_hits, 0);

        // Put
        optimizer.put_in_op_cache(key1.clone(), op1.clone());

        // Hit
        assert!(optimizer.get_from_op_cache(&key1).is_some());
        assert_eq!(optimizer.op_cache_misses, 1);
        assert_eq!(optimizer.op_cache_hits, 1);

        // Another Hit
        assert!(optimizer.get_from_op_cache(&key1).is_some());
        assert_eq!(optimizer.op_cache_misses, 1);
        assert_eq!(optimizer.op_cache_hits, 2);

        // New key, Miss
        let op2 = GeometricOperation::RegionFill { start: Coordinate3D::new(0,0,0), end: Coordinate3D::new(1,1,1), fill_byte:0, compression_ratio: 1.0 };
        let key2 = OperationKey::new(&op2).unwrap();
        assert!(optimizer.get_from_op_cache(&key2).is_none());
        assert_eq!(optimizer.op_cache_misses, 2);
        assert_eq!(optimizer.op_cache_hits, 2);
    }


    #[test]
    fn process_empty_buffer() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        let coords = optimizer.process_streaming_data(None);
        assert!(coords.is_empty());
    }

    #[test]
    fn process_one_coord_exact_bytes() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[10, 20, 30]);
        let coords = optimizer.process_streaming_data(None);
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0], Coordinate3D::new(10, 20, 30));
        assert_eq!(optimizer.streaming_buffer.len(), 0);
    }

    #[test]
    fn process_multiple_coords() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3, 4,5,6, 7,8,9]);
        let coords = optimizer.process_streaming_data(None);
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[0], Coordinate3D::new(1,2,3));
        assert_eq!(coords[1], Coordinate3D::new(4,5,6));
        assert_eq!(coords[2], Coordinate3D::new(7,8,9));
        assert_eq!(optimizer.streaming_buffer.len(), 0);
    }

    #[test]
    fn process_incomplete_last_coord() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3, 4,5]); // One full coord, one incomplete
        let coords = optimizer.process_streaming_data(None);
        assert_eq!(coords.len(), 1);
        assert_eq!(coords[0], Coordinate3D::new(1,2,3));
        assert_eq!(optimizer.streaming_buffer.len(), 2); // Remaining 2 bytes
        assert_eq!(optimizer.streaming_buffer.pop_front(), Some(4));
        assert_eq!(optimizer.streaming_buffer.pop_front(), Some(5));
    }

    #[test]
    fn process_with_num_coords_to_process_limit() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3, 4,5,6, 7,8,9, 10,11,12]); // 4 coords
        let coords = optimizer.process_streaming_data(Some(2)); // Process only 2
        assert_eq!(coords.len(), 2);
        assert_eq!(coords[0], Coordinate3D::new(1,2,3));
        assert_eq!(coords[1], Coordinate3D::new(4,5,6));
        assert_eq!(optimizer.streaming_buffer.len(), 6); // Remaining 2 coords (6 bytes)
    }
    
    #[test]
    fn process_with_num_coords_limit_exceeding_available() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3, 4,5,6]); // 2 coords
        let coords = optimizer.process_streaming_data(Some(5)); // Request 5, but only 2 available
        assert_eq!(coords.len(), 2);
        assert_eq!(coords[0], Coordinate3D::new(1,2,3));
        assert_eq!(coords[1], Coordinate3D::new(4,5,6));
        assert_eq!(optimizer.streaming_buffer.len(), 0);
    }

    #[test]
    fn process_all_with_none_parameter() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3, 4,5,6]); // 2 coords
        let coords = optimizer.process_streaming_data(None); // Process all
        assert_eq!(coords.len(), 2);
        assert_eq!(coords[0], Coordinate3D::new(1,2,3));
        assert_eq!(coords[1], Coordinate3D::new(4,5,6));
        assert_eq!(optimizer.streaming_buffer.len(), 0);
    }

    #[test]
    fn repeated_calls_process_new_data() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 5);
        optimizer.add_to_streaming_buffer(&[1,2,3]);
        let coords1 = optimizer.process_streaming_data(None);
        assert_eq!(coords1.len(), 1);
        assert_eq!(coords1[0], Coordinate3D::new(1,2,3));
        assert_eq!(optimizer.streaming_buffer.len(), 0);

        optimizer.add_to_streaming_buffer(&[4,5,6, 7,8,9]);
        let coords2 = optimizer.process_streaming_data(None);
        assert_eq!(coords2.len(), 2);
        assert_eq!(coords2[0], Coordinate3D::new(4,5,6));
        assert_eq!(coords2[1], Coordinate3D::new(7,8,9));
        assert_eq!(optimizer.streaming_buffer.len(), 0);
    }
    
    #[test]
    fn check_pool_usage_indirectly() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 1); // Pool capacity 1
        
        // Initial state of pool: 1 item (Coordinate3D::default())
        // This is an approximation, actual pool might be empty until first get/release cycle depending on impl.
        // Our ObjectPool::new pre-populates. So, pool.items.len() == 1

        optimizer.add_to_streaming_buffer(&[1,2,3]); // Coord A
        let coords_a = optimizer.process_streaming_data(None);
        assert_eq!(coords_a.len(), 1);
        // After get(), pool.items.len() should be 0.
        // We can't directly check optimizer.coordinate_pool.items.len() as it's not pub.
        // However, if we process another coord, it should get a *new* default() if pool was empty,
        // or the one we "released" (if we had release logic).

        // To test reuse, we'd need to release coords_a[0] back to the pool.
        // The current design returns Vec<Coordinate3D>, "giving" ownership to caller.
        // So, let's simulate this by getting more items than pool capacity.
        
        let mut coord_obj_for_release = coords_a[0]; // This is a copy, not the pooled one
                                                    // To properly test pool, process_streaming_data
                                                    // should return PooledObject<T> which releases on drop,
                                                    // or we need a manual release mechanism.
        
        // For now, let's just verify that getting more items than initial capacity works (allocates new)
        optimizer.add_to_streaming_buffer(&[4,5,6]); // Coord B
        optimizer.add_to_streaming_buffer(&[7,8,9]); // Coord C

        // Pool had 1, took 1 for A. Pool has 0.
        // Takes 1 for B (new allocation, as pool is empty). Pool has 0.
        // Takes 1 for C (new allocation). Pool has 0.
        let coords_bc = optimizer.process_streaming_data(None);
        assert_eq!(coords_bc.len(), 2);
        assert_eq!(coords_bc[0], Coordinate3D::new(4,5,6));
        assert_eq!(coords_bc[1], Coordinate3D::new(7,8,9));
        
        // This test doesn't fully verify pooling/reuse due to ownership model.
        // It mainly checks that coordinates are correctly created.
        // A more direct pool test would be on ObjectPool itself.
        // We can verify that the values are distinct from default if a pooled object was reused and modified.
        // But since we get new objects, this test is limited for pool verification.
        // We can release explicitly for a better test of the pool.
        optimizer.coordinate_pool.release(coord_obj_for_release); // Release "A" (or its copy)
        // Pool now has 1 item (the released one, or a default if not matching)

        optimizer.add_to_streaming_buffer(&[10,11,12]); // Coord D
        let coords_d = optimizer.process_streaming_data(None);
        assert_eq!(coords_d.len(), 1);
        // This coord_d[0] should be the one from the pool.
        // If it was coord_obj_for_release, its value would be 1,2,3.
        // If it's a fresh one (from T::default if pool was empty or released didn't match), it's 0,0,0.
        // Our pool just pushes, so it will be coord_obj_for_release.
        assert_eq!(coords_d[0], Coordinate3D::new(10,11,12), "Value should be new, object potentially reused");
        // The above assert is wrong for testing reuse: coord_obj is modified *after* get().
        // To test reuse: get an obj, release it, get again, check if it's the *same instance* (not possible without pointers)
        // or if it's a *default* one (if pool was empty) vs a *potentially modified then released* one.
        // Current pool get() returns T::default() if empty or pops. Release pushes.
        // So if we release(A), then get(), we should get A back.
        // Then we modify it to D.

        // Let's simplify:
        // 1. Get C1 from pool (pool had default, now empty)
        // 2. Release C1 (pool has C1)
        // 3. Get C2 from pool (pool gives C1, now empty). C2 is C1.
        // 4. Modify C2.
        let mut p = ObjectPool::<Coordinate3D>::new(1); // pool has [default]
        let c1_val = Coordinate3D::new(1,1,1);
        let mut obj1 = p.get(); // obj1 is default. pool is empty.
        obj1.x = 1; obj1.y = 1; obj1.z = 1; // obj1 is now c1_val
        p.release(obj1); // pool has [c1_val]

        let mut obj2 = p.get(); // obj2 is c1_val. pool is empty.
        assert_eq!(obj2, c1_val, "Should get the released object back from pool");
        obj2.x = 2; // Modify it
        p.release(obj2); // pool has [c1_val_modified_to_2,1,1]
        
        let obj3 = p.get();
        assert_eq!(obj3.x, 2, "Should be the modified object from pool");

    }
}
