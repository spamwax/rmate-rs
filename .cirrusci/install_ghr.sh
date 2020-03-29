#!/usr/bin/env /usr/local/bin/bash

set -ex

echo "shell: $SHELL"
echo "home: $HOME"
pwd

# /usr/sbin/pkg install -y git

# cd /tmp && git clone https://github.com/tcnksm/ghr

[ -f `pwd`/ghr/ghr ] || ls -lh .
git clone https://github.com/tcnksm/ghr
cd ghr && go build || true
