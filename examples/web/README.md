# SafeGL WebGL example

Draws a rotating triangle with SafeGL's `webgl` level, the OpenGL ES 3.0 API of WebGL 2, on a
browser canvas.

It is a separate crate because the main crate's examples depend on glutin, which doesn't build
for `wasm32-unknown-unknown`.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/safegl_web_example.wasm
python -m http.server 8000
```

Then open <http://localhost:8000>.
