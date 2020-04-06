# sashstore

Aims to be a simple (but safe) partitioned key--value store in rust.

## main binary

Run with:

```
cargo run --release -- --threads 1 --transport tcp
```

To benchmark use `redis-benchmark`:

```
redis-benchmark -t get -r 10000 -n 1000000 -e -d 8 -h 192.168.100.117 -p 6666
```

## hashbench

Benchmarks partitioned hash-table implementations:

```bash
cd benches
bash run.sh
```