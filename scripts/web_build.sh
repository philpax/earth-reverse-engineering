#!/bin/sh
set -e
cargo build --release -p rocktree-client --target wasm32-unknown-unknown --no-default-features --features webgpu
wasm-bindgen \
    --no-typescript \
    --target web  \
    --out-dir ./build/ \
    --out-name "rocktree_client" \
    ./target/wasm32-unknown-unknown/release/rocktree-client.wasm

# Optimize WASM if wasm-opt is available.
if command -v wasm-opt > /dev/null 2>&1; then
    wasm-opt -Oz -o build/rocktree_client_bg.wasm build/rocktree_client_bg.wasm
fi

cat <<EOF > build/index.html
<!DOCTYPE html>
<html lang="en">
  <head>
    <title>rocktree-client</title>
  </head>
  <body style="margin: 0px; width: 100vw; height: 100vh;">
    <script type="module">
      import init from "./rocktree_client.js";

      init().catch((error) => {
        if (
          !error.message.startsWith(
            "Using exceptions for control flow, don't mind me. This isn't actually an error!"
          )
        ) {
          throw error;
        }
      });
    </script>
  </body>
</html>
EOF
