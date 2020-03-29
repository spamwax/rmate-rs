#!/usr/local/bin/bash
set -ex
pwd
git clone https://github.com/listboss/rmate-rust.git rmate || true
ls -la
cd rmate
cargo run -- --help || true
