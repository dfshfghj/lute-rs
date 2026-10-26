// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML → Markdown，对应 Go 版 h2m.go 的 HTML2Tree / HTML2Markdown。

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::dom::{is_known_tag, Dom, DomType};
use crate::editor;
use crate::lex;
use crate::options::ParseOptions;

/// 对应 Go 版 Lute.HTML2Tree。
pub fn html2tree(dom_str: &str, options: ParseOptions) -> Tree {
    html2tree_with_render_options(dom_str, options, crate::options::RenderOptions::new())
}

pub(crate) fn html2tree_with_render_options(
    dom_str: &str,
    options: ParseOptions,
    render_options: crate::options::RenderOptions,
) -> Tree {
    let mut dom = crate::dom::parse(dom_str);
    // 与 Go 版 HTMLNode2Tree 一致：在通用展开前保存单元格块结构。
    if options.protyle_wysiwyg && !options.disable_table_cell_rich {
        crate::parse::table_rich::preserve_html_table_cell_structure(&mut dom);
    }
    // HTMLNode2Tree uses the same normalization as the editor conversion paths.
    fix_table_structure(&mut dom);
    crate::vditor::adjust_vditor_dom(&mut dom, &options);
    dom.adjust_li_paragraphs();
    let root = dom.root;
    let mut children = dom.nodes[root].children.clone();
    // 对应 Go 版 util.ParseHTML：返回 body 子树。首节点为注释/DOCTYPE 时 Go 返回整个文档。
    let skip_head = match children.first().copied() {
        Some(first) => {
            let typ = dom.nodes[first].typ;
            typ != DomType::Comment && typ != DomType::Doctype
        }
        None => false,
    };
    if skip_head {
        let mut i = 0;
        while i < children.len() {
            let node = &dom.nodes[children[i]];
            if node.typ == DomType::Text && node.data.trim().is_empty() {
                i += 1;
                continue;
            }
            if node.typ == DomType::Element
                && matches!(
                    node.data.as_str(),
                    "script" | "style" | "title" | "meta" | "link" | "base" | "noscript"
                )
            {
                i += 1;
                continue;
            }
            break;
        }
        children = children[i..].to_vec();
    }
    let mut builder = Builder {
        dom: &mut dom,
        tree: Tree::new(),
        tip: Node(std::ptr::null_mut()),
        options,
        render_options,
        source_node: None,
    };
    builder.tip = builder.tree.root_raw();
    for c in children {
        if builder.dom.nodes[c].parent == Some(root) {
            builder.gen(c);
        }
    }
    builder.finish()
}

/// 对应 Go 版 Lute.HTML2Markdown。
pub fn html2markdown(
    dom_str: &str,
    parse_options: ParseOptions,
    render_options: crate::options::RenderOptions,
) -> String {
    html2markdown_ext(dom_str, parse_options, render_options, None)
}

/// html2markdown 的扩展版本，支持用户自定义渲染器。
pub fn html2markdown_ext(
    dom_str: &str,
    parse_options: ParseOptions,
    render_options: crate::options::RenderOptions,
    ext: Option<crate::render::ExtRendererFunc>,
) -> String {
    let tree =
        html2tree_with_render_options(dom_str, parse_options.clone(), render_options.clone());
    let mut r = crate::render::format::FormatRenderer::new(&tree, render_options, parse_options);
    r.base.ext_renderer = ext;
    String::from_utf8_lossy(&r.render()).to_string()
}

struct Builder<'a> {
    // Go genASTByDOM allocates one primary node before dispatching the tag.
    source_node: Option<Node>,
    dom: &'a mut Dom,
    tree: Tree,
    tip: Node,
    options: ParseOptions,
    render_options: crate::options::RenderOptions,
}

impl<'a> Builder<'a> {
    fn finish(mut self) -> Tree {
        // HTMLNode2Tree repairs sibling lists produced by external HTML: ul.ul
        // belongs to the preceding li, rather than the outer ul directly.
        let mut moves = Vec::new();
        fn find_lists(node: Node, moves: &mut Vec<(Node, Node)>) {
            if node.typ() == NodeType::NodeList
                && node.parent().is_some_and(|p| p.typ() == NodeType::NodeList)
            {
                if let Some(previous) = node.previous() {
                    moves.push((node, previous));
                }
            }
            for child in node.children() {
                find_lists(child, moves);
            }
        }
        find_lists(self.tree.root_raw(), &mut moves);
        for (list, previous) in moves {
            previous.append_child(list);
        }
        let mut styled = false;
        crate::ast::walk_raw(self.tree.root_raw(), &mut |node, entering| {
            if entering
                && matches!(node.typ(), NodeType::NodeTextMark | NodeType::NodeCodeSpan)
                && !node.ial_attr("style").is_empty()
            {
                styled = true;
            }
            crate::ast::WalkStatus::WalkContinue
        });
        if styled {
            crate::parse::nested_inlines2flatted_spans_hybrid(&mut self.tree, false);
            let mut marks = Vec::new();
            crate::ast::walk_raw(self.tree.root_raw(), &mut |node, entering| {
                if entering && node.typ() == NodeType::NodeTextMark {
                    marks.push(node);
                }
                crate::ast::WalkStatus::WalkContinue
            });
            for node in marks {
                crate::parse::html_text_style::apply_html_text_style(
                    &mut self.tree,
                    node,
                    &node.ial_attr("style"),
                );
            }
        }
        self.tree
    }

    fn apply_html_style(&mut self, idx: usize, node: Node) {
        if !(crate::PROTYLE && self.options.protyle_wysiwyg) || node.parent().is_none() {
            return;
        }
        let style = crate::parse::html_text_style::html_node_text_style(self.dom, idx).css();
        if style.is_empty() {
            return;
        }
        if node.typ() == NodeType::NodeCodeSpan {
            node.set_ial_attr("style", &style);
            let ial = self.new_node_raw(NodeType::NodeKramdownSpanIAL);
            ial.set_tokens(crate::parse::ial::ial_to_tokens(&node.ial_raw()));
            node.insert_after(ial);
        } else {
            crate::parse::html_text_style::apply_html_text_style(&mut self.tree, node, &style);
            if node.typ() == NodeType::NodeTextMark {
                crate::parse::html_text_style::apply_html_text_semantics(node, self.dom, idx);
            }
        }
    }

    fn new_node_raw(&mut self, typ: NodeType) -> Node {
        self.tree.new_node_raw(typ)
    }

    fn text_node(&mut self, tokens: &str) -> Node {
        let n = self.new_node_raw(NodeType::NodeText);
        n.set_tokens(tokens.as_bytes().to_vec());
        n
    }

    fn gen(&mut self, idx: usize) {
        normalize_code_block_banner(self.dom, idx);
        let node_info = &self.dom.nodes[idx];
        if node_info.typ == DomType::Comment || node_info.typ == DomType::Doctype {
            return;
        }
        if node_info.typ == DomType::Text {
            self.gen_text(idx);
            return;
        }

        let tag = node_info.data.clone();
        if "meta" == tag {
            return;
        }
        if "svg" == tag {
            return;
        }
        let data_render = self.dom.attr(idx, "data-render");
        if "1" == data_render {
            return;
        }
        let class = self.dom.attr(idx, "class");
        if class.starts_with("line-number") && !class.starts_with("line-numbers") {
            return;
        }
        if class.contains("mw-editsection") || class.contains("citation-comment") {
            return;
        }

        if self.gen_tabs(idx, &class) {
            return;
        }
        if self.gen_callout(idx, &class) {
            return;
        }

        // 自定义标签：直接递归子节点
        if !is_known_tag(&tag) {
            let children = self.dom.nodes[idx].children.clone();
            for c in children {
                self.gen(c);
            }
            return;
        }

        let node = self.new_node_raw(NodeType::NodeText);
        node.set_tokens(tag.as_bytes().to_vec());
        if tag == "table" {
            self.set_table_caption(idx, node);
        }
        let mut with_ial = !node.ial_raw().is_empty();
        let mut with_dom_ial = false;
        for (key, value) in self.dom.nodes[idx].attrs.clone() {
            if self.options.html2_markdown_attrs.contains(&key)
                || (key.starts_with("custom-")
                    && self
                        .options
                        .html2_markdown_attrs
                        .iter()
                        .any(|a| a == "custom-*"))
            {
                node.set_ial_attr(&key, &value);
                with_ial = true;
                with_dom_ial = true;
            }
        }
        if with_dom_ial {
            self.set_dom_ial(idx, node);
        }
        // Capture tokens before dispatch, as the Go deferred block-IAL does.
        let ial_tokens = with_ial.then(|| crate::parse::ial::ial_to_tokens(&node.ial_raw()));
        let parent_node = self.source_node.replace(node);
        self.gen_element(idx, &tag, &class);
        self.source_node = parent_node;
        if let Some(tokens) = ial_tokens {
            let ial = self.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(tokens);
            self.tip.append_child(ial);
        }
    }

    fn primary_node(&mut self, typ: NodeType) -> Node {
        let node = self.source_node.expect("DOM element primary node");
        node.set_typ(typ);
        node
    }

