#!/usr/bin/env bash
set -euo pipefail

vm=${1:?missing vm}
target=${2:?missing target}
BUILD_TYPE=${3:?missing BUILD_TYPE}
RUN_TESTS=${4:?missing RUN_TESTS}

echo "==> Running inside VM"
echo "vm=$vm"
echo "target=$target"
echo "BUILD_TYPE=$BUILD_TYPE"
echo "RUN_TESTS=$RUN_TESTS"

cd "$HOME/rmate-rs"

case "$BUILD_TYPE" in
  debug)
    echo cargo build --target "$target"
    ;;
  release)
    echo cargo build --release --target "$target"
    ;;
  *)
    echo "error: unknown BUILD_TYPE: $BUILD_TYPE" >&2
    exit 2
    ;;
esac

case "$RUN_TESTS" in
  true|yes|1)
    echo cargo test --target "$target"
    ;;
  false|no|0)
    echo "==> Skipping tests"
    ;;
  *)
    echo "error: unknown RUN_TESTS: $RUN_TESTS" >&2
    exit 2
    ;;
esac
