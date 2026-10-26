//! Faithful external text-style import from Go parse/html_text_style*.go.
use crate::ast::{self, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::dom::{Dom, DomType};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTMLTextStyle {
    pub color: String,
    pub background: String,
    pub family: String,
    pub size: f64,
    pub size_unit: String,
}

fn is_html_text_color(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphabetic()) {
        return true;
    }
    if let Some((function, rest)) = value.split_once('(') {
        if matches!(
            function.to_ascii_lowercase().as_str(),
            "rgb" | "rgba" | "hsl" | "hsla"
        ) {
            return rest.strip_suffix(')').is_some_and(|s| {
                !s.is_empty() && s.bytes().all(|b| b"0123456789.,%+ /-".contains(&b))
            });
        }
        if function.eq_ignore_ascii_case("var") {
            return rest.strip_suffix(')').is_some_and(|s| {
                let s = s.trim();
                ["--b3-font-color", "--b3-font-background"]
                    .iter()
                    .any(|prefix| {
                        s.strip_prefix(prefix).is_some_and(|digits| {
                            digits
                                .as_bytes()
                                .first()
                                .is_some_and(|b| (b'1'..=b'9').contains(b))
                                && digits.bytes().all(|b| b.is_ascii_digit())
                        })
                    })
            });
        }
    }
    false
}

pub fn html_style_declarations(style: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut important = HashMap::new();
    let (mut start, mut depth, mut quote, mut escaped) = (0, 0i32, None, false);
    let terminated = format!("{style};");
    for (i, c) in terminated.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if matches!(c, '\'' | '"') {
            quote = Some(c);
            continue;
        }
        if c == '(' {
            depth += 1;
        }
        if c == ')' {
            depth -= 1;
        }
        if c != ';' || depth != 0 {
            continue;
        }
        let declaration = &terminated[start..i];
        start = i + 1;
        let Some((key, value)) = declaration.split_once(':') else {
            continue;
        };
        let mut key = key.trim().to_lowercase();
        let mut value = value.trim();
        let mut priority = false;
        if let Some(i) = value.rfind('!') {
            if value[i + 1..].trim().eq_ignore_ascii_case("important") {
                priority = true;
                value = value[..i].trim();
            }
        }
        if key == "background" && is_html_text_color(value) {
            key = "background-color".into();
        }
        if !important.get(&key).copied().unwrap_or(false) || priority {
            important.insert(key.clone(), priority);
            result.insert(key, value.to_owned());
        }
    }
    result
}

impl HTMLTextStyle {
    pub fn css(&self) -> String {
        let mut result = String::new();
        for (key, value) in [
            ("color", &self.color),
            ("background-color", &self.background),
            ("font-family", &self.family),
        ] {
            if !value.is_empty() {
                result.push_str(&format!("{key}: {value};"));
            }
        }
        if !self.size_unit.is_empty() {
            result.push_str(&format!("font-size: {:.6}{};", self.size, self.size_unit));
        }
        result
    }
}

fn safe_html_font_family(value: &str) -> bool {
    if value.len() > 1024 {
        return false;
    }
    let mut quote = None;
    for c in value.chars() {
        if matches!(c, '\'' | '"') {
            if quote == Some(c) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(c);
            }
            continue;
        }
        if !crate::render::is_letter_or_digit(c)
            && !c.is_whitespace()
            && !matches!(c, '-' | '_' | ',')
        {
            return false;
        }
    }
    quote.is_none()
}

