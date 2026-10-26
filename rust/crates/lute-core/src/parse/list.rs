// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 列表与列表项。

use crate::ast::{new_node_id, ListData, NodeType, RawNode as Node};
use crate::lex;
use crate::parse::ial::{ial_to_tokens, parse_kramdown_ial_in_list_item};
use crate::parse::{Parser, BLOCK_START_CONTAINER, BLOCK_START_NONE};

/// 判断列表、列表项（* - + 1.）是否开始。
pub fn list_start(p: &mut Parser, container: Node) -> i32 {
    if NodeType::NodeList != container.typ() && p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let (data, ial) = match p.parse_list_marker(container) {
        Some(v) => v,
        None => return BLOCK_START_NONE,
    };

    let empty_list_item = p.ctx.tip.filter(|tip| {
        p.options.ensure_list_item_paragraph
            && tip.typ() == NodeType::NodeListItem
            && tip
                .first_child()
                .is_none_or(|first| first.typ() == NodeType::NodeKramdownBlockIAL)
    });
    p.close_unmatched_blocks();
    if let Some(item) = empty_list_item.filter(|item| item.first_child().is_none()) {
        let paragraph = p.tree.new_node_raw(NodeType::NodeParagraph);
        if crate::CAP_KRAMDOWN_BLOCK_IAL && p.options.kramdown_block_ial {
            let id = new_node_id();
            paragraph.set_id(id.clone());
            paragraph.set_ial_attr("id", &id);
            let ial = p.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(ial_to_tokens(&paragraph.data().kramdown_ial));
            paragraph.append_child(ial);
        }
        item.append_child(paragraph);
    }

    let lists_match = NodeType::NodeList == container.typ()
        && container
            .data()
            .list_data
            .as_ref()
            .map(|ld| lists_match(ld, &data))
            .unwrap_or(false);
    let mut created_list = None;
    if p.ctx.tip.map(|n| n.typ()) != Some(NodeType::NodeList) || !lists_match {
        let list = p.add_child(NodeType::NodeList);
        list.data_mut().list_data = Some(data.clone());
        created_list = Some(list);
    }
    let list_item = p.add_child(NodeType::NodeListItem);
    list_item.data_mut().list_data = Some(data.clone());
    if (crate::CAP_KRAMDOWN_BLOCK_IAL && p.options.kramdown_block_ial) && !ial.is_empty() {
        let length = ial_to_tokens(&ial).len();
        list_item.data_mut().kramdown_ial = ial;
        list_item.set_id(list_item.ial_attr("id"));
        p.ctx.offset += length;
    }
    list_item.set_tokens(data.marker.clone());

    if 1 == data.typ || (3 == data.typ && 0 == data.bullet_char) {
        // 修正有序列表项序号
        let num = match list_item.previous() {
            Some(prev) => prev.data().list_data.as_ref().map(|d| d.num).unwrap_or(-1) + 1,
            None => data.start,
        };
        list_item.data_mut().list_data.as_mut().unwrap().num = num;
        // Go 版中列表和第一个列表项共享同一个 ListData，序号修改会同时反映到列表上
        if let Some(list) = created_list {
            if let Some(ld) = list.data_mut().list_data.as_mut() {
                ld.num = num;
            }
        }
    }
    BLOCK_START_CONTAINER
}

fn lists_match(list_data: &ListData, item_data: &ListData) -> bool {
    list_data.typ == item_data.typ
        && ((0 == list_data.delimiter && 0 == item_data.delimiter)
            || list_data.delimiter == item_data.delimiter)
        && list_data.bullet_char == item_data.bullet_char
}

impl<'a> Parser<'a> {
    pub(super) fn list_item_continue(&mut self, list_item: Node) -> i32 {
        let data = list_item.data().list_data.clone().unwrap_or_default();
        if self.ctx.blank {
            if list_item.first_child().is_none() {
                // 列表项后面是空的
                return 1;
            }
            self.ctx.advance_next_nonspace();
        } else if self.ctx.indent >= (data.marker_offset + data.padding) as usize {
            self.ctx
                .advance_offset((data.marker_offset + data.padding) as usize, true);
        } else {
            return 1;
        }
        0
    }

