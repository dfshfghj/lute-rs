// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 轻量 HTML DOM，供 HTML → Markdown 使用。
//!
//! 结构与 Go 版 x/net/html 的 Node 对齐：元素、文本、注释、DOCTYPE，
//! 属性保持小写键名。解析器覆盖常见 HTML，并实现主要的隐式闭合规则。

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DomType {
    Document,
    Element,
    Text,
    Comment,
    Doctype,
}

#[derive(Debug)]
pub struct DomNode {
    pub typ: DomType,
    pub data: String,
    pub attrs: Vec<(String, String)>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// HTML 文档树。
pub struct Dom {
    pub nodes: Vec<DomNode>,
    pub root: usize,
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

impl Dom {
    pub fn new() -> Self {
        let root = DomNode {
            typ: DomType::Document,
            data: String::new(),
            attrs: Vec::new(),
            parent: None,
            children: Vec::new(),
        };
        Self {
            nodes: vec![root],
            root: 0,
        }
    }

    pub fn create(&mut self, typ: DomType, data: impl Into<String>) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(DomNode {
            typ,
            data: data.into(),
            attrs: Vec::new(),
            parent: None,
            children: Vec::new(),
        });
        idx
    }

    pub fn append_child(&mut self, parent: usize, child: usize) {
        self.unlink(child);
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    pub fn insert_before(&mut self, reference: usize, node: usize) {
        self.unlink(node);
        let parent = match self.nodes[reference].parent {
            Some(p) => p,
            None => return,
        };
        self.nodes[node].parent = Some(parent);
        if let Some(pos) = self.nodes[parent]
            .children
            .iter()
            .position(|&c| c == reference)
        {
            self.nodes[parent].children.insert(pos, node);
        }
    }

    pub fn insert_after(&mut self, reference: usize, node: usize) {
        self.unlink(node);
        let parent = match self.nodes[reference].parent {
            Some(p) => p,
            None => return,
        };
        self.nodes[node].parent = Some(parent);
        if let Some(pos) = self.nodes[parent]
            .children
            .iter()
            .position(|&c| c == reference)
        {
            self.nodes[parent].children.insert(pos + 1, node);
        }
    }

    pub fn unlink(&mut self, node: usize) {
        if let Some(parent) = self.nodes[node].parent.take() {
            if let Some(pos) = self.nodes[parent].children.iter().position(|&c| c == node) {
                self.nodes[parent].children.remove(pos);
            }
        }
    }

    pub fn attr(&self, node: usize, key: &str) -> String {
        for (k, v) in &self.nodes[node].attrs {
            if k == key {
                return v.clone();
            }
        }
        String::new()
    }

    pub fn has_attr(&self, node: usize, key: &str) -> bool {
        self.nodes[node].attrs.iter().any(|(k, _)| k == key)
    }

    pub fn set_attr(&mut self, node: usize, key: &str, value: &str) {
        for (k, v) in &mut self.nodes[node].attrs {
            if k == key {
                *v = value.to_string();
                return;
            }
        }
        self.nodes[node]
            .attrs
            .push((key.to_string(), value.to_string()));
    }

    pub fn remove_attr(&mut self, node: usize, key: &str) {
        self.nodes[node].attrs.retain(|(k, _)| k != key);
    }

    pub fn parent_is(&self, node: usize, tags: &[&str]) -> bool {
        let mut cur = self.nodes[node].parent;
        while let Some(p) = cur {
            if self.nodes[p].typ == DomType::Element && tags.contains(&self.nodes[p].data.as_str())
            {
                return true;
            }
            cur = self.nodes[p].parent;
        }
        false
    }

    /// 对应 Go 版 util.DomText。
    pub fn text(&self, node: usize) -> String {
        if self.nodes[node].typ == DomType::Text {
            return self.nodes[node].data.clone();
        }
        let mut buf = String::new();
        // DomText starts at the requested node's children. Filtering its own
        // data-render marker would erase heading-marker contents in Vditor IR.
        for &child in &self.nodes[node].children {
            self.text0(child, &mut buf);
        }
        buf
    }

    fn text0(&self, node: usize, buf: &mut String) {
        let n = &self.nodes[node];
        if n.typ == DomType::Comment {
            return;
        }
        let data_render = self.attr(node, "data-render");
        if "1" == data_render || "2" == data_render {
            return;
        }
        match n.typ {
            DomType::Text => buf.push_str(&n.data),
            DomType::Element => match n.data.as_str() {
                "br" => buf.push('\n'),
                "p" => buf.push_str("\n\n"),
                _ => {}
            },
            _ => {}
        }
        for &c in &n.children {
            self.text0(c, buf);
        }
    }

    /// 对应 Go 版 util.DomHTML（html.Render）。
    pub fn html(&self, node: usize) -> String {
        let mut buf = String::new();
        self.render_html(node, &mut buf);
        buf.replace(crate::editor::ZWSP, "")
    }

    fn render_html(&self, node: usize, buf: &mut String) {
        let n = &self.nodes[node];
        match n.typ {
            DomType::Text => {
                if n.parent.is_some_and(|parent| {
                    matches!(
                        self.nodes[parent].data.as_str(),
                        "iframe"
                            | "noembed"
                            | "noframes"
                            | "noscript"
                            | "plaintext"
                            | "script"
                            | "style"
                            | "xmp"
                    )
                }) {
                    buf.push_str(&n.data);
                } else {
                    buf.push_str(&escape_dom(&n.data));
                }
            }
            DomType::Comment => {
                buf.push_str("<!--");
                buf.push_str(&n.data);
                buf.push_str("-->");
            }
            DomType::Doctype => {
                buf.push_str("<!DOCTYPE ");
                buf.push_str(&n.data);
                buf.push('>');
            }
            DomType::Element => {
                buf.push('<');
                buf.push_str(&n.data);
                for (k, v) in &n.attrs {
                    buf.push(' ');
                    buf.push_str(k);
                    buf.push_str("=\"");
                    buf.push_str(&escape_dom(v));
                    buf.push('"');
                }
                if VOID_TAGS.contains(&n.data.as_str()) {
                    // 对应 Go 版 html.Render：空元素写作 <hr/>，不带结束标签。
                    buf.push_str("/>");
                    return;
                }
                buf.push('>');
                for &c in &n.children {
                    self.render_html(c, buf);
                }
                buf.push_str("</");
                buf.push_str(&n.data);
                buf.push('>');
            }
            DomType::Document => {
                for &c in &n.children {
                    self.render_html(c, buf);
                }
            }
        }
    }

    /// 递归收集指定标签的后代节点。
    pub fn children_by_tag(&self, node: usize, tag: &str) -> Vec<usize> {
        let mut ret = Vec::new();
        self.collect_by_tag(node, tag, &mut ret);
        ret
    }

    fn collect_by_tag(&self, node: usize, tag: &str, ret: &mut Vec<usize>) {
        for &c in &self.nodes[node].children {
            if self.nodes[c].typ == DomType::Element && self.nodes[c].data == tag {
                ret.push(c);
            }
            self.collect_by_tag(c, tag, ret);
        }
    }

    /// 直接子节点中第一个指定标签。
    pub fn child_by_tag(&self, node: usize, tag: &str) -> Option<usize> {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .find(|&c| self.nodes[c].typ == DomType::Element && self.nodes[c].data == tag)
    }

    /// 对应 Go 版 Lute.domChild：深度优先查找后代元素（不含 node 自身）。
    pub fn descendant_by_tag(&self, node: usize, tag: &str) -> Option<usize> {
        for &c in &self.nodes[node].children {
            if let Some(found) = self.descendant_by_tag0(c, tag) {
                return Some(found);
            }
        }
        None
    }

    fn descendant_by_tag0(&self, node: usize, tag: &str) -> Option<usize> {
        if DomType::Element == self.nodes[node].typ && self.nodes[node].data == tag {
            return Some(node);
        }
        for &c in &self.nodes[node].children {
            if let Some(found) = self.descendant_by_tag0(c, tag) {
                return Some(found);
            }
        }
        None
    }

    pub fn prev_sibling(&self, node: usize) -> Option<usize> {
        let parent = self.nodes[node].parent?;
        let children = &self.nodes[parent].children;
        let pos = children.iter().position(|&c| c == node)?;
        if 0 == pos {
            None
        } else {
            Some(children[pos - 1])
        }
    }

    pub fn next_sibling(&self, node: usize) -> Option<usize> {
        let parent = self.nodes[node].parent?;
        let children = &self.nodes[parent].children;
        let pos = children.iter().position(|&c| c == node)?;
        children.get(pos + 1).copied()
    }

    pub fn is_element(&self, node: usize, tag: &str) -> bool {
        self.nodes[node].typ == DomType::Element && self.nodes[node].data == tag
    }

    /// 直接子节点中指定标签的全部节点，对应 Go 版按 FirstChild 遍历的写法。
    pub fn direct_children_by_tag(&self, node: usize, tag: &str) -> Vec<usize> {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .filter(|&c| self.nodes[c].typ == DomType::Element && self.nodes[c].data == tag)
            .collect()
    }

    /// 递归查找第一个指定标签的后代（包含自身），对应 Go 版 DomChildByTypeAndClass。
    pub fn find_descendant_by_tag_and_class(
        &self,
        node: usize,
        tag: &str,
        class: &str,
    ) -> Option<usize> {
        if self.nodes[node].typ == DomType::Element
            && self.nodes[node].data == tag
            && self.attr(node, "class").contains(class)
        {
            return Some(node);
        }
        for &c in &self.nodes[node].children {
            if let Some(ret) = self.find_descendant_by_tag_and_class(c, tag, class) {
                return Some(ret);
            }
        }
        None
    }

    /// 返回节点上以指定前缀开头的属性值。
    pub fn attr_values_with_prefix(&self, node: usize, prefix: &str) -> Vec<String> {
        self.nodes[node]
            .attrs
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// 对应 Go 版 Lute.adjustVditorDOMListItemInP：将 li 下的连续行级节点用 p 包裹。
    pub fn adjust_li_paragraphs(&mut self) {
        let lis: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.typ == DomType::Element && n.data == "li")
            .map(|(i, _)| i)
            .collect();
        for li in lis {
            let kids = self.nodes[li].children.clone();
            let mut new_kids: Vec<usize> = Vec::with_capacity(kids.len());
            let mut pending: Vec<usize> = Vec::new();
            for c in kids {
                let block = self.nodes[c].typ == DomType::Element
                    && matches!(
                        self.nodes[c].data.as_str(),
                        "p" | "blockquote" | "ul" | "ol" | "div" | "table"
                    );
                if block {
                    if !pending.is_empty() {
                        let p = self.create(DomType::Element, "p");
                        self.nodes[p].parent = Some(li);
                        for &s in &pending {
                            self.nodes[s].parent = Some(p);
                            self.nodes[p].children.push(s);
                        }
                        new_kids.push(p);
                        pending.clear();
                    }
                    new_kids.push(c);
                } else {
                    pending.push(c);
                }
            }
            if !pending.is_empty() {
                let p = self.create(DomType::Element, "p");
                self.nodes[p].parent = Some(li);
                for &s in &pending {
                    self.nodes[s].parent = Some(p);
                    self.nodes[p].children.push(s);
                }
                new_kids.push(p);
            }
            self.nodes[li].children = new_kids;
        }
    }
}

/// Corresponds to Go tabTitleMarkdown: protect title boundaries while converting
/// inline DOM, then preserve ASCII boundary spaces as Markdown entities.
pub(crate) fn tab_title_markdown(
    dom: &Dom,
    title: usize,
    convert: impl FnOnce(&str) -> String,
) -> String {
    let mut html = String::from("LuteTabTitleStart");
    for &child in &dom.nodes[title].children {
        html.push_str(&dom.html(child));
    }
    html.push_str("LuteTabTitleEnd");
    let markdown = convert(&html);
    let title = markdown.trim();
    let title = title.strip_prefix("LuteTabTitleStart").unwrap_or(title);
    let title = title.strip_suffix("LuteTabTitleEnd").unwrap_or(title);
    let leading = title.len() - title.trim_start_matches(' ').len();
    if leading == title.len() {
        return "&#32;".repeat(leading);
    }
    let trailing = title.len() - title.trim_end_matches(' ').len();
    format!(
        "{}{}{}",
        "&#32;".repeat(leading),
        &title[leading..title.len() - trailing],
        "&#32;".repeat(trailing)
    )
}

// html.Render uses the same escaping for TextNode data and attribute values.
fn escape_dom(s: &str) -> String {
    crate::html::escape_string(s).replace('\r', "&#13;")
}

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

include!("html_atoms.rs");

/// 对应 Go 版 html/atom 表：判断是否为已知 HTML 元素。
/// 未知标签在 Go 的 genASTContenteditable 中会退化为文本 `<tag>`。
pub fn is_known_tag(tag: &str) -> bool {
    KNOWN_ATOMS.binary_search(&tag).is_ok()
}

const RAW_TEXT_TAGS: &[&str] = &["script", "style", "noscript", "textarea", "title"];

const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
    "details",
];

/// 解析 HTML 字符串，对应 Go 版 util.ParseHTML。
pub fn parse(html: &str) -> Dom {
    let mut dom = Dom::new();
    let root = dom.root;
    let mut stack: Vec<usize> = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if b'<' != bytes[i] {
            let start = i;
            while i < bytes.len() && b'<' != bytes[i] {
                i += 1;
            }
            let raw = String::from_utf8_lossy(&bytes[start..i]);
            let text = crate::html::unescape_string(&raw);
            let node = dom.create(DomType::Text, text);
            let parent = *stack.last().unwrap_or(&root);
            dom.append_child(parent, node);
            continue;
        }

        // 注释
        if bytes[i..].starts_with(b"<!--") {
            let start = i + 4;
            let mut end = bytes.len();
            let mut j = start;
            while j + 2 < bytes.len() {
                if &bytes[j..j + 3] == b"-->" {
                    end = j;
                    j += 3;
                    break;
                }
                j += 1;
            }
            let data = String::from_utf8_lossy(&bytes[start..end]).to_string();
            let node = dom.create(DomType::Comment, data);
            let parent = *stack.last().unwrap_or(&root);
            dom.append_child(parent, node);
            i = if end < bytes.len() { j } else { bytes.len() };
            continue;
        }

        // DOCTYPE
        if bytes[i..].starts_with(b"<!") {
            let start = i + 2;
            let mut end = start;
            while end < bytes.len() && b'>' != bytes[end] {
                end += 1;
            }
            let data = String::from_utf8_lossy(&bytes[start..end])
                .trim()
                .to_string();
            let node = dom.create(DomType::Doctype, data);
            dom.append_child(root, node);
            i = if end < bytes.len() {
                end + 1
            } else {
                bytes.len()
            };
            continue;
        }

        // 结束标签
        if bytes[i..].starts_with(b"</") {
            let mut j = i + 2;
            while j < bytes.len() && b'>' != bytes[j] {
                j += 1;
            }
            let name = String::from_utf8_lossy(&bytes[i + 2..j])
                .trim()
                .to_ascii_lowercase();
            if let Some(pos) = stack
                .iter()
                .rposition(|&n| dom.nodes[n].typ == DomType::Element && dom.nodes[n].data == name)
            {
                stack.truncate(pos);
            }
            i = if j < bytes.len() { j + 1 } else { bytes.len() };
            continue;
        }

        // 开始标签
        let mut j = i + 1;
        let mut name = String::new();
        while j < bytes.len() && !is_tag_name_end(bytes[j]) {
            name.push(bytes[j] as char);
            j += 1;
        }
        let name = name.to_ascii_lowercase();
        if name.is_empty() {
            // 单独的 <
            let node = dom.create(DomType::Text, "<");
            let parent = *stack.last().unwrap_or(&root);
            dom.append_child(parent, node);
            i += 1;
            continue;
        }

        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            if b'>' == bytes[j] {
                j += 1;
                break;
            }
            if b'/' == bytes[j] {
                if j + 1 < bytes.len() && b'>' == bytes[j + 1] {
                    self_closing = true;
                    j += 2;
                    break;
                }
                j += 1;
                continue;
            }
            let astart = j;
            while j < bytes.len()
                && !(bytes[j] as char).is_whitespace()
                && !matches!(bytes[j], b'=' | b'>' | b'/')
            {
                j += 1;
            }
            let key = String::from_utf8_lossy(&bytes[astart..j]).to_ascii_lowercase();
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            let mut val = String::new();
            if j < bytes.len() && b'=' == bytes[j] {
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && (b'"' == bytes[j] || b'\'' == bytes[j]) {
                    let quote = bytes[j];
                    j += 1;
                    let vstart = j;
                    while j < bytes.len() && quote != bytes[j] {
                        j += 1;
                    }
                    val = String::from_utf8_lossy(&bytes[vstart..j]).to_string();
                    if j < bytes.len() {
                        j += 1;
                    }
                } else {
                    let vstart = j;
                    while j < bytes.len() && !(bytes[j] as char).is_whitespace() && b'>' != bytes[j]
                    {
                        j += 1;
                    }
                    val = String::from_utf8_lossy(&bytes[vstart..j]).to_string();
                }
            }
            if !key.is_empty() {
                attrs.push((key, crate::html::unescape_string(&val)));
            }
        }

