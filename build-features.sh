#!/bin/bash

echo "[des-ndl]"
cargo build -p des-ndl

echo "[des-macros-core]"
cargo build -p des-macros-core

echo "[des-macros]"
cargo build -p des-macros

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
echo "[des] net + ndl"
cargo build -p des --features net --features ndl
echo "[des] net + metrics"
cargo build -p des --features net --features metrics
echo "[des] net + ndl + metrics"
cargo build -p des --features net --features ndl --features metrics
echo "[des] net + async"
cargo build -p des --features net --features async
echo "[des] net + async + metrics"
cargo build -p des --features net --features metrics --features async


echo "[des] multi-threaded"
cargo build -p des --features multi-threaded
echo "[des] multi-threaded + net"
cargo build -p des --features multi-threaded --features net
echo "[des] multi-threaded + net + async"
cargo build -p des --features multi-threaded --features net --features async


# 'tests' build
# ... dependent on target 'des'
echo "[examples]"
cargo build -p examples