pub fn resolve_html_text_style(dom: &Dom, idx: usize, parent: HTMLTextStyle) -> HTMLTextStyle {
    let tag = dom.nodes[idx].data.as_str();
    let raw = dom.attr(idx, "style");
    let mut style = parent.clone();
    if raw.is_empty() && tag != "small" && tag != "font" {
        return style;
    }
    let mut decl = html_style_declarations(&raw);
    if tag == "font" {
        for (attr, key) in [("color", "color"), ("face", "font-family")] {
            decl.entry(key.into())
                .or_insert_with(|| dom.attr(idx, attr));
        }
        if !decl.contains_key("font-size") {
            if let Ok(size @ 1..=7) = dom.attr(idx, "size").parse::<usize>() {
                decl.insert(
                    "font-size".into(),
                    [
                        "",
                        "x-small",
                        "small",
                        "medium",
                        "large",
                        "x-large",
                        "xx-large",
                        "xxx-large",
                    ][size]
                        .into(),
                );
            }
        }
    }
    for key in ["color", "background-color", "font-family"] {
        let Some(mut value) = decl.get(key).cloned().filter(|v| !v.is_empty()) else {
            continue;
        };
        match value.to_lowercase().as_str() {
            "inherit" | "unset" => continue,
            "initial" | "revert" | "revert-layer" => value.clear(),
            "currentcolor" => {
                if key == "color" {
                    continue;
                }
                value = style.color.clone();
            }
            _ => {}
        }
        let reset = matches!(
            decl[key].to_lowercase().as_str(),
            "initial" | "revert" | "revert-layer"
        );
        if reset
            || if key == "font-family" {
                safe_html_font_family(&value)
            } else {
                is_html_text_color(&value)
            }
        {
            match key {
                "color" => style.color = value,
                "background-color" => style.background = value,
                _ => style.family = value,
            }
        }
    }
    let mut value = decl
        .get("font-size")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if value.is_empty() && tag == "small" {
        value = "smaller".into();
    }
    let (base, unit) = if parent.size_unit.is_empty() {
        (1.0, "em")
    } else {
        (parent.size, parent.size_unit.as_str())
    };
    match value.as_str() {
        "" | "inherit" | "unset" => {}
        "initial" | "medium" | "revert" | "revert-layer" => {
            style.size = 1.0;
            style.size_unit = "rem".into();
        }
        "smaller" | "larger" => {
            style.size = if value == "smaller" {
                base / 1.2
            } else {
                base * 1.2
            };
            style.size_unit = unit.into();
        }
        _ => {
            let ratio = match value.as_str() {
                "xx-small" => Some(0.6),
                "x-small" => Some(0.75),
                "small" => Some(0.888889),
                "large" => Some(1.2),
                "x-large" => Some(1.5),
                "xx-large" => Some(2.0),
                "xxx-large" => Some(3.0),
                _ => None,
            };
            if let Some(ratio) = ratio {
                style.size = ratio;
                style.size_unit = "rem".into();
            } else if let Some((number, suffix)) = parse_size(&value) {
                if number > 0.0 && number <= 10000.0 {
                    let (size, new_unit) = match suffix {
                        "em" => (base * number, unit),
                        "%" => (base * number / 100.0, unit),
                        "px" | "rem" => (number, suffix),
                        "pt" => (number * (96.0 / 72.0), "px"),
                        "pc" => (number * 16.0, "px"),
                        "in" => (number * 96.0, "px"),
                        "cm" => (number * (96.0 / 2.54), "px"),
                        "mm" => (number * (96.0 / 25.4), "px"),
                        _ => unreachable!(),
                    };
                    style.size = size;
                    style.size_unit = new_unit.into();
                }
            }
        }
    }
    style
}

fn parse_size(value: &str) -> Option<(f64, &str)> {
    for unit in ["rem", "px", "pt", "pc", "in", "cm", "mm", "em", "%"] {
        if let Some(number) = value.strip_suffix(unit) {
            if !number.as_bytes().last().is_some_and(u8::is_ascii_digit) {
                return None;
            }
            if number.bytes().filter(|&b| b == b'.').count() > 1
                || !number.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            {
                return None;
            }
            return number.parse().ok().map(|n| (n, unit));
        }
    }
    None
}

pub fn html_node_text_style(dom: &Dom, idx: usize) -> HTMLTextStyle {
    let mut parents = Vec::new();
    let mut p = Some(idx);
    while let Some(idx) = p {
        parents.push(idx);
        p = dom.nodes[idx].parent;
    }
    let mut style = HTMLTextStyle::default();
    for idx in parents.into_iter().rev() {
        style = resolve_html_text_style(dom, idx, style);
    }
    style
}

pub(crate) fn apply_html_text_semantics(node: Node, dom: &Dom, idx: usize) {
    let (mut weight, mut italic) = (String::new(), String::new());
    let mut p = Some(idx);
    while let Some(idx) = p {
        let decl = html_style_declarations(&dom.attr(idx, "style"));
        if weight.is_empty() {
            weight = decl
                .get("font-weight")
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
        }
        if italic.is_empty() {
            italic = decl
                .get("font-style")
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
        }
        let decoration = format!(
            "{} {}",
            decl.get("text-decoration")
                .map(String::as_str)
                .unwrap_or(""),
            decl.get("text-decoration-line")
                .map(String::as_str)
                .unwrap_or("")
        );
        for (css, typ) in [("underline", "u"), ("line-through", "s")] {
            if decoration.contains(css) && !node.is_text_mark_type(typ) {
                node.data_mut().text_mark_type.push_str(&format!(" {typ}"));
            }
        }
        p = dom.nodes[idx].parent;
    }
    if (matches!(weight.as_str(), "bold" | "bolder") || weight.parse::<i64>().unwrap_or(0) >= 600)
        && !node.is_text_mark_type("strong")
    {
        node.data_mut().text_mark_type.push_str(" strong");
    }
    if (italic == "italic" || italic.starts_with("oblique")) && !node.is_text_mark_type("em") {
        node.data_mut().text_mark_type.push_str(" em");
    }
}

