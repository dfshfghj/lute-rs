// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 标题 ID、块引用、文件注解引用等行级扩展解析。

use crate::ast::{walk_raw as walk, NodeType, RawNode as Node, WalkStatus};
use crate::editor;
use crate::html;
use crate::lex;

use crate::parse::link::parse_link_title;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseHeadingID。
    pub(super) fn parse_heading_id(
        &mut self,
        block: Node,
        ctx: &mut InlineContext,
    ) -> Option<Node> {
        if !self.options.heading_id || NodeType::NodeHeading != block.typ() || 3 > ctx.tokens_len {
            ctx.pos += 1;
            return Some(self.new_text(b"{"));
        }

        let start_pos = ctx.pos;
        let mut content = ctx.tokens[start_pos..].to_vec();
        let curly_braces_end = find_sub(&content, b"}").map(|v| v as isize).unwrap_or(-1);
        if 2 > curly_braces_end {
            ctx.pos += 1;
            return Some(self.new_text(b"{"));
        }

        let curly_braces_start = find_sub(&content, b"{").map(|v| v as isize).unwrap_or(-1);
        if 0 > curly_braces_start {
            return None;
        }

        let length = content.len();
        if length - 1 != curly_braces_end as usize {
            let mut suffix = b"}".to_vec();
            suffix.extend_from_slice(editor::CARET.as_bytes());
            if !content.ends_with(&suffix) && content.ends_with(editor::CARET.as_bytes()) {
                // # foo {id}b‸
                ctx.pos += 1;
                return Some(self.new_text(b"{"));
            }
        }

        if self.options.vditor_wysiwyg {
            content = replace_bytes(&content, editor::CARET.as_bytes(), b"");
        }
        let id = content[curly_braces_start as usize + 1..curly_braces_end as usize].to_vec();
        ctx.pos += curly_braces_end as usize + 1;
        if let Some(last) = block.last_child() {
            let tokens = last.tokens_vec();
            let mut end = tokens.len();
            while 0 < end && b' ' == tokens[end - 1] {
                end -= 1;
            }
            last.set_tokens(tokens[..end].to_vec());
        }
        let n = self.new_node_raw(NodeType::NodeHeadingID);
        n.set_tokens(id);
        Some(n)
    }

    /// 对应 Go 版 Tree.parseBlockRef。
    pub(super) fn parse_block_ref(&mut self, ctx: &mut InlineContext) -> Node {
        if !self.options.block_ref {
            ctx.pos += 1;
            return self.new_text(b"(");
        }

        let tokens = &ctx.tokens[ctx.pos..];
        if 5 > tokens.len() || b'(' != tokens[0] || b'(' != tokens[1] {
            ctx.pos += 1;
            return self.new_text(b"(");
        }

        let save_pos = ctx.pos;
        ctx.pos += 2;
        let mut id: Vec<u8> = Vec::new();
        let mut text: Vec<u8> = Vec::new();
        let mut subtype = String::new();
        let mut matched = false;
        loop {
            let (ok, passed, remains) = lex::spnl(&ctx.tokens[ctx.pos..]);
            if !ok {
                break;
            }
            ctx.pos += passed.len();

            let (passed, remains, id2) = parse_block_ref_id(remains);
            id = id2;
            if 1 > passed.len() {
                break;
            }
            ctx.pos += passed.len();
            matched = b')' == passed[passed.len() - 1]
                && 2 <= passed.len()
                && b')' == passed[passed.len() - 2];
            if matched {
                break;
            }
            if remains.is_empty() || !lex::is_whitespace(remains[0]) {
                break;
            }
            // 跟空格的话后续尝试锚文本解析
            let (ok, passed, remains) = lex::spnl(&remains);
            if !ok {
                break;
            }
            ctx.pos += passed.len() + 1;
            matched = 2 <= remains.len() && b')' == remains[0] && b')' == remains[1];
            if matched {
                ctx.pos += 1;
                break;
            }
            let (valid, passed, remains, text2, subtype2) = parse_block_ref_text(remains);
            if !valid {
                break;
            }
            text = text2.unwrap_or_default();
            subtype = subtype2;
            ctx.pos += passed.len();
            let (ok, passed, remains) = lex::spnl(&remains);
            ctx.pos += passed.len();
            matched = ok && 1 < remains.len();
            if matched {
                matched = b')' == remains[0] && b')' == remains[1];
                ctx.pos += 2;
            }
            break;
        }
        if !matched {
            ctx.pos = save_pos + 1;
            return self.new_text(b"(");
        }

        let ret = self.new_node_raw(NodeType::NodeBlockRef);
        ret.append_child(self.new_node_raw(NodeType::NodeOpenParen));
        ret.append_child(self.new_node_raw(NodeType::NodeOpenParen));
        let id_node = self.new_node_raw(NodeType::NodeBlockRefID);
        id_node.set_tokens(id);
        ret.append_child(id_node);
        if 0 < text.len() {
            ret.append_child(self.new_node_raw(NodeType::NodeBlockRefSpace));
            let text_node = self.new_node_raw(if "d" == subtype {
                NodeType::NodeBlockRefDynamicText
            } else {
                NodeType::NodeBlockRefText
            });
            text_node.set_tokens(text);
            ret.append_child(text_node);
        }
        ret.append_child(self.new_node_raw(NodeType::NodeCloseParen));
        ret.append_child(self.new_node_raw(NodeType::NodeCloseParen));
        ret
    }

    /// 对应 Go 版 Tree.parseFileAnnotationRef。
    pub(super) fn parse_file_annotation_ref(&mut self, ctx: &mut InlineContext) -> Option<Node> {
        if !self.options.file_annotation_ref {
            return None;
        }

        let tokens = &ctx.tokens[ctx.pos..];
        if 2 > tokens.len() || b'<' != tokens[0] || b'<' != tokens[1] {
            return None;
        }

        let mut id: Vec<u8> = Vec::new();
        let mut text: Vec<u8> = Vec::new();
        let save_pos = ctx.pos;
        ctx.pos += 2;
        let mut matched = false;
        loop {
            let (ok, passed, remains) = lex::spnl(&ctx.tokens[ctx.pos..]);
            if !ok {
                break;
            }
            ctx.pos += passed.len();

            let (passed, remains, id2) = parse_file_annotation_ref_id(remains);
            id = id2;
            if passed.is_empty() {
                ctx.pos = save_pos;
                break;
            }
            ctx.pos += passed.len();
            matched = 2 <= passed.len()
                && b'>' == passed[passed.len() - 1]
                && b'>' == passed[passed.len() - 2];
            if matched {
                break;
            }
            if remains.is_empty() || !lex::is_whitespace(remains[0]) {
                break;
            }
            // 跟空格的话后续尝试 title 解析
            let (ok, passed, remains) = lex::spnl(&remains);
            if !ok {
                break;
            }
            ctx.pos += passed.len() + 1;
            matched = 2 <= remains.len() && b'>' == remains[0] && b'>' == remains[1];
            if matched {
                ctx.pos += 1;
                break;
            }
            let (valid, passed, remains, title) = parse_link_title(remains);
            if !valid {
                break;
            }
            text = title.unwrap_or_default();
            ctx.pos += passed.len();
            let (ok, passed, remains) = lex::spnl(&remains);
            ctx.pos += passed.len();
            matched = ok && 1 < remains.len();
            if matched {
                matched = b'>' == remains[0] && b'>' == remains[1];
                ctx.pos += 2;
            }
            break;
        }
        if !matched {
            ctx.pos = save_pos;
            return None;
        }

        let ret = self.new_node_raw(NodeType::NodeFileAnnotationRef);
        ret.append_child(self.new_node_raw(NodeType::NodeLess));
        ret.append_child(self.new_node_raw(NodeType::NodeLess));
        let id_node = self.new_node_raw(NodeType::NodeFileAnnotationRefID);
        id_node.set_tokens(id);
        ret.append_child(id_node);
        if !text.is_empty() {
            ret.append_child(self.new_node_raw(NodeType::NodeFileAnnotationRefSpace));
            let text_node = self.new_node_raw(NodeType::NodeFileAnnotationRefText);
            text_node.set_tokens(text);
            ret.append_child(text_node);
        }
        ret.append_child(self.new_node_raw(NodeType::NodeGreater));
        ret.append_child(self.new_node_raw(NodeType::NodeGreater));
        Some(ret)
    }

    /// 对应 Go 版 Tree.parseKramdownSpanIAL。
    pub(super) fn parse_kramdown_span_ial(&mut self) {
        walk(self.tree.root_raw(), &mut |n, entering| {
            if !entering {
                return WalkStatus::WalkContinue;
            }
            if let Some(previous) = n.previous() {
                if previous.typ() == NodeType::NodeKramdownSpanIAL && n.typ() == NodeType::NodeText
                {
                    if let Some(target) = previous.previous() {
                        let (tokens, end) = span_ial_text(n);
                        if let Some((pos, ial)) = parse_span_ial(&tokens) {
                            if ial.iter().all(|(key, value)| {
                                target.ial_attr(key) == html::unescape_attr_val(value)
                            }) {
                                merge_span_ial_text(n, end);
                                n.set_tokens(tokens[pos + 1..].to_vec());
                            }
                        }
                        return WalkStatus::WalkContinue;
                    }
                }
            }
            if !matches!(
                n.typ(),
                NodeType::NodeEmphasis
                    | NodeType::NodeStrong
                    | NodeType::NodeCodeSpan
                    | NodeType::NodeStrikethrough
                    | NodeType::NodeTag
                    | NodeType::NodeMark
                    | NodeType::NodeInlineMath
                    | NodeType::NodeImage
                    | NodeType::NodeTextMark
            ) {
                return WalkStatus::WalkContinue;
            }
            let Some(next) = n.next().filter(|next| next.typ() == NodeType::NodeText) else {
                return WalkStatus::WalkContinue;
            };
            let (tokens, end) = span_ial_text(next);
            if let Some((pos, ial)) = parse_span_ial(&tokens) {
                n.data_mut().kramdown_ial = ial;
                merge_span_ial_text(next, end);
                next.set_tokens(tokens[pos + 1..].to_vec());
                if next.tokens().is_empty() {
                    next.unlink();
                }
                let span_ial = self.new_node_raw(NodeType::NodeKramdownSpanIAL);
                span_ial.set_tokens(tokens[..pos + 1].to_vec());
                n.insert_after(span_ial);
            }
            WalkStatus::WalkContinue
        });
    }
}

