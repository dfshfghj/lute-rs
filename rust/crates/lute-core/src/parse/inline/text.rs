// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 文本、转义与换行解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;

use super::{InlineContext, InlineParser, FULL_WIDTH_TILDE};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseText。
    pub(super) fn parse_text(&mut self, ctx: &mut InlineContext) -> Node {
        let start = ctx.pos;
        while ctx.pos < ctx.tokens_len {
            let token = ctx.tokens[ctx.pos];
            if self.is_marker(token)
                || (self.options.full_width_strikethrough
                    && ctx.tokens[ctx.pos..].starts_with(FULL_WIDTH_TILDE))
            {
                break;
            }
            ctx.pos += 1;
        }
        let tokens = ctx.tokens[start..ctx.pos].to_vec();
        self.new_text(&tokens)
    }

    /// 对应 Go 版 Tree.isMarker。
    fn is_marker(&self, token: u8) -> bool {
        if lex::is_marker(token) {
            return true;
        }
        self.options.sup && b'^' == token
    }

    /// 对应 Go 版 Tree.parseBackslash。
    pub(super) fn parse_backslash(&mut self, block: Node, ctx: &mut InlineContext) -> Option<Node> {
        if ctx.pos == ctx.tokens_len - 1 {
            ctx.pos += 1;
            return Some(self.new_text(b"\\"));
        }

        ctx.pos += 1;
        let mut token = ctx.tokens[ctx.pos];
        if b'\n' == token {
            ctx.pos += 1;
            let n = self.new_node_raw(NodeType::NodeHardBreak);
            n.set_tokens(vec![token]);
            return Some(n);
        }
        if lex::is_ascii_punct(token) {
            if b'<' == token && self.oldtip_is_table {
                // 表格单元格内存在多行时末尾输入转义符 \ 导致 <br /> 暴露
                let is_br = &ctx.tokens[ctx.pos..];
                if is_br.starts_with(b"<br />")
                    || is_br.starts_with(b"<br/>")
                    || is_br.starts_with(b"<br>")
                {
                    return Some(self.new_text(b"\\"));
                }
            }

            ctx.pos += 1;
            let n = self.new_node_raw(NodeType::NodeBackslash);
            block.append_child(n);
            let content = self.new_node_raw(NodeType::NodeBackslashContent);
            content.set_tokens(vec![token]);
            n.append_child(content);
            return None;
        }

        if self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || (crate::PROTYLE && self.options.protyle_wysiwyg)
        {
            // 处理 \‸x 情况，插入符后的字符才是待转义的
            let tokens = &ctx.tokens[ctx.pos..];
            let caret = crate::editor::CARET.as_bytes();
            if caret.len() < tokens.len() && tokens.starts_with(caret) {
                token = ctx.tokens[ctx.pos + caret.len()];
                if lex::is_ascii_punct(token) {
                    if b'<' == token && self.oldtip_is_table {
                        let is_br = &ctx.tokens[ctx.pos + caret.len()..];
                        if is_br.starts_with(b"<br />")
                            || is_br.starts_with(b"<br/>")
                            || is_br.starts_with(b"<br>")
                        {
                            return Some(self.new_text(b"\\"));
                        }
                    }

                    ctx.pos += caret.len();
                    ctx.pos += 1;
                    let n = self.new_node_raw(NodeType::NodeBackslash);
                    block.append_child(n);
                    let content = self.new_node_raw(NodeType::NodeBackslashContent);
                    content.set_tokens(vec![token]);
                    n.append_child(content);
                    if crate::PROTYLE && self.options.protyle_wysiwyg {
                        let text = self.new_node_raw(NodeType::NodeText);
                        text.set_tokens(caret.to_vec());
                        n.insert_before(text);
                    } else {
                        let text = self.new_node_raw(NodeType::NodeText);
                        text.set_tokens(caret.to_vec());
                        block.append_child(text);
                    }
                    return None;
                }
            }
        }

        Some(self.new_text(b"\\"))
    }

    /// 对应 Go 版 Tree.parseNewline。
    pub(super) fn parse_newline(&mut self, block: Node, ctx: &mut InlineContext) -> Node {
        let pos = ctx.pos;
        ctx.pos += 1;

        let mut is_hard_break = false;
        if let Some(lastc) = block.last_child() {
            if NodeType::NodeText == lastc.typ() {
                let tokens = lastc.tokens_vec();
                let value_len = tokens.len();
                if 0 < value_len && b' ' == tokens[value_len - 1] {
                    let (_, trimmed) = lex::trim_right(&tokens);
                    lastc.set_tokens(trimmed.to_vec());
                    if 1 < value_len {
                        is_hard_break = b' ' == tokens[value_len - 2];
                    }
                }
            }
        }

        let ret = self.new_node_raw(NodeType::NodeSoftBreak);
        ret.set_tokens(vec![ctx.tokens[pos]]);
        if crate::PROTYLE && self.options.protyle_wysiwyg {
            // Protyle 中的换行符都是软换行
            return ret;
        }
        if is_hard_break {
            ret.set_typ(NodeType::NodeHardBreak);
        }
        ret
    }
}
