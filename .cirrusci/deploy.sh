#!/usr/bin/env /usr/local/bin/bash

# This script takes care of building your crate and packaging it for release

set -ex


main() {
    local src=$(pwd) stage=$(mktemp -d)
    export ghr_exe="$src/ghr/ghr"

    os_name="$CIRRUS_OS"-$(freebsd-version -u | cut -f 1 -d '-') || os_name=freebsd-12.1
    arch=x86_$(getconf LONG_BIT) || arch=x86_64

    export artifacts=rmate_"$os_name"_"$arch.tar.gz"
    tar czvf "$artifacts" "target/release/rmate" || true
    [ -f "$artifacts" ] || true

    CIRRUS_SHA1=$CIRRUS_CHANGE_IN_REPO
    echo ${CIRRUS_REPO_OWNER} ${CIRRUS_REPO_NAME} ${CIRRUS_SHA1} ${CIRRUS_TAG} ${artifacts}

    [ -f "$ghr_exe" ] && ls -lh "$ghr_exe"
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRRUS_REPO_OWNER} -r ${CIRRUS_REPO_NAME} -c ${CIRRUS_SHA1} -delete ${CIRRUS_TAG} ${artifacts} || true
}

if [[ -n "$CIRRUS_TEST" || ( "$CIRRUS_BRANCH" == 'master' && -z "$CIRRUS_TAG" ) ]]; then
    echo "This is a test or marster commit, FreeBSD CI only builds tagged releases."
elif [ -n "$CIRRUS_TAG" ]; then
    if [ -z "$GITHUB_TOKEN" ]; then
        echo "Github access token not set, exitting."
    else
        echo "This is a tagged commit, running deploy.sh"
        main
    fi
else
    echo "Derp... (branch: ${CIRRUS_BRANCH})"
fi

