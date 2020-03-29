#!/usr/bin/env /usr/local/bin/bash

# This script takes care of building your crate and packaging it for release

set -ex

export PATH=$HOME/.cargo/bin:$PATH

main() {
    export ghr_exe=/tmp/ghr/ghr
    local src=$(pwd) stage=$(mktemp -d)
    os_name="$CIRRUS_OS"-$(freebsd-version -u | cut -f 1 -d '-') || os_name=freebsd-12.1
    arch=x86_$(getconf LONG_BIT) || arch=x86_64
    CIRRUS_SHA1=$(git rev-parse --verify HEAD) || true
    echo $CIRRUS_CHANGE_IN_REPO
    export artifacts=rmate_"$os_name"_"$arch.tar.gz"
    tar czvf "$artifacts" "target/release/rmate" || true
    ls -lh
    [ -f "$artifacts" ] || true
    echo ${CIRRUS_REPO_OWNER} ${CIRRUS_REPO_NAME} ${CIRRUS_SHA1} ${CIRRUS_TAG} ${artifacts}
    [ -f "$ghr_exe" ] && ls -lh "$ghr_exe"
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRRUS_REPO_OWNER} -r ${CIRRUS_REPO_NAME} -c ${CIRRUS_SHA1} -delete ${CIRRUS_TAG} ${artifacts} || true
}

if [ -n "$CIRRUS_TEST" ]; then
    echo "CIRRUS_TEST is set, exitting"
    exit 1 || true
fi
if [ -z "$CIRRUS_TAG" ]; then
    echo "Not a tagged commit, exitting."
    exit 1 || true
elif [ -z "$GITHUB_TOKEN" ]; then
    echo "Github access token not set, exitting."
    exit 2 || true
else
    echo "This is a tagged commit, running before_deploy"
fi

main
