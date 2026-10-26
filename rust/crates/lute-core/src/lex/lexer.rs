// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 行读取器，负责行尾规范化与 NUL 替换。

use super::{ITEM_CARRIAGE_RETURN, ITEM_NEWLINE};

/// 词法分析器，对应 Go 版 lex.Lexer。
pub struct Lexer {
    input: Vec<u8>,
    length: usize,
    offset: usize,
}

impl Lexer {
    pub fn new(input: &[u8]) -> Self {
        let mut input = input.to_vec();
        if !input.is_empty() && ITEM_NEWLINE != input[input.len() - 1] {
            input.push(ITEM_NEWLINE);
        }
        let length = input.len();
        Lexer {
            input,
            length,
            offset: 0,
        }
    }

    /// 返回下一行，行尾统一为 \n。
    pub fn next_line(&mut self) -> Option<Vec<u8>> {
        if self.offset >= self.length {
            return None;
        }

        let start = self.offset;
        let mut has_nul = false;
        let mut i = self.offset;
        let mut end: Option<usize> = None;

        while i < self.length {
            let b = self.input[i];
            if ITEM_NEWLINE == b {
                i += 1;
                end = Some(i);
                break;
            } else if ITEM_CARRIAGE_RETURN == b {
                // 返回当前行时规范化行尾，并跳过 CRLF 中的 LF。
                self.input[i] = ITEM_NEWLINE;
                i += 1;
                end = Some(i);
                if i < self.length && ITEM_NEWLINE == self.input[i] {
                    i += 1;
                }
                break;
            } else if 0 == b {
                has_nul = true;
            }
            i += 1;
        }

        let end = end.unwrap_or(i);
        let mut ret = self.input[start..end].to_vec();
        if has_nul {
            let mut replaced = Vec::with_capacity(ret.len() + 4);
            for b in ret {
                if 0 == b {
                    replaced.extend_from_slice("\u{FFFD}".as_bytes());
                } else {
                    replaced.push(b);
                }
            }
            ret = replaced;
        }
        self.offset = i;
        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(input: &str) -> String {
        let mut lexer = Lexer::new(input.as_bytes());
        let mut out = String::new();
        while let Some(line) = lexer.next_line() {
            out.push_str(&String::from_utf8_lossy(&line));
        }
        out
    }

    #[test]
    fn normalize_lines() {
        let cases = [
            ("", ""),
            ("a", "a\n"),
            ("a\n", "a\n"),
            ("a\r", "a\n"),
            ("a\r\nb\rc\n", "a\nb\nc\n"),
            ("\r\r\n\n", "\n\n\n"),
            ("\x00", "\u{FFFD}\n"),
            (
                "a\x00\x00b\r\nc\x00\rd",
                "a\u{FFFD}\u{FFFD}b\nc\u{FFFD}\nd\n",
            ),
            ("中文\r\n日文\x00\r", "中文\n日文\u{FFFD}\n"),
        ];
        for (input, want) in cases {
            assert_eq!(want, normalize(input), "input={:?}", input);
        }
    }

    #[test]
    fn end_of_input_returns_none() {
        let mut lexer = Lexer::new(b"a");
        assert!(lexer.next_line().is_some());
        assert!(lexer.next_line().is_none());
        assert!(lexer.next_line().is_none());
    }
}
