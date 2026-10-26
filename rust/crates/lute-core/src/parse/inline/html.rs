// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 行级 HTML 解析。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;

use super::{InlineContext, InlineParser};

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.parseInlineHTML。
    pub(super) fn parse_inline_html(&mut self, ctx: &mut InlineContext) -> Node {
        let editor = self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg);
        let caret = crate::editor::CARET.as_bytes();
        let caret_in_tag = editor
            && ctx
                .tokens
                .windows(caret.len())
                .position(|w| w == caret)
                .is_some_and(|i| i > ctx.pos);
        let caret_left_space =
            caret_in_tag && ctx.tokens.windows(" ‸".len()).any(|w| w == " ‸".as_bytes());
        let mut tokens = ctx.tokens.clone();
        if caret_in_tag {
            tokens = replace_bytes(&tokens, caret, crate::editor::CARET_REPLACEMENT.as_bytes());
            tokens = replace_bytes(&tokens, b"\"caretreplacement", b"\" caretreplacement");
        }
        let start_pos = ctx.pos;
        let mut ret = self.new_text(&[tokens[ctx.pos]]);
        if 3 > ctx.tokens_len || ctx.tokens_len <= start_pos + 1 {
            ctx.pos += 1;
            return ret;
        }

        let mut tags: Vec<u8> = vec![tokens[start_pos]];
        let remains: Vec<u8>;
        if b'/' == tokens[start_pos + 1] && 1 < ctx.tokens_len - (start_pos + 1) {
            // 闭合标签
            tags.push(b'/');
            let (r, tag_name) = parse_tag_name(&tokens[ctx.pos + 2..]);
            if tag_name.is_empty() {
                ctx.pos += 1;
                return ret;
            }
            tags.extend_from_slice(&tag_name);
            remains = r;
        } else {
            let (r, tag_name) = parse_tag_name(&tokens[ctx.pos + 1..]);
            if !tag_name.is_empty() {
                tags.extend_from_slice(&tag_name);
                let mut tokens = r;
                loop {
                    let (valid, r, attr, _, _) = tag_attr(&tokens);
                    if !valid {
                        ctx.pos += 1;
                        return ret;
                    }
                    tokens = r;
                    tags.extend_from_slice(&attr);
                    if attr.is_empty() {
                        break;
                    }
                }
                remains = tokens;
            } else if let Some((_, comment)) = parse_html_comment(&tokens[ctx.pos + 1..]) {
                tags.extend_from_slice(&comment);
                tags = restore_caret_tokens(
                    tags,
                    editor && caret_in_tag || (crate::PROTYLE && self.options.protyle_wysiwyg),
                    caret_left_space,
                );
                ctx.pos += tags.len();
                let n = self.new_node_raw(NodeType::NodeInlineHTML);
                n.set_tokens(tags);
                return n;
            } else if let Some((_, ins)) = parse_processing_instruction(&tokens[ctx.pos + 1..]) {
                tags.extend_from_slice(&ins);
                tags = restore_caret_tokens(
                    tags,
                    editor && caret_in_tag || (crate::PROTYLE && self.options.protyle_wysiwyg),
                    caret_left_space,
                );
                ctx.pos += tags.len();
                let n = self.new_node_raw(NodeType::NodeInlineHTML);
                n.set_tokens(tags);
                return n;
            } else if let Some((_, decl)) = parse_declaration(&tokens[ctx.pos + 1..]) {
                tags.extend_from_slice(&decl);
                tags = restore_caret_tokens(
                    tags,
                    editor && caret_in_tag || (crate::PROTYLE && self.options.protyle_wysiwyg),
                    caret_left_space,
                );
                ctx.pos += tags.len();
                let n = self.new_node_raw(NodeType::NodeInlineHTML);
                n.set_tokens(tags);
                return n;
            } else if let Some((_, cdata)) = parse_cdata(&tokens[ctx.pos + 1..]) {
                tags.extend_from_slice(&cdata);
                tags = restore_caret_tokens(
                    tags,
                    editor && caret_in_tag || (crate::PROTYLE && self.options.protyle_wysiwyg),
                    caret_left_space,
                );
                ctx.pos += tags.len();
                let n = self.new_node_raw(NodeType::NodeInlineHTML);
                n.set_tokens(tags);
                return n;
            } else {
                ctx.pos += 1;
                return ret;
            }
        }

        let (whitespaces, t) = lex::trim_left(&remains);
        let tokens = t;
        let length = tokens.len();
        if 1 > length {
            ctx.pos = start_pos + 1;
            return ret;
        }

        if b'>' == tokens[0] || (1 < length && b'/' == tokens[0] && b'>' == tokens[1]) {
            tags.extend_from_slice(&whitespaces);
            tags.push(tokens[0]);
            if b'/' == tokens[0] {
                tags.push(tokens[1]);
            }
            tags = restore_caret_tokens(
                tags,
                editor && caret_in_tag || (crate::PROTYLE && self.options.protyle_wysiwyg),
                caret_left_space,
            );
            ctx.pos += tags.len();
            if crate::PROTYLE && self.options.protyle_wysiwyg {
                if [b"<br />".as_slice(), b"<br/>", b"<br>"]
                    .iter()
                    .any(|tag| tags.eq_ignore_ascii_case(tag))
                {
                    return self.new_node_raw(NodeType::NodeBr);
                }
                if tags.starts_with(b"<span data-type=") {
                    return self.process_span_tag(tags, "<span data-type=", "</span>", ctx);
                }
                for tag in [
                    "kbd", "u", "sup", "sub", "mark", "s", "del", "strike", "em", "i", "strong",
                    "b",
                ] {
                    let opener = format!("<{tag}>");
                    if tags.eq_ignore_ascii_case(opener.as_bytes()) {
                        return self.process_span_tag(tags, &opener, &format!("</{tag}>"), ctx);
                    }
                }
                if tags.to_ascii_lowercase().starts_with(b"<img ") {
                    let dom = crate::dom::parse(&String::from_utf8_lossy(&tags));
                    if dom.nodes[dom.root].children.len() == 1 {
                        let element = dom.nodes[dom.root].children[0];
                        if dom.nodes[element].data == "img" {
                            let image = self.new_node_raw(NodeType::NodeImage);
                            for typ in [NodeType::NodeBang, NodeType::NodeOpenBracket] {
                                image.append_child(self.new_node_raw(typ));
                            }
                            let alt = dom.attr(element, "alt");
                            if !alt.is_empty() {
                                let text = self.new_node_raw(NodeType::NodeLinkText);
                                text.set_tokens(alt.as_bytes().to_vec());
                                image.append_child(text);
                            }
                            image.append_child(self.new_node_raw(NodeType::NodeCloseBracket));
                            image.append_child(self.new_node_raw(NodeType::NodeOpenParen));
                            for (attr, typ) in [
                                ("src", NodeType::NodeLinkDest),
                                ("title", NodeType::NodeLinkTitle),
                            ] {
                                let value = dom.attr(element, attr);
                                if !value.is_empty() {
                                    let child = self.new_node_raw(typ);
                                    child.set_tokens(value.as_bytes().to_vec());
                                    image.append_child(child);
                                }
                            }
                            image.append_child(self.new_node_raw(NodeType::NodeCloseParen));
                            for dimension in ["width", "height"] {
                                let value = dom.attr(element, dimension);
                                let value = value.trim();
                                if !value.is_empty() {
                                    let unit = if value.bytes().all(|b| b.is_ascii_digit()) {
                                        "px"
                                    } else {
                                        ""
                                    };
                                    let style = format!("{dimension}: {value}{unit};");
                                    image.set_ial_attr("style", &style);
                                    let ial = self.new_node_raw(NodeType::NodeKramdownSpanIAL);
                                    ial.set_tokens(crate::parse::ial::ial_to_tokens(
                                        &image.ial_raw(),
                                    ));
                                    image.insert_after(ial);
                                    break;
                                }
                            }
                            return image;
                        }
                    }
                }
            }
            ret = self.new_node_raw(NodeType::NodeInlineHTML);
            ret.set_tokens(tags);
            return ret;
        }

        ctx.pos = start_pos + 1;
        ret
    }
    /// Go processSpanTag consumes one non-nested matching element and delegates
    /// its metadata to the same SetTextMarkNode path used by BlockDOM parsing.
    fn process_span_tag(
        &mut self,
        tags: Vec<u8>,
        start_tag: &str,
        end_tag: &str,
        ctx: &mut InlineContext,
    ) -> Node {
        let fallback = self.new_node_raw(NodeType::NodeInlineHTML);
        fallback.set_tokens(tags.clone());
        let remains = &ctx.tokens[ctx.pos..];
        let Some(end) = remains
            .windows(end_tag.len())
            .position(|part| part == end_tag.as_bytes())
        else {
            return fallback;
        };
        if remains
            .windows(start_tag.len())
            .position(|part| part == start_tag.as_bytes())
            .is_some_and(|inner| inner < end)
        {
            return fallback;
        }
        let native_tag = start_tag != "<span data-type=";
        if native_tag && !self.options.html_tag2_text_mark {
            return fallback;
        }
        let mut fragment = tags;
        fragment.extend_from_slice(&remains[..end + end_tag.len()]);
        let dom = crate::dom::parse(&String::from_utf8_lossy(&fragment));
        if dom.nodes[dom.root].children.len() != 1 {
            return fallback;
        }
        let element = dom.nodes[dom.root].children[0];
        let mut typ = if native_tag {
            match dom.nodes[element].data.as_str() {
                "b" => "strong".to_owned(),
                "i" => "em".to_owned(),
                "del" | "strike" => "s".to_owned(),
                tag => tag.to_owned(),
            }
        } else {
            dom.attr(element, "data-type")
        };
        if let Some(&first) = dom.nodes[element].children.first() {
            let child = &dom.nodes[first];
            if child.typ == crate::dom::DomType::Element
                && child.data != typ
                && matches!(child.data.as_str(), "sup" | "sub" | "em" | "strong")
            {
                typ.push(' ');
                typ.push_str(&child.data);
            }
        }
        let node = self.new_node_raw(NodeType::NodeTextMark);
        node.data_mut().text_mark_type = typ;
        if crate::PROTYLE {
            crate::blockdom::set_text_mark_node(self.tree, node, &dom, element, self.options);
        }
        ctx.pos += end + end_tag.len();
        node
    }
}

