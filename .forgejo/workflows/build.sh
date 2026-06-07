#!/usr/bin/env bash
set -e

# 1. Declare output text colors for logging
GREEN='\033[0;32m'
NC='\033[0m'

# 2. Capture the initial state of build_type (passed from Forgejo environment)
# If not set by your pipeline engine, it defaults to "debug"
INITIAL_BUILD_TYPE="${build_type:-debug}"

# 3. Separate your folder path directory from your actual compiler flag
if [[ "$INITIAL_BUILD_TYPE" == "release" ]]; then
    build_dir="release"
    cargo_flag="--release"
else
    build_dir="debug"
    cargo_flag="" # Debug is default, no flag needed
fi

# 4. Now paths evaluate perfectly without empty gaps or double slashes
binary_path="target/$TARGET/$build_dir/rmate"
binary_folder="target/$TARGET/$build_dir"
MACOS_VM_HOST="macos-sonoma-ci"

build_non_macos() {
    if [[ -z "$USE_CROSS" || "$USE_CROSS" == "false" ]]; then
        cargo_runner="cargo"
    else
        export CROSS_DEBUG=1
        export CROSS_NO_WARNINGS=0
        cargo_runner="cross"
    fi

    # FIXED: Uses the clean $cargo_flag separation
    $cargo_runner build --target "$TARGET" $cargo_flag
}

# 1. Keep the SSH command clean and simple
SSH_CMD=(ssh -o ConnectTimeout=10 -o BatchMode=yes "$MACOS_VM_HOST")

# 2. Force Zsh to source your profile explicitly at the start of every session
run_on_macos() {
    "${SSH_CMD[@]}" "$1"
}

prepare_mac_os() {
    echo "Checking and updating dependencies on macOS..."
    run_on_macos "brew list pcre2 &>/dev/null || brew install pcre2"
    run_on_macos "rustc --version"
}

build_on_macos() {
    # 1. Unique directory for this run's clone on macOS
    local remote_dir="/tmp/rmate-ci-${GITHUB_RUN_ID:-manual}-${TARGET}"

    echo "Creating remote directory structure on macOS VM..."
    run_on_macos "rm -rf \"$remote_dir\" && mkdir -p \"$remote_dir/rmate-rs\""

    echo "Syncing local source files to macOS VM via rsync..."
    rsync -az --exclude='.git/' --exclude='target/' \
        -e "ssh -o ConnectTimeout=10 -o BatchMode=yes" \
        "$GITHUB_WORKSPACE/" "$MACOS_VM_HOST:$remote_dir/rmate-rs/"

    echo "Building binary with Cargo on macOS..."
    # FIXED: Uses the correct $cargo_flag separation
    run_on_macos "cd \"$remote_dir/rmate-rs\" && cargo build --target $TARGET $cargo_flag"
    run_on_macos "cd \"$remote_dir/rmate-rs\" && ls -l $binary_path && strip $binary_path && ls -l $binary_path"

    echo "Downloading binary artifact back to local workspace..."
    mkdir -p "$GITHUB_WORKSPACE/$binary_folder"

    # FIXED: Paths resolve to .../release/rmate or .../debug/rmate flawlessly
    scp "$MACOS_VM_HOST:$remote_dir/rmate-rs/$binary_path" "$GITHUB_WORKSPACE/$binary_folder/"
}

show_context() {
    printf "ENVIRONMENT VARIABLES:\n"
    printf "\tUSE_CROSS: ${GREEN}%s${NC}\n" "${USE_CROSS:-}"
    printf "\tARM: ${GREEN}%s${NC}\n" "${ARM:-}"
    printf "\tTARGET: ${GREEN}%s${NC}\n" "$TARGET"
    printf "\tBUILD_TYPE: ${GREEN}%s${NC}\n" "$INITIAL_BUILD_TYPE"
    printf "\tGITHUB_WORKSPACE: ${GREEN}%s${NC}\n" "$GITHUB_WORKSPACE"
    printf "\tHOME: ${GREEN}%s${NC}\n\n" "$HOME"

    printf "Building binaries target/${GREEN}%s${NC}/%s/rmate\n\n" "$TARGET" "$build_dir"

    echo "on runner"
    printf "Current path: %s\n" "$(pwd)"
    ls -la "$(pwd)"
    ls -l "$binary_path" || true
    echo "========"
}

show_context

case "$TARGET" in
    *apple*)
        prepare_mac_os
        build_on_macos
        ;;
    *)
        build_non_macos
        ;;
esac