    pub(super) fn list_finalize(&mut self, list: Node) {
        let first_item = list.first_child();
        let mut item = list.first_child();

        // 检查子列表项之间是否包含空行，包含的话说明该列表是非紧凑的
        while let Some(li) = item {
            if ends_with_blank_line(li) && li.next().is_some() {
                mark_loose(list, first_item);
                break;
            }

            let mut sub_item = li.first_child();
            let mut loose = false;
            while let Some(sub) = sub_item {
                if ends_with_blank_line(sub) && (li.next().is_some() || sub.next().is_some()) {
                    loose = true;
                    break;
                }
                sub_item = sub.next();
            }
            if loose {
                mark_loose(list, first_item);
                break;
            }
            item = li.next();
        }
        if crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial {
            let mut current = list.first_child();
            while let Some(item) = current {
                current = item.next();
                let Some(first) = item.first_child() else {
                    if item.typ() != NodeType::NodeKramdownBlockIAL {
                        let id = new_node_id();
                        item.data_mut().kramdown_ial = vec![("id".into(), id.clone())];
                        item.set_id(id);
                        self.append_list_item_ial(item);
                        let paragraph = self.tree.new_node_raw(NodeType::NodeParagraph);
                        let id = new_node_id();
                        paragraph.set_id(id.clone());
                        paragraph.set_ial_attr("id", &id);
                        // In Go the IAL is inserted while the paragraph is still detached.
                        // Keep that allocation/order without attaching a different structure.
                        let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
                        ial.set_tokens(ial_to_tokens(&paragraph.data().kramdown_ial));
                        paragraph.insert_after(ial);
                        item.append_child(paragraph);
                    }
                    continue;
                };
                if first.tokens().len() > 7 && first.tokens().starts_with(b"{") {
                    let ial = parse_kramdown_ial_in_list_item(first.tokens());
                    if !ial.is_empty() {
                        item.set_id(ial[0].1.clone());
                        item.data_mut().kramdown_ial = ial;
                        self.append_list_item_ial(item);
                        if let Some(close) = first.tokens().iter().position(|&byte| byte == b'}') {
                            first.set_tokens(
                                lex::trim_whitespace(&first.tokens()[close + 1..]).to_vec(),
                            );
                        }
                    }
                } else {
                    if item.data().kramdown_ial.is_empty() {
                        let id = new_node_id();
                        item.set_id(id.clone());
                        item.set_ial_attr("id", &id);
                    }
                    self.append_list_item_ial(item);
                }
            }
        }
    }

    fn append_list_item_ial(&mut self, item: Node) {
        let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        ial.set_tokens(ial_to_tokens(&item.data().kramdown_ial));
        item.insert_after(ial);
    }

    /// 对应 Go 版 parseListMarker。
    fn parse_list_marker(&mut self, container: Node) -> Option<(ListData, Vec<(String, String)>)> {
        if 4 <= self.ctx.indent {
            return None;
        }

        let mut ln = self.ctx.current_line.clone();
        if crate::PROTYLE && self.options.protyle_wysiwyg {
            if let Some(dot) = ln.windows(2).position(|bytes| bytes == b". ") {
                let before = String::from_utf8_lossy(&ln[..dot]);
                if before.contains(crate::editor::CARET) {
                    let digits = before.replace(crate::editor::CARET, "");
                    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                        let line = String::from_utf8_lossy(&ln)
                            .replace(crate::editor::CARET, "")
                            .replacen(". ", &format!(". {}", crate::editor::CARET), 1);
                        ln = line.into_bytes();
                        self.ctx.current_line = ln.clone();
                    }
                }
            }
        }
        let tokens = &ln[self.ctx.next_nonspace..];
        if tokens.is_empty() {
            return None;
        }

        let mut data = ListData {
            typ: 0,
            tight: true,
            marker_offset: self.ctx.indent as i32,
            num: -1,
            ..ListData::default()
        };

        let mut marker_length = 1usize;
        let mut marker = vec![tokens[0]];
        if lex::ITEM_PLUS == marker[0]
            || lex::ITEM_HYPHEN == marker[0]
            || lex::ITEM_ASTERISK == marker[0]
        {
            data.bullet_char = marker[0];
        } else if let Some((ordered, delim)) = parse_ordered_list_marker(tokens) {
            if NodeType::NodeParagraph != container.typ() || ordered.as_slice() == b"1" {
                data.typ = 1; // 有序列表
                data.start = String::from_utf8_lossy(&ordered).parse().unwrap_or(0);
                marker_length = ordered.len() + 1;
                data.delimiter = delim;
                marker = ordered;
            } else {
                return None;
            }
        } else {
            return None;
        }

        if 1 == data.typ {
            marker.push(b'.');
        }
        data.marker = marker;

        let token = ln[self.ctx.next_nonspace + marker_length];

