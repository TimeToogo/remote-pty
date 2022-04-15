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

echo "= cargo build"
cargo build --release --target $target

TARGETDIR=${CARGO_TARGET_DIR:-$DIR/../../target}/$target/release

cd $TARGETDIR

# rename symbols so our pty functions override libc's
# we override the symbols pointing to libc with the prefix "__libc__" so they
# can be later linked to their libc impl's
#
# we then rename our versions (intercept_*) to the original libc names
# so ours will override those provided by libc
echo "= renaming our lib's symbols"

# remove symbol versioning
# objcopy --remove-section .gnu.version --remove-section .gnu.version_r libremote_pty_slave.so libremote_pty_slave.renamed.so
# objcopy  libremote_pty_slave.so libremote_pty_slave.renamed.so
# nm libremote_pty_slave.renamed.so 2>&1 | grep '@GLIBC' | awk '{print $2}' | \
#     xargs -L1 sh -c 'objcopy --redefine-sym $0=$(echo $0 | cut -d"@" -f1) libremote_pty_slave.renamed.so'

# objcopy --redefine-syms=$DIR/symbols.map libremote_pty_slave.renamed.so

gcc -shared -fPIC -flto -static-libgcc \
    -o libremote_pty_slave.linked.so \
    -Wl,--whole-archive ./libremote_pty_slave.a \
    -Wl,--no-whole-archive \
    -Wl,-lpthread

echo "= done"