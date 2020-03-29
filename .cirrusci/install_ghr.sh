#!/usr/bin/env /usr/local/bin/bash

set -ex

echo $SHELL
echo $HOME
pwd

export PATH=$HOME/.cargo/bin:$PATH

which cargo || true

/usr/sbin/pkg install -y git

cd /tmp && git clone https://github.com/tcnksm/ghr
cd ghr && go build || true
