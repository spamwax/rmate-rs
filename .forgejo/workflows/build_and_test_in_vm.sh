#!/usr/bin/env bash
set -euo pipefail

: "${TARGETS:?TARGETS is required}"
: "${BUILD_TYPE:=debug}"
: "${BUILD_JOBS:=2}"
: "${BUILD_PARALLEL:=true}"
: "${RUN_TESTS:=false}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"
virsh_uri="qemu:///system"
vm=

REMOTE_USER="forgejo"
# REMOTE_HOST="192.168.122.11"
SSH_KEY="/srv/forgejo-runner/.ssh/opi5-vm-ci_ed25519"
REMOTE_DIR="/tmp/rmate-ci-${USER:-runner}-$$"
REMOTE_SCRIPT="${REMOTE_DIR}/run_inside_vm.sh"

ssh_base=(
  ssh
  -o BatchMode=yes
  -o StrictHostKeyChecking=accept-new
)

scp_base=(
  scp
  -o BatchMode=yes
  -o StrictHostKeyChecking=accept-new
)

normalize_targets() {
    printf '%s\n' "$TARGETS" |
        tr ',' '\n' |
        awk '{$1=$1; print}' |
        sed '/^$/d'
}

export -f build_one_target
export script_dir repo_root BUILD_TYPE

mapfile -t targets < <(normalize_targets)

if (( ${#targets[@]} == 0 )); then
        echo "No targets were provided."
            exit 1
fi


check_vm_status() {
    shopt -s nocasematch
    local target
    target=$1
    case "$target" in
        *freebsd*)
            vm="freebsd-14.3-x86_64"
            ;;
        *illumos*)
            vm="omnios-r151058"
            ;;
        *)
            echo "No VM configured for target: $target" >&2
            exit 1
            ;;
    esac
    shopt -u nocasematch

    echo "Target: $target"
    echo "VM: $vm"
    echo "libvirt URI: $virsh_uri"

    echo "Current libvirt domains:"
    virsh -c "$virsh_uri" list --all || true

    state="$(virsh -c "$virsh_uri" domstate "$vm" 2>&1 || true)"
    echo "$vm state: $state"

}
check_vm_status "$@"
REMOTE_HOST="$vm"
echo "Building ${#targets[@]} target(s) with BUILD_TYPE=$BUILD_TYPE"

for target in "${targets[@]}"; do
    echo "$target"
    if ! "$script_dir"/ensure_vm_ready.sh "$target"; then
        printf "Could not build & test for %s" "$target"
        error_flag=true
    else
        echo "==> Creating remote work dir"
        "${ssh_base[@]}" "$REMOTE_HOST" "mkdir -p '$REMOTE_DIR'"
        echo "==> Copying run_inside_vm.sh"
        "${scp_base[@]}" "$script_dir/run_inside_vm.sh" "$REMOTE_HOST:$REMOTE_SCRIPT"
        echo "==> Running inside VM"
        "${ssh_base[@]}" "$REMOTE_HOST" \
          "chmod +x '$REMOTE_SCRIPT' && '$REMOTE_SCRIPT' '$vm' '$target' '$BUILD_TYPE' '$RUN_TESTS'"
        echo "==> Cleaning remote work dir"
        "${ssh_base[@]}" "$REMOTE_HOST" "rm -rf '$REMOTE_DIR'"
        # run_inside_vm.sh "$vm" "$target" "$BUILD_TYPE" "$RUN_TESTS"
        # Run following inside VM
        # first ensure target is passed on
        # clone the repo in /tmp/rmate-rs_$BUILD_TYPE
        # run cargo --target $target
        # run target/$target/debug/rmate-rs if test running is needed
        # communaicate the results back
        # if $BUILD_TYPE is release copy back the binary
    fi

done

if [ "$error_flag" -eq 1 ]; then
    exit 1
else
    exit 0
fi
