// geometric_crypto/src/core/memory_optimizer.rs
use crate::core::coordinates::Coordinate3D;
use crate::compression::operations::GeometricOperation; // For LruCache value
use lru::LruCache;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher}; // For OperationKey
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
    pub coordinate_pool: ObjectPool<Coordinate3D>, // Made pub for potential direct use/testing
    pub operation_cache: LruCache<OperationKey, GeometricOperation>,
    pub streaming_buffer: VecDeque<u8>, // Using VecDeque as a RingBuffer
}

impl MemoryOptimizer {
    pub fn new(op_cache_capacity: NonZeroUsize, buffer_capacity: usize, coord_pool_capacity: usize) -> Self {
        Self {
            coordinate_pool: ObjectPool::new(coord_pool_capacity),
            operation_cache: LruCache::new(op_cache_capacity),
            streaming_buffer: VecDeque::with_capacity(buffer_capacity),
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

   pub fn profile_memory_usage(&self) -> MemoryProfile {
       // TODO: Implement actual memory profiling.
       // - Measure heap_usage: could use external crates or estimate based on collection sizes.
       // - Calculate cache_efficiency: (hits / (hits + misses)) for operation_cache.
       //   LruCache provides hits() and misses() methods, but they are not public.
       //   A wrapper around LruCache might be needed to track this, or use len() / cap().
       // - Analyze memory_fragmentation (very complex, likely a placeholder).

       let op_cache_len = self.operation_cache.len();
       let op_cache_cap = self.operation_cache.cap().get(); // NonZeroUsize -> usize
       let cache_eff = if op_cache_cap > 0 { op_cache_len as f64 / op_cache_cap as f64 } else { 0.0 };


       MemoryProfile {
           heap_usage: std::mem::size_of_val(&self.coordinate_pool.items) +
                       std::mem::size_of_val(&self.operation_cache) + // Size of LruCache struct itself
                       op_cache_len * (std::mem::size_of::<OperationKey>() + std::mem::size_of::<GeometricOperation>()) + // Approx size of elements
                       self.streaming_buffer.capacity() * std::mem::size_of::<u8>(), // Approx size of buffer
           cache_efficiency: cache_eff, // Using fullness as a proxy for efficiency for now
           memory_fragmentation: 0.0, // Placeholder
       }
   }
}

/// Placeholder for memory usage profiling data.
#[derive(Debug, Default, Clone, Copy, PartialEq)] // Added derives for MemoryProfile
pub struct MemoryProfile {
    pub heap_usage: usize,        // Estimated or actual heap used by optimizer's structures
    pub cache_efficiency: f64,  // E.g., hit_ratio for operation_cache
    pub memory_fragmentation: f32,// Placeholder, harder to measure simply
}

impl MemoryProfile { // Added a simple constructor for easier testing
    pub fn new(heap_usage: usize, cache_efficiency: f64, memory_fragmentation: f32) -> Self {
        Self { heap_usage, cache_efficiency, memory_fragmentation }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::Coordinate3D;
    use crate::compression::operations::{GeometricOperation, InterpolationType, EncodingScheme, CoordinateDelta, CompressedValues};
    use std::io::Cursor; 


    #[test]
    fn test_object_pool_basic() {
        let mut pool = ObjectPool::<Coordinate3D>::new(5);
        assert_eq!(pool.items.len(), 5);

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
    fn test_profile_memory_usage_stub() {
        let mut optimizer = MemoryOptimizer::new(NonZeroUsize::new(10).unwrap(), 1024, 10); // mut for cache modification
        let profile_initial = optimizer.profile_memory_usage();

        // Basic checks for placeholder values or rough estimates
        assert!(profile_initial.heap_usage > 0); // Should have some size
        assert_eq!(profile_initial.cache_efficiency, 0.0); // Initially empty cache
        assert_eq!(profile_initial.memory_fragmentation, 0.0); // Placeholder
        
        // Add an item to cache to check efficiency calculation change
        let op = GeometricOperation::RegionFill { 
            start: Coordinate3D::default(), end: Coordinate3D::default(), fill_byte: 0, compression_ratio: 0.0
        };
        let key = OperationKey::new(&op).unwrap();
        optimizer.operation_cache.put(key, op); // Modify cache
        
        let profile_after_put = optimizer.profile_memory_usage();
        assert!(profile_after_put.cache_efficiency > 0.0, "Cache efficiency should increase after put");
        assert_eq!(profile_after_put.cache_efficiency, 1.0 / 10.0, "Cache efficiency for 1 item in cap 10");
    }
    
    #[test]
    fn test_memory_profile_constructor() { 
        let profile = MemoryProfile::new(100, 0.9, 0.1);
        assert_eq!(profile.heap_usage, 100);
        assert!((profile.cache_efficiency - 0.9).abs() < f64::EPSILON);
        assert!((profile.memory_fragmentation - 0.1).abs() < f32::EPSILON);
    }

    // New tests for the implemented process_streaming_data
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
