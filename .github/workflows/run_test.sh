#!/usr/bin/env bash
set -x

GREEN=$'\e[0;32m'
RED=$'\e[0;31m'
NC=$'\e[0m'


# build an rmate.rc file
cat << EOB > "$HOME/.rmate.rc"
host: auto
port: 52698
unixsocket: ~/.rmate.socket
EOB

sleep 2

echo "Running tests using target/${GREEN}$TARGET${NC}/debug/rmate"; echo
binary_path=target/"$TARGET/$BUILD_TYPE"/rmate
file "$binary_path"

# use cross if building on special platforms
if [[ -n "$USE_CROSS" && "$USE_CROSS" == "true" ]]; then
    echo "Running ${GREEN}$TARGET${NC} binary under docker. (Using Rust's cross)"
    # Show help message
    cross run --target "$TARGET" -- --help || echo
    # Test with local .rmate.rc
    cross run --target "$TARGET" -- -vvv -w Cargo.toml 2>output.log || echo
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 1
    fi
    printf "\n\n\n"
    sleep 2
    exit 0
fi

if [[ -z "$ARM" || "$ARM" == 'false' ]]; then
    # Show help message
    $binary_path --help
    printf "\n\n\n"
    sleep 2

    # Test with local .rmate.rc
    $binary_path -vvv -w Cargo.toml 2>output.log || echo
    grep "Connection refused (os error " ./output.log
    grep "Read disk settings-> { host: Some(" ./output.log
    printf "\n\n\n"
    sleep 2

    # Test with environment variables
    export RMATE_HOST=auto
    export RMATE_PORT=55555
    $binary_path -vvv -w Cargo.toml 2>output.log || echo
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 1
    fi
    if ! pcregrep -q -M 'host: Some\(\n\s+"localhost",\n\s+\),\n\s+port: Some\(\n\s+55555,\n\s+\),' ./output.log; then
        cat ./output.log
        exit 1
    fi
    if ! grep -q "Finding host automatically from SSH_CONNECTION" ./output.log; then
        cat ./output.log
        exit 1
    fi
    if ! grep -q "from SSH_CONNECTION: localhost" ./output.log; then
        cat ./output.log
        exit 1
    fi
    printf "\n\n\n"
    sleep 2
else # Use qemu to run ARM-based binaries for Linux OS.
    if [[ "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
        libpath="/usr/aarch64-linux-gnu"
        export LD_LIBRARY_PATH=$libpath/lib64
        arm_runner="qemu-aarch64-static"
    else
        libpath="/usr/arm-linux-gnueabihf"
        export LD_LIBRARY_PATH=$libpath/lib
        arm_runner="qemu-arm-static"
    fi
    $arm_runner -L $libpath "$binary_path" -vvv -w Cargo.toml 2>output.log
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 1
    fi
    if ! grep -q "Read disk settings-> { host: Some(" ./output.log; then
        cat ./output.log
        exit 1
    fi
    printf "\n\n\n"
    sleep 2
fi

