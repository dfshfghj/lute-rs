// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML 块。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::ial::{ial2_map_un_esc, parse_kramdown_block_ial_tokens};
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

const TAGS1: &[&[u8]] = &[b"<script", b"<pre", b"<style", b"<textarea"];
const CLOSE_TAGS1: &[&[u8]] = &[b"</script>", b"</pre>", b"</style>", b"</textarea>"];

const TAGS6: &[&[u8]] = &[
    b"<address",
    b"<article",
    b"<aside",
    b"<base",
    b"<basefont",
    b"<blockquote",
    b"<body",
    b"<caption",
    b"<center",
    b"<col",
    b"<colgroup",
    b"<dd",
    b"<details",
    b"<dialog",
    b"<dir",
    b"<div",
    b"<dl",
    b"<dt",
    b"<fieldset",
    b"<figcaption",
    b"<figure",
    b"<footer",
    b"<form",
    b"<frame",
    b"<frameset",
    b"<h1",
    b"<h2",
    b"<h3",
    b"<h4",
    b"<h5",
    b"<h6",
    b"<head",
    b"<header",
    b"<hr",
    b"<html",
    b"<iframe",
    b"<legend",
    b"<li",
    b"<link",
    b"<main",
    b"<menu",
    b"<menuitem",
    b"<nav",
    b"<noframes",
    b"<ol",
    b"<optgroup",
    b"<option",
    b"<p",
    b"<param",
    b"<section",
    b"<source",
    b"<summary",
    b"<table",
    b"<tbody",
    b"<td",
    b"<tfoot",
    b"<th",
    b"<thead",
    b"<title",
    b"<tr",
    b"<track",
    b"<ul",
    b"<video",
    b"<audio",
    b"</address",
    b"</article",
    b"</aside",
    b"</base",
    b"</basefont",
    b"</blockquote",
    b"</body",
    b"</caption",
    b"</center",
    b"</col",
    b"</colgroup",
    b"</dd",
    b"</details",
    b"</dialog",
    b"</dir",
    b"</div",
    b"</dl",
    b"</dt",
    b"</fieldset",
    b"</figcaption",
    b"</figure",
    b"</footer",
    b"</form",
    b"</frame",
    b"</frameset",
    b"</h1",
    b"</h2",
    b"</h3",
    b"</h4",
    b"</h5",
    b"</h6",
    b"</head",
    b"</header",
    b"</hr",
    b"</html",
    b"</iframe",
    b"</legend",
    b"</li",
    b"</link",
    b"</main",
    b"</menu",
    b"</menuitem",
    b"</nav",
    b"</noframes",
    b"</ol",
    b"</optgroup",
    b"</option",
    b"</p",
    b"</param",
    b"</section",
    b"</source",
    b"</summary",
    b"</table",
    b"</tbody",
    b"</td",
    b"</tfoot",
    b"</th",
    b"</thead",
    b"</title",
    b"</tr",
    b"</track",
    b"</ul",
    b"</video",
    b"</audio",
];

