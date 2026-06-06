#!/usr/bin/env bash
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN is required}"
: "${GITHUB_REPO:?GITHUB_REPO is required}"
: "${GITHUB_REF_NAME:?GITHUB_REF_NAME is required}"

tag="${GITHUB_REF_NAME:-${GITHUB_REF#refs/tags/}}"

echo "Syncing Forgejo-built release assets to GitHub release: $GITHUB_REPO $tag"
gh --version

if ! gh release view "$tag" -R "$GITHUB_REPO" >/dev/null 2>&1; then
    echo "GitHub release $tag does not exist. Creating it."
    gh release create "$tag" \
        -R "$GITHUB_REPO" \
        --verify-tag \
        --title "$tag" \
        --notes "Synced from Forgejo-built release artifacts."
fi

mapfile -t github_assets < <(
    gh release view "$tag" \
        -R "$GITHUB_REPO" \
        --json assets \
        -q '.assets[].name'
)

shopt -s nullglob
local_assets=(rmate_*.tar.gz rmate_*.zip)

if ((${#local_assets[@]} == 0)); then
    echo "No local release assets found to sync."
    exit 0
fi

missing_assets=()
for file in "${local_assets[@]}"; do
    asset_name="$(basename "$file")"
    found=false

    for github_asset in "${github_assets[@]}"; do
        if [[ "$github_asset" == "$asset_name" ]]; then
            found=true
            break
        fi
    done

    if [[ "$found" == "true" ]]; then
        echo "GitHub already has $asset_name; skipping."
    else
        echo "GitHub is missing $asset_name; will upload."
        missing_assets+=("$file")
    fi
done

if ((${#missing_assets[@]} == 0)); then
    echo "GitHub release already has all Forgejo-built assets."
    exit 0
fi

printf 'Uploading missing assets to GitHub release %s:\n' "$tag"
printf '  %s\n' "${missing_assets[@]}"
gh release upload "$tag" "${missing_assets[@]}" -R "$GITHUB_REPO"