/// 对应 Go 版 Tree.parseCDATA。
fn parse_cdata(tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if 8 > tokens.len() {
        return None;
    }
    if b'!' != tokens[0] || b'[' != tokens[1] || &tokens[2..7] != b"CDATA" || b'[' != tokens[7] {
        return None;
    }

    let mut content: Vec<u8> = tokens[..7].to_vec();
    let rest = &tokens[7..];
    let length = rest.len();
    let mut i = 0;
    while i < length {
        let token = rest[i];
        content.push(token);
        if i + 2 < length && b']' == token && b']' == rest[i + 1] && b'>' == rest[i + 2] {
            break;
        }
        i += 1;
    }
    let rest = &rest[i..];
    if 3 > rest.len() || b']' != rest[0] || b']' != rest[1] || b'>' != rest[2] {
        return None;
    }
    content.push(rest[1]);
    content.push(rest[2]);
    Some((rest[3..].to_vec(), content))
}

/// 对应 Go 版 Tree.parseDeclaration。
fn parse_declaration(tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if 2 > tokens.len() || b'!' != tokens[0] {
        return None;
    }

    for &token in &tokens[1..] {
        if lex::is_whitespace(token) {
            break;
        }
        if !token.is_ascii_uppercase() {
            return None;
        }
    }

    let mut content: Vec<u8> = vec![tokens[0], tokens[1]];
    let rest = &tokens[2..];
    let length = rest.len();
    let mut i = 0;
    while i < length {
        let token = rest[i];
        content.push(token);
        if b'>' == token {
            break;
        }
        i += 1;
    }
    let rest = &rest[i..];
    if 1 > rest.len() || b'>' != rest[0] {
        return None;
    }
    Some((rest[1..].to_vec(), content))
}

