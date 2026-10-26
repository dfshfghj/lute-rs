// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 脚注定义。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_CONTAINER, BLOCK_START_NONE};

/// 判断脚注定义（[^label]）是否开始。
pub fn footnotes_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.footnotes || p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let marker = lex::peek(&p.ctx.current_line, p.ctx.next_nonspace);
    if lex::ITEM_OPEN_BRACKET != marker {
        return BLOCK_START_NONE;
    }
    let caret = lex::peek(&p.ctx.current_line, p.ctx.next_nonspace + 1);
    if lex::ITEM_CARET != caret {
        return BLOCK_START_NONE;
    }

    let mut label = vec![lex::ITEM_CARET];
    let mut i = p.ctx.next_nonspace + 2;
    while i < p.ctx.current_line_len {
        let token = p.ctx.current_line[i];
        if lex::ITEM_SPACE == token || lex::ITEM_NEWLINE == token || lex::ITEM_TAB == token {
            return BLOCK_START_NONE;
        }
        if lex::ITEM_CLOSE_BRACKET == token {
            break;
        }
        label.push(token);
        i += 1;
    }
    if i >= p.ctx.current_line_len {
        return BLOCK_START_NONE;
    }
    if lex::ITEM_COLON != lex::peek(&p.ctx.current_line, i + 1) {
        return BLOCK_START_NONE;
    }
    p.ctx.advance_offset(1, false);

    p.close_unmatched_blocks();
    p.ctx.advance_offset(label.len() + 2, true);

    if NodeType::NodeFootnotesDefBlock
        != p.ctx.tip.map(|n| n.typ()).unwrap_or(NodeType::NodeDocument)
    {
        p.add_child(NodeType::NodeFootnotesDefBlock);
    }

    let def = p.add_child(NodeType::NodeFootnotesDef);
    def.set_tokens(label);
    BLOCK_START_CONTAINER
}

impl<'a> Parser<'a> {
    pub(super) fn footnotes_continue(&mut self, _def: Node) -> i32 {
        if self.ctx.blank {
            return 0;
        }
        if 4 > self.ctx.indent {
            return 1;
        }
        self.ctx.advance_offset(4, true);
        0
    }
}
