// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 术语拼写修正，对应 Go 版 render/term_typographer.go。

use std::collections::HashMap;

use crate::lex;

/// 修正 tokens 中出现的术语拼写问题，对应 Go 版 BaseRenderer.FixTermTypo。
pub fn fix_term_typo(tokens: &mut [u8], terms: &HashMap<String, String>) {
    fix_term_typo0(tokens, terms)
}

fn fix_term_typo0(tokens: &mut [u8], terms: &HashMap<String, String>) {
    let length = tokens.len();
    let mut before: u8 = 0;
    let mut i = 0;
    while i < length {
        let token = tokens[i];
        if is_not_term(token) {
            i += 1;
            continue;
        }
        if 1 <= i {
            before = tokens[i - 1];
            if !is_not_term(before) {
                // 前一个字节必须是非术语，否则无法分隔
                i += 1;
                continue;
            }
        }
        if lex::is_ascii_punct(before) {
            // 比如术语前面如果是 . 则不进行修正，因为可能是链接
            // 比如 test.html 虽然不能识别为自动链接，但是也不能进行修正
            i += 1;
            continue;
        }

        let mut j = i;
        let mut after: u8 = 0;
        while j < length {
            after = tokens[j];
            if is_not_term(after) || lex::ITEM_DOT == after {
                break;
            }
            j += 1;
        }
        if lex::is_ascii_punct(after) {
            // 比如术语后面如果是 . 则不进行修正，因为可能是链接
            // 比如 github.com 虽然不能识别为自动链接，但是也不能进行修正
            i += 1;
            continue;
        }

        let original_term = tokens[i..j].to_ascii_lowercase();
        let original_term = String::from_utf8_lossy(&original_term).to_string();
        if let Some(to) = terms.get(&original_term) {
            let to = to.as_bytes();
            for (token, replacement) in tokens[i..j].iter_mut().zip(to) {
                *token = *replacement;
            }
        }
        i += 1;
    }
}

fn is_not_term(token: u8) -> bool {
    token >= 0x80 || lex::is_whitespace(token) || lex::is_ascii_punct(token)
}