/// 对应 Go 版 Tree.parseProcessingInstruction。
fn parse_processing_instruction(tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if tokens.is_empty() || b'?' != tokens[0] {
        return None;
    }

    let mut content: Vec<u8> = vec![tokens[0]];
    let rest = &tokens[1..];
    let length = rest.len();
    let mut i = 0;
    while i < length {
        let token = rest[i];
        content.push(token);
        if i + 1 < length && b'?' == token && b'>' == rest[i + 1] {
            break;
        }
        i += 1;
    }
    let rest = &rest[i..];
    if 1 > rest.len() || b'?' != rest[0] || b'>' != rest[1] {
        return None;
    }
    content.push(rest[1]);
    Some((rest[2..].to_vec(), content))
}

/// 对应 Go 版 Tree.parseHTMLComment。
fn parse_html_comment(tokens: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if 3 > tokens.len() || b'!' != tokens[0] || b'-' != tokens[1] || b'-' != tokens[2] {
        return None;
    }

    let length = tokens.len();
    let mut comment = Vec::new();
    let mut i = 0;
    while i < length {
        comment.push(tokens[i]);
        if i + 2 < length && b'-' == tokens[i] && b'-' == tokens[i + 1] && b'>' == tokens[i + 2] {
            break;
        }
        i += 1;
    }
    let rest = &tokens[i..];
    if 3 > rest.len() || b'-' != rest[0] || b'-' != rest[1] || b'>' != rest[2] {
        return None;
    }
    comment.push(rest[1]);
    comment.push(rest[2]);
    Some((rest[3..].to_vec(), comment))
}

