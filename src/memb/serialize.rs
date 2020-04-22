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

fn u32_to_slice(x: u32) -> [u8; 4] {
    let b1: u8 = ((x >> 24) & 0xff) as u8;
    let b2: u8 = ((x >> 16) & 0xff) as u8;
    let b3: u8 = ((x >> 8) & 0xff) as u8;
    let b4: u8 = (x & 0xff) as u8;
    return [b1, b2, b3, b4];
}

fn slice_to_u32(x: &[u8]) -> u32 {
    let b1: u32 = ((x[0] as u32 >> 24) & 0xff);
    let b2: u32 = ((x[1] as u32 >> 16) & 0xff);
    let b3: u32 = ((x[2] as u32 >> 8) & 0xff);
    let b4: u32 = (x[3] as u32 & 0xff);
    return b1 | b2 | b3 | b4;
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
    buf.clear();

    match value {
        Value::Get(_req_id, _key) => unreachable!("We shouldn't return that to the clients"),
        Value::Set(req_id, _, _, _) => unreachable!("We shouldn't return that to the clients"),
        Value::Value(request_id, k, flags, v) => {
            // Construct UDP header
            buf.extend_from_slice(&u16::to_be_bytes(*request_id));
            buf.extend_from_slice(&u16::to_be_bytes(0)); // seq number
            buf.extend_from_slice(&u16::to_be_bytes(1)); // #datagram
            buf.extend_from_slice(&u16::to_be_bytes(0)); // reserved
            buf.extend_from_slice(b"VALUE ");
            buf.extend_from_slice(k.as_slice());
            buf.extend_from_slice(" ".as_bytes());
            buf.extend_from_slice(format!(" {}", *flags).as_bytes());
            buf.extend_from_slice(format!(" {}\r\n", v.len()).as_bytes());
            buf.extend_from_slice(v.as_slice());
            buf.extend_from_slice(b" END\r\n");
        }
        Value::Stored(request_id) => {
            buf.extend_from_slice(&u16::to_be_bytes(*request_id));
            buf.extend_from_slice(&u16::to_be_bytes(0)); // seq number
            buf.extend_from_slice(&u16::to_be_bytes(1)); // #datagram
            buf.extend_from_slice(&u16::to_be_bytes(0)); // reserved
            buf.extend_from_slice(b"STORED\r\n")
        }
        Value::NotStored(request_id) => {
            buf.extend_from_slice(&u16::to_be_bytes(*request_id));
            buf.extend_from_slice(&u16::to_be_bytes(0)); // seq number
            buf.extend_from_slice(&u16::to_be_bytes(1)); // #datagram
            buf.extend_from_slice(&u16::to_be_bytes(0)); // reserved
            buf.extend_from_slice(b"NOT_STORED\r\n")
        }
        _ => unreachable!("Unexpected response"),
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
        // The frame header is 8 bytes long, as follows (all values are 16-bit integers
        //     in network byte order, high byte first):
        //
        // 0-1 Request ID
        // 2-3 Sequence number
        // 4-5 Total number of datagrams in this message
        // 6-7 Reserved for future use; must be 0
        //
        // The request ID is supplied by the client. Typically it will be a
        // monotonically increasing value starting from a random seed, but the client
        // is free to use whatever request IDs it likes. The server's response will
        // contain the same ID as the incoming request. The client uses the request ID
        // to differentiate between responses to outstanding requests if there are
        // several pending from the same server; any datagrams with an unknown request
        // ID are probably delayed responses to an earlier request and should be
        // discarded.
        //
        // The sequence number ranges from 0 to n-1, where n is the total number of
        // datagrams in the message. The client should concatenate the payloads of the
        // datagrams for a given response in sequence number order; the resulting byte
        // stream will contain a complete response in the same format as the TCP
        // protocol (including terminating \r\n sequences).

        // 0-1 Request ID
        let mut buf = [
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
        ];
        let request_id = u16::from_be_bytes(buf);
        // 2-3 Sequence number
        let mut buf = [
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
        ];
        let sequence_nr = u16::from_be_bytes(buf);
        debug_assert_eq!(sequence_nr, 0, "No multi-packet means this is 0");

        // 4-5 Total number of datagrams in this message
        let mut buf = [
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
        ];
        let datagram_total = u16::from_be_bytes(buf);
        debug_assert_eq!(datagram_total, 1, "Don't support multi-packet");

        // 6-7 Reserved for future use; must be 0
        let mut buf = [
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
            self.reader.pop_front().ok_or(DecodeError::UnexpectedEof)?,
        ];
        let reserved = u16::from_be_bytes(buf);
        //debug_assert_eq!(reserved, 0);

        log::info!(
            "request_id = {} sequence_nr = {} datagram_total= {} reserved = {}",
            request_id,
            sequence_nr,
            datagram_total,
            reserved
        );

        let mut buffer = Vec::with_capacity(12);
        self.read_until(' ' as u8, &mut buffer);
        let op = buffer.as_slice();

        // parse opcode
        match op {
            b"set " => {
                // https://github.com/memcached/memcached/blob/master/doc/protocol.txt#L199
                log::trace!("Set");

                let mut key_buf = Vec::with_capacity(65);
                self.read_until(' ' as u8, &mut key_buf);
                key_buf.pop();
                trace!("got key: {:?}", key_buf);

                let mut flag_buf = Vec::with_capacity(4);
                self.read_until(' ' as u8, &mut flag_buf);
                debug_assert!(flag_buf.len() <= 4);
                flag_buf.resize(4, 0);
                let flags = slice_to_u32(flag_buf.as_slice());

                self.skip_until_newline();

                let mut val_buf = Vec::with_capacity(8);
                self.read_until('\r' as u8, &mut val_buf);
                val_buf.pop(); // remove \r
                trace!("got val: {:?}", val_buf);

                Ok(Value::Set(request_id, key_buf, flags, val_buf))
            }
            b"get " => {
                log::trace!("Get");

                let mut key_buf = Vec::with_capacity(65);
                self.read_until(' ' as u8, &mut key_buf);
                key_buf.pop();
                key_buf.pop(); // \r ?
                trace!("got key: {:?}", key_buf);
                //debug_assert_eq!(key_buf.len(), 64, "Sanity check parsing key");

                self.skip_until_newline();

                Ok(Value::Get(request_id, key_buf))
            }
            _ => return Err(DecodeError::InvalidOpCode),
        }
    }
}
