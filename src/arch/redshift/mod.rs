use alloc::vec::Vec;

use super::{CmdArgs, CpuId, NumaTopology, PlatformSupport, ThreadId, Transport};
use crate::SashStore;

pub struct Platform;

impl PlatformSupport for Platform {
    /// Return a set of core ids that you want threads to be spawned on.
    // The Ids will later be passed to spawn and also to `arch::pin_thread`.
    fn allocate_cores(&mut self, _how_many: usize, _strategy: NumaTopology) -> Vec<CpuId> {
        vec![1, 2, 3, 4]
    }

    /// Do something to make the log crate work, initialize a logger.
    fn init_logging(&mut self) {
        /* NOP */
    }

    /// Return command-line arguments.
    ///
    /// Can either parse them somehow or just return a static struct...
    fn parse_args(&mut self) -> CmdArgs {
        CmdArgs {
            threads: 4,
            capacity: 10000,
            numa_strategy: NumaTopology::Interleave,
            transport: Transport::Udp,
            tcp_connections_per_port: 1,
            port: 6666,
        }
    }

    /// Spawn a thread that executes `f` on core id `on_core`.
    /// The `on_core` parameter will be one of the elements
    /// that has been returned by `allocate_cores`.
    ///
    /// Returns ThreadId for the newly spawned thread.
    fn spawn<F>(&mut self, _f: F, _on_core: CpuId) -> ThreadId
    where
        F: FnOnce() -> ThreadId,
        F: Send + 'static,
    {
        unimplemented!("spawn")
    }

    /// Waits until the thread identified by `tid` is done.
    fn join(&mut self, _tid: ThreadId) {
        unimplemented!("join")
    }
}

/// The per-core server loop it's called on every spawned thread.
///
/// 1. It should open a connection on port `cmd.port` + `tid`
/// 2. Listen for incoming messages
/// 3. Give message to `SashStore::handle_network_request`
/// 4. Send result of `SashStore::handle_network_request` back to client
#[allow(unused)]
pub fn server_loop(core: CpuId, tid: ThreadId, config: &CmdArgs, kvstore: &mut SashStore) {
    unimplemented!("server_loop")
}
