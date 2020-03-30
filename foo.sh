#!/bin/bash

echo $PATH
echo

export PATH=/root/.cargo/bin:$PATH

rustup target add $TARGET
rustc --version; cargo --version; rustup --version