/// 对应 Go 版 TagAttr。返回 (valid, remains, attr, name, val)。
pub(crate) fn tag_attr(tokens: &[u8]) -> (bool, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let original = tokens.to_vec();
    let mut whitespaces = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if !lex::is_whitespace(token) {
            break;
        }
        whitespaces.push(token);
        i += 1;
    }
    if whitespaces.is_empty() {
        return (true, original, Vec::new(), Vec::new(), Vec::new());
    }
    let tokens = &tokens[i..];

    let (tokens, attr_name) = parse_attr_name(tokens);
    if attr_name.is_empty() {
        return (true, original, Vec::new(), Vec::new(), Vec::new());
    }

    let (valid, tokens, val_spec) = parse_attr_val_spec(&tokens);
    if !valid {
        return (false, original, Vec::new(), Vec::new(), Vec::new());
    }

    let remains = tokens.clone();
    let mut attr = Vec::new();
    attr.extend_from_slice(&whitespaces);
    attr.extend_from_slice(&attr_name);
    attr.extend_from_slice(&val_spec);
    let mut name = Vec::new();
    let mut val = Vec::new();
    if !val_spec.is_empty() {
        name = attr_name;
        val = val_spec[2..val_spec.len() - 1].to_vec();
    }
    (true, remains, attr, name, val)
}

