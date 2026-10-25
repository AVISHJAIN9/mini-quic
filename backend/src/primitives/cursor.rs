use crate::error::CodecError;
use crate::primitives::varint::VarInt;

/// Zero-allocation, bounds-checked binary cursor reader for QUIC wire formats.
#[derive(Debug)]
pub struct BufferReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BufferReader<'a> {
    /// Create a new `BufferReader` over the given byte slice.
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current read position (offset from beginning of buffer).
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Total length of the underlying buffer.
    #[inline]
    pub fn total_len(&self) -> usize {
        self.buf.len()
    }

    /// Number of bytes remaining to read.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Whether there are remaining unread bytes.
    #[inline]
    pub fn has_remaining(&self) -> bool {
        self.pos < self.buf.len()
    }

    /// Slice of the remaining unread bytes.
    #[inline]
    pub fn remaining_slice(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }

    /// Slice of the bytes read so far.
    #[inline]
    pub fn read_slice(&self) -> &'a [u8] {
        &self.buf[..self.pos]
    }

    /// Peek at the next byte without advancing the cursor.
    pub fn peek_u8(&self) -> Result<u8, CodecError> {
        if self.pos >= self.buf.len() {
            Err(CodecError::UnexpectedEof {
                offset: self.pos,
                needed: 1,
                available: 0,
            })
        } else {
            Ok(self.buf[self.pos])
        }
    }

    /// Read exactly `len` bytes from the cursor.
    pub fn read_exact(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if self.remaining() < len {
            return Err(CodecError::UnexpectedEof {
                offset: self.pos,
                needed: len,
                available: self.remaining(),
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.buf[start..self.pos])
    }

    /// Advance the cursor forward by `count` bytes.
    pub fn advance(&mut self, count: usize) -> Result<(), CodecError> {
        if self.remaining() < count {
            return Err(CodecError::UnexpectedEof {
                offset: self.pos,
                needed: count,
                available: self.remaining(),
            });
        }
        self.pos += count;
        Ok(())
    }

    /// Read a single `u8`.
    pub fn read_u8(&mut self) -> Result<u8, CodecError> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    /// Read a big-endian `u16`.
    pub fn read_be_u16(&mut self) -> Result<u16, CodecError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Read a big-endian 24-bit integer into a `u32`.
    pub fn read_be_u24(&mut self) -> Result<u32, CodecError> {
        let bytes = self.read_exact(3)?;
        Ok(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
    }

    /// Read a big-endian `u32`.
    pub fn read_be_u32(&mut self) -> Result<u32, CodecError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a big-endian `u64`.
    pub fn read_be_u64(&mut self) -> Result<u64, CodecError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read a QUIC variable-length integer (`VarInt`).
    pub fn read_varint(&mut self) -> Result<VarInt, CodecError> {
        let initial_offset = self.pos;
        if !self.has_remaining() {
            return Err(CodecError::UnexpectedEof {
                offset: initial_offset,
                needed: 1,
                available: 0,
            });
        }

        let first = self.buf[self.pos];
        let prefix = first >> 6;
        let len = match prefix {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            0b11 => 8,
            _ => unreachable!(),
        };

        if self.remaining() < len {
            return Err(CodecError::UnexpectedEof {
                offset: initial_offset,
                needed: len,
                available: self.remaining(),
            });
        }

        let val = match len {
            1 => {
                self.pos += 1;
                (first & 0x3F) as u64
            }
            2 => {
                let b1 = self.buf[self.pos + 1];
                self.pos += 2;
                (((first & 0x3F) as u64) << 8) | (b1 as u64)
            }
            4 => {
                let b1 = self.buf[self.pos + 1];
                let b2 = self.buf[self.pos + 2];
                let b3 = self.buf[self.pos + 3];
                self.pos += 4;
                (((first & 0x3F) as u64) << 24)
                    | ((b1 as u64) << 16)
                    | ((b2 as u64) << 8)
                    | (b3 as u64)
            }
            8 => {
                let bytes = &self.buf[self.pos..self.pos + 8];
                let mut arr = [0u8; 8];
                arr.copy_from_slice(bytes);
                arr[0] &= 0x3F;
                self.pos += 8;
                u64::from_be_bytes(arr)
            }
            _ => unreachable!(),
        };

        Ok(VarInt(val))
    }
}

/// Zero-allocation, bounds-checked binary cursor writer for QUIC wire formats.
#[derive(Debug)]
pub struct BufferWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufferWriter<'a> {
    /// Create a new `BufferWriter` over the given mutable byte slice.
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current write position (bytes written so far).
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Number of bytes written so far.
    #[inline]
    pub fn written_len(&self) -> usize {
        self.pos
    }

    /// Total capacity of the buffer.
    #[inline]
    pub fn total_capacity(&self) -> usize {
        self.buf.len()
    }

    /// Remaining write capacity in the buffer.
    #[inline]
    pub fn remaining_capacity(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Slice of the bytes written so far.
    #[inline]
    pub fn written_slice(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    /// Mutable slice of the remaining unwritten buffer.
    #[inline]
    pub fn remaining_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    /// Write an exact byte slice into the buffer.
    pub fn write_exact(&mut self, data: &[u8]) -> Result<(), CodecError> {
        if self.remaining_capacity() < data.len() {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: data.len(),
                available: self.remaining_capacity(),
            });
        }
        let end = self.pos + data.len();
        self.buf[self.pos..end].copy_from_slice(data);
        self.pos = end;
        Ok(())
    }

    /// Write a single `u8`.
    pub fn write_u8(&mut self, val: u8) -> Result<(), CodecError> {
        if self.remaining_capacity() < 1 {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: 1,
                available: 0,
            });
        }
        self.buf[self.pos] = val;
        self.pos += 1;
        Ok(())
    }

    /// Write a big-endian `u16`.
    pub fn write_be_u16(&mut self, val: u16) -> Result<(), CodecError> {
        if self.remaining_capacity() < 2 {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: 2,
                available: self.remaining_capacity(),
            });
        }
        let bytes = val.to_be_bytes();
        self.buf[self.pos..self.pos + 2].copy_from_slice(&bytes);
        self.pos += 2;
        Ok(())
    }

    /// Write a big-endian 24-bit integer from the lower 24 bits of a `u32`.
    pub fn write_be_u24(&mut self, val: u32) -> Result<(), CodecError> {
        if self.remaining_capacity() < 3 {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: 3,
                available: self.remaining_capacity(),
            });
        }
        let bytes = val.to_be_bytes();
        self.buf[self.pos..self.pos + 3].copy_from_slice(&bytes[1..4]);
        self.pos += 3;
        Ok(())
    }

    /// Write a big-endian `u32`.
    pub fn write_be_u32(&mut self, val: u32) -> Result<(), CodecError> {
        if self.remaining_capacity() < 4 {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: 4,
                available: self.remaining_capacity(),
            });
        }
        let bytes = val.to_be_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&bytes);
        self.pos += 4;
        Ok(())
    }

    /// Write a big-endian `u64`.
    pub fn write_be_u64(&mut self, val: u64) -> Result<(), CodecError> {
        if self.remaining_capacity() < 8 {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: 8,
                available: self.remaining_capacity(),
            });
        }
        let bytes = val.to_be_bytes();
        self.buf[self.pos..self.pos + 8].copy_from_slice(&bytes);
        self.pos += 8;
        Ok(())
    }

    /// Write a QUIC variable-length integer (`VarInt`).
    ///
    /// Returns the number of bytes written.
    pub fn write_varint(&mut self, val: VarInt) -> Result<usize, CodecError> {
        let len = val.encoded_len();
        if self.remaining_capacity() < len {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: len,
                available: self.remaining_capacity(),
            });
        }
        let bytes_written = val.encode(&mut self.buf[self.pos..self.pos + len])?;
        self.pos += bytes_written;
        Ok(bytes_written)
    }

    /// Fill `count` bytes with a repeated byte value (e.g. for PADDING).
    pub fn fill(&mut self, byte: u8, count: usize) -> Result<(), CodecError> {
        if self.remaining_capacity() < count {
            return Err(CodecError::BufferTooShort {
                offset: self.pos,
                needed: count,
                available: self.remaining_capacity(),
            });
        }
        let end = self.pos + count;
        self.buf[self.pos..end].fill(byte);
        self.pos = end;
        Ok(())
    }
}