    // Corresponds to setBlockIAL2, called only after an exported DOM attribute matches.
    fn set_dom_ial(&mut self, idx: usize, node: Node) {
        for key in [
            "bookmark",
            "style",
            "name",
            "memo",
            "alias",
            "fold",
            "heading-fold",
            "parent-fold",
        ] {
            let mut value = self.dom.attr(idx, key);
            if value.is_empty() {
                continue;
            }
            if matches!(key, "bookmark" | "style" | "name" | "memo" | "alias") {
                value = crate::html::unescape_string(&value);
            }
            if key == "style" {
                value = value
                    .trim()
                    .replace('\n', "")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            node.set_ial_attr(key, &value);
        }
        for (key, value) in self.dom.nodes[idx].attrs.clone() {
            if key.starts_with("custom-") {
                node.set_ial_attr(&key, &crate::html::unescape_string(&value));
            }
        }
    }

    fn gen_element(&mut self, idx: usize, tag: &str, class: &str) {
        match tag {
            "p" | "div" | "section" | "dt" | "dd" => self.gen_paragraph(idx, tag, class),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.gen_heading(idx, tag),
            "hr" => {
                let n = self.primary_node(NodeType::NodeThematicBreak);
                self.tip.append_child(n);
            }
            "blockquote" => {
                let n = self.primary_node(NodeType::NodeBlockquote);
                n.set_tokens(tag.as_bytes().to_vec());
                let marker = self.new_node_raw(NodeType::NodeBlockquoteMarker);
                marker.set_tokens(b">".to_vec());
                n.append_child(marker);
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            "ol" | "ul" => {
                let n = self.primary_node(NodeType::NodeList);
                n.set_tokens(tag.as_bytes().to_vec());
                let mut ld = crate::ast::ListData::default();
                if "ol" == tag {
                    ld.typ = 1;
                }
                ld.tight = true;
                n.data_mut().list_data = Some(ld);
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            "li" => self.gen_list_item(idx),
            "pre" => self.gen_pre(idx),
            "em" | "i" => self.gen_em(idx),
            "strong" | "b" => self.gen_strong(idx),
            "code" => self.gen_code(idx),
            "br" => {
                if NodeType::NodeLink == self.tip.typ() || NodeType::NodeHeading == self.tip.typ() {
                    return;
                }
                if self.dom.next_sibling(idx).is_none() {
                    return;
                }
                let n = self.primary_node(NodeType::NodeHardBreak);
                n.set_tokens(b"\n".to_vec());
                self.tip.append_child(n);
            }
            "a" => self.gen_link(idx),
            "img" => self.gen_img(idx),
            "input" => {
                let n = self.primary_node(NodeType::NodeTaskListItemMarker);
                let data_task = self.dom.attr(idx, "data-task");
                let checked = self.dom.has_attr(idx, "checked");
                let marker = if 1 == data_task.len() {
                    data_task.as_bytes()[0]
                } else if checked {
                    b'X'
                } else {
                    b' '
                };
                n.data_mut().task_list_item_marker = marker;
                n.data_mut().task_list_item_checked = marker != b' ' && marker != 0;
                self.tip.append_child(n);
                if let Some(parent) = self.tip.parent() {
                    match parent.data_mut().list_data.as_mut() {
                        Some(data) => data.typ = 3,
                        None => {
                            parent.data_mut().list_data = Some(crate::ast::ListData {
                                typ: 3,
                                ..Default::default()
                            })
                        }
                    }
                }
            }
            "del" | "s" | "strike" => {
                if !self.options.gfm_strikethrough && !self.options.html_tag2_text_mark {
                    self.gen_html_tag(idx, "s");
                } else {
                    let n = self.primary_node(NodeType::NodeStrikethrough);
                    n.set_tokens(tag.as_bytes().to_vec());
                    let m = self.new_node_raw(NodeType::NodeStrikethrough2OpenMarker);
                    m.set_tokens(b"~~".to_vec());
                    n.append_child(m);
                    self.tip.append_child(n);
                    self.tip = n;
                    self.gen_children(idx);
                    let close = self.new_node_raw(NodeType::NodeStrikethrough2CloseMarker);
                    close.set_tokens(b"~~".to_vec());
                    self.tip.append_child(close);
                    self.append_space(idx);
                    self.leave();
                }
            }
            "u" => {
                if crate::PROTYLE && self.options.protyle_wysiwyg {
                    let n = self.primary_node(NodeType::NodeUnderline);
                    n.set_tokens(tag.as_bytes().to_vec());
                    let m = self.new_node_raw(NodeType::NodeUnderlineOpenMarker);
                    n.append_child(m);
                    self.tip.append_child(n);
                    self.tip = n;
                    self.gen_children(idx);
                    let close = self.new_node_raw(NodeType::NodeUnderlineCloseMarker);
                    self.tip.append_child(close);
                    self.leave();
                } else {
                    self.gen_html_tag(idx, "u");
                }
            }
            "mark" => {
                if !self.options.mark && !self.options.html_tag2_text_mark {
                    self.gen_html_tag(idx, "mark");
                } else {
                    let n = self.primary_node(NodeType::NodeMark);
                    n.set_tokens(tag.as_bytes().to_vec());
                    let m = self.new_node_raw(NodeType::NodeMark2OpenMarker);
                    m.set_tokens(b"==".to_vec());
                    n.append_child(m);
                    self.tip.append_child(n);
                    self.tip = n;
                    self.gen_children(idx);
                    let close = self.new_node_raw(NodeType::NodeMark2CloseMarker);
                    close.set_tokens(b"==".to_vec());
                    self.tip.append_child(close);
                    self.append_space(idx);
                    self.leave();
                }
            }
            "sup" => {
                if !self.options.sup && !self.options.html_tag2_text_mark {
                    self.gen_html_tag(idx, "sup");
                } else {
                    let n = self.primary_node(NodeType::NodeSup);
                    n.set_tokens(tag.as_bytes().to_vec());
                    let m = self.new_node_raw(NodeType::NodeSupOpenMarker);
                    n.append_child(m);
                    self.tip.append_child(n);
                    self.tip = n;
                    self.gen_children(idx);
                    let close = self.new_node_raw(NodeType::NodeSupCloseMarker);
                    self.tip.append_child(close);
                    self.append_space(idx);
                    self.leave();
                }
            }
            "sub" => {
                if !self.options.sub && !self.options.html_tag2_text_mark {
                    self.gen_html_tag(idx, "sub");
                } else {
                    let n = self.primary_node(NodeType::NodeSub);
                    n.set_tokens(tag.as_bytes().to_vec());
                    let m = self.new_node_raw(NodeType::NodeSubOpenMarker);
                    n.append_child(m);
                    self.tip.append_child(n);
                    self.tip = n;
                    self.gen_children(idx);
                    let close = self.new_node_raw(NodeType::NodeSubCloseMarker);
                    self.tip.append_child(close);
                    self.append_space(idx);
                    self.leave();
                }
            }
            "table" => self.gen_table(idx),
            "thead" => {
                if self.dom.nodes[idx].children.is_empty() {
                    return;
                }
                let n = self.primary_node(NodeType::NodeTableHead);
                n.set_tokens(b"thead".to_vec());
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            "tbody" => {
                self.gen_children(idx);
            }
            "tr" => {
                if self.dom.nodes[idx].children.is_empty() {
                    return;
                }
                let n = self.primary_node(NodeType::NodeTableRow);
                n.set_tokens(b"tr".to_vec());
                let thead_count = self.dom.nodes[idx]
                    .parent
                    .and_then(|p| self.dom.nodes[p].parent)
                    .map(|t| self.dom.children_by_tag(t, "thead").len())
                    .unwrap_or(0);
                if self.tip.child_by_type(NodeType::NodeTableHead).is_none() && thead_count < 1 {
                    let n = self.new_node_raw(NodeType::NodeTableHead);
                    self.tip.append_child(n);
                    self.tip = n;
                    let row = self.primary_node(NodeType::NodeTableRow);
                    row.set_tokens(b"tr".to_vec());
                    self.tip.append_child(row);
                    self.tip = row;
                    self.gen_children(idx);
                    self.leave();
                    self.leave();
                    return;
                }
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            "th" | "td" => {
                let n = self.primary_node(NodeType::NodeTableCell);
                n.set_tokens(tag.as_bytes().to_vec());
                let align = self.dom.attr(idx, "align");
                let style = self.dom.attr(idx, "style");
                n.data_mut().table_cell_align = table_cell_align_value(&align, &style);
                self.tip.append_child(n);
                self.set_table_cell_span_ial(n, idx);
                self.tip = n;
                // 富文本单元格直接使用载荷投影，不把内部块暴露为文档块。
                if self
                    .dom
                    .has_attr(idx, crate::ast::TABLE_CELL_RICH_ATTRIBUTE)
                {
                    let encoded = self.dom.attr(idx, crate::ast::TABLE_CELL_RICH_ATTRIBUTE);
                    n.data_mut().table_cell_rich =
                        Some(crate::ast::decode_table_cell_rich(&encoded));
                    let _ = crate::parse::table_rich::apply_table_cell_rich_projection(n);
                    self.leave();
                    return;
                }
                self.gen_children(idx);
                self.leave();
            }
            "colgroup" | "col" => {}
            "span" | "small" => self.gen_span(idx),
            "kbd" => {
                if let Some(last) = self.tip.last_child() {
                    if NodeType::NodeKbd == last.typ() {
                        let zwsp = self.text_node(editor::ZWSP);
                        self.tip.append_child(zwsp);
                    }
                }
                let kbd = self.primary_node(NodeType::NodeKbd);
                kbd.append_child(self.new_node_raw(NodeType::NodeKbdOpenMarker));
                let text = self.dom.text(idx);
                kbd.append_child(self.text_node(&text));
                kbd.append_child(self.new_node_raw(NodeType::NodeKbdCloseMarker));
                self.tip.append_child(kbd);
            }
            "font" => {
                if crate::PROTYLE && self.options.protyle_wysiwyg {
                    self.gen_children(idx);
                    return;
                }
                let n = self.primary_node(NodeType::NodeText);
                let mut tokens = self.dom.text(idx);
                while tokens.contains("\n\n") {
                    tokens = tokens.replace("\n\n", "\n");
                }
                while tokens.contains("\n  ") {
                    tokens = tokens.replace("\n  ", "\n ");
                }
                tokens = tokens.replace("\n ", "\n");
                tokens = tokens.replace('\n', " ");
                n.set_tokens(tokens.into_bytes());
                self.tip.append_child(n);
            }
            "details" => {
                let n = self.primary_node(NodeType::NodeList);
                let ld = crate::ast::ListData {
                    tight: true,
                    ..Default::default()
                };
                n.data_mut().list_data = Some(ld);
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
                self.leave();
                self.leave();
            }
            "summary" => {
                if NodeType::NodeList != self.tip.typ() {
                    return;
                }
                let li = self.new_node_raw(NodeType::NodeListItem);
                let ld = crate::ast::ListData {
                    marker: b"*".to_vec(),
                    bullet_char: b'*',
                    ..Default::default()
                };
                li.data_mut().list_data = Some(ld);
                let p = self.primary_node(NodeType::NodeParagraph);
                li.append_child(p);
                self.tip.append_child(li);
                self.tip = p;
                self.gen_children(idx);
                self.leave();
            }
            "iframe" | "audio" | "video" => {
                let n = self.primary_node(if tag == "video" {
                    NodeType::NodeText
                } else {
                    NodeType::NodeHTMLBlock
                });
                if tag == "video" {
                    if let Some(source) = self.dom.children_by_tag(idx, "source").first().copied() {
                        let src = self.dom.attr(source, "src");
                        self.dom.unlink(source);
                        self.dom.nodes[idx].attrs.clear();
                        self.dom.set_attr(idx, "src", &src);
                        self.dom.set_attr(idx, "controls", "controls");
                    }
                }
                n.set_tokens(self.dom.html(idx).into_bytes());
                self.tip.append_child(n);
            }
            "noscript" => {}
            "script" => {
                let text = self.dom.text(idx);
                if !text.is_empty() {
                    if self.tip.is_container_block()
                        || self.dom.attr(idx, "type").contains("mode=display")
                        || self.dom.nodes[idx].parent.is_some_and(|p| {
                            self.dom.attr(p, "class").contains("math display")
                                && self.dom.nodes[p].children.last() == Some(&idx)
                        })
                    {
                        self.append_math_block(&text);
                    } else {
                        self.append_inline_math(&text);
                    }
                }
            }
            "figcaption" => {
                if self.tip.is_block() {
                    if let Some(last) = self.tip.last_child() {
                        if NodeType::NodeImage == last.typ() {
                            if let Some(close) = last.child_by_type(NodeType::NodeCloseParen) {
                                let title = self.dom.text(idx).trim().to_string();
                                if !title.is_empty() && "null" != title && "undefined" != title {
                                    let sp = self.new_node_raw(NodeType::NodeLinkSpace);
                                    close.insert_before(sp);
                                    let t = self.new_node_raw(NodeType::NodeLinkTitle);
                                    t.set_tokens(title.into_bytes());
                                    close.insert_before(t);
                                    return;
                                }
                            }
                        }
                    }
                    if NodeType::NodeDocument != self.tip.typ() {
                        let br = self.new_node_raw(NodeType::NodeHardBreak);
                        self.tip.append_child(br);
                        self.gen_children(idx);
                        return;
                    }
                }
                let n = self.primary_node(NodeType::NodeParagraph);
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            "figure" | "picture" => {
                if !self.tip.is_block() {
                    self.gen_children(idx);
                    return;
                }
                let n = self.primary_node(NodeType::NodeParagraph);
                self.tip.append_child(n);
                self.tip = n;
                self.gen_children(idx);
                self.leave();
            }
            _ => {
                self.gen_children(idx);
            }
        }
    }

    fn gen_callout(&mut self, idx: usize, class: &str) -> bool {
        if !self.options.callout
            || !self.dom.is_element(idx, "div")
            || !class.split_whitespace().any(|name| name == "callout")
        {
            return false;
        }
        let typ = self.dom.attr(idx, "data-subtype").trim().to_string();
        if typ.is_empty() || typ.contains([']', '\r', '\n']) {
            return false;
        }
        let Some(info) = self.direct_child_by_class(idx, "callout-info") else {
            return false;
        };
        let Some(content) = self.direct_child_by_class(idx, "callout-content") else {
            return false;
        };
        let callout = self.new_node_raw(NodeType::NodeCallout);
        callout.data_mut().callout_type = typ;
        if let Some(icon) = self.direct_child_by_class(info, "callout-icon") {
            if let Some(image) = self.dom.children_by_tag(icon, "img").first().copied() {
                let src = self.dom.attr(image, "src").trim().to_string();
                if crate::parse::callout::is_valid_callout_image_src(&src) {
                    callout.data_mut().callout_icon = src;
                    callout.data_mut().callout_icon_type = 1;
                }
            } else {
                callout.data_mut().callout_icon = self.dom.text(icon).trim().to_string();
            }
        }
        if let Some(title) = self.direct_child_by_class(info, "callout-title") {
            callout.data_mut().callout_title = html2markdown(
                &self.dom.html(title),
                self.options.clone(),
                self.render_options.clone(),
            )
            .trim()
            .to_string();
            callout.data_mut().callout_title_explicit = true;
        }
        self.tip.append_child(callout);
        self.tip = callout;
        self.gen_children(content);
        self.leave();
        true
    }

    fn direct_child_by_class(&self, idx: usize, class: &str) -> Option<usize> {
        self.dom.nodes[idx].children.iter().copied().find(|&child| {
            self.dom
                .attr(child, "class")
                .split_whitespace()
                .any(|name| name == class)
        })
    }

    /// Corresponds to genASTByTabsDOM in protyle_tabs.go. Standard HTML body
    /// children use this builder, while an editable title block uses BlockDOM.
    fn gen_tabs(&mut self, idx: usize, class: &str) -> bool {
        let is_item = class.split_whitespace().any(|c| c == "tab-item");
        if !self.options.tabs || (!is_item && !class.split_whitespace().any(|c| c == "tabs")) {
            return false;
        }
        let node = self.new_node_raw(if is_item {
            NodeType::NodeTabItem
        } else {
            NodeType::NodeTabs
        });
        if is_item {
            if let Some(title) = self
                .direct_child_by_class(idx, "tab-item-info")
                .and_then(|info| self.direct_child_by_class(info, "tab-item-title"))
            {
                node.data_mut().tab_item_title =
                    crate::dom::tab_title_markdown(self.dom, title, |html| {
                        html2markdown(html, self.options.clone(), self.render_options.clone())
                    });
            }
        }
        for name in ["tabs-active-id", "tabs-position", "tabs-task"] {
            let value = self.dom.attr(idx, name);
            if !value.is_empty() {
                node.set_ial_attr(name, &value);
            }
        }
        let id = self.dom.attr(idx, "data-node-id");
        node.set_id(id.clone());
        if !id.is_empty() {
            node.set_ial_attr("id", &id);
        }
        self.tip.append_child(node);
        self.tip = node;
        if is_item {
            if let Some(info) = self.direct_child_by_class(idx, "tab-item-info") {
                if let Some(title) = self.dom.nodes[info]
                    .children
                    .iter()
                    .copied()
                    .find(|&child| self.dom.attr(child, "tabs-title") == "true")
                {
                    if crate::PROTYLE {
                        let html = self.dom.html(title);
                        let mut title_tree =
                            crate::blockdom::blockdom2tree(&html, self.options.clone());
                        let children: Vec<_> = title_tree.root_raw().children().collect();
                        for child in children {
                            self.tip.append_child(child);
                        }
                        self.tree.adopt(&mut title_tree);
                    }
                }
            }
            if let Some(content) = self.direct_child_by_class(idx, "tab-item-content") {
                self.gen_children(content);
            }
        } else {
            for child in self.dom.nodes[idx].children.clone() {
                let class = self.dom.attr(child, "class");
                if class.split_whitespace().any(|c| c == "tab-item") {
                    self.gen_tabs(child, &class);
                }
            }
        }
        self.leave();
        if !node.ial_raw().is_empty() {
            let ial = self.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(crate::parse::ial::ial_to_tokens(&node.ial_raw()));
            node.insert_after(ial);
        }
        true
    }

    fn gen_children(&mut self, idx: usize) {
        let children = self.dom.nodes[idx].children.clone();
        for c in children {
            if self.dom.nodes[c].parent == Some(idx) {
                self.gen(c);
            }
        }
    }

    fn leave(&mut self) {
        if let Some(p) = self.tip.parent() {
            self.tip = p;
        }
    }

    fn gen_text(&mut self, idx: usize) {
        let mut tokens = self.dom.nodes[idx].data.clone();
        if self
            .dom
            .prev_sibling(idx)
            .is_some_and(|previous| self.dom.attr(previous, "class") == "fn__space5")
        {
            return;
        }
        let parent = self.dom.nodes[idx].parent;

        while tokens.contains("\n  ") {
            tokens = tokens.replace("\n  ", "\n ");
        }
        tokens = tokens.replace("\n ", "\n");
        tokens = tokens.trim_matches(|c| c == '\t' || c == '\n').to_string();

        if self.dom.parent_is(idx, &["table"]) {
            if "\n" == self.dom.nodes[idx].data {
                if self.tip.first_child().is_none() || self.dom.next_sibling(idx).is_none() {
                    return;
                }
                let br = self.new_node_raw(NodeType::NodeBr);
                self.tip.append_child(br);
                return;
            }
            if self.dom.nodes[idx].data.trim().is_empty() {
                let n = self.text_node(" ");
                self.tip.append_child(n);
                return;
            }
            tokens = trim_table_cell_text(self.dom, idx, &tokens).replace('\n', " ");
        }
        tokens = tokens.replace('\u{a0}', " ");
        tokens = tokens.replace('\n', " ");

        if matches!(
            self.tip.typ(),
            NodeType::NodeStrong
                | NodeType::NodeEmphasis
                | NodeType::NodeStrikethrough
                | NodeType::NodeMark
                | NodeType::NodeSup
                | NodeType::NodeSub
        ) {
            if tokens.starts_with(' ') || tokens.trim().is_empty() {
                if let Some(last) = self.tip.last_child() {
                    if last.is_marker() {
                        tokens.insert_str(0, editor::ZWSP);
                    }
                }
            }
            if tokens.ends_with(' ') && self.dom.next_sibling(idx).is_none() {
                tokens.push_str(editor::ZWSP);
            }
        }

        if NodeType::NodeParagraph == self.tip.typ()
            && tokens.starts_with("$$")
            && tokens.ends_with("$$")
            && 2 == tokens.matches("$$").count()
        {
            let tex = tokens[2..tokens.len() - 2].trim().to_string();
            self.append_math_block(&tex);
            return;
        }

        // Match genASTByDOM: unadorned spans retain literal inline syntax;
        // other Protyle text is represented by backslash AST nodes rather than
        // embedding escapes into text tokens.
        let literal_span = parent
            .is_some_and(|p| self.dom.is_element(p, "span") && self.dom.nodes[p].attrs.is_empty());
        let text_style = if crate::PROTYLE && self.options.protyle_wysiwyg {
            crate::parse::html_text_style::html_node_text_style(self.dom, idx).css()
        } else {
            String::new()
        };
        let escape_markers = text_style.is_empty()
            && !literal_span
            && (crate::PROTYLE && self.options.protyle_wysiwyg)
            && self.tip.typ() != NodeType::NodeLink;
        if text_style.is_empty()
            && !literal_span
            && !(crate::PROTYLE && self.options.protyle_wysiwyg)
        {
            let escaped = lex::escape_common_markers(tokens.as_bytes());
            tokens = String::from_utf8_lossy(&escaped).to_string();
            if self.dom.parent_is(idx, &["table"]) {
                tokens = tokens.replace("\\|", "|").replace('|', "\\|");
            }
        }

        if tokens.is_empty() {
            return;
        }

        if NodeType::NodeListItem == self.tip.typ()
            || self.tip.parent().map(|p| p.typ()) == Some(NodeType::NodeListItem)
            || self.tip.parent().and_then(|p| p.parent()).map(|p| p.typ())
                == Some(NodeType::NodeListItem)
        {
            if let Some(pos) = tokens.find(". ") {
                if 0 < pos && tokens[..pos].bytes().all(|b| b.is_ascii_digit()) {
                    tokens = tokens[pos + 2..].to_string();
                }
            }
            let unordered = self
                .tip
                .data()
                .list_data
                .as_ref()
                .map(|d| 0 == d.typ)
                .unwrap_or(false)
                || self
                    .tip
                    .parent()
                    .and_then(|p| p.data().list_data.as_ref().map(|d| 0 == d.typ))
                    .unwrap_or(false)
                || self
                    .tip
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.data().list_data.as_ref().map(|d| 0 == d.typ))
                    .unwrap_or(false);
            if unordered {
                if let Some(rest) = tokens.strip_prefix('•') {
                    tokens = rest.trim().to_string();
                }
                if let Some(rest) = tokens.strip_prefix('●') {
                    tokens = rest.trim().to_string();
                }
            }
        }

        if NodeType::NodeListItem == self.tip.typ() {
            let p = self.new_node_raw(NodeType::NodeParagraph);
            let n = self.text_node(&tokens);
            p.append_child(n);
            self.tip.append_child(p);
            self.tip = p;
            if escape_markers {
                self.escape_html_text_markers(n);
            }
            self.apply_html_style(idx, n);
        } else if let Some(p) = parent {
            if self.dom.is_element(p, "dt") {
                let strong = self.new_node_raw(NodeType::NodeStrong);
                let open = self.new_node_raw(NodeType::NodeStrongA6kOpenMarker);
                open.set_tokens(b"**".to_vec());
                strong.append_child(open);
                let text = self.dom.text(idx);
                strong.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeStrongA6kCloseMarker);
                close.set_tokens(b"**".to_vec());
                strong.append_child(close);
                self.tip.append_child(strong);
                return;
            }
            let n = self.text_node(&tokens);
            if self.dom.is_element(p, "a") {
                n.set_typ(NodeType::NodeLinkText);
            }
            self.tip.append_child(n);
            if escape_markers {
                self.escape_html_text_markers(n);
            }
            self.apply_html_style(idx, n);
        } else {
            let n = self.text_node(&tokens);
            self.tip.append_child(n);
            if escape_markers {
                self.escape_html_text_markers(n);
            }
            self.apply_html_style(idx, n);
        }
    }

    /// h2m.go: escapeHTMLTextMarkers. Preserve each escape as an AST node so
    /// flattening text marks cannot turn marker escapes into literal backslashes.
    fn escape_html_text_markers(&mut self, node: Node) {
        let tokens = node.tokens_vec();
        let mut start = 0;
        for (i, &token) in tokens.iter().enumerate() {
            if !lex::is_protyle_inline_marker(token) {
                continue;
            }
            if start < i {
                let text = self.new_node_raw(node.typ());
                text.set_tokens(tokens[start..i].to_vec());
                node.insert_before(text);
            }
            let backslash = self.new_node_raw(NodeType::NodeBackslash);
            let content = self.new_node_raw(NodeType::NodeBackslashContent);
            content.set_tokens(vec![token]);
            backslash.append_child(content);
            node.insert_before(backslash);
            start = i + 1;
        }
        if start == 0 {
            return;
        }
        if start < tokens.len() {
            node.set_tokens(tokens[start..].to_vec());
        } else {
            node.unlink();
        }
    }

    fn gen_paragraph(&mut self, idx: usize, tag: &str, class: &str) {
        if NodeType::NodeLink == self.tip.typ() {
            self.gen_children(idx);
            return;
        }
        if self.dom.parent_is(idx, &["table"]) {
            if self
                .dom
                .prev_sibling(idx)
                .map(|p| self.dom.nodes[p].data.contains('\n'))
                .unwrap_or(false)
            {
                self.gen_children(idx);
                return;
            }
            if self
                .dom
                .next_sibling(idx)
                .map(|p| self.dom.nodes[p].data.contains('\n'))
                .unwrap_or(false)
            {
                self.gen_children(idx);
                return;
            }
            if self.tip.first_child().is_none() {
                self.gen_children(idx);
                return;
            }
            let br = self.new_node_raw(NodeType::NodeBr);
            self.tip.append_child(br);
            self.gen_children(idx);
            return;
        }
        if NodeType::NodeHeading == self.tip.typ() {
            self.gen_children(idx);
            return;
        }

        if "div" == tag || "section" == tag {
            if "language-math" == class
                && self.dom.attr(idx, "data-math").is_empty()
                && self.dom.nodes[idx].children.len() == 1
                && self.dom.nodes[self.dom.nodes[idx].children[0]].typ == DomType::Text
            {
                let text = self.dom.text(idx);
                self.append_math_block(&text);
                return;
            }
            let mut language = String::new();
            if class.contains("-source-") {
                language = class[class.rfind("-source-").unwrap() + 8..].to_string();
            } else if class.contains("-text-html-basic") {
                language = "html".to_string();
            } else if class.contains("language-") && !class.contains("-math") {
                language = class[class.rfind("language-").unwrap() + 9..].to_string();
                if !matches!(
                    language.as_str(),
                    "mermaid"
                        | "echarts"
                        | "abc"
                        | "graphviz"
                        | "mindmap"
                        | "flowchart"
                        | "plantuml"
                        | "infographic"
                ) {
                    language.clear();
                }
            }
            if !language.is_empty() {
                let n = self.primary_node(NodeType::NodeCodeBlock);
                n.data_mut().is_fenced_code_block = true;
                let open = self.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                open.set_tokens(b"```".to_vec());
                open.data_mut().code_block_fence_len = 3;
                n.append_child(open);
                let info = self.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                info.data_mut().code_block_info = language.into_bytes();
                n.append_child(info);
                let mut tokens = self.dom.text(idx).replace('\u{a0}', " ");
                let suffix = format!("\n{}", editor::ZWSP);
                if tokens.ends_with(&suffix) {
                    tokens.truncate(tokens.len() - suffix.len());
                }
                let content = self.new_node_raw(NodeType::NodeCodeBlockCode);
                content.set_tokens(tokens.into_bytes());
                n.append_child(content);
                let close = self.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                close.set_tokens(b"```".to_vec());
                close.data_mut().code_block_fence_len = 3;
                n.append_child(close);
                self.tip.append_child(n);
                return;
            }
            if class.contains("MathJax") {
                if let Some(next) = self.dom.next_sibling(idx).filter(|&n| {
                    self.dom.is_element(n, "script")
                        && self.dom.attr(n, "type").contains("math/tex")
                }) {
                    let tex = self.dom.text(next);
                    self.append_math_block(&tex);
                    self.dom.unlink(next);
                    return;
                }
            }
            let tex = self.dom.attr(idx, "data-tex");
            if !tex.trim().is_empty() {
                self.append_math_block(tex.trim());
                return;
            }
        }
        if class.to_lowercase().contains("mathjax") {
            return;
        }

        if self.dom.text(idx).trim().is_empty() {
            let has_media = self.dom.children_by_tag(idx, "img").len()
                + self.dom.children_by_tag(idx, "picture").len()
                + self.dom.children_by_tag(idx, "iframe").len()
                + self.dom.children_by_tag(idx, "video").len()
                + self.dom.children_by_tag(idx, "audio").len()
                + self.dom.children_by_tag(idx, "source").len()
                + self.dom.children_by_tag(idx, "canvas").len()
                + self.dom.children_by_tag(idx, "svg").len()
                + self.dom.children_by_tag(idx, "math").len()
                > 0;
            if !has_media {
                if let Some(tex) = get_formula(self.dom, idx) {
                    self.append_math_block(&tex);
                    return;
                }
                let first_span = self.dom.children_by_tag(idx, "span").first().copied();
                if !first_span.is_some_and(|span| {
                    !self.dom.attr(span, "data-type").is_empty()
                        || !self.dom.attr(span, "data-tex").is_empty()
                }) {
                    return;
                }
            }
        }

        let n = self.primary_node(NodeType::NodeParagraph);
        n.set_tokens(tag.as_bytes().to_vec());
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        self.leave();
    }

    fn gen_heading(&mut self, idx: usize, tag: &str) {
        if NodeType::NodeLink == self.tip.typ() {
            self.gen_children(idx);
            return;
        }
        let level = tag.as_bytes()[1] - b'0';
        let n = self.primary_node(NodeType::NodeHeading);
        n.set_tokens(tag.as_bytes().to_vec());
        n.data_mut().heading_level = level as i32;
        let marker = self.new_node_raw(NodeType::NodeHeadingC8hMarker);
        marker.set_tokens(vec![b'#'; level as usize]);
        n.append_child(marker);
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        self.leave();
    }

    fn gen_list_item(&mut self, idx: usize) {
        let mut marker = self.dom.attr(idx, "data-marker");
        let mut bullet = 0u8;
        let parent_tag = self.dom.nodes[idx]
            .parent
            .map(|p| self.dom.nodes[p].data.clone())
            .unwrap_or_default();
        if marker.is_empty() {
            if "ol" == parent_tag {
                let start = self.dom.nodes[idx]
                    .parent
                    .map(|p| self.dom.attr(p, "start"))
                    .unwrap_or_default();
                marker = if start.is_empty() {
                    "1.".to_string()
                } else {
                    format!("{start}.")
                };
                if let Some(last) = self.tip.last_child() {
                    if NodeType::NodeListItem == last.typ() {
                        if let Some(ld) = last.data().list_data.as_ref() {
                            let num: String = String::from_utf8_lossy(&ld.marker)
                                .trim_end_matches('.')
                                .to_string();
                            let n: i32 = num.parse().unwrap_or(0);
                            marker = format!("{}.", n + 1);
                        }
                    }
                }
            } else {
                marker = "*".to_string();
                bullet = b'*';
            }
        } else if "ol" == parent_tag && "1." != marker {
            // 子有序列表必须从 1 开始
            let grand = self.dom.nodes[idx]
                .parent
                .and_then(|p| self.dom.nodes[p].parent)
                .map(|p| self.dom.nodes[p].data.clone())
                .unwrap_or_default();
            if "ol" == grand || "ul" == grand {
                marker = "1.".to_string();
            }
        }
        let typ = self
            .tip
            .data()
            .list_data
            .as_ref()
            .map(|d| d.typ)
            .unwrap_or(0);
        if 1 == typ && !marker.ends_with('.') {
            marker.push('.');
        }

        let n = self.primary_node(NodeType::NodeListItem);
        n.set_tokens(b"li".to_vec());
        let ld = crate::ast::ListData {
            typ,
            marker: marker.into_bytes(),
            bullet_char: bullet,
            ..Default::default()
        };
        n.data_mut().list_data = Some(ld);
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        self.leave();
    }

    /// h2m.go normalizes editor chrome and per-line wrappers before reading
    /// code text. Keep the transformations on the DOM, so whitespace extraction
    /// follows the same path as ordinary pre.code.
    fn normalize_pre(&mut self, idx: usize) -> bool {
        if let Some(content) = descendant_by_class(self.dom, idx, "cm-content") {
            let lines: Vec<_> = self.dom.nodes[content]
                .children
                .iter()
                .copied()
                .filter(|&child| has_class(self.dom, child, "cm-line"))
                .map(|child| {
                    let text = self.dom.text(child);
                    text.strip_suffix('\n').unwrap_or(&text).to_string()
                })
                .collect();
            if !lines.is_empty() {
                let code = self.dom.create(DomType::Element, "code");
                let text = lines.join("\n");
                if !text.is_empty() {
                    let child = self.dom.create(DomType::Text, text);
                    self.dom.append_child(code, child);
                }
                let language = self.dom.attr(content, "data-language");
                if !language.trim().is_empty() {
                    self.dom
                        .set_attr(code, "class", &format!("language-{}", language.trim()));
                }
                for child in self.dom.nodes[idx].children.clone() {
                    self.dom.unlink(child);
                }
                self.dom.append_child(idx, code);
            }
        }
        let codes = self.dom.children_by_tag(idx, "code");
        if let Some(&code) = codes.first() {
            while let Some(previous) = self.dom.prev_sibling(code) {
                self.dom.unlink(previous);
            }
        }
        let Some(mut first) = self.dom.nodes[idx].children.first().copied() else {
            return false;
        };
        if self.dom.attr(first, "class").contains("iconfont") {
            self.dom.unlink(first);
            let Some(next) = self.dom.nodes[idx].children.first().copied() else {
                return false;
            };
            first = next;
        }
        if self.dom.is_element(first, "div") {
            if self.dom.next_sibling(first).is_none() {
                if codes.len() == 1 {
                    self.dom.append_child(idx, codes[0]);
                    self.dom.unlink(first);
                }
            } else {
                for div in self.dom.nodes[idx].children.clone() {
                    let code = self.dom.create(DomType::Element, "code");
                    for child in self.dom.nodes[div].children.clone() {
                        self.dom.append_child(code, child);
                    }
                    self.dom.unlink(div);
                    self.dom.append_child(idx, code);
                }
            }
        }
        let Some(mut first) = self.dom.nodes[idx].children.first().copied() else {
            return false;
        };
        let mut span_to_code = false;
        if self.dom.is_element(first, "ol") && self.dom.next_sibling(first).is_none() {
            let lis = self.dom.nodes[first].children.clone();
            let paragraph = lis
                .first()
                .and_then(|&li| self.dom.nodes[li].children.first())
                .copied();
            if let Some(p) = paragraph.filter(|&p| self.dom.is_element(p, "p")) {
                let inline = self.dom.nodes[p].children.first().copied();
                if inline.is_some_and(|c| {
                    self.dom.is_element(c, "span") || self.dom.nodes[c].typ == DomType::Text
                }) {
                    for li in &lis {
                        if let Some(p) = self.dom.nodes[*li].children.first().copied() {
                            let code = self.dom.create(DomType::Element, "code");
                            for child in self.dom.nodes[p].children.clone() {
                                self.dom.append_child(code, child);
                            }
                            self.dom.append_child(p, code);
                            span_to_code = true;
                        }
                    }
                }
                if self.dom.nodes[p]
                    .children
                    .first()
                    .is_some_and(|&c| self.dom.is_element(c, "code"))
                {
                    for li in lis {
                        if let Some(code) = self.dom.nodes[li]
                            .children
                            .first()
                            .and_then(|&p| self.dom.nodes[p].children.first())
                            .copied()
                        {
                            self.dom.append_child(idx, code);
                        }
                        self.dom.unlink(li);
                    }
                    self.dom.unlink(first);
                    if let Some(&next) = self.dom.nodes[idx].children.first() {
                        first = next;
                    }
                }
            }
        }
        if self.dom.is_element(first, "code") {
            if self
                .dom
                .next_sibling(first)
                .is_some_and(|next| self.dom.is_element(next, "code"))
            {
                let following: Vec<_> = self.dom.nodes[idx]
                    .children
                    .iter()
                    .copied()
                    .skip(1)
                    .collect();
                for child in following {
                    insert_dom_br_before(self.dom, child);
                }
            }
            if let Some(&child) = self.dom.nodes[first].children.first() {
                if self.dom.is_element(child, "span") {
                    let following: Vec<_> = self.dom.nodes[first]
                        .children
                        .iter()
                        .copied()
                        .skip(1)
                        .collect();
                    for child in following {
                        if self.dom.attr(child, "class").contains("token-line") {
                            insert_dom_br_before(self.dom, child);
                        }
                    }
                }
                if self.dom.is_element(child, "ol") {
                    let following: Vec<_> = self.dom.nodes[child]
                        .children
                        .iter()
                        .copied()
                        .skip(1)
                        .collect();
                    for child in following {
                        insert_dom_br_before(self.dom, child);
                    }
                }
            }
            if let Some(last) = self.dom.nodes[idx].children.last().copied() {
                if self.dom.is_element(last, "ul") {
                    self.dom.unlink(last);
                }
            }
        }
        if self.dom.is_element(first, "pre") {
            let following: Vec<_> = self.dom.nodes[first]
                .children
                .iter()
                .copied()
                .skip(1)
                .collect();
            for child in following {
                insert_dom_br_before(self.dom, child);
            }
        }
        span_to_code
    }

    fn gen_pre(&mut self, idx: usize) {
        let span_to_code = self.normalize_pre(idx);
        let children = self.dom.nodes[idx].children.clone();
        let first = match children.first() {
            Some(&c) => c,
            None => return,
        };
        let first_tag = if self.dom.nodes[first].typ == DomType::Element {
            self.dom.nodes[first].data.clone()
        } else {
            String::new()
        };
        let recognized = self.dom.nodes[first].typ == DomType::Text
            || matches!(
                first_tag.as_str(),
                "span" | "code" | "section" | "pre" | "a" | "strong" | "b" | "i" | "p"
            );
        if !recognized {
            let n = self.primary_node(NodeType::NodeHTMLBlock);
            n.set_tokens(self.dom.html(idx).into_bytes());
            self.tip.append_child(n);
            return;
        }

        let n = self.primary_node(NodeType::NodeCodeBlock);
        n.set_tokens(b"pre".to_vec());
        n.data_mut().is_fenced_code_block = true;
        let open = self.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
        open.set_tokens(b"```".to_vec());
        open.data_mut().code_block_fence_len = 3;
        n.append_child(open);
        let info = self.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
        n.append_child(info);

        if matches!(first_tag.as_str(), "code" | "span" | "a") {
            let mut class = self.dom.attr(first, "class");
            if let Some(pp) = self.dom.nodes[idx]
                .parent
                .and_then(|p| self.dom.nodes[p].parent)
            {
                let parent_class = self.dom.attr(pp, "class");
                if parent_class.contains("language-") {
                    class = parent_class;
                }
            }
            if class.starts_with("lang-") {
                class = class.replace("lang-", "language-");
            }
            if !class.contains("language-") {
                class = self.dom.attr(idx, "class");
            }
            if class.contains("language-") {
                let language = class[class.find("language-").unwrap() + 9..]
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_string();
                if "fallback" != language && "chroma" != language {
                    if let Some(info) = n.child_by_type(NodeType::NodeCodeBlockFenceInfoMarker) {
                        info.data_mut().code_block_info = language.into_bytes();
                    }
                }
            } else if "code" == first_tag && !span_to_code {
                let class = self.dom.attr(first, "class");
                if !class.contains(' ') {
                    info.data_mut().code_block_info = class.into_bytes();
                }
            }

            if let Some(info) = n.child_by_type(NodeType::NodeCodeBlockFenceInfoMarker) {
                if info.data().code_block_info.is_empty() {
                    let class = self.dom.attr(idx, "class");
                    if !class.contains(' ') && "fallback" != class && "chroma" != class {
                        info.data_mut().code_block_info = class.into_bytes();
                    }
                }
                if info.data().code_block_info.is_empty() {
                    let lang = self.dom.attr(idx, "data-language");
                    if !lang.contains(' ') {
                        info.data_mut().code_block_info = lang.into_bytes();
                    }
                }
                if info
                    .data()
                    .code_block_info
                    .iter()
                    .any(|&b| b == b'-' || b == b'_' || b == b' ')
                {
                    info.data_mut().code_block_info.clear();
                }
            }
        }

        if self.dom.is_element(first, "p") {
            self.dom.nodes[first].data = "div".to_string();
        }
        let mut tokens = self.dom.text(idx).replace('\u{a0}', " ");
        let suffix = format!("\n{}", editor::ZWSP);
        if tokens.ends_with(&suffix) {
            tokens.truncate(tokens.len() - suffix.len());
        }
        let content = self.new_node_raw(NodeType::NodeCodeBlockCode);
        content.set_tokens(tokens.into_bytes());
        n.append_child(content);
        let close = self.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
        close.set_tokens(b"```".to_vec());
        close.data_mut().code_block_fence_len = 3;
        n.append_child(close);
        if self.tip.parent_is(&[NodeType::NodeTable]) {
            let mut parent = self.tip.parent();
            while let Some(table) = parent {
                if table.typ() == NodeType::NodeTable {
                    let single = table
                        .first_child()
                        .filter(|head| {
                            head.typ() == NodeType::NodeTableHead
                                && table.last_child() == Some(*head)
                        })
                        .filter(|head| head.first_child() == head.last_child())
                        .and_then(|head| head.first_child())
                        .and_then(|row| row.first_child())
                        .is_some_and(|cell| cell.typ() == NodeType::NodeTableCell);
                    if single {
                        table.insert_before(n);
                        table.unlink();
                        self.tip = n;
                        let mut dom_parent = self.dom.nodes[idx].parent;
                        for _ in 0..32 {
                            let Some(parent) = dom_parent else {
                                break;
                            };
                            let class = self.dom.attr(parent, "class");
                            let language = class
                                .find("language-")
                                .map(|position| &class[position + 9..])
                                .or_else(|| {
                                    class
                                        .find("highlight ")
                                        .map(|position| &class[position + 10..])
                                });
                            if let Some(language) = language {
                                info.data_mut().code_block_info = language.as_bytes().to_vec();
                                break;
                            }
                            dom_parent = self.dom.nodes[parent].parent;
                        }
                        return;
                    }
                }
                parent = table.parent();
            }
            let lines: Vec<_> = content
                .tokens_vec()
                .split(|&b| b == b'\n')
                .map(|line| line.to_vec())
                .collect();
            for (position, line) in lines.iter().enumerate() {
                if line.is_empty() {
                    continue;
                }
                let code = self.new_node_raw(NodeType::NodeCodeSpan);
                let open = self.new_node_raw(NodeType::NodeCodeSpanOpenMarker);
                open.set_tokens(b"`".to_vec());
                code.append_child(open);
                let text = self.new_node_raw(NodeType::NodeCodeSpanContent);
                text.set_tokens(line.clone());
                code.append_child(text);
                let close = self.new_node_raw(NodeType::NodeCodeSpanCloseMarker);
                close.set_tokens(b"`".to_vec());
                code.append_child(close);
                self.tip.append_child(code);
                if position + 1 < lines.len() {
                    let br = self.new_node_raw(if crate::PROTYLE && self.options.protyle_wysiwyg {
                        NodeType::NodeBr
                    } else {
                        NodeType::NodeHardBreak
                    });
                    if !(crate::PROTYLE && self.options.protyle_wysiwyg) {
                        br.set_tokens(b"\n".to_vec());
                    }
                    self.tip.append_child(br);
                }
            }
        } else {
            self.tip.append_child(n);
        }
    }

    fn gen_em(&mut self, idx: usize) {
        let text = self.dom.text(idx);
        if text.trim().is_empty() {
            self.gen_children(idx);
            return;
        }
        if NodeType::NodeEmphasis == self.tip.typ() || self.tip.parent_is(&[NodeType::NodeEmphasis])
        {
            self.gen_children(idx);
            return;
        }
        if let Some(last) = self.tip.last_child() {
            if NodeType::NodeStrong == last.typ() || NodeType::NodeEmphasis == last.typ() {
                let zwsp = self.text_node(editor::ZWSP);
                self.tip.append_child(zwsp);
            }
        }
        let use_marker = (self.options.inline_asterisk && self.options.inline_underscore)
            || self.options.html_tag2_text_mark;
        if !use_marker {
            self.gen_html_tag(idx, "em");
            return;
        }
        let n = self.primary_node(NodeType::NodeEmphasis);
        n.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
        let m = self.new_node_raw(NodeType::NodeEmA6kOpenMarker);
        m.set_tokens(b"*".to_vec());
        n.append_child(m);
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        let close = self.new_node_raw(NodeType::NodeEmA6kCloseMarker);
        close.set_tokens(b"*".to_vec());
        self.tip.append_child(close);
        self.append_space(idx);
        self.leave();
    }

    fn gen_strong(&mut self, idx: usize) {
        let text = self.dom.text(idx);
        if text.trim().is_empty() {
            self.gen_children(idx);
            return;
        }
        if NodeType::NodeStrong == self.tip.typ() || self.tip.parent_is(&[NodeType::NodeStrong]) {
            self.gen_children(idx);
            return;
        }
        if let Some(last) = self.tip.last_child() {
            if NodeType::NodeStrong == last.typ() || NodeType::NodeEmphasis == last.typ() {
                let zwsp = self.text_node(editor::ZWSP);
                self.tip.append_child(zwsp);
            }
        }
        let use_marker = (self.options.inline_asterisk && self.options.inline_underscore)
            || self.options.html_tag2_text_mark;
        if !use_marker {
            self.gen_html_tag(idx, "strong");
            return;
        }
        let n = self.primary_node(NodeType::NodeStrong);
        n.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
        let m = self.new_node_raw(NodeType::NodeStrongA6kOpenMarker);
        m.set_tokens(b"**".to_vec());
        n.append_child(m);
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        let close = self.new_node_raw(NodeType::NodeStrongA6kCloseMarker);
        close.set_tokens(b"**".to_vec());
        self.tip.append_child(close);
        self.append_space(idx);
        self.leave();
    }

    fn gen_html_tag(&mut self, idx: usize, tag: &str) {
        let n = self.primary_node(NodeType::NodeHTMLTag);
        n.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
        let open = self.new_node_raw(NodeType::NodeHTMLTagOpen);
        open.set_tokens(format!("<{tag}>").into_bytes());
        n.append_child(open);
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        let close = self.new_node_raw(NodeType::NodeHTMLTagClose);
        close.set_tokens(format!("</{tag}>").into_bytes());
        self.tip.append_child(close);
        self.append_space(idx);
        self.leave();
    }

    fn gen_code(&mut self, idx: usize) {
        remove_dom_comments(self.dom, idx);
        if self.dom.nodes[idx].children.is_empty() {
            return;
        }
        if let Some(last) = self.tip.last_child() {
            if NodeType::NodeCodeSpan == last.typ() {
                let zwsp = self.text_node(editor::ZWSP);
                self.tip.append_child(zwsp);
            }
        }
        let mut code = self.dom.html(idx);
        if let Some(pos) = code.find('>') {
            code = code[pos + 1..].to_string();
        }
        if code.ends_with("</code>") {
            code.truncate(code.len() - 7);
        }
        if code_text_only(self.dom, idx) {
            code = self.dom.text(idx).replace(['\u{a0}', '\n'], " ");
        }

        let n = self.primary_node(NodeType::NodeCodeSpan);
        n.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
        if code.contains("```") {
            n.data_mut().code_marker_len = 2;
        } else if code.contains("``") {
            n.data_mut().code_marker_len = 1;
        } else if code.contains('`') {
            n.data_mut().code_marker_len = 2;
        }
        let open = self.new_node_raw(NodeType::NodeCodeSpanOpenMarker);
        open.set_tokens(b"`".to_vec());
        n.append_child(open);
        let content = self.new_node_raw(NodeType::NodeCodeSpanContent);
        let mut tokens = code.into_bytes();
        if code_contains_double_backtick(&tokens) {
            let mut padded = b" ".to_vec();
            padded.extend_from_slice(&tokens);
            tokens = padded;
        }
        content.set_tokens(tokens);
        n.append_child(content);
        if code_contains_double_backtick(content.tokens()) {
            let mut padded = content.tokens_vec();
            padded.push(b' ');
            content.set_tokens(padded);
        }
        let close = self.new_node_raw(NodeType::NodeCodeSpanCloseMarker);
        close.set_tokens(b"`".to_vec());
        n.append_child(close);
        self.tip.append_child(n);
        self.apply_html_style(idx, n);
    }

    fn gen_link(&mut self, idx: usize) {
        let text = self.dom.text(idx);
        if text.trim().is_empty()
            && self
                .dom
                .parent_is(idx, &["h1", "h2", "h3", "h4", "h5", "h6", "div", "section"])
            && self.dom.children_by_tag(idx, "img").is_empty()
        {
            return;
        }
        if text.trim().is_empty() && self.dom.nodes[idx].children.is_empty() {
            return;
        }
        if let Some(first) = self.dom.nodes[idx].children.first() {
            if self.dom.is_element(*first, "img")
                && self.dom.attr(*first, "src").contains("wikimedia.org")
            {
                self.gen_children(idx);
                return;
            }
        }
        let n = self.primary_node(NodeType::NodeLink);
        n.set_tokens(b"a".to_vec());
        n.append_child(self.new_node_raw(NodeType::NodeOpenBracket));
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        let close_bracket = self.new_node_raw(NodeType::NodeCloseBracket);
        self.tip.append_child(close_bracket);
        let open_paren = self.new_node_raw(NodeType::NodeOpenParen);
        self.tip.append_child(open_paren);
        let dest = self.new_node_raw(NodeType::NodeLinkDest);
        dest.set_tokens(self.dom.attr(idx, "href").into_bytes());
        self.tip.append_child(dest);
        let title = self.dom.attr(idx, "title");
        if !title.is_empty() {
            self.tip
                .append_child(self.new_node_raw(NodeType::NodeLinkSpace));
            let t = self.new_node_raw(NodeType::NodeLinkTitle);
            t.set_tokens(title.into_bytes());
            self.tip.append_child(t);
        }
        let close_paren = self.new_node_raw(NodeType::NodeCloseParen);
        self.tip.append_child(close_paren);
        self.leave();
    }

    fn gen_img(&mut self, idx: usize) {
        let img_class = self.dom.attr(idx, "class");
        let img_alt = self.dom.attr(idx, "alt");
        if "emoji" == img_class {
            let n = self.primary_node(NodeType::NodeText);
            n.set_tokens(img_alt.into_bytes());
            self.tip.append_child(n);
            return;
        }

        let n = self.primary_node(NodeType::NodeImage);
        n.set_tokens(b"img".to_vec());
        n.append_child(self.new_node_raw(NodeType::NodeBang));
        n.append_child(self.new_node_raw(NodeType::NodeOpenBracket));
        if !img_alt.is_empty() {
            let alt = img_alt
                .trim()
                .replace(']', "")
                .replace('\n', " ")
                .replace("动图封面", "动图");
            let t = self.new_node_raw(NodeType::NodeLinkText);
            t.set_tokens(alt.into_bytes());
            n.append_child(t);
        }
        n.append_child(self.new_node_raw(NodeType::NodeCloseBracket));
        n.append_child(self.new_node_raw(NodeType::NodeOpenParen));
        let mut src = self.dom.attr(idx, "src");
        if img_class.contains("ztext-gif") && src.contains("zhimg.com") {
            src = src.replacen(".jpg", ".webp", 1);
        }
        if src.starts_with("data:image") || src.ends_with("img-loading.svg") {
            let data_src = self.dom.attr(idx, "data-src");
            if data_src.starts_with("http://") || data_src.starts_with("https://") {
                src = data_src;
            }
        }
        let data_original = self.dom.attr(idx, "data-original");
        if !data_original.is_empty()
            && !data_original.starts_with('/')
            && (src.is_empty() || !src.ends_with(".gif"))
        {
            src = data_original;
        }
        if src.is_empty() {
            let srcset = self.dom.attr(idx, "srcset");
            if !srcset.is_empty() {
                if srcset.contains(',') {
                    let parts: Vec<&str> = srcset.split(',').collect();
                    src = parts[parts.len() - 1].trim().to_string();
                    if src.contains(' ') {
                        src = src.split(' ').next().unwrap_or("").trim().to_string();
                    }
                } else {
                    src = srcset.trim().to_string();
                    if src.contains(' ') {
                        src = srcset.split(' ').next().unwrap_or("").trim().to_string();
                    }
                }
            }
        }
        if src.contains("wikipedia/commons/thumb/") {
            let ext = if src.contains(".svg.png") {
                Some(".svg")
            } else {
                src.rfind('.').map(|position| &src[position..])
            };
            if let Some(ext) = ext {
                if let Some(position) = src.find(&format!("{ext}/")) {
                    src = src[..position + ext.len()].replace("/commons/thumb/", "/commons/");
                }
            }
        }
        let dest = self.new_node_raw(NodeType::NodeLinkDest);
        dest.set_tokens(src.into_bytes());
        n.append_child(dest);

        let mut link_title = String::new();
        let mut figcaption = None;
        let mut next = self.dom.next_sibling(idx);
        while let Some(nx) = next {
            if self.dom.nodes[nx].typ == DomType::Text && self.dom.nodes[nx].data.trim().is_empty()
            {
                next = self.dom.next_sibling(nx);
                continue;
            }
            if self.dom.is_element(nx, "figcaption") {
                figcaption = Some(nx);
                break;
            }
            next = self.dom.next_sibling(nx);
        }
        if let Some(fc) = figcaption {
            link_title = self.dom.text(fc);
            self.dom.unlink(fc);
        }
        if link_title.is_empty() {
            link_title = self.dom.attr(idx, "title");
        }
        let link_title = link_title.trim().to_string();
        if !link_title.is_empty() && "null" != link_title && "undefined" != link_title {
            n.append_child(self.new_node_raw(NodeType::NodeLinkSpace));
            let t = self.new_node_raw(NodeType::NodeLinkTitle);
            t.set_tokens(link_title.into_bytes());
            n.append_child(t);
        }
        n.append_child(self.new_node_raw(NodeType::NodeCloseParen));

        let width = self.dom.attr(idx, "width").trim().to_string();
        let height = self.dom.attr(idx, "height").trim().to_string();
        let style = if !width.is_empty() {
            let w = if width.bytes().all(|b| b.is_ascii_digit()) {
                format!("{width}px")
            } else {
                width
            };
            Some(format!("width: {w};"))
        } else if !height.is_empty() {
            let h = if height.bytes().all(|b| b.is_ascii_digit()) {
                format!("{height}px")
            } else {
                height
            };
            Some(format!("height: {h};"))
        } else {
            None
        };

        if NodeType::NodeDocument == self.tip.typ() {
            let p = self.new_node_raw(NodeType::NodeParagraph);
            self.tip.append_child(p);
            self.tip = p;
        }
        let ial = if let Some(style) = style {
            let ial_node = self.new_node_raw(NodeType::NodeKramdownSpanIAL);
            ial_node.set_tokens(crate::parse::ial::ial_to_tokens(&[(
                "style".to_string(),
                style,
            )]));
            Some(ial_node)
        } else {
            None
        };
        self.tip.append_child(n);
        if let Some(ial) = ial {
            self.tip.append_child(ial);
        }
        self.tip = n;
        self.leave();
    }

    fn gen_span(&mut self, idx: usize) {
        let class = self.dom.attr(idx, "class");
        let text_style = if crate::PROTYLE && self.options.protyle_wysiwyg {
            crate::parse::html_text_style::html_node_text_style(self.dom, idx).css()
        } else {
            String::new()
        };
        if "fn__space5" == class {
            return;
        }
        if "tip" == class {
            if let Some(last) = self.tip.last_child() {
                if NodeType::NodeText == last.typ() {
                    last.data_mut().typ = NodeType::NodeTextMark;
                    last.data_mut().text_mark_type = "inline-memo".to_string();
                    last.data_mut().text_mark_text_content = last.tokens_str();
                    last.data_mut().text_mark_inline_memo_content = self.dom.text(idx);
                    if let Some(prev) = last.previous() {
                        if NodeType::NodeText == prev.typ() {
                            let trimmed = prev.tokens_str().trim().to_string();
                            prev.set_tokens(trimmed.into_bytes());
                            if prev.tokens().is_empty() {
                                prev.unlink();
                            }
                        }
                    }
                    return;
                }
            }
        }
        let title = self.dom.attr(idx, "title").trim().to_string();
        if !title.is_empty() && self.tip.is_block() {
            let n = self.primary_node(NodeType::NodeTextMark);
            n.data_mut().text_mark_type = "inline-memo".to_string();
            n.data_mut().text_mark_text_content = self.dom.text(idx);
            n.data_mut().text_mark_inline_memo_content = title;
            self.tip.append_child(n);
            self.apply_html_style(idx, n);
            if let Some(previous) = n.previous().filter(|n| n.typ() == NodeType::NodeText) {
                previous.set_tokens(previous.tokens_str().trim().as_bytes().to_vec());
                if previous.tokens().is_empty() {
                    previous.unlink();
                }
            }
            return;
        }

        let data_type = self.dom.attr(idx, "data-type");
        if data_type.contains(' ') {
            let n = self.primary_node(NodeType::NodeTextMark);
            n.data_mut().text_mark_type = data_type;
            n.data_mut().text_mark_text_content = self.dom.text(idx);
            self.tip.append_child(n);
            self.apply_html_style(idx, n);
            return;
        }
        let data_type = data_type.split(' ').next().unwrap_or("").to_string();
        match data_type.as_str() {
            "inline-math" => {
                let content = self.dom.attr(idx, "data-content");
                self.append_inline_math(&content);
                return;
            }
            "code" => {
                let code = self.primary_node(NodeType::NodeCodeSpan);
                let content = self.dom.text(idx);
                if content.contains('`') {
                    code.data_mut().code_marker_len = 2;
                }
                let open = self.new_node_raw(NodeType::NodeCodeSpanOpenMarker);
                open.set_tokens(b"`".to_vec());
                code.append_child(open);
                let c = self.new_node_raw(NodeType::NodeCodeSpanContent);
                c.set_tokens(content.into_bytes());
                code.append_child(c);
                let close = self.new_node_raw(NodeType::NodeCodeSpanCloseMarker);
                close.set_tokens(b"`".to_vec());
                code.append_child(close);
                self.tip.append_child(code);
                return;
            }
            "tag" => {
                let tag = self.primary_node(NodeType::NodeTag);
                let open = self.new_node_raw(NodeType::NodeTagOpenMarker);
                open.set_tokens(b"#".to_vec());
                tag.append_child(open);
                let text = self.dom.text(idx);
                tag.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeTagCloseMarker);
                close.set_tokens(b"#".to_vec());
                tag.append_child(close);
                self.tip.append_child(tag);
                return;
            }
            "kbd" => {
                let kbd = self.primary_node(NodeType::NodeKbd);
                kbd.append_child(self.new_node_raw(NodeType::NodeKbdOpenMarker));
                let text = self.dom.text(idx);
                kbd.append_child(self.text_node(&text));
                kbd.append_child(self.new_node_raw(NodeType::NodeKbdCloseMarker));
                self.tip.append_child(kbd);
                return;
            }
            "sub" => {
                let sub = self.primary_node(NodeType::NodeSub);
                sub.append_child(self.new_node_raw(NodeType::NodeSubOpenMarker));
                let text = self.dom.text(idx);
                sub.append_child(self.text_node(&text));
                sub.append_child(self.new_node_raw(NodeType::NodeSubCloseMarker));
                self.tip.append_child(sub);
                return;
            }
            "sup" => {
                let sup = self.primary_node(NodeType::NodeSup);
                sup.append_child(self.new_node_raw(NodeType::NodeSupOpenMarker));
                let text = self.dom.text(idx);
                sup.append_child(self.text_node(&text));
                sup.append_child(self.new_node_raw(NodeType::NodeSupCloseMarker));
                self.tip.append_child(sup);
                return;
            }
            "mark" => {
                let mark = self.primary_node(NodeType::NodeMark);
                let open = self.new_node_raw(NodeType::NodeMark2OpenMarker);
                open.set_tokens(b"==".to_vec());
                mark.append_child(open);
                let text = self.dom.text(idx);
                mark.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeMark2CloseMarker);
                close.set_tokens(b"==".to_vec());
                mark.append_child(close);
                self.tip.append_child(mark);
                return;
            }
            "s" => {
                let s = self.primary_node(NodeType::NodeStrikethrough);
                let open = self.new_node_raw(NodeType::NodeStrikethrough2OpenMarker);
                open.set_tokens(b"~~".to_vec());
                s.append_child(open);
                let text = self.dom.text(idx);
                s.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeStrikethrough2CloseMarker);
                close.set_tokens(b"~~".to_vec());
                s.append_child(close);
                self.tip.append_child(s);
                return;
            }
            "u" => {
                let u = self.primary_node(NodeType::NodeUnderline);
                u.append_child(self.new_node_raw(NodeType::NodeUnderlineOpenMarker));
                let text = self.dom.text(idx);
                u.append_child(self.text_node(&text));
                u.append_child(self.new_node_raw(NodeType::NodeUnderlineCloseMarker));
                self.tip.append_child(u);
                return;
            }
            "em" => {
                let em = self.primary_node(NodeType::NodeEmphasis);
                let open = self.new_node_raw(NodeType::NodeEmA6kOpenMarker);
                open.set_tokens(b"*".to_vec());
                em.append_child(open);
                let text = self.dom.text(idx);
                em.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeEmA6kCloseMarker);
                close.set_tokens(b"*".to_vec());
                em.append_child(close);
                self.tip.append_child(em);
                return;
            }
            "strong" => {
                let strong = self.primary_node(NodeType::NodeStrong);
                let open = self.new_node_raw(NodeType::NodeStrongA6kOpenMarker);
                open.set_tokens(b"**".to_vec());
                strong.append_child(open);
                let text = self.dom.text(idx);
                strong.append_child(self.text_node(&text));
                let close = self.new_node_raw(NodeType::NodeStrongA6kCloseMarker);
                close.set_tokens(b"**".to_vec());
                strong.append_child(close);
                self.tip.append_child(strong);
                return;
            }
            "block-ref" => {
                let r = self.primary_node(NodeType::NodeBlockRef);
                r.append_child(self.new_node_raw(NodeType::NodeOpenParen));
                r.append_child(self.new_node_raw(NodeType::NodeOpenParen));
                let id = self.new_node_raw(NodeType::NodeBlockRefID);
                id.set_tokens(self.dom.attr(idx, "data-id").into_bytes());
                r.append_child(id);
                r.append_child(self.new_node_raw(NodeType::NodeBlockRefSpace));
                let text = self.dom.text(idx);
                let text_type = if "s" == self.dom.attr(idx, "data-subtype") {
                    NodeType::NodeBlockRefText
                } else {
                    NodeType::NodeBlockRefDynamicText
                };
                let t = self.new_node_raw(text_type);
                t.set_tokens(text.into_bytes());
                r.append_child(t);
                r.append_child(self.new_node_raw(NodeType::NodeCloseParen));
                r.append_child(self.new_node_raw(NodeType::NodeCloseParen));
                self.tip.append_child(r);
                return;
            }
            _ => {}
        }

        // h2m.go handles normalized math spans before ordinary CSS styling.
        let tex = self.dom.attr(idx, "data-tex").trim().to_owned();
        if !tex.is_empty() {
            let has_display_child = ["katex-display", "MathJax_SVG_Display"]
                .iter()
                .any(|class| {
                    self.dom
                        .find_descendant_by_tag_and_class(idx, "span", class)
                        .is_some()
                });
            if class.split_whitespace().any(|c| c == "language-math") {
                if has_display_child {
                    self.append_math_block(&tex);
                } else {
                    self.append_inline_math(&tex);
                }
                return;
            }
            let parent = self.dom.nodes[idx].parent;
            let parent_class = parent
                .map(|p| self.dom.attr(p, "class"))
                .unwrap_or_default();
            if (class.contains("math-inline")
                && (self.dom.prev_sibling(idx).is_some()
                    || self.dom.next_sibling(idx).is_some()
                    || self.dom.parent_is(idx, &["table"])))
                || parent_class.contains("math-inline")
            {
                self.append_inline_math(&tex);
                return;
            }
            let parent_inline = parent.is_some()
                && (self
                    .dom
                    .parent_is(idx, &["strong", "em", "i", "b", "span", "td", "th"])
                    || parent_class.contains("inline"));
            let last_or_blank = |node| {
                self.dom.next_sibling(node).is_none_or(|next| {
                    self.dom.nodes[next].typ == DomType::Text
                        && self.dom.text(next).trim().is_empty()
                })
            };
            let only_child = self.dom.prev_sibling(idx).is_none() && last_or_blank(idx);
            let parent_only_child =
                parent.is_some_and(|p| self.dom.prev_sibling(p).is_none() && last_or_blank(p));
            let is_span = self.dom.is_element(idx, "span");
            let parent_block = parent.is_some()
                && (self.dom.parent_is(idx, &["div", "section", "p"])
                    || parent_class.contains("block"));
            let mathjax_display = self
                .dom
                .find_descendant_by_tag_and_class(idx, "span", "MathJax_SVG_Display")
                .is_some();
            let display = (parent_inline && is_span && only_child && parent_only_child)
                || (!parent_inline && only_child)
                || (!parent_inline && parent_block && is_span && only_child)
                || (is_span && class == "katex-display")
                || mathjax_display
                || tex.ends_with("\\\\")
                || tex.contains('\n')
                || tex.contains("\\tag{");
            if display {
                self.append_math_block(&tex);
            } else {
                self.append_inline_math(&tex);
            }
            return;
        }

        if class.trim().to_lowercase().contains("mathjax") {
            if let Some(script) = self.dom.children_by_tag(idx, "script").first().copied() {
                let tex = self.dom.text(script);
                if !tex.is_empty() {
                    self.append_inline_math(&tex);
                }
            }
            return;
        }
        let style = self.dom.attr(idx, "style");
        if text_style.is_empty() && style.contains("underline") {
            let text = self.dom.text(idx);
            if text.is_empty() {
                return;
            }
            if let Some(last) = self.tip.last_child() {
                if NodeType::NodeHTMLTag != last.typ() {
                    if NodeType::NodeUnderline == self.tip.typ()
                        || self.tip.parent_is(&[NodeType::NodeUnderline])
                    {
                        return;
                    }
                    let u = self.primary_node(NodeType::NodeUnderline);
                    u.append_child(self.new_node_raw(NodeType::NodeUnderlineOpenMarker));
                    u.append_child(self.text_node(&text));
                    u.append_child(self.new_node_raw(NodeType::NodeUnderlineCloseMarker));
                    self.tip.append_child(u);
                    return;
                }
            }
        }
        if text_style.is_empty() && style.contains("bold") {
            let text = self.dom.text(idx);
            if text.is_empty() {
                return;
            }
            if let Some(last) = self.tip.last_child() {
                if NodeType::NodeHTMLTag != last.typ() {
                    if NodeType::NodeStrong == self.tip.typ()
                        || self.tip.parent_is(&[NodeType::NodeStrong])
                    {
                        self.gen_children(idx);
                        return;
                    }
                    if last.typ() == NodeType::NodeStrong {
                        if let Some(previous) = last
                            .last_child()
                            .and_then(|close| close.previous())
                            .filter(|n| n.typ() == NodeType::NodeText)
                        {
                            let mut tokens = previous.tokens_vec();
                            tokens.extend_from_slice(text.as_bytes());
                            previous.set_tokens(tokens);
                            return;
                        }
                    }
                    let strong = self.primary_node(NodeType::NodeStrong);
                    let open = self.new_node_raw(NodeType::NodeStrongA6kOpenMarker);
                    open.set_tokens(b"**".to_vec());
                    strong.append_child(open);
                    strong.append_child(self.text_node(&text));
                    let close = self.new_node_raw(NodeType::NodeStrongA6kCloseMarker);
                    close.set_tokens(b"**".to_vec());
                    strong.append_child(close);
                    self.tip.append_child(strong);
                    return;
                }
            }
        }

        self.gen_children(idx);
    }

