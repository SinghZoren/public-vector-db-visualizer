if (Test-Path ".env.local") {
    Write-Host "Loading environment variables from .env.local..."
    $envContent = Get-Content ".env.local" | Where-Object { $_ -notmatch '^#' -and $_.Trim() -ne '' }
    foreach ($line in $envContent) {
        $key, $value = $line -split '=', 2
        Set-Item -Path "env:$key" -Value $value
    }
} else {
    Write-Host "Warning: .env.local not found. Make sure to create it with TURSO_DATABASE_URL and TURSO_AUTH_TOKEN"
}

echo "Building for WASM..."
export TURSO_DATABASE_URL=$TURSO_DATABASE_URL
export TURSO_AUTH_TOKEN=$TURSO_AUTH_TOKEN
cargo build --release --target wasm32-unknown-unknown --lib

Write-Host "Generating WASM bindings..."
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

Write-Host "Build complete! Serve the files with a local server:"
Write-Host "python -m http.server 8000"
Write-Host "Then open http://localhost:8000 in your browser"
