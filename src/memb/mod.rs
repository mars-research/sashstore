//! Parse the memcached binary format
//!
//! Based on the description provided here:
//! https://github.com/memcached/memcached/wiki/BinaryProtocolRevamped
//!
//! and a bit of `tcpdump -i lo udp port 11211 -vv -X`

#![allow(unused)] // For now

use alloc::vec::Vec;
use core::cell::Ref;

pub mod serialize;

// Let's separate the vocabularies of client and server to make
// reasoning about lifetimes easier. Here ClientValue<'req> will
// contain references to the original request, whereas
// ServerValue<'kv> will refer to the Index.

/// Data format description for a parsed packet from the client
#[derive(Debug)]
pub enum ClientValue<'req> {
    Get(u16, &'req [u8]),
    Set(u16, &'req [u8], u32, &'req [u8]),
}

/// Data format description for a packet to be sent out
pub enum ServerValue<'kv> {
    // (seq, key, key_len, val_ref)
    Value(u16, [u8; 250], usize, Ref<'kv, (u32, Vec<u8>)>),
    Stored(u16),
    NotStored(u16),
    NoReply,
}

/// A decoder error
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DecodeError {
    InvalidOpCode,
    UnexpectedEof,
}
