#!/usr/bin/env bash
set -euo pipefail

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

cd "$repo_root"

for target in "${targets[@]}"; do
    echo
    echo "============================================================"
    echo "Preparing artifact for target: $target"
    echo "============================================================"

    export TARGET="$target"
    export BUILD_TYPE
    export RELEASE_COMMIT="${RELEASE_COMMIT:-true}"

    ".forgejo/workflows/prepare_artifact.sh"
done

echo
echo "Prepared artifacts:"
find . -maxdepth 1 -type f \( -name 'rmate_*.tar.gz' -o -name 'rmate_*.zip' \) -ls
