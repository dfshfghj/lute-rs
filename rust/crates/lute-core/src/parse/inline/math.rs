// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 行级公式解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseInlineMath。
    pub(super) fn parse_inline_math(&mut self, ctx: &mut InlineContext) -> Node {
        if 3 > ctx.tokens_len || !self.options.inline_math {
            ctx.pos += 1;
            return self.new_text(b"$");
        }

        let start_pos = ctx.pos;
        let mut block_start_pos = start_pos;
        let mut dollars = 0;
        while block_start_pos < ctx.tokens_len && b'$' == ctx.tokens[block_start_pos] {
            dollars += 1;
            block_start_pos += 1;
        }
        if 2 <= dollars {
            if crate::PROTYLE && self.options.protyle_wysiwyg {
                // Protyle 不允许从行级派生块级
                ctx.pos += 1;
                return self.new_text(b"$");
            }

            // 块节点
            let mut match_block = false;
            let mut block_end_pos = block_start_pos + dollars;
            while block_end_pos < ctx.tokens_len {
                let token = ctx.tokens[block_end_pos];
                if b'$' == token
                    && block_end_pos < ctx.tokens_len - 1
                    && b'$' == ctx.tokens[block_end_pos + 1]
                {
                    match_block = true;
                    break;
                }
                block_end_pos += 1;
            }
            if match_block {
                let ret = self.new_node_raw(NodeType::NodeMathBlock);
                ret.append_child(self.new_node_raw(NodeType::NodeMathBlockOpenMarker));
                let content = self.new_node_raw(NodeType::NodeMathBlockContent);
                content.set_tokens(ctx.tokens[block_start_pos..block_end_pos].to_vec());
                ret.append_child(content);
                ret.append_child(self.new_node_raw(NodeType::NodeMathBlockCloseMarker));
                ctx.pos = block_end_pos + 2;
                return ret;
            }
        }

        if !self.options.inline_math_allow_digit_after_open_marker
            && ctx.tokens_len > start_pos + 1
            && lex::is_digit(ctx.tokens[start_pos + 1])
        {
            // $ 后面不能紧跟数字
            ctx.pos += 3;
            if ctx.tokens.len() < start_pos + 3 {
                return self.new_text(&ctx.tokens[start_pos..]);
            }
            return self.new_text(&ctx.tokens[start_pos..start_pos + 3]);
        }

        let end = match_inline_math_end(
            &ctx.tokens[start_pos + 1..],
            self.options.inline_math_allow_digit_after_open_marker,
        );
        if 1 > end {
            ctx.pos += 1;
            return self.new_text(b"$");
        }

        if self.options.text_mark
            && ctx.tokens[start_pos + 1..start_pos + end as usize + 1]
                .windows(5)
                .any(|w| w == b"<span")
        {
            // 中间包含 span 节点的话打断公式，以 span 优先
            ctx.pos += 1;
            return self.new_text(b"$");
        }

        let end_pos = start_pos + end as usize + 2;

        let tokens = ctx.tokens[start_pos + 1..end_pos - 1].to_vec();
        if 1 > lex::trim_whitespace(&tokens).len() {
            ctx.pos += 1;
            return self.new_text(b"$");
        }

        let ret = self.new_node_raw(NodeType::NodeInlineMath);
        ret.append_child(self.new_node_raw(NodeType::NodeInlineMathOpenMarker));
        let content = self.new_node_raw(NodeType::NodeInlineMathContent);
        content.set_tokens(tokens);
        ret.append_child(content);
        ret.append_child(self.new_node_raw(NodeType::NodeInlineMathCloseMarker));

        ctx.pos = end_pos;
        ret
    }
}

/// 对应 Go 版 Tree.matchInlineMathEnd。
fn match_inline_math_end(tokens: &[u8], allow_digit_after_open_marker: bool) -> isize {
    let length = tokens.len();
    let mut pos = 0;
    while pos < length {
        if b'$' == tokens[pos] && 0 < pos && b'\\' != tokens[pos - 1] {
            if pos < length - 1 {
                if !lex::is_digit(tokens[pos + 1]) || allow_digit_after_open_marker {
                    return pos as isize;
                }
            } else {
                return pos as isize;
            }
        } else if b'\n' == tokens[pos] {
            return -1;
        }
        pos += 1;
    }
    -1
}
