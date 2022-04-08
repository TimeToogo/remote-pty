#!/bin/bash

DIR=$(dirname $0)

# rename symbols so our pty functions override libc's
objcopy --redefine-syms=$DIR/symbols.map $DIR/../../target/x86_64-unknown-linux-musl/debug/libremote_pty_slave.a