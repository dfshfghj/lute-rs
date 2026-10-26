// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 语法树文本导出，用于与 Go 版做差分测试。
//!
//! 默认只做块级解析；`--inline` 时执行完整的块级 + 行级解析。
//!
//! echo '["# foo\n"]' | cargo run --example astdump -- --inline

use std::io::Read;

use lute_core::options::ParseOptions;
use lute_core::parse::{parse, parse_block};

fn main() {
    let inline = std::env::args().any(|a| a == "--inline");
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let markdowns: Vec<String> = serde_json::from_str(&input).unwrap();

    let results: Vec<String> = markdowns
        .iter()
        .map(|markdown| {
            let tree = if inline {
                parse("", markdown.as_bytes(), ParseOptions::new())
            } else {
                parse_block("", markdown.as_bytes(), ParseOptions::new())
            };
            tree.root().dump()
        })
        .collect();

    println!("{}", serde_json::to_string(&results).unwrap());
}
