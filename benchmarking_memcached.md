# Benchmarking memcached protocol

## sashstore

Running:

`./clients/memaslap -s 127.0.0.1:6666 -U -S 1s -T1 -c 1`

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        46680        46680        49.0       0          15       660        18         6.53       18.48
Global   7        330612       47230        48.4       0          14       2980       18         7.21       18.23

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        5186         5186         5.4        0          16       89         22         3.91       22.08
Global   7        36735        5247         5.4        0          16       9446       22         55.58      21.92

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        51868        51868        54.5       0          15       660        18         7.48       18.81
Global   7        367350       52478        53.8       0          14       9446       18         19.28      18.57
```

More optimization in compiler (e.g., lto = true and codegen-units = 1):

```log
Get Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        61564        61564        55.9       0          12       206        13         5.48       13.63
Global   13       700842       53910        63.9       0          7        1906       15         7.14       15.60

Set Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        6841         6841         6.2        0          14       38         17         6.79       17.71
Global   13       77872        5990         7.1        0          9        16110      20         69.58      19.65

Total Statistics
Type     Time(s)  Ops          TPS(ops/s)   Net(M/s)   Get_miss   Min(us)  Max(us)    Avg(us)    Std_dev    Geo_dist  
Period   1        68407        68407        62.2       0          12       206        14         4.07       13.99
Global   13       778717       59901        71.0       0          7        16110      16         22.73      15.97
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
