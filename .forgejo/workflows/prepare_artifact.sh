#!/usr/bin/env bash
set -ex

binary_path="target/$TARGET/$BUILD_TYPE/rmate"

create_linux() {
    printf "In create_linux: %s\n" "$(pwd)"

    artifacts=rmate_"$TARGET".tar.gz
    strip_cmd=

    # Map architecture-specific cross-strippers present on Linux host
    if [[ $TARGET == "aarch64-unknown-linux-gnu" ]]; then
        strip_cmd="/usr/bin/aarch64-linux-gnu-strip"
    elif [[ $TARGET == "armv7-unknown-linux-gnueabihf" ]]; then
        strip_cmd="/usr/bin/arm-linux-gnueabihf-strip"
    elif [[ $TARGET == "x86_64-pc-windows-gnu" ]]; then
        strip_cmd="/usr/bin/x86_64-w64-mingw32-strip"
    elif [[ $TARGET == "x86_64-unknown-linux-gnu" || $TARGET == "i686-unknown-linux-gnu" || \
        $TARGET == "x86_64-unknown-illumos" || \
        $TARGET == "x86_64-unknown-freebsd" || $TARGET == "i686-unknown-freebsd" ]]; then
        strip_cmd=$(which strip)
    fi

    cp "$binary_path" ./rmate
    echo "Before strip:"
    ls -l "$binary_path"
    if [[ -n "$strip_cmd" && -x "$strip_cmd" ]]; then
        "$strip_cmd" rmate || true
        echo "After strip:"
        ls -l rmate
    fi

    tar czvf "$artifacts" rmate
    rm -f rmate
    ls -la
}

create_macos() {
    printf "In create_macos: %s\n" "$(pwd)"
    artifacts="rmate_${TARGET}.zip"

    # FIXED: The binary was already natively stripped on the macOS VM during the build step,
    # or it should be skipped here on Linux to avoid file corruption.
    cp "$binary_path" ./rmate

    zip "$artifacts" rmate
    rm -f rmate
    ls -la
}

if [ -z "$RELEASE_COMMIT" ]; then
    echo "Not a tagged commit. Exiting."
    exit 1
fi

echo "Preparing release for $TARGET"

if [[ $TARGET == *"apple"* ]]; then
    create_macos
else
    create_linux
fi
