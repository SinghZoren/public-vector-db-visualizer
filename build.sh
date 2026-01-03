#!/bin/bash

# Exit on error
set -e

echo "Setting up build environment..."

# Ensure dist directory exists
mkdir -p dist

# Install wasm-bindgen-cli if not present (needed for Cloudflare/CI)
if ! command -v wasm-bindgen &> /dev/null
then
    echo "wasm-bindgen not found, installing..."
    # We use a specific version to match the wasm-bindgen crate version in Cargo.toml
    cargo install wasm-bindgen-cli --version 0.2.106
fi

# Add wasm target
rustup target add wasm32-unknown-unknown

echo "Building for WASM..."
# TURSO env vars should be provided by Cloudflare Pages Environment Variables
cargo build --release --target wasm32-unknown-unknown --lib

echo "Generating WASM bindings..."
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

echo "Copying files to dist/..."
cp index.html dist/
cp web_config.js dist/
cp -r pkg dist/

echo "Build complete! Files are in dist/"
