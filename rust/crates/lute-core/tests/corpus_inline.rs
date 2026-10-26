// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! conformance 用例中默认选项下的 markdown 语料，完整语法树与 Go 版逐字节比对。
//!
//! 语料取自 conformance/cases/*.json 中不带 setup 的 MarkdownStr / FormatStr / Md2HTML
//! 用例，覆盖 GFM 自动链接、Emoji、脚注、表格等 CommonMark 之外的特性。

use lute_core::options::ParseOptions;
use lute_core::parse::parse;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn corpus_inline_ast_matches_go() {
    let root = repo_root();
    let markdowns: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(root.join("conformance/golden/corpus_markdown.json"))
            .expect("读取语料"),
    )
    .expect("解析语料");
    let golden: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(root.join("conformance/golden/ast_inline_corpus.json"))
            .expect("读取 Go 快照"),
    )
    .expect("解析 Go 快照");

    assert_eq!(markdowns.len(), golden.len(), "用例数量不一致");

    let mut failures = Vec::new();
    for (idx, (markdown, want)) in markdowns.iter().zip(&golden).enumerate() {
        let got = parse("", markdown.as_bytes(), ParseOptions::new())
            .root()
            .dump();
        if &got != want {
            failures.push((idx, markdown.clone(), want.clone(), got));
        }
    }

    if !failures.is_empty() {
        for (idx, markdown, want, got) in failures.iter().take(3) {
            eprintln!("语料 {idx}: {markdown:?}");
            eprintln!("--- Go ---\n{want}");
            eprintln!("--- Rust ---\n{got}");
        }
        panic!("{} / {} 个语料的语法树不一致", failures.len(), golden.len());
    }
}
