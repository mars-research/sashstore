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

## memcached

```bash
sudo apt-get install memcached
sudo apt-get build-dep libmemcached
wget https://launchpad.net/libmemcached/1.0/1.0.18/+download/libmemcached-1.0.18.tar.gz
tar zxvf libmemcached-1.0.18.tar.gz
cd libmemcached-1.0.18
./configure --enable-memaslap
make -j 6
sudo make install
```

Start server (UDP)

```bash
/usr/bin/memcached -m 64 -U 11211 -u memcache
```

Start client

```bash
./clients/memaslap -s 127.0.0.1:11211 -S 1s -B -T2 -c 128 > out
```
