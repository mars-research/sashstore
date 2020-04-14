use alloc::collections::VecDeque;
use alloc::vec::Vec;

use log::trace;

use super::DecodeError;
use super::Value;

/// Encodes memcached Value to a binary buffer.
///
/// Avoids allocating the buffer by passing an existing one in.
pub fn encode_with_buf(mut res: Vec<u8>, value: &Value) -> Vec<u8> {
    buf_encode(value, &mut res);
    res
}

/// Encodes memcached Value to binary buffer.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::new();
    buf_encode(value, &mut res);
    res
}

/// Encode return value:
///
/// After sending the command line and the data block the client awaits
/// the reply, which may be:
/// - "STORED\r\n", to indicate success.
/// - "NOT_STORED\r\n" to indicate the data was not stored, but not
/// because of an error. This normally means that the
/// condition for an "add" or a "replace" command wasn't met.
///
/// For GET:
/// Each item sent by the server looks like this
///
/// VALUE <key> <flags> <bytes> [<cas unique>]\r\n
/// <data block>\r\n
///
/// - <key> is the key for the item being sent
/// - <flags> is the flags value set by the storage command
/// - <bytes> is the length of the data block to follow, *not* including
/// its delimiting \r\n
/// - <cas unique> is a unique 64-bit integer that uniquely identifies
/// this specific item.
/// - <data block> is the data for this item.
///
/// If some of the keys appearing in a retrieval request are not sent back
/// by the server in the item list this means that the server does not
/// hold items with such keys (because they were never stored, or stored
/// but deleted to make space for more items, or expired, or explicitly
///
/// deleted by a client).
#[inline]
fn buf_encode(value: &Value, buf: &mut Vec<u8>) {
    match value {
        Value::Get(_) => unreachable!("We shouldn't return that to the clients"),
        Value::Set(_, _) => unreachable!("We shouldn't return that to the clients"),
        Value::Value(k, v, flags) => {
            buf.extend_from_slice("VALUE ".as_bytes());
            buf.extend_from_slice(k.as_slice());
            buf.extend_from_slice(" ".as_bytes());
            buf.extend_from_slice(flags.as_slice());
            buf.extend_from_slice(format!(" {}\r\n", v.len()).as_bytes());
            buf.extend_from_slice(v.as_slice());
            buf.extend_from_slice("\r\n".as_bytes());
        }
        Value::Stored => buf.extend_from_slice("STORED\r\n".as_bytes()),
        Value::NotStored => buf.extend_from_slice("NOT_STORED\r\n".as_bytes()),
        _ => buf.extend_from_slice("NOT_STORED\r\n".as_bytes()),
    }
}

/// A streaming memcached Decoder.
#[derive(Debug)]
pub struct Decoder {
    buf_bulk: bool,
    reader: VecDeque<u8>,
}

impl Into<Vec<u8>> for Decoder {
    fn into(self) -> Vec<u8> {
        self.reader.into()
    }
}

impl Decoder {
    /// Creates a Decoder instance with given VecDequeue for decoding the memcached packets.
    pub fn new(reader: VecDeque<u8>) -> Self {
        Decoder {
            buf_bulk: false,
            reader: reader,
        }
    }

    pub fn with_buf_bulk(reader: VecDeque<u8>) -> Self {
        Decoder {
            buf_bulk: true,
            reader: reader,
        }
    }

    // Conversion of self.reader.read_exact(buf.as_mut_slice())?;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), DecodeError> {
        for i in 0..buf.len() {
            match self.reader.pop_front() {
                None => return Err(DecodeError::UnexpectedEof),
                Some(c) => buf[i] = c,
            }
        }
        Ok(())
    }

    // Conversion of: self.reader.read_until(b'\n', &mut res)?;
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> usize {
        let mut popped = 0;
        loop {
            match self.reader.pop_front() {
                None => return popped,
                Some(c) => {
                    popped += 1;
                    buf.push(c);

                    if c == byte {
                        return popped;
                    }
                }
            }
        }
    }

    fn skip_until_newline(&mut self) -> usize {
        let mut popped = 0;
        let mut seen_carriage_return = false;
        loop {
            match self.reader.pop_front() {
                None => return popped,
                Some(c) => {
                    popped += 1;

                    if c as char == '\r' {
                        seen_carriage_return = true;
                        continue;
                    }

                    if c as char == '\n' && seen_carriage_return {
                        return popped;
                    } else {
                        seen_carriage_return = false;
                    }
                }
            }
        }
    }

    /// It will read buffers from the inner BufReader, and return a Value
    pub fn decode(&mut self) -> Result<Value, DecodeError> {
        for _i in 0..8 {
            // The frame header is 8 bytes long, as follows (all values are 16-bit integers
            // in network byte order, high byte first):
            // 0-1 Request ID
            // 2-3 Sequence number
            // 4-5 Total number of datagrams in this message
            // 6-7 Reserved for future use; must be 0
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?;
        }

        let mut buffer = Vec::with_capacity(12);
        self.read_until(' ' as u8, &mut buffer);
        let op =
            core::str::from_utf8(&buffer.as_slice()).map_err(|_e| DecodeError::InvalidOpCode)?;

        // parse opcode
        match op {
            "set " => {
                // https://github.com/memcached/memcached/blob/master/doc/protocol.txt#L199
                log::trace!("Set");

                let mut key_buf = Vec::with_capacity(65);
                self.read_until(' ' as u8, &mut key_buf);
                key_buf.pop();
                trace!("got key: {:?}", key_buf);
                debug_assert_eq!(key_buf.len(), 64, "Sanity check parsing key");

                self.skip_until_newline();

                let mut val_buf = Vec::with_capacity(8);
                self.read_until('\r' as u8, &mut val_buf);
                val_buf.pop(); // remove \r
                trace!("got val: {:?}", val_buf);
                debug_assert_eq!(val_buf.len(), 8, "Sanity check parsing value");

                Ok(Value::Set(key_buf, val_buf))
            }
            "get " => {
                log::trace!("Get");

                let mut key_buf = Vec::with_capacity(65);
                self.read_until(' ' as u8, &mut key_buf);
                key_buf.pop();
                trace!("got key: {:?}", key_buf);
                debug_assert_eq!(key_buf.len(), 64, "Sanity check parsing key");

                self.skip_until_newline();

                Ok(Value::Get(key_buf))
            }
            _ => return Err(DecodeError::InvalidOpCode),
        }
    }
}
