//! A safe key--value store (sashstore)
#![forbid(unsafe_code)]
#![feature(test)]
#![cfg_attr(all(target_os = "redshift"), no_std)]

extern crate alloc;

#[cfg(test)]
extern crate test;

use alloc::collections::VecDeque;

use log::trace;

mod arch;
mod indexmap;

mod memb;

use arch::PlatformSupport;
use memb::{serialize::encode_with_buf, serialize::Decoder, Value};

pub struct SashStore {
    map: indexmap::Index<Vec<u8>, Vec<u8>>,
}

impl SashStore {
    /// Initialize a new SashStore instance.
    fn with_capacity(cap: usize) -> Self {
        SashStore {
            map: indexmap::Index::with_capacity(cap),
        }
    }

    /// Execute the content of a packet buffer in our KV store.
    pub fn handle_network_request(&mut self, buf: Vec<u8>) -> Vec<u8> {
        let reader = VecDeque::from(buf);
        let mut decoder = Decoder::new(reader);
        let response = match decoder.decode() {
            Ok(value) => {
                trace!("Received value={:#?}", value);
                self.execute_cmd(value)
            }
            Err(e) => panic!("Couldn't parse request {:?}", e),
        };
        encode_with_buf(decoder.into(), &response)
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd(&mut self, cmd: Value) -> Value {
        match cmd {
            Value::Get(key) => {
                trace!("Execute .get for {:?}", key);
                let r = self.map.get(&key);
                match r {
                    Some(val) => Value::Value(key, val.to_vec(), vec![0x0, 0x0, 0x0, 0x0]),
                    None => Value::NoReply,
                }
            }
            Value::Set(key, value) => {
                trace!("Set for {:?} {:?}", key, value);
                self.map.insert(key, value);
                Value::Stored
            }
            _ => unreachable!(),
        }
    }
}

fn main() {
    // Get platform abstraction layer
    let mut platform = arch::get_platform();

    // Parse configuration and figure out core assignment
    let cmd = platform.parse_args();
    platform.init_logging();
    let cores = platform.allocate_cores(cmd.threads, cmd.numa_strategy);

    // Spawn threads on cores
    let mut tids = Vec::with_capacity(cmd.threads);
    for (idx, core) in cores.into_iter().enumerate() {
        let tid = platform.spawn(
            move || {
                trace!("Worker thread says hi from core {}.", core);
                let mut map: SashStore = SashStore::with_capacity(cmd.capacity);
                arch::arch::server_loop(core, idx, &cmd, &mut map);
                0
            },
            core,
        );
        tids.push(tid);
    }

    // Wait till server is done (it's never done, just use Ctrl+c)
    for tid in tids {
        platform.join(tid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    const SET_CMD_CHAR: [char; 50] = [
        '*', '3', '\r', '\n', '$', '3', '\r', '\n', 'S', 'E', 'T', '\r', '\n', '$', '1', '6', '\r',
        '\n', 'k', 'e', 'y', ':', '0', '0', '0', '0', '0', '0', '0', '0', '6', '6', '3', '0', '\r',
        '\n', '$', '8', '\r', '\n', 'x', 'x', 'x', 'x', 'x', 'x', 'x', 'x', '\r', '\n',
    ];

    const GET_CMD_CHAR: [char; 36] = [
        '*', '2', '\r', '\n', '$', '3', '\r', '\n', 'G', 'E', 'T', '\r', '\n', '$', '1', '6', '\r',
        '\n', 'k', 'e', 'y', ':', '0', '0', '0', '0', '0', '0', '0', '0', '4', '9', '8', '2', '\r',
        '\n',
    ];

    #[bench]
    fn bench_set_requests(b: &mut Bencher) {
        let mut kv = SashStore::with_capacity(10_000);
        b.iter(|| {
            let buf: Vec<u8> = SET_CMD_CHAR.iter().map(|c| *c as u8).collect();
            let _r = kv.handle_network_request(buf);
        });
    }

    #[bench]
    fn bench_get_requests(b: &mut Bencher) {
        let mut kv = SashStore::with_capacity(10_000);
        b.iter(|| {
            let buf: Vec<u8> = GET_CMD_CHAR.iter().map(|c| *c as u8).collect();
            let _r = kv.handle_network_request(buf);
        });
    }
}
