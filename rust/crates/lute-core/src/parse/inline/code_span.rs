// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 代码段解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseCodeSpan。
    pub(super) fn parse_code_span(&mut self, block: Node, ctx: &mut InlineContext) -> Node {
        let start_pos = ctx.pos;
        let mut n = 0;
        while start_pos + n < ctx.tokens_len && b'`' == ctx.tokens[start_pos + n] {
            n += 1;
        }

        let backticks = ctx.tokens[start_pos..start_pos + n].to_vec();
        if ctx.tokens_len <= start_pos + n {
            ctx.pos += n;
            return self.new_text(&backticks);
        }
        let open_marker = self.new_node_raw(NodeType::NodeCodeSpanOpenMarker);
        open_marker.set_tokens(backticks.clone());

        let end = match_code_span_end(&ctx.tokens[start_pos + n..], n);
        if 1 > end {
            ctx.pos += n;
            return self.new_text(&backticks);
        }
        let end_pos = start_pos + end as usize + n;
        let close_marker = self.new_node_raw(NodeType::NodeCodeSpanCloseMarker);
        close_marker.set_tokens(ctx.tokens[end_pos..end_pos + n].to_vec());

        let mut text_tokens = ctx.tokens[start_pos + n..end_pos].to_vec();
        for b in text_tokens.iter_mut() {
            if b'\n' == *b {
                *b = b' ';
            }
        }
        if 2 < text_tokens.len()
            && b' ' == text_tokens[0]
            && b' ' == text_tokens[text_tokens.len() - 1]
            && !lex::is_blank_line(&text_tokens)
        {
            // 如果首尾是空格并且整行不是空行时剔除首尾的一个空格
            let mut open_tokens = open_marker.tokens_vec();
            open_tokens.push(text_tokens[0]);
            open_marker.set_tokens(open_tokens);
            let mut cm = vec![text_tokens[text_tokens.len() - 1]];
            cm.extend_from_slice(&ctx.tokens[end_pos..end_pos + n]);
            close_marker.set_tokens(cm);
            text_tokens = text_tokens[1..text_tokens.len() - 1].to_vec();
        }

        if self.options.gfm_table && NodeType::NodeTableCell == block.typ() {
            // 表格中的代码中带有管道符的处理
            text_tokens = replace_bytes(&text_tokens, b"\\|", b"|");
        }

        if crate::PROTYLE && self.options.protyle_wysiwyg {
            if !text_tokens.starts_with(b"<span data-type=\"code\">") {
                text_tokens = replace_bytes(&text_tokens, b"&", b"&amp;");
            }
            let mut options = self.options.clone();
            options.protyle_wysiwyg_auto_link = false;
            options.inline_code_html = true;
            let tree = crate::parse::inline("", &text_tokens, options);
            let mut content = Vec::new();
            crate::ast::walk_raw(tree.root_raw(), &mut |node, entering| {
                let typ = node.typ();
                if !entering {
                    if typ == NodeType::NodeLinkTitle {
                        content.push(b'"');
                    }
                    if typ == NodeType::NodeTextMark {
                        match node.data().text_mark_type.as_str() {
                            "kbd" => content.extend_from_slice(b"</kbd>"),
                            "u" => content.extend_from_slice(b"</u>"),
                            "sup" => content.extend_from_slice(b"</sup>"),
                            "sub" => content.extend_from_slice(b"</sub>"),
                            _ => {}
                        }
                    }
                    return crate::ast::WalkStatus::WalkContinue;
                }
                content.extend_from_slice(code_content_marker(typ));
                match typ {
                    NodeType::NodeTextMark => {
                        match node.data().text_mark_type.as_str() {
                            "kbd" => content.extend_from_slice(b"<kbd>"),
                            "u" => content.extend_from_slice(b"<u>"),
                            "sup" => content.extend_from_slice(b"<sup>"),
                            "sub" => content.extend_from_slice(b"<sub>"),
                            _ => {}
                        }
                        content.extend_from_slice(node.data().text_mark_text_content.as_bytes());
                    }
                    NodeType::NodeText
                    | NodeType::NodeLinkText
                    | NodeType::NodeLinkTitle
                    | NodeType::NodeLinkDest => {
                        if typ == NodeType::NodeLinkTitle {
                            content.push(b'"');
                        }
                        let tokens = node.tokens();
                        if let Some(start) =
                            tokens.windows(16).position(|p| p == b"<span data-type=")
                        {
                            if let Some(end) = tokens[start..].iter().position(|&b| b == b'>') {
                                content.extend_from_slice(&tokens[..start]);
                                content.extend_from_slice(&tokens[start + end + 1..]);
                                if let Some(close) = ctx.tokens[end_pos + 1..]
                                    .windows(9)
                                    .position(|p| p == b"</span>{:")
                                {
                                    let start = end_pos + 1 + close;
                                    if let Some(end) =
                                        ctx.tokens[start..].iter().position(|&b| b == b'}')
                                    {
                                        ctx.tokens.drain(start..=start + end);
                                    } else {
                                        ctx.tokens = replace_first(&ctx.tokens, b"</span>", b"");
                                    }
                                } else {
                                    ctx.tokens = replace_first(&ctx.tokens, b"</span>", b"");
                                }
                                ctx.tokens_len = ctx.tokens.len();
                            } else {
                                content.extend_from_slice(tokens);
                            }
                        } else {
                            content.extend_from_slice(tokens);
                        }
                    }
                    NodeType::NodeLinkSpace => content.push(b' '),
                    NodeType::NodeBackslashContent => {
                        content.push(b'\\');
                        content.extend_from_slice(node.tokens());
                    }
                    NodeType::NodeHTMLEntity => content
                        .extend_from_slice(crate::html::escape_html(&node.tokens_str()).as_bytes()),
                    NodeType::NodeInlineMathContent | NodeType::NodeEmojiAlias => {
                        content.extend_from_slice(node.tokens())
                    }
                    NodeType::NodeCodeSpanContent => {
                        content.push(b'`');
                        content.extend_from_slice(node.tokens());
                        content.push(b'`');
                    }
                    _ => {}
                }
                crate::ast::WalkStatus::WalkContinue
            });
            text_tokens =
                crate::html::unescape_html_str(&String::from_utf8_lossy(&content)).into_bytes();
        }

        let ret = self.new_node_raw(NodeType::NodeCodeSpan);
        ret.data_mut().code_marker_len = n as i32;
        ret.append_child(open_marker);
        let content = self.new_node_raw(NodeType::NodeCodeSpanContent);
        content.set_tokens(text_tokens);
        ret.append_child(content);
        ret.append_child(close_marker);
        ctx.pos = end_pos + n;
        ret
    }
}