/// 判断 HTML 块（<）是否开始。
pub fn html_block_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    if lex::ITEM_LESS != lex::peek(&p.ctx.current_line, p.ctx.next_nonspace) {
        return BLOCK_START_NONE;
    }

    if (p.options.vditor_wysiwyg
        && p.ctx
            .current_line
            .windows(14)
            .any(|w| w == b"vditor-comment"))
        || (p.options.protyle_wysiwyg && p.ctx.current_line.windows(6).any(|w| w == b"<span "))
    {
        return BLOCK_START_NONE;
    }

    let tokens = &p.ctx.current_line[p.ctx.next_nonspace..];
    let html_type = parse_html(&p.ctx, tokens);
    if 0 == html_type {
        return BLOCK_START_NONE;
    }

    p.close_unmatched_blocks();
    if p.options.protyle_wysiwyg {
        let tokens = lex::trim_whitespace(&p.ctx.current_line[p.ctx.next_nonspace..]);
        let typ = if tokens.starts_with(b"<iframe") && tokens.ends_with(b">") {
            if tokens
                .windows(b"data-subtype=\"widget\"".len())
                .any(|w| w == b"data-subtype=\"widget\"")
            {
                Some(NodeType::NodeWidget)
            } else {
                Some(NodeType::NodeIFrame)
            }
        } else if tokens.starts_with(b"<video") && tokens.ends_with(b">") {
            Some(NodeType::NodeVideo)
        } else if tokens.starts_with(b"<audio") && tokens.ends_with(b">") {
            Some(NodeType::NodeAudio)
        } else {
            None
        };
        if let Some(typ) = typ {
            p.add_child(typ);
            return BLOCK_START_LEAF;
        }
        if tokens.starts_with(b"<div")
            && tokens
                .windows(b"data-type=\"NodeAttributeView\"".len())
                .any(|w| w == b"data-type=\"NodeAttributeView\"")
            && tokens
                .windows(b"data-av-type=\"".len())
                .any(|w| w == b"data-av-type=\"")
            && tokens.ends_with(b"</div>")
        {
            let attr = |key: &str| -> Option<String> {
                let start = tokens
                    .windows(key.len())
                    .position(|w| w == key.as_bytes())?
                    + key.len();
                let end = tokens[start..].iter().position(|&b| b == b'"')? + start;
                Some(String::from_utf8_lossy(&tokens[start..end]).into_owned())
            };
            let av_type = attr("data-av-type=\"").unwrap_or_default();
            let av_id = attr("data-av-id=\"").unwrap_or_else(crate::ast::new_node_id);
            let av = p.add_child(NodeType::NodeAttributeView);
            av.data_mut().attribute_view_type = av_type;
            av.data_mut().attribute_view_id = av_id;
            return BLOCK_START_LEAF;
        }
        if !lex::trim_left(&p.ctx.current_line[p.ctx.next_nonspace..])
            .1
            .starts_with(b"<div")
        {
            return BLOCK_START_NONE;
        }
    }
    let block = p.add_child(NodeType::NodeHTMLBlock);
    block.data_mut().html_block_type = html_type;
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    pub(super) fn html_block_continue(&mut self, html: Node) -> i32 {
        if crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial {
            let mut tokens = &self.ctx.current_line[self.ctx.offset..];
            if tokens.len() > 7 && tokens.windows(4).any(|w| w == b"id=\"") {
                if self.ctx.tip.and_then(|n| n.parent()).map(|n| n.typ())
                    == Some(NodeType::NodeListItem)
                {
                    tokens = lex::trim_left(tokens).1;
                }
                if let Some(ial) = parse_kramdown_block_ial_tokens(tokens).filter(|v| !v.is_empty())
                {
                    let tip = self.ctx.tip.expect("tip");
                    tip.set_id(ial2_map_un_esc(&ial).get("id").cloned().unwrap_or_default());
                    tip.data_mut().kramdown_ial = ial;
                    return 1;
                }
            }
        }
        if self.ctx.blank {
            let typ = html.data().html_block_type;
            if 6 == typ || 7 == typ {
                return 1;
            }
        }
        0
    }

    pub(super) fn html_block_finalize(&mut self, html: Node) {
        let mut tokens = html.tokens_vec();
        lex::replace_newline_space(&mut tokens);
        html.set_tokens(lex::trim_right(&tokens).1.to_vec());
    }

    /// 对应 Go 版 isHTMLBlockClose。
    pub(super) fn is_html_block_close(&mut self, tokens: &[u8], html_type: i32) -> bool {
        if (crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial)
            && tokens.len() > 7
            && tokens.windows(4).any(|w| w == b"id=\"")
        {
            if let Some(ial) = parse_kramdown_block_ial_tokens(tokens).filter(|v| !v.is_empty()) {
                let tip = self.ctx.tip.expect("tip");
                tip.set_id(ial2_map_un_esc(&ial).get("id").cloned().unwrap_or_default());
                tip.data_mut().kramdown_ial = ial;
                let node = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
                node.set_tokens(tokens.to_vec());
                tip.insert_after(node);
                return true;
            }
        }
        let length = tokens.len();
        match html_type {
            1 => 0 <= lex::accept_tokenss(tokens, CLOSE_TAGS1),
            2 => {
                if 3 <= length {
                    for i in 0..=length - 3 {
                        if lex::ITEM_HYPHEN == tokens[i]
                            && lex::ITEM_HYPHEN == tokens[i + 1]
                            && lex::ITEM_GREATER == tokens[i + 2]
                        {
                            return true;
                        }
                    }
                }
                false
            }
            3 => {
                if 2 <= length {
                    for i in 0..=length - 2 {
                        if lex::ITEM_QUESTION == tokens[i] && lex::ITEM_GREATER == tokens[i + 1] {
                            return true;
                        }
                    }
                }
                false
            }
            4 => tokens.contains(&lex::ITEM_GREATER),
            5 => {
                if 2 <= length {
                    for i in 0..=length - 2 {
                        if lex::ITEM_CLOSE_BRACKET == tokens[i]
                            && lex::ITEM_CLOSE_BRACKET == tokens[i + 1]
                        {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

fn parse_html(ctx: &crate::parse::Context, tokens: &[u8]) -> i32 {
    let (_, tokens) = lex::trim_left(tokens);
    let length = tokens.len();
    if 3 > length {
        // at least <? and a newline
        return 0;
    }

    if lex::ITEM_LESS != tokens[0] {
        return 0;
    }

    let mut typ = 1;

    let pos = lex::accept_tokenss(tokens, TAGS1);
    if 0 <= pos {
        let pos = pos as usize;
        if lex::is_whitespace(tokens[pos]) || lex::ITEM_GREATER == tokens[pos] {
            return typ;
        }
    }

    let pos = lex::accept_tokenss(tokens, TAGS6);
    if 0 <= pos {
        let pos = pos as usize;
        if lex::is_whitespace(tokens[pos]) || lex::ITEM_GREATER == tokens[pos] {
            typ = 6;
            return typ;
        }
        if pos + 1 < tokens.len()
            && lex::ITEM_SLASH == tokens[pos]
            && lex::ITEM_GREATER == tokens[pos + 1]
        {
            typ = 6;
            return typ;
        }
    }

    let tag = lex::trim_whitespace(tokens);
    let is_open = is_open_tag(tag);
    if is_open && ctx.tip.map(|n| n.typ()) != Some(NodeType::NodeParagraph) {
        typ = 7;
        return typ;
    }
    let is_close = is_close_tag(tag);
    if is_close && ctx.tip.map(|n| n.typ()) != Some(NodeType::NodeParagraph) {
        typ = 7;
        return typ;
    }

    if tokens.starts_with(b"<!--") {
        typ = 2;
        return typ;
    }

    if tokens.starts_with(b"<?") {
        typ = 3;
        return typ;
    }

    if 2 < length && tokens.starts_with(b"<!") {
        let following = &tokens[2..];
        if b'A' <= following[0] && b'Z' >= following[0] {
            typ = 4;
            return typ;
        }
        if following.starts_with(b"[CDATA[") {
            typ = 5;
            return typ;
        }
    }
    0
}

fn is_open_tag(tokens: &[u8]) -> bool {
    let length = tokens.len();
    if 3 > length {
        return false;
    }

    if lex::ITEM_LESS != tokens[0] {
        return false;
    }
    if lex::ITEM_GREATER != tokens[length - 1] {
        return false;
    }
    let tokens = if lex::ITEM_SLASH == tokens[length - 2] {
        &tokens[1..length - 2]
    } else {
        &tokens[1..length - 1]
    };

    if tokens.is_empty() {
        return false;
    }

    if lex::is_whitespace(tokens[0]) {
        // < 后面不能跟空白
        return false;
    }

    let name_and_attrs = lex::split_whitespace(tokens);
    let name = name_and_attrs[0];
    if name.is_empty() || !lex::is_ascii_letter(name[0]) {
        return false;
    }
    if 1 < name.len() {
        for &n in &name[1..] {
            if !lex::is_ascii_letter_num_hyphen(n) {
                return false;
            }
        }
    }

    for attr in &name_and_attrs[1..] {
        if 1 >= attr.len() {
            continue;
        }

        let mut parts = attr.splitn(2, |&b| b == b'=');
        let name = parts.next().unwrap_or(&[]);
        if name.is_empty() {
            // 等号前面空格的情况
            continue;
        }
        if !lex::is_ascii_letter(name[0])
            && lex::ITEM_UNDERSCORE != name[0]
            && lex::ITEM_COLON != name[0]
        {
            return false;
        }

        if 1 < name.len() {
            for &n in &name[1..] {
                if !lex::is_ascii_letter(n)
                    && !lex::is_digit(n)
                    && lex::ITEM_UNDERSCORE != n
                    && lex::ITEM_DOT != n
                    && lex::ITEM_COLON != n
                    && lex::ITEM_HYPHEN != n
                {
                    return false;
                }
            }
        }

        if let Some(value) = parts.next() {
            if value.len() >= 2 && value.starts_with(b"'") && value.ends_with(b"'") {
                let value = &value[1..value.len() - 1];
                if value.contains(&b'\'') {
                    return false;
                }
                continue;
            }
            if value.len() >= 2 && value.starts_with(b"\"") && value.ends_with(b"\"") {
                let value = &value[1..value.len() - 1];
                if value.contains(&b'"') {
                    return false;
                }
                continue;
            }
            if value.iter().any(|&b| b == b' ' || b == b'\t' || b == b'\n')
                || value
                    .iter()
                    .any(|&b| matches!(b, b'"' | b'\'' | b'=' | b'<' | b'>' | b'`'))
            {
                return false;
            }
        }
    }
    true
}

fn is_close_tag(tokens: &[u8]) -> bool {
    let tokens = lex::trim_whitespace(tokens);
    let length = tokens.len();
    if 4 > length {
        return false;
    }

    if lex::ITEM_LESS != tokens[0] || lex::ITEM_SLASH != tokens[1] {
        return false;
    }
    if lex::ITEM_GREATER != tokens[length - 1] {
        return false;
    }

    let name = &tokens[2..length - 1];
    if name.is_empty() || !lex::is_ascii_letter(name[0]) {
        return false;
    }
    if 1 < name.len() {
        for &n in &name[1..] {
            if !lex::is_ascii_letter_num_hyphen(n) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod source_alignment_tests {
    use crate::{ast::NodeType, options::ParseOptions, parse::parse};

    #[test]
    fn protyle_html_block_routing() {
        for (input, typ) in [
            ("<iframe src=\"x\"></iframe>", NodeType::NodeIFrame),
            (
                "<iframe data-subtype=\"widget\"></iframe>",
                NodeType::NodeWidget,
            ),
            ("<video src=\"x\"></video>", NodeType::NodeVideo),
            ("<audio src=\"x\"></audio>", NodeType::NodeAudio),
            ("<div>foo</div>", NodeType::NodeHTMLBlock),
            ("<span id=\"x\">foo</span>", NodeType::NodeParagraph),
        ] {
            let mut options = ParseOptions::new();
            options.protyle_wysiwyg = true;
            let tree = parse("", input.as_bytes(), options);
            assert_eq!(tree.root().first_child().unwrap().typ(), typ, "{input}");
        }
    }

    #[test]
    fn html_ial_interrupts_block_and_preserves_id() {
        let mut options = ParseOptions::new();
        options.kramdown_block_ial = true;
        let tree = parse(
            "",
            b"<div>\nfoo\n</div>\n{: id=\"20221022222200-2b6wj74\"}\n",
            options,
        );
        let html = tree.root().first_child().unwrap();
        assert_eq!(html.typ(), NodeType::NodeHTMLBlock);
        assert_eq!(html.id(), "20221022222200-2b6wj74");
        assert!(!html.tokens().windows(2).any(|w| w == b"{:"));
    }

    #[test]
    fn attribute_view_preserves_metadata() {
        let mut options = ParseOptions::new();
        options.protyle_wysiwyg = true;
        let tree = parse("", b"<div data-type=\"NodeAttributeView\" data-av-type=\"table\" data-av-id=\"test-av\"></div>", options);
        let av = tree.root_raw().first_child().unwrap();
        assert_eq!(av.typ(), NodeType::NodeAttributeView);
        assert_eq!(av.data().attribute_view_type, "table");
        assert_eq!(av.data().attribute_view_id, "test-av");
    }
}
