// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 中西文自动空格，对应 Go 版 render/space.go。

use crate::char_class::{is_cjk, is_full_width, is_print, is_punct};
use crate::editor;

/// 把 tokens 中的中西文之间加上空格，对应 Go 版 BaseRenderer.Space。
pub fn space(tokens: &[u8]) -> Vec<u8> {
    space0(&String::from_utf8_lossy(tokens)).into_bytes()
}

/// 对应 Go 版 render.Space0。
pub fn space0(text: &str) -> String {
    let runes: Vec<char> = text.chars().collect();
    let length = runes.len();
    let mut ret = String::with_capacity(text.len() + 16);
    let mut i = 0;
    while i < length {
        let r = runes[i];
        if i < length.saturating_sub(3)
            && 'i' == runes[i + 1]
            && 'n' == runes[i + 2]
            && 'g' == runes[i + 3]
            && crate::char_class::is_han(r)
        {
            // ing 前不需要空格，如 打码ing https://github.com/88250/lute/issues/9
            ret.push(r);
            ret.push_str("ing");
            i += 4;
            continue;
        }
        ret = add_space_at_boundary(ret, r);
        i += 1;
    }
    ret
}

fn add_space_at_boundary(prefix: String, next_char: char) -> String {
    if prefix.is_empty() {
        return next_char.to_string();
    }

    // Emoji 1-9，对应 Go 版字符串比较语义
    if prefix.as_str() >= "1" && "9" >= prefix.as_str() && 0xFE0F == next_char as u32 {
        return format!("{prefix}{next_char}");
    }

    let current_char = prefix.chars().next_back().unwrap_or('\0');
    if allow_space(current_char, next_char) {
        return format!("{prefix} {next_char}");
    }
    format!("{prefix}{next_char}")
}

/// 判断两个字符之间是否允许插入空格，对应 Go 版 render.allowSpace。
pub fn allow_space(current_char: char, next_char: char) -> bool {
    let caret = editor::CARET.chars().next().unwrap_or('\u{2038}');
    if current_char.is_whitespace()
        || next_char.is_whitespace()
        || caret == current_char
        || caret == next_char
        || !is_print(current_char)
        || !is_print(next_char)
    {
        return false;
    }

    let current_is_cjk = is_cjk(current_char);
    let next_is_punct = ('%' != next_char
        && '@' != next_char
        && (is_punct(next_char) || '~' == next_char || '=' == next_char || '#' == next_char))
        || is_full_width(next_char);
    if current_is_cjk && next_is_punct {
        return false;
    }

    let current_is_punct = ('%' != current_char
        && '@' != current_char
        && (is_punct(current_char)
            || '~' == current_char
            || '=' == current_char
            || '#' == current_char))
        || is_full_width(current_char);
    let next_is_cjk = is_cjk(next_char);
    if next_is_cjk && current_is_punct {
        return false;
    }

    if (!current_is_cjk && !next_is_cjk) || (current_is_cjk && next_is_cjk) {
        return false;
    }
    true
}
