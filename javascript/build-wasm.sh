#!/usr/bin/env bash
# Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
# Copyright (c) 2019-present, b3log.org
#
# 构建 Rust + WebAssembly 产物：
#   javascript/pkg/          ESM（wasm-pack --target web），供 lute.js / 测试使用
#   javascript/lute.min.js   传统 script / CommonJS 单文件（内嵌 wasm），供 lute.html、demo.js 使用

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="${here}/.."
tmp="${root}/rust/target/lute-wasm-nomod"

cd "${root}/rust/crates/lute-wasm"

wasm-pack build --target web --out-dir "${here}/pkg" --out-name lute_wasm
wasm-pack build --target no-modules --out-dir "${tmp}" --out-name lute_wasm

python3 "${here}/build-wasm-bundle.py" "${tmp}" "${here}/lute.min.js"
