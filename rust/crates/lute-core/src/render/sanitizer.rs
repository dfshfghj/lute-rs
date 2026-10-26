// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML 安全过滤，对应 Go 版 render/sanitizer.go。
//!
//! 这里实现了一个聚焦于安全过滤的 HTML 词法分析器：识别开始/结束/自闭合标签、
//! 属性、注释、声明和文本，并跳过危险元素的内容。它不是完整的 HTML5 解析器，
//! 但覆盖了 sanitizer 需要的 token 语义。

use crate::editor;
use crate::html;

/// 需要连同内容一起跳过的元素。
fn is_skip_element(name: &str) -> bool {
    matches!(
        name,
        "frame"
            | "frameset"
            | "noembed"
            | "noframes"
            | "noscript"
            | "nostyle"
            | "object"
            | "script"
            | "style"
            | "title"
    )
}

/// 承载 URL 的属性，需要做危险协议过滤。
fn is_url_attr(name: &str) -> bool {
    matches!(name, "src" | "srcset" | "href" | "action" | "xlink:href")
}

/// 对应 Go 版 render.Sanitize。
pub fn sanitize_str(s: &str) -> String {
    String::from_utf8_lossy(&sanitize(s.as_bytes())).to_string()
}

/// 对应 Go 版 sanitize。
pub fn sanitize(tokens: &[u8]) -> Vec<u8> {
    let caret = editor::CARET.as_bytes();
    let caret_left_space = find(tokens, format!(" {}", editor::CARET).as_bytes()).is_some();
    let tokens = replace(tokens, caret, editor::CARET_REPLACEMENT.as_bytes());

    let mut out: Vec<u8> = Vec::with_capacity(tokens.len());
    let mut skip_content = false;
    let mut skipping_count: i64 = 0;
    let mut most_recently_started = String::new();
    let mut i = 0usize;

    while i < tokens.len() {
        if b'<' != tokens[i] {
            let start = i;
            while i < tokens.len() && b'<' != tokens[i] {
                i += 1;
            }
            if !skip_content {
                let text = html::unescape_string(&String::from_utf8_lossy(&tokens[start..i]));
                out.extend_from_slice(escape_string(&text).as_bytes());
            }
            continue;
        }

        // 注释
        if tokens[i..].starts_with(b"<!--") {
            match find(&tokens[i..], b"-->") {
                Some(end) => i += end + 3,
                None => i = tokens.len(),
            }
            continue;
        }

        // 声明（DOCTYPE 等）
        if tokens[i..].starts_with(b"<!") {
            while i < tokens.len() && b'>' != tokens[i] {
                i += 1;
            }
            if i < tokens.len() {
                i += 1;
            }
            continue;
        }

        // 结束标签
        if tokens[i..].starts_with(b"</") {
            let mut j = i + 2;
            let name_start = j;
            while j < tokens.len() && is_tag_name_byte(tokens[j]) {
                j += 1;
            }
            let name = String::from_utf8_lossy(&tokens[name_start..j]).to_lowercase();
            while j < tokens.len() && b'>' != tokens[j] {
                j += 1;
            }
            if j < tokens.len() {
                j += 1;
            }
            i = j;

            if most_recently_started == name {
                most_recently_started.clear();
            }
            if is_skip_element(&name) {
                skipping_count -= 1;
                if 0 == skipping_count {
                    skip_content = false;
                }
                out.push(b' ');
                continue;
            }
            if !skip_content {
                out.extend_from_slice(b"</");
                out.extend_from_slice(name.as_bytes());
                out.push(b'>');
            }
            continue;
        }

        // 开始标签
        if i + 1 < tokens.len() && tokens[i + 1].is_ascii_alphabetic() {
            let mut j = i + 1;
            let name_start = j;
            while j < tokens.len() && is_tag_name_byte(tokens[j]) {
                j += 1;
            }
            let name = String::from_utf8_lossy(&tokens[name_start..j]).to_lowercase();

            let mut attrs: Vec<(String, String)> = Vec::new();
            let mut self_closing = false;
            loop {
                while j < tokens.len() && tokens[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= tokens.len() {
                    break;
                }
                if b'>' == tokens[j] {
                    j += 1;
                    break;
                }
                if b'/' == tokens[j] {
                    j += 1;
                    if j < tokens.len() && b'>' == tokens[j] {
                        j += 1;
                        self_closing = true;
                        break;
                    }
                    continue;
                }

                // 属性名
                let key_start = j;
                while j < tokens.len()
                    && !tokens[j].is_ascii_whitespace()
                    && b'=' != tokens[j]
                    && b'>' != tokens[j]
                    && b'/' != tokens[j]
                {
                    j += 1;
                }
                let key = String::from_utf8_lossy(&tokens[key_start..j]).to_lowercase();
                while j < tokens.len() && tokens[j].is_ascii_whitespace() {
                    j += 1;
                }
                let mut val = String::new();
                if j < tokens.len() && b'=' == tokens[j] {
                    j += 1;
                    while j < tokens.len() && tokens[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < tokens.len() && (b'"' == tokens[j] || b'\'' == tokens[j]) {
                        let quote = tokens[j];
                        j += 1;
                        let val_start = j;
                        while j < tokens.len() && quote != tokens[j] {
                            j += 1;
                        }
                        val =
                            html::unescape_string(&String::from_utf8_lossy(&tokens[val_start..j]));
                        if j < tokens.len() {
                            j += 1;
                        }
                    } else {
                        let val_start = j;
                        while j < tokens.len()
                            && !tokens[j].is_ascii_whitespace()
                            && b'>' != tokens[j]
                        {
                            j += 1;
                        }
                        val =
                            html::unescape_string(&String::from_utf8_lossy(&tokens[val_start..j]));
                    }
                }
                if !key.is_empty() && !attrs.iter().any(|(k, _)| *k == key) {
                    attrs.push((key, val));
                }
            }
            i = j;
            most_recently_started = name.clone();

            if is_skip_element(&name) {
                skip_content = true;
                skipping_count += 1;
                out.push(b' ');
                // 原始文本元素：直接跳到对应的结束标签
                if is_raw_text_element(&name) && !self_closing {
                    if let Some(pos) = find_end_tag(&tokens[i..], &name) {
                        i += pos;
                    }
                }
                continue;
            }

            let attrs = sanitize_attrs(&attrs);
            if !skip_content {
                out.push(b'<');
                out.extend_from_slice(name.as_bytes());
                for (k, v) in &attrs {
                    out.push(b' ');
                    out.extend_from_slice(k.as_bytes());
                    out.extend_from_slice(b"=\"");
                    out.extend_from_slice(escape_string(v).as_bytes());
                    out.push(b'"');
                }
                if self_closing {
                    // Go writes linkable tokens through writeLinkableBuf, while
                    // html.Token.String emits other self-closing tags as />.
                    if matches!(
                        name.as_str(),
                        "a" | "area" | "blockquote" | "img" | "link" | "script"
                    ) {
                        out.push(b' ');
                    }
                    out.push(b'/');
                }
                out.push(b'>');
            }
            continue;
        }

        // 字面量 '<'
        out.push(b'<');
        i += 1;
    }

    let mut ret = out;
    if caret_left_space {
        ret = replace(
            &ret,
            format!("\"{}", editor::CARET_REPLACEMENT).as_bytes(),
            format!("\" {}", editor::CARET_REPLACEMENT).as_bytes(),
        );
    } else {
        ret = replace(
            &ret,
            format!("\" {}", editor::CARET_REPLACEMENT).as_bytes(),
            format!("\"{}", editor::CARET_REPLACEMENT).as_bytes(),
        );
    }
    replace(&ret, editor::CARET_REPLACEMENT.as_bytes(), caret)
}

/// 对应 Go 版 sanitizeAttrs。
fn sanitize_attrs(attrs: &[(String, String)]) -> Vec<(String, String)> {
    let mut ret = Vec::with_capacity(attrs.len());
    for (k, v) in attrs {
        if !allow_attr(k) {
            continue;
        }
        if "srcdoc" == k {
            continue;
        }
        if is_url_attr(k) {
            let val = remove_space(v.trim().to_lowercase().as_str());
            if val.starts_with("data:image/svg+xml")
                || val.starts_with("data:text/html")
                || val.starts_with("javascript")
            {
                continue;
            }
            let new_val = html::unescape_attr_val(&sanitize_str(&val));
            if val != new_val {
                continue;
            }
        }
        ret.push((k.clone(), v.clone()));
    }
    ret
}

fn allow_attr(name: &str) -> bool {
    if name.to_lowercase().starts_with("on") {
        return false;
    }
    !matches!(name, "http-equiv" | "formaction")
}

/// 对应 Go 版 removeSpace：去掉除空格以外的所有空白。
fn remove_space(s: &str) -> String {
    s.chars()
        .filter(|c| *c == ' ' || !c.is_whitespace())
        .collect()
}

fn is_raw_text_element(name: &str) -> bool {
    matches!(name, "script" | "style" | "title" | "textarea")
}

fn is_tag_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b'-' == b || b':' == b
}

/// 查找结束标签的位置（大小写不敏感），返回相对下标。
fn find_end_tag(tokens: &[u8], name: &str) -> Option<usize> {
    let mut i = 0;
    while i + 1 < tokens.len() {
        if b'<' == tokens[i] && b'/' == tokens[i + 1] {
            let start = i + 2;
            let mut j = start;
            while j < tokens.len() && is_tag_name_byte(tokens[j]) {
                j += 1;
            }
            if String::from_utf8_lossy(&tokens[start..j]).eq_ignore_ascii_case(name) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 对应 Go 版 html.EscapeString（x/net/html 语义）。
fn escape_string(s: &str) -> String {
    let mut ret = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ret.push_str("&amp;"),
            '\'' => ret.push_str("&#39;"),
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            '"' => ret.push_str("&#34;"),
            '\r' => ret.push_str("&#13;"),
            _ => ret.push(c),
        }
    }
    ret
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn replace(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i..].starts_with(old) {
            ret.extend_from_slice(new);
            i += old.len();
        } else {
            ret.push(tokens[i]);
            i += 1;
        }
    }
    ret
}

#[cfg(test)]
mod go_function_boundaries {
    use super::*;

    /// render/sanitizer.go: removeSpace preserves ASCII space and removes Unicode whitespace.
    #[test]
    fn remove_space_preserves_only_literal_spaces() {
        assert_eq!(
            "javascript:",
            remove_space(" java\t\n\r\u{85}\u{a0}script: ").trim()
        );
        assert_eq!("a b", remove_space("a \u{2003}\u{3000}b"));
    }

    /// render/sanitizer.go: sanitizeAttrs filters all URL attributes using identical rules.
    #[test]
    fn dangerous_protocols_and_event_attributes_are_removed() {
        for key in ["src", "srcset", "href", "action", "xlink:href"] {
            for value in [
                "javascript:alert(1)",
                " JaVa\nScRiPt:alert(1) ",
                "data:text/html,payload",
                "DATA:IMAGE/SVG+XML,payload",
            ] {
                assert!(
                    sanitize_attrs(&[(key.into(), value.into())]).is_empty(),
                    "{key}={value}"
                );
            }
            let safe = [(key.into(), "https://example.com/a?b=c".into())];
            assert_eq!(safe.to_vec(), sanitize_attrs(&safe));
        }
        for key in [
            "onerror",
            "OnLoad",
            "ONCLICK",
            "http-equiv",
            "formaction",
            "srcdoc",
        ] {
            assert!(
                sanitize_attrs(&[(key.into(), "value".into())]).is_empty(),
                "{key}"
            );
        }
        assert!(allow_attr("data-onclick"));
    }
}
