// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 链接引用定义。

use crate::ast::{NodeType, RawNode as Node};
use crate::html;
use crate::lex;
use crate::parse::Parser;

impl<'a> Parser<'a> {
    /// 对应 Go 版 Context.parseLinkRefDef。返回剩余 Tokens，未匹配返回 None。
    pub(super) fn parse_link_ref_def(&mut self, tokens: &[u8]) -> Option<Vec<u8>> {
        if !self.options.link_ref {
            return None;
        }

        let (_, tokens) = lex::trim_left(tokens);
        if tokens.is_empty() {
            return None;
        }

        let (n, remains, label) = parse_link_label(tokens, &self.options)?;
        if 2 > n || label.is_empty() {
            return None;
        }

        if remains.is_empty() || b':' != remains[0] {
            return None;
        }

        let remains = &remains[1..];
        let (whitespaces, remains) = lex::trim_left(remains);
        let (newlines, _, _) = lex::stat_whitespace(&whitespaces);
        if 1 < newlines {
            return None;
        }

        let (link_dest, mut remains, destination) = self.parse_link_dest(remains)?;
        let _ = link_dest;

        let (whitespaces, remains2) = lex::trim_left(&remains);
        if whitespaces.is_empty() && !remains2.is_empty() {
            return None;
        }
        let (newlines, spaces1, tabs1) = lex::stat_whitespace(&whitespaces);

        let (_, tokens_after) = lex::trim_left(remains2);
        let (valid_title, _, title_remains, title) = parse_link_title(tokens_after);
        if !valid_title && 1 > newlines {
            return None;
        }
        if 0 < spaces1 + tabs1
            && !lex::is_blank_line(&title_remains)
            && lex::ITEM_NEWLINE != title_remains[0]
        {
            return None;
        }

        let title_line = tokens_after.to_vec();
        let (whitespaces, tokens_after) = lex::trim_left(&title_remains);
        let (_, spaces2, tabs2) = lex::stat_whitespace(&whitespaces);
        remains = if !lex::is_blank_line(tokens_after) && 0 < spaces2 + tabs2 {
            title_line
        } else {
            tokens_after.to_vec()
        };

        let link = self.new_link(
            NodeType::NodeLink,
            &label,
            &destination,
            title.as_deref(),
            1,
        );
        let def = self.tree.new_node_raw(NodeType::NodeLinkRefDef);
        def.set_tokens(label);
        def.append_child(link);

        let mut def_block = self.ctx.tip.expect("tip");
        if NodeType::NodeLinkRefDefBlock != def_block.typ() {
            def_block = self.tree.new_node_raw(NodeType::NodeLinkRefDefBlock);
        }
        def_block.append_child(def);
        self.ctx
            .tip
            .expect("tip")
            .parent()
            .expect("tip parent")
            .append_child(def_block);

        Some(remains)
    }

    /// 对应 Go 版 Context.parseLinkDest。
    fn parse_link_dest(&self, tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let (ret, remains, destination) = match self.parse_link_dest1(tokens) {
            Some(v) => v,
            None => self.parse_link_dest2(tokens)?,
        };
        let destination = if !self.options.vditor_wysiwyg
            && !self.options.vditor_ir
            && !self.options.vditor_sv
            && !(crate::PROTYLE && self.options.protyle_wysiwyg)
        {
            html::encode_destination(&html::unescape_bytes(&destination))
        } else {
            destination
        };
        Some((ret, remains, destination))
    }

    fn parse_link_dest1(&self, tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let length = tokens.len();
        if 2 > length || lex::ITEM_LESS != tokens[0] {
            return None;
        }

        let mut ret = Vec::with_capacity(256);
        let mut destination = Vec::with_capacity(256);
        let mut closed = false;
        let mut i = 0;
        while i < length {
            let token = tokens[i];
            ret.push(token);
            let mut size = 1;
            if 0 < i {
                size = utf8_len(tokens, i);
                ret.extend_from_slice(&tokens[i + 1..i + size]);
                destination.extend_from_slice(&tokens[i..i + size]);
                if lex::ITEM_LESS == token && !lex::is_backslash_escape_punct(tokens, i) {
                    return None;
                }
            }

            if lex::ITEM_GREATER == token && !lex::is_backslash_escape_punct(tokens, i) {
                closed = true;
                destination.truncate(destination.len().saturating_sub(1));
                break;
            }
            i += size;
        }

        if !closed {
            return None;
        }

        Some((ret, tokens[i + 1..].to_vec(), destination))
    }

