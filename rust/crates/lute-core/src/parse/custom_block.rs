// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 围栏自定义块（;;;info）。

use crate::ast::{NodeType, RawNode as Node};
use crate::html;
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 判断围栏自定义块（;;;info）是否开始。
pub fn custom_block_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.custom_block || p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let (offset, info) = match p.parse_custom_block() {
        Some(v) => v,
        None => return BLOCK_START_NONE,
    };

    p.close_unmatched_blocks();
    let block = p.add_child(NodeType::NodeCustomBlock);
    block.data_mut().custom_block_fence_offset = offset;
    block.data_mut().custom_block_info = info;
    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(3, false);
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    pub(super) fn custom_block_continue(&mut self, custom_block: Node) -> i32 {
        let ln = self.ctx.current_line.clone();
        let indent = self.ctx.indent;
        if indent <= 3 && self.is_custom_block_close(&ln[self.ctx.next_nonspace..]) {
            self.finalize(custom_block);
            return 2;
        }

        // 跳过围栏标记符 ; 之前可能存在的空格
        let mut i = custom_block.data().custom_block_fence_offset;
        while 0 < i {
            let token = lex::peek(&ln, self.ctx.offset);
            if lex::ITEM_SPACE != token && lex::ITEM_TAB != token {
                break;
            }
            self.ctx.advance_offset(1, true);
            i -= 1;
        }
        0
    }

    pub(super) fn custom_block_finalize(&mut self, custom_block: Node) {
        let content = custom_block.tokens_vec();
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
        custom_block.set_tokens(content[i + 1..].to_vec());
    }

    fn parse_custom_block(&self) -> Option<(i32, String)> {
        let marker = self.ctx.current_line[self.ctx.next_nonspace];
        if lex::ITEM_SEMICOLON != marker {
            return None;
        }

        let mut fence_len = 0;
        let mut i = self.ctx.next_nonspace;
        while i < self.ctx.current_line_len && lex::ITEM_SEMICOLON == self.ctx.current_line[i] {
            fence_len += 1;
            i += 1;
        }

        if 3 != fence_len {
            return None;
        }

        let info_tokens = &self.ctx.current_line[self.ctx.next_nonspace + fence_len..];
        if info_tokens.contains(&lex::ITEM_SEMICOLON) {
            // info 部分不能包含 ;
            return None;
        }

        if !info_tokens.ends_with(b"\n") {
            return None;
        }

        let mut info = String::from_utf8_lossy(lex::trim_whitespace(info_tokens)).into_owned();
        info = html::unescape_string(&info);
        if let Some(idx) = info.find(' ') {
            info.truncate(idx);
        }
        if info.replace(crate::editor::CARET, "").is_empty() {
            return None;
        }

        Some((self.ctx.indent as i32, info))
    }

    fn is_custom_block_close(&mut self, tokens: &[u8]) -> bool {
        let mut tokens = lex::trim_whitespace(tokens).to_vec();
        if self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg)
        {
            let end_caret = tokens.ends_with(crate::editor::CARET.as_bytes());
            tokens = String::from_utf8_lossy(&tokens)
                .replace(crate::editor::CARET, "")
                .into_bytes();
            if end_caret {
                let tip = self.ctx.tip.expect("tip");
                let content = tip.tokens();
                let mut content = content.strip_suffix(b"\n").unwrap_or(content).to_vec();
                content.extend_from_slice(crate::editor::CARET.as_bytes());
                tip.set_tokens(content);
            }
        }
        tokens == b";;;"
    }
}

#[cfg(test)]
mod source_alignment_tests {
    use crate::{ast::NodeType, options::ParseOptions, parse::parse};

    #[test]
    fn custom_fence_requires_info_beyond_caret() {
        let options = ParseOptions {
            custom_block: true,
            protyle_wysiwyg: true,
            ..ParseOptions::new()
        };
        let tree = parse("", ";;;‸\n".as_bytes(), options.clone());
        assert_eq!(
            tree.root().first_child().unwrap().typ(),
            NodeType::NodeParagraph
        );
        let tree = parse("", ";;;info\nfoo\n;;;‸\n".as_bytes(), options);
        let block = tree.root().first_child().unwrap();
        assert_eq!(block.typ(), NodeType::NodeCustomBlock);
        assert_eq!(block.tokens(), "foo‸".as_bytes());
    }
}
