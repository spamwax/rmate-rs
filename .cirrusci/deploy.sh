#!/usr/bin/env /usr/local/bin/bash

# This script takes care of building your crate and packaging it for release

set -ex

export PATH=$HOME/.cargo/bin:$PATH

main() {
    exoprt ghr_exe=/tmp/ghr/ghr
    export || true
    env || true
    setenv || true
    local src=$(pwd) stage=$(mktemp -d)
    os_name=$(uname -a | tr -s '[:blank:]' ' ' | cut -f 2 -d ' ') || os_name=freebsd-12.1
    arch=$(uname -a | tr -s '[:blank:]' ' ' | cut -f 8 -d ' ') || arch=amd64
    CIRRUS_SHA1=$(git rev-parse --verify HEAD)
    tar czvf "rmate_$os_name_$arch.tar.gz" "target/release/rmate"
    export artifacts=rmate.tar.gz
    echo ${CIRRUS_REPO_OWNER} ${CIRRUS_REPO_NAME} ${CIRRUS_SHA1} ${CIRCLE_TAG} ${artifacts}
    [ -f "$ghr" ] && ls -lh "$ghr"
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRCLE_PROJECT_USERNAME} -r ${CIRCLE_PROJECT_REPONAME} -c ${CIRRUS_SHA1} -delete ${CIRRUS_TAG} ${artifacts} || true
}

if [ -n "$CIRRUS_TEST" ]; then
    echo "CIRCLE_TEST is set, exitting"
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
