// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 引述。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_CONTAINER, BLOCK_START_NONE};

/// 判断引述（>）是否开始。
pub fn blockquote_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let marker = lex::peek(&p.ctx.current_line, p.ctx.next_nonspace);
    if lex::ITEM_GREATER != marker {
        return BLOCK_START_NONE;
    }

    let mut markers = vec![marker];
    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(1, false);
    // > 后面的空格是可选的
    let whitespace = lex::peek(&p.ctx.current_line, p.ctx.offset);
    if lex::ITEM_SPACE == whitespace || lex::ITEM_TAB == whitespace {
        p.ctx.advance_offset(1, true);
        markers.push(whitespace);
    }
    p.close_unmatched_blocks();
    p.add_child(NodeType::NodeBlockquote);
    p.add_child_marker(NodeType::NodeBlockquoteMarker, &markers);
    BLOCK_START_CONTAINER
}

impl<'a> Parser<'a> {
    /// 对应 Go 版 BlockquoteContinue。
    pub(super) fn blockquote_continue(&mut self, _blockquote: Node) -> i32 {
        if !self.ctx.indented
            && lex::peek(&self.ctx.current_line, self.ctx.next_nonspace) == lex::ITEM_GREATER
        {
            self.ctx.advance_next_nonspace();
            self.ctx.advance_offset(1, false);
            let token = lex::peek(&self.ctx.current_line, self.ctx.offset);
            if lex::ITEM_SPACE == token || lex::ITEM_TAB == token {
                self.ctx.advance_offset(1, true);
            }
            return 0;
        }
        1
    }
}
