// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 超级块（{{{ blocks }}}），对应 Go 版 parse/super_block.go。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_CONTAINER, BLOCK_START_NONE};

/// 判断超级块是否开始。
pub fn super_block_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.super_block || p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let layout = match p.parse_super_block() {
        Some(layout) => layout,
        None => return BLOCK_START_NONE,
    };

    p.close_unmatched_blocks();
    p.add_child(NodeType::NodeSuperBlock);
    p.add_child_marker(NodeType::NodeSuperBlockOpenMarker, &[]);
    p.add_child_marker(NodeType::NodeSuperBlockLayoutMarker, &layout);
    // 整行过
    p.ctx.offset = p.ctx.current_line_len - 1;
    BLOCK_START_CONTAINER
}

impl<'a> Parser<'a> {
    pub(super) fn super_block_continue(&mut self, _super_block: Node) -> i32 {
        let tip = match self.ctx.tip {
            Some(tip) => tip,
            None => return 0,
        };
        if let Some(last) = tip.last_child() {
            if NodeType::NodeSuperBlockCloseMarker == last.typ() && last.close() {
                return 1;
            }
        }

        if is_super_block_close(&self.ctx.current_line[self.ctx.next_nonspace..]) {
            let mut n = Some(tip);
            while let Some(cur) = n {
                if NodeType::NodeSuperBlock == cur.typ() {
                    return 3; // 闭合
                }
                n = cur.parent();
            }
        }
        0
    }

    pub(super) fn super_block_finalize(&mut self, super_block: Node) {
        // 最终化所有子块
        for child in super_block.children() {
            if child.close() {
                continue;
            }
            self.finalize(child);
        }
    }

    /// 对应 Go 版 Context.closeSuperBlockChildren。
    pub(super) fn close_super_block_children(&mut self) {
        let mut n = self.ctx.tip;
        while let Some(cur) = n {
            if NodeType::NodeSuperBlock == cur.typ() {
                break;
            }
            self.finalize(cur);
            n = cur.parent();
        }
    }

    fn parse_super_block(&self) -> Option<Vec<u8>> {
        let marker = self.ctx.current_line[self.ctx.next_nonspace];
        if lex::ITEM_OPEN_BRACE != marker {
            return None;
        }

        let fence_char = marker;
        let mut fence_len = 0;
        let mut i = self.ctx.next_nonspace;
        while i < self.ctx.current_line_len && fence_char == self.ctx.current_line[i] {
            fence_len += 1;
            i += 1;
        }
        if 3 != fence_len {
            return None;
        }

        let layout =
            lex::trim_whitespace(&self.ctx.current_line[self.ctx.next_nonspace + fence_len..])
                .to_vec();
        if !layout.is_empty()
            && !layout.eq_ignore_ascii_case(b"row")
            && !layout.eq_ignore_ascii_case(b"col")
        {
            return None;
        }
        Some(layout)
    }
}

fn is_super_block_close(tokens: &[u8]) -> bool {
    let tokens = lex::trim_whitespace(tokens);
    let caret = crate::editor::CARET.as_bytes();
    let mut stripped = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i..].starts_with(caret) {
            i += caret.len();
        } else {
            stripped.push(tokens[i]);
            i += 1;
        }
    }
    b"}}}" == &stripped[..]
}
