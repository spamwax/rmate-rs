#!/usr/bin/env bash
set -euo pipefail
set -x

: "${TARGETS:?TARGETS is required}"
: "${BUILD_TYPE:=debug}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"

normalize_targets() {
    printf '%s\n' "$TARGETS" |
        tr ',' '\n' |
        awk '{$1=$1; print}' |
        sed '/^$/d'
}

mapfile -t targets < <(normalize_targets)

if (( ${#targets[@]} == 0 )); then
    echo "No targets were provided."
    exit 1
fi

echo "Testing ${#targets[@]} target(s) with BUILD_TYPE=$BUILD_TYPE"
printf '  %s\n' "${targets[@]}"

cd "$repo_root"

for target in "${targets[@]}"; do
    echo
    echo "============================================================"
    echo "Testing target: $target"
    echo "============================================================"

    export TARGET="$target"
    export BUILD_TYPE
    # export USE_CROSS=false
    export RUST_LOG=trace

    case "$target" in
        aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf)
            export ARM=true
            ;;
        *)
            export ARM=false
            ;;
    esac

    binary_path="target/$target/$BUILD_TYPE/rmate"

    if [[ ! -x "$binary_path" ]]; then
        echo "Expected binary does not exist or is not executable: $binary_path"
        exit 1
    fi

    rm -f ./output.log
    "$script_dir/run_test.sh"

    # Keep a per-target copy for debugging after the batch finishes.
    if [[ -f ./output.log ]]; then
        cp ./output.log "./output-${target}.log"
    fi

    echo "Test passed for $target"
done

echo
echo "All batched Linux tests completed successfully."
