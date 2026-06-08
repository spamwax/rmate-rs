#!/usr/bin/env bash
set -euo pipefail

: "${TARGETS:?TARGETS is required}"
: "${BUILD_TYPE:=debug}"
: "${BUILD_JOBS:=2}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"

normalize_targets() {
    printf '%s\n' "$TARGETS" |
        tr ',' '\n' |
        awk '{$1=$1; print}' |
        sed '/^$/d'
}

build_one_target() {
    local target="$1"

    echo
    echo "============================================================"
    echo "Building target: $target"
    echo "============================================================"

    cd "$repo_root"

    export TARGET="$target"
    export BUILD_TYPE
    export USE_CROSS=false

    case "$target" in
        aarch64-*|armv7-*|arm-*)
            export ARM=true
            ;;
        *)
            export ARM=false
            ;;
    esac

    "$script_dir/build.sh"

    local binary_path="target/$target/$BUILD_TYPE/rmate"

    echo "Verifying built binary for $target:"
    ls -l "$binary_path"
    file "$binary_path"
}

export -f build_one_target
export script_dir repo_root BUILD_TYPE

mapfile -t targets < <(normalize_targets)

if (( ${#targets[@]} == 0 )); then
    echo "No targets were provided."
    exit 1
fi

echo "Building ${#targets[@]} target(s) with BUILD_TYPE=$BUILD_TYPE"
echo "Parallelism: BUILD_JOBS=$BUILD_JOBS"
printf '  %s\n' "${targets[@]}"

printf '%s\n' "${targets[@]}" |
    parallel --jobs "$BUILD_JOBS" --halt soon,fail=1 --line-buffer build_one_target {}

echo
echo "All batched Linux builds completed successfully."
