// geometric_crypto/src/network/streaming.rs
use serde::{Serialize, Deserialize};
use bincode; // For serializing/deserializing the frame itself if needed, or just its data part
use super::NetworkError; // From network/mod.rs
use std::collections::HashMap; // Added for deframe_data

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum StreamingFrame {
    Header {
        total_payload_size: u64,
        num_chunks: u32,
        // Potentially other metadata like payload hash, encryption IV for the stream etc.
    },
    DataChunk {
        chunk_id: u32, // 0-indexed
        data: Vec<u8>,
    },
    Footer {
        // Example: checksum of the entire original payload or sequence of chunk hashes
        checksum: u32, // Simple checksum for now (e.g., sum of all bytes in original payload)
    },
}

/// Frames a large payload into a sequence of `StreamingFrame`s.
pub fn frame_data(
    payload: &[u8],
    chunk_size: usize,
) -> Result<Vec<StreamingFrame>, NetworkError> {
    if chunk_size == 0 {
        return Err(NetworkError::InvalidFormat("Chunk size cannot be zero".to_string()));
    }

    let total_payload_size = payload.len() as u64;
    // Calculate num_chunks, ensuring it handles payload.len() == 0 correctly
    let num_chunks = if total_payload_size == 0 {
        0 // No data chunks if payload is empty, but we might still send Header/Footer.
          // For this impl, if payload is empty, we'll send Header(0,0), Footer.
    } else {
        (total_payload_size + chunk_size as u64 - 1) / chunk_size as u64 // Ceiling division
    } as u32;


    let mut frames = Vec::new();

    // 1. Header Frame
    frames.push(StreamingFrame::Header {
        total_payload_size,
        num_chunks,
    });

    // 2. Data Chunk Frames
    if total_payload_size > 0 {
        for (i, chunk_bytes) in payload.chunks(chunk_size).enumerate() {
            frames.push(StreamingFrame::DataChunk {
                chunk_id: i as u32,
                data: chunk_bytes.to_vec(),
            });
        }
    }

    // 3. Footer Frame
    // Simple checksum: sum of all bytes in payload, modulo u32::MAX (implicitly by wrapping_add)
    let checksum = payload.iter().fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32));
    frames.push(StreamingFrame::Footer { checksum });

    Ok(frames)
}

/// Reassembles a payload from a sequence of `StreamingFrame`s.
pub fn deframe_data(
    frames: Vec<StreamingFrame> // Taking ownership to consume
) -> Result<Vec<u8>, NetworkError> {
    if frames.len() < 2 { // Must have at least Header and Footer
        return Err(NetworkError::InvalidFormat("Too few frames for valid stream".to_string()));
    }

    let mut header_frame: Option<StreamingFrame> = None;
    let mut footer_frame: Option<StreamingFrame> = None;
    let mut data_chunks: HashMap<u32, Vec<u8>> = HashMap::new(); // Store chunks by chunk_id

    for frame in frames {
        match frame {
            StreamingFrame::Header { .. } => {
                if header_frame.is_some() {
                    return Err(NetworkError::InvalidFormat("Multiple Header frames found".to_string()));
                }
                header_frame = Some(frame);
            }
            StreamingFrame::DataChunk { chunk_id, data } => {
                if data_chunks.insert(chunk_id, data).is_some() {
                    return Err(NetworkError::InvalidFormat(format!("Duplicate chunk_id: {}", chunk_id)));
                }
            }
            StreamingFrame::Footer { .. } => {
                if footer_frame.is_some() {
                    return Err(NetworkError::InvalidFormat("Multiple Footer frames found".to_string()));
                }
                footer_frame = Some(frame);
            }
        }
    }

    // Validate header and footer presence
    let header = header_frame.ok_or_else(|| NetworkError::InvalidFormat("Header frame missing".to_string()))?;
    let footer = footer_frame.ok_or_else(|| NetworkError::InvalidFormat("Footer frame missing".to_string()))?;

    let (total_payload_size, expected_num_chunks) = match header {
        StreamingFrame::Header { total_payload_size, num_chunks } => (total_payload_size, num_chunks),
        _ => unreachable!(), // Should be Header
    };
    
    let expected_checksum = match footer {
        StreamingFrame::Footer { checksum } => checksum,
        _ => unreachable!(), // Should be Footer
    };

    // Basic validation of chunk counts
    if data_chunks.len() as u32 != expected_num_chunks {
        return Err(NetworkError::InvalidFormat(format!(
            "Chunk count mismatch: Header expected {}, got {}",
            expected_num_chunks, data_chunks.len()
        )));
    }
    
    // Reconstruct payload
    let mut payload = Vec::with_capacity(total_payload_size as usize);
    for i in 0..expected_num_chunks {
        let chunk_data = data_chunks.remove(&i)
            .ok_or_else(|| NetworkError::InvalidFormat(format!("Missing chunk_id: {}", i)))?;
        payload.extend_from_slice(&chunk_data);
    }

    if payload.len() as u64 != total_payload_size {
         return Err(NetworkError::InvalidFormat(format!(
            "Reconstructed payload size {} does not match header specified size {}",
            payload.len(), total_payload_size
        )));
    }

    // Verify checksum
    let calculated_checksum = payload.iter().fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32));
    if calculated_checksum != expected_checksum {
        return Err(NetworkError::InvalidFormat(format!(
            "Checksum mismatch: Expected {}, calculated {}",
            expected_checksum, calculated_checksum
        )));
    }

    Ok(payload)
}


