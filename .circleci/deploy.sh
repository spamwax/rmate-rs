#!/bin/bash
# This script takes care of deploying your artifacts to github
set -ex

deploy() {
    prev_dir=$(pwd)
    cd /tmp
    curl -L -O https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/ghr_"$GHRELEASER_VERSION"_linux_amd64.tar.gz
    tar xzvf ghr_"$GHRELEASER_VERSION"_linux_amd64.tar.gz
    # unzip ghr_"$GHRELEASER_VERSION"_darwin_amd64.zip
    ghr_exe=$(pwd)/ghr_"$GHRELEASER_VERSION"_linux_amd64/ghr
    cd "$prev_dir"
    export artifacts=/tmp/rmate_$TARGET.tar.gz
    [ -f "$artifacts" ] || true
    export CIRCLE_PROJECT_USERNAME=listboss
    export CIRCLE_PROJECT_REPONAME=rmate-rust
    export CIRCLE_TAG=v1.0
    echo ${CIRCLE_PROJECT_USERNAME} ${CIRCLE_PROJECT_REPONAME} ${CIRCLE_SHA1} ${CIRCLE_TAG} ${artifacts}
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRCLE_PROJECT_USERNAME} -r ${CIRCLE_PROJECT_REPONAME} -c ${CIRCLE_SHA1} -delete ${CIRCLE_TAG} ${artifacts}
}

main() {
    prev_dir=$(pwd)
    cd /tmp
    case $CIRCLE_OS_NAME in
        linux)
            ghr_link=https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/ghr_"$GHRELEASER_VERSION"_linux_amd64.tar.gz
            ghr_archive=ghr_"$GHRELEASER_VERSION"_linux_amd64
            curl -L -O https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/$ghr_archive.tar.gz

            tar xzvf "$ghr_archive".tar.gz
            export artifacts=/tmp/rmate_$TARGET.tar.gz
            ;;
        macos)
            ghr_link=https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/ghr_"$GHRELEASER_VERSION"_darwin_amd64.zip
            ghr_archive=ghr_"$GHRELEASER_VERSION"_darwin_amd64
            curl -L -O https://github.com/tcnksm/ghr/releases/download/"$GHRELEASER_VERSION"/$ghr_archive.zip

            unzip "$ghr_archive".zip
            export artifacts=/tmp/rmate_$TARGET.zip
            ;;
        *)
            echo ">$CIRCLE_OS_NAME< not a supported CIRCLE_OS_NAME"
            exit 1
    esac
    ghr_exe=$(pwd)/$ghr_archive/ghr
    cd "$prev_dir"
    [ -f "$artifacts" ] || true && echo "WARNING!!!!!!!!!=========================================="
    ls -l "$artifacts"
    # export CIRCLE_PROJECT_USERNAME=listboss
    # export CIRCLE_PROJECT_REPONAME=rmate-rust
    # export CIRCLE_TAG=v1.0
    echo ${CIRCLE_PROJECT_USERNAME} ${CIRCLE_PROJECT_REPONAME} ${CIRCLE_SHA1} ${CIRCLE_TAG} ${artifacts}
    "$ghr_exe" -t ${GITHUB_TOKEN} -u ${CIRCLE_PROJECT_USERNAME} -r ${CIRCLE_PROJECT_REPONAME} -c ${CIRCLE_SHA1} -replace ${CIRCLE_TAG} ${artifacts}
}

pwd

# if [[ "$TARGET" == "i686-unknown-linux-gnu" ]]; then
  # source /root/.cargo/env
  # source /home/circleci/.cargo/env
# fi

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

echo "Running deploy.sh for tag: $CIRCLE_TAG"
main
