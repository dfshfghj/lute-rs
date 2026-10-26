// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 强调、加粗、删除线等分隔符解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::editor;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.handleDelim。
    pub(super) fn handle_delim(&mut self, block: Node, ctx: &mut InlineContext) {
        let marker = vec![ctx.tokens[ctx.pos]];
        let typ = ctx.tokens[ctx.pos];
        self.handle_delim_by_marker(block, ctx, &marker, typ, false);
    }

    /// 对应 Go 版 Tree.handleDelimByMarker。
    pub(super) fn handle_delim_by_marker(
        &mut self,
        block: Node,
        ctx: &mut InlineContext,
        marker: &[u8],
        typ: u8,
        full_width_strikethrough: bool,
    ) {
        let start_pos = ctx.pos;
        let (num, can_open, can_close) =
            self.scan_delims(ctx, marker, typ, full_width_strikethrough);

        let text = ctx.tokens[start_pos..ctx.pos].to_vec();
        let node = self.new_text(&text);
        block.append_child(node);

        if can_open || can_close {
            let previous = ctx.delimiter_top;
            let idx = ctx.delimiters.len();
            ctx.delimiters.push(super::Delimiter {
                node,
                typ,
                num,
                original_num: num,
                unit_len: marker.len(),
                can_open,
                can_close,
                previous,
                next: None,
                active: false,
                image: false,
                bracket_after: false,
                index: 0,
                previous_delimiter: None,
            });
            ctx.delimiter_top = Some(idx);
            if let Some(prev) = previous {
                ctx.delimiter_mut(prev).next = Some(idx);
            }
        }
    }

    /// 对应 Go 版 Tree.scanDelims。
    fn scan_delims(
        &mut self,
        ctx: &mut InlineContext,
        marker: &[u8],
        typ: u8,
        full_width_strikethrough: bool,
    ) -> (i32, bool, bool) {
        let start_pos = ctx.pos;
        let mut delimiters_count = 0;
        while ctx.pos < ctx.tokens_len && ctx.tokens[ctx.pos..].starts_with(marker) {
            // #Tag# 标记使用一个井号，不贪婪合并连续井号
            if b'#' == typ && self.options.tag && delimiters_count >= 1 {
                break;
            }
            delimiters_count += 1;
            ctx.pos += marker.len();
        }

        let mut token_before = '\n';
        let mut token_after = '\n';
        if 0 < start_pos {
            let c = ctx.tokens[start_pos - 1];
            if c >= 0x80 {
                token_before = decode_last_char(&ctx.tokens[..start_pos]);
            } else {
                token_before = c as char;
            }

            if self.editor_mode() && editor::CARET == token_before.to_string() {
                // 跳过插入符位置向前看
                let caret_len = editor::CARET.len();
                if 0 < start_pos as isize - caret_len as isize {
                    let end = start_pos - caret_len;
                    let c = ctx.tokens[end - 1];
                    if c >= 0x80 {
                        token_before = decode_last_char(&ctx.tokens[..end]);
                    } else {
                        token_before = c as char;
                    }
                }
            }
        }

        if ctx.tokens_len > ctx.pos {
            let t = ctx.tokens[ctx.pos];
            if t >= 0x80 {
                token_after = decode_char(&ctx.tokens[ctx.pos..]);
            } else {
                token_after = t as char;
            }
        }

        let mut after_is_whitespace = token_after.is_whitespace();
        let mut after_is_punct = is_punct_or_symbol(token_after);
        if (b'*' == typ && '~' == token_after)
            || (b'~' == typ && '*' == token_after)
            || (b'^' == typ && ('+' == token_after || '-' == token_after))
            || (b'~' == typ && ('+' == token_after || '-' == token_after))
        {
            after_is_punct = false;
        }
        let before_is_whitespace = token_before.is_whitespace();
        let mut before_is_punct = is_punct_or_symbol(token_before);
        if (b'*' == typ && '~' == token_before)
            || (b'~' == typ && '*' == token_before)
            || (b'^' == typ && ('+' == token_before || '-' == token_before))
            || (b'~' == typ && ('+' == token_before || '-' == token_before))
        {
            before_is_punct = false;
        }

        if crate::PROTYLE && self.options.protyle_wysiwyg {
            if b'#' != typ {
                after_is_punct = false;
                before_is_punct = false;
            }
            if b'_' == typ && editor::CARET == token_before.to_string() {
                after_is_whitespace = true;
            }
            if b'_' == typ && editor::CARET == token_after.to_string() {
                after_is_whitespace = true;
            }
        }

        let is_left_flanking =
            !after_is_whitespace && (!after_is_punct || before_is_whitespace || before_is_punct);
        let is_right_flanking =
            !before_is_whitespace && (!before_is_punct || after_is_whitespace || after_is_punct);
        let mut can_open;
        let mut can_close;
        if b'_' == typ {
            can_open = is_left_flanking && (!is_right_flanking || before_is_punct);
            can_close = is_right_flanking && (!is_left_flanking || after_is_punct);
        } else if b'=' == typ {
            if !self.options.mark || 2 != delimiters_count {
                can_open = false;
                can_close = false;
            } else {
                can_open = is_left_flanking;
                can_close = is_right_flanking;
            }
        } else if b'#' == typ {
            if !self.options.tag || 1 != delimiters_count {
                can_open = false;
                can_close = false;
            } else {
                can_open = is_left_flanking;
                can_close = is_right_flanking;
            }
        } else if b'^' == typ {
            if !self.options.sup || 1 != delimiters_count {
                can_open = false;
                can_close = false;
            } else {
                can_open = is_left_flanking;
                can_close = is_right_flanking;
            }
        } else if b'~' == typ {
            if full_width_strikethrough && 2 != delimiters_count {
                return (delimiters_count, false, false);
            }
            if self.options.sub {
                if self.options.gfm_strikethrough && 3 == delimiters_count {
                    can_open = is_left_flanking;
                    can_close = is_right_flanking;
                } else if 1 != delimiters_count {
                    can_open = false;
                    can_close = false;
                    if self.options.gfm_strikethrough && 2 == delimiters_count {
                        can_open = is_left_flanking;
                        can_close = is_right_flanking;
                    }
                } else {
                    can_open = is_left_flanking;
                    can_close = is_right_flanking;
                }
            } else if self.options.gfm_strikethrough {
                if 1 == delimiters_count {
                    if !self.options.gfm_strikethrough1 {
                        can_open = false;
                        can_close = false;
                    } else {
                        can_open = is_left_flanking;
                        can_close = is_right_flanking;
                    }
                } else {
                    can_open = is_left_flanking;
                    can_close = is_right_flanking;
                }
            } else {
                can_open = is_left_flanking;
                can_close = is_right_flanking;
            }
        } else {
            can_open = is_left_flanking;
            can_close = is_right_flanking;
        }
        (delimiters_count, can_open, can_close)
    }

    /// 对应 Go 版 Tree.processEmphasis。
    pub(super) fn process_emphasis(
        &mut self,
        stack_bottom: Option<usize>,
        ctx: &mut InlineContext,
    ) {
        if ctx.delimiter_top.is_none() {
            return;
        }

        let mut openers_bottom: [Option<usize>; 128] = [None; 128];
        for b in [b'_', b'*', b'~', b'=', b'#', b'^'] {
            openers_bottom[b as usize] = stack_bottom;
        }

        // 找到 stack_bottom 之上的第一个 closer
        let mut closer = ctx.delimiter_top;
        while let Some(c) = closer {
            if ctx.delimiter(c).previous == stack_bottom {
                break;
            }
            closer = ctx.delimiter(c).previous;
        }

        let mut odd_match = false;
        while let Some(closer_idx) = closer {
            let closercc = ctx.delimiter(closer_idx).typ;
            if !ctx.delimiter(closer_idx).can_close {
                closer = ctx.delimiter(closer_idx).next;
                continue;
            }

            // 向后查找第一个匹配的 opener
            let mut opener = ctx.delimiter(closer_idx).previous;
            let mut opener_found = false;
            while let Some(opener_idx) = opener {
                if Some(opener_idx) == stack_bottom
                    || Some(opener_idx) == openers_bottom[closercc as usize]
                {
                    break;
                }
                let (o_orig, o_can_close, o_typ, o_can_open) = {
                    let o = ctx.delimiter(opener_idx);
                    (o.original_num, o.can_close, o.typ, o.can_open)
                };
                let (c_orig, c_can_open) = {
                    let c = ctx.delimiter(closer_idx);
                    (c.original_num, c.can_open)
                };
                odd_match =
                    (c_can_open || o_can_close) && c_orig % 3 != 0 && (o_orig + c_orig) % 3 == 0;
                if o_typ == closercc && o_can_open && !odd_match {
                    opener_found = true;
                    break;
                }
                opener = ctx.delimiter(opener_idx).previous;
            }
            let old_closer = closer_idx;

            if !opener_found {
                closer = ctx.delimiter(closer_idx).next;
            } else {
                let opener_idx = opener.expect("opener found");

                if b'#' == closercc && self.options.tag {
                    let closer_node = ctx.delimiter(closer_idx).node;
                    let mut tag_content = String::new();
                    let mut n = ctx.delimiter(opener_idx).node.next();
                    while let Some(node) = n {
                        if node == closer_node {
                            break;
                        }
                        tag_content.push_str(&node.text());
                        n = node.next();
                    }
                    let content = tag_content
                        .replace(editor::CARET, "")
                        .replace(editor::ZWSP, "");
                    if content.trim().is_empty() {
                        closer = ctx.delimiter(closer_idx).next;
                        continue;
                    }
                }

                let (o_num, c_num) = (ctx.delimiter(opener_idx).num, ctx.delimiter(closer_idx).num);
                let use_delims = if 2 <= c_num && 2 <= o_num { 2 } else { 1 };

                let opener_inl = ctx.delimiter(opener_idx).node;
                let closer_inl = ctx.delimiter(closer_idx).node;

                if self.options.gfm_strikethrough || self.options.sub {
                    if b'~' == closercc && o_num != c_num {
                        closer = ctx.delimiter(closer_idx).next;
                        continue;
                    }
                } else if b'~' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                if self.options.sup {
                    if b'^' == closercc && o_num != c_num {
                        closer = ctx.delimiter(closer_idx).next;
                        continue;
                    }
                } else if b'^' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                if !self.options.inline_asterisk && b'*' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                if !self.options.inline_underscore && b'_' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                if self.options.mark {
                    if b'=' == closercc && o_num != c_num {
                        closer = ctx.delimiter(closer_idx).next;
                        continue;
                    }
                } else if b'=' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                if self.options.tag {
                    if b'#' == closercc && o_num != c_num {
                        closer = ctx.delimiter(closer_idx).next;
                        continue;
                    }
                } else if b'#' == closercc {
                    closer = ctx.delimiter(closer_idx).next;
                    continue;
                }

                ctx.delimiter_mut(opener_idx).num -= use_delims;
                ctx.delimiter_mut(closer_idx).num -= use_delims;

                let opener_unit = ctx.delimiter(opener_idx).unit_len;
                let closer_unit = ctx.delimiter(closer_idx).unit_len;
                let opener_token_len = use_delims as usize * opener_unit;
                let closer_token_len = use_delims as usize * closer_unit;

                let ot = opener_inl.tokens_vec();
                let mut opener_tokens = ot[ot.len() - opener_token_len..].to_vec();
                opener_inl.set_tokens(ot[..ot.len() - opener_token_len].to_vec());
                let ct = closer_inl.tokens_vec();
                let mut closer_tokens = ct[ct.len() - closer_token_len..].to_vec();
                closer_inl.set_tokens(ct[..ct.len() - closer_token_len].to_vec());
                if 1 < opener_unit {
                    opener_tokens = vec![b'~'; use_delims as usize];
                }
                if 1 < closer_unit {
                    closer_tokens = vec![b'~'; use_delims as usize];
                }

                let open_marker = self.new_node_raw(NodeType::NodeText);
                open_marker.set_tokens(opener_tokens);
                let em_strong = self.new_node_raw(NodeType::NodeText);
                let close_marker = self.new_node_raw(NodeType::NodeText);
                close_marker.set_tokens(closer_tokens);
                open_marker.data_mut().close = true;
                em_strong.data_mut().close = true;
                close_marker.data_mut().close = true;

                if 1 == use_delims {
                    if b'*' == closercc {
                        em_strong.set_typ(NodeType::NodeEmphasis);
                        open_marker.set_typ(NodeType::NodeEmA6kOpenMarker);
                        close_marker.set_typ(NodeType::NodeEmA6kCloseMarker);
                    } else if b'_' == closercc {
                        em_strong.set_typ(NodeType::NodeEmphasis);
                        open_marker.set_typ(NodeType::NodeEmU8eOpenMarker);
                        close_marker.set_typ(NodeType::NodeEmU8eCloseMarker);
                    } else if b'~' == closercc {
                        if self.options.sub {
                            em_strong.set_typ(NodeType::NodeSub);
                            open_marker.set_typ(NodeType::NodeSubOpenMarker);
                            close_marker.set_typ(NodeType::NodeSubCloseMarker);
                        } else if self.options.gfm_strikethrough && self.options.gfm_strikethrough1
                        {
                            em_strong.set_typ(NodeType::NodeStrikethrough);
                            open_marker.set_typ(NodeType::NodeStrikethrough1OpenMarker);
                            close_marker.set_typ(NodeType::NodeStrikethrough1CloseMarker);
                        }
                    } else if b'=' == closercc {
                        if self.options.mark {
                            em_strong.set_typ(NodeType::NodeMark);
                            open_marker.set_typ(NodeType::NodeMark1OpenMarker);
                            close_marker.set_typ(NodeType::NodeMark1CloseMarker);
                        }
                    } else if b'#' == closercc {
                        if self.options.tag {
                            em_strong.set_typ(NodeType::NodeTag);
                            open_marker.set_typ(NodeType::NodeTagOpenMarker);
                            close_marker.set_typ(NodeType::NodeTagCloseMarker);
                        }
                    } else if b'^' == closercc && self.options.sup {
                        em_strong.set_typ(NodeType::NodeSup);
                        open_marker.set_typ(NodeType::NodeSupOpenMarker);
                        close_marker.set_typ(NodeType::NodeSupCloseMarker);
                    }
                } else if b'*' == closercc {
                    em_strong.set_typ(NodeType::NodeStrong);
                    open_marker.set_typ(NodeType::NodeStrongA6kOpenMarker);
                    close_marker.set_typ(NodeType::NodeStrongA6kCloseMarker);
                } else if b'_' == closercc {
                    em_strong.set_typ(NodeType::NodeStrong);
                    open_marker.set_typ(NodeType::NodeStrongU8eOpenMarker);
                    close_marker.set_typ(NodeType::NodeStrongU8eCloseMarker);
                } else if b'~' == closercc {
                    if self.options.gfm_strikethrough {
                        em_strong.set_typ(NodeType::NodeStrikethrough);
                        open_marker.set_typ(NodeType::NodeStrikethrough2OpenMarker);
                        close_marker.set_typ(NodeType::NodeStrikethrough2CloseMarker);
                    }
                } else if b'=' == closercc {
                    if self.options.mark {
                        em_strong.set_typ(NodeType::NodeMark);
                        open_marker.set_typ(NodeType::NodeMark2OpenMarker);
                        close_marker.set_typ(NodeType::NodeMark2CloseMarker);
                    }
                }

                let mut tmp = opener_inl.next();
                while let Some(t) = tmp {
                    if t == closer_inl {
                        break;
                    }
                    let next = t.next();
                    t.unlink();
                    em_strong.append_child(t);
                    tmp = next;
                }

                em_strong.prepend_child(open_marker);
                em_strong.append_child(close_marker);
                opener_inl.insert_after(em_strong);

                // 移除 opener 与 closer 之间的分隔符
                if ctx.delimiter(opener_idx).next != Some(closer_idx) {
                    ctx.delimiter_mut(opener_idx).next = Some(closer_idx);
                    ctx.delimiter_mut(closer_idx).previous = Some(opener_idx);
                }

                if 0 == ctx.delimiter(opener_idx).num {
                    opener_inl.unlink();
                    ctx.remove_delimiter(opener_idx);
                }

                if 0 == ctx.delimiter(closer_idx).num {
                    closer_inl.unlink();
                    let temp = ctx.delimiter(closer_idx).next;
                    ctx.remove_delimiter(closer_idx);
                    closer = temp;
                }
            }

            if !opener_found && !odd_match {
                // 为后续查找设置下限
                openers_bottom[closercc as usize] = ctx.delimiter(old_closer).previous;
                if !ctx.delimiter(old_closer).can_open {
                    ctx.remove_delimiter(old_closer);
                }
            }
        }

        // 移除所有分隔符
        while let Some(top) = ctx.delimiter_top {
            if Some(top) == stack_bottom {
                break;
            }
            ctx.remove_delimiter(top);
        }
    }
}

/// 判断字符是否属于 Unicode 标点或符号。
pub(super) fn is_punct_or_symbol(c: char) -> bool {
    let cp = c as u32;
    crate::char_class::PUNCT_OR_SYMBOL
        .binary_search_by(|&(start, end)| {
            if cp < start {
                std::cmp::Ordering::Greater
            } else if cp > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// 解码 tokens 末尾的字符，非法 UTF-8 按 U+FFFD 处理。
fn decode_last_char(tokens: &[u8]) -> char {
    let len = tokens.len();
    let from = len.saturating_sub(4);
    for start in (from..len).rev() {
        if let Ok(s) = std::str::from_utf8(&tokens[start..]) {
            let mut it = s.chars();
            if let (Some(c), None) = (it.next(), it.next()) {
                return c;
            }
        }
    }
    '\u{FFFD}'
}

/// 解码 tokens 开头的字符，非法 UTF-8 按 U+FFFD 处理。
fn decode_char(tokens: &[u8]) -> char {
    std::str::from_utf8(tokens)
        .ok()
        .and_then(|s| s.chars().next())
        .unwrap_or('\u{FFFD}')
}
