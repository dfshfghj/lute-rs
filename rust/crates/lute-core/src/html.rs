// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML 转义与实体解码。
//!
//! 目前只覆盖 Lute 自身会产生和消费的实体集合；完整的实体表随 M4 的 html 模块一起移植。

use crate::editor::IAL_VAL_ESC_NEWLINE;

/// 对应 Go 版 html.EscapeHTML。
pub fn escape_html(s: &str) -> String {
    let mut ret = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => ret.push_str("&amp;"),
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            '"' => ret.push_str("&quot;"),
            _ => ret.push(c),
        }
    }
    ret
}

/// 对应 Go 版 html.EscapeHTMLStr。
pub fn escape_html_str(s: &str) -> String {
    escape_html(s)
}

/// 对应 Go 版 html.EscapeString（标准库语义）。
///
/// 与 `escape_html` 的区别：双引号转义为 `&#34;`，单引号转义为 `&#39;`。
pub fn escape_string(s: &str) -> String {
    let mut ret = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => ret.push_str("&amp;"),
            '\'' => ret.push_str("&#39;"),
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            '"' => ret.push_str("&#34;"),
            _ => ret.push(c),
        }
    }
    ret
}

/// 对应 Go 版 html.EscapeAttrVal。
pub fn escape_attr_val(v: &str) -> String {
    let mut ret = escape_html(v);
    ret = ret.replace('\n', IAL_VAL_ESC_NEWLINE);
    ret = ret.replace('{', "&#123;");
    ret = ret.replace('}', "&#125;");
    ret
}

/// 对应 Go 版 html.UnescapeAttrVal。
pub fn unescape_attr_val(v: &str) -> String {
    let mut ret = v.replace(IAL_VAL_ESC_NEWLINE, "\n");
    ret = ret.replace("&#123;", "{");
    ret = ret.replace("&#125;", "}");
    unescape_string(&ret)
}

/// 对应 Go 版 html.UnescapeHTMLStr。
pub fn unescape_html_str(h: &str) -> String {
    unescape_string(h)
}

/// 对应 Go 版 html.EncodeDestination。
pub fn encode_destination(rawurl: &[u8]) -> Vec<u8> {
    const HEX: &[u8] = b"0123456789ABCDEF";
    const SAFE: &[u8] =
        b"!#$&'()*+,-./0123456789:;=?@ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz~";

    let mut ret = Vec::with_capacity(256);
    let mut i = 0;
    while i < rawurl.len() {
        let ch = match std::str::from_utf8(&rawurl[i..]) {
            Ok(s) => s.chars().next().unwrap(),
            Err(e) => {
                let valid = e.valid_up_to();
                if valid == 0 {
                    // utf8.DecodeRune in Go consumes the original invalid byte;
                    // EncodeDestination percent-encodes that byte unchanged.
                    let byte = rawurl[i];
                    ret.extend_from_slice(&[
                        b'%',
                        HEX[(byte >> 4) as usize],
                        HEX[(byte & 15) as usize],
                    ]);
                    i += 1;
                    continue;
                }
                std::str::from_utf8(&rawurl[i..i + valid])
                    .unwrap()
                    .chars()
                    .next()
                    .unwrap()
            }
        };
        let rlen = ch.len_utf8();
        let r = ch as u32;
        if 0x80 <= r {
            for &b in &rawurl[i..i + rlen] {
                ret.push(b'%');
                ret.push(HEX[((b >> 4) & 0xf) as usize]);
                ret.push(HEX[(b & 0xf) as usize]);
            }
        } else if r == '%' as u32 {
            if i + 2 < rawurl.len()
                && crate::lex::is_hex_digit(rawurl[i + 1])
                && crate::lex::is_hex_digit(rawurl[i + 2])
            {
                ret.push(b'%');
                ret.push(crate::lex::token_to_upper(rawurl[i + 1]));
                ret.push(crate::lex::token_to_upper(rawurl[i + 2]));
                i += 2;
            } else {
                ret.extend_from_slice(b"%25");
            }
        } else if !SAFE.contains(&(r as u8)) {
            ret.push(b'%');
            ret.push(HEX[((r >> 4) & 0xf) as usize]);
            ret.push(HEX[(r & 0xf) as usize]);
        } else {
            ret.push(r as u8);
        }
        i += rlen;
    }
    ret
}

