#!/bin/bash
echo "Installing wasm-pack..."
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

echo "Building Wasm..."
wasm-pack build --target web --out-dir web/public/wasm --no-typescript
