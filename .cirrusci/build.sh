#!/usr/bin/env /usr/local/bin/bash

set -ex
export PATH=$HOME/.cargo/bin:$PATH
pwd

cargo run --release -- --help || true
