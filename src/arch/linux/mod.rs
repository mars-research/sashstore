use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::collections::HashMap;
use std::thread::JoinHandle;

use clap::{crate_version, value_t, App, Arg};

use super::{CmdArgs, CpuId, NumaTopology, PlatformSupport, ThreadId, Transport};

mod net;
mod topology;

pub use net::server_loop;

#[derive(Default)]
pub struct Platform {
    tid_count: ThreadId,
    handles: HashMap<ThreadId, JoinHandle<()>>,
}

impl PlatformSupport for Platform {
    fn allocate_cores(&mut self, how_many: usize, strategy: NumaTopology) -> Vec<CpuId> {
        let topo = topology::MachineTopology::new();
        let cpuinfo = topo.allocate(strategy, how_many, false);
        cpuinfo.iter().map(|c| c.core as CpuId).collect()
    }

    fn init_logging(&mut self) {
        let _r = env_logger::try_init();
    }

    fn parse_args(&mut self) -> CmdArgs {
        let matches = App::new("Concurrent Hashmap Server")
            .version(crate_version!())
            .about("Spawn a server with a partitioned hash-map")
            .arg(
                Arg::with_name("threads")
                    .short("t")
                    .long("threads")
                    .help("Set the number of threads")
                    .default_value("1")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("capacity")
                    .short("c")
                    .long("capacity")
                    .help("Hash-table initial size")
                    .default_value("10000")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("thread-mapping")
                    .long("thread-mapping")
                    .takes_value(true)
                    .possible_values(&["interleave", "sequential"])
                    .default_value("interleave")
                    .help("Strategy on how to assign threads to NUMA nodes."),
            )
            .arg(
                Arg::with_name("transport")
                    .long("transport")
                    .takes_value(true)
                    .possible_values(&["tcp", "udp"])
                    .default_value("udp")
                    .help("Transport layer."),
            )
            .arg(
                Arg::with_name("incoming-tcp-connection")
                    .long("incoming-tcp-connections")
                    .takes_value(true)
                    .default_value("1")
                    .help("How many incoming TCP connections we expect per port (only valid if transport is TCP)."),
            )
            .arg(
                Arg::with_name("port")
                    .long("port")
                    .takes_value(true)
                    .default_value("6666")
                    .help("Default (starting) port."),
            )
            .get_matches();

        let threads = value_t!(matches, "threads", usize).unwrap_or_else(|e| e.exit());
        let capacity = value_t!(matches, "capacity", usize).unwrap_or_else(|e| e.exit());
        let port = value_t!(matches, "port", usize).unwrap_or_else(|e| e.exit());
        let tcp_connections_per_port =
            value_t!(matches, "incoming-tcp-connection", usize).unwrap_or_else(|e| e.exit());
        let tm_str = value_t!(matches, "thread-mapping", String).unwrap_or_else(|e| e.exit());
        let numa_strategy = if tm_str == "interleave" {
            NumaTopology::Interleave
        } else {
            NumaTopology::Sequential
        };
        let transport_str = value_t!(matches, "transport", String).unwrap_or_else(|e| e.exit());
        let transport = if transport_str == "tcp" {
            Transport::Tcp
        } else {
            Transport::Udp
        };

        CmdArgs {
            threads,
            capacity,
            numa_strategy,
            transport,
            port,
            tcp_connections_per_port,
        }
    }

    fn spawn<F>(&mut self, f: F, on_core: CpuId) -> ThreadId
    where
        F: FnOnce() -> ThreadId,
        F: Send + 'static,
    {
        let handler = std::thread::spawn(move || {
            pin_thread(on_core);
            f();
        });

        self.tid_count += 1;
        self.handles.insert(self.tid_count, handler);
        self.tid_count
    }

    fn join(&mut self, tid: ThreadId) {
        self.handles.remove(&tid).unwrap().join().unwrap();
    }
}

/// Pin a thread to a core
fn pin_thread(id: CpuId) {
    core_affinity::set_for_current(core_affinity::CoreId { id });
}