    fn set_table_cell_span_ial(&mut self, node: Node, idx: usize) {
        for name in ["colspan", "rowspan"] {
            let value = self.dom.attr(idx, name);
            if !value.is_empty() {
                node.set_ial_attr(name, &value);
            }
        }
        if self.dom.attr(idx, "class") == "fn__none" {
            node.set_ial_attr("class", "fn__none");
        }
        let mut column = 0;
        let mut previous = node.previous();
        while let Some(cell) = previous {
            if cell.typ() == NodeType::NodeTableCell {
                column += 1;
            }
            previous = cell.previous();
        }
        let mut table = node.parent().and_then(|row| row.parent());
        if table.is_some_and(|n| n.typ() == NodeType::NodeTableHead) {
            table = table.and_then(|n| n.parent());
        }
        let align = table
            .and_then(|t| t.first_child())
            .filter(|n| n.typ() == NodeType::NodeTableHead)
            .and_then(|head| head.first_child())
            .and_then(|row| {
                row.children()
                    .filter(|n| n.typ() == NodeType::NodeTableCell)
                    .nth(column)
            })
            .map(|cell| cell.data().table_cell_align)
            .unwrap_or(0);
        if node.data().table_cell_align != align {
            let name = match node.data().table_cell_align {
                1 => "left",
                2 => "center",
                3 => "right",
                _ => "start",
            };
            node.set_ial_attr("style", &format!("text-align: {name};"));
        }
        if !node.ial_raw().is_empty() {
            let ial = self.new_node_raw(NodeType::NodeKramdownSpanIAL);
            ial.set_tokens(crate::parse::ial::ial_to_tokens(&node.ial_raw()));
            node.prepend_child(ial);
        }
    }

