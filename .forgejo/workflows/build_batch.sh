#!/usr/bin/env bash
set -euo pipefail

: "${TARGETS:?TARGETS is required}"
: "${BUILD_TYPE:=debug}"
: "${BUILD_JOBS:=2}"
: "${BUILD_PARALLEL:=true}"

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
echo "BUILD_PARALLEL=$BUILD_PARALLEL"
if [[ "$BUILD_PARALLEL" == "true" || "$BUILD_PARALLEL" == "1" || "$BUILD_PARALLEL" == "yes" ]]; then
    echo "Parallelism: BUILD_JOBS=$BUILD_JOBS"
else
    echo "Parallelism disabled; targets will be built sequentially."
fi
printf '  %s\n' "${targets[@]}"

if [[ "$BUILD_PARALLEL" == "true" || "$BUILD_PARALLEL" == "1" || "$BUILD_PARALLEL" == "yes" ]]; then
    printf '%s\n' "${targets[@]}" |
        parallel --jobs "$BUILD_JOBS" --halt soon,fail=1 --line-buffer build_one_target {}
else
    for target in "${targets[@]}"; do
        build_one_target "$target"
    done
fi

echo
if [[ "$BUILD_PARALLEL" == "true" || "$BUILD_PARALLEL" == "1" || "$BUILD_PARALLEL" == "yes" ]]; then
    echo "All batched Linux builds completed successfully. (concurrency: $BUILD_JOBS)"
else
    echo "All batched Linux builds completed successfully. (sequential)"
fi
