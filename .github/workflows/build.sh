#!/usr/bin/env bash

if [[ -z "$USE_CROSS" || "$USE_CROSS" == "false" ]]; then
    cargo build --target "$TARGET" "--$BUILD_TYPE"
else
    cross build --target "$TARGET" "--$BUILD_TYPE"
fi
