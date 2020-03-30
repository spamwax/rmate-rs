#!/bin/bash
# This script takes care of deploying your artifacts to github
set -ex

deploy() {
    prev_dir=$(pwd)
    cd /tmp
    ghr_v0.13.0_linux_amd64.tar.gz
    curl -L -O https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/ghr_"$GHRELEASER_VERSION"_linux_amd64.tar.gz
    tar xzvf ghr_"$GHRELEASER_VERSION"_linux_amd64.tar.gz
    # unzip ghr_"$GHRELEASER_VERSION"_darwin_amd64.zip
    ghr_exe=$(pwd)/ghr_"$GHRELEASER_VERSION"_linux_amd64/ghr
    cd "$prev_dir"
    export artifacts=/tmp/rmate_$TARGET.tar.gz
    [ -f "$artifacts" ] || true
    echo ${CIRCLE_PROJECT_USERNAME} ${CIRCLE_PROJECT_REPONAME} ${CIRCLE_SHA1} ${CIRCLE_TAG} ${artifacts}
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRCLE_PROJECT_USERNAME} -r ${CIRCLE_PROJECT_REPONAME} -c ${CIRCLE_SHA1} -delete ${CIRCLE_TAG} ${artifacts}
}

if [ -n "$CIRCLE_TEST" ]; then
    echo "CIRCLE_TEST is set, exitting"
fi
if [ -z "$CIRCLE_TAG" ]; then
    echo "Not a tagged commit, exitting."
    exit 1
elif [ -z "$GITHUB_TOKEN" ]; then
    echo "Github access token not set, exitting."
fi

if [ -z "$GHRELEASER_VERSION" ]; then
    echo "ghr version was not set using v0.13.0"
    export GHRELEASER_VERSION="v0.13.0"
fi

deploy
