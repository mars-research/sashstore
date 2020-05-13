use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::format;

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
    pub reader: Vec<u8>,
    read_until: usize,
}

/* impl Into<Vec<u8>> for Decoder {
    fn into(self) -> Vec<u8> {
        self.reader.into()
    }
} */

impl Decoder {
    /// Creates a Decoder instance with given VecDequeue for decoding the memcached packets.
    pub fn new(reader: Vec<u8>) -> Self {
        Decoder {
            buf_bulk: false,
            reader: reader,
            read_until: 0,
        }
    }

    pub fn with_buf_bulk(reader: Vec<u8>) -> Self {
        Decoder {
            buf_bulk: true,
            reader: reader,
            read_until: 0,
        }
    }

    // Conversion of self.reader.read_exact(buf.as_mut_slice())?;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), DecodeError> {
        assert!(buf.len() <= self.reader.len() - self.read_until);
        let old_until = self.read_until;
        for i in 0..buf.len() {
            buf[i] = self.reader[old_until + i];
        }
        self.read_until += buf.len();
        Ok(())
    }

    // Conversion of: self.reader.read_until(b'\n', &mut res)?;
    fn read_until(&mut self, byte: u8)  -> & [u8] {
        let old_until = self.read_until;
        trace!("read_until {}", old_until);
        for i in self.read_until..self.reader.len() {
            if self.reader[i] == byte {
                self.read_until = i+1;
                return &self.reader.as_slice()[old_until..i+1];
            } else {
                continue;
            }
        }
        &[]
    }

    fn skip_until_newline(&mut self) {
        let old_until = self.read_until;
        let cur_buf = self.reader.split_at(self.read_until).1;
        trace!("cur_buf.len {}", cur_buf.len());

        for (i, el) in cur_buf.windows(2).enumerate() {
            //trace!("{:?}", el); 
            if el == &[b'\r', b'\n'] {
                self.read_until = old_until + i + 3;  
                trace!("read_until {}", self.read_until);
                break;
            }
        }
    }

    fn set_until(&mut self, until: usize) {
        self.read_until = until;
    }

    /// It will read buffers from the inner BufReader, and return a Value
    ///
    /// Mostly info from here
    /// https://github.com/memcached/memcached/blob/master/doc/protocol.txt#L199
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

        trace!("buflen {}", self.reader.len());
        assert!(self.reader.len() >= 8);
        // 0-1 Request ID
        let mut buf = [
            self.reader[0],
            self.reader[1],
        ];
        let request_id = u16::from_be_bytes(buf);
        // 2-3 Sequence number
        let mut buf = [
            self.reader[2],
            self.reader[3],
        ];
        let sequence_nr = u16::from_be_bytes(buf);
        debug_assert_eq!(sequence_nr, 0, "No multi-packet means this is 0");

        // 4-5 Total number of datagrams in this message
        let mut buf = [
            self.reader[4],
            self.reader[5],
        ];
        let datagram_total = u16::from_be_bytes(buf);
        debug_assert_eq!(datagram_total, 1, "Don't support multi-packet");

        // 6-7 Reserved for future use; must be 0
        let mut buf = [
            self.reader[6],
            self.reader[7],
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

        {
            self.set_until(8);
        }

        let op = self.read_until(' ' as u8);

        trace!("op {:?}", op);
        // parse opcode
        match op {
            b"set " => {
                let mut key_buf = Vec::with_capacity(65);

                {
                    key_buf = self.read_until(' ' as u8).to_vec();
                    key_buf.pop();
                }

                trace!("got key: {:?}", String::from_utf8(key_buf.clone()));

                let mut flags = 0u32;
                {
                    let flag_buf = {
                        self.read_until(' ' as u8)
                    };
                    debug_assert!(flag_buf.len() <= 4);
                    let mut _flag = [0u8; 4];

                    if flag_buf.len() < 4 {
                        for (i, e) in flag_buf.iter().enumerate() {
                            if *e != b' ' {
                                _flag[i] = *e;
                            }
                        }
                    }

                    flags = u32::from_be_bytes(_flag);
                }

                trace!("flags {:x}", flags);

                self.skip_until_newline();

                let mut val_buf = self.read_until('\r' as u8).to_vec();
                val_buf.pop();

                trace!("got val: {:?}", String::from_utf8(val_buf.clone()));

                Ok(Value::Set(request_id, key_buf, flags, val_buf))
            }
            b"get " => {
                log::trace!("Get");

                let mut key_buf = Vec::with_capacity(65);

                key_buf = self.read_until('\r' as u8).to_vec();
                key_buf.pop();

                trace!("got key: {:?}", key_buf);

                Ok(Value::Get(request_id, key_buf))
            }
            _ => return Err(DecodeError::InvalidOpCode),
        }
    }
}
