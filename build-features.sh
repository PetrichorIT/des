#!/bin/bash

# 'ndl' build

echo "[ndl]"
cargo build -p ndl

# 'des_macros' build
# ... dependent on target 'ndl'

echo "[des_macros]"
cargo build -p des_macros

# 'des' build
# ... dependent on target 'des_macros' with feature 'net'

echo "[des]"
cargo build -p des
echo "[des] cqueue"
cargo build -p des --features cqueue
echo "[des] metrics"
cargo build -p des --features internal-metrics
echo "[des] cqueue + metrics"
cargo build -p des --features cqueue --features internal-metrics
echo "[des] net"
cargo build -p des --features net
echo "[des] net + cqueue"
cargo build -p des --features net --features cqueue
echo "[des] net + metrics"
cargo build -p des --features net --features internal-metrics
echo "[des] net + cqueue + metrics"
cargo build -p des --features net --features cqueue --features internal-metrics
echo "[des] net6"
cargo build -p des --features net --features net-ipv6
echo "[des] net6 + cqueue"
cargo build -p des --features net --features net-ipv6 --features cqueue
echo "[des] net6 + metrics"
cargo build -p des --features net --features net-ipv6 --features internal-metrics
echo "[des] net6 + cqueue + metrics"
cargo build -p des --features net --features net-ipv6 --features cqueue --features internal-metrics
echo "[des] net + async"
cargo build -p des --features net --features async
echo "[des] net + async + cqueue"
cargo build -p des --features net --features cqueue --features async
echo "[des] net + async + metrics"
cargo build -p des --features net --features internal-metrics --features async
echo "[des] net + async + cqueue + metrics"
cargo build -p des --features net --features cqueue --features internal-metrics --features async
echo "[des] net6 + async"
cargo build -p des --features net --features net-ipv6 --features async
echo "[des] net6 + async + cqueue"
cargo build -p des --features net --features net-ipv6 --features cqueue --features async
echo "[des] net6 + async + metrics"
cargo build -p des --features net --features net-ipv6 --features internal-metrics --features async
echo "[des] net6 + async + cqueue + metrics"
cargo build -p des --features net --features net-ipv6 --features cqueue --features internal-metrics --features async


# 'tests' build
# ... dependent on target 'des'
echo "[examples]"
cargo build -p examples