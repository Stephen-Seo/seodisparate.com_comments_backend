#!/usr/bin/env bash

set -ve

CC_aarch64_linux_gnu=/usr/bin/aarch64-linux-gnu-gcc

/usr/bin/env \
    CC_aarch64_linux_gnu=${CC_aarch64_linux_gnu} \
    CXX_aarch64_linux_gnu=/usr/bin/aarch64-linux-gnu-g++ \
    AR_aarch64_linux_gnu=/usr/bin/aarch64-linux-gnu-ar \
    RANLIB_aarch64_linux_gnu=/usr/bin/aarch64-linux-gnu-ranlib \
    LD_aarch64_linux_gnu=/usr/bin/aarch64-linux-gnu-ld \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CC_aarch64_linux_gnu}" \
    cargo build --target aarch64-unknown-linux-gnu "$@"
