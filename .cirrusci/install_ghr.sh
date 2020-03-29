#!/usr/local/bin/bash

set -ex

echo $SHELL
echo $HOME
echo $(pwd)
which cargo || true
cd /tmp && git clone https://github.com/tcnksm/ghr
cd ghr && go build || true
