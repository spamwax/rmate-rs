#!/usr/bin/env bash
set -euo pipefail
set -x

MACOS_VM_HOST="macos-sonoma-ci"
SSH_CMD=(ssh -o ConnectTimeout=10 -o BatchMode=yes "$MACOS_VM_HOST")

remote_dir="/tmp/rmate-universal-${GITHUB_RUN_ID:-manual}"
remote_workdir="$remote_dir/rmate-rs"
artifact_name="rmate_universal-apple-darwin.zip"

run_on_macos() {
    "${SSH_CMD[@]}" "$1"
}

copy_to_vm() {
    echo "Creating remote directory structure on macOS VM..."
    run_on_macos "rm -rf '$remote_dir' && mkdir -p '$remote_workdir'"

    echo "Copying macOS artifact files to macOS VM..."
    shopt -s nullglob
    local artifacts=("$GITHUB_WORKSPACE"/rmate_*apple*.zip)
    shopt -u nullglob

    if (( ${#artifacts[@]} != 2 )); then
        printf 'Expected exactly 2 macOS artifacts, found %s:\n' "${#artifacts[@]}" >&2
        printf '  %s\n' "${artifacts[@]:-}" >&2
        exit 1
    fi

    scp "${artifacts[@]}" "$MACOS_VM_HOST:$remote_workdir/"
}

make_it_fat() {
    run_on_macos "
        set -e
        cd '$remote_workdir'

        rm -rf x86_64 aarch64 rmate rmate-x86_64 rmate-aarch64 '$artifact_name'

        unzip -o rmate_x86_64-apple-darwin.zip -d x86_64
        mv x86_64/rmate rmate-x86_64

        unzip -o rmate_aarch64-apple-darwin.zip -d aarch64
        mv aarch64/rmate rmate-aarch64

        lipo -create \
            -output rmate \
            rmate-x86_64 \
            rmate-aarch64

        lipo -info rmate
        file rmate
        zip '$artifact_name' rmate
    "
}

copy_to_vm
make_it_fat

scp "$MACOS_VM_HOST:$remote_workdir/$artifact_name" "$GITHUB_WORKSPACE/"
ls -l "$GITHUB_WORKSPACE/$artifact_name"
