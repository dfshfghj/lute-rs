// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 链接、图片与方括号解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::html;
use crate::lex;
use crate::options::ParseOptions;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseOpenBracket。
    pub(super) fn parse_open_bracket(&mut self, ctx: &mut InlineContext) -> Node {
        let start_pos = ctx.pos;
        ctx.pos += 1;
        let tokens = ctx.tokens[start_pos..ctx.pos].to_vec();
        let ret = self.new_text(&tokens);
        ctx.add_bracket(ret, ctx.pos - 1, false);
        ret
    }

    /// 对应 Go 版 Tree.parseBang。
    pub(super) fn parse_bang(&mut self, ctx: &mut InlineContext) -> Node {
        let start_pos = ctx.pos;
        ctx.pos += 1;
        if ctx.pos < ctx.tokens_len && b'[' == ctx.tokens[ctx.pos] {
            ctx.pos += 1;
            let tokens = ctx.tokens[start_pos..ctx.pos].to_vec();
            let ret = self.new_text(&tokens);
            ctx.add_bracket(ret, start_pos + 2, true);
            return ret;
        }

        let tokens = ctx.tokens[start_pos..ctx.pos].to_vec();
        self.new_text(&tokens)
    }

    /// 对应 Go 版 Tree.parseCloseBracket。
    pub(super) fn parse_close_bracket(&mut self, ctx: &mut InlineContext) -> Node {
        let close_bracket = vec![ctx.tokens[ctx.pos]];
        ctx.pos += 1;
        let start_pos = ctx.pos;

        // 获取最新一个 [ 或者 ![
        let opener_idx = match ctx.bracket_top {
            Some(idx) => idx,
            None => return self.new_text(&close_bracket),
        };

        if !ctx.bracket(opener_idx).active {
            ctx.remove_bracket();
            return self.new_text(&close_bracket);
        }

        let is_image = ctx.bracket(opener_idx).image;

        let mut open_paren: Vec<u8> = Vec::new();
        let mut dest: Vec<u8> = Vec::new();
        let mut space: Option<Vec<u8>> = None;
        let mut title: Vec<u8> = Vec::new();
        let mut close_paren: Vec<u8> = Vec::new();
        let savepos = ctx.pos;
        let mut matched = false;

        // 尝试解析内联链接 [text](url "title")
        if ctx.pos + 1 < ctx.tokens_len && b'(' == ctx.tokens[ctx.pos] {
            ctx.pos += 1;
            let mut is_link;
            let mut passed;
            let mut remains;
            loop {
                let (ok, p, r) = lex::spnl(&ctx.tokens[ctx.pos - 1..]);
                is_link = ok;
                passed = p;
                remains = r.to_vec();
                if !is_link {
                    break;
                }
                ctx.pos += passed.len();

                match parse_inline_link_dest(&remains, self.options) {
                    Some((p, r, d)) => {
                        passed = p;
                        remains = r;
                        dest = d;
                    }
                    None => break,
                }

                if self.editor_mode() && !is_image && ctx.bracket(opener_idx).node.next().is_none()
                {
                    break;
                }

                ctx.pos += passed.len();
                open_paren = passed[0..1].to_vec();
                close_paren = passed[passed.len() - 1..].to_vec();
                matched = b')' == passed[passed.len() - 1];
                if matched && 1 < remains.len() {
                    // 解决 [foo](bar.com(baz) "bar.com(baz)") 这种情况
                    matched = !lex::is_whitespace(remains[0]) && b'"' != remains[1];
                }

                if matched {
                    ctx.pos -= 1;
                    break;
                }
                if 1 > remains.len() || !lex::is_whitespace(remains[0]) {
                    break;
                }
                // 跟空格的话后续尝试 title 解析
                let (ok, p, r) = lex::spnl(&remains);
                if !ok {
                    break;
                }
                passed = p;
                remains = r.to_vec();
                space = Some(passed.clone());
                ctx.pos += passed.len();
                if remains.is_empty() {
                    break;
                }
                matched = b')' == remains[0];
                close_paren = remains[0..1].to_vec();
                if matched {
                    break;
                }
                ctx.pos += 1;
                let (valid_title, p, r, t) = crate::parse::link::parse_link_title(&remains);
                if !valid_title {
                    break;
                }
                passed = p;
                remains = r;
                title = t.unwrap_or_default();
                ctx.pos += passed.len();
                let (ok, p, r) = lex::spnl(&remains);
                is_link = ok;
                passed = p;
                remains = r.to_vec();
                ctx.pos += passed.len();
                matched = is_link && 0 < remains.len();
                if matched {
                    if self.editor_mode() {
                        let caret = crate::editor::CARET.as_bytes();
                        if remains.starts_with("‸)".as_bytes()) {
                            if !title.is_empty() {
                                remains.drain(..caret.len() + 1);
                                let mut swapped = ")‸".as_bytes().to_vec();
                                swapped.extend_from_slice(&remains);
                                remains = swapped;
                                let start = ctx.pos - 1;
                                ctx.tokens[start..start + remains.len()].copy_from_slice(&remains);
                            } else {
                                title = caret.to_vec();
                                remains.drain(..caret.len());
                                ctx.pos += caret.len();
                            }
                        } else if remains.starts_with(")‸".as_bytes()) && title.is_empty() {
                            title = caret.to_vec();
                            remains = replace_bytes(&remains, caret, b"");
                            ctx.pos += caret.len();
                        }
                    }
                    matched = b')' == remains[0];
                }
                close_paren = remains;
                break;
            }
            if !matched {
                ctx.pos = savepos;
            }
        }

        let mut reflabel: Option<Vec<u8>> = None;
        let mut link_type = 0;
        if !matched {
            // 尝试解析链接 label
            let beforelabel = ctx.pos;
            let parsed =
                crate::parse::link::parse_link_label(&ctx.tokens[beforelabel..], self.options);
            let mut n = 0;
            if let Some((nn, _, label)) = parsed {
                n = nn;
                if 2 < n {
                    reflabel = Some(label);
                    ctx.pos += n;
                }
            }
            if 2 >= n && !ctx.bracket(opener_idx).bracket_after {
                // [text][] 格式，将 text 视为 label 进行解析
                let mut start = ctx.bracket(opener_idx).index;
                if b'[' == ctx.tokens[start] {
                    start += 1;
                }
                reflabel = Some(ctx.tokens[start..start_pos - 1].to_vec());
                ctx.pos += 2;
            }
            if 0 == n {
                ctx.pos = start_pos;
            }
            if let Some(label) = reflabel.clone() {
                if self.options.footnotes {
                    // 查找脚注
                    if let Some((idx, footnotes_def)) = self.find_footnotes_def(&label) {
                        ctx.remove_bracket();

                        // 脚注标签按字面匹配，清除标签内的分隔符及全部行级节点。
                        let previous_delimiter = ctx.bracket(opener_idx).previous_delimiter;
                        while let Some(top) = ctx.delimiter_top {
                            if Some(top) == previous_delimiter {
                                break;
                            }
                            ctx.remove_delimiter(top);
                        }
                        let opener_node = ctx.bracket(opener_idx).node;
                        while let Some(next) = opener_node.next() {
                            next.unlink();
                        }
                        opener_node.unlink(); // [

                        let mut ref_id = idx.to_string();
                        let refs_len = footnotes_def.data().footnotes_refs.len();
                        if 0 < refs_len {
                            ref_id.push_str(&format!(":{}", refs_len + 1));
                        }
                        let reference = self.new_node_raw(NodeType::NodeFootnotesRef);
                        reference.set_tokens(label.clone());
                        reference.data_mut().footnotes_ref_id = ref_id;
                        reference.data_mut().footnotes_ref_label =
                            replace_bytes(&label, crate::editor::CARET.as_bytes(), b"");
                        footnotes_def.data_mut().footnotes_refs.push(reference);
                        return reference;
                    }
                }

                // 查找链接引用定义
                if let Some(link) = self.find_link_ref_def_link(&label) {
                    if let Some(d) = link.child_by_type(NodeType::NodeLinkDest) {
                        dest = d.tokens_vec();
                    }
                    if let Some(title_node) = link.child_by_type(NodeType::NodeLinkTitle) {
                        title = title_node.tokens_vec();
                    }
                    matched = true;
                    link_type = 3;
                }
            }
        }

        if matched {
            let node = self.new_node_raw(NodeType::NodeLink);
            node.data_mut().link_type = link_type;
            if let Some(label) = &reflabel {
                node.data_mut().link_ref_label = label.clone();
            }

            let opener_node = ctx.bracket(opener_idx).node;
            if is_image {
                node.set_typ(NodeType::NodeImage);
                let bang = self.new_node_raw(NodeType::NodeBang);
                bang.set_tokens(opener_node.tokens()[..1].to_vec());
                node.append_child(bang);
                opener_node.set_tokens(opener_node.tokens()[1..].to_vec());
            }
            let open_bracket = self.new_node_raw(NodeType::NodeOpenBracket);
            open_bracket.set_tokens(opener_node.tokens_vec());
            node.append_child(open_bracket);

            let mut tmp = opener_node.next();
            while let Some(t) = tmp {
                let next = t.next();
                t.unlink();
                if NodeType::NodeText == t.typ() {
                    t.set_typ(NodeType::NodeLinkText);
                    if (crate::PROTYLE && self.options.protyle_wysiwyg)
                        && t.tokens()
                            .windows(crate::editor::CARET.len())
                            .any(|p| p == crate::editor::CARET.as_bytes())
                    {
                        let caret = self.new_text(crate::editor::CARET.as_bytes());
                        node.insert_after(caret);
                        t.set_tokens(replace_bytes(
                            t.tokens(),
                            crate::editor::CARET.as_bytes(),
                            b"",
                        ));
                    }
                }
                node.append_child(t);
                tmp = next;
            }
            let close_bracket_node = self.new_node_raw(NodeType::NodeCloseBracket);
            close_bracket_node.set_tokens(close_bracket);
            node.append_child(close_bracket_node);
            let open_paren_node = self.new_node_raw(NodeType::NodeOpenParen);
            open_paren_node.set_tokens(open_paren);
            node.append_child(open_paren_node);
            if (crate::PROTYLE && self.options.protyle_wysiwyg)
                && dest
                    .windows(crate::editor::CARET.len())
                    .any(|p| p == crate::editor::CARET.as_bytes())
            {
                let caret = self.new_text(crate::editor::CARET.as_bytes());
                node.insert_after(caret);
                dest = replace_bytes(&dest, crate::editor::CARET.as_bytes(), b"");
            }
            let dest_node = self.new_node_raw(NodeType::NodeLinkDest);
            dest_node.set_tokens(dest);
            node.append_child(dest_node);
            if let Some(space) = space {
                let space_node = self.new_node_raw(NodeType::NodeLinkSpace);
                space_node.set_tokens(space);
                node.append_child(space_node);
            }
            if 0 < title.len() {
                let title_node = self.new_node_raw(NodeType::NodeLinkTitle);
                title_node.set_tokens(title);
                node.append_child(title_node);
            }
            let close_paren_node = self.new_node_raw(NodeType::NodeCloseParen);
            close_paren_node.set_tokens(close_paren);
            node.append_child(close_paren_node);

            let previous_delimiter = ctx.bracket(opener_idx).previous_delimiter;
            self.process_emphasis(previous_delimiter, ctx);
            ctx.remove_bracket();
            opener_node.unlink();

            // 链接中不能包含链接，将更早的链接 opener 置为失效
            if !is_image {
                let mut opener = ctx.bracket_top;
                while let Some(idx) = opener {
                    if !ctx.bracket(idx).image {
                        ctx.bracket_mut(idx).active = false;
                    }
                    opener = ctx.bracket(idx).previous;
                }
            }

            node
        } else {
            ctx.remove_bracket();
            ctx.pos = start_pos;
            self.new_text(&close_bracket)
        }
    }
}