/// 对应 Go 版 html.DecodeDestination。
pub fn decode_destination(rawurl: &[u8]) -> Vec<u8> {
    const REPLACEMENT: &[u8] = b"\xEF\xBF\xBD";

    let mut ret: Vec<u8> = Vec::with_capacity(rawurl.len());
    let mut i = 0;
    'outer: while i < rawurl.len() {
        let ch = match std::str::from_utf8(&rawurl[i..]) {
            Ok(s) => s.chars().next().unwrap(),
            Err(e) => {
                let valid = e.valid_up_to();
                if valid == 0 {
                    ret.extend_from_slice(REPLACEMENT);
                    i += 1;
                    continue;
                }
                std::str::from_utf8(&rawurl[i..i + valid])
                    .unwrap()
                    .chars()
                    .next()
                    .unwrap()
            }
        };
        let rlen = ch.len_utf8();
        if ch == '%'
            && i + 2 < rawurl.len()
            && crate::lex::is_hex_digit(rawurl[i + 1])
            && crate::lex::is_hex_digit(rawurl[i + 2])
        {
            let b = unhex(rawurl[i + 1]) << 4 | unhex(rawurl[i + 2]);
            if b < 0x80 {
                ret.push(b);
                i += 3;
                continue;
            }
            let n = match b {
                b if b & 0xe0 == 0xc0 => 1,
                b if b & 0xf0 == 0xe0 => 2,
                b if b & 0xf8 == 0xf0 => 3,
                _ => 0,
            };
            if n == 0 {
                ret.extend_from_slice(REPLACEMENT);
                i += 3;
                continue;
            }
            let mut rb = [0u8; 4];
            rb[0] = b;
            let mut j = i + 3;
            for k in 0..n {
                let (byte, next) = advance(rawurl, j);
                if next > rawurl.len() || byte & 0xc0 != 0x80 {
                    ret.extend_from_slice(REPLACEMENT);
                    i += 3;
                    continue 'outer;
                }
                rb[k + 1] = byte;
                j = next;
            }
            match std::str::from_utf8(&rb[..n + 1]) {
                Ok(s) => ret.extend_from_slice(s.as_bytes()),
                Err(_) => ret.extend_from_slice(REPLACEMENT),
            }
            i = j;
            continue;
        }
        ret.extend_from_slice(&rawurl[i..i + rlen]);
        i += rlen;
    }
    ret
}

fn unhex(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn advance(s: &[u8], pos: usize) -> (u8, usize) {
    if pos >= s.len() {
        return (0, s.len() + 1);
    }
    if s[pos] != b'%' {
        return (s[pos], pos + 1);
    }
    if pos + 2 < s.len()
        && crate::lex::is_hex_digit(s[pos + 1])
        && crate::lex::is_hex_digit(s[pos + 2])
    {
        return (unhex(s[pos + 1]) << 4 | unhex(s[pos + 2]), pos + 3);
    }
    (b'%', pos + 1)
}

/// 对应 Go 版 html.UnescapeBytes：先反转义 HTML 实体，再去掉标点前的反斜杠。
pub fn unescape_bytes(tokens: &[u8]) -> Vec<u8> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let unescaped = unescape_html_str(&String::from_utf8_lossy(tokens));
    let bytes = unescaped.as_bytes();
    let mut ret = Vec::with_capacity(bytes.len());
    for i in 0..bytes.len() {
        if crate::lex::is_backslash_escape_punct(bytes, i) {
            ret.pop();
        }
        ret.push(bytes[i]);
    }
    ret
}

/// 对应 Go 版 html.HtmlUnescapeString：只识别以分号结尾的实体。
pub fn unescape_string(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }

    let bytes = s.as_bytes();
    let mut ret: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if b'&' == bytes[i] {
            if let Some((decoded, consumed)) = parse_entity(&bytes[i..]) {
                ret.extend_from_slice(&decoded);
                i += consumed;
                continue;
            }
        }
        ret.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&ret).into_owned()
}

/// 对应 Go 版 html.parseEntity。返回 (解码结果, 消耗的字节数)。
fn parse_entity(s: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut st = 0;
    let mut n = 0usize;
    let mut i = 1;
    while i < s.len() {
        let b = s[i];
        match st {
            0 => {
                if b == b'#' {
                    st = 1;
                } else if b.is_ascii_alphabetic() {
                    n = 1;
                    st = 2;
                } else {
                    return None;
                }
            }
            1 => {
                if b == b'x' || b == b'X' {
                    st = 3;
                } else if b.is_ascii_digit() {
                    n = 1;
                    st = 4;
                } else {
                    return None;
                }
            }
            2 => {
                if b.is_ascii_alphanumeric() {
                    n += 1;
                    if n > 31 {
                        return None;
                    }
                } else if b == b';' {
                    let key = std::str::from_utf8(&s[i - n..=i]).ok()?;
                    return entities().get(key).map(|v| (v.as_bytes().to_vec(), i + 1));
                } else {
                    return None;
                }
            }
            3 => {
                if b.is_ascii_hexdigit() {
                    n = 1;
                    st = 5;
                } else {
                    return None;
                }
            }
            4 => {
                if b.is_ascii_digit() {
                    n += 1;
                    if n > 8 {
                        return None;
                    }
                } else if b == b';' {
                    let c = std::str::from_utf8(&s[i - n..i])
                        .ok()?
                        .parse::<i64>()
                        .ok()?;
                    return Some((entity_code(c), i + 1));
                } else {
                    return None;
                }
            }
            5 => {
                if b.is_ascii_hexdigit() {
                    n += 1;
                    if n > 8 {
                        return None;
                    }
                } else if b == b';' {
                    let c =
                        i64::from_str_radix(std::str::from_utf8(&s[i - n..i]).ok()?, 16).ok()?;
                    return Some((entity_code(c), i + 1));
                } else {
                    return None;
                }
            }
            _ => unreachable!(),
        }
        i += 1;
    }
    None
}

