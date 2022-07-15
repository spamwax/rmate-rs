#!/usr/bin/env bash
set -ex

create_linux() {
  pwd
  artifacts=rmate_"$TARGET".tar.gz
  strip_cmd="strip"

  if [[ $TARGET == *"aarch64"* ]]; then
    strip_cmd="/usr/bin/aarch64-linux-gnu-strip"
  elif [[ $TARGET == *"arm"* ]]; then
    strip_cmd="/usr/bin/arm-linux-gnueabi-strip"
  fi
  cp target/"$TARGET/$BUILD_TYPE"/rmate . || true
  "$strip_cmd" rmate || true
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

if [[ $TARGET == *"linux"* ]]; then
    create_linux
else
    create_macos
fi