        // 隐式闭合
        apply_implied_close(&dom, &mut stack, &name);

        let node = dom.create(DomType::Element, name.clone());
        dom.nodes[node].attrs = attrs;
        let parent = *stack.last().unwrap_or(&root);
        dom.append_child(parent, node);

        if VOID_TAGS.contains(&name.as_str()) || self_closing {
            i = j;
            continue;
        }

        // 原始文本元素
        if RAW_TEXT_TAGS.contains(&name.as_str()) {
            let close = format!("</{name}");
            let rest = String::from_utf8_lossy(&bytes[j..]).to_string();
            let lower = rest.to_ascii_lowercase();
            let end = lower.match_indices(&close).find_map(|(pos, _)| {
                lower
                    .as_bytes()
                    .get(pos + close.len())
                    .filter(|&&next| is_tag_name_end(next))
                    .map(|_| pos)
            });
            let raw = &rest[..end.unwrap_or(rest.len())];
            let text = if matches!(name.as_str(), "script" | "style" | "noscript") {
                raw.to_string()
            } else {
                crate::html::unescape_string(raw)
            };
            if !text.is_empty() {
                let tnode = dom.create(DomType::Text, text);
                dom.append_child(node, tnode);
            }
            i = if let Some(pos) = end {
                let after = j + pos;
                let mut k = after;
                while k < bytes.len() && b'>' != bytes[k] {
                    k += 1;
                }
                if k < bytes.len() {
                    k + 1
                } else {
                    bytes.len()
                }
            } else {
                bytes.len()
            };
            continue;
        }