    fn parse_link_dest2(&self, tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let length = tokens.len();
        if 1 > length {
            return None;
        }

        let mut ret = Vec::with_capacity(256);
        let mut destination = Vec::with_capacity(256);
        let mut open_parens = 0i32;
        let mut i = 0;
        while i < length {
            let token = tokens[i];
            ret.push(token);
            let size = utf8_len(tokens, i);
            ret.extend_from_slice(&tokens[i + 1..i + size]);
            destination.extend_from_slice(&tokens[i..i + size]);

            if lex::is_whitespace(token) || lex::is_control(token) {
                destination.truncate(destination.len().saturating_sub(1));
                ret.truncate(ret.len().saturating_sub(1));
                break;
            }

            if lex::ITEM_OPEN_PAREN == token && !lex::is_backslash_escape_punct(tokens, i) {
                open_parens += 1;
            }
            if lex::ITEM_CLOSE_PAREN == token && !lex::is_backslash_escape_punct(tokens, i) {
                open_parens -= 1;
                if 1 > open_parens {
                    i += 1;
                    break;
                }
            }

            i += size;
        }

        let remains = tokens[i..].to_vec();
        if length > i && !lex::is_whitespace(tokens[i]) {
            return None;
        }

        Some((ret, remains, destination))
    }

    /// 对应 Go 版 Tree.newLink。
    fn new_link(
        &mut self,
        typ: NodeType,
        text: &[u8],
        dest: &[u8],
        title: Option<&[u8]>,
        link_type: i32,
    ) -> Node {
        let text = html::decode_destination(text);

        let ret = self.tree.new_node_raw(typ);
        ret.data_mut().link_type = link_type;
        if NodeType::NodeImage == typ {
            let bang = self.tree.new_node_raw(NodeType::NodeBang);
            ret.append_child(bang);
        }
        let open = self.tree.new_node_raw(NodeType::NodeOpenBracket);
        ret.append_child(open);
        let link_text = self.tree.new_node_raw(NodeType::NodeLinkText);
        link_text.set_tokens(text.clone());
        ret.append_child(link_text);
        let close = self.tree.new_node_raw(NodeType::NodeCloseBracket);
        ret.append_child(close);
        let open_paren = self.tree.new_node_raw(NodeType::NodeOpenParen);
        ret.append_child(open_paren);
        let link_dest = self.tree.new_node_raw(NodeType::NodeLinkDest);
        link_dest.set_tokens(dest.to_vec());
        ret.append_child(link_dest);
        if let Some(title) = title {
            let link_title = self.tree.new_node_raw(NodeType::NodeLinkTitle);
            link_title.set_tokens(title.to_vec());
            ret.append_child(link_title);
        }
        let close_paren = self.tree.new_node_raw(NodeType::NodeCloseParen);
        ret.append_child(close_paren);
        if 1 == link_type {
            ret.data_mut().link_ref_label = text;
        }
        ret
    }
}

pub(super) fn parse_link_label(
    tokens: &[u8],
    options: &crate::options::ParseOptions,
) -> Option<(usize, Vec<u8>, Vec<u8>)> {
    let length = tokens.len();
    if 2 > length || lex::ITEM_OPEN_BRACKET != tokens[0] {
        return None;
    }

    let mut label = Vec::new();
    let mut closed = false;
    let mut i = 1;
    let mut remains = Vec::new();
    while i < length {
        let size = utf8_len(tokens, i);
        label.extend_from_slice(&tokens[i..i + size]);
        if lex::ITEM_CLOSE_BRACKET == tokens[i] && !lex::is_backslash_escape_punct(tokens, i) {
            closed = true;
            label.truncate(label.len() - 1);
            remains = tokens[i + 1..].to_vec();
            break;
        }
        if lex::ITEM_OPEN_BRACKET == tokens[i] && !lex::is_backslash_escape_punct(tokens, i) {
            return None;
        }
        i += size;
    }

    if !closed || 999 < label.len() {
        return None;
    }

    let n = i + 1;
    let mut label = lex::trim_whitespace(&label).to_vec();
    if !(options.vditor_wysiwyg
        || options.vditor_ir
        || options.vditor_sv
        || options.protyle_wysiwyg)
    {
        for b in label.iter_mut() {
            if lex::ITEM_NEWLINE == *b {
                *b = lex::ITEM_SPACE;
            }
        }
        let mut j = 0;
        while j < label.len() {
            if lex::ITEM_SPACE == label[j] && j < label.len() - 1 && lex::ITEM_SPACE == label[j + 1]
            {
                label.remove(j);
            }
            // Go advances after removal too: three spaces become two, not one.
            j += 1;
        }
    }
    Some((n, remains, label))
}

pub(super) fn parse_link_title(tokens: &[u8]) -> (bool, Vec<u8>, Vec<u8>, Option<Vec<u8>>) {
    if tokens.is_empty() {
        return (true, Vec::new(), tokens.to_vec(), None);
    }
    if lex::ITEM_OPEN_BRACKET == tokens[0] {
        return (true, Vec::new(), tokens.to_vec(), None);
    }

    let (valid, passed, remains, title) =
        parse_link_title_match(lex::ITEM_DOUBLEQUOTE, lex::ITEM_DOUBLEQUOTE, tokens);
    if valid {
        return (
            true,
            passed,
            remains,
            title.map(|t| html::unescape_bytes(&t)),
        );
    }
    let (valid, passed, remains, title) =
        parse_link_title_match(lex::ITEM_SINGLEQUOTE, lex::ITEM_SINGLEQUOTE, tokens);
    if valid {
        return (
            true,
            passed,
            remains,
            title.map(|t| html::unescape_bytes(&t)),
        );
    }
    let (valid, passed, remains, title) =
        parse_link_title_match(lex::ITEM_OPEN_PAREN, lex::ITEM_CLOSE_PAREN, tokens);
    (
        valid,
        passed,
        remains,
        title.map(|t| html::unescape_bytes(&t)),
    )
}