    fn set_table_caption(&mut self, idx: usize, n: Node) {
        if let Some(caption) = self.dom.child_by_tag(idx, "caption") {
            let title = self.dom.text(caption).trim().to_string();
            let style = self.dom.attr(caption, "style");
            self.dom.unlink(caption);
            if !title.is_empty() {
                let bottom = style
                    .split(';')
                    .find_map(|declaration| {
                        declaration
                            .split_once(':')
                            .filter(|(key, _)| key.trim().eq_ignore_ascii_case("caption-side"))
                    })
                    .is_some_and(|(_, value)| {
                        value
                            .trim()
                            .to_lowercase()
                            .trim_end_matches("!important")
                            .trim()
                            == "bottom"
                    });
                let style = if bottom {
                    " style=\"caption-side: bottom;\""
                } else {
                    ""
                };
                let caption = format!(
                    "<caption contenteditable=\"false\"{style}>{}</caption>",
                    crate::html::escape_html_str(&title)
                );
                n.set_ial_attr("caption", &crate::html::escape_html_str(&caption));
            }
        }
    }

    fn gen_table(&mut self, idx: usize) {
        let n = self.primary_node(NodeType::NodeTable);
        n.set_tokens(b"table".to_vec());
        let mut aligns = Vec::new();
        if let Some(head) = self.dom.child_by_tag(idx, "thead") {
            if let Some(tr) = self.dom.child_by_tag(head, "tr") {
                for &th in &self.dom.nodes[tr].children {
                    if !self.dom.is_element(th, "th") {
                        continue;
                    }
                    let align = self.dom.attr(th, "align");
                    aligns.push(match align.to_lowercase().as_str() {
                        "left" => 1,
                        "center" => 2,
                        "right" => 3,
                        _ => 0,
                    });
                }
            }
        }
        n.data_mut().table_aligns = aligns;
        if let Some(last) = self.tip.last_child() {
            if !last.is_block() {
                let br = self.new_node_raw(NodeType::NodeHardBreak);
                self.tip.append_child(br);
            }
        }
        self.tip.append_child(n);
        self.tip = n;
        self.gen_children(idx);
        self.leave();
    }

