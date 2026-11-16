#!/usr/bin/env bash
# Everything CI would check: format, clippy and tests of the native workspace, clippy of the
# plugin for wasm32-wasip2, the plugin's native tests, and that the plugin builds.
#
# For the wasm target the whole stable toolchain of rustup has to come first in PATH: with a
# distribution's rustc first (Fedora), clippy-driver takes its sysroot and finds no std for wasm.
set -euo pipefail
root=$(realpath "$(dirname "$0")/..")
cd "$root"
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
rustup=$(command -v rustup || echo "$HOME/.cargo/bin/rustup")
if [ -x "$rustup" ]; then
    toolchain_bin=$(dirname "$("$rustup" which --toolchain stable rustc)")
    export PATH="$toolchain_bin:$PATH" RUSTUP_TOOLCHAIN=stable
fi
cargo fmt --manifest-path crates/bingrove-plugin/Cargo.toml --check
cargo clippy --manifest-path crates/bingrove-plugin/Cargo.toml --target wasm32-wasip2 \
    --target-dir target/plugin -- -D warnings
# The plugin's own tests run natively: its tools must stay within what Meshgrove accepts.
cargo test --manifest-path crates/bingrove-plugin/Cargo.toml --lib --target-dir target/plugin-native
scripts/build.sh
