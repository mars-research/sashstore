//! A safe key--value store (sashstore)
//#![forbid(unsafe_code)]
#![feature(test)]
#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use arrayvec::ArrayVec;

use log::trace;
use console::println;

pub mod indexmap;

mod memb;

use memb::{serialize::buf_encode, serialize::Decoder, ClientValue, ServerValue};
use fnv::FnvHasher;

type FnvHashFactory = BuildHasherDefault<FnvHasher>;

pub type KVKey = ArrayVec<[u8; 64]>;
pub type KVVal = (u32, ArrayVec<[u8; 1024]>);

pub struct SashStore {
    /// Maps key -> (flags, value)
    map: indexmap::Index<KVKey, KVVal, FnvHashFactory>,
}

impl SashStore {
    /// Initialize a new SashStore instance.
    pub fn with_capacity(capacity: usize) -> Self {
        const DEFAULT_MAX_LOAD: f64 = 0.7;
        const DEFAULT_GROWTH_POLICY: f64 = 2.0;
        const DEFAULT_PROBING: fn(usize, usize) -> usize = |hash, i| hash + i + i * i;
        
        SashStore {
            map: indexmap::Index::with_capacity_and_parameters(
                capacity,
                indexmap::Parameters {
                    max_load: DEFAULT_MAX_LOAD,
                    growth_policy: DEFAULT_GROWTH_POLICY,
                    hasher_builder: Default::default(),
                    probe: DEFAULT_PROBING,
                },
            )
        }
    }

    pub fn print_stats(&self) {
        println!("capacity={}, len={}", self.map.capacity(), self.map.len());
    }

    /// Execute the content of a packet buffer in our KV store.
    pub fn handle_network_request(&mut self, buf: Vec<u8>) -> Vec<u8> {
        //let reader = VecDeque::from(buf);
        //println!("<= req_buf {:x?} {}", buf.as_ptr(), buf.len());
        let mut decoder = Decoder::new(buf);
        let response = match decoder.decode() {
            Ok(value) => {
                // trace!("Received value={:?}", value);
                self.execute_cmd(value)
            }
            Err(e) => panic!("Couldn't parse request {:?}", e),
        };
        let mut buf = decoder.destroy();
        buf_encode(&response, &mut buf);
        //println!("=> resp_buf {:x?} {}", resp_buf.as_ptr(), resp_buf.len());
        buf
    }

    /// Execute a parsed command against our KV store
    fn execute_cmd<'req, 'kv>(&'kv mut self, cmd: ClientValue<'req>) -> ServerValue<'kv> {
        match cmd {
            ClientValue::Get(req_id, key) => {
                trace!("Execute .get for {:?}", key);

                if key.len() > 64 {
                    // Illegal key
                    panic!("key too long");
                    return ServerValue::NoReply;
                }

                let r = self.map.get(key);
                let mut ret;
                match r {
                    Some(value) => {
                        // one copy here
                        let mut key_vec = ArrayVec::new();
                        key_vec.try_extend_from_slice(key).expect("Key too long");
                        ret = ServerValue::Value(req_id, key_vec, value)
                    },
                    None => {
                        unreachable!("didn't find value for key {:?}", key);
                        ret = ServerValue::NoReply
                    },
                }

                let end = unsafe { core::arch::x86_64::_rdtsc() };
                // println!("get took {:?}", end - start);

                ret
            }
            ClientValue::Set(req_id, key, flags, value) => {
                trace!("Set for {:?} {:?}", key, value);
                let start = unsafe { core::arch::x86_64::_rdtsc() };

                let r = if key.len() <= 250 {
                    let mut key_vec: ArrayVec<[u8; 64]> = ArrayVec::new();
                    let mut value_vec: ArrayVec<[u8; 1024]> = ArrayVec::new();

                    key_vec.try_extend_from_slice(&key).expect("rua");
                    value_vec.try_extend_from_slice(&value).expect("rua");

                    self.map.insert(key_vec, (flags, value_vec));
                    ServerValue::Stored(req_id)
                } else {
                    ServerValue::NotStored(req_id)
                };

                let end = unsafe { core::arch::x86_64::_rdtsc() };
                // println!("set took {:?}", end - start);

                r
            }
            _ => unreachable!(),
        }
    }
}
