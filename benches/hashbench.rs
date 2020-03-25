//! Loosely based on the benchmark of the rust-evmap (https://github.com/jonhoo/rust-evmap/)

use std::sync::{Arc, Barrier};
use std::thread;
use std::time;

use clap::{crate_version, value_t, App, Arg};
use index;
use indexmap;
use rand::{distributions::Distribution, Rng, RngCore, SeedableRng};
use std::collections::HashMap;
use zipf::ZipfDistribution;

mod utils;
use utils::pin_thread;
use utils::topology::{MachineTopology, ThreadMapping};
use utils::Operation;

fn main() {
    let args = std::env::args().filter(|e| e != "--bench");

    let matches = App::new("Concurrent Hashmap Throughput Benchmarker")
        .version(crate_version!())
        .about("Benchmark partitioned safe/unsafe hashmap code")
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .help("Set the number of threads")
                .default_value("1")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("write-ratio")
                .short("w")
                .long("write-ratio")
                .help("Set the write ratio in percent [1..100]")
                .default_value("10")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("runtime")
                .short("r")
                .long("runtime")
                .help("Experiment runtime in seconds")
                .default_value("10")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("capacity")
                .short("c")
                .long("capacity")
                .help("Hash-table initial size")
                .default_value("10000000") // 10 M
                .takes_value(true),
        )
        .arg(
            Arg::with_name("benchmark")
                .short("b")
                .multiple(true)
                .takes_value(true)
                .possible_values(&["std", "index", "indexmap"])
                .help("What HashMap versions to benchmark."),
        )
        .arg(
            Arg::with_name("distribution")
                .short("d")
                .long("distribution")
                .takes_value(true)
                .possible_values(&["skewed", "uniform"])
                .default_value("uniform")
                .help("What key distribution to use."),
        )
        .get_matches_from(args);

    let threads = value_t!(matches, "threads", usize).unwrap_or_else(|e| e.exit());
    let write_ratio = value_t!(matches, "write-ratio", usize).unwrap_or_else(|e| e.exit());
    let dist = value_t!(matches, "distribution", String).unwrap_or_else(|e| e.exit());
    let capacity = value_t!(matches, "capacity", usize).unwrap_or_else(|e| e.exit());
    let runtime_sec = value_t!(matches, "runtime", u64).unwrap_or_else(|e| e.exit());
    //println!("threads={} write_ratio={} dist={} capacity={} runtime_sec={}",threads, write_ratio, dist, capacity, runtime_sec);

    let span = capacity;
    let dur = time::Duration::from_secs(runtime_sec);

    let stat = |benchmark: &str, results: Vec<usize>| {
        for (tid, result) in results.into_iter().enumerate() {
            // if you change this line also change the run.sh benchmark script
            // benchmark,threads,write_ratio,capacity,dist,tid,total_ops,duration
            println!(
                "{},{},{},{},{},{},{},{}",
                benchmark,
                threads,
                write_ratio,
                span,
                dist,
                tid,
                result,
                dur.as_secs_f64()
            )
        }
    };

    let topology = MachineTopology::new();
    let barrier = Arc::new(Barrier::new(threads));
    let mut join = Vec::with_capacity(threads);

    let versions: Vec<&str> = match matches.values_of("compare") {
        Some(iter) => iter.collect(),
        None => vec!["std", "index", "indexmap"],
    };

    if versions.contains(&"index") {
        let mut cpus = topology
            .allocate(ThreadMapping::Sequential, threads, true)
            .into_iter();

        join.extend((0..threads).into_iter().map(|_| {
            let b = barrier.clone();
            let cpu = cpus.next().unwrap().cpu;
            let dist = dist.clone();

            let thread = thread::spawn(move || {
                let mut map: Arc<index::Index<u64, u64>> =
                    Arc::new(index::Index::with_capacity(capacity));
                for i in 0..capacity {
                    Arc::make_mut(&mut map).insert(i as u64, (i + 1) as u64);
                }
                pin_thread(cpu);
                let start = time::Instant::now();
                let end = start + time::Duration::from_secs(2);
                drive(map.clone(), end, write_ratio, span, &dist);
                b.wait();

                let start = time::Instant::now();
                let end = start + dur;
                drive(map.clone(), end, write_ratio, span, &dist)
            });

            thread
        }));

        let tputs: Vec<usize> = join.drain(..).map(|jh| jh.join().unwrap()).collect();
        stat("index", tputs);
    }

    if versions.contains(&"indexmap") {
        let mut cpus = topology
            .allocate(ThreadMapping::Sequential, threads, true)
            .into_iter();

        join.extend((0..threads).into_iter().map(|_| {
            let b = barrier.clone();
            let cpu = cpus.next().unwrap().cpu;
            let dist = dist.clone();

            let thread = thread::spawn(move || {
                let mut map: Arc<indexmap::IndexMap<u64, u64>> =
                    Arc::new(indexmap::IndexMap::with_capacity(capacity));
                for i in 0..capacity {
                    Arc::make_mut(&mut map).insert(i as u64, (i + 1) as u64);
                }
                pin_thread(cpu);
                let start = time::Instant::now();
                let end = start + time::Duration::from_secs(2);
                drive(map.clone(), end, write_ratio, span, &dist);
                b.wait();

                let start = time::Instant::now();
                let end = start + dur;
                drive(map.clone(), end, write_ratio, span, &dist)
            });

            thread
        }));

        let tputs: Vec<usize> = join.drain(..).map(|jh| jh.join().unwrap()).collect();
        stat("indexmap", tputs);
    }

    if versions.contains(&"std") {
        let mut cpus = topology
            .allocate(ThreadMapping::Sequential, threads, true)
            .into_iter();

        join.extend((0..threads).into_iter().map(|_| {
            let b = barrier.clone();
            let cpu = cpus.next().unwrap().cpu;
            let dist = dist.clone();

            let thread = thread::spawn(move || {
                let mut map: Arc<HashMap<u64, u64>> = Arc::new(HashMap::with_capacity(capacity));
                for i in 0..capacity {
                    Arc::make_mut(&mut map).insert(i as u64, (i + 1) as u64);
                }
                pin_thread(cpu);
                let start = time::Instant::now();
                let end = start + time::Duration::from_secs(2);
                drive(map.clone(), end, write_ratio, span, &dist);
                b.wait();

                let start = time::Instant::now();
                let end = start + dur;
                drive(map.clone(), end, write_ratio, span, &dist)
            });
            thread
        }));

        let tputs: Vec<usize> = join.drain(..).map(|jh| jh.join().unwrap()).collect();
        stat("hashbrown", tputs);
    }
}