/// 对应 Go 版 parseAttrValSpec。
fn parse_attr_val_spec(tokens: &[u8]) -> (bool, Vec<u8>, Vec<u8>) {
    let mut val_spec = Vec::new();
    let mut i = 0;
    let mut token = 0u8;
    while i < tokens.len() {
        token = tokens[i];
        if !lex::is_whitespace(token) {
            break;
        }
        val_spec.push(token);
        i += 1;
    }
    if b'=' != token {
        return (true, tokens.to_vec(), Vec::new());
    }
    val_spec.push(token);
    let tokens = &tokens[i + 1..];
    if 1 > tokens.len() {
        return (false, tokens.to_vec(), Vec::new());
    }

    i = 0;
    while i < tokens.len() {
        token = tokens[i];
        if !lex::is_whitespace(token) {
            break;
        }
        val_spec.push(token);
        i += 1;
    }
    token = tokens[i];
    val_spec.push(token);
    let tokens = &tokens[i + 1..];
    let mut closed = false;
    let mut i = 0usize;
    if b'"' == token {
        while i < tokens.len() {
            token = tokens[i];
            val_spec.push(token);
            if b'"' == token {
                closed = true;
                break;
            }
            i += 1;
        }
    } else if b'\'' == token {
        while i < tokens.len() {
            token = tokens[i];
            val_spec.push(token);
            if b'\'' == token {
                closed = true;
                break;
            }
            i += 1;
        }
    } else {
        while i < tokens.len() {
            token = tokens[i];
            if b'>' == token {
                // Go decrements i before building remains so the closing angle
                // bracket is consumed by parseInlineHTML, not the attribute.
                return if closed {
                    (true, tokens[i..].to_vec(), val_spec)
                } else {
                    (false, Vec::new(), Vec::new())
                };
            }
            val_spec.push(token);
            if lex::is_whitespace(token) {
                break;
            }
            if b'"' == token
                || b'\'' == token
                || b'=' == token
                || b'<' == token
                || b'>' == token
                || b'`' == token
            {
                closed = false;
                break;
            }
            closed = true;
            i += 1;
        }
    }

    if !closed {
        return (false, Vec::new(), Vec::new());
    }

    let remains = if i + 1 <= tokens.len() {
        tokens[i + 1..].to_vec()
    } else {
        Vec::new()
    };
    (true, remains, val_spec)
}

/// 对应 Go 版 parseAttrName。
fn parse_attr_name(tokens: &[u8]) -> (Vec<u8>, Vec<u8>) {
    if tokens.is_empty() {
        return (Vec::new(), Vec::new());
    }
    if !lex::is_ascii_letter(tokens[0]) && b'_' != tokens[0] && b':' != tokens[0] {
        return (tokens.to_vec(), Vec::new());
    }
    let mut attr_name = vec![tokens[0]];
    let tokens = &tokens[1..];
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if !lex::is_ascii_letter_num_hyphen(token)
            && b'_' != token
            && b'.' != token
            && b':' != token
        {
            break;
        }
        attr_name.push(token);
        i += 1;
    }
    (tokens[i..].to_vec(), attr_name)
}

/// 对应 Go 版 Tree.parseTagName。
fn parse_tag_name(tokens: &[u8]) -> (Vec<u8>, Vec<u8>) {
    if 1 > tokens.len() {
        return (Vec::new(), Vec::new());
    }

    let token = tokens[0];
    if !lex::is_ascii_letter(token) {
        return (tokens.to_vec(), Vec::new());
    }
    let mut tag_name = vec![token];
    let mut i = 1;
    while i < tokens.len() {
        let token = tokens[i];
        if !lex::is_ascii_letter_num_hyphen(token) {
            break;
        }
        tag_name.push(token);
        i += 1;
    }
    (tokens[i..].to_vec(), tag_name)
}

fn replace_bytes(input: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut remaining = input;
    while let Some(i) = remaining.windows(from.len()).position(|part| part == from) {
        out.extend_from_slice(&remaining[..i]);
        out.extend_from_slice(to);
        remaining = &remaining[i + from.len()..];
    }
    out.extend_from_slice(remaining);
    out
}

fn restore_caret_tokens(mut tokens: Vec<u8>, restore: bool, left_space: bool) -> Vec<u8> {
    if restore {
        if !tokens
            .windows(b"caretreplacement ".len())
            .any(|part| part == b"caretreplacement ")
            && !left_space
        {
            tokens = replace_bytes(&tokens, b"\" caretreplacement", b"\"caretreplacement");
        }
        tokens = replace_bytes(
            &tokens,
            crate::editor::CARET_REPLACEMENT.as_bytes(),
            crate::editor::CARET.as_bytes(),
        );
    }
    tokens
}

