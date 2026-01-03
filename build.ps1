if (!(Test-Path "dist")) { New-Item -ItemType Directory -Path "dist" }
if (!(Test-Path "pkg")) { New-Item -ItemType Directory -Path "pkg" }

if (Test-Path ".env.local") {
    Write-Host "Loading environment variables from .env.local..."
    $envContent = Get-Content ".env.local" | Where-Object { $_ -notmatch '^#' -and $_.Trim() -ne '' }
    foreach ($line in $envContent) {
        $key, $value = $line -split '=', 2
        Set-Item -Path "env:$key" -Value $value
    }
}

Write-Host "Building for WASM..."
$env:TURSO_DATABASE_URL = $env:TURSO_DATABASE_URL
$env:TURSO_AUTH_TOKEN = $env:TURSO_AUTH_TOKEN
cargo build --release --target wasm32-unknown-unknown --lib

Write-Host "Generating WASM bindings..."
wasm-bindgen --out-dir pkg --target web target/wasm32-unknown-unknown/release/vecors.wasm

Write-Host "Copying files to dist/..."
Copy-Item "index.html" "dist/"
Copy-Item "web_config.js" "dist/"
Copy-Item -Recurse "pkg" "dist/"

Write-Host "Build complete! Files are in dist/"
Write-Host "Serve with: python -m http.server 8000 --directory dist"
