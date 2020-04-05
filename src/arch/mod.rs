use alloc::vec::Vec;
use core::fmt;

/// Linux specific code,
#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
pub mod arch;

/// Redshift specific code,
#[cfg(target_os = "redshift")]
#[path = "redshift/mod.rs"]
pub mod arch;

#[cfg(not(target_os = "redshift"))]
#[path = "redshift/mod.rs"]
pub mod redshift_compile_check;

pub type Socket = usize;
pub type CpuId = usize;
pub type ThreadId = usize;

/// Control settings of the KV server.
///
/// Get this struct through `PlatformSupport.parse_args`
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct CmdArgs {
    /// How many threads/cores to allocate.
    pub threads: usize,

    /// How to allocate threads on the system.
    pub numa_strategy: NumaTopology,

    /// Initial hash-map capacity.
    pub capacity: usize,

    /// Transport layer
    transport: Transport,

    /// In case of TCP transport, how many connections we expect per port
    tcp_connections_per_port: usize,

    /// Start port address
    port: usize,
}

pub trait PlatformSupport {
    fn allocate_cores(&mut self, how_many: usize, strategy: NumaTopology) -> Vec<CpuId>;
    fn init_logging(&mut self);
    fn parse_args(&mut self) -> CmdArgs;
    fn spawn<F>(&mut self, f: F, on_core: CpuId) -> ThreadId
    where
        F: FnOnce() -> ThreadId,
        F: Send + 'static;
    fn join(&mut self, tid: ThreadId);
}

pub fn get_platform() -> impl PlatformSupport {
    arch::Platform::default()
}

/// Defines the strategy how threads are allocated in the system.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum NumaTopology {
    /// Allocate threads on the same socket (as much as possible before going to the next).
    Sequential,
    /// Spread thread allocation out across sockets.
    Interleave,
}

impl fmt::Display for NumaTopology {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NumaTopology::Sequential => write!(f, "Sequential"),
            NumaTopology::Interleave => write!(f, "Interleave"),
        }
    }
}

impl fmt::Debug for NumaTopology {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NumaTopology::Sequential => write!(f, "TM=Sequential"),
            NumaTopology::Interleave => write!(f, "TM=Interleave"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Transport {
    Udp,
    Tcp,
}
