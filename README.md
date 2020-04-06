# sashstore

Aims to be a simple (but safe) partitioned key--value store in rust.

## main binary

Run with:

```bash
cargo run --release -- --threads 1 --transport tcp
```

To benchmark use `redis-benchmark`:

```bash
redis-benchmark -t get -r 10000 -n 1000000 -e -d 8 -h 192.168.100.117 -p 6666
```

## hashbench

Benchmarks partitioned hash-table implementations:

```bash
cd benches
bash run.sh
```

## Application benchmarks

```bash
$ cargo bench --bin sashstore
test tests::bench_get_requests ... bench:         414 ns/iter (+/- 6)
test tests::bench_set_requests ... bench:         613 ns/iter (+/- 19)
```
