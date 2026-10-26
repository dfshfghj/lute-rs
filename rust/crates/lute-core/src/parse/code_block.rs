// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 围栏代码块与缩进代码块。

use crate::ast::{NodeType, RawNode as Node};
use crate::html;
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

struct FencedCode {
    fence_char: u8,
    fence_len: i32,
    fence_offset: i32,
    open_fence: Vec<u8>,
    info: Vec<u8>,
}

/// 判断围栏代码块（```）是否开始。
pub fn fence_code_block_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let fenced = match p.parse_fenced_code() {
        Some(v) => v,
        None => return BLOCK_START_NONE,
    };

    p.close_unmatched_blocks();
    let code_block = p.add_child(NodeType::NodeCodeBlock);
    {
        let d = code_block.data_mut();
        d.is_fenced_code_block = true;
        d.code_block_fence_len = fenced.fence_len;
        d.code_block_fence_char = fenced.fence_char;
        d.code_block_fence_offset = fenced.fence_offset;
        d.code_block_open_fence = fenced.open_fence;
        d.code_block_info = fenced.info;
    }
    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(fenced.fence_len as usize, false);
    BLOCK_START_LEAF
}

/// 判断缩进代码块（    code）是否开始。
pub fn indent_code_block_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.indent_code_block || !p.ctx.indented {
        return BLOCK_START_NONE;
    }

    if p.ctx.tip.map(|n| n.typ()) != Some(NodeType::NodeParagraph) && !p.ctx.blank {
        p.ctx.advance_offset(4, true);
        p.close_unmatched_blocks();
        p.add_child(NodeType::NodeCodeBlock);
        return BLOCK_START_LEAF;
    }
    BLOCK_START_NONE
}

impl<'a> Parser<'a> {
    pub(super) fn code_block_continue(&mut self, code_block: Node) -> i32 {
        let ln = self.ctx.current_line.clone();
        let indent = self.ctx.indent;
        if code_block.data().is_fenced_code_block {
            let fence_char = code_block.data().code_block_fence_char;
            let fence_len = code_block.data().code_block_fence_len;
            let close = if indent <= 3 {
                self.is_fenced_code_close(&ln[self.ctx.next_nonspace..], fence_char, fence_len)
            } else {
                None
            };
            if let Some(close_fence) = close {
                code_block.data_mut().code_block_close_fence = close_fence;
                self.finalize(code_block);
                return 2;
            }

            // 跳过围栏标记符之前可能存在的空格
            let mut i = code_block.data().code_block_fence_offset;
            while 0 < i {
                let token = lex::peek(&ln, self.ctx.offset);
                if lex::ITEM_SPACE != token && lex::ITEM_TAB != token {
                    break;
                }
                self.ctx.advance_offset(1, true);
                i -= 1;
            }
        } else {
            // 缩进代码块
            if 4 <= indent {
                self.ctx.advance_offset(4, true);
            } else if self.ctx.blank {
                self.ctx.advance_next_nonspace();
            } else {
                return 1;
            }
        }
        0
    }

    pub(super) fn code_block_finalize(&mut self, code_block: Node) {
        if code_block.data().is_fenced_code_block {
            let content = code_block.tokens_vec();
            if content.is_empty() {
                return;
            }
            let mut i = 0;
            while i < content.len() {
                if lex::ITEM_NEWLINE == content[i] {
                    break;
                }
                i += 1;
            }
            code_block.set_tokens(content[i + 1..].to_vec());
        } else {
            let mut tokens = code_block.tokens_vec();
            lex::replace_newline_space(&mut tokens);
            code_block.set_tokens(tokens);
        }
    }

    fn parse_fenced_code(&self) -> Option<FencedCode> {
        let marker = self.ctx.current_line[self.ctx.next_nonspace];
        if lex::ITEM_BACKTICK != marker && lex::ITEM_TILDE != marker {
            return None;
        }

        let fence_char = marker;
        let mut fence_len = 0;
        let mut i = self.ctx.next_nonspace;
        while i < self.ctx.current_line_len && fence_char == self.ctx.current_line[i] {
            fence_len += 1;
            i += 1;
        }

        if 3 > fence_len {
            return None;
        }

        let open_fence = self.ctx.current_line
            [self.ctx.next_nonspace..self.ctx.next_nonspace + fence_len]
            .to_vec();

        let info_tokens = &self.ctx.current_line[self.ctx.next_nonspace + fence_len..];
        if (crate::PROTYLE && self.options.protyle_wysiwyg) && info_tokens.contains(&b'~') {
            return None;
        }
        if lex::ITEM_BACKTICK == marker && info_tokens.contains(&b'`') {
            // info 部分不能包含 `
            return None;
        }

        let mut info = html::unescape_bytes(lex::trim_whitespace(info_tokens));
        if let Some(idx) = info.iter().position(|&b| b == b' ') {
            info.truncate(idx);
        }

        Some(FencedCode {
            fence_char,
            fence_len: fence_len as i32,
            fence_offset: self.ctx.indent as i32,
            open_fence,
            info,
        })
    }

    fn is_fenced_code_close(
        &mut self,
        tokens: &[u8],
        open_marker: u8,
        num: i32,
    ) -> Option<Vec<u8>> {
        if tokens.is_empty() || tokens[0] != open_marker {
            return None;
        }
        if num as usize > lex::accept(tokens, open_marker) {
            return None;
        }
        let raw = lex::trim_whitespace(tokens);
        let editor = self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg);
        let tokens = if editor {
            if raw.ends_with(crate::editor::CARET.as_bytes()) {
                let tip = self.ctx.tip.expect("tip");
                let content = tip.tokens();
                let mut content = content.strip_suffix(b"\n").unwrap_or(content).to_vec();
                content.extend_from_slice(crate::editor::CARET.as_bytes());
                tip.set_tokens(content);
            }
            String::from_utf8_lossy(raw)
                .replace(crate::editor::CARET, "")
                .into_bytes()
        } else {
            raw.to_vec()
        };
        if tokens.iter().any(|&token| token != open_marker) {
            return None;
        }
        Some(tokens.to_vec())
    }
}
