@echo off
REM Simple Windows batch file to build the project

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
pause