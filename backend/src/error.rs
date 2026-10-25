use std::fmt;

/// Structured error type for Mini-QUIC codec operations across all wire encodings,
/// binary cursors, headers, and frame operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// Unexpected end of input buffer while reading.
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    /// Invalid variable-length integer encoding (e.g. value exceeds 2^62 - 1).
    InvalidVarInt {
        offset: usize,
        value: u64,
    },
    /// Buffer has insufficient capacity for write operation.
    BufferTooShort {
        offset: usize,
        needed: usize,
        available: usize,
    },
    /// Reserved bits or fixed bit (0x40) did not match the RFC 9000 specification.
    InvalidReservedBits {
        offset: usize,
        value: u8,
    },
    /// Malformed length field in header or frame.
    MalformedLength {
        offset: usize,
        reason: String,
    },
    /// Unsupported QUIC version.
    UnsupportedVersion {
        offset: usize,
        version: u32,
    },
    /// Unrecognized or invalid frame type.
    InvalidFrameType {
        offset: usize,
        frame_type: u64,
    },
    /// Connection ID length exceeds maximum allowed by RFC 9000 (20 bytes).
    InvalidConnectionIdLength {
        offset: usize,
        len: usize,
    },
    /// Unrecognized long header packet type bits.
    InvalidPacketType {
        offset: usize,
        type_bits: u8,
    },
    /// Invalid packet number byte length (must be 1, 2, 3, or 4).
    InvalidPacketNumberLength {
        offset: usize,
        len: usize,
    },
    /// Incoming datagram is smaller than the required minimum MTU (e.g. 1200 bytes for Initial).
    DatagramTooSmall {
        offset: usize,
        actual: usize,
        minimum: usize,
    },
}

impl std::error::Error for CodecError {}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::UnexpectedEof {
                offset,
                needed,
                available,
            } => {
                write!(
                    f,
                    "unexpected EOF at byte offset {}: needed {} bytes, but only {} bytes available",
                    offset, needed, available
                )
            }
            CodecError::InvalidVarInt { offset, value } => {
                write!(
                    f,
                    "invalid variable-length integer at byte offset {}: value 0x{:x} exceeds max (2^62 - 1)",
                    offset, value
                )
            }
            CodecError::BufferTooShort {
                offset,
                needed,
                available,
            } => {
                write!(
                    f,
                    "buffer too short at byte offset {}: required {} bytes capacity, but only {} bytes remain",
                    offset, needed, available
                )
            }
            CodecError::InvalidReservedBits { offset, value } => {
                write!(
                    f,
                    "invalid reserved or fixed bits at byte offset {}: byte 0x{:02x} violates RFC 9000 constraints",
                    offset, value
                )
            }
            CodecError::MalformedLength { offset, reason } => {
                write!(
                    f,
                    "malformed length at byte offset {}: {}",
                    offset, reason
                )
            }
            CodecError::UnsupportedVersion { offset, version } => {
                write!(
                    f,
                    "unsupported QUIC version at byte offset {}: 0x{:08x}",
                    offset, version
                )
            }
            CodecError::InvalidFrameType { offset, frame_type } => {
                write!(
                    f,
                    "invalid frame type at byte offset {}: 0x{:02x}",
                    offset, frame_type
                )
            }
            CodecError::InvalidConnectionIdLength { offset, len } => {
                write!(
                    f,
                    "invalid Connection ID length at byte offset {}: length {} exceeds max allowed (20 bytes)",
                    offset, len
                )
            }
            CodecError::InvalidPacketType { offset, type_bits } => {
                write!(
                    f,
                    "invalid packet type bits at byte offset {}: 0x{:02x}",
                    offset, type_bits
                )
            }
            CodecError::InvalidPacketNumberLength { offset, len } => {
                write!(
                    f,
                    "invalid packet number length at byte offset {}: {} bytes (valid lengths are 1, 2, 3, or 4)",
                    offset, len
                )
            }
            CodecError::DatagramTooSmall {
                offset,
                actual,
                minimum,
            } => {
                write!(
                    f,
                    "datagram too small at byte offset {}: datagram size {} bytes is less than minimum {} bytes",
                    offset, actual, minimum
                )
            }
        }
    }
}
