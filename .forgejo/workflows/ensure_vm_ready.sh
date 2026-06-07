#!/usr/bin/env bash
set -euo pipefail

target="${1:?usage: ensure_vm_ready.sh TARGET}"
virsh_uri="qemu:///system"
max_attempts="${VM_READY_MAX_ATTEMPTS:-60}"
sleep_seconds="${VM_READY_SLEEP_SECONDS:-5}"

shopt -s nocasematch
case "$target" in
    *freebsd*)
        vm="freebsd-14.3"
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

if ! grep -qi '^running' <<<"$state"; then
    echo "Starting $vm VM"
    virsh -c "$virsh_uri" start "$vm"
else
    echo "$vm is already running."
fi

echo "Waiting for SSH on $vm..."
for attempt in $(seq 1 "$max_attempts"); do
    printf 'Attempt %s/%s: ' "$attempt" "$max_attempts"

    if ssh -o BatchMode=yes -o ConnectTimeout=5 "$vm" uname -a >/tmp/ensure-vm-ready.out 2>/tmp/ensure-vm-ready.err; then
        echo "ready"
        cat /tmp/ensure-vm-ready.out
        exit 0
    else
        rc=$?
    fi

    echo "not ready yet, ssh rc=$rc"

    if [[ $attempt -eq 1 || $((attempt % 6)) -eq 0 ]]; then
        echo "Last SSH stderr:"
        sed 's/^/  /' /tmp/ensure-vm-ready.err || true
        echo "libvirt domstate:"
        virsh -c "$virsh_uri" domstate "$vm" || true
        echo "libvirt IP info:"
        virsh -c "$virsh_uri" domifaddr "$vm" || true
        virsh -c "$virsh_uri" net-dhcp-leases default || true
    fi

    sleep "$sleep_seconds"
done

echo "$vm VM did not become reachable over SSH after $((max_attempts * sleep_seconds)) seconds." >&2
echo "Final diagnostics:" >&2
virsh -c "$virsh_uri" domstate "$vm" >&2 || true
virsh -c "$virsh_uri" domifaddr "$vm" >&2 || true
virsh -c "$virsh_uri" net-dhcp-leases default >&2 || true
exit 1
