#!/bin/bash
# 
# adapted from https://github.com/robxu9/bash-static
#
# build static bash because we need exercises in minimalism
# Copyright © 2015 Robert Xu <robxu9@gmail.com>
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the “Software”), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.
#
# For Linux, also builds musl for truly static linking if
# musl is not installed.

set -e 
set -o pipefail

DIR=$(realpath $(dirname $0))
# DIR=/tmp/bash
SDIR=$(realpath $(dirname $0))

mkdir -p $DIR
cd $DIR

# load version info
bash_version="5.1"
bash_patch_level=16
musl_version="1.2.3"
kernel_headers_version="4.19.88-1"
busybox_version="1.34.1"
busybox_sha256="415fbd89e5344c96acf449d94a6f956dbed62e18e835fc83e064db33a34bd549"

target="$1"
arch="$2"

if [[ "$target" == "" ]]; then
  echo "! no target specified" >&2
  exit 1
fi

if [[ "$arch" == "" ]]; then
  echo "! no arch specified" >&2
  exit 1
fi

mkdir -p build # make build directory
pushd build

# pre-prepare gpg for verificaiton
echo "= preparing gpg"
GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
# public key for bash
gpg --batch --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys 7C0135FB088AAF6C66C650B9BB5869F064EA74AB
# public key for musl
gpg --batch --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys 836489290BB6B70F99FFDA0556BCDB593020450F
# public key for busybox
gpg --batch --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C9E9416F76E610DBD09D040F47B70C55ACC9965B

# download tarballs
echo "= downloading bash"
curl -LO http://ftp.gnu.org/gnu/bash/bash-${bash_version}.tar.gz
curl -LO http://ftp.gnu.org/gnu/bash/bash-${bash_version}.tar.gz.sig
gpg --batch --verify bash-${bash_version}.tar.gz.sig bash-${bash_version}.tar.gz

echo "= extracting bash"
rm -rf bash-${bash_version}
tar -xf bash-${bash_version}.tar.gz

echo "= patching bash"
bash_patch_prefix=$(echo "bash${bash_version}" | sed -e 's/\.//g')
for lvl in $(seq $bash_patch_level); do
    curl -LO http://ftp.gnu.org/gnu/bash/bash-${bash_version}-patches/"${bash_patch_prefix}"-"$(printf '%03d' "$lvl")"
    curl -LO http://ftp.gnu.org/gnu/bash/bash-${bash_version}-patches/"${bash_patch_prefix}"-"$(printf '%03d' "$lvl")".sig
    gpg --batch --verify "${bash_patch_prefix}"-"$(printf '%03d' "$lvl")".sig "${bash_patch_prefix}"-"$(printf '%03d' "$lvl")"

    pushd bash-${bash_version}
    patch -p0 < ../"${bash_patch_prefix}"-"$(printf '%03d' "$lvl")"
    popd
done

# apply custom patches
echo "= applying custom patches"
pushd bash-${bash_version}
patch -p1 < $SDIR/patches/bash-busybox-builtin.patch
popd

configure_args=()

if [ "$(grep ID= < /etc/os-release | head -n1)" = "ID=alpine" ]; then
  echo "= skipping installation of musl because this is alpine linux (and it is already installed)"
else
  echo "= downloading musl"
  curl -LO https://musl.libc.org/releases/musl-${musl_version}.tar.gz
  curl -LO https://musl.libc.org/releases/musl-${musl_version}.tar.gz.asc
  gpg --batch --verify musl-${musl_version}.tar.gz.asc musl-${musl_version}.tar.gz

  echo "= extracting musl"
  tar -xf musl-${musl_version}.tar.gz

  echo "= building musl"
  working_dir=$(pwd)

  install_dir=${working_dir}/musl-install

  pushd musl-${musl_version}
  ./configure --prefix="${install_dir}" 
  make -j "$(nproc)" install
  popd # musl-${musl-version}

  echo "= setting CC to musl-gcc"
  export CC=${working_dir}/musl-install/bin/musl-gcc

  echo "= downloading musl-compatible kernel headers"

  curl -fL -o kernel-headers.tar.gz https://github.com/sabotage-linux/kernel-headers/archive/refs/tags/v$kernel_headers_version.tar.gz
  mkdir -p ${working_dir}/kernel-headers/
  tar -xf kernel-headers.tar.gz -C ${working_dir}/kernel-headers/ --strip-components 1
