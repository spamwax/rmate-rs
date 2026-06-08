#!/usr/bin/env bash
set -euo pipefail

: "${TARGETS:?TARGETS is required}"
: "${BUILD_TYPE:=debug}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

normalize_targets() {
    # Accept newline-separated, comma-separated, or space-separated targets.
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

echo "Building ${#targets[@]} target(s) with BUILD_TYPE=$BUILD_TYPE"
printf '  %s\n' "${targets[@]}"

for target in "${targets[@]}"; do
    echo
    echo "============================================================"
    echo "Building target: $target"
    echo "============================================================"

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

    binary_path="target/$target/$BUILD_TYPE/rmate"

    echo "Verifying built binary:"
    ls -l "$binary_path"
    file "$binary_path"
done

echo
echo "All batched Linux builds completed successfully."
