#!/bin/bash

# Ensure dist directory exists
mkdir -p dist

# Ensure pkg directory exists
mkdir -p pkg

echo "Building for WASM..."
# Note: TURSO env vars should be provided by the environment during build
cargo build --release --target wasm32-unknown-unknown --lib

echo "Generating WASM bindings..."
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

echo "Copying files to dist/..."
cp index.html dist/
cp web_config.js dist/
cp -r pkg dist/
# If there are any assets, copy them too
if [ -d "assets" ]; then
    cp -r assets dist/
fi

echo "Build complete! Files are in dist/"
