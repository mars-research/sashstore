//! A safe key--value store (sashstore)

#![forbid(unsafe_code)]
#![cfg_attr(all(target_os = "redshift"), no_std)]

extern crate alloc;
use alloc::collections::VecDeque;

use log::trace;

mod andreamap;
mod arch;
mod resp;

use andreamap::ResizingHashMap;

use arch::PlatformSupport;
use resp::{serialize::encode_with_buf, serialize::Decoder, value::Value};

pub struct SashStore {
    map: ResizingHashMap<String>,
}

impl SashStore {
    pub fn with_capacity(cap: usize) -> Self {
        SashStore {
            map: ResizingHashMap::new(cap),
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
            Err(_e) => Value::StaticError("Couldn't parse request"),
        };
        encode_with_buf(decoder.into(), &response)
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd(&mut self, cmd: Value) -> Value {
        if let Value::Array(cmd_elements) = cmd {
            trace!("CMD elements are {:?}", cmd_elements);
            if cmd_elements.len() >= 2 {
                if let Value::Bulk(opcode) = &cmd_elements[0] {
                    match opcode.as_str() {
                        "GET" => {
                            if let Value::Bulk(key) = &cmd_elements[1] {
                                match key.trim_start_matches("key:").parse::<u64>() {
                                    Ok(num) => {
                                        trace!("Execute .get for {}", num);
                                        let r = self.map.get(num);
                                        r.map_or(Value::Null, |v| {
                                            Value::Array(vec![Value::String(v.clone())])
                                        })
                                    }
                                    Err(_e) => Value::StaticError("GET key is not an u64."),
                                }
                            } else {
                                Value::StaticError("GET key is not type bulk?")
                            }
                        }
                        "SET" => {
                            if cmd_elements.len() != 3 {
                                return Value::StaticError(
                                    "wrong number of arguments for 'set' command",
                                );
                            }
                            if let (Value::Bulk(key), Value::Bulk(val)) =
                                (&cmd_elements[1], &cmd_elements[2])
                            {
                                trace!("Set for {} {}", key, val);
                                self.map.insert(0, val.to_string());
                                Value::Array(vec![Value::Bulk(String::from("OK"))])
                            } else {
                                Value::StaticError("SET key/val are not type bulk?")
                            }
                        }
                        _ => Value::Error(format!("Unsupported command {}", opcode.as_str())),
                    }
                } else {
                    Value::StaticError("Command: first argument is not type Bulk?")
                }
            } else {
                Value::StaticError("Command: array has less than 2 elements?")
            }
        } else {
            Value::StaticError("Command is not an array?")
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
