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

# get a compiled musl libc static lib
echo "= downloading $ARCH musl libc.a"
docker run --rm muslcc/x86_64:$ARCH-linux-musl cat /$ARCH-linux-musl/lib/libc.a > musl-libc.a

# we prefix all libc symbols with __libc__*
# so our remote pty static lib can link against the musl impl's
echo "= creating prefixed musl libc.a"
objcopy --prefix-symbols=__libc__ musl-libc.a musl-libc.prefixed.a

# perform the linking to original impl's into a shared library
# in order to discover the required symbols from musl
# this is mildly overkill as we mostly know from the symbol.map file but oh well
echo "= linking against musl libc"
gcc -Wl,-Map -Wl,ld.mapfile -nostdlib -nodefaultlibs -shared -fPIC -o /dev/null \
    -Wl,--whole-archive libremote_pty_slave.renamed.a \
    -Wl,--no-whole-archive musl-libc.prefixed.a

echo "= finding required libc symbols"
grep -Po '(__libc__[a-z0-9_]+|__errno_location)' ld.mapfile > libc.keepsyms
objcopy --strip-all --discard-all --keep-symbols=libc.keepsyms musl-libc.prefixed.a musl-libc.copied.a
rm -rf muslobjects &&  mkdir -p muslobjects
ar x musl-libc.copied.a --output=muslobjects
# remove all objects with empty symtab
find muslobjects/ -type f -exec bash -c '[[ $(nm {} 2>&1 | grep "no symbols") > 0 ]] && rm -f {}' \;
# prefix object files to prevent collisions and recombine into static lib
find muslobjects/ -type f -exec bash -c 'mv {} $(dirname {})/__libc__$(basename {})' \;
rm -f musl-libc.filtered.a
ar crs musl-libc.filtered.a muslobjects/*

echo "= embedding musl libc symbols into combined static lib"
rm -rf combinedlib/
mkdir -p combinedlib
ar x libremote_pty_slave.renamed.a --output=combinedlib/
ar x musl-libc.filtered.a --output=combinedlib/
rm -f libremote_pty_slave.linked.a
ar crs libremote_pty_slave.linked.a combinedlib/*

echo "= done"