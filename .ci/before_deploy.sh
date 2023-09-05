#!/usr/bin/env bash

cp target/release/rmate .
# tar cvzf rmate_"$(uname -m)"_FreeBSD-"$(uname -K)".tar.gz rmate
tar cvzf rmate_FreeBSD.tar.gz rmate
