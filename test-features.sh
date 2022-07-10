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

# "runtime", "time", "net"
cargo test -p des --features full

# 'ndl' tests

cargo test -p ndl