#[cfg(test)]
mod tests {
    use super::*;
    // use std::collections::HashMap; // Not needed directly in tests, but used by deframe_data

    #[test]
    fn test_frame_data_empty_payload() {
        let payload: Vec<u8> = Vec::new();
        let chunk_size = 1024;
        let frames_result = frame_data(&payload, chunk_size);
        assert!(frames_result.is_ok());
        let frames = frames_result.unwrap();
        
        assert_eq!(frames.len(), 2); // Header and Footer
        match &frames[0] {
            StreamingFrame::Header { total_payload_size, num_chunks } => {
                assert_eq!(*total_payload_size, 0);
                assert_eq!(*num_chunks, 0);
            }
            _ => panic!("Expected Header frame first"),
        }
        match &frames[1] {
            StreamingFrame::Footer { checksum } => {
                assert_eq!(*checksum, 0); // Checksum of empty payload is 0
            }
            _ => panic!("Expected Footer frame last"),
        }
    }

    #[test]
    fn test_frame_data_single_chunk() {
        let payload = vec![1,2,3,4,5];
        let chunk_size = 10;
        let frames = frame_data(&payload, chunk_size).unwrap();

        assert_eq!(frames.len(), 3); // Header, 1 DataChunk, Footer
        if let StreamingFrame::Header { total_payload_size, num_chunks } = &frames[0] {
            assert_eq!(*total_payload_size, 5);
            assert_eq!(*num_chunks, 1);
        } else { panic!("Bad header"); }
        if let StreamingFrame::DataChunk { chunk_id, data } = &frames[1] {
            assert_eq!(*chunk_id, 0);
            assert_eq!(*data, payload);
        } else { panic!("Bad data chunk"); }
        if let StreamingFrame::Footer { checksum } = &frames[2] {
            assert_eq!(*checksum, 1u32+2+3+4+5);
        } else { panic!("Bad footer"); }
    }

    #[test]
    fn test_frame_data_multiple_chunks() {
        let payload: Vec<u8> = (0..15).collect(); // 15 bytes
        let chunk_size = 5; // 3 chunks
        let frames = frame_data(&payload, chunk_size).unwrap();

        assert_eq!(frames.len(), 3 + 2); // Header, 3 DataChunks, Footer
        if let StreamingFrame::Header{ total_payload_size, num_chunks } = &frames[0] {
            assert_eq!(*total_payload_size, 15);
            assert_eq!(*num_chunks, 3);
        } else { panic!("Bad header"); }
        // Check chunks
        for i in 0..3 {
            if let StreamingFrame::DataChunk{ chunk_id, data } = &frames[i+1] {
                assert_eq!(*chunk_id, i as u32);
                let expected_chunk_data: Vec<u8> = ( (i*chunk_size) .. ((i+1)*chunk_size) ).map(|x| x as u8).collect();
                assert_eq!(*data, expected_chunk_data);
            } else { panic!("Missing or bad data chunk {}", i); }
        }
        if let StreamingFrame::Footer { checksum } = &frames[4] {
            let expected_sum = payload.iter().fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32));
            assert_eq!(*checksum, expected_sum);
        } else { panic!("Bad footer"); }
    }
    
    #[test]
    fn test_frame_data_chunk_size_zero() {
        let payload = vec![1,2,3];
        let result = frame_data(&payload, 0);
        assert!(result.is_err());
        match result.err().unwrap() {
            NetworkError::InvalidFormat(msg) => assert!(msg.contains("Chunk size cannot be zero")),
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_deframe_data_valid_stream() {
        let payload: Vec<u8> = (0..100).collect(); // 100 bytes
        let chunk_size = 20; // 5 chunks
        let frames = frame_data(&payload, chunk_size).unwrap();
        
        let deframed_payload_result = deframe_data(frames);
        assert!(deframed_payload_result.is_ok());
        assert_eq!(deframed_payload_result.unwrap(), payload);
    }

    #[test]
    fn test_deframe_data_missing_header() {
        let payload: Vec<u8> = (0..10).collect();
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        frames.remove(0); // Remove header
        let result = deframe_data(frames);
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if s.contains("Header frame missing")));
    }

    #[test]
    fn test_deframe_data_missing_footer() {
        let payload: Vec<u8> = (0..10).collect();
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        frames.pop(); // Remove footer
        let result = deframe_data(frames);
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if s.contains("Footer frame missing")));
    }
    
    #[test]
    fn test_deframe_data_duplicate_chunk() {
        let payload: Vec<u8> = (0..10).collect();
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        // Duplicate chunk 0
        if let StreamingFrame::DataChunk{data, ..} = &frames[1] {
            frames.insert(2, StreamingFrame::DataChunk{chunk_id: 0, data: data.clone()});
        } else { panic!("Test setup failed: frames[1] not a DataChunk"); }
        let result = deframe_data(frames);
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if s.contains("Duplicate chunk_id: 0")) );
    }

    #[test]
    fn test_deframe_data_missing_chunk() {
        let payload: Vec<u8> = (0..15).collect(); // 3 chunks of 5
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        frames.remove(2); // Remove chunk_id 1 (frames[0]=H, frames[1]=C0, frames[2]=C1, frames[3]=C2, frames[4]=F)
        let result = deframe_data(frames);
        assert!(result.is_err());
        // assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if
        //     s.contains("Missing chunk_id: 1")));
        // The actual error when a chunk is missing will be a chunk count mismatch
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if
            s.contains("Chunk count mismatch")));
    }
    
    #[test]
    fn test_deframe_data_payload_size_mismatch() {
        let payload: Vec<u8> = (0..10).collect();
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        if let StreamingFrame::Header{ref mut total_payload_size, ..} = &mut frames[0] {
            *total_payload_size = 9; // Tamper with size
        } else {panic!("Test setup failed");}
        let result = deframe_data(frames);
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if s.contains("Reconstructed payload size 10 does not match header specified size 9")));
    }

    #[test]
    fn test_deframe_data_checksum_mismatch() {
        let payload: Vec<u8> = (0..10).collect();
        let chunk_size = 5;
        let mut frames = frame_data(&payload, chunk_size).unwrap();
        if let StreamingFrame::Footer{ref mut checksum, ..} = frames.last_mut().unwrap() {
            *checksum = checksum.wrapping_add(1); // Tamper with checksum
        } else {panic!("Test setup failed");}
        let result = deframe_data(frames);
        assert!(matches!(result, Err(NetworkError::InvalidFormat(s)) if s.contains("Checksum mismatch")));
    }

}