    fn append_inline_math(&mut self, tex: &str) {
        let mut tex = tex.trim().to_string();
        for (prefix, suffix) in [("\\(", "\\)"), ("\\[", "\\]")] {
            if tex.starts_with(prefix) {
                tex = tex[prefix.len()..].to_string();
            }
            if tex.ends_with(suffix) {
                tex.truncate(tex.len() - suffix.len());
            }
        }
        if tex.is_empty() {
            return;
        }
        let n = self.new_node_raw(NodeType::NodeInlineMath);
        let open = self.new_node_raw(NodeType::NodeInlineMathOpenMarker);
        open.set_tokens(b"$".to_vec());
        n.append_child(open);
        let content = self.new_node_raw(NodeType::NodeInlineMathContent);
        content.set_tokens(tex.into_bytes());
        n.append_child(content);
        let close = self.new_node_raw(NodeType::NodeInlineMathCloseMarker);
        close.set_tokens(b"$".to_vec());
        n.append_child(close);
        self.tip.append_child(n);
        self.tip = n;
        self.leave();
    }

    fn append_math_block(&mut self, tex: &str) {
        let mut tex = tex.trim().to_string();
        for (prefix, suffix) in [("\\(", "\\)"), ("\\[", "\\]")] {
            if tex.starts_with(prefix) {
                tex = tex[prefix.len()..].to_string();
            }
            if tex.ends_with(suffix) {
                tex.truncate(tex.len() - suffix.len());
            }
        }
        if tex.is_empty() {
            return;
        }
        let n = self.new_node_raw(NodeType::NodeMathBlock);
        let open = self.new_node_raw(NodeType::NodeMathBlockOpenMarker);
        open.set_tokens(b"$$".to_vec());
        n.append_child(open);
        let content = self.new_node_raw(NodeType::NodeMathBlockContent);
        content.set_tokens(tex.into_bytes());
        n.append_child(content);
        let close = self.new_node_raw(NodeType::NodeMathBlockCloseMarker);
        close.set_tokens(b"$$".to_vec());
        n.append_child(close);
        if NodeType::NodeParagraph == self.tip.typ() {
            self.tip.insert_after(n);
            if self.tip.first_child().is_none() {
                self.tip.unlink();
            }
        } else {
            self.tip.append_child(n);
        }
        self.tip = n;
        self.leave();
    }