fn ensure_html_text_mark(node: Node) -> bool {
    match node.typ() {
        NodeType::NodeText | NodeType::NodeLinkText | NodeType::NodeHTMLEntity => {
            if node.parent_is(&[NodeType::NodeImage]) {
                return false;
            }
            node.data_mut().text_mark_type = "text".into();
            node.data_mut().text_mark_text_content = crate::html::escape_html(&node.tokens_str());
            node.clear_tokens();
            node.set_typ(NodeType::NodeTextMark);
        }
        NodeType::NodeTextMark => {}
        _ => return false,
    }
    let mut p = node.parent();
    while let Some(parent) = p {
        if parent.is_block() {
            break;
        }
        if parent.typ() == NodeType::NodeLink {
            if !node.is_text_mark_type("a") {
                node.data_mut().text_mark_type = format!("a {}", node.data().text_mark_type);
            }
            if let Some(dest) = parent.child_by_type(NodeType::NodeLinkDest) {
                node.data_mut().text_mark_a_href = dest.tokens_str();
            }
            if let Some(title) = parent.child_by_type(NodeType::NodeLinkTitle) {
                node.data_mut().text_mark_a_title = title.tokens_str();
            }
        }
        p = parent.parent();
    }
    true
}

pub(crate) fn apply_html_text_style(tree: &mut Tree, node: Node, style: &str) {
    if style.is_empty() || !ensure_html_text_mark(node) {
        return;
    }
    node.set_ial_attr("style", style);
    let tokens = super::ial::ial_to_tokens(&node.ial_raw());
    if let Some(ial) = node
        .next()
        .filter(|n| n.typ() == NodeType::NodeKramdownSpanIAL)
    {
        ial.set_tokens(tokens);
    } else {
        let ial = tree.new_node_raw(NodeType::NodeKramdownSpanIAL);
        ial.set_tokens(tokens);
        node.insert_after(ial);
    }
}

#[derive(Clone)]
struct StyleFrame {
    node: Node,
    dom: usize,
    style: HTMLTextStyle,
    protected: bool,
}

pub(crate) fn inline_style_tag(node: Node, dom: &mut Dom) -> Option<(usize, bool)> {
    if node.typ() != NodeType::NodeInlineHTML {
        return None;
    }
    let raw = node.tokens_str();
    let raw = raw.trim_start();
    let closing = raw.starts_with("</");
    let rest = raw.strip_prefix(if closing { "</" } else { "<" })?;
    let end = rest
        .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
        .unwrap_or(rest.len());
    let tag = rest[..end].to_ascii_lowercase();
    if !matches!(
        tag.as_str(),
        "span"
            | "small"
            | "font"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "s"
            | "del"
            | "strike"
            | "u"
            | "code"
            | "a"
            | "sub"
            | "sup"
            | "mark"
            | "kbd"
    ) || !rest.contains('>')
    {
        return None;
    }
    // Go's tokenizer distinguishes a self-closing token from a start tag,
    // including a slash at the end of an unquoted attribute value.
    if !closing {
        let bytes = rest.as_bytes();
        let mut i = end;
        while i < bytes.len() && bytes[i] != b'>' {
            if bytes[i].is_ascii_whitespace() {
                i += 1;
            } else if bytes[i] == b'/' {
                if bytes.get(i + 1) == Some(&b'>') {
                    return None;
                }
                i += 1;
            } else {
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'=' | b'/' | b'>')
                {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if bytes.get(i) != Some(&b'=') {
                    continue;
                }
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if let Some(&quote @ (b'\'' | b'"')) = bytes.get(i) {
                    i += 1;
                    while i < bytes.len() && bytes[i] != quote {
                        i += 1;
                    }
                    i += usize::from(i < bytes.len());
                } else {
                    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>') {
                            return None;
                        }
                        i += 1;
                    }
                }
            }
        }
    }
    let idx = dom.create(DomType::Element, &tag);
    if !closing {
        let parsed = crate::dom::parse(raw);
        if let Some(element) = parsed
            .nodes
            .iter()
            .find(|n| n.typ == DomType::Element && n.data == tag)
        {
            dom.nodes[idx].attrs = element.attrs.clone();
        }
    }
    Some((idx, closing))
}

