#!/bin/bash

#
# This should only test 'real' test not examples in 
# the 'tests' crate
#

# 'des' tests
#
# *Features cqueue/internal-metrics has no test cases*
#
# There are the following test cases
# - runtime
# - sync 
# - time
# - net (v4 or v6)

# "runtime", "time"
cargo test -p des
# "runtime", "time", "net"
cargo test -p des --features net
# "runtime", "time", "net(v6)"
cargo test -p des --features net --features net-ipv6
# "runtime", "time", "net", "async" (not that v4/v6 does not matter to async)
cargo test -p des --features net --features async

# 'ndl' tests

cargo test -p ndl