pub(super) fn parse_link_title_match(
    opener: u8,
    closer: u8,
    tokens: &[u8],
) -> (bool, Vec<u8>, Vec<u8>, Option<Vec<u8>>) {
    let length = tokens.len();
    if 2 > length || opener != tokens[0] {
        return (false, Vec::new(), tokens.to_vec(), None);
    }

    let mut passed = Vec::new();
    let mut title = Vec::new();
    let mut closed = false;
    let mut i = 1;
    while i < length {
        let size = utf8_len(tokens, i);
        passed.extend_from_slice(&tokens[i..i + size]);
        title.extend_from_slice(&tokens[i..i + size]);
        if closer == tokens[i] && !lex::is_backslash_escape_punct(tokens, i) {
            closed = true;
            title.truncate(title.len() - 1);
            break;
        }
        i += size;
    }

    if !closed {
        return (false, Vec::new(), tokens.to_vec(), None);
    }

    (true, passed, tokens[i + 1..].to_vec(), Some(title))
}

/// 返回 tokens[i..] 首字符的 UTF-8 字节长度（非法字节按 1 处理）。
fn utf8_len(tokens: &[u8], i: usize) -> usize {
    match std::str::from_utf8(&tokens[i..]) {
        Ok(s) => s.chars().next().map(|c| c.len_utf8()).unwrap_or(1),
        Err(e) => {
            if 0 < e.valid_up_to() {
                std::str::from_utf8(&tokens[i..i + e.valid_up_to()])
                    .unwrap()
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1)
            } else {
                1
            }
        }
    }
}

#[cfg(test)]
mod go_link_label_tests {
    use super::parse_link_label;

    /// Go parse/performance_regression_test.go: TestLinkLabelBoundaries.
    #[test]
    fn go_link_label_boundaries() {
        let limit = "a".repeat(999);
        for (input, want, remains, consumed) in [
            (
                "[中文] trailing".to_owned(),
                "中文".to_owned(),
                " trailing",
                8,
            ),
            (r"[a\]b] tail".to_owned(), r"a\]b".to_owned(), " tail", 6),
            ("[a\n  b] tail".to_owned(), "a  b".to_owned(), " tail", 7),
            (format!("[{limit}] tail"), limit, " tail", 1001),
        ] {
            let (n, rest, label) =
                parse_link_label(input.as_bytes(), &crate::options::ParseOptions::new())
                    .expect(&input);
            assert_eq!(n, consumed, "{input:?}");
            assert_eq!(label, want.as_bytes(), "{input:?}");
            assert_eq!(rest, remains.as_bytes(), "{input:?}");
        }
        for input in [
            "[a[b]".to_owned(),
            "[unclosed".to_owned(),
            format!("[{}]", "a".repeat(1000)),
        ] {
            assert!(
                parse_link_label(input.as_bytes(), &crate::options::ParseOptions::new()).is_none(),
                "{input:?}"
            );
        }
    }

    #[test]
    fn link_label_editor_modes_preserve_internal_whitespace() {
        for mode in 0..4 {
            let mut options = crate::options::ParseOptions::new();
            options.vditor_wysiwyg = mode == 0;
            options.vditor_ir = mode == 1;
            options.vditor_sv = mode == 2;
            options.protyle_wysiwyg = mode == 3;
            assert_eq!(
                parse_link_label(b"[a\n  b] tail", &options).unwrap(),
                (7, b" tail".to_vec(), b"a\n  b".to_vec())
            );
        }
    }

    #[test]
    fn link_label_empty_whitespace_and_byte_length_boundaries() {
        for input in ["", "[", "plain"] {
            assert!(
                parse_link_label(input.as_bytes(), &crate::options::ParseOptions::new()).is_none(),
                "{input:?}"
            );
        }
        // Go TrimWhitespace preserves non-nil empty slices. Successful empty
        // labels carry their consumed length, which collapsed references need.
        for input in ["[]", "[ ]", "[\n\t ]"] {
            assert_eq!(
                parse_link_label(input.as_bytes(), &crate::options::ParseOptions::new()),
                Some((input.len(), Vec::new(), Vec::new())),
                "{input:?}"
            );
        }
        let label = "中".repeat(333);
        assert_eq!(
            parse_link_label(
                format!("[{label}]").as_bytes(),
                &crate::options::ParseOptions::new()
            )
            .unwrap()
            .2,
            label.as_bytes()
        );
        assert!(parse_link_label(
            format!("[{}]", "中".repeat(334)).as_bytes(),
            &crate::options::ParseOptions::new()
        )
        .is_none());
    }
}
