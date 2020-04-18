#!/usr/bin/env /usr/local/bin/bash

set -ex

if [[ -n "$CIRRUS_TEST" || ( "$CIRRUS_BRANCH" == 'master' && -z "$CIRRUS_TAG" ) ]]; then
    echo "This is a test or marster commit, FreeBSD CI only builds tagged releases."
elif [ -n "$CIRRUS_TAG" ]; then
    echo "shell: $SHELL   home: $HOME"
    pwd

    git clone https://github.com/tcnksm/ghr
    cd ghr && go build || true
else
    echo "Derp... (branch: ${CIRRUS_BRANCH})"
fi
