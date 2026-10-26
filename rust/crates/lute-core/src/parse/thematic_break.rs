// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 分隔线。

use crate::ast::RawNode as Node;
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 判断分隔线（--- ***）是否开始。
pub fn thematic_break_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    if !p.parse_thematic_break() {
        return BLOCK_START_NONE;
    }

    p.close_unmatched_blocks();
    let thematic_break = p.add_child(crate::ast::NodeType::NodeThematicBreak);
    p.ctx
        .advance_offset(p.ctx.current_line_len - p.ctx.offset, false);
    let _ = thematic_break;
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    fn parse_thematic_break(&self) -> bool {
        let mut marker_cnt = 0;
        let mut marker = 0u8;
        let ln = &self.ctx.current_line;
        let length = ln.len();
        for &token in ln
            .iter()
            .take(length.saturating_sub(1))
            .skip(self.ctx.next_nonspace)
        {
            if lex::ITEM_SPACE == token || lex::ITEM_TAB == token {
                continue;
            }

            if lex::ITEM_HYPHEN != token
                && lex::ITEM_UNDERSCORE != token
                && lex::ITEM_ASTERISK != token
            {
                return false;
            }

            if 0 != marker {
                if marker != token {
                    return false;
                }
            } else {
                marker = token;
            }
            marker_cnt += 1;
        }

        3 <= marker_cnt
    }
}