        stack.push(node);
        i = j;
    }

    dom
}

fn is_tag_name_end(b: u8) -> bool {
    (b as char).is_whitespace() || b'>' == b || b'/' == b
}

fn apply_implied_close(dom: &Dom, stack: &mut Vec<usize>, new_tag: &str) {
    let top = match stack.last() {
        Some(&t) => t,
        None => return,
    };
    let top_tag = dom.nodes[top].data.as_str();

    let close_p = BLOCK_TAGS.contains(&new_tag) && "p" == top_tag;
    if close_p {
        stack.pop();
        return;
    }
    if "li" == new_tag && "li" == top_tag {
        stack.pop();
        return;
    }
    if ("dt" == new_tag || "dd" == new_tag) && ("dt" == top_tag || "dd" == top_tag) {
        stack.pop();
        return;
    }
    if "tr" == new_tag && ("td" == top_tag || "th" == top_tag) {
        stack.pop();
        return apply_implied_close(dom, stack, new_tag);
    }
    if "tr" == new_tag && "tr" == top_tag {
        stack.pop();
        return;
    }
    if ("td" == new_tag || "th" == new_tag) && ("td" == top_tag || "th" == top_tag) {
        stack.pop();
        return;
    }
    if ("thead" == new_tag || "tbody" == new_tag || "tfoot" == new_tag)
        && matches!(top_tag, "thead" | "tbody" | "tfoot" | "tr" | "td" | "th")
    {
        stack.pop();
        return apply_implied_close(dom, stack, new_tag);
    }
    if "option" == new_tag && "option" == top_tag {
        stack.pop();
        return;
    }
    if "a" == new_tag {
        if let Some(pos) = stack
            .iter()
            .rposition(|&n| dom.nodes[n].typ == DomType::Element && dom.nodes[n].data == "a")
        {
            stack.truncate(pos);
        }
    }
}