trait Backend {
    fn b_get(&mut self, key: u64) -> u64;
    fn b_put(&mut self, key: u64, value: u64);
}

/// Generate a random sequence of operations
///
/// # Arguments
///  - `write_ratio`: Probability of generation a write give a value in [0..100]
///  - `span`: Maximum key-space
///  - `distribution`: Supported distribution 'uniform' or 'skewed'
fn generate_operation(
    rng: &mut rand::rngs::SmallRng,
    write_ratio: usize,
    span: usize,
    distribution: &String,
) -> Operation<OpRd, OpWr> {
    assert!(distribution == "skewed" || distribution == "uniform");

    let skewed = distribution == "skewed";
    let zipf = ZipfDistribution::new(span - 1, 1.03).expect("Can't make ZipDistribution");

    let id = if skewed {
        zipf.sample(rng) as u64
    } else {
        // uniform
        rng.gen_range(0, span as u64)
    };

    if rng.gen::<usize>() % 100 < write_ratio {
        Operation::WriteOperation(OpWr::Put(id, rng.next_u64()))
    } else {
        Operation::ReadOperation(OpRd::Get(id))
    }
}

fn drive<B: Backend>(
    mut backend: B,
    end: time::Instant,
    write_ratio: usize,
    span: usize,
    distribution: &String,
) -> usize {
    let mut ops = 0;
    let mut rng = rand::rngs::SmallRng::from_entropy();

    while time::Instant::now() < end {
        match generate_operation(&mut rng, write_ratio, span, &distribution) {
            Operation::ReadOperation(OpRd::Get(id)) => {
                backend.b_get(id);
            }
            Operation::WriteOperation(OpWr::Put(id, val)) => {
                backend.b_put(id, val);
            }
        }

        ops += 1;
    }

    ops
}

/// Operations we can perform on the stack.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum OpWr {
    /// Add an item to the hash-map.
    Put(u64, u64),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum OpRd {
    /// Get item from the hash-map.
    Get(u64),
}

impl Backend for Arc<HashMap<u64, u64>> {
    fn b_put(&mut self, key: u64, val: u64) {
        Arc::make_mut(self).insert(key, val);
    }

    fn b_get(&mut self, key: u64) -> u64 {
        self.get(&key).map(|v| *v).unwrap()
    }
}

impl Backend for Arc<index::Index<u64, u64>> {
    fn b_put(&mut self, key: u64, val: u64) {
        Arc::make_mut(self).insert(key, val);
    }

    fn b_get(&mut self, key: u64) -> u64 {
        self.get(&key).map(|v| *v).unwrap()
    }
}

impl Backend for Arc<indexmap::IndexMap<u64, u64>> {
    fn b_put(&mut self, key: u64, val: u64) {
        Arc::make_mut(self).insert(key, val);
    }

    fn b_get(&mut self, key: u64) -> u64 {
        self.get(&key).map(|v| *v).unwrap()
    }
}
