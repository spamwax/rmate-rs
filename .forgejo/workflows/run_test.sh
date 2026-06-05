#!/usr/bin/env bash
# set -x

GREEN=$'\e[0;32m'
NC=$'\e[0m'
# RED=$'\e[0;31m'


# build an rmate.rc file
cat << EOB > "$GITHUB_WORKSPACE/.rmate.rc"
host: auto
port: 52698
unixsocket: ~/.rmate.socket
EOB

export RUST_LOG=trace
sleep 2

printf "ENVIRONMENT VARIABLES:\n"
printf "\tUSE_CROSS: ${GREEN}$USE_CROSS${NC}\n"
printf "\tARM: ${GREEN}$ARM${NC}\n\n"
printf "\tGITHUB_WORKSPACE: ${GREEN}$GITHUB_WORKSPACE${NC}\n\n"
printf "\tHOME: ${GREEN}$HOME${NC}\n\n"

echo "Running tests using target/${GREEN}$TARGET${NC}/debug/rmate"; echo
binary_path=target/"$TARGET/$BUILD_TYPE"/rmate
file "$binary_path"

pwd
ls -la
ls -l "$binary_path"

# Run FreeBSD binaries inside the manually managed FreeBSD test VM on ghoolak.
# This is needed because cross can build FreeBSD targets but cannot run them.
if [[ "$TARGET" == *"freebsd"* ]]; then
    echo "Running ${GREEN}$TARGET${NC} binary inside FreeBSD VM."

    vm_host="freebsd-rmate-test"
    remote_dir="/tmp/rmate-ci-${GITHUB_RUN_ID:-manual}-${TARGET}"

    # shellcheck disable=2029
    ssh "$vm_host" "rm -rf '$remote_dir' && mkdir -p '$remote_dir'"
    scp "$binary_path" "$vm_host:$remote_dir/rmate"
    scp .rmate.rc "$vm_host:$remote_dir/.rmate.rc"
    scp .rmate.rc "$vm_host:/home/forgejo/.rmate.rc"
    scp .rmate.rc "$vm_host:$GITHUB_WORKSPACE/.rmate.rc" || true
    scp Cargo.toml "$vm_host:$remote_dir/Cargo.toml"

    # shellcheck disable=2029
    ssh "$vm_host" "ls -l ~ && ls -l $remote_dir"
    # shellcheck disable=2029
    ssh "$vm_host" "chmod +x '$remote_dir/rmate' && '$remote_dir/rmate' --help"
    # shellcheck disable=2029
    ssh "$vm_host" "'$remote_dir/rmate' -vvv -w '$remote_dir/Cargo.toml' 2>'$remote_dir/output.log' || true; cat '$remote_dir/output.log'" > output.log

    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 10
    fi
    if ! grep -q "Read disk settings-> { host: Some(" ./output.log; then
        cat ./output.log
        exit 11
    fi

    printf "\n\n\n"
    sleep 2
    cat ./output.log
    exit 0
fi

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
    cat ./output.log
    exit 0
fi

if [[ -z "$ARM" || "$ARM" == 'false' ]]; then
    echo "Running ${GREEN}$TARGET${NC} binary (non-ARM Linux or macOS)."
    # Show help message
    $binary_path --help
    printf "\n\n\n"
    sleep 2

    # Test with local .rmate.rc
    $binary_path -vvv -w Cargo.toml 2>output.log || echo
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 2
    fi
    if ! grep -q "Read disk settings-> { host: Some(" ./output.log; then
        cat ./output.log
        exit 3
    fi
    printf "\n\n\n"
    sleep 2

    # Test with environment variables
    export RMATE_HOST=auto
    export RMATE_PORT=55555
    $binary_path -vvv -w Cargo.toml 2>output.log || echo
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 4
    fi
    PCGREP=
    if [[ $TARGET == *"apple"* ]]; then
        PCGREP=pcre2grep
    else
        PCGREP=pcregrep
    fi
    if ! "$PCGREP" -q -M 'host: Some\(\n\s+"localhost",\n\s+\),\n\s+port: Some\(\n\s+55555,\n\s+\),' ./output.log; then
        cat ./output.log
        exit 5
    fi
    if ! grep -q "Finding host automatically from SSH_CONNECTION" ./output.log; then
        cat ./output.log
        exit 6
    fi
    if ! grep -q "from SSH_CONNECTION: localhost" ./output.log; then
        cat ./output.log
        exit 7
    fi
    printf "\n\n\n"
    sleep 2
    cat ./output.log
elif [[ -n "$ARM" || "$ARM" == "true" ]]; then # Use qemu to run ARM-based binaries for Linux OS.
    echo "Running ${GREEN}$TARGET${NC} binary (ARM Linux) using qemu."
    if [[ "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
        libpath="/usr/aarch64-linux-gnu"
        export LD_LIBRARY_PATH=$libpath/lib64
        arm_runner="qemu-aarch64-static"
    else
        libpath="/usr/arm-linux-gnueabihf"
        export LD_LIBRARY_PATH=$libpath/lib
        arm_runner="qemu-arm-static"
    fi
    export RUST_BACKTRACE=full
    $arm_runner -L $libpath "$binary_path" -vvv -w Cargo.toml 2>output.log
    if ! grep -q "Connection refused (os error " ./output.log; then
        cat ./output.log
        exit 8
    fi
    if ! grep -q "Read disk settings-> { host: Some(" ./output.log; then
        cat ./output.log
        exit 9
    fi
    printf "\n\n\n"
    sleep 2
    cat ./output.log
else
    echo "This is not ARM and not using cross. Skipping test execution!"
    file "$binary_path"
    printf "\n\n\n"
    exit 0
fi
