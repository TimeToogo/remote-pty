#!/bin/bash

set -e

DIR=$(realpath $(dirname $0))

target=$1

if [[ -z "$target" ]];
then
    echo "usage: $0 [rust target]"
    exit 1
fi

ARCH=$(echo "$target" | cut -d'-' -f1)

cd $DIR

echo "= cargo build"
cargo build --release --target $target

TARGETDIR=${CARGO_TARGET_DIR:-$DIR/../../target}/$target/release
cd $TARGETDIR

echo "= creating shared lib"
gcc -shared -fPIC -flto -static-libgcc \
    -o libremote_pty_slave.linked.so \
    -Wl,--whole-archive ./libremote_pty_slave.a \
    -Wl,--no-whole-archive \
    -Wl,-lpthread \
    -Wl,-ldl

echo "= done"