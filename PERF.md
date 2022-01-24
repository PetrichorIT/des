# Preformance statistics

## Test system

CPU: AMD Ryzen 7 3700X 8-core (only one used).
RAM: 32 GiB
L1 cache: 512 KiB
L2 cache: 4 MiB
L3 cache: 32 MiB

## Test case 'ndl'

Event count: 40_001_301
Simtime end: ~ 1days 14h 20min 4s 140ms
100 sperate processes with ~ 100_000 packet hops.

### Blank run

release + debuginfo + no_log
features = [ "net" ]

real 0m5,963s
user 0m5,949s
sys 0m0,012s

### Static gates

release + debuginfo + no_log
features = [ "static_gates" ]

real 0m5,356s
user 0m5,355s
sys 0m0,000s

### Static channels

release + debuginfo + no_log
features = [ "static_channels" ]

real 0m5,436s
user 0m5,428s
sys 0m0,004s

### Static modules

release + debuginfo + no_log
features = [ "static_modules" ]

real 0m5,658s
user 0m5,639s
sys 0m0,008s

### All statics

release + debuginfo + no_log
features = [ "net-static" ]

real 0m4,429s
user 0m4,418s
sys 0m0,004s

### Precise simtime

release + debuginfo + no_log
features = [ "simtime-u128" ]

real 0m7,383s
user 0m7,382s
sys 0m0,000s

### Precise simtime & static all

release + debuginfo + no_log
features = [ "simtime-u128", "static" ]

real 0m6,234s
user 0m6,229s
sys 0m0,000s