    fn append_space(&mut self, idx: usize) {
        let next = match self.dom.next_sibling(idx) {
            Some(n) => n,
            None => return,
        };
        let next_text = self.dom.text(next);
        if next_text.is_empty() {
            return;
        }
        let first = match next_text.chars().next() {
            Some(c) => c,
            None => return,
        };
        if first.is_whitespace() {
            return;
        }
        let is_punct = |c: char| {
            crate::char_class::is_punct(c)
                || crate::char_class::in_table(crate::char_class::PUNCT_OR_SYMBOL, c)
        };
        if is_punct(first) {
            let zwsp1 = self.text_node(editor::ZWSP);
            self.tip.insert_before(zwsp1);
            let zwsp2 = self.text_node(editor::ZWSP);
            self.tip.insert_after(zwsp2);
            return;
        }
        let cur_text = self.dom.text(idx);
        if cur_text.is_empty() {
            return;
        }
        if let Some(last) = cur_text.chars().next_back() {
            if is_punct(last) {
                if let Some(text) = self.tip.child_by_type(NodeType::NodeText) {
                    let mut tokens = editor::ZWSP.as_bytes().to_vec();
                    tokens.extend_from_slice(text.tokens());
                    tokens.extend_from_slice(editor::ZWSP.as_bytes());
                    text.set_tokens(tokens);
                }
                return;
            }
        }
        let prefix: String = cur_text
            .chars()
            .take_while(|&c| ' ' == c || '\u{a0}' == c)
            .map(|_| ' ')
            .collect();
        if !prefix.is_empty() {
            if let Some(prev) = self.tip.previous() {
                if NodeType::NodeText == prev.typ() {
                    let mut t = prev.tokens_vec();
                    t.extend_from_slice(prefix.as_bytes());
                    prev.set_tokens(t);
                } else {
                    let n = self.text_node(&prefix);
                    prev.insert_after(n);
                }
            } else {
                let n = self.text_node(&prefix);
                self.tip.append_child(n);
            }
            if let Some(text) = self.tip.child_by_type(NodeType::NodeText) {
                let t = text
                    .tokens_str()
                    .trim_start_matches([' ', '\u{160}'])
                    .to_string();
                text.set_tokens(t.into_bytes());
            }
        }
        let suffix: String = cur_text
            .chars()
            .rev()
            .take_while(|&c| ' ' == c || '\u{a0}' == c)
            .map(|_| ' ')
            .collect();
        if !suffix.is_empty() {
            if let Some(text) = self.tip.last_child() {
                if NodeType::NodeText == text.typ() {
                    let t = text
                        .tokens_str()
                        .trim_end_matches([' ', '\u{160}'])
                        .to_string();
                    if t.is_empty() {
                        text.unlink();
                    } else {
                        text.set_tokens(t.into_bytes());
                    }
                }
            }
            let n = self.text_node(&suffix);
            self.tip.insert_after(n);
        }
    }
}

