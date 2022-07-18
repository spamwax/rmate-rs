#!/usr/bin/env bash
set -ex

create_linux() {
  pwd
  artifacts=rmate_"$TARGET".tar.gz
  strip_cmd=

  if [[ $TARGET == "aarch64-unknown-linux-gnu" ]]; then
    strip_cmd="/usr/bin/aarch64-linux-gnu-strip"
  elif [[ $TARGET == "armv7-unknown-linux-gnueabihf" ]]; then
    strip_cmd="/usr/bin/arm-linux-gnueabihf-strip"
  elif [[ $TARGET == "x86_64-unknown-linux-gnu" || $TARGET == "i686-unknown-linux-gnu" ]]; then
    strip_cmd=$(which strip)
  fi

  cp target/"$TARGET/$BUILD_TYPE"/rmate . || true
  if [[ -n "$strip_cmd" ]]; then
    "$strip_cmd" rmate || true
  fi

  tar czvf "$artifacts" rmate
  ls -la
}

create_macos() {
    pwd
    artifacts=rmate_"$TARGET".zip
    cp target/"$TARGET/$BUILD_TYPE"/rmate . || true
    strip rmate || true
    zip "$artifacts" rmate
    ls -la
}

if [ -z "$RELEASE_COMMIT" ]; then
    echo "Not a tagged commit. Exiting."
    exit 1
fi


echo "Preparing release for $TARGET"

if [[ $TARGET == *"apple"* ]]; then
    create_macos
else
    create_linux
fi