#[cfg(test)]
mod raw_text_tests {
    use super::*;

    #[test]
    fn dom_text_filters_descendants_but_not_requested_root() {
        let dom = parse("<h2>foo<span data-render=\"2\">------</span></h2>");
        let heading = dom.children_by_tag(dom.root, "h2")[0];
        let marker = dom.children_by_tag(heading, "span")[0];
        assert_eq!(dom.text(heading), "foo");
        assert_eq!(dom.text(marker), "------");
        let dom = parse("<p>a<br>b</p>");
        let paragraph = dom.children_by_tag(dom.root, "p")[0];
        assert_eq!(dom.text(paragraph), "a\nb");
        assert_eq!(dom.text(dom.root), "\n\na\nb");
    }

    #[test]
    fn scripting_enabled_noscript_keeps_markup_as_raw_text() {
        let dom = parse("<figure><noscript><img src=\"x&amp;y\"></noscript><figcaption>caption</figcaption></figure>");
        let noscript = dom.children_by_tag(dom.root, "noscript")[0];
        let children = &dom.nodes[noscript].children;
        assert_eq!(children.len(), 1);
        assert_eq!(dom.nodes[children[0]].typ, DomType::Text);
        assert_eq!(dom.nodes[children[0]].data, "<img src=\"x&amp;y\">");
        assert_eq!(
            dom.html(noscript),
            "<noscript><img src=\"x&amp;y\"></noscript>"
        );
        assert_eq!(dom.children_by_tag(dom.root, "figcaption").len(), 1);
    }

    #[test]
    fn raw_text_eof_and_closing_tag_boundaries_preserve_input() {
        for tag in ["noscript", "script", "style"] {
            let dom = parse(&format!("<{tag}>a&amp;</{tag}x>b"));
            let raw = dom.nodes[dom.root].children[0];
            assert_eq!(
                dom.nodes[dom.nodes[raw].children[0]].data,
                format!("a&amp;</{tag}x>b")
            );
        }
        let dom = parse("<noscript>x</NoScRiPt ><p>after</p>");
        assert_eq!(dom.nodes[dom.root].children.len(), 2);
        let textarea = parse("<textarea>&amp;<b>x</b>");
        let raw = textarea.nodes[textarea.root].children[0];
        assert_eq!(textarea.text(raw), "&<b>x</b>");
        assert_eq!(
            textarea.html(raw),
            "<textarea>&amp;&lt;b&gt;x&lt;/b&gt;</textarea>"
        );
    }
}
