#!/usr/bin/env bash
set -ex

if [[ -z "$USE_CROSS" || "$USE_CROSS" == "false" ]]; then
    cargo_runner="cargo"
else
    export CROSS_DEBUG=1
    export CROSS_NO_WARNINGS=0
    cargo_runner="cross"
fi

build_type=
if [[ "$BUILD_TYPE" == "release" ]]; then
    build_type="--release"
fi

$cargo_runner build --target "$TARGET" $build_type

