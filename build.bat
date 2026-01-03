@echo off

if not exist dist mkdir dist
if not exist pkg mkdir pkg

echo Loading environment variables from .env.local...
if exist .env.local (
    for /f "tokens=1,2 delims==" %%a in (.env.local) do (
        if not "%%a"=="" (
            set %%a=%%b
        )
    )
)

echo Building for WASM...
set RUSTFLAGS=--cfg getrandom_backend="wasm_js"
cargo build --release --target wasm32-unknown-unknown --lib

echo Generating WASM bindings...
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

echo Copying files to dist/...
copy index.html dist\
copy web_config.js dist\
if not exist dist\pkg mkdir dist\pkg
xcopy /y /s pkg\* dist\pkg\

echo.
echo Build complete! Files are in dist/
echo Serve with: python -m http.server 8000 --directory dist
echo.
pause