fn code_contains_double_backtick(tokens: &[u8]) -> bool {
    tokens.windows(2).any(|w| w == b"``")
}

/// h2m.go: fixTableStructure/fixOneTableStructure. HTML omits covered cells;
/// Lute's table AST uses explicit placeholders at every covered grid position.
fn fix_table_structure(dom: &mut Dom) {
    fn span(dom: &Dom, cell: usize, key: &str) -> usize {
        dom.attr(cell, key)
            .parse::<usize>()
            .ok()
            .filter(|&n| n > 0)
            .unwrap_or(1)
    }
    fn cell(dom: &Dom, idx: usize) -> bool {
        dom.is_element(idx, "td") || dom.is_element(idx, "th")
    }
    fn placeholder(dom: &mut Dom, hidden: bool) -> usize {
        let td = dom.create(DomType::Element, "td");
        if hidden {
            dom.set_attr(td, "class", "fn__none");
        }
        td
    }
    for table in dom.children_by_tag(dom.root, "table") {
        let rows = dom.children_by_tag(table, "tr");
        let max_cols = rows
            .iter()
            .map(|&row| {
                dom.nodes[row]
                    .children
                    .iter()
                    .copied()
                    .filter(|&c| cell(dom, c) && dom.attr(c, "class") != "fn__none")
                    .map(|c| span(dom, c, "colspan"))
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0);
        if max_cols == 0 {
            continue;
        }
        let mut occupied = vec![vec![false; max_cols]; rows.len()];
        for (ri, &row) in rows.iter().enumerate() {
            let cells: Vec<_> = dom.nodes[row]
                .children
                .iter()
                .copied()
                .filter(|&c| cell(dom, c) && dom.attr(c, "class") != "fn__none")
                .collect();
            let mut column = 0;
            for cell in cells {
                while column < max_cols && occupied[ri][column] {
                    let td = placeholder(dom, true);
                    dom.insert_before(cell, td);
                    column += 1;
                }
                if column >= max_cols {
                    break;
                }
                let colspan = span(dom, cell, "colspan");
                let rowspan = span(dom, cell, "rowspan").min(rows.len() - ri);
                for row in occupied.iter_mut().skip(ri).take(rowspan) {
                    for slot in row.iter_mut().skip(column).take(colspan) {
                        *slot = true;
                    }
                }
                for _ in 1..colspan.min(max_cols - column + 1) {
                    let td = placeholder(dom, true);
                    dom.insert_after(cell, td);
                }
                column += colspan;
            }
            while column < max_cols {
                let td = placeholder(dom, occupied[ri][column]);
                dom.append_child(row, td);
                column += 1;
            }
        }
        if dom.children_by_tag(table, "thead").is_empty() {
            let head_rows = dom.nodes[rows[0]]
                .children
                .iter()
                .copied()
                .filter(|&c| cell(dom, c))
                .map(|c| span(dom, c, "rowspan"))
                .max()
                .unwrap_or(1);
            if head_rows > 1 && head_rows <= rows.len() {
                let head = dom.create(DomType::Element, "thead");
                dom.insert_before(rows[0], head);
                for &row in rows.iter().take(head_rows) {
                    dom.append_child(head, row);
                }
            }
        }
    }
}

fn trim_table_cell_text(dom: &Dom, idx: usize, tokens: &str) -> String {
    let mut trimmed = tokens.trim().to_string();
    if !dom.nodes[idx]
        .parent
        .is_some_and(|p| dom.is_element(p, "td") || dom.is_element(p, "th"))
    {
        return trimmed;
    }
    let data = &dom.nodes[idx].data;
    let previous = dom.prev_sibling(idx);
    let next = dom.next_sibling(idx);
    if trimmed.is_empty() {
        return if previous.is_some() && next.is_some() && data.contains([' ', '\u{a0}']) {
            " ".to_string()
        } else {
            String::new()
        };
    }
    if previous.is_some_and(|p| !dom.is_element(p, "br")) && data.starts_with([' ', '\u{a0}']) {
        trimmed.insert(0, ' ');
    }
    if next.is_some_and(|p| !dom.is_element(p, "br")) && data.ends_with([' ', '\u{a0}']) {
        trimmed.push(' ');
    }
    trimmed
}

fn get_formula(dom: &Dom, idx: usize) -> Option<String> {
    let formula = dom.attr(idx, "data-tex");
    if !formula.trim().is_empty() {
        return Some(formula.trim().to_string());
    }
    dom.nodes[idx]
        .children
        .iter()
        .find_map(|&child| get_formula(dom, child))
}

// h2m_code_banner.go: consume a recognized language banner before its children
// can produce an ordinary paragraph. Existing pre/code languages take priority.
fn normalize_code_block_banner(dom: &mut Dom, idx: usize) {
    if !dom.is_element(idx, "div") || !has_class(dom, idx, "md-code-block") {
        return;
    }
    let (mut banner, mut pre) = (None, None);
    for &child in &dom.nodes[idx].children {
        if has_class(dom, child, "md-code-block-banner") {
            if banner.replace(child).is_some() {
                return;
            }
        } else if dom.is_element(child, "pre") && pre.replace(child).is_some() {
            return;
        }
    }
    let (Some(banner), Some(pre)) = (banner, pre) else {
        return;
    };
    let mut text = String::new();
    code_banner_text(dom, banner, &mut text);
    let language = text.trim();
    if !is_code_banner_language(language) {
        return;
    }
    let codes = dom.children_by_tag(pre, "code");
    if codes.len() > 1 {
        return;
    }
    let code = if let Some(&code) = codes.first() {
        code
    } else {
        let code = dom.create(DomType::Element, "code");
        for child in dom.nodes[pre].children.clone() {
            dom.append_child(code, child);
        }
        dom.append_child(pre, code);
        code
    };
    let pre_class = dom.attr(pre, "class");
    let has_pre_language = pre_class.contains("language-")
        || !dom.attr(pre, "data-language").is_empty()
        || (!pre_class.is_empty()
            && !pre_class.contains([' ', '\t', '\r', '\n', '-', '_'])
            && pre_class != "fallback"
            && pre_class != "chroma");
    if dom.attr(code, "class").is_empty() && !has_pre_language {
        dom.set_attr(code, "class", &format!("language-{language}"));
    }
    dom.unlink(banner);
}

fn code_banner_text(dom: &Dom, idx: usize, text: &mut String) {
    if ["button", "a", "svg"]
        .iter()
        .any(|tag| dom.is_element(idx, tag))
        || dom.attr(idx, "role") == "button"
        || dom.attr(idx, "aria-hidden") == "true"
        || has_class(dom, idx, "ds-icon-button")
    {
        return;
    }
    if dom.nodes[idx].typ == DomType::Text {
        text.push_str(&dom.nodes[idx].data);
        text.push(' ');
    }
    for &child in &dom.nodes[idx].children {
        code_banner_text(dom, child, text);
    }
}

fn is_code_banner_language(language: &str) -> bool {
    !language.is_empty()
        && language.len() <= 64
        && language.bytes().enumerate().all(|(i, byte)| {
            byte.is_ascii_alphabetic()
                || (i > 0 && (byte.is_ascii_digit() || b"#+.".contains(&byte)))
        })
        && !matches!(
            language.to_ascii_lowercase().as_str(),
            "copy" | "copied" | "expand" | "collapse" | "download" | "run"
        )
}

fn has_class(dom: &Dom, idx: usize, class: &str) -> bool {
    dom.attr(idx, "class")
        .split_whitespace()
        .any(|name| name == class)
}

fn descendant_by_class(dom: &Dom, idx: usize, class: &str) -> Option<usize> {
    for &child in &dom.nodes[idx].children {
        if has_class(dom, child, class) {
            return Some(child);
        }
        if let Some(found) = descendant_by_class(dom, child, class) {
            return Some(found);
        }
    }
    None
}

fn insert_dom_br_before(dom: &mut Dom, idx: usize) {
    let br = dom.create(DomType::Element, "br");
    dom.insert_before(idx, br);
}

fn remove_dom_comments(dom: &mut Dom, idx: usize) {
    for child in dom.nodes[idx].children.clone() {
        if dom.nodes[child].typ == DomType::Comment {
            dom.unlink(child);
        } else {
            remove_dom_comments(dom, child);
        }
    }
}

fn code_text_only(dom: &Dom, idx: usize) -> bool {
    for &child in &dom.nodes[idx].children {
        match dom.nodes[child].typ {
            DomType::Text => continue,
            DomType::Element => {
                let tag = dom.nodes[child].data.as_str();
                if !matches!(
                    tag,
                    "abbr"
                        | "b"
                        | "br"
                        | "del"
                        | "em"
                        | "i"
                        | "kbd"
                        | "mark"
                        | "p"
                        | "q"
                        | "s"
                        | "samp"
                        | "small"
                        | "span"
                        | "strike"
                        | "strong"
                        | "sub"
                        | "sup"
                        | "time"
                        | "u"
                        | "var"
                ) {
                    return false;
                }
                if !code_text_only(dom, child) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

pub(crate) fn table_cell_align_value(align: &str, style: &str) -> i32 {
    let value = match align.trim().to_lowercase().as_str() {
        "left" => 1,
        "center" => 2,
        "right" => 3,
        _ => 0,
    };
    if let Some(v) = table_cell_style_align(style) {
        return v;
    }
    value
}

fn table_cell_style_align(style: &str) -> Option<i32> {
    let mut alignment = None;
    for declaration in style.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        if !property.trim().eq_ignore_ascii_case("text-align") {
            continue;
        }
        let value = value
            .trim()
            .strip_suffix("!important")
            .unwrap_or(value.trim())
            .trim()
            .to_lowercase();
        alignment = Some(match value.as_str() {
            "left" => 1,
            "center" => 2,
            "right" => 3,
            _ => 0,
        });
    }
    alignment
}

#[cfg(test)]
mod go_style_structural_tests {
    use crate::ast::{walk_raw, NodeType, WalkStatus};
    use crate::lute::Lute;
    use crate::render::Renderer;

    fn engine() -> Lute {
        let mut engine = Lute::new();
        engine.parse_options.protyle_wysiwyg = true;
        engine.render_options.protyle_wysiwyg = true;
        engine.parse_options.html_tag2_text_mark = true;
        engine.parse_options.kramdown_block_ial = true;
        engine.parse_options.kramdown_span_ial = true;
        engine.render_options.kramdown_block_ial = true;
        engine.render_options.kramdown_span_ial = true;
        engine
    }

    // Go: test/html_text_style_test.go::TestHTMLQuotedFontFamilyRoundTrip
    #[test]
    fn go_html_quoted_font_family_round_trip() {
        let mut engine = engine();
        engine.parse_options.heading_id = false;
        engine.render_options.heading_id = false;
        engine.render_options.auto_space = false;
        for input in [
            r#"<p style='color:red;font-family:"Mona Sans VF", "Segoe UI", Arial;font-size:14px'>before <strong>bold</strong> after</p>"#,
            r#"<h2 style="color:red;font-family:&quot;Mona Sans VF&quot;, &quot;Segoe UI&quot;, Arial;font-size:14px">before <strong>bold</strong> after</h2>"#,
            r#"<table><tr><td style='color:red;font-family:"Mona Sans VF", "Segoe UI", Arial;font-size:14px'>before <strong>bold</strong> after</td></tr></table>"#,
        ] {
            let mut dom = engine.html2blockdom(input);
            for round in 0..3 {
                assert!(
                    !dom.contains("{: style="),
                    "round {round} leaked attributes: {dom}"
                );
                let tree = engine.blockdom2tree(&dom);
                let mut styled = 0;
                walk_raw(tree.root_raw(), &mut |n, entering| {
                    if entering && n.typ() == NodeType::NodeTextMark {
                        styled += 1;
                        let style = n.ial_attr("style");
                        for want in [
                            r#"font-family: "Mona Sans VF", "Segoe UI", Arial;"#,
                            "color: red;",
                            "font-size: 14.000000px;",
                        ] {
                            assert!(style.contains(want), "round {round} lost {want:?}: {dom}");
                        }
                        if n.data().text_mark_text_content == "bold" {
                            assert!(n.is_text_mark_type("strong"), "bold was lost: {dom}");
                        }
                    }
                    WalkStatus::WalkContinue
                });
                assert!(styled >= 3, "text fragments were lost: {dom}");
                dom = engine.spin_blockdom(&dom);
            }
        }
    }

    // Go: test/html_text_style_test.go::TestMarkdownHTMLStyleBoundaries
    #[test]
    fn go_markdown_html_style_boundaries() {
        let engine = Lute::new();
        for input in [
            "`<small>x</small>`",
            "```html\n<small>x</small>\n```",
            "before <small>unclosed",
        ] {
            let mut tree = crate::parse::parse("", input.as_bytes(), engine.parse_options.clone());
            crate::parse::html_text_style::normalize_inline_html_text_styles(&mut tree);
            walk_raw(tree.root_raw(), &mut |n, entering| {
                if entering && n.typ() == NodeType::NodeTextMark {
                    assert!(
                        n.ial_attr("style").is_empty(),
                        "unexpected style in {input}"
                    );
                }
                WalkStatus::WalkContinue
            });
        }
        let input = r#"<small>A &amp; <a href="java&#x09;script:alert(1)">B</a></small>"#;
        let mut tree = crate::parse::parse("", input.as_bytes(), engine.parse_options.clone());
        crate::parse::html_text_style::normalize_inline_html_text_styles(&mut tree);
        walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering && n.typ() == NodeType::NodeTextMark {
                assert!(n.data().text_mark_a_href.is_empty(), "unsafe link retained");
            }
            WalkStatus::WalkContinue
        });
        let input = r#"<span data-type="block-ref" data-id="20260101000000-abcdefg">reference</span> <small>small</small>"#;
        let mut tree = crate::parse::parse("", input.as_bytes(), engine.parse_options.clone());
        crate::parse::html_text_style::normalize_inline_html_text_styles(&mut tree);
        let mut protected = false;
        walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering
                && n.typ() == NodeType::NodeInlineHTML
                && n.tokens_str().contains("data-type=\"block-ref\"")
            {
                protected = true;
            }
            WalkStatus::WalkContinue
        });
        assert!(
            protected,
            "native text mark was consumed by external style conversion"
        );
    }

    // Go: test/html_text_style_test.go::TestMarkdownHTMLTextStyle
    #[test]
    fn go_markdown_html_text_style() {
        let engine = Lute::new();
        let mut failures = Vec::new();
        for input in [
            r#"A<small>B **C** <small>D</small> [E](https://example.com)</small>F"#,
            r#"A<span style="color:red;font-family:Arial">B<strong>C</strong></span>F"#,
        ] {
            let mut tree = crate::parse::parse("", input.as_bytes(), engine.parse_options.clone());
            crate::parse::html_text_style::normalize_inline_html_text_styles(&mut tree);
            let mut styled = 0;
            walk_raw(tree.root_raw(), &mut |n, entering| {
                if entering && n.typ() == NodeType::NodeTextMark && !n.ial_attr("style").is_empty()
                {
                    styled += 1;
                }
                if entering && n.typ() == NodeType::NodeInlineHTML {
                    failures.push(format!("unconverted HTML: {}", n.tokens_str()));
                }
                WalkStatus::WalkContinue
            });
            if styled == 0 {
                failures.push(format!("no styled text: {input}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    // Go: test/flashcard_occlusion_test.go::TestFlashcardOcclusionIDRoundTrip
    // Partial: Rust has no ProtyleExportRenderer; all other original assertions remain.
    #[test]
    fn go_flashcard_occlusion_id_round_trip() {
        let mut engine = Lute::new();
        engine.parse_options.html_tag2_text_mark = true;
        engine.parse_options.protyle_wysiwyg = true;
        engine.render_options.protyle_wysiwyg = true;
        let dom = r#"<div data-node-id="20260812000000-abcdefg" data-type="NodeParagraph" class="p"><div contenteditable="true"><span data-type="mark" data-occlusion-id="20260812000001-occla01">alpha</span><span data-type="mark" data-occlusion-id="20260812000002-occlb02">beta</span></div><div class="protyle-attr" contenteditable="false"></div></div>"#;
        let tree = engine.blockdom2tree(dom);
        let first = tree
            .root_raw()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap();
        assert_eq!(
            first.data().text_mark_flashcard_occlusion_id,
            "20260812000001-occla01"
        );
        assert_eq!(
            first
                .next()
                .unwrap()
                .data()
                .text_mark_flashcard_occlusion_id,
            "20260812000002-occlb02"
        );
        let node = tree.root().first_child().unwrap();
        let rendered = engine.render_node_blockdom(&node);
        let markdown = engine.blockdom2md(dom);
        let round_trip = engine.md2blockdom(&markdown, false);
        for (kind, output) in [
            ("rendered", &rendered),
            ("Markdown", &markdown),
            ("round trip", &round_trip),
        ] {
            for want in [
                "data-occlusion-id=\"20260812000001-occla01\"",
                "data-occlusion-id=\"20260812000002-occlb02\"",
            ] {
                assert!(output.contains(want), "{kind} lost {want}: {output}");
            }
        }
        let tree = engine.blockdom2tree(dom);
        let html = engine.tree2html(&tree);
        assert!(
            !html.contains("data-occlusion-id"),
            "standard HTML should not expose flashcard occlusion IDs: {html}"
        );
        let tree = engine.blockdom2tree(dom);
        let mut renderer = crate::render::protyle_preview::ProtylePreviewRenderer::new(
            &tree,
            engine.render_options.clone(),
            engine.parse_options.clone(),
        );
        let html = String::from_utf8_lossy(&renderer.render()).into_owned();
        assert!(
            !html.contains("data-occlusion-id"),
            "preview HTML should not expose flashcard occlusion IDs: {html}"
        );
    }
}

#[cfg(test)]
mod source_alignment_tests {
    #[test]
    fn code_banner_language_limits_and_ambiguous_code_nodes() {
        for language in ["C++", "C#", "go1.24", &"a".repeat(64)] {
            assert!(is_code_banner_language(language), "{language}");
        }
        for language in [
            "",
            "1go",
            "a-b",
            "a_b",
            "bash script",
            "复制",
            "COPY",
            "Run",
            &"a".repeat(65),
        ] {
            assert!(!is_code_banner_language(language), "{language}");
        }
        let mut dom = crate::dom::parse(
            r#"<div class="md-code-block"><div class="md-code-block-banner"><a>download</a><i aria-hidden="true">icon</i><span>go</span></div><pre><code>a</code><code>b</code></pre></div>"#,
        );
        let block = dom.nodes[dom.root].children[0];
        let original = dom.html(block);
        normalize_code_block_banner(&mut dom, block);
        assert_eq!(
            dom.html(block),
            original,
            "ambiguous code elements must be preserved"
        );
        let pre = dom.child_by_tag(block, "pre").unwrap();
        let codes = dom.children_by_tag(pre, "code");
        dom.unlink(codes[1]);
        normalize_code_block_banner(&mut dom, block);
        assert_eq!(dom.attr(codes[0], "class"), "language-go");
        assert!(dom.child_by_tag(block, "div").is_none());
        let normalized = dom.html(block);
        normalize_code_block_banner(&mut dom, block);
        assert_eq!(dom.html(block), normalized);
    }

    #[test]
    fn exported_dom_attributes_follow_the_common_primary_node() {
        use crate::ast::{walk_raw, WalkStatus};
        for (tag, body, typ) in [
            ("p", "text", NodeType::NodeParagraph),
            ("h2", "text", NodeType::NodeHeading),
            ("blockquote", "text", NodeType::NodeBlockquote),
            ("ul", "<li>text</li>", NodeType::NodeList),
            ("pre", "<code>text</code>", NodeType::NodeCodeBlock),
            ("strong", "text", NodeType::NodeStrong),
            ("img", "", NodeType::NodeImage),
            ("table", "<tr><td>text</td></tr>", NodeType::NodeTable),
        ] {
            let options = ParseOptions {
                html2_markdown_attrs: vec!["name".into()],
                ..Default::default()
            };
            let input = format!("<{tag} name=\"label\" custom-extra=\"yes\" memo=\"note\" style=\"color: red;\n font-size: 12px;\">{body}</{tag}>");
            let tree = html2tree(&input, options);
            let mut roots = Vec::new();
            let mut ials = Vec::new();
            walk_raw(tree.root_raw(), &mut |node, entering| {
                if entering && node.typ() == typ {
                    roots.push(node);
                }
                if entering && node.typ() == NodeType::NodeKramdownBlockIAL {
                    ials.push(node);
                }
                WalkStatus::WalkContinue
            });
            assert_eq!(roots.len(), 1, "{tag}");
            assert_eq!(roots[0].ial_attr("name"), "label", "{tag}");
            assert_eq!(roots[0].ial_attr("custom-extra"), "yes", "{tag}");
            assert_eq!(roots[0].ial_attr("memo"), "note", "{tag}");
            assert_eq!(
                roots[0].ial_attr("style"),
                "color: red; font-size: 12px;",
                "{tag}"
            );
            assert_eq!(ials.len(), 1, "{tag}");
            assert_eq!(
                ials[0].tokens(),
                crate::parse::ial::ial_to_tokens(&roots[0].ial_raw()),
                "{tag}"
            );
        }
        let options = ParseOptions {
            html2_markdown_attrs: vec!["custom-*".into()],
            ..Default::default()
        };
        let tree = html2tree("<p name=\"not selected\">plain</p><custom-tag custom-name=\"ignored\">custom</custom-tag>", options);
        assert!(tree
            .root_raw()
            .children()
            .all(|node| node.ial_raw().is_empty()));
    }
    use super::*;

    #[test]
    fn code_comments_are_removed_before_text_only_classification() {
        let mut dom = crate::dom::parse("<code><!--a--><span>x<!--b-->y</span><!--c--></code>");
        let code = dom.children_by_tag(dom.root, "code")[0];
        remove_dom_comments(&mut dom, code);
        assert!(code_text_only(&dom, code));
        assert_eq!(dom.text(code), "xy");
        assert!(!dom.html(code).contains("<!--"));
        let mut dom = crate::dom::parse("<code><!--a--><span>x<img src=\"x.svg\">y</span></code>");
        let code = dom.children_by_tag(dom.root, "code")[0];
        remove_dom_comments(&mut dom, code);
        assert!(!code_text_only(&dom, code));
        assert!(dom.html(code).contains("<img"));
    }

    #[test]
    fn merged_table_grid_preserves_spans_and_fills_covered_positions() {
        let mut dom = crate::dom::parse("<table><tr><td rowspan=\"2\">a</td><td colspan=\"2\">b</td></tr><tr><td>c</td><td>d</td></tr></table>");
        fix_table_structure(&mut dom);
        let rows = dom.children_by_tag(dom.root, "tr");
        for &row in &rows {
            assert_eq!(dom.nodes[row].children.len(), 3);
        }
        assert_eq!(
            dom.attr(dom.nodes[rows[0]].children[2], "class"),
            "fn__none"
        );
        assert_eq!(
            dom.attr(dom.nodes[rows[1]].children[0], "class"),
            "fn__none"
        );
        let heads = dom.children_by_tag(dom.root, "thead");
        assert_eq!(heads.len(), 1);
        assert_eq!(dom.nodes[heads[0]].children, rows);

        // Invalid spans default to one; ragged cells are ordinary empty cells.
        let mut dom = crate::dom::parse(
            "<table><tr><td colspan=\"0\">a</td></tr><tr><td>b</td><td>c</td></tr></table>",
        );
        fix_table_structure(&mut dom);
        let row = dom.children_by_tag(dom.root, "tr")[0];
        assert_eq!(dom.nodes[row].children.len(), 2);
        assert_eq!(dom.attr(dom.nodes[row].children[1], "class"), "");
    }

    #[test]
    fn table_text_boundaries_keep_inline_spaces_but_trim_after_breaks() {
        let dom = crate::dom::parse("<table><tr><td><b>a</b> b <i>c</i><br> d </td></tr></table>");
        let cell = dom.children_by_tag(dom.root, "td")[0];
        let between = dom.nodes[cell].children[1];
        let after_br = dom.nodes[cell].children[4];
        assert_eq!(trim_table_cell_text(&dom, between, " b "), " b ");
        assert_eq!(trim_table_cell_text(&dom, after_br, " d "), "d");
    }

    #[test]
    fn normalized_code_and_suppressed_wrappers_retain_visible_content() {
        let engine = crate::lute::Lute::new();
        for (input, expected) in [
            ("<p><strong><strong>visible</strong></strong></p>", "**visible**\n"),
            ("<a href=\"x\"><h3>visible</h3></a>", "[visible](x)\n"),
            ("<table><tr><td><p>visible</p></td></tr></table>", "| visible |\n| --------- |\n"),
            ("<pre><div class=\"cm-content\" data-language=\"go\"><div class=\"cm-line\">a</div><div class=\"cm-line\"></div><div class=\"cm-line\">b</div></div></pre>", "```go\na\n\nb\n```\n"),
        ] {
            assert_eq!(engine.html2md(input), expected, "{input}");
        }
    }
}
