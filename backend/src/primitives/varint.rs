use crate::error::CodecError;
use std::fmt;
use std::io::{Cursor, Read};
use std::ops::Deref;

/// A QUIC 62-bit Variable-Length Integer as defined in RFC 9000 §16.
///
/// QUIC variable-length integers encode values between `0` and `2^62 - 1`
/// into 1, 2, 4, or 8 bytes of wire data, with the leading 2 bits specifying the length.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarInt(pub u64);

impl VarInt {
    /// Maximum value representable by a 62-bit VarInt (2^62 - 1).
    pub const MAX_VALUE: u64 = (1u64 << 62) - 1; // 0x3fff_ffff_ffff_ffff

    /// Maximum VarInt instance.
    pub const MAX: VarInt = VarInt(Self::MAX_VALUE);

    /// Zero constant.
    pub const ZERO: VarInt = VarInt(0);

    /// Construct a new `VarInt` with value bounds checking.
    #[inline]
    pub fn from_u64(val: u64) -> Result<Self, CodecError> {
        if val > Self::MAX_VALUE {
            Err(CodecError::InvalidVarInt {
                offset: 0,
                value: val,
            })
        } else {
            Ok(VarInt(val))
        }
    }

    /// Returns the inner `u64` value.
    #[inline]
    pub const fn into_inner(self) -> u64 {
        self.0
    }

    /// Calculate the wire length required to encode this `VarInt` (1, 2, 4, or 8 bytes).
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        if self.0 <= 0x3F {
            1
        } else if self.0 <= 0x3FFF {
            2
        } else if self.0 <= 0x3FFF_FFFF {
            4
        } else {
            8
        }
    }

    /// Encode this `VarInt` into a mutable byte buffer.
    ///
    /// Returns the number of bytes written (`1`, `2`, `4`, or `8`).
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, CodecError> {
        let len = self.encoded_len();
        if buf.len() < len {
            return Err(CodecError::BufferTooShort {
                offset: 0,
                needed: len,
                available: buf.len(),
            });
        }

        match len {
            1 => {
                buf[0] = self.0 as u8;
            }
            2 => {
                let v = ((self.0 as u16) | 0x4000).to_be_bytes();
                buf[..2].copy_from_slice(&v);
            }
            4 => {
                let v = ((self.0 as u32) | 0x8000_0000).to_be_bytes();
                buf[..4].copy_from_slice(&v);
            }
            8 => {
                let v = (self.0 | 0xC000_0000_0000_0000).to_be_bytes();
                buf[..8].copy_from_slice(&v);
            }
            _ => unreachable!(),
        }

        Ok(len)
    }

    /// Decode a `VarInt` from a `std::io::Cursor<&[u8]>`.
    pub fn decode(buf: &mut Cursor<&[u8]>) -> Result<VarInt, CodecError> {
        let offset = buf.position() as usize;
        let mut first_byte = [0u8; 1];
        if buf.read_exact(&mut first_byte).is_err() {
            return Err(CodecError::UnexpectedEof {
                offset,
                needed: 1,
                available: 0,
            });
        }

        let prefix = first_byte[0] >> 6;
        let len = match prefix {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            0b11 => 8,
            _ => unreachable!(),
        };

        let total_available = (buf.get_ref().len()).saturating_sub(offset);
        if total_available < len {
            return Err(CodecError::UnexpectedEof {
                offset,
                needed: len,
                available: total_available,
            });
        }

        let mut rest = [0u8; 8];
        rest[0] = first_byte[0] & 0x3F;

        if len > 1 {
            if buf.read_exact(&mut rest[1..len]).is_err() {
                return Err(CodecError::UnexpectedEof {
                    offset,
                    needed: len,
                    available: total_available,
                });
            }
        }

        let val = match len {
            1 => rest[0] as u64,
            2 => u16::from_be_bytes([rest[0], rest[1]]) as u64,
            4 => u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64,
            8 => u64::from_be_bytes(rest),
            _ => unreachable!(),
        };

        Ok(VarInt(val))
    }

    /// Decode a `VarInt` directly from a byte slice.
    ///
    /// Returns the decoded `VarInt` and the number of bytes consumed.
    pub fn decode_from_slice(slice: &[u8]) -> Result<(VarInt, usize), CodecError> {
        if slice.is_empty() {
            return Err(CodecError::UnexpectedEof {
                offset: 0,
                needed: 1,
                available: 0,
            });
        }

        let first = slice[0];
        let prefix = first >> 6;
        let len = match prefix {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            0b11 => 8,
            _ => unreachable!(),
        };

        if slice.len() < len {
            return Err(CodecError::UnexpectedEof {
                offset: 0,
                needed: len,
                available: slice.len(),
            });
        }

        let val = match len {
            1 => (first & 0x3F) as u64,
            2 => (((first & 0x3F) as u64) << 8) | (slice[1] as u64),
            4 => {
                (((first & 0x3F) as u64) << 24)
                    | ((slice[1] as u64) << 16)
                    | ((slice[2] as u64) << 8)
                    | (slice[3] as u64)
            }
            8 => {
                (((first & 0x3F) as u64) << 56)
                    | ((slice[1] as u64) << 48)
                    | ((slice[2] as u64) << 40)
                    | ((slice[3] as u64) << 32)
                    | ((slice[4] as u64) << 24)
                    | ((slice[5] as u64) << 16)
                    | ((slice[6] as u64) << 8)
                    | (slice[7] as u64)
            }
            _ => unreachable!(),
        };

        Ok((VarInt(val), len))
    }
}

impl Deref for VarInt {
    type Target = u64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u8> for VarInt {
    #[inline]
    fn from(v: u8) -> Self {
        VarInt(v as u64)
    }
}

impl From<u16> for VarInt {
    #[inline]
    fn from(v: u16) -> Self {
        VarInt(v as u64)
    }
}

impl From<u32> for VarInt {
    #[inline]
    fn from(v: u32) -> Self {
        VarInt(v as u64)
    }
}

impl TryFrom<u64> for VarInt {
    type Error = CodecError;

    #[inline]
    fn try_from(v: u64) -> Result<Self, Self::Error> {
        VarInt::from_u64(v)
    }
}

impl From<VarInt> for u64 {
    #[inline]
    fn from(v: VarInt) -> Self {
        v.0
    }
}

impl fmt::Debug for VarInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarInt({})", self.0)
    }
}

impl fmt::Display for VarInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
