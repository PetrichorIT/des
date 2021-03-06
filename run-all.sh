#!/bin/bash

cargo run --release --bin t-inet
cargo run --release --bin t-metrics
cargo run --release --bin t-metrics2 --features des/metrics
cargo run --release --bin t-ndl
cargo run --release --bin t-pingpong
cargo run --release --bin t-ptrhell
cargo run --release --bin t-utils
cargo run --release --bin t-waiter
cargo run --release --bin t-proto
cargo run --release --bin t-droptest
cargo run --release --bin t-macro2