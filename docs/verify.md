# 验证

四类检查，都不需要 Go 工具链。

## 1. 引擎测试（两个构建 profile）

    cargo test --manifest-path rust/Cargo.toml --workspace
    cargo test --manifest-path rust/Cargo.toml --workspace --features lute-core/javascript

预期都是 **367 passed / 0 failed**。两个 profile 的差别见
[known-divergences.md](./known-divergences.md)——native 与 javascript 构建在代码高亮、
大小写折叠上目标不同，各自对齐各自的期望。

## 2. 对齐证据是否新鲜

    python3 rust/tools/check_reports.py

预期 `全部一致`。报告里记录了产出时的源码指纹（`rust_core_sources_sha256`），
任何源码改动都会让它变 STALE——此时按脚本打印的命令重新生成：

    LUTE_GO_TEST_REPORT=/tmp/go-original.json cargo test \
      --manifest-path rust/Cargo.toml -p lute-core --test go_original_tables
    cargo test --manifest-path rust/Cargo.toml --workspace --no-fail-fast > /tmp/ws.log
    python3 rust/tools/summarize_go_tests.py /tmp/go-original.json \
      --workspace-log /tmp/ws.log --out conformance/reports/go-original-rust.json

javascript profile 加 `--features javascript`，输出到
`conformance/reports/go-original-rust-javascript.json`。

## 3. JS 绑定与 demo

    node javascript/conformance/renderers.mjs    # PASS 30 renderer cases
    node demo/test-api.mjs                       # 10/10 通过

`renderers.mjs` 跑自定义渲染器与全局函数；`test-api.mjs` 复刻了 Vditor 4.0.1 里
`options → Lute` 的适配函数（`dist/index.min.js` 模块 214），setter 少一个、签名不对都会先炸。

## 4. 代码规范

    cd rust
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

## 当前结果

| 检查 | 结果 |
| --- | --- |
| `cargo test --workspace`（native） | 367 passed / 0 failed |
| `cargo test --workspace --features lute-core/javascript` | 367 passed / 0 failed |
| 原表（两个 profile） | 2075 / 2075 |
| `renderers.mjs` | 30 / 30 |
| `demo/test-api.mjs` | 10 / 10 |
| `check_reports.py` | 全部一致 |
| `cargo fmt` / `clippy -D warnings` | 通过 / 0 error |
