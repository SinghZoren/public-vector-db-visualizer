#!/bin/bash

# Exit on error
set -e

echo "Setting up build environment (v2)..."

# 1. Install Rust if missing
if ! command -v cargo &> /dev/null
then
    echo "Rust/Cargo not found. Installing Rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Rust/Cargo found: $(cargo --version)"
fi

# Ensure cargo is in PATH for the rest of the script
export PATH="$HOME/.cargo/bin:$PATH"

# 2. Add wasm target
rustup target add wasm32-unknown-unknown

# 3. Install wasm-bindgen-cli if not present
if ! command -v wasm-bindgen &> /dev/null
then
    echo "wasm-bindgen not found, installing..."
    # We use a specific version to match the wasm-bindgen crate version in Cargo.toml
    cargo install wasm-bindgen-cli --version 0.2.106
fi

echo "Building for WASM..."
# TURSO env vars should be provided by Cloudflare Pages Environment Variables
cargo build --release --target wasm32-unknown-unknown --lib

echo "Generating WASM bindings..."
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

echo "Copying files to dist/..."
mkdir -p dist
cp index.html dist/
cp web_config.js dist/
cp -r pkg dist/
# Copy the trained brain if it exists (Note: Cloudflare has a 25MB limit per file)
if [ -f "trained_brain.bin" ]; then
    cp trained_brain.bin dist/
fi

echo "Build complete! Files are in dist/"
