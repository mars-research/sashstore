# sashstore

Aims to be a simple (but safe) partitioned key--value store in rust.

```bash
cd benches
bash run.sh
```

We propose to implement our approach using POSIX APIs for OS threads, memory
management, and so on, and Linux’s XDP interface for networking. The KV store
spawns an OS thread for each CPU core assigned for the application
with pthread_create and allocates memory regions individually foreach CPU core
with mmap. The packet steering logic is implemented as an eBPF program. The KV
store uses the AF_XDP socket type to open a kernel-bypass channel between the XDP
subsystem and the user space threads. The NIC program determines the CPU of a
request key by parsing the L7 protocol headers, looks up the per-CPU AF_XDP
socket (XSK) and uses the bpf_redirect_map function to forward the packet to the
XSK. The userspace process then receives the packet via the XSK and performs the
necessary protocol and request processing. For example, in the case of Memcached
over UDP, the application needs to parse the UDP headers and the Memcached
request, and perform the requested operation such asgetorset, to retrieve or
update a value, respectively