/// 对应 Go 版 Context.parseInlineLinkDest。
/// 返回 (passed, remains, destination)，None 表示未匹配。
pub(crate) fn parse_inline_link_dest(
    tokens: &[u8],
    options: &ParseOptions,
) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let length = tokens.len();
    if 2 > length {
        return None;
    }

    let mut passed: Vec<u8> = Vec::with_capacity(256);
    let mut destination: Vec<u8> = Vec::with_capacity(256);
    let remains;

    let is_pointy_brackets = b'<' == tokens[1];
    if is_pointy_brackets {
        let mut match_end = false;
        passed.push(tokens[0]);
        passed.push(tokens[1]);
        let mut i = 2;
        let mut size;
        while i < length {
            size = 1;
            let token = tokens[i];
            if b'\n' == token {
                return None;
            }

            let mut dest;
            if token < 0x80 {
                passed.push(token);
                dest = vec![token];
            } else {
                size = utf8_len(tokens, i);
                let runes = &tokens[i..i + size];
                passed.extend_from_slice(runes);
                dest = runes.to_vec();
            }
            destination.append(&mut dest);
            if b'>' == token && !lex::is_backslash_escape_punct(tokens, i) {
                destination.pop();
                match_end = true;
                break;
            }
            i += size;
        }

        if !match_end || length <= i + 1 {
            return None;
        }

        if b'>' == tokens[i + 1] || b')' == tokens[i + 1] {
            passed.push(tokens[i + 1]);
            remains = tokens[i + 2..].to_vec();
        } else {
            remains = tokens[i + 1..].to_vec();
        }
    } else {
        let mut open_parens = 0i32;
        let mut i = 0;
        let mut size;
        let mut dest_started = false;
        while i < length {
            size = 1;
            let token = tokens[i];
            let mut dest;
            if token < 0x80 {
                passed.push(token);
                dest = vec![token];
            } else {
                size = utf8_len(tokens, i);
                let runes = &tokens[i..i + size];
                passed.extend_from_slice(runes);
                dest = runes.to_vec();
            }
            destination.append(&mut dest);
            if !dest_started && !lex::is_whitespace(token) && 0 < i {
                dest_started = true;
                destination.remove(0);
                destination = lex::trim_whitespace(&destination).to_vec();
            }
            if !options.img_path_allow_space {
                if dest_started && (lex::is_whitespace(token) || lex::is_control(token)) {
                    destination.truncate(destination.len().saturating_sub(size));
                    passed.truncate(passed.len().saturating_sub(1));
                    open_parens -= 1;
                    break;
                }
            } else if dest_started && lex::is_whitespace(token) && i + 1 < length {
                let next_token = tokens[i + 1];
                if b'"' == next_token || b'\'' == next_token {
                    destination.truncate(destination.len().saturating_sub(size));
                    passed.truncate(passed.len().saturating_sub(1));
                    open_parens -= 1;
                    break;
                }
            }
            if b'(' == token && !lex::is_backslash_escape_punct(tokens, i) {
                open_parens += 1;
            }
            if b')' == token && !lex::is_backslash_escape_punct(tokens, i) {
                open_parens -= 1;
                if 1 > open_parens {
                    if destination.first() == Some(&b'(') {
                        destination.remove(0);
                    }
                    destination.pop();
                    break;
                }
            }
            i += size;
        }

        remains = tokens[i..].to_vec();
        if length > i && b')' != tokens[i] && b' ' != tokens[i] && b'\n' != tokens[i] {
            return None;
        }

        if 0 != open_parens {
            return None;
        }
    }

    if !options.data_image && destination.to_ascii_lowercase().starts_with(b"data:image") {
        return None;
    }

    if (!options.vditor_wysiwyg
        && !options.vditor_ir
        && !options.vditor_sv
        && !options.protyle_wysiwyg)
        && !options.img_path_allow_space
    {
        destination = html::encode_destination(&html::unescape_bytes(&destination));
    }
    Some((passed, remains, destination))
}

/// 返回 tokens[i..] 首字符的 UTF-8 字节长度（非法字节按 1 处理）。
fn utf8_len(tokens: &[u8], i: usize) -> usize {
    match std::str::from_utf8(&tokens[i..]) {
        Ok(s) => s.chars().next().map(|c| c.len_utf8()).unwrap_or(1),
        Err(_) => 1,
    }
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
