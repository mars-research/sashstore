//! A safe key--value store (sashstore)
#![forbid(unsafe_code)]
#![feature(test)]
#![cfg_attr(all(target_os = "redshift"), no_std)]

extern crate alloc;

#[cfg(test)]
extern crate test;

use log::trace;

mod arch;
mod indexmap;

mod memb;

use arch::PlatformSupport;
use memb::{serialize::encode_with_buf, serialize::Decoder, Value};

#[cfg(target_os = "linux")]
use jemallocator::Jemalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub struct SashStore {
    /// Maps key -> (flags, value)
    map: indexmap::Index<Vec<u8>, (u32, Vec<u8>)>,
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
        //let reader = VecDeque::from(buf);
        //println!("<= req_buf {:x?} {}", buf.as_ptr(), buf.len());
        let mut decoder = Decoder::new(buf);
        let response = match decoder.decode() {
            Ok(value) => {
                trace!("Received value={:?}", value);
                self.execute_cmd(value)
            }
            Err(e) => panic!("Couldn't parse request {:?}", e),
        };
        let resp_buf = encode_with_buf(decoder.reader, &response);
        //println!("=> resp_buf {:x?} {}", resp_buf.as_ptr(), resp_buf.len());
        resp_buf
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd(&mut self, cmd: Value) -> Value {
        match cmd {
            Value::Get(req_id, key) => {
                trace!("Execute .get for {:?}", key);
                let r = self.map.get(&key);
                match r {
                    Some(value) => Value::Value(req_id, key, value.0, value.1.to_vec()),
                    None => {
                        unreachable!("didn't find value for key {:?}", key);
                        //Value::NoReply
                    }
                }
            }
            Value::Set(req_id, key, flags, value) => {
                trace!("Set for {:?} {:?}", key, value);
                self.map.insert(key, (flags, value));
                Value::Stored(req_id)
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