#[cfg(test)]
mod source_aligned_tests {
    use crate::{ast::NodeType, options::ParseOptions, parse::inline};

    fn options() -> ParseOptions {
        let mut options = ParseOptions::new();
        options.protyle_wysiwyg = true;
        options
    }

    /// Go parseInlineHTML/processSpanTag must advance past exactly one closer.
    #[test]
    fn process_span_tag_keeps_following_text_and_attributes() {
        let tree = inline("", br#"<span data-type="block-ref" data-id="ref" data-subtype="d" custom-test="kept">Title</span>tail"#, options());
        let paragraph = tree.root_raw().first_child().unwrap();
        let mark = paragraph.first_child().unwrap();
        assert_eq!(mark.typ(), NodeType::NodeTextMark);
        assert_eq!(mark.data().text_mark_type, "block-ref");
        assert_eq!(mark.data().text_mark_block_ref_id, "ref");
        assert_eq!(mark.data().text_mark_block_ref_subtype, "d");
        assert_eq!(mark.data().text_mark_text_content, "Title");
        assert_eq!(mark.ial_attr("custom-test"), "kept");
        assert!(paragraph.text().ends_with("tail"));
    }

    /// Go processSpanTag rejects absent closers and a same-kind nested opener.
    #[test]
    fn process_span_tag_fallback_does_not_consume_unmatched_content() {
        for input in [
            r#"<span data-type="strong">unclosed"#,
            r#"<span data-type="strong"><span data-type="em">inner</span></span>"#,
        ] {
            let tree = inline("", input.as_bytes(), options());
            let first = tree
                .root_raw()
                .first_child()
                .unwrap()
                .first_child()
                .unwrap();
            assert_eq!(first.typ(), NodeType::NodeText);
            assert!(first.tokens().starts_with(br#"<span data-type="strong">"#));
            if input.ends_with("unclosed") {
                assert_eq!(first.tokens(), input.as_bytes());
            } else {
                assert!(first.next().is_some());
            }
        }
    }

    /// Go HTMLTag2TextMark gates native HTML tags, but never data-type spans.
    #[test]
    fn native_text_mark_tags_obey_option_and_aliases() {
        for (tag, typ) in [
            ("b", "strong"),
            ("i", "em"),
            ("del", "s"),
            ("strike", "s"),
            ("u", "u"),
        ] {
            let input = format!("<{tag}>text</{tag}>");
            for enabled in [false, true] {
                let mut options = options();
                options.html_tag2_text_mark = enabled;
                let tree = inline("", input.as_bytes(), options);
                let first = tree
                    .root_raw()
                    .first_child()
                    .unwrap()
                    .first_child()
                    .unwrap();
                assert_eq!(
                    first.typ(),
                    if enabled {
                        NodeType::NodeTextMark
                    } else {
                        NodeType::NodeText
                    }
                );
                if enabled {
                    assert_eq!(first.data().text_mark_type, typ);
                    assert_eq!(first.data().text_mark_text_content, "text");
                } else {
                    assert_eq!(first.tokens(), input.as_bytes());
                }
            }
        }
    }

    /// Go parseInlineHTML recognizes br spelling variants only in Protyle mode.
    #[test]
    fn protyle_line_break_recognition_preserves_plain_html_mode() {
        for br in ["<br>", "<br/>", "<br />", "<BR>"] {
            for protyle in [false, true] {
                let mut options = options();
                options.protyle_wysiwyg = protyle;
                let tree = inline("", br.as_bytes(), options);
                let node = tree
                    .root_raw()
                    .first_child()
                    .unwrap()
                    .first_child()
                    .unwrap();
                assert_eq!(
                    node.typ(),
                    if protyle {
                        NodeType::NodeBr
                    } else {
                        NodeType::NodeInlineHTML
                    }
                );
            }
        }
    }
}
