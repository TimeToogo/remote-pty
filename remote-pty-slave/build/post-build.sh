#!/bin/bash

DIR=$(dirname $0)

target=$1

if [[ -z "$target" ]];
then
    echo "usage: $0 [target]"
    exit 1
fi

# rename symbols so our pty functions override libc's
objcopy --redefine-syms=$DIR/symbols.map $DIR/../../target/$target/release/libremote_pty_slave.a