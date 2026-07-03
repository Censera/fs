#!/usr/bin/env bash
set -euo pipefail

NAME=fsize
OUT=dist

mkdir -p "$OUT"

TARGETS=(
    x86_64-unknown-linux-musl
    aarch64-unknown-linux-gnu
    armv7-unknown-linux-gnueabihf
    riscv64gc-unknown-linux-gnu
    x86_64-pc-windows-gnu
)

for target in "${TARGETS[@]}"; do
    echo "Building $target..."

    cargo build \
    --release \
    --target "$target"

    arch="${target%%-*}"

    if [[ "$target" == *windows* ]]; then
        cp \
        "target/$target/release/$NAME.exe" \
        "$OUT/${NAME}-windows-${arch}.exe"
        else
            cp \
            "target/$target/release/$NAME" \
            "$OUT/${NAME}-linux-${arch}"
            fi
            done

            echo
            ls -lh "$OUT"
