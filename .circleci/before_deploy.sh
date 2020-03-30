#!/bin/bash
# This script takes care of building your crate and packaging it for release

set -ex

main() {
    case $CIRCLE_OS_NAME in
        linux)
            create_tar
            ;;
        macos)
            build_mac_artifact
            ;;
        *)
            echo "$CIRCLE_OS_NAME not a supported OS"
            exit 1
    esac

    # case $TARGET in
    #     x86_64-apple-darwin)
    #         build_mac_artifact
    #         ;;
    #     i686-unknown-linux-gnu)
    #         create_tar
    #         ;;
    #     x86_64-unknown-linux-gnu|arm-unknown-linux-gnueabihf|armv7-unknown-linux-gnueabihf|armv7-unknown-linux-gnueabi)
    #         create_tar
    #         ;;
    #     arm-unknown-linux-gnueabihf|armv7-unknown-linux-gnueabihf)
    #         create_tar
    #         ;;
    #     aarch64-unknown-linux-gnu)
    #         create_tar
    #         ;;
    #     *)
    #         return
    #         ;;
    # esac

}

create_tar() {
  echo `pwd`
  ls -la
  artifacts=rmate_"$TARGET".tar.gz
  strip_cmd=strip

  if [[ $TARGET == *"aarch64"* ]]; then
    strip_cmd="/usr/bin/aarch64-linux-gnu-strip"
  elif [[ $TARGET == *"arm"* ]]; then
    strip_cmd="/usr/bin/arm-linux-gnueabi-strip"
  fi
  "$strip_cmd" target/$TARGET/release/rmate || true
  tar czvf "$artifacts" "target/$TARGET/release/rmate"
  mv "$artifacts" /tmp
}

build_mac_artifact() {
    echo `pwd`
    ls -la
    artifacts=rmate_"$TARGET".zip
    strip target/$TARGET/release/rmate || true
    zip "$artifacts" target/$TARGET/release/rmate
    mv "$artifacts" /tmp
    # src=$1
    # stage=$2
    # test -f Cargo.lock || cargo generate-lockfile

    # # TODO Update this to build the artifacts that matter to you
    # # cross rustc --bin alfred-pinboard-rs --target "$TARGET" --release -- -C lto

    # # TODO Update this to package the right artifacts
    # # res_dir="$src/res/workflow"
    # res_dir="$src/res/workflow/"

    # # echo "Copying executable to workflow's folder..."
    # cp "$src/target/$TARGET/release/alfred-pinboard-rs" "$stage"
    # cp "$res_dir"/* "$stage"

    # # echo "Creating the workflow bundle..."
    # cd "$stage" || exit
    # strip ./alfred-pinboard-rs || true
    # rm -f AlfredPinboardRust.alfredworkflow

    # zip -r AlfredPinboardRust.alfredworkflow ./*

    # case $TARGET in
    #     x86_64-apple-darwin)
    #         mv ./AlfredPinboardRust.alfredworkflow "$src/target/alfred-pinboard-rust-$CIRCLE_TAG.alfredworkflow"
    #         ;;
    #     i686-apple-darwin)
    #         tar czf "$src/$TARGET-$CRATE_NAME-$CIRCLE_TAG.tar.gz" ./AlfredPinboardRust.alfredworkflow
    #         ;;
    #     *)
    #         return
    #         ;;
    # esac
    # cd "$src"

}

# if [[ "$TARGET" == "i686-unknown-linux-gnu" ]]; then
  # source /root/.cargo/env
  # source /home/circleci/.cargo/env
# fi
pwd
if [ -z "$CIRCLE_TAG" ]; then
    echo "Not a tagged commit. Exitting"
    exit 1
else
    echo "This is a tagged commit, running before_deploy"
fi

main
