cargo test -p des
cargo test -p des --features cqueue
cargo test -p des --features internal-metrics
cargo test -p des --features cqueue --features internal-metrics
cargo test -p des --features net
cargo test -p des --features net --features cqueue
cargo test -p des --features net --features internal-metrics
cargo test -p des --features net --features cqueue --features internal-metrics
cargo test -p des --features net --features net-ipv6
cargo test -p des --features net --features net-ipv6 --features cqueue
cargo test -p des --features net --features net-ipv6 --features internal-metrics
cargo test -p des --features net --features net-ipv6 --features cqueue --features internal-metrics
