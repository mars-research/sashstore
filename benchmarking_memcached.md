# Benchmarking memcached protocol

## sashstore

Running:

`./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 1`

Using `--capacity 5000000`, compile settings LTO and codegen-units=1, jemalloc allocator:

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        58672        58672        56.8       0          7        655        14         6.27       14.30
Global   13       711049       54696        60.9       0          7        2891       15         7.49       15.39

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        6519         6519         6.3        0          8        59         17         4.88       16.91
Global   13       79006        6077         6.8        0          8        412        18         5.52       17.93

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        65190        65190        63.1       0          7        655        14         6.86       14.54
Global   13       790058       60773        67.6       0          7        2891       16         5.70       15.62
```

## memcached

Running:

`./clients/memaslap -s 127.0.0.1:11211 -U -S 1s -T1 -c 1`

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        63743        63743        60.2       0          8        559        13         5.70       13.28
Global   10       580084       58008        66.1       0          7        887        14         6.47       14.65

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        7082         7082         6.7        0          10       68         15         5.44       15.19
Global   10       64454        6445         7.3        0          9        393        16         8.21       16.38

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        70825        70825        66.9       0          8        559        13         6.15       13.46
Global   10       644541       64454        73.5       0          7        887        15         4.63       14.82
```