/// Preserve entity spelling while collecting attribute source split by inline parsing.
fn span_ial_text(start: Node) -> (Vec<u8>, Node) {
    let mut tokens = start.tokens_vec();
    if !tokens.starts_with(b"{:") || tokens.contains(&b'}') {
        return (tokens, start);
    }
    let mut next = start.next();
    while let Some(n) = next {
        match n.typ() {
            NodeType::NodeText => tokens.extend_from_slice(n.tokens()),
            NodeType::NodeHTMLEntity => tokens.extend_from_slice(&n.data().html_entity_tokens),
            _ => return (start.tokens_vec(), start),
        }
        if n.typ() == NodeType::NodeText && n.tokens().contains(&b'}') {
            return (tokens, n);
        }
        next = n.next();
    }
    (start.tokens_vec(), start)
}

fn merge_span_ial_text(start: Node, end: Node) {
    if start == end {
        return;
    }
    while let Some(next) = start.next() {
        next.unlink();
        if next == end {
            break;
        }
    }
}

/// Corresponds to Context.parseKramdownSpanIAL, including caret removal.
pub(crate) fn parse_span_ial(tokens: &[u8]) -> Option<(usize, Vec<(String, String)>)> {
    let pos = tokens.iter().position(|&b| b == b'}')?;
    if !tokens.starts_with(b"{:") || pos < 5 {
        return None;
    }
    let mut tokens = tokens[2..pos].to_vec();
    let mut ial = Vec::new();
    loop {
        let (valid, remains, attr, name, value) = super::html::tag_attr(&tokens);
        if !valid || attr.is_empty() {
            break;
        }
        ial.push((
            String::from_utf8_lossy(&name).replace(editor::CARET, ""),
            String::from_utf8_lossy(&value).replace(editor::CARET, ""),
        ));
        tokens = remains;
    }
    if ial.is_empty() {
        None
    } else {
        Some((pos, ial))
    }
}