fn replace_first(tokens: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let Some(start) = tokens.windows(from.len()).position(|part| part == from) else {
        return tokens.to_vec();
    };
    let mut result = tokens[..start].to_vec();
    result.extend_from_slice(to);
    result.extend_from_slice(&tokens[start + from.len()..]);
    result
}

fn code_content_marker(typ: NodeType) -> &'static [u8] {
    use NodeType::*;
    match typ {
        NodeTagOpenMarker | NodeTagCloseMarker => b"#",
        NodeEmA6kOpenMarker | NodeEmA6kCloseMarker => b"*",
        NodeEmU8eOpenMarker | NodeEmU8eCloseMarker => b"_",
        NodeStrongA6kOpenMarker | NodeStrongA6kCloseMarker => b"**",
        NodeStrongU8eOpenMarker | NodeStrongU8eCloseMarker => b"__",
        NodeStrikethrough2OpenMarker | NodeStrikethrough2CloseMarker => b"~~",
        NodeSupOpenMarker | NodeSupCloseMarker => b"^",
        NodeSubOpenMarker | NodeSubCloseMarker => b"~",
        NodeInlineMathOpenMarker | NodeInlineMathCloseMarker => b"$",
        NodeKbdOpenMarker => b"<kbd>",
        NodeKbdCloseMarker => b"</kbd>",
        NodeUnderlineOpenMarker => b"<u>",
        NodeUnderlineCloseMarker => b"</u>",
        NodeMark2OpenMarker | NodeMark2CloseMarker => b"==",
        NodeBang => b"!",
        NodeOpenBracket => b"[",
        NodeCloseBracket => b"]",
        NodeOpenParen => b"(",
        NodeCloseParen => b")",
        _ => b"",
    }
}

/// 对应 Go 版 Tree.matchCodeSpanEnd。
fn match_code_span_end(tokens: &[u8], num: usize) -> isize {
    let length = tokens.len();
    let mut pos = 0;
    while pos < length {
        let l = lex::accept(&tokens[pos..], b'`');
        if num == l {
            let next = pos + l;
            if length - 1 > next && b'`' == tokens[next] {
                pos = next;
                continue;
            }
            return pos as isize;
        }
        if 0 < l {
            pos += l;
        } else {
            pos += 1;
        }
    }
    -1
}

fn replace_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i..].starts_with(old) {
            ret.extend_from_slice(new);
            i += old.len();
        } else {
            ret.push(tokens[i]);
            i += 1;
        }
    }
    ret
}
