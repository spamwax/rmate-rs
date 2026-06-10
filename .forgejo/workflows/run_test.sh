#!/usr/bin/env bash
set -euo pipefail

GREEN=$'\e[0;32m'
NC=$'\e[0m'
RED=$'\e[0;31m'

# shellcheck disable=SC2329
dump_output_on_error() {
    local rc=$?

    if [[ $rc -ne 0 && -f ./output.log ]]; then
        printf '\n%s\n' '----- output.log after failure -----'
        cat ./output.log
        printf '%s\n' '----- end output.log -----'
    fi

    exit "$rc"
}

trap dump_output_on_error ERR

write_test_rc() {
    cat << EOB > "$GITHUB_WORKSPACE/.rmate.rc"
host: auto
port: 62698
unixsocket: ~/.rmate.socket
EOB
}

show_context() {
    printf "ENVIRONMENT VARIABLES:\n"
    printf "\tUSE_CROSS: ${GREEN}%s${NC}\n" "${USE_CROSS:-}"
    printf "\tARM: ${GREEN}%s${NC}\n" "${ARM:-}"
    printf "\tTARGET: ${GREEN}%s${NC}\n" "$TARGET"
    printf "\tBUILD_TYPE: ${GREEN}%s${NC}\n" "$BUILD_TYPE"
    printf "\tGITHUB_WORKSPACE: ${GREEN}%s${NC}\n" "$GITHUB_WORKSPACE"
    printf "\tHOME: ${GREEN}%s${NC}\n\n" "$HOME"

    printf "Running tests using target/${GREEN}%s${NC}/%s/rmate\n\n" "$TARGET" "$BUILD_TYPE"
    file "$binary_path"

    echo "on runner"
    pwd
    ls -la
    echo "========"
    ls -l "$binary_path"
    file "$binary_path"
    echo "========"
}

assert_output_contains() {
    local pattern=$1
    local exit_code=$2

    if ! grep -q "$pattern" ./output.log; then
        cat ./output.log
        exit "$exit_code"
    fi
}

show_output_and_exit_ok() {
    printf "\n\n\n"
    sleep 2
    cat ./output.log
    exit 0
}

FREEBSD_VM_HOST="freebsd-14.3"
ILLUMOS_VM_HOST="omnios-r151058"
MACOS_VM_HOST="macos-sonoma-ci"

prepare_mac_os() {
    ssh "$MACOS_VM_HOST" "source ~/.zprofile; brew install pcre2"
    ssh "$MACOS_VM_HOST" "source ~/.zprofile; rustc --version"
}

run_vm_test() {
    local vm_type=$1
    local remote_dir="/tmp/rmate-ci-${GITHUB_RUN_ID:-manual}-${TARGET}"
    local vm_host

    if [[ "$vm_type" == *freebsd* ]]; then
        vm_host="$FREEBSD_VM_HOST"
    elif [[ "$vm_type" == *illumos* ]]; then
        vm_host="$ILLUMOS_VM_HOST"
    elif [[ "$vm_type" == *macos* ]]; then
        vm_host="$MACOS_VM_HOST"
    else
        printf "${RED}%s${NC}\n" "No VM available for $vm_type"
        exit 1
    fi

    echo "Running ${GREEN}$TARGET${NC} binary inside $vm_host VM."

    # shellcheck disable=SC2029
    ssh "$vm_host" "rm -rf '$remote_dir' && mkdir -p '$remote_dir'"
    scp "$binary_path" "$vm_host:$remote_dir/rmate"
    scp "$GITHUB_WORKSPACE/.rmate.rc" "$vm_host:$remote_dir/.rmate.rc"
    scp "$GITHUB_WORKSPACE/.rmate.rc" "$vm_host:~/.rmate.rc"
    scp Cargo.toml "$vm_host:$remote_dir/Cargo.toml"

    # shellcheck disable=SC2029
    ssh "$vm_host" "echo 'in $vm_host:' && uname -a && pwd && echo HOME=\$HOME && ls -la '$remote_dir'"
    # shellcheck disable=SC2029
    ssh "$vm_host" "chmod +x '$remote_dir/rmate' && file '$remote_dir/rmate' && (ldd '$remote_dir/rmate' || true)"
    # shellcheck disable=SC2029
    ssh "$vm_host" "'$remote_dir/rmate' --help"
    # shellcheck disable=SC2029
    ssh "$vm_host" "cd '$remote_dir' && pwd && ./rmate -vvv -w Cargo.toml 2>output.log || true; cat output.log" > output.log

    assert_output_contains "Connection refused (os error " 10
    assert_output_contains "Read disk settings-> { host: Some(" 11

    # shellcheck disable=2029
    ssh "$vm_host" "rm -rf '$remote_dir' ~/.rmate.rc" || true
    show_output_and_exit_ok
}