/// 对应 Go 版 Context.parseBlockRefID。
fn parse_block_ref_id(tokens: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let length = tokens.len();
    if 1 > length {
        return (Vec::new(), tokens.to_vec(), Vec::new());
    }

    let caret = editor::CARET.as_bytes();
    let mut i = 0;
    while i < length {
        let token = tokens[i];
        if caret.contains(&token) {
            i += 1;
            continue;
        }

        if lex::is_whitespace(token) || b')' == token || !lex::is_ascii_letter_num_hyphen(tokens[i])
        {
            break;
        }
        i += 1;
    }
    let mut remains = tokens[i..].to_vec();
    let id = tokens[..i].to_vec();
    if 2 > remains.len() || !is_node_id_pattern(&id) {
        return (Vec::new(), remains, id);
    }
    let mut passed = id.clone();
    if remains.starts_with(caret) {
        passed.extend_from_slice(caret);
        remains = remains[caret.len()..].to_vec();
    }
    let closed = b')' == remains[0] && 2 <= remains.len() && b')' == remains[1];
    if closed {
        passed.extend_from_slice(b"))");
        return (passed, remains, id);
    }

    if !lex::is_whitespace(remains[0]) {
        return (Vec::new(), remains, id);
    }
    (passed, remains, id)
}

/// 对应 Go 版 Context.parseBlockRefText。
fn parse_block_ref_text(tokens: &[u8]) -> (bool, Vec<u8>, Vec<u8>, Option<Vec<u8>>, String) {
    if 1 > tokens.len() {
        return (true, Vec::new(), tokens.to_vec(), None, String::new());
    }
    if b'[' == tokens[0] {
        return (true, Vec::new(), tokens.to_vec(), None, String::new());
    }

    let (valid, passed, remains, title) = parse_link_title_match(b'"', b'"', tokens);
    let mut subtype = "s".to_string();
    let (valid, passed, remains, title) = if !valid {
        let (v, p, r, t) = parse_link_title_match(b'\'', b'\'', tokens);
        subtype = "d".to_string();
        (v, p, r, t)
    } else {
        (valid, passed, remains, title)
    };
    let title = title.map(|t| html::unescape_bytes(&t));
    (valid, passed, remains, title, subtype)
}

