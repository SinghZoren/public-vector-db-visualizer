# Public 3D Vector Space

A Bevy-powered 3D vector space that projects text into 3D coordinates using static projection and stores data in Turso.

## Setup

1. **Install Rust and WASM target:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32-unknown-unknown
   ```

2. **Install wasm-bindgen:**
   ```bash
   cargo install wasm-bindgen-cli
   ```

3. **Configure Turso credentials:**
   - Create a `.env.local` file in the project root:
     ```
     TURSO_DATABASE_URL=https://your-database-name.turso.io
     TURSO_AUTH_TOKEN=your-auth-token-here
     ```
   - Ensure your Turso URL uses `https://` (not `libsql://`)

4. **Create the database table:**
   ```sql
   CREATE TABLE nodes (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     text TEXT NOT NULL,
     x REAL NOT NULL,
     y REAL NOT NULL,
     z REAL NOT NULL,
     created_at DATETIME DEFAULT CURRENT_TIMESTAMP
   );
   ```

5. **Build and run:**
   ```bash
   .\build.bat  # This will automatically load credentials from .env.local
   ```

6. **Serve locally:**
   ```bash
   python -m http.server 8000
   ```

7. **Open in browser:**
   Navigate to `http://localhost:8000`

## Features

- **Static Projection:** Text is hashed and projected into 3D space using pseudo-random vectors
- **Turso Integration:** All nodes are stored and retrieved from Turso database
- **Real-time Animation:** Gentle floating animation using sine waves
- **WASM-Powered:** Runs entirely in the browser
- **Simple UI:** HTML form for adding new nodes

## Architecture

- `turso.rs`: HTTP client for Turso database API
- `math.rs`: Static projection algorithm (hash → pseudo-random → dot product → Vec3)
- `main.rs`: Bevy app setup with 3D scene and WASM bindings
- `index.html`: Simple HTML interface for user input

## Dependencies

- Bevy 0.14 (WASM-compatible)
- reqwest (WASM-compatible HTTP client)
- Turso HTTP API for database operations