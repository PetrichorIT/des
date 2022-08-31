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
cargo build -p des --features metrics
echo "[des] cqueue + metrics"
cargo build -p des --features cqueue --features metrics
echo "[des] net"
cargo build -p des --features net
echo "[des] net + cqueue"
cargo build -p des --features net --features cqueue
echo "[des] net + metrics"
cargo build -p des --features net --features metrics
echo "[des] net + cqueue + metrics"
cargo build -p des --features net --features cqueue --features metrics
echo "[des] net-std"
cargo build -p des --features net --features std-net
echo "[des] net-std + cqueue"
cargo build -p des --features net --features std-net --features cqueue
echo "[des] net-std + metrics"
cargo build -p des --features net --features std-net --features metrics
echo "[des] net-std + cqueue + metrics"
cargo build -p des --features net --features std-net --features cqueue --features metrics
echo "[des] net + std-net + async"
cargo build -p des --features net --features async
echo "[des] net + std-net + async + cqueue"
cargo build -p des --features net --features cqueue --features async
echo "[des] net + std-net + async + metrics"
cargo build -p des --features net --features metrics --features async
echo "[des] net + std-net + async + cqueue + metrics"
cargo build -p des --features net --features cqueue --features metrics --features async
echo "[des] net + std-net + async + async-sharedrt"
cargo build -p des --features net --features async --features async-sharedrt
echo "[des] net + std-net + async + async-sharedrt + cqueue"
cargo build -p des --features net --features async --features async-sharedrt --features cqueue
echo "[des] net + std-net + async + async-sharedrt + metrics"
cargo build -p des --features net --features async --features async-sharedrt --features metrics 
echo "[des] net + std-net + async + async-sharedrt + cqueue + metrics"
cargo build -p des --features net --features async --features async-sharedrt --features cqueue --features metrics



# 'tests' build
# ... dependent on target 'des'
echo "[examples]"
cargo build -p examples