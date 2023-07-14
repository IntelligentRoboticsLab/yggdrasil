#!/bin/bash


if [ "$(uname)" = "Darwin" ]; then
    # set cross compile flags for macos
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
    export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
    export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"
    export TARGET_CC=$(which clang)
    export CC_X86_64_UNKNOWN_LINUX_GNU=x86_64-unknown-linux-gnu-gcc
    export CXX_X86_64_UNKNOWN_LINUX_GNU=x86_64-unknown-linux-gnu-g++
    export AR_X86_64_UNKNOWN_LINUX_GNU=x86_64-unknown-linux-gnu-ar
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc
fi

# build binary
cargo b -r --target=x86_64-unknown-linux-gnu

# set interpreter to proper interpreter in compiled binary
# todo: might not be necessary on linux
patchelf target/x86_64-unknown-linux-gnu/release/yggdrasil --set-interpreter /lib/ld-linux-x86-64.so.2

# copy binary to nao
scp target/x86_64-unknown-linux-gnu/release/yggdrasil nao@10.1.8.$1:~/
ssh nao@10.1.8.$1 -t ./yggdrasil
