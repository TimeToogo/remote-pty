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
objcopy --redefine-syms=$DIR/symbols.map libremote_pty_slave.a libremote_pty_slave.renamed.a

# get a compiled musl libc static lib
echo "= downloading $ARCH musl libc.a"
docker run --rm muslcc/x86_64:$ARCH-linux-musl cat /$ARCH-linux-musl/lib/libc.a > musl-libc.a

# we prefix all libc symbols with __libc__*
# so our remote pty static lib can call the original musl impl's
echo "= creating prefixed musl libc.a"
objcopy --prefix-symbols=__libc__ musl-libc.a musl-libc.prefixed.a
# ensure __errno_location does not get prefixed so all libc's 
# point to the same errno
objcopy --redefine-sym __libc_____errno_location=__errno_location musl-libc.prefixed.a
objcopy --redefine-sym __libc____errno_location=__errno_location musl-libc.prefixed.a

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
find muslobjects/ -type f -exec bash -c '[[ $(nm {} 2>&1 | awk "\$2 == \"T\"" | wc -l) == 0 ]] && rm -f {}' \;
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

echo "= creating shared lib from static lib"
gcc -Wl,-Map -Wl,mapfile -shared -fPIC -flto -o libremote_pty_slave.linked.so \
    -Wl,--whole-archive libremote_pty_slave.linked.a \
    -Wl,--no-whole-archive \
    -Wl,-lpthread
# ensure our __errno_location is used when the shared library is loaded
objcopy --globalize-symbol __errno_location libremote_pty_slave.linked.so

echo "= done"