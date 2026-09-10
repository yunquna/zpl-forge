#!/usr/bin/env bash
set -euo pipefail
wasm_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$wasm_root"
wasm_rust_bin="$(dirname "$(rustup which rustc)")"
export PATH="$wasm_rust_bin:$PATH"
export RUSTC="$wasm_rust_bin/rustc"
if [[ "$(uname -s)" == Darwin ]]; then
  export DYLD_LIBRARY_PATH="$wasm_rust_bin/../lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
fi
wasm_bindgen_bin="${WASM_BINDGEN:-wasm-bindgen}"
if [[ "$("$wasm_bindgen_bin" --version)" != 'wasm-bindgen 0.2.128' ]]; then
  echo 'Install wasm-bindgen-cli 0.2.128 or set WASM_BINDGEN to that executable.' >&2
  exit 1
fi
wasm_target_dir="${CARGO_TARGET_DIR:-$wasm_root/wasm/target}"
cargo build --manifest-path wasm/Cargo.toml --locked --release \
  --target wasm32-unknown-unknown --target-dir "$wasm_target_dir" -j 2 "$@"
"$wasm_bindgen_bin" --target bundler --out-name zpl_forge --out-dir wasm/pkg \
  "$wasm_target_dir/wasm32-unknown-unknown/release/yqn_zpl_forge_wasm.wasm"
cp wasm/package.json wasm/README.md LICENSE-MIT LICENSE-APACHE wasm/pkg/
mkdir -p wasm/pkg/licenses
cp src/assets/*LICENSE.txt src/assets/OFL.txt wasm/pkg/licenses/
{
  git rev-parse HEAD
  if [[ -n "$(git status --porcelain)" ]]; then echo 'working-tree: dirty'; else echo 'working-tree: clean'; fi
  rustc --version
  "$wasm_bindgen_bin" --version
} > wasm/pkg/provenance.txt
