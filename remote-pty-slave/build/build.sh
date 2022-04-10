#!/bin/bash

set -e

DIR=$(dirname $0)

target=$1

if [[ -z "$target" ]];
then
    echo "usage: $0 [rust target]"
    exit 1
fi

ARCH=$(echo "$target" | cut -d'-' -f1)

echo "= cargo build"
cargo build --release --target $target

TARGETDIR=$DIR/../../target/$target/release

# rename symbols so our pty functions override libc's
# we override the symbols pointing to libc with the prefix "__libc__" so they
# can be later linked to their libc impl's
#
# we then rename our versions (intercept_*) to the original libc names
# so ours will override those provided by libc
echo "= renaming our lib's symbols"
objcopy --redefine-syms=$DIR/symbols.map $TARGETDIR/libremote_pty_slave.a $TARGETDIR/libremote_pty_slave.renamed.a

# get a compiled musl libc static lib
echo "= downloading $ARCH musl libc.a"
[ ! -f $TARGETDIR/musl-libc.a ] && (docker run --rm muslcc/x86_64:$ARCH-linux-musl cat /$ARCH-linux-musl/lib/libc.a > $TARGETDIR/musl-libc.a)

# we prefix all libc symbols with __libc__*
# so our remote pty static lib can call the original musl impl's
echo "= creating prefixed musl libc.a"
cp $TARGETDIR/musl-libc.a $TARGETDIR/musl-libc.prefixed.a
objcopy --prefix-symbols=__libc__ $TARGETDIR/musl-libc.prefixed.a

# perform the linking to original impl's into a shared library
# in order to discover the required symbols from musl
# this is mildly overkill as we mostly know from the symbol.map file but oh well
echo "= linking against musl libc"
gcc -Wl,-Map -Wl,ld.mapfile -nostdlib -nodefaultlibs -shared -fPIC -o /dev/null \
    -Wl,--whole-archive $TARGETDIR/libremote_pty_slave.renamed.a \
    -Wl,--no-whole-archive $TARGETDIR/musl-libc.prefixed.a

echo "= finding required libc symbols"
grep -Po '(__libc__[a-z0-9_]+)' ld.mapfile > libc.keepsyms
objcopy --strip-all --discard-all --keep-symbols=libc.keepsyms musl-libc.prefixed.a musl-libc.copied.a
rm -rf $TARGETDIR/muslobjects &&  mkdir -p $TARGETDIR/muslobjects
ar x musl-libc.copied.a --output=$TARGETDIR/muslobjects
# remove all objects with empty symtab
find $TARGETDIR/muslobjects/ -type f -exec bash -c '[[ $(nm {} 2>&1 | awk "\$2 == \"T\"" | wc -l) == 0 ]] && rm -f {}' \;
# prefix object files to prevent collisions and recombine into static lib
find $TARGETDIR/muslobjects/ -type f -exec bash -c 'mv {} $(dirname {})/__libc__$(basename {})' \;
rm -f musl-libc.filtered.a
ar crs musl-libc.filtered.a $TARGETDIR/muslobjects/*

echo "= embedding musl libc symbols into combined static lib"
rm -rf $TARGETDIR/combinedlib/
mkdir -p $TARGETDIR/combinedlib
ar x libremote_pty_slave.renamed.a --output=$TARGETDIR/combinedlib/
ar x musl-libc.filtered.a --output=$TARGETDIR/combinedlib/
rm -f libremote_pty_slave_linked.a
ar crs libremote_pty_slave.linked.a $TARGETDIR/combinedlib/*

echo "= creating shared lib from static lib"
gcc -Wl,-Map -Wl,mapfile -shared -fPIC -flto -o ./libremote_pty_slave.linked.so \
    -Wl,--whole-archive $TARGETDIR/libremote_pty_slave.linked.a \
    -Wl,--no-whole-archive \
    -Wl,-lpthread

echo "= done"