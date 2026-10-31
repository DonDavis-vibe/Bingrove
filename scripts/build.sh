#!/usr/bin/env bash
# Builds bingrove.wasm, the Meshgrove plugin:
#
#   scripts/build.sh            → target/plugin/wasm32-wasip2/release/bingrove.wasm
#   scripts/build.sh --debug    → …/debug/bingrove.wasm (fast build, slow geometry)
#
# Needs `rustup target add wasm32-wasip2` once. A distribution's Rust (Fedora's /usr/bin/cargo)
# often comes first in PATH and has no wasm targets, so rustup's stable toolchain is used.
set -euo pipefail
root=$(realpath "$(dirname "$0")/..")
profile=(--release)
dir=release
if [ "${1:-}" = "--debug" ]; then
    profile=()
    dir=debug
fi
rustup=$(command -v rustup || echo "$HOME/.cargo/bin/rustup")
cargo=(cargo)
if [ -x "$rustup" ]; then
    cargo=("$rustup" run stable cargo)
    RUSTC=$("$rustup" which --toolchain stable rustc)
    export RUSTC
fi
"${cargo[@]}" build --manifest-path "$root/crates/bingrove-plugin/Cargo.toml" --target wasm32-wasip2 \
    --target-dir "$root/target/plugin" ${profile[@]+"${profile[@]}"}
out=$root/target/plugin/wasm32-wasip2/$dir/bingrove_plugin.wasm
cp "$out" "$root/target/plugin/bingrove.wasm"
ls -l "$root/target/plugin/bingrove.wasm"