/// 对应 Go 版 Context.parseLinkTitleMatch。
fn parse_link_title_match(
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

/// 对应 Go 版 ast.IsNodeIDPattern。
fn is_node_id_pattern(s: &[u8]) -> bool {
    if 22 != s.len() {
        return false;
    }
    if 1 != s.iter().filter(|&&b| b == b'-').count() {
        return false;
    }
    let dash = s.iter().position(|&b| b == b'-').unwrap();
    let id_part = &s[..dash];
    let rand_part = &s[dash + 1..];
    if 14 != id_part.len() || 7 != rand_part.len() {
        return false;
    }
    if !id_part.iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    rand_part
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
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

fn utf8_len(tokens: &[u8], i: usize) -> usize {
    match std::str::from_utf8(&tokens[i..]) {
        Ok(s) => s.chars().next().map(|c| c.len_utf8()).unwrap_or(1),
        Err(_) => 1,
    }
}

/// 对应 Go 版 Context.parseFileAnnotationRefID。
fn parse_file_annotation_ref_id(tokens: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let length = tokens.len();
    if 1 > length {
        return (Vec::new(), tokens.to_vec(), Vec::new());
    }

    let caret = editor::CARET.as_bytes();
    let mut i = 0;
    while i < length {
        let token = tokens[i];
        if caret.contains(&token) {
            i += 1;
            continue;
        }
        // 遇到嵌套起始符立即拒绝，避免连续无效输入反复扫描剩余文本。
        if b'<' == token {
            return (Vec::new(), tokens.to_vec(), Vec::new());
        }
        if tokens[i..].starts_with(b" \"") {
            break;
        }
        if b'>' == token {
            break;
        }
        i += 1;
    }

    let mut remains = tokens[i..].to_vec();
    let id_part = &tokens[..i];
    if id_part
        .iter()
        .any(|b| matches!(b, b'<' | b'>' | b'"' | b'\r' | b'\n' | b'\\'))
    {
        return (Vec::new(), remains, Vec::new());
    }
    if !id_part.starts_with(b"assets/") {
        return (Vec::new(), remains, Vec::new());
    }
    let mut id_part = &id_part[b"assets/".len()..];
    // 查询参数和片段属于资源定位信息，不参与文件路径和标注 ID 校验。
    if let Some(query) = id_part.iter().position(|&b| b'?' == b || b'#' == b) {
        id_part = &id_part[..query];
    }
    let separator = match id_part.iter().rposition(|&b| b'/' == b) {
        Some(sep) if 0 < sep => sep,
        _ => return (Vec::new(), remains, Vec::new()),
    };
    let file_part = &id_part[..separator];
    let last_slash = file_part
        .iter()
        .rposition(|&b| b'/' == b)
        .map(|v| v as isize)
        .unwrap_or(-1);
    if !file_part.to_ascii_lowercase().ends_with(b".pdf")
        || 4 >= file_part.len() as isize - last_slash - 1
    {
        return (Vec::new(), remains, Vec::new());
    }
    for part in file_part.split(|&b| b == b'/') {
        if part.is_empty() || b"." == part || b".." == part {
            return (Vec::new(), remains, Vec::new());
        }
    }
    let annotation_id_part = &id_part[separator + 1..];
    if !is_node_id_pattern(annotation_id_part) {
        return (Vec::new(), remains, Vec::new());
    }

    let id = tokens[..i].to_vec();
    if 2 > remains.len() {
        return (Vec::new(), remains, id);
    }
    let mut passed = id.clone();
    if remains.starts_with(caret) {
        passed.extend_from_slice(caret);
        remains = remains[caret.len()..].to_vec();
    }
    let closed = 2 <= remains.len() && b'>' == remains[0] && b'>' == remains[1];
    if closed {
        passed.extend_from_slice(b">>");
        return (passed, remains, id);
    }
    if !lex::is_whitespace(remains[0]) {
        return (Vec::new(), remains, id);
    }
    (passed, remains, id)
}

#[cfg(test)]
mod go_file_annotation_tests {
    use super::parse_file_annotation_ref_id;

    /// Go parse/file_annotation_ref_test.go:
    /// TestFileAnnotationRefRejectsNestedOpenersWithoutConsumingInput.
    #[test]
    fn go_file_annotation_rejects_nested_openers_without_consuming_input() {
        let tokens = vec![b'<'; 65536];
        let (passed, remains, id) = parse_file_annotation_ref_id(&tokens);
        assert!(passed.is_empty());
        assert!(id.is_empty());
        assert_eq!(remains.len(), tokens.len());
        assert_eq!(remains, tokens);
    }

    /// Additional boundaries of the same Go parseFileAnnotationRefID state machine.
    #[test]
    fn annotation_empty_input_and_nested_openers_preserve_bytes() {
        for input in [
            b"".as_slice(),
            b"<",
            b"assets/a.pdf/<",
            b"assets/a.pdf/20260101000000-abcdefg<tail",
        ] {
            let (passed, remains, id) = parse_file_annotation_ref_id(input);
            assert!(passed.is_empty(), "{input:?}");
            assert!(id.is_empty(), "{input:?}");
            assert_eq!(remains, input, "{input:?}");
        }
    }

    #[test]
    fn annotation_valid_id_preserves_query_and_fragment_in_tokens() {
        for suffix in ["", "?page=1", "#page=2"] {
            let id = format!("assets/folder/a.PDF/20260101000000-abcdefg{suffix}");
            let input = format!("{id}>>tail");
            let (passed, remains, parsed_id) = parse_file_annotation_ref_id(input.as_bytes());
            assert_eq!(passed, format!("{id}>>").as_bytes());
            assert_eq!(parsed_id, id.as_bytes());
            assert_eq!(remains, b">>tail");
        }
    }

    #[test]
    fn annotation_rejects_invalid_resource_paths_and_ids() {
        for path in [
            "assets/.pdf",
            "assets/a.txt",
            "assets/../a.pdf",
            "assets/./a.pdf",
            "assets//a.pdf",
            "other/a.pdf",
        ] {
            let input = format!("{path}/20260101000000-abcdefg>>");
            let (passed, _, id) = parse_file_annotation_ref_id(input.as_bytes());
            assert!(passed.is_empty(), "{input}");
            assert!(id.is_empty(), "{input}");
        }
        for id in [
            "",
            "20260101000000-abcdef",
            "20260101000000-abcdefgh",
            "not-an-id",
        ] {
            let input = format!("assets/a.pdf/{id}>>");
            let (passed, _, parsed_id) = parse_file_annotation_ref_id(input.as_bytes());
            assert!(passed.is_empty(), "{input}");
            assert!(parsed_id.is_empty(), "{input}");
        }
    }
}
