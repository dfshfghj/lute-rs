// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML 实体解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::html;
use crate::lex;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseEntity。
    pub(super) fn parse_entity(&mut self, ctx: &mut InlineContext) -> Node {
        let and = vec![ctx.tokens[ctx.pos]];
        if 2 > ctx.tokens_len || ctx.tokens_len <= ctx.pos + 1 {
            ctx.pos += 1;
            return self.new_text(&and);
        }

        let start = ctx.pos;
        let mut numeric = false;
        if 3 < ctx.tokens_len {
            numeric = b'#' == ctx.tokens[start + 1];
        }
        let mut i = ctx.pos;
        let mut token;
        let mut end_with_semicolon = false;
        while i < ctx.tokens_len {
            token = ctx.tokens[i];
            if lex::is_whitespace(token) {
                break;
            }
            if b';' == token {
                i += 1;
                end_with_semicolon = true;
                break;
            }
            i += 1;
        }

        if !end_with_semicolon {
            ctx.pos += 1;
            return self.new_text(&and);
        }

        let entity_name = ctx.tokens[start..i].to_vec();
        if numeric {
            let entity_name_len = entity_name.len();
            if 10 < entity_name_len || 4 > entity_name_len {
                ctx.pos += 1;
                return self.new_text(&and);
            }

            if (b'x' == entity_name[2] || b'X' == entity_name[2]) && 5 > entity_name_len {
                ctx.pos += 1;
                return self.new_text(&and);
            }
        }

        let v = html::unescape_string(std::str::from_utf8(&entity_name).unwrap_or(""));
        if v.as_bytes() == entity_name.as_slice() {
            ctx.pos += 1;
            return self.new_text(&and);
        }
        ctx.pos += i - start;
        let n = self.new_node_raw(NodeType::NodeHTMLEntity);
        n.set_tokens(v.into_bytes());
        n.data_mut().html_entity_tokens = entity_name;
        n
    }
}
