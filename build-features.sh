#!/bin/bash


# 'ndl' build

cargo build -p ndl

# 'des_derive' build
# ... dependent on target 'ndl'

cargo build -p des_derive

# 'des' build
# ... dependent on target 'des_derive' with feature 'net'

cargo build -p des
cargo build -p des --features cqueue
cargo build -p des --features internal-metrics
cargo build -p des --features cqueue --features internal-metrics
cargo build -p des --features net
cargo build -p des --features net --features cqueue
cargo build -p des --features net --features internal-metrics
cargo build -p des --features net --features cqueue --features internal-metrics
cargo build -p des --features net --features net-ipv6
cargo build -p des --features net --features net-ipv6 --features cqueue
cargo build -p des --features net --features net-ipv6 --features internal-metrics
cargo build -p des --features net --features net-ipv6 --features cqueue --features internal-metrics


# 'tests' build
# ... dependent on target 'des'

cargo build -p tests