#!/usr/bin/env /usr/local/bin/bash

set -ex
export PATH=$HOME/.cargo/bin:$PATH
pwd

git clone https://github.com/listboss/rmate-rust.git rmate || true
ls -la
cd rmate
pwd
echo "path: $PATH"

cargo run -- --help || true