pub(crate) fn normalize_inline_html_text_styles(tree: &mut Tree) {
    let mut blocks = Vec::new();
    ast::walk_raw(tree.root_raw(), &mut |node, entering| {
        if entering
            && matches!(
                node.typ(),
                NodeType::NodeParagraph | NodeType::NodeHeading | NodeType::NodeTableCell
            )
        {
            blocks.push(node);
            return WalkStatus::WalkSkipChildren;
        }
        WalkStatus::WalkContinue
    });
    for block in blocks {
        normalize_block(tree, block);
    }
}

fn normalize_block(tree: &mut Tree, block: Node) {
    let mut dom = Dom::new();
    let mut tags = HashMap::new();
    let mut pairs = HashMap::new();
    let mut stack: Vec<StyleFrame> = Vec::new();
    let mut has_style = false;
    ast::walk_raw(block, &mut |node, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let Some((idx, closing)) = inline_style_tag(node, &mut dom) else {
            return WalkStatus::WalkContinue;
        };
        tags.insert(node, (idx, closing));
        if !closing {
            stack.push(StyleFrame {
                node,
                dom: idx,
                style: HTMLTextStyle::default(),
                protected: false,
            });
        } else if let Some(open) =
            stack.pop_if(|open| dom.nodes[open.dom].data == dom.nodes[idx].data)
        {
            pairs.insert(open.node, node);
            pairs.insert(node, open.node);
            if !resolve_html_text_style(&dom, open.dom, HTMLTextStyle::default())
                .css()
                .is_empty()
            {
                has_style = true;
            }
        }
        WalkStatus::WalkContinue
    });
    if !has_style {
        return;
    }
    // The Go helper walks a temporary Tree rooted at the existing block.
    super::transform::nested_inlines2flatted_spans_hybrid_at(tree, block, false);
    stack.clear();
    let mut unlinks = Vec::new();
    ast::walk_raw(block, &mut |node, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if let Some(&other) = pairs.get(&node) {
            let (idx, closing) = tags[&node];
            let mut protected = stack.last().is_some_and(|frame| frame.protected);
            if closing {
                if stack.last().is_some_and(|frame| frame.node == other) {
                    stack.pop();
                }
            } else {
                protected = protected
                    || !dom.attr(idx, "data-type").is_empty()
                    || (dom.nodes[idx].data == "span" && !dom.attr(idx, "title").is_empty());
                let parent = stack
                    .last()
                    .map(|frame| frame.style.clone())
                    .unwrap_or_default();
                if let Some(frame) = stack.last() {
                    dom.nodes[idx].parent = Some(frame.dom);
                }
                let style = resolve_html_text_style(&dom, idx, parent);
                stack.push(StyleFrame {
                    node,
                    dom: idx,
                    style,
                    protected,
                });
            }
            if !protected {
                unlinks.push(node);
            }
            return WalkStatus::WalkContinue;
        }
        let Some(frame) = stack.last().filter(|frame| !frame.protected) else {
            return WalkStatus::WalkContinue;
        };
        if !matches!(
            node.typ(),
            NodeType::NodeText
                | NodeType::NodeLinkText
                | NodeType::NodeTextMark
                | NodeType::NodeHTMLEntity
        ) {
            return WalkStatus::WalkContinue;
        }
        let mut style = frame.style.clone();
        let own = node.ial_attr("style");
        if !own.is_empty() {
            let idx = dom.create(DomType::Element, "");
            dom.set_attr(idx, "style", &own);
            style = resolve_html_text_style(&dom, idx, style);
        }
        apply_html_text_style(tree, node, &style.css());
        if node.typ() == NodeType::NodeTextMark {
            apply_html_text_semantics(node, &dom, frame.dom);
        }
        for frame in &stack {
            let typ = match dom.nodes[frame.dom].data.as_str() {
                "strong" | "b" => "strong",
                "em" | "i" => "em",
                "s" | "del" | "strike" => "s",
                "u" => "u",
                "code" => "code",
                "a" => "a",
                "sub" => "sub",
                "sup" => "sup",
                "mark" => "mark",
                "kbd" => "kbd",
                _ => continue,
            };
            if typ == "a" {
                let href = dom.nodes[frame.dom]
                    .attrs
                    .iter()
                    .rev()
                    .find(|(key, _)| key == "href")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_whitespace() && !c.is_control())
                    .collect::<String>()
                    .to_lowercase();
                if ["javascript:", "vbscript:", "data:"]
                    .iter()
                    .any(|scheme| href.starts_with(scheme))
                {
                    continue;
                }
            }
            if node.typ() != NodeType::NodeTextMark {
                ensure_html_text_mark(node);
            }
            if !node.is_text_mark_type(typ) {
                node.data_mut().text_mark_type = format!(
                    "{} {typ}",
                    node.data()
                        .text_mark_type
                        .strip_suffix("text")
                        .unwrap_or(&node.data().text_mark_type)
                )
                .trim()
                .into();
            }
            if typ == "a" {
                for (key, value) in &dom.nodes[frame.dom].attrs {
                    if key == "href" {
                        node.data_mut().text_mark_a_href = value.clone();
                    }
                    if key == "title" {
                        node.data_mut().text_mark_a_title = value.clone();
                    }
                }
            }
        }
        WalkStatus::WalkContinue
    });
    for node in unlinks {
        node.unlink();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resolve(tag: &str, css: &str, parent: HTMLTextStyle) -> HTMLTextStyle {
        let mut dom = Dom::new();
        let idx = dom.create(DomType::Element, tag);
        dom.set_attr(idx, "style", css);
        resolve_html_text_style(&dom, idx, parent)
    }
    // Go: parse/html_text_style_test.go::TestResolveHTMLTextStyle
    #[test]
    fn go_resolve_html_text_style() {
        let parent = HTMLTextStyle {
            color: "red".into(),
            family: "Arial".into(),
            size: 24.0,
            size_unit: "px".into(),
            ..Default::default()
        };
        for (name, css, tag, size, unit, color, bg, family) in [
            ("inherit", "", "span", 24.0, "px", "red", "", "Arial"),
            ("small", "", "small", 20.0, "px", "red", "", "Arial"),
            (
                "explicit small",
                "font-size:18px",
                "small",
                18.0,
                "px",
                "red",
                "",
                "Arial",
            ),
            (
                "percentage",
                "font-size:50%",
                "span",
                12.0,
                "px",
                "red",
                "",
                "Arial",
            ),
            (
                "em",
                "font-size:1.5em",
                "span",
                36.0,
                "px",
                "red",
                "",
                "Arial",
            ),
            (
                "pt",
                "font-size:12pt",
                "span",
                16.0,
                "px",
                "red",
                "",
                "Arial",
            ),
            (
                "priority",
                "COLOR: blue !important;color: green;background-color:currentColor",
                "span",
                24.0,
                "px",
                "blue",
                "blue",
                "Arial",
            ),
            (
                "family",
                r#"font-family: 'Noto Sans', "Microsoft YaHei", sans-serif"#,
                "span",
                24.0,
                "px",
                "red",
                "",
                r#"'Noto Sans', "Microsoft YaHei", sans-serif"#,
            ),
            (
                "unsafe",
                r#"color:url(https://example.com);background-color:expression(alert(1));font-family:"x";position:fixed;font-size:calc(1px + 2px)"#,
                "span",
                24.0,
                "px",
                "red",
                "",
                r#""x""#,
            ),
        ] {
            let got = resolve(tag, css, parent.clone());
            assert_eq!(
                got,
                HTMLTextStyle {
                    color: color.into(),
                    background: bg.into(),
                    family: family.into(),
                    size,
                    size_unit: unit.into()
                },
                "{name}"
            );
            assert!(!got.css().contains("position"));
        }
    }
    // Go: parse/html_text_style_test.go::TestHTMLStyleDeclarations
    #[test]
    fn go_html_style_declarations() {
        let got = html_style_declarations(
            r#"font-family:"A;B";color:rgb(1, 2, 3); COLOR:blue ! important;color:red;"#,
        );
        assert_eq!(got["font-family"], r#""A;B""#);
        assert_eq!(got["color"], "blue");
    }
    // Go: parse/html_text_style_test.go::TestHTMLBackgroundShorthand
    #[test]
    fn go_background_shorthand() {
        for (css, want) in [
            ("background-color:red;background:blue", "blue"),
            ("background:blue;background-color:red", "red"),
            ("background:blue!important;background-color:red", "blue"),
            ("background-color:red!important;background:blue", "red"),
            ("background:rgb(255,165,61)", "rgb(255,165,61)"),
            (
                "background:var(--b3-font-background6)",
                "var(--b3-font-background6)",
            ),
            (
                "background-color:red;background:var(--b3-font-background6)",
                "var(--b3-font-background6)",
            ),
            ("background:url(https://example.com/image.png)", ""),
        ] {
            assert_eq!(
                resolve("", css, HTMLTextStyle::default()).background,
                want,
                "{css}"
            );
        }
    }
    // Go: parse/html_text_style_test.go::TestHTMLTextThemeColors
    #[test]
    fn go_theme_colors() {
        for value in [
            "var(--b3-font-color6)",
            "var(--b3-font-background13)",
            "VAR( --b3-font-color10 )",
        ] {
            let got = resolve(
                "",
                &format!("color:{value};background-color:{value}"),
                HTMLTextStyle::default(),
            );
            assert_eq!(got.color, value);
            assert_eq!(got.background, value);
        }
        for value in [
            "var(--other)",
            "var(--B3-font-color6)",
            "var(--b3-font-color0)",
            "var(--b3-font-color6, url(https://example.com))",
            "var(--b3-font-color6) url(https://example.com)",
            "var(--b3-font-color6",
        ] {
            assert!(
                resolve("", &format!("color:{value}"), HTMLTextStyle::default())
                    .color
                    .is_empty(),
                "{value}"
            );
        }
    }
    #[test]
    fn size_grammar_limits_and_relative_defaults() {
        for value in ["0px", "10001px", "1.px", "+2px", "2e2px", "-1px", "1.2.3px"] {
            assert_eq!(
                resolve(
                    "span",
                    &format!("font-size:{value}"),
                    HTMLTextStyle::default()
                ),
                HTMLTextStyle::default(),
                "{value}"
            );
        }
        assert_eq!(
            resolve("span", "font-size:50%", HTMLTextStyle::default()).css(),
            "font-size: 0.500000em;"
        );
        assert_eq!(
            resolve("span", "font-size:.5em", HTMLTextStyle::default()).css(),
            "font-size: 0.500000em;"
        );
    }
    #[test]
    fn inline_style_token_types_match_go_tokenizer() {
        let mut tree = Tree::new();
        let mut dom = Dom::new();
        for (raw, accepted) in [
            ("<span style=\"color:red\">", true),
            ("<span-x style=\"color:red\">", false),
            ("</span-x>", false),
            ("<span style=\"color:red\" />", false),
            ("<span style=\"color:red\"/>", false),
            ("<span style=color:red/>", false),
            ("<span/>", false),
            ("</SPAN>", true),
        ] {
            let node = tree.new_node_raw(NodeType::NodeInlineHTML);
            node.set_tokens(raw.as_bytes().to_vec());
            assert_eq!(
                inline_style_tag(node, &mut dom).is_some(),
                accepted,
                "{raw}"
            );
        }
    }
    #[test]
    fn font_family_uses_go_letter_and_decimal_digit_categories() {
        for family in ["Noto Sans 中文", "Font ١٢٣"] {
            assert!(safe_html_font_family(family), "{family}");
        }
        for family in ["Font ²", "Font Ⅳ", "Font \u{0345}"] {
            assert!(!safe_html_font_family(family), "{family}");
        }
    }
    #[test]
    fn paired_style_preserves_nested_markdown_and_unclosed_tags() {
        let options = crate::options::ParseOptions {
            protyle_wysiwyg: true,
            ..Default::default()
        };
        let tree = crate::parse::parse(
            "",
            b"<span style=\"color:red\">**bold** plain</span>",
            options.clone(),
        );
        let mut marks = Vec::new();
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering && n.typ() == NodeType::NodeTextMark {
                marks.push((n.data().text_mark_type.clone(), n.ial_attr("style")));
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(marks.len(), 2);
        assert!(marks[0].0.contains("strong"));
        assert!(marks.iter().all(|(_, s)| s == "color: red;"));
        let tree = crate::parse::parse("", b"<span style=\"color:red\">unclosed", options);
        let paragraph = tree.root_raw().first_child().unwrap();
        assert!(paragraph.children().all(|n| n.typ() == NodeType::NodeText));
        assert_eq!(paragraph.text(), "<span style=\"color:red\">unclosed");
    }
}
