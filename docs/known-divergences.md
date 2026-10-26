# 已知差异

> 基准：上游 Go 实现（输出冻结在 `conformance/golden/`）
> 现状：Rust + WebAssembly（`javascript/lute.min.js` / `javascript/pkg`）

## 不支持

**代码块语法高亮（Chroma）**：Go 版有，Rust 版没有接入。`SetCodeSyntaxHighlight`、
`SetCodeSyntaxHighlightLineNum`、`SetCodeSyntaxHighlightInlineStyle`、
`SetCodeSyntaxHighlightStyleName` 这些选项会被接受，但不产生高亮输出。

这是目前唯一明确的缺口。

## 已验证一致

| 项目 | 结果 |
| --- | --- |
| 原表（Go 测试导出的输入/输出对） | 2075 / 2075 |
| native profile | 2075 / 2075 |
| javascript profile | 2075 / 2075 |
| 工作区测试函数 | 367 passed / 0 failed |
| 自定义渲染器（`renderers.mjs`） | 30 / 30 |

报告见 `conformance/reports/`，快照见 `conformance/golden/`。

## 编译期裁剪的影响

关掉 `emoji` / `autolink` 后不再与 Go 默认行为一致（内置表缺失），
这是裁剪的预期结果，不属于「差异」。见 `docs/feature-trim.md`。