        // 列表项标记符后必须是空白字符
        if !lex::is_whitespace(token) {
            return None;
        }

        // 如果要打断段落，则列表项内容部分不能为空
        if NodeType::NodeParagraph == container.typ() && lex::ITEM_NEWLINE == token {
            return None;
        }

        // 到这里说明满足列表规则，开始解析并计算内部缩进空格数
        self.ctx.advance_next_nonspace();
        self.ctx.advance_offset(marker_length, true);
        let spaces_start_col = self.ctx.column;
        let spaces_start_offset = self.ctx.offset;
        loop {
            self.ctx.advance_offset(1, true);
            let token = lex::peek(&ln, self.ctx.offset);
            if self.ctx.column - spaces_start_col >= 5
                || 0 == token
                || (lex::ITEM_SPACE != token && lex::ITEM_TAB != token)
            {
                break;
            }
        }

        let token = lex::peek(&ln, self.ctx.offset);
        let is_blank_item = 0 == token || lex::ITEM_NEWLINE == token;
        let spaces_after_marker = self.ctx.column - spaces_start_col;
        if !(1..5).contains(&spaces_after_marker) || is_blank_item {
            data.padding = marker_length as i32 + 1;
            self.ctx.column = spaces_start_col;
            self.ctx.offset = spaces_start_offset;
            let token = lex::peek(&ln, self.ctx.offset);
            if lex::ITEM_SPACE == token || lex::ITEM_TAB == token {
                self.ctx.advance_offset(1, true);
            }
        } else {
            data.padding = marker_length as i32 + spaces_after_marker as i32;
        }

        let mut ial = Vec::new();
        if !is_blank_item {
            // IAL precedes the optional task marker and belongs to the list item.
            let mut tokens = &ln[self.ctx.offset..];
            if crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial {
                ial = parse_kramdown_ial_in_list_item(tokens);
                if !ial.is_empty() {
                    if let Some(end) = tokens.iter().position(|&byte| byte == b'}') {
                        tokens = &tokens[end + 1..];
                    }
                }
            }
            let normalized;
            if self.options.vditor_wysiwyg || self.options.vditor_ir || self.options.vditor_sv {
                normalized = String::from_utf8_lossy(tokens)
                    .replace(crate::editor::CARET, "")
                    .into_bytes();
                tokens = &normalized;
            }
            if 3 <= tokens.len()
                && lex::ITEM_OPEN_BRACKET == tokens[0]
                && lex::ITEM_CLOSE_BRACKET != tokens[1]
                && lex::ITEM_CLOSE_BRACKET == tokens[2]
                && (3 == tokens.len() || lex::is_whitespace(tokens[3]))
            {
                let marker = tokens[1];
                if self.options.is_valid_task_list_item_marker(marker) {
                    data.typ = 3;
                    data.checked = marker != b' ';
                }
            }
        }

        Some((data, ial))
    }
}

fn parse_ordered_list_marker(tokens: &[u8]) -> Option<(Vec<u8>, u8)> {
    let mut marker = Vec::new();
    let mut delimiter = 0u8;
    for (i, &token) in tokens.iter().enumerate() {
        if !lex::is_digit(token) || 8 < i {
            delimiter = token;
            break;
        }
        marker.push(token);
    }

    if marker.is_empty() || (lex::ITEM_DOT != delimiter && lex::ITEM_CLOSE_PAREN != delimiter) {
        return None;
    }
    Some((marker, delimiter))
}

/// 标记列表为松散模式。
///
/// Go 版中列表与第一个列表项共享同一个 ListData，所以修改会同时作用于第一个列表项。
fn mark_loose(list: Node, first_item: Option<Node>) {
    if let Some(d) = list.data_mut().list_data.as_mut() {
        d.tight = false;
    }
    if let Some(item) = first_item {
        if let Some(d) = item.data_mut().list_data.as_mut() {
            d.tight = false;
        }
    }
}

/// 判断块节点 block 是否是空行结束。列表或列表项会继续向下判断。
fn ends_with_blank_line(block: Node) -> bool {
    let mut current = Some(block);
    while let Some(n) = current {
        if n.data().last_line_blank {
            return true;
        }
        let typ = n.typ();
        if !n.data().last_line_checked
            && (NodeType::NodeList == typ || NodeType::NodeListItem == typ)
        {
            n.data_mut().last_line_checked = true;
            current = n.last_child();
        } else {
            n.data_mut().last_line_checked = true;
            break;
        }
    }
    false
}
