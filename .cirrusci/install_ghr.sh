#!/usr/bin/env /usr/local/bin/bash

set -ex

echo "shell: $SHELL   home: $HOME"
pwd

git clone https://github.com/tcnksm/ghr
cd ghr && go build || true
