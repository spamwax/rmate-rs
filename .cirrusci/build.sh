#!/usr/bin/env /usr/local/bin/bash

set -ex
export PATH=$HOME/.cargo/bin:$PATH
pwd
ls -la
ls -la ../

# git clone https://github.com/listboss/rmate-rust.git rmate || true
# cd rmate
echo "path: $PATH"

cargo run --release -- --help || true
