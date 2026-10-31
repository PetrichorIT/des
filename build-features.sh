#!/bin/bash

echo "[des-net-utils]"
cargo build -p des-sync-utils

echo "[des-macros-core]"
cargo build -p des-macros-core

echo "[des-macros]"
cargo build -p des-macros

echo "[des]"
cargo build -p des
echo "[des] cqueue"
cargo build -p des --features cqueue
echo "[des] serde"
cargo build -p des --features cqueue
echo "[des] net"
cargo build -p des
echo "[des] net + async"
cargo build -p des  --features async


echo "[des] tracing"
cargo build -p des --features tracing
echo "[des] tracing + net"
cargo build -p des --features tracing
echo "[des] tracing + net + async + unstable-tokio-enable-time"
cargo build -p des --features tracing  --features async --features unstable-tokio-enable-time



# 'tests' build
# ... dependent on target 'des'
echo "[examples]"
cargo build -p examples
