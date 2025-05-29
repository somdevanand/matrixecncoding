// geometric_crypto/src/network/protocol.rs

/// The current version of the geometric crypto network protocol.
/// Used to ensure compatibility between communicating parties.
pub const PROTOCOL_VERSION: u16 = 1;

// Future Considerations for Network Protocol Resilience and Features:
// - Packet Loss Recovery:
//   - Implement mechanisms for detecting lost frames/packets (e.g., sequence numbers in StreamingFrame or packet headers).
//   - Add support for NACKs (Negative Acknowledgements) or selective retransmission requests.
//   - Consider FEC (Forward Error Correction) for high-latency or very lossy environments.
//
// - Out-of-Order Delivery Handling:
//   - Ensure the deframing logic (e.g., in streaming.rs) can correctly reassemble payloads
//     even if StreamingFrames arrive out of order, using `chunk_id` and `num_chunks`.
//   - This might involve buffering out-of-order frames up to a certain limit.
//
// - Connection State Management:
//   - For connection-oriented protocols (e.g., over TCP), manage connection lifecycle (setup, keep-alive, teardown).
//   - For connectionless (e.g., UDP), ensure each message/frame is self-contained or part of a managed session.
//
// - Congestion Control:
//   - If applicable (e.g., for high-throughput scenarios), implement or integrate congestion
//     control mechanisms to adapt sending rates to network conditions.
//
// - More Advanced Checksums/Integrity:
//   - While `IntegrityProof` handles overall data integrity, individual frames/packets might
//     benefit from simpler, faster checksums (e.g., CRC32) for quick error detection at the network level.
//     The current `StreamingFrame::Footer` has a basic checksum.
