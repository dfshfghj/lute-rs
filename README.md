# lute-rs

Lute 的 Rust 实现

```
rust/crates/lute-core     Markdown 引擎
rust/crates/lute-wasm     wasm-bindgen 绑定层，导出 Vditor 需要的同名 API
rust/tools/               报告生成
javascript/lute.js        ESM 兼容层
javascript/lute.min.js    兼容 Vditor 调用的产物
demo/                     Vditor demo
conformance/              对齐文档
```

## demo

```bash
# 构建 WASM（需要 rustup target add wasm32-unknown-unknown 与 wasm-pack）
npm --prefix demo install
npm --prefix demo run build:lute

npm --prefix demo start
```