fi

echo "= downloading busybox"

tarball="busybox-${busybox_version}.tar.bz2";
curl -fL -o busybox.tar.bz2.sig "https://busybox.net/downloads/$tarball.sig";
curl -fL -o busybox.tar.bz2 "https://busybox.net/downloads/$tarball";
echo "$busybox_sha256 *busybox.tar.bz2" | sha256sum -c -;
gpg --batch --verify busybox.tar.bz2.sig busybox.tar.bz2;
rm -rf ./busybox
mkdir -p ./busybox;
tar -xf busybox.tar.bz2 -C ./busybox --strip-components 1;
rm busybox.tar.bz2*

pushd busybox

echo "= building libbusybox"

setConfs='
  CONFIG_LAST_SUPPORTED_WCHAR=0
  CONFIG_BUILD_LIBBUSYBOX=y
  CONFIG_FEATURE_LIBBUSYBOX_STATIC=y
';

make defconfig;

for confV in $setConfs; do
  conf="${confV%=*}";
  sed -i \
    -e "s!^$conf=.*\$!$confV!" \
    -e "s!^# $conf is not set\$!$confV!" \
    .config;
  if ! grep -q "^$confV\$" .config; then
    echo "$confV" >> .config;
  fi;
done;
make oldconfig

# inject script to build raw object file
sed -i -e 's/^exit 0.*$/\#/' scripts/trylink
echo '
  echo "= building static lib to $sharedlib_dir/libbusybox.a"
  $CC $CFLAGS $LDFLAGS \
      -r -o "$sharedlib_dir/libbusybox.o" \
      -fPIC -Wl,-static \
      -Wl,--undefined=lbb_main \
      $SORT_COMMON \
      $SORT_SECTION \
      $START_GROUP $A_FILES $END_GROUP \
      $l_list \
      `INFO_OPTS`

  exit 0
' >> scripts/trylink

make CC="$CC" CFLAGS="$CFLAGS -I${working_dir}/kernel-headers/$arch/include/ -fPIC" -j "$(nproc)"

# prefix defined symbols so we dont cause any conflicts  
cp 0_lib/libbusybox.o 0_lib/libbusybox.prefixed.o
nm 0_lib/libbusybox.o | \
  grep -E ' ([a-tv-z]|[A-TV-Z]) ' | \
  awk '{printf "%s bb_%s\n", $3, $3}' | \
  sort | uniq > /tmp/bbsym.map
objcopy --redefine-syms /tmp/bbsym.map 0_lib/libbusybox.prefixed.o

popd

echo "= building bash"

REMOTE_PTY_LIB="${CARGO_TARGET_DIR:-$SDIR/../target}/$arch-unknown-$target-musl/release/libremote_pty_slave.linked.a"
BUSYBOX_LIB="$DIR/build/busybox/0_lib/libbusybox.prefixed.o"

pushd bash-${bash_version}
autoconf -f
# statically link against our remote-pty-slave library
# overriding the musl tty functions
LDFLAGS="-Wl,-zmuldefs -Wl,--whole-archive $REMOTE_PTY_LIB -Wl,--no-whole-archive $BUSYBOX_LIB" \
  CFLAGS="$CFLAGS -static -Os" \
  ./configure --without-bash-malloc "${configure_args[@]}" || (cat config.log && exit 1)
make CC="$CC"
make tests
popd # bash-${bash_version}

popd # build

if [ ! -d releases ]; then
  mkdir releases
fi

echo "= extracting bash binary"
cp build/bash-${bash_version}/bash releases

echo "= done"