run_cross_test() {
    echo "Running ${GREEN}$TARGET${NC} binary under Docker using Rust cross."

    cross run --target "$TARGET" -- --help
    cross run --target "$TARGET" -- -vvv -w Cargo.toml 2> output.log || true

    assert_output_contains "Connection refused (os error " 1
    show_output_and_exit_ok
}

run_native_test() {
    echo "Running ${GREEN}$TARGET${NC} binary natively."

    "$binary_path" --help
    printf "\n\n\n"
    sleep 2

    "$binary_path" -vvv -w Cargo.toml 2> output.log || true
    assert_output_contains "Connection refused (os error " 2
    assert_output_contains "Read disk settings-> { host: Some(" 3

    export RMATE_HOST=auto
    export RMATE_PORT=55555
    "$binary_path" -vvv -w Cargo.toml 2> output.log || true

    assert_output_contains "Connection refused (os error " 4

    local pcgrep="pcregrep"
    if [[ "$TARGET" == *"apple"* ]]; then
        pcgrep="pcre2grep"
    fi

    if ! "$pcgrep" -q -M 'host: Some\(\n\s+"localhost",\n\s+\),\n\s+port: Some\(\n\s+55555,\n\s+\),' ./output.log; then
        cat ./output.log
        exit 5
    fi
    assert_output_contains "Finding host automatically from SSH_CONNECTION" 6
    assert_output_contains "from SSH_CONNECTION: localhost" 7
    show_output_and_exit_ok
}

run_linux_qemu_test() {
    echo "Running ${GREEN}$TARGET${NC} binary using QEMU."

    local libpath
    local qemu_runner

    case "$TARGET" in
        x86_64-unknown-linux-gnu)
            libpath="/usr/x86_64-linux-gnu"
            qemu_runner="qemu-x86_64-static"
            ;;
        i686-unknown-linux-gnu)
            libpath="/usr/i686-linux-gnu"
            qemu_runner="qemu-i386-static"
            ;;
        aarch64-unknown-linux-gnu)
            libpath="/usr/aarch64-linux-gnu"
            qemu_runner="qemu-aarch64-static"
            ;;
        armv7-unknown-linux-gnueabihf)
            libpath="/usr/arm-linux-gnueabihf"
            qemu_runner="qemu-arm-static"
            ;;
        *)
            echo "No QEMU Linux runner configured for $TARGET."
            exit 12
            ;;
    esac

    if ! command -v "$qemu_runner" >/dev/null 2>&1; then
        echo "Missing QEMU runner: $qemu_runner"
        exit 13
    fi

    if [[ ! -d "$libpath" ]]; then
        echo "Missing target library path: $libpath"
        exit 14
    fi

    export RUST_BACKTRACE=full
    "$qemu_runner" -L "$libpath" "$binary_path" --help
    "$qemu_runner" -L "$libpath" "$binary_path" -vvv -w Cargo.toml 2> output.log || true

    assert_output_contains "Connection refused (os error " 8
    assert_output_contains "Read disk settings-> { host: Some(" 9
    show_output_and_exit_ok
}

write_test_rc
rm -f ./output.log
export RUST_LOG=trace
sleep 2

binary_path="target/$TARGET/$BUILD_TYPE/rmate"
show_context

case "$TARGET" in
    *illumos*)
        run_vm_test illumos
        ;;
    *freebsd*)
        run_vm_test freebsd
        ;;
    *apple*)
        prepare_mac_os
        run_vm_test macos-sonoma-ci
        ;;
    x86_64-unknown-linux-gnu|i686-unknown-linux-gnu|aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf)
        if [[ "$(uname -m)" == "x86_64" && "$TARGET" == "x86_64-unknown-linux-gnu" ]]; then
            run_native_test
        elif [[ "$(uname -m)" == "i686" && "$TARGET" == "i686-unknown-linux-gnu" ]]; then
            run_native_test
        elif [[ "$(uname -m)" == "aarch64" && "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
            run_native_test
        elif [[ "$(uname -m)" == "armv7l" && "$TARGET" == "armv7-unknown-linux-gnueabihf" ]]; then
            run_native_test
        else
            run_linux_qemu_test
        fi
        ;;
    *)
        if [[ "${USE_CROSS:-false}" == "true" ]]; then
            run_cross_test
        else
            run_native_test
        fi
        ;;
esac
