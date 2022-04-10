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

# # rename symbols so our pty functions override libc's
# # we override the symbols pointing to libc with the prefix "libc_" so they
# # can be later linked to their libc impl's
# #
# # we then rename our versions (intercept_*) to the original libc names
# # so ours will override those provided by libc
# echo "= renaming our lib's symbols"
# objcopy --redefine-syms=$DIR/symbols.map $TARGETDIR/libremote_pty_slave.a

# # get a compiled musl libc static lib
# echo "= downloading $ARCH musl libc.a"
# [ ! -f $TARGETDIR/musl-libc.a ] && docker run --rm muslcc/x86_64:$ARCH-linux-musl cat /$ARCH-linux-musl/lib/libc.a > $TARGETDIR/musl-libc.a

# # we prefix all libc symbols with libc_*
# # so our remote pty static lib can call the original musl impl's
# echo "= creating prefixed musl libc.a"
# cp $TARGETDIR/musl-libc.a $TARGETDIR/musl-libc.prefixed.a
# objcopy --prefix-symbols=libc_ $TARGETDIR/musl-libc.prefixed.a

# perform the linking to original impl's into a shared library
echo "= linking against musl"
gcc -nostdlib -nodefaultlibs -shared -static -fPIC -o $TARGETDIR/libremote_pty_slave.linked.a \
    -Wl,--whole-archive $TARGETDIR/libremote_pty_slave.a \
    -Wl,--no-whole-archive $TARGETDIR/musl-libc.prefixed.a

echo "= done"