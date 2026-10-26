// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 数学公式块。

use crate::ast::{NodeType, RawNode as Node};
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};
use crate::{editor, lex};

/// 判断数学公式块（$$）是否开始。
pub fn math_block_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let offset = match p.parse_math_block() {
        Some(v) => v,
        None => return BLOCK_START_NONE,
    };

    p.close_unmatched_blocks();
    let block = p.add_child(NodeType::NodeMathBlock);
    block.data_mut().math_block_dollar_offset = offset;
    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(offset as usize, false);
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    pub(super) fn math_block_continue(&mut self, math_block: Node) -> i32 {
        let ln = self.ctx.current_line.clone();
        let indent = self.ctx.indent;
        if 3 >= indent && self.is_math_block_close(&ln[self.ctx.next_nonspace..]) {
            self.finalize(math_block);
            return 2;
        }

        // 跳过 $ 之前可能存在的空格
        let mut i = math_block.data().math_block_dollar_offset;
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

    pub(super) fn math_block_finalize(&mut self, math_block: Node) {
        let tokens = math_block.tokens_vec();
        if 2 > tokens.len() {
            self.append_math_children(math_block, Vec::new());
            return;
        }

        let mut tokens = tokens[2..].to_vec();
        tokens = lex::trim_whitespace(&tokens).to_vec();
        if self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg)
        {
            let marker_caret = format!("$${}", editor::CARET);
            if tokens.ends_with(marker_caret.as_bytes()) {
                tokens.truncate(tokens.len() - marker_caret.len());
                tokens.extend_from_slice(editor::CARET.as_bytes());
            }
        }
        if tokens.ends_with(b"$$") {
            tokens.truncate(tokens.len() - 2);
        }
        if tokens
            .windows(b"<span data-type=".len())
            .any(|w| w == b"<span data-type=")
        {
            let inline_tree = crate::parse::inline("", &tokens, self.options.clone());
            tokens = inline_tree.root_raw().content().into_bytes();
        }
        self.append_math_children(math_block, tokens);
    }

    fn append_math_children(&mut self, math_block: Node, tokens: Vec<u8>) {
        math_block.clear_tokens();
        let open = self.tree.new_node_raw(NodeType::NodeMathBlockOpenMarker);
        math_block.append_child(open);
        let content = self.tree.new_node_raw(NodeType::NodeMathBlockContent);
        content.set_tokens(tokens);
        math_block.append_child(content);
        let close = self.tree.new_node_raw(NodeType::NodeMathBlockCloseMarker);
        math_block.append_child(close);
    }

    fn parse_math_block(&self) -> Option<i32> {
        let marker = self.ctx.current_line[self.ctx.next_nonspace];
        if lex::ITEM_DOLLAR != marker {
            return None;
        }

        let mut fence_length = 0;
        let mut i = self.ctx.next_nonspace;
        while i < self.ctx.current_line_len && marker == self.ctx.current_line[i] {
            fence_length += 1;
            i += 1;
        }

        if 2 > fence_length {
            return None;
        }
        Some(self.ctx.indent as i32)
    }

    fn is_math_block_close(&mut self, tokens: &[u8]) -> bool {
        // Go's block IAL interruption attaches the attributes before finalizing the math.
        if (crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial)
            && tokens.len() > b"{: id=\"".len()
            && tokens.windows(b"id=\"".len()).any(|w| w == b"id=\"")
        {
            if let Some(ial) =
                super::ial::parse_kramdown_block_ial_tokens(tokens).filter(|ial| !ial.is_empty())
            {
                if let Some(tip) = self.ctx.tip {
                    let id = ial
                        .iter()
                        .rev()
                        .find(|(key, _)| key == "id")
                        .map(|(_, value)| crate::html::unescape_attr_val(value))
                        .unwrap_or_default();
                    tip.set_id(id);
                    tip.data_mut().kramdown_ial = ial;
                    let node = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
                    node.set_tokens(tokens.to_vec());
                    tip.insert_after(node);
                    return true;
                }
            }
        }
        if tokens.is_empty() || lex::ITEM_DOLLAR != tokens[0] {
            return false;
        }
        if 2 > lex::accept(tokens, lex::ITEM_DOLLAR) {
            return false;
        }
        let tokens = lex::trim_whitespace(tokens);
        tokens.iter().all(|&token| lex::ITEM_DOLLAR == token)
    }
}
