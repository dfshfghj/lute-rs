# 对齐情况

本目录是 Rust 实现与上游 Go 实现的行为对齐情况。（只放了数据）

为什么不带生成器：

- 必须连同整个 Go 实现一起才能跑；
- 依赖具体版本，版本不同会生成不同的表；

报告里的 `rust_core_sources_sha256` 是产出报告时的源码指纹，用来判断报告有没有过期：

```bash
python3 rust/tools/check_reports.py
#   current  go-original-rust.json  3bcff9e4ca50
#   STALE    go-original-rust-javascript.json  c79e5ed7e606
```

（STALE 表示报告描述的不是当前代码，重跑一遍即可）

    cases/      手写用例
    golden/     上游输出快照
    fixtures/   上游规范与数据（CommonMark、emoji 表、kramdown）
    reports/    对齐报告（native / javascript）

| 文件 | 使用方 |
| --- | --- |
| `golden/options.json` | `options.rs` |
| `golden/ast_commonmark.json` | `tests/commonmark_block.rs` |
| `golden/ast_inline_commonmark.json` / `_js.json` | `tests/commonmark_inline.rs` |
| `golden/html_commonmark_js.json` | `tests/go_commonmark_html.rs` |
| `golden/corpus_markdown.json` + `ast_inline_corpus.json` | `tests/corpus_inline.rs` |
| `fixtures/commonmark-spec.json` | `tests/commonmark_block.rs` / `commonmark_inline.rs` |
| `fixtures/commonmark-spec-kramdown.md` | `tests/go_api_structural.rs` |
| `fixtures/emoji_unicode17.json` | `tests/go_api_structural.rs` |
| `fixtures/case1.md` | `tests/go_vditor_structural.rs` |

## 报告

| 文件 | 内容 |
| --- | --- |
| `reports/go-original-rust.json` | native profile：原表 2075/2075，测试函数 367 passed / 0 failed |
| `reports/go-original-rust-javascript.json` | javascript profile，同上 |
| `reports/go-test-coverage.json` | 285 个原 Go Test 函数的迁移清单 |
| `reports/go-table-assertion-review.json` | 每个原函数的断言覆盖审阅 |
