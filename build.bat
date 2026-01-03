@echo off

echo Loading environment variables from .env.local...
for /f "tokens=1,2 delims==" %%a in (.env.local) do (
    if not "%%a"=="" (
        set %%a=%%b
    )
)

echo Building for WASM...
set RUSTFLAGS=--cfg getrandom_backend="wasm_js"
cargo build --release --target wasm32-unknown-unknown

echo Generating WASM bindings...
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

echo.
echo Build complete! Serve the files with a local server:
echo python -m http.server 8000
echo Then open http://localhost:8000 in your browser
echo.
echo --- TRAINING INSTRUCTIONS ---
echo To train the semantic model offline (using your full dataset):
echo 1. Run: cargo run --release --bin trainer
echo 2. Wait for it to finish (this creates trained_brain.bin)
echo 3. Run this build.bat again to embed the weights into the WASM app
echo.
pause