fn entity_code(c: i64) -> Vec<u8> {
    if !is_valid_entity_code(c) {
        return "\u{FFFD}".as_bytes().to_vec();
    }
    char::from_u32(c as u32)
        .map(|ch| ch.to_string().into_bytes())
        .unwrap_or_else(|| "\u{FFFD}".as_bytes().to_vec())
}

/// 对应 Go 版 html.isValidEntityCode。
fn is_valid_entity_code(c: i64) -> bool {
    if !(0..=0x10FFFF).contains(&c) || (0xD800..=0xDFFF).contains(&c) {
        return false;
    }
    if (0xFDD0..=0xFDEF).contains(&c) || c & 0xffff == 0xffff || c & 0xffff == 0xfffe {
        return false;
    }
    !(c <= 0x08 || c == 0x0b || (0x0e..=0x1f).contains(&c) || (0x7f..=0x9f).contains(&c))
}

fn entities() -> &'static std::collections::HashMap<&'static str, &'static str> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static str>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| crate::html_entity::ENTITIES.iter().copied().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_roundtrip() {
        let raw = "<a href=\"x\">&'</a>";
        let escaped = escape_html_str(raw);
        assert_eq!("&lt;a href=&quot;x&quot;&gt;&amp;'&lt;/a&gt;", escaped);
        assert_eq!(raw, unescape_string(&escaped));
    }

    #[test]
    fn attr_val_escapes_braces_and_newline() {
        let escaped = escape_attr_val("a\nb{c}");
        assert_eq!("a_esc_newline_b&#123;c&#125;", escaped);
        assert_eq!("a\nb{c}", unescape_attr_val(&escaped));
    }

    #[test]
    fn unknown_entity_is_kept() {
        assert_eq!("&unknown;", unescape_string("&unknown;"));
        assert_eq!("&", unescape_string("&"));
    }

    #[test]
    fn numeric_entity() {
        assert_eq!("{", unescape_string("&#123;"));
        assert_eq!("😀", unescape_string("&#x1F600;"));
    }
}

#[cfg(test)]
mod go_function_boundaries {
    use super::*;

    /// html/unesc_str.go: isValidEntityCode excludes controls, surrogates and noncharacters.
    #[test]
    fn entity_scalar_boundaries() {
        for value in [
            -1, 0, 8, 11, 14, 31, 127, 159, 0xD800, 0xDFFF, 0xFDD0, 0xFDEF, 0xFFFE, 0xFFFF,
            0x10FFFF, 0x110000,
        ] {
            assert!(!is_valid_entity_code(value), "{value:x}");
            assert_eq!("�".as_bytes(), entity_code(value));
        }
        for value in [
            9, 10, 12, 13, 32, 126, 160, 0xD7FF, 0xE000, 0xFDCF, 0xFDF0, 0x10FFFD,
        ] {
            assert!(is_valid_entity_code(value), "{value:x}");
            assert_eq!(
                char::from_u32(value as u32).unwrap().to_string().as_bytes(),
                entity_code(value)
            );
        }
    }

    /// html/unesc_str.go: entity recognition is bounded and requires a semicolon.
    #[test]
    fn entity_parser_rejects_incomplete_and_oversized_entities() {
        for source in [
            b"&".as_slice(),
            b"&#",
            b"&#x",
            b"&#12",
            b"&#x12",
            b"&unknown;",
            b"&#123456789;",
            b"&#x123456789;",
        ] {
            assert!(parse_entity(source).is_none(), "{source:?}");
        }
        assert_eq!(Some((b"&".to_vec(), 5)), parse_entity(b"&amp;tail"));
        assert_eq!(
            Some(("😀".as_bytes().to_vec(), 9)),
            parse_entity(b"&#x1F600;tail")
        );
    }

    /// html/escape_encode.go: EncodeDestination retains valid escapes, escapes stray %.
    #[test]
    fn destination_percent_unicode_and_invalid_bytes() {
        for (source, expected) in [
            (b"%2f%zz%".as_slice(), b"%2F%25zz%25".as_slice()),
            ("中 文".as_bytes(), b"%E4%B8%AD%20%E6%96%87"),
            (b"\xff", b"%FF"),
        ] {
            assert_eq!(expected, encode_destination(source), "{source:?}");
        }
    }
}
