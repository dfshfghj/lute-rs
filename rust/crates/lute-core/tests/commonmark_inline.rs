// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! CommonMark 652 个示例的完整（块级 + 行级）语法树与 Go 版逐字节比对。

use lute_core::options::ParseOptions;
use lute_core::parse::parse;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn commonmark_inline_ast_matches_go() {
    let root = repo_root();
    let spec: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(root.join("conformance/fixtures/commonmark-spec.json"))
            .expect("读取规范用例"),
    )
    .expect("解析规范用例");
    let golden: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(root.join(
            if cfg!(any(target_arch = "wasm32", feature = "javascript")) {
                "conformance/golden/ast_inline_commonmark_js.json"
            } else {
                "conformance/golden/ast_inline_commonmark.json"
            },
        ))
        .expect("读取 Go 快照"),
    )
    .expect("解析 Go 快照");

    assert_eq!(spec.len(), golden.len(), "用例数量不一致");

    let mut failures = Vec::new();
    for (case, want) in spec.iter().zip(&golden) {
        let markdown = case["markdown"].as_str().unwrap_or_default();
        let got = parse("", markdown.as_bytes(), ParseOptions::new())
            .root()
            .dump();
        if &got != want {
            failures.push((
                case["example"].as_i64().unwrap_or(0),
                markdown.to_string(),
                want.clone(),
                got,
            ));
        }
    }

    if !failures.is_empty() {
        for (example, markdown, want, got) in failures.iter().take(3) {
            eprintln!("示例 {example}: {markdown:?}");
            eprintln!("--- Go ---\n{want}");
            eprintln!("--- Rust ---\n{got}");
        }
        panic!(
            "{} / {} 个用例的完整语法树不一致",
            failures.len(),
            golden.len()
        );
    }
}
