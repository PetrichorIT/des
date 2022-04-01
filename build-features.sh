#!/bin/bash

# 'des' build

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

# 'ndl' build

cargo build -p ndl

# 'des_derive' build

cargo build -p des_derive

# 'tests' build

cargo build -p tests