// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Protyle Block DOM 与 AST 之间的转换，对齐 Go 版 protyle.go 的
//! BlockDOM2Tree / genASTByBlockDOM。

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::dom::{Dom, DomType};
use crate::editor;
use crate::html as html_util;
use crate::options::ParseOptions;

/// 对应 Go 版 Lute.BlockDOM2Tree。
pub fn blockdom2tree(html: &str, options: ParseOptions) -> Tree {
    blockdom2tree_with_render_options(html, options, &crate::options::RenderOptions::new())
}

pub(crate) fn blockdom2tree_with_render_options(
    html: &str,
    options: ParseOptions,
    render_options: &crate::options::RenderOptions,
) -> Tree {
    let mut html = html.to_string();
    for (from, to) in [
        ("\n<wbr>\n</strong>", "</strong>\n<wbr>\n"),
        ("\n<wbr>\n</em>", "</em>\n<wbr>\n"),
        ("\n<wbr>\n</s>", "</s>\n<wbr>\n"),
        ("\n<wbr>\n</u>", "</u>\n<wbr>\n"),
        ("\n<wbr>\n</span>", "</span>\n<wbr>\n"),
    ] {
        html = html.replace(from, to);
    }
    html = html.replace("<wbr>", editor::CARET);

    let mut start_spaces = 0;
    for c in html.chars() {
        if c == ' ' {
            start_spaces += 1;
        } else {
            break;
        }
    }
    let mut end_spaces = 0;
    for c in html.chars().rev() {
        if c == ' ' {
            end_spaces += 1;
        } else {
            break;
        }
    }
    html = html.trim().to_string();
    html = format!(
        "{}{}{}",
        "&nbsp;".repeat(start_spaces),
        html,
        "&nbsp;".repeat(end_spaces)
    );
    while html.ends_with("\t\n") {
        html = format!("{}\n", &html[..html.len() - 2]);
    }
    while html.ends_with("    \n") {
        html = format!("{}\n", &html[..html.len() - 5]);
    }

    let mut dom = crate::dom::parse(&html);
    // 对应 Go 版 BlockDOM2Tree 中的两个前置规范化与 adjustVditorDOM(htmlRoot)。
    let dom_root = dom.root;
    crate::vditor::normalize_protyle_inline_code_caret(&mut dom, dom_root);
    crate::vditor::normalize_protyle_inline_caret_placeholders(&mut dom, dom_root);
    crate::vditor::adjust_vditor_dom(&mut dom, &options);

    let mut tree = Tree::new();
    let root = tree.root_raw();
    let children = dom.nodes[dom.root].children.clone();
    {
        let mut b = Builder {
            dom: &mut dom,
            tree: &mut tree,
            tip: root,
            options,
            render_options: render_options.clone(),
        };
        for c in children {
            b.gen(c);
        }
    }
    post_process_blockdom_tree(&tree);
    tree
}

/// 对应 Go 版 BlockDOM2Tree 末尾的 ast.Walk 调整：
/// 合并相邻代码节点/强调节点/文本标记，并把不换行空格还原为普通空格。
fn post_process_blockdom_tree(tree: &Tree) {
    let lute = crate::lute::Lute::new();
    let mut nodes = Vec::new();
    crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering {
            nodes.push(n);
        }
        crate::ast::WalkStatus::WalkContinue
    });

    for n in nodes {
        if n.parent().is_none() && n != tree.root_raw() {
            // 已在合并中被删除的节点。
            continue;
        }
        match n.typ() {
            NodeType::NodeInlineHTML
            | NodeType::NodeHTMLBlock
            | NodeType::NodeCodeSpanContent
            | NodeType::NodeCodeBlockCode
            | NodeType::NodeInlineMathContent
            | NodeType::NodeMathBlockContent
            | NodeType::NodeCodeSpan
            | NodeType::NodeInlineMath => {
                let Some(next) = n.next() else { continue };
                if NodeType::NodeCodeSpan != next.typ()
                    || n.data().code_marker_len != next.data().code_marker_len
                {
                    continue;
                }
                let Some(next_content) = next.first_child().and_then(|c| c.next()) else {
                    continue;
                };
                let Some(target) = n.first_child().and_then(|c| c.next()) else {
                    continue;
                };
                let mut tokens = target.tokens().to_vec();
                tokens.extend_from_slice(next_content.tokens());
                target.set_tokens(tokens);
                next.unlink();
            }
            NodeType::NodeStrong
            | NodeType::NodeEmphasis
            | NodeType::NodeStrikethrough
            | NodeType::NodeUnderline => lute.merge_same_span_raw(n),
            NodeType::NodeTextMark => lute.merge_same_text_mark_raw(n),
            NodeType::NodeText => {
                // 对应 Go 版 bytes.ReplaceAll(tokens, []byte("\u00a0"), []byte(" "))：
                // 匹配的是 U+00A0 的 UTF-8 编码（C2 A0），不是单字节 0xA0。
                let tokens = n.tokens();
                if tokens.windows(2).any(|w| w == [0xc2, 0xa0]) {
                    n.set_tokens(replace_all(tokens, &[0xc2, 0xa0], b" "));
                }
            }
            _ => {}
        }
    }
}

/// 对应 Go 版 Lute.Tree2BlockDOM。
pub fn tree2blockdom(
    tree: &Tree,
    render_options: crate::options::RenderOptions,
    parse_options: ParseOptions,
) -> String {
    tree2blockdom_ext(tree, render_options, parse_options, None).replace(editor::CARET, "<wbr>")
}

/// Protyle 渲染原语，支持自定义渲染器。对应 Go 直接调用 renderer.Render()；
/// Tree2BlockDOM 包装层负责替换光标，Md2BlockDOM/HTML2BlockDOM 保留原始光标。
pub fn tree2blockdom_ext(
    tree: &Tree,
    render_options: crate::options::RenderOptions,
    parse_options: ParseOptions,
    ext: Option<crate::render::ExtRendererFunc>,
) -> String {
    let mut r = crate::render::protyle::ProtyleRenderer::new(tree, render_options, parse_options);
    r.base.ext_renderer = ext;
    use crate::render::Renderer;
    String::from_utf8_lossy(&r.render()).to_string()
}

struct Builder<'a> {
    dom: &'a mut Dom,
    tree: &'a mut Tree,
    tip: Node,
    options: ParseOptions,
    render_options: crate::options::RenderOptions,
}

impl<'a> Builder<'a> {
    fn direct_child_by_class(&self, idx: usize, class: &str) -> Option<usize> {
        self.dom.nodes[idx].children.iter().copied().find(|&child| {
            self.dom
                .attr(child, "class")
                .split_whitespace()
                .any(|name| name == class)
        })
    }

    /// Go tabTitleMarkdown protects boundary spaces before rendering inline DOM.
    fn tab_title_markdown(&self, title: usize) -> String {
        crate::dom::tab_title_markdown(self.dom, title, |html| {
            blockdom2md_with_render_options(html, self.options.clone(), &self.render_options)
        })
    }

    fn new_node_raw(&mut self, typ: NodeType) -> Node {
        self.tree.new_node_raw(typ)
    }

    fn gen(&mut self, idx: usize) {
        let class = self.dom.attr(idx, "class");
        if class.contains("protyle-custom") {
            return;
        }
        if class == "protyle-attr"
            || class == "callout-info"
            || class.contains("tab-item-info")
            || class.contains("__copy")
            || class.contains("sb__resize")
            || class.contains("protyle-linenumber__rows")
            || class.contains("hljs")
        {
            return;
        }
        if self.dom.attr(idx, "spin") == "1" {
            return;
        }
        if class.contains("protyle-action") {
            self.gen_action(idx);
            return;
        }
        if self.dom.attr(idx, "contenteditable") == "true" {
            self.gen_contenteditable(idx);
            return;
        }

        let data_type = self.dom.attr(idx, "data-type");
        let node_id = self.dom.attr(idx, "data-node-id");
        let has_id = !node_id.is_empty() && !self.dom.parent_is(idx, &["table"]);

        match data_type.as_str() {
            "NodeTabs" | "NodeTabItem" => {
                let typ = if data_type == "NodeTabs" {
                    NodeType::NodeTabs
                } else {
                    NodeType::NodeTabItem
                };
                let node = self.new_node_raw(typ);
                node.data_mut().id = node_id.clone();
                if typ == NodeType::NodeTabItem {
                    if let Some(info) = self.direct_child_by_class(idx, "tab-item-info") {
                        if let Some(title) = self.direct_child_by_class(info, "tab-item-title") {
                            node.data_mut().tab_item_title = self.tab_title_markdown(title);
                        }
                    }
                }
                self.tip.append_child(node);
                self.tip = node;
                if typ == NodeType::NodeTabItem {
                    if let Some(info) = self.direct_child_by_class(idx, "tab-item-info") {
                        for child in self.dom.nodes[info].children.clone() {
                            if self.dom.attr(child, "data-type") == "NodeParagraph"
                                && self.dom.attr(child, "tabs-title") == "true"
                            {
                                self.gen(child);
                                break;
                            }
                        }
                    }
                }
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeBlockQueryEmbed" => {
                let node = self.new_node_raw(NodeType::NodeBlockQueryEmbed);
                node.data_mut().id = node_id.clone();
                node.append_child(self.new_node_raw(NodeType::NodeOpenBrace));
                node.append_child(self.new_node_raw(NodeType::NodeOpenBrace));
                let mut content = self.dom.attr(idx, "data-content");
                content = content.replace('\n', editor::IAL_VAL_ESC_NEWLINE);
                let script = self.new_node_raw(NodeType::NodeBlockQueryEmbedScript);
                script.set_tokens(content.into_bytes());
                node.append_child(script);
                node.append_child(self.new_node_raw(NodeType::NodeCloseBrace));
                node.append_child(self.new_node_raw(NodeType::NodeCloseBrace));
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeTable" => {
                self.gen_table(idx, &node_id, has_id);
                return;
            }
            "NodeParagraph" => {
                let node = self.new_node_raw(NodeType::NodeParagraph);
                node.data_mut().id = node_id.clone();
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeHeading" => {
                let text = self.dom.text(idx);
                let mut level = self
                    .dom
                    .attr(idx, "data-subtype")
                    .chars()
                    .nth(1)
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1) as i32;
                let tmp = text.trim_start_matches(' ');
                if tmp.starts_with('#') {
                    if let Some(pos) = tmp.find(&format!(" {}", editor::CARET)) {
                        if pos > 0 {
                            if let Some(first) = self.dom.nodes[idx]
                                .children
                                .first()
                                .copied()
                                .and_then(|child| self.dom.nodes[child].children.first().copied())
                            {
                                let content = self.dom.nodes[first].data.clone();
                                let content = content.strip_prefix(&tmp[..pos]).unwrap_or(&content);
                                self.dom.nodes[first].data =
                                    content.strip_prefix(' ').unwrap_or(content).to_string();
                            }
                            level = tmp[..pos].matches('#').count() as i32;
                        }
                    }
                }
                let node = self.new_node_raw(NodeType::NodeHeading);
                node.data_mut().id = node_id.clone();
                node.data_mut().heading_level = level;
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeBlockquote" => {
                let node = self.new_node_raw(NodeType::NodeBlockquote);
                node.data_mut().id = node_id.clone();
                let marker = self.new_node_raw(NodeType::NodeBlockquoteMarker);
                marker.set_tokens(b">".to_vec());
                node.append_child(marker);
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeList" | "NodeMindmap" => {
                let typ = if data_type == "NodeList" {
                    NodeType::NodeList
                } else {
                    NodeType::NodeMindmap
                };
                let node = self.new_node_raw(typ);
                node.data_mut().id = node_id.clone();
                let subtype = self.dom.attr(idx, "data-subtype");
                let ld = crate::ast::ListData {
                    typ: match subtype.as_str() {
                        "u" => 0,
                        "o" => 1,
                        "t" => 3,
                        _ => 0,
                    },
                    marker: self.dom.attr(idx, "data-marker").into_bytes(),
                    ..Default::default()
                };
                node.data_mut().list_data = Some(ld);
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeListItem" | "NodeMindmapItem" => {
                self.gen_list_item(idx, &data_type, &node_id, has_id);
                return;
            }
            "NodeSuperBlock" => {
                let node = self.new_node_raw(NodeType::NodeSuperBlock);
                node.data_mut().id = node_id.clone();
                self.tip.append_child(node);
                node.append_child(self.new_node_raw(NodeType::NodeSuperBlockOpenMarker));
                let layout = self.new_node_raw(NodeType::NodeSuperBlockLayoutMarker);
                layout.set_tokens(self.dom.attr(idx, "data-sb-layout").into_bytes());
                node.append_child(layout);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeSuperBlockCloseMarker));
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeMathBlock" => {
                let node = self.new_node_raw(NodeType::NodeMathBlock);
                node.data_mut().id = node_id.clone();
                node.append_child(self.new_node_raw(NodeType::NodeMathBlockOpenMarker));
                let content = self.new_node_raw(NodeType::NodeMathBlockContent);
                content.set_tokens(self.dom.attr(idx, "data-content").into_bytes());
                node.append_child(content);
                node.append_child(self.new_node_raw(NodeType::NodeMathBlockCloseMarker));
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeCodeBlock" => {
                let node = self.new_node_raw(NodeType::NodeCodeBlock);
                node.data_mut().id = node_id.clone();
                node.data_mut().is_fenced_code_block = true;
                let open = self.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                open.set_tokens(vec![0x60u8; 3]);
                node.append_child(open);
                let language = self.dom.attr(idx, "data-subtype");
                if !language.is_empty() {
                    let info = self.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                    info.data_mut().code_block_info = language.into_bytes();
                    node.append_child(info);
                    let code = self.new_node_raw(NodeType::NodeCodeBlockCode);
                    code.set_tokens(self.dom.attr(idx, "data-content").into_bytes());
                    node.append_child(code);
                    let close = self.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                    close.set_tokens(vec![0x60u8; 3]);
                    node.append_child(close);
                    self.tip.append_child(node);
                    self.after_block(idx, node, has_id);
                    return;
                }
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                let close = self.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                close.set_tokens(vec![0x60u8; 3]);
                node.append_child(close);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeHTMLBlock" => {
                let node = self.new_node_raw(NodeType::NodeHTMLBlock);
                node.data_mut().id = node_id.clone();
                // 对应 Go 版 n.FirstChild.NextSibling.FirstChild：跳过 protyle-icons，
                // 取 protyle-html 节点上的 data-content。
                let content = self.dom.nodes[idx]
                    .children
                    .get(1)
                    .and_then(|&c| self.dom.nodes[c].children.first().copied())
                    .map(|c| self.dom.attr(c, "data-content"))
                    .unwrap_or_default();
                node.set_tokens(content.into_bytes());
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeYamlFrontMatter" => {
                let node = self.new_node_raw(NodeType::NodeYamlFrontMatter);
                node.data_mut().id = node_id.clone();
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeThematicBreak" => {
                let node = self.new_node_raw(NodeType::NodeThematicBreak);
                node.data_mut().id = node_id.clone();
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeIFrame" | "NodeWidget" | "NodeVideo" | "NodeAudio" => {
                let typ = match data_type.as_str() {
                    "NodeIFrame" => NodeType::NodeIFrame,
                    "NodeWidget" => NodeType::NodeWidget,
                    "NodeVideo" => NodeType::NodeVideo,
                    _ => NodeType::NodeAudio,
                };
                let node = self.new_node_raw(typ);
                node.data_mut().id = node_id.clone();
                let tag = match typ {
                    NodeType::NodeIFrame | NodeType::NodeWidget => "iframe",
                    NodeType::NodeVideo => "video",
                    _ => "audio",
                };
                let inner = self.dom.nodes[idx]
                    .children
                    .first()
                    .copied()
                    .and_then(|c| self.dom.child_by_tag(c, tag));
                if let Some(inner) = inner {
                    node.set_tokens(self.dom.html(inner).into_bytes());
                }
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeAttributeView" => {
                let node = self.new_node_raw(NodeType::NodeAttributeView);
                node.data_mut().id = node_id.clone();
                let mut av_id = self.dom.attr(idx, "data-av-id");
                if av_id.is_empty() {
                    av_id = crate::ast::new_node_id();
                }
                node.data_mut().attribute_view_id = av_id;
                node.data_mut().attribute_view_type = self.dom.attr(idx, "data-av-type");
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeCustomBlock" => {
                let node = self.new_node_raw(NodeType::NodeCustomBlock);
                node.data_mut().id = node_id.clone();
                node.data_mut().custom_block_info = self.dom.attr(idx, "data-info");
                node.set_tokens(self.dom.attr(idx, "data-content").into_bytes());
                self.tip.append_child(node);
                self.after_block(idx, node, has_id);
                return;
            }
            "NodeCallout" => {
                self.gen_callout(idx, &node_id, has_id);
                return;
            }
            _ => {}
        }

        if class == "callout-content" || class.contains("tab-item-content") {
            self.gen_children(idx);
            return;
        }
        if self.dom.nodes[idx].typ != DomType::Element
            || !crate::dom::is_known_tag(&self.dom.nodes[idx].data)
        {
            if self.tip.typ() == NodeType::NodeDocument {
                let p = self.new_node_raw(NodeType::NodeParagraph);
                self.tip.append_child(p);
                self.tip = p;
            }
            self.gen_contenteditable(idx);
            return;
        }
        let tag = self.dom.nodes[idx].data.clone();
        if matches!(
            tag.as_str(),
            "u" | "code" | "strong" | "em" | "kbd" | "mark" | "s" | "sub" | "sup" | "span"
        ) {
            if self.tip.typ() == NodeType::NodeDocument {
                let p = self.new_node_raw(NodeType::NodeParagraph);
                self.tip.append_child(p);
                self.tip = p;
            }
            self.gen_contenteditable(idx);
            return;
        }
        if self.tip.typ() == NodeType::NodeListItem && tag == "input" {
            let node = self.new_node_raw(NodeType::NodeTaskListItemMarker);
            revive_task_marker(
                node,
                &self.dom.attr(idx, "data-task"),
                self.dom.has_attr(idx, "checked"),
            );
            self.tip.append_child(node);
            return;
        }
        let node = self.new_node_raw(NodeType::NodeInlineHTML);
        node.set_tokens(self.dom.html(idx).into_bytes());
        self.tip.append_child(node);
    }

    fn gen_children(&mut self, idx: usize) {
        let children = self.dom.nodes[idx].children.clone();
        for c in children {
            self.gen(c);
        }
    }

    fn leave(&mut self) {
        if let Some(p) = self.tip.parent() {
            self.tip = p;
        }
    }

    fn after_block(&mut self, idx: usize, node: Node, has_id: bool) {
        if has_id {
            let id = node.data().id.clone();
            node.set_ial_attr("id", &id);
            self.set_block_ial(idx, node);
            let tokens = ial2_tokens(&node.data().kramdown_ial);
            let ial = self.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(tokens);
            self.tip.append_child(ial);
        }
    }

    fn set_block_ial(&mut self, idx: usize, node: Node) {
        for name in [
            "tabs-active-id",
            "tabs-position",
            "tabs-title",
            "tabs-placeholder",
            "tabs-task",
            "caption",
            "icon",
            "refcount",
            "av-names",
            "bookmark",
            "style",
            "name",
            "memo",
            "alias",
            "fold",
            "heading-fold",
            "parent-fold",
            "updated",
            "linewrap",
            "ligatures",
            "linenumber",
            "breadcrumb",
            "data-export-md",
            "data-export-html",
        ] {
            let mut value = self.dom.attr(idx, name);
            if value.is_empty() {
                continue;
            }
            if matches!(
                name,
                "bookmark"
                    | "style"
                    | "name"
                    | "memo"
                    | "alias"
                    | "data-export-md"
                    | "data-export-html"
            ) {
                value = html_util::unescape_string(&value);
            }
            if name == "style" {
                value = style_value(&value);
            }
            node.set_ial_attr(name, &value);
        }
        for (k, v) in self.dom.nodes[idx].attrs.clone() {
            if k.starts_with("custom-") {
                node.set_ial_attr(&k, &html_util::unescape_string(&v));
            }
        }
        if self.dom.attr(idx, "data-type") == "NodeTable" {
            if let Some(colgroup) = self.dom.descendant_by_tag(idx, "colgroup") {
                let mut val = String::new();
                for (i, col) in self.dom.nodes[colgroup]
                    .children
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if i > 0 {
                        val.push('|');
                    }
                    val.push_str(&self.dom.attr(col, "style"));
                }
                node.set_ial_attr("colgroup", &val);
            }
        }
    }

    fn gen_action(&mut self, idx: usize) {
        if self.tip.typ() == NodeType::NodeCodeBlock {
            let language_node = self.dom.nodes[idx].children.first().copied();
            let mut language = String::new();
            if let Some(lang) = language_node {
                if let Some(first) = self.dom.nodes[lang].children.first().copied() {
                    language = self.dom.nodes[first].data.clone();
                }
            }
            let info = self.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
            info.data_mut().code_block_info = language.into_bytes();
            self.tip.append_child(info);
            let code_idx = self
                .dom
                .next_sibling(idx)
                .and_then(|n| self.dom.nodes[n].children.last().copied());
            let mut code = code_idx.map(|c| self.dom.text(c)).unwrap_or_default();
            let caret_tail = format!("\n\n{}", editor::CARET);
            if code.ends_with(&caret_tail) {
                code.truncate(code.len() - caret_tail.len());
                code.push('\n');
                code.push_str(editor::CARET);
                code.push('\n');
            }
            let lines: Vec<&str> = code.split('\n').collect();
            let mut buf = String::new();
            for (i, line) in lines.iter().enumerate() {
                let mut line = line.to_string();
                if line.contains("\x60\x60\x60") {
                    line = line.replace(&format!("{}\x60\x60\x60", editor::ZWJ), "\x60\x60\x60");
                    line = line.replace("\x60\x60\x60", &format!("{}\x60\x60\x60", editor::ZWJ));
                }
                buf.push_str(&line);
                if i < lines.len() - 1 {
                    buf.push('\n');
                }
            }
            let code_node = self.new_node_raw(NodeType::NodeCodeBlockCode);
            code_node.set_tokens(buf.into_bytes());
            self.tip.append_child(code_node);
        } else if (self.tip.typ() == NodeType::NodeListItem
            || self.tip.typ() == NodeType::NodeMindmapItem)
            && self
                .tip
                .data()
                .list_data
                .as_ref()
                .map(|d| d.typ)
                .unwrap_or(0)
                == 3
        {
            let parent = self.dom.nodes[idx].parent;
            let checked = parent
                .map(|p| self.dom.attr(p, "class").contains("protyle-task--done"))
                .unwrap_or(false);
            let marker = self.new_node_raw(NodeType::NodeTaskListItemMarker);
            let data_task = parent
                .map(|p| self.dom.attr(p, "data-task"))
                .unwrap_or_default();
            revive_task_marker(marker, &data_task, checked);
            self.tip.append_child(marker);
        }
    }

    fn gen_list_item(&mut self, idx: usize, data_type: &str, node_id: &str, has_id: bool) {
        let marker = self.dom.attr(idx, "data-marker");
        let container = if data_type == "NodeMindmapItem" {
            NodeType::NodeMindmap
        } else {
            NodeType::NodeList
        };
        if self.tip.typ() != container {
            let parent = self.new_node_raw(container);
            let subtype = self.dom.attr(idx, "data-subtype");
            let mut ld = crate::ast::ListData::default();
            match subtype.as_str() {
                "u" => {
                    ld.typ = 0;
                    ld.bullet_char = b'*';
                }
                "o" => {
                    ld.typ = 1;
                    ld.num = marker.trim_end_matches('.').parse().unwrap_or(0);
                    ld.delimiter = b'.';
                }
                "t" => {
                    ld.typ = 3;
                    ld.bullet_char = b'*';
                }
                _ => {}
            }
            parent.data_mut().list_data = Some(ld);
            self.tip.append_child(parent);
            self.tip = parent;
        }
        let typ = if data_type == "NodeMindmapItem" {
            NodeType::NodeMindmapItem
        } else {
            NodeType::NodeListItem
        };
        let node = self.new_node_raw(typ);
        node.data_mut().id = node_id.to_string();
        let subtype = self.dom.attr(idx, "data-subtype");
        let mut ld = crate::ast::ListData::default();
        match subtype.as_str() {
            "u" => {
                ld.typ = 0;
                ld.bullet_char = b'*';
            }
            "o" => {
                ld.typ = 1;
                ld.num = marker.trim_end_matches('.').parse().unwrap_or(0);
                ld.delimiter = b'.';
            }
            "t" => {
                ld.typ = 3;
                ld.bullet_char = b'*';
            }
            _ => {}
        }
        ld.marker = marker.into_bytes();
        node.data_mut().list_data = Some(ld);
        self.tip.append_child(node);
        self.tip = node;
        self.gen_children(idx);
        self.leave();
        self.after_block(idx, node, has_id);
    }

    fn gen_table(&mut self, idx: usize, node_id: &str, has_id: bool) {
        let fallback = |this: &mut Self| {
            let node = this.new_node_raw(NodeType::NodeParagraph);
            node.data_mut().id = node_id.to_string();
            this.tip.append_child(node);
            this.after_block(idx, node, has_id);
        };
        let Some(table_div) = self.dom.nodes[idx].children.first().copied() else {
            fallback(self);
            return;
        };
        let Some(table) = self.dom.descendant_by_tag(table_div, "table") else {
            fallback(self);
            return;
        };
        normalize_protyle_table_spans(self.dom, table);
        let Some(thead) = self.dom.child_by_tag(table, "thead") else {
            fallback(self);
            return;
        };
        let Some(head_row) = self.dom.nodes[thead].children.first().copied() else {
            fallback(self);
            return;
        };
        let mut aligns = Vec::new();
        for th in self.dom.nodes[head_row].children.clone() {
            aligns.push(table_align_value(
                &self.dom.attr(th, "align"),
                &self.dom.attr(th, "style"),
            ));
        }

        let node = self.new_node_raw(NodeType::NodeTable);
        node.data_mut().id = node_id.to_string();
        node.data_mut().table_aligns = aligns;
        let caption = self.dom.attr(idx, "caption");
        if !caption.is_empty() {
            node.set_ial_attr("caption", &caption);
            if let Some(caption_node) = self.dom.nodes[table]
                .children
                .first()
                .copied()
                .filter(|&child| self.dom.nodes[child].data == "caption")
            {
                self.dom.unlink(caption_node);
            }
        }
        self.tip.append_child(node);
        self.tip = node;
        self.gen_contenteditable(table);
        self.leave();
        self.after_block(idx, node, has_id);
    }

    fn gen_callout(&mut self, idx: usize, node_id: &str, has_id: bool) {
        let node = self.new_node_raw(NodeType::NodeCallout);
        node.data_mut().id = node_id.to_string();
        node.data_mut().callout_type = self.dom.attr(idx, "data-subtype");
        let icon_span = self
            .dom
            .children_by_tag(idx, "span")
            .into_iter()
            .find(|&c| self.dom.attr(c, "class") == "callout-icon");
        let icon = icon_span.and_then(|s| self.dom.nodes[s].children.first().copied());
        match icon {
            None => {
                node.data_mut().callout_icon =
                    crate::parse::callout::get_callout_icon(&node.data().callout_type).to_string();
            }
            Some(icon) => {
                if self.dom.nodes[icon].data == "img" {
                    node.data_mut().callout_icon = self.dom.attr(icon, "src");
                    node.data_mut().callout_icon_type = 1;
                } else {
                    node.data_mut().callout_icon = self.dom.text(icon);
                }
            }
        }
        let title_span = self
            .dom
            .children_by_tag(idx, "span")
            .into_iter()
            .find(|&c| self.dom.attr(c, "class") == "callout-title");
        if let Some(title_span) = title_span {
            let mut title_html = String::new();
            for c in self.dom.nodes[title_span].children.clone() {
                title_html.push_str(&self.dom.html(c));
            }
            let title = blockdom2md(&title_html, self.options.clone());
            node.data_mut().callout_title = title.trim().to_string();
            node.data_mut().callout_title_explicit = true;
        }
        self.tip.append_child(node);
        self.tip = node;
        self.gen_children(idx);
        self.leave();
        self.after_block(idx, node, has_id);
    }

    fn gen_contenteditable(&mut self, idx: usize) {
        if self.tip.typ() == NodeType::NodeCodeBlock || self.tip.typ() == NodeType::NodeCustomBlock
        {
            return;
        }
        let tag = self.dom.nodes[idx].data.clone();
        if tag == "colgroup" {
            return;
        }
        if self.dom.attr(idx, "class") == "svg" {
            return;
        }

        if self.dom.nodes[idx].typ != DomType::Element || !crate::dom::is_known_tag(&tag) {
            self.gen_ce_text(idx);
            self.gen_ce_children(idx);
            return;
        }

        // Go genASTContenteditable applies the same empty/caret and TextMark
        // contract to native inline tags as to span[data-type].
        if matches!(
            tag.as_str(),
            "code"
                | "sub"
                | "sup"
                | "u"
                | "kbd"
                | "em"
                | "i"
                | "strong"
                | "b"
                | "s"
                | "del"
                | "strike"
                | "mark"
        ) {
            if matches!(
                tag.as_str(),
                "em" | "i" | "strong" | "b" | "s" | "del" | "strike" | "mark"
            ) && self.dom.nodes[idx]
                .children
                .first()
                .is_none_or(|&first| self.dom.nodes[first].data == "br")
            {
                return;
            }
            if matches!(tag.as_str(), "strong" | "b") {
                if let Some(previous) = self.tip.last_child() {
                    let caret_tail = format!("\\{}", editor::CARET);
                    let mut tokens = previous.tokens_vec();
                    if tokens.ends_with(caret_tail.as_bytes()) {
                        tokens = replace_all(
                            &tokens,
                            caret_tail.as_bytes(),
                            format!("\\\\{}", editor::CARET).as_bytes(),
                        );
                    }
                    if tokens.ends_with(b"\\") {
                        tokens = replace_all(&tokens, b"\\", b"\\\\");
                    }
                    previous.set_tokens(tokens);
                }
            }
            if self.inline_placeholder(idx) {
                return;
            }
            if self.options.text_mark {
                self.append_text_mark(idx);
                return;
            }
        }

        match tag.as_str() {
            "thead" => {
                let grand = self.dom.nodes[idx]
                    .parent
                    .and_then(|p| self.dom.nodes[p].parent)
                    .unwrap_or(idx);
                if self.dom.parent_is(grand, &["table"]) {
                    let node = self.new_node_raw(NodeType::NodeText);
                    let mut text = self.dom.text(idx);
                    text = text.replace(editor::CARET, "");
                    node.set_tokens(text.trim().as_bytes().to_vec());
                    self.tip.append_child(node);
                    return;
                }
                let node = self.new_node_raw(NodeType::NodeTableHead);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
            }
            "tbody" => self.gen_ce_children(idx),
            "tr" => {
                let node = self.new_node_raw(NodeType::NodeTableRow);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
            }
            "th" | "td" => {
                let node = self.new_node_raw(NodeType::NodeTableCell);
                node.set_tokens(tag.as_bytes().to_vec());
                node.data_mut().table_cell_align =
                    table_align_value(&self.dom.attr(idx, "align"), &self.dom.attr(idx, "style"));
                self.tip.append_child(node);
                set_span_ial(self.tree, node, self.dom, idx);
                // 富文本单元格只读取载荷并投影，内部节点不作为可编辑文档块展开。
                if self
                    .dom
                    .has_attr(idx, crate::ast::TABLE_CELL_RICH_ATTRIBUTE)
                {
                    let encoded = self.dom.attr(idx, crate::ast::TABLE_CELL_RICH_ATTRIBUTE);
                    node.data_mut().table_cell_rich =
                        Some(crate::ast::decode_table_cell_rich(&encoded));
                    let _ = crate::parse::table_rich::apply_table_cell_rich_projection(node);
                    return;
                }
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
            }
            "code" => {
                let node = self.new_node_raw(NodeType::NodeCodeSpan);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeCodeSpanOpenMarker));
                node.append_child(self.new_node_raw(NodeType::NodeCodeSpanContent));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeCodeSpanCloseMarker));
            }
            "span" => self.gen_span(idx),
            "em" | "i" => {
                let node = self.new_node_raw(NodeType::NodeEmphasis);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeEmA6kOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeEmA6kCloseMarker));
            }
            "strong" | "b" => {
                let node = self.new_node_raw(NodeType::NodeStrong);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeStrongA6kOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeStrongA6kCloseMarker));
            }
            "u" => {
                let node = self.new_node_raw(NodeType::NodeUnderline);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeUnderlineOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeUnderlineCloseMarker));
            }
            "kbd" => {
                let node = self.new_node_raw(NodeType::NodeKbd);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeKbdOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeKbdCloseMarker));
            }
            "mark" => {
                let node = self.new_node_raw(NodeType::NodeMark);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeMark1OpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeMark1CloseMarker));
            }
            "sub" => {
                let node = self.new_node_raw(NodeType::NodeSub);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeSubOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeSubCloseMarker));
            }
            "sup" => {
                let node = self.new_node_raw(NodeType::NodeSup);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeSupOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_ce_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeSupCloseMarker));
            }
            "br" => {
                if self.tip.typ() == NodeType::NodeHeading {
                    return;
                }
                if self
                    .dom
                    .prev_sibling(idx)
                    .is_some_and(|previous| self.dom.nodes[previous].data == "\n")
                    && self.dom.parent_is(idx, &["table"])
                {
                    return;
                }
                if self.tip.typ() == NodeType::NodeParagraph {
                    let node = self.new_node_raw(NodeType::NodeText);
                    node.set_tokens(b"\n".to_vec());
                    self.tip.append_child(node);
                    return;
                }
                let node = self.new_node_raw(NodeType::NodeBr);
                self.tip.append_child(node);
            }
            "input" => {
                let node = self.new_node_raw(NodeType::NodeTaskListItemMarker);
                revive_task_marker(
                    node,
                    &self.dom.attr(idx, "data-task"),
                    self.dom.has_attr(idx, "checked"),
                );
                self.tip.append_child(node);
            }
            "img" => {
                if self.dom.attr(idx, "class") == "emoji" {
                    let node = self.new_node_raw(NodeType::NodeEmoji);
                    let image = self.new_node_raw(NodeType::NodeEmojiImg);
                    let alt = self.dom.attr(idx, "alt");
                    image.set_tokens(
                        format!(
                            "<img alt=\"{alt}\" class=\"emoji\" src=\"{}\" title=\"{alt}\" />",
                            self.dom.attr(idx, "src")
                        )
                        .into_bytes(),
                    );
                    let alias = self.new_node_raw(NodeType::NodeEmojiAlias);
                    alias.set_tokens(format!(":{alt}:").into_bytes());
                    image.append_child(alias);
                    node.append_child(image);
                    self.tip.append_child(node);
                } else {
                    // Ordinary images are represented by their outer img span.
                    self.gen_ce_children(idx);
                }
            }
            _ => self.gen_ce_children(idx),
        }
    }

    fn gen_ce_text(&mut self, idx: usize) {
        let mut content = self.dom.nodes[idx].data.clone();
        if content.is_empty() {
            return;
        }
        let node = self.new_node_raw(NodeType::NodeText);
        node.set_tokens(if self.dom.nodes[idx].typ == DomType::Element {
            format!("<{content}>").into_bytes()
        } else {
            content.clone().into_bytes()
        });
        if self.tip.typ() == NodeType::NodeLink {
            node.data_mut().typ = NodeType::NodeLinkText;
        } else if self.tip.typ() == NodeType::NodeHeading {
            content = content.replace('\n', "");
            node.set_tokens(content.clone().into_bytes());
        } else if self.tip.typ() == NodeType::NodeStrong {
            content = content.replace("**", "");
            content = content.replace(&format!("*{}", editor::CARET), editor::CARET);
            content = content.replace(&format!("{}*", editor::CARET), editor::CARET);
            node.set_tokens(content.clone().into_bytes());
        }
        if self.dom.parent_is(idx, &["table"]) {
            content = content.strip_suffix('\n').unwrap_or(&content).to_string();
            let next = self.dom.next_sibling(idx);
            if (next.is_none() && !content.contains('\n'))
                || next.is_some_and(|next| {
                    self.dom.nodes[next].data == "br" && content.starts_with('\n')
                })
            {
                content = content.replace('\n', "");
            }
            if content.contains('\\') && content.replace('\\', "").trim().is_empty() {
                content = content.replace('\\', "\\\\");
            }
            node.set_tokens(content.replace('\n', "<br />").into_bytes());
            let tokens = node.tokens_vec();
            let parts = split_without_backslash_escape(&tokens, b'|');
            let mut joined = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                joined.extend_from_slice(part);
                if i < parts.len() - 1 {
                    joined.extend_from_slice(b"\\|");
                }
            }
            node.set_tokens(joined);
        }
        if self.tip.typ() == NodeType::NodeCodeSpan || self.tip.typ() == NodeType::NodeInlineMath {
            if let Some(first) = self.tip.first_child() {
                if let Some(content_node) = first.next() {
                    if self
                        .tip
                        .previous()
                        .is_some_and(|prev| prev.typ() == self.tip.typ())
                    {
                        content_node.set_tokens(content.as_bytes().to_vec());
                    } else {
                        if let Some(backslash) = content_node
                            .next()
                            .filter(|node| node.typ() == NodeType::NodeBackslash)
                        {
                            if let Some(text) = backslash.first_child() {
                                content = format!("{}{}", text.tokens_str(), content);
                            }
                            backslash.unlink();
                        }
                        let mut tokens = content_node.tokens_vec();
                        tokens.extend_from_slice(content.as_bytes());
                        content_node.set_tokens(tokens);
                    }
                }
            }
            return;
        }
        if self.tip.typ() == NodeType::NodeKbd {
            let mut tokens = node.tokens_vec();
            tokens = replace_all(&tokens, b"\\\\", b"\\");
            tokens = replace_all(&tokens, b"\\", b"\\\\");
            node.set_tokens(tokens);
            if node.tokens_vec() == editor::CARET.as_bytes() {
                let parent = self.tip.parent();
                self.tip.unlink();
                if let Some(parent) = parent {
                    self.tip = parent;
                }
            }
        }
        self.tip.append_child(node);
    }

    fn gen_ce_children(&mut self, idx: usize) {
        let children = self.dom.nodes[idx].children.clone();
        for c in children {
            self.gen_contenteditable(c);
        }
    }

    /// 对应 Go 版 Lute.isCaret：判断节点是否只包含插入符/占位符。
    fn is_caret(&self, idx: usize) -> (bool, bool) {
        let text = self.dom.text(idx);
        let trimmed = text.trim();
        if trimmed.is_empty() && text.len() > 1 && text.contains(editor::CARET) {
            return (true, false);
        }
        let is_caret = editor::CARET == text
            || format!("{}{}", editor::ZWSP, editor::CARET) == text
            || format!("{}{}", editor::CARET, editor::ZWSP) == text;
        let mut is_empty = text.is_empty() || editor::ZWSP == text;
        if !self.dom.attr(idx, "data-content").is_empty() {
            is_empty = false;
        }
        (is_caret, is_empty)
    }

    fn gen_span(&mut self, idx: usize) {
        let mut data_type = self.dom.attr(idx, "data-type");
        if data_type.is_empty() {
            data_type = "text".to_string();
        }
        if data_type.contains("span") {
            let node = self.new_node_raw(NodeType::NodeText);
            node.set_tokens(self.dom.text(idx).into_bytes());
            self.tip.append_child(node);
            return;
        }
        if data_type.contains("img") {
            data_type = "img".to_string();
        }
        // Protect a preceding escape before a span is serialized to Markdown.
        if let Some(previous) = self.tip.last_child() {
            let text = previous.text();
            let caret = text.ends_with(&format!("\\{}", editor::CARET));
            let end = text
                .strip_suffix(editor::CARET)
                .filter(|_| caret)
                .unwrap_or(&text);
            if end.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 1 {
                let mut tokens = previous.tokens_vec();
                if caret {
                    tokens.truncate(tokens.len().saturating_sub(editor::CARET.len()));
                }
                tokens.push(b'\\');
                if caret {
                    tokens.extend_from_slice(editor::CARET.as_bytes());
                }
                previous.set_tokens(tokens);
            }
        }
        if matches!(data_type.as_str(), "block-ref" | "file-annotation-ref") {
            let text = self.dom.text(idx);
            let text = text.trim();
            if text.is_empty() {
                return;
            }
            if text == editor::CARET {
                let node = self.new_node_raw(NodeType::NodeText);
                node.set_tokens(editor::CARET.as_bytes().to_vec());
                self.tip.append_child(node);
                return;
            }
            if self.options.text_mark {
                self.append_text_mark(idx);
                return;
            }
            let block = data_type == "block-ref";
            let node = self.new_node_raw(if block {
                NodeType::NodeBlockRef
            } else {
                NodeType::NodeFileAnnotationRef
            });
            let open = if block {
                NodeType::NodeOpenParen
            } else {
                NodeType::NodeLess
            };
            let close = if block {
                NodeType::NodeCloseParen
            } else {
                NodeType::NodeGreater
            };
            node.append_child(self.new_node_raw(open));
            node.append_child(self.new_node_raw(open));
            let id = self.new_node_raw(if block {
                NodeType::NodeBlockRefID
            } else {
                NodeType::NodeFileAnnotationRefID
            });
            id.set_tokens(self.dom.attr(idx, "data-id").into_bytes());
            node.append_child(id);
            node.append_child(self.new_node_raw(if block {
                NodeType::NodeBlockRefSpace
            } else {
                NodeType::NodeFileAnnotationRefSpace
            }));
            let subtype = self.dom.attr(idx, "data-subtype");
            let label = self.new_node_raw(if !block {
                NodeType::NodeFileAnnotationRefText
            } else if subtype.is_empty() || subtype == "s" {
                NodeType::NodeBlockRefText
            } else {
                NodeType::NodeBlockRefDynamicText
            });
            label.set_tokens(if block && self.dom.parent_is(idx, &["table"]) {
                text.replace('|', "&#124;").into_bytes()
            } else {
                text.as_bytes().to_vec()
            });
            node.append_child(label);
            node.append_child(self.new_node_raw(close));
            node.append_child(self.new_node_raw(close));
            self.tip.append_child(node);
            return;
        }
        if matches!(data_type.as_str(), "tag" | "inline-memo" | "a") {
            if data_type == "a" && self.tip.typ() == NodeType::NodeLink {
                self.gen_ce_children(idx);
                return;
            }
            if self.inline_placeholder(idx) {
                return;
            }
            if self.options.text_mark {
                self.append_text_mark(idx);
                return;
            }
        }
        if data_type == "inline-math" && self.options.text_mark {
            if !self.dom.attr(idx, "data-content").is_empty() {
                self.append_text_mark(idx);
            }
            return;
        }
        match data_type.as_str() {
            "tag" => {
                let node = self.new_node_raw(NodeType::NodeTag);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeTagOpenMarker));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeTagCloseMarker));
            }
            "inline-math" => {
                let content = text_mark_inline_math_data(self.dom, idx);
                if content.is_empty() {
                    return;
                }
                let node = self.new_node_raw(NodeType::NodeInlineMath);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeInlineMathOpenMarker));
                let content_node = self.new_node_raw(NodeType::NodeInlineMathContent);
                content_node.set_tokens(content.into_bytes());
                node.append_child(content_node);
                node.append_child(self.new_node_raw(NodeType::NodeInlineMathCloseMarker));
                self.tip.append_child(node);
            }
            "inline-memo" => {
                let node = self.new_node_raw(NodeType::NodeText);
                node.set_tokens(self.dom.text(idx).into_bytes());
                self.tip.append_child(node);
            }
            "a" => {
                if self.dom.nodes[idx].children.is_empty() {
                    return;
                }
                if self.tip.typ() == NodeType::NodeLink {
                    return;
                }
                let node = self.new_node_raw(NodeType::NodeLink);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                node.append_child(self.new_node_raw(NodeType::NodeOpenBracket));
                self.tip.append_child(node);
                self.tip = node;
                self.gen_children(idx);
                self.leave();
                node.append_child(self.new_node_raw(NodeType::NodeCloseBracket));
                node.append_child(self.new_node_raw(NodeType::NodeOpenParen));
                let dest = self.new_node_raw(NodeType::NodeLinkDest);
                dest.set_tokens(
                    crate::vditor::trim_link_path(
                        &self.dom.attr(idx, "data-href"),
                        &self.render_options,
                    )
                    .as_bytes()
                    .to_vec(),
                );
                node.append_child(dest);
                let title = self.dom.attr(idx, "data-title");
                if !title.is_empty() {
                    node.append_child(self.new_node_raw(NodeType::NodeLinkSpace));
                    let title_node = self.new_node_raw(NodeType::NodeLinkTitle);
                    let title = title
                        .replace('"', "&quot;")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">");
                    title_node.set_tokens(title.into_bytes());
                    node.append_child(title_node);
                }
                node.append_child(self.new_node_raw(NodeType::NodeCloseParen));
            }
            "img" => self.gen_image(idx),
            "backslash" => {
                let node = self.new_node_raw(NodeType::NodeBackslash);
                node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
                let children = self.dom.nodes[idx].children.clone();
                let Some(&first) = children.first() else {
                    return;
                };
                if children.len() == 1 && !self.dom.nodes[first].children.is_empty() {
                    return;
                }
                if children.len() == 1 && self.dom.nodes[first].typ == DomType::Text {
                    let text = self.new_node_raw(NodeType::NodeText);
                    text.set_tokens(self.dom.nodes[first].data.clone().into_bytes());
                    node.append_child(text);
                    self.tip.append_child(node);
                    return;
                }
                if let Some(&second) = children.get(1) {
                    let content = self.new_node_raw(NodeType::NodeBackslashContent);
                    content.set_tokens(
                        self.dom.nodes[second]
                            .data
                            .replace("\\\\", "\\")
                            .into_bytes(),
                    );
                    node.append_child(content);
                }
                self.tip.append_child(node);
            }
            "text" | "" => {
                let (is_caret, is_empty) = self.is_caret(idx);
                if is_caret {
                    let node = self.new_node_raw(NodeType::NodeText);
                    node.set_tokens(editor::CARET.as_bytes().to_vec());
                    self.tip.append_child(node);
                    return;
                }
                if is_empty {
                    return;
                }
                self.append_text_mark(idx);
            }
            _ => {
                let (is_caret, is_empty) = self.is_caret(idx);
                if is_caret {
                    let text = self.new_node_raw(NodeType::NodeText);
                    text.set_tokens(editor::CARET.as_bytes().to_vec());
                    self.tip.append_child(text);
                    return;
                }
                if is_empty {
                    return;
                }
                data_type = data_type
                    .replace("search-mark", "")
                    .replace("virtual-block-ref", "")
                    .trim()
                    .to_string();
                if data_type.replace("backslash", "").trim().is_empty() {
                    let node = self.new_node_raw(NodeType::NodeText);
                    node.set_tokens(self.dom.text(idx).into_bytes());
                    self.tip.append_child(node);
                } else {
                    self.dom.set_attr(idx, "data-type", &data_type);
                    self.append_text_mark(idx);
                }
            }
        }
    }

    fn append_text_mark(&mut self, idx: usize) {
        let node = self.new_node_raw(NodeType::NodeTextMark);
        self.tip.append_child(node);
        set_text_mark_node(self.tree, node, self.dom, idx, &self.options);
    }

    /// Return true when Go consumes an empty inline or reduces it to its caret.
    fn inline_placeholder(&mut self, idx: usize) -> bool {
        let (caret, empty) = self.is_caret(idx);
        if caret {
            let node = self.new_node_raw(NodeType::NodeText);
            node.set_tokens(editor::CARET.as_bytes().to_vec());
            self.tip.append_child(node);
        }
        caret || empty
    }

    fn gen_image(&mut self, idx: usize) {
        // 对应 Go 版 lute.domChild：图片元素可能被多层 span 包裹，需深度优先查找。
        let img = self.dom.descendant_by_tag(idx, "img").or_else(|| {
            if self.dom.nodes[idx].data == "img" {
                Some(idx)
            } else {
                None
            }
        });
        let Some(img) = img else {
            return;
        };
        let node = self.new_node_raw(NodeType::NodeImage);
        node.set_tokens(self.dom.nodes[idx].data.as_bytes().to_vec());
        node.append_child(self.new_node_raw(NodeType::NodeBang));
        node.append_child(self.new_node_raw(NodeType::NodeOpenBracket));
        let mut alt = self.dom.attr(img, "alt");
        alt = alt.replace(editor::CARET, "");
        let alt_node = self.new_node_raw(NodeType::NodeLinkText);
        alt_node.set_tokens(alt.into_bytes());
        node.append_child(alt_node);
        node.append_child(self.new_node_raw(NodeType::NodeCloseBracket));
        node.append_child(self.new_node_raw(NodeType::NodeOpenParen));
        let mut src = self.dom.attr(img, "data-src");
        if src.is_empty() {
            src = self.dom.attr(img, "src");
        }
        let dest = self.new_node_raw(NodeType::NodeLinkDest);
        dest.set_tokens(src.trim().as_bytes().to_vec());
        node.append_child(dest);
        let mut title = self.dom.nodes[img]
            .attrs
            .iter()
            .find(|(key, _)| key == "data-title")
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| self.dom.attr(img, "title"));
        if !title.is_empty() {
            node.append_child(self.new_node_raw(NodeType::NodeLinkSpace));
            title = title
                .replace('"', "&quot;")
                .replace("&lt;", "<")
                .replace("&gt;", ">");
            let title_node = self.new_node_raw(NodeType::NodeLinkTitle);
            title_node.set_tokens(title.into_bytes());
            node.append_child(title_node);
        }
        node.append_child(self.new_node_raw(NodeType::NodeCloseParen));
        self.tip.append_child(node);
        set_span_ial(self.tree, node, self.dom, img);
    }
}

/// 对应 Go 版 Lute.blockDOM2Md。
pub fn blockdom2md(html: &str, options: ParseOptions) -> String {
    blockdom2md_with_render_options(html, options, &crate::options::RenderOptions::new())
}

pub(crate) fn blockdom2md_with_render_options(
    html: &str,
    options: ParseOptions,
    render_options: &crate::options::RenderOptions,
) -> String {
    let mut tree = blockdom2tree_with_render_options(html, options.clone(), render_options);
    tree2md_with_render_options(&mut tree, options, render_options)
}

/// 对应 Go 版 Lute.blockDOMTree2Md。
pub fn tree2md(tree: &mut Tree, options: ParseOptions) -> String {
    tree2md_with_render_options(tree, options, &crate::options::RenderOptions::new())
}

pub(crate) fn tree2md_with_render_options(
    tree: &mut Tree,
    options: ParseOptions,
    configured_render_options: &crate::options::RenderOptions,
) -> String {
    wrap_tab_item_fragments(tree);
    let mut render_options = crate::options::RenderOptions::new();
    render_options.auto_space = false;
    render_options.fix_term_typo = false;
    render_options.kramdown_block_ial = true;
    render_options.kramdown_span_ial = true;
    render_options.keep_paragraph_beginning_space = true;
    render_options.protyle_wysiwyg = true;
    render_options.super_block = true;
    render_options.export_normalize_task_list_marker = false;
    render_options.unordered_list_marker = configured_render_options.unordered_list_marker.clone();
    // 对应 Go 版 blockDOMTree2Md（SpinBlockDOM 路径）：使用基础 FormatRenderer，
    // 由它写出 table-cell-rich IAL，使富文本源能穿过 Markdown 往返。
    let mut r = crate::render::format::FormatRenderer::new(tree, render_options, options);
    String::from_utf8_lossy(&r.render()).to_string()
}

fn ial2_tokens(ial: &[(String, String)]) -> Vec<u8> {
    let mut buf = String::from("{: ");
    for (i, (k, v)) in ial.iter().enumerate() {
        buf.push_str(k);
        buf.push('=');
        buf.push('"');
        buf.push_str(v);
        buf.push('"');
        if i < ial.len() - 1 {
            buf.push(' ');
        }
    }
    buf.push('}');
    buf.into_bytes()
}

/// Go normalizeProtyleTableSpans: occupancy resets at each HTML row group.
fn normalize_protyle_table_spans(dom: &mut Dom, table: usize) {
    for section in dom.nodes[table].children.clone() {
        let head = dom.nodes[section].data == "thead";
        if !matches!(
            dom.nodes[section].data.as_str(),
            "thead" | "tbody" | "tfoot"
        ) {
            continue;
        }
        let rows: Vec<_> = dom.nodes[section]
            .children
            .iter()
            .copied()
            .filter(|&row| dom.nodes[row].data == "tr")
            .collect();
        let mut occupied = std::collections::HashMap::<usize, usize>::new();
        for (row_index, &row) in rows.iter().enumerate() {
            let mut column = 0;
            for cell in dom.nodes[row].children.clone() {
                if !matches!(dom.nodes[cell].data.as_str(), "td" | "th") {
                    continue;
                }
                if !dom
                    .attr(cell, "class")
                    .split_whitespace()
                    .any(|class| class == "fn__none")
                {
                    while occupied.get(&column).copied().unwrap_or(0) > row_index {
                        let placeholder = new_table_placeholder(dom, head);
                        dom.insert_before(cell, placeholder);
                        column += 1;
                    }
                    let colspan = dom
                        .attr(cell, "colspan")
                        .parse::<usize>()
                        .unwrap_or(1)
                        .clamp(1, 1000);
                    let rowspan = dom
                        .attr(cell, "rowspan")
                        .parse::<usize>()
                        .unwrap_or(1)
                        .clamp(1, rows.len() - row_index);
                    for offset in 0..colspan {
                        occupied.insert(column + offset, row_index + rowspan);
                    }
                }
                column += 1;
            }
            while occupied.get(&column).copied().unwrap_or(0) > row_index {
                let placeholder = new_table_placeholder(dom, head);
                dom.append_child(row, placeholder);
                column += 1;
            }
        }
    }
}

fn new_table_placeholder(dom: &mut Dom, head: bool) -> usize {
    let cell = dom.create(DomType::Element, if head { "th" } else { "td" });
    dom.set_attr(cell, "class", "fn__none");
    cell
}

fn table_align_value(align: &str, style: &str) -> i32 {
    crate::html2md::table_cell_align_value(align, style)
}

fn style_value(style: &str) -> String {
    style
        .trim()
        .replace('\n', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_without_backslash_escape(tokens: &[u8], sep: u8) -> Vec<Vec<u8>> {
    crate::lex::split_without_backslash_escape(tokens, sep)
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect()
}

fn replace_all(buf: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return buf.to_vec();
    }
    let mut ret = Vec::with_capacity(buf.len());
    let mut i = 0;
    while i < buf.len() {
        if buf[i..].starts_with(from) {
            ret.extend_from_slice(to);
            i += from.len();
        } else {
            ret.push(buf[i]);
            i += 1;
        }
    }
    ret
}

/// Go parse.SetSpanIAL: metadata and its syntax node must be kept together.
fn set_span_ial(tree: &mut Tree, node: Node, dom: &Dom, idx: usize) {
    let mut attrs = Vec::new();
    let style = dom.attr(idx, "style");
    if !style.is_empty() {
        attrs.push(("style".to_string(), style_value(&style)));
    }
    for (key, value) in &dom.nodes[idx].attrs {
        if key.starts_with("custom-") {
            attrs.push((key.clone(), html_util::unescape_html_str(value)));
        }
    }
    if matches!(dom.nodes[idx].data.as_str(), "td" | "th") {
        for key in ["colspan", "rowspan", "class"] {
            let value = dom.attr(idx, key);
            if !value.is_empty() {
                attrs.push((key.to_string(), value));
            }
        }
    }
    for (key, value) in attrs {
        node.set_ial_attr(&key, &value);
    }
    if dom.nodes[idx].data == "img" {
        if let Some(parent) = dom.nodes[idx].parent {
            let style = dom.attr(parent, "style");
            if !style.is_empty() {
                let value = format!("{}{}", node.ial_attr("style"), style);
                node.set_ial_attr("style", &value);
            }
            if let Some(grandparent) = dom.nodes[parent].parent {
                let style = dom.attr(grandparent, "style");
                if !style.is_empty() {
                    node.set_ial_attr("parent-style", &style);
                }
            }
        }
    }
    if node.data().kramdown_ial.is_empty() {
        return;
    }
    let ial = tree.new_node_raw(NodeType::NodeKramdownSpanIAL);
    ial.set_tokens(ial2_tokens(&node.data().kramdown_ial));
    if node.typ() == NodeType::NodeTableCell {
        node.prepend_child(ial);
    } else {
        node.insert_after(ial);
    }
}

/// Shared Go parse.SetTextMarkNode metadata population for DOM and inline HTML.
pub(crate) fn set_text_mark_node(
    tree: &mut Tree,
    node: Node,
    dom: &Dom,
    idx: usize,
    options: &ParseOptions,
) {
    node.set_typ(NodeType::NodeTextMark);
    let mut data_type = dom.attr(idx, "data-type");
    if data_type.is_empty() {
        data_type = if dom.nodes[idx].data == "span" {
            "text".to_string()
        } else if !node.data().text_mark_type.is_empty() {
            node.data().text_mark_type.clone()
        } else {
            match dom.nodes[idx].data.as_str() {
                "b" => "strong",
                "i" => "em",
                "del" | "strike" => "s",
                tag => tag,
            }
            .to_string()
        };
    }
    node.data_mut().text_mark_type = data_type.clone();
    node.clear_tokens();
    node.data_mut().text_mark_text_content.clear();
    node.data_mut().text_mark_flashcard_occlusion_id = dom.attr(idx, "data-occlusion-id");
    let mut types: Vec<&str> = data_type.split(' ').collect();
    if let Some(index) = types.iter().position(|typ| {
        matches!(
            *typ,
            "a" | "inline-memo" | "block-ref" | "file-annotation-ref" | "inline-math"
        )
    }) {
        let first = types.remove(index);
        types.insert(0, first);
    }
    let mut math = false;
    for typ in types {
        match typ {
            "inline-math" => {
                node.data_mut().text_mark_inline_math_content =
                    text_mark_inline_math_data(dom, idx);
                math = true;
            }
            "a" | "inline-memo" | "block-ref" | "file-annotation-ref" => {
                match typ {
                    "a" => {
                        node.data_mut().text_mark_a_href =
                            html_util::escape_html_str(&dom.attr(idx, "data-href"));
                        node.data_mut().text_mark_a_title =
                            html_util::escape_html_str(&dom.attr(idx, "data-title"))
                                .replace("&amp;lt;", "<")
                                .replace("&amp;gt;", ">");
                    }
                    "block-ref" => {
                        node.data_mut().text_mark_block_ref_id = dom.attr(idx, "data-id");
                        let subtype = dom.attr(idx, "data-subtype");
                        node.data_mut().text_mark_block_ref_subtype = if subtype.is_empty() {
                            "s".to_string()
                        } else {
                            subtype
                        };
                    }
                    "file-annotation-ref" => {
                        node.data_mut().text_mark_file_annotation_ref_id = dom.attr(idx, "data-id")
                    }
                    "inline-memo" => {
                        node.data_mut().text_mark_inline_memo_content =
                            html_util::unescape_html_str(
                                &dom.attr(idx, "data-inline-memo-content")
                                    .replace(editor::ZWSP, "")
                                    .replace('\n', editor::IAL_VAL_ESC_NEWLINE),
                            )
                            .replace('"', "&quot;");
                    }
                    _ => unreachable!(),
                }
                let content = text_mark_text_data(&dom.text(idx));
                node.data_mut().text_mark_text_content = content
                    .strip_prefix('\n')
                    .unwrap_or(&content)
                    .replace('\'', "&apos;");
            }
            _ if math => {}
            _ => {
                let mut content = text_mark_text_data(&dom.text(idx));
                if ["strong", "em", "s", "mark", "sup", "sub"]
                    .iter()
                    .any(|typ| node.is_text_mark_type(typ))
                {
                    // Go startEndBlank deliberately recognizes only these four
                    // boundary characters; TrimSpace below has wider semantics.
                    let boundary = |c| matches!(c, ' ' | '\t' | '\n' | '　');
                    let start =
                        &content[..content.len() - content.trim_start_matches(boundary).len()];
                    let end = if start == content {
                        ""
                    } else {
                        &content[content.trim_end_matches(boundary).len()..]
                    };
                    if !start.is_empty()
                        && !node
                            .previous()
                            .is_some_and(|n| n.text().ends_with([' ', '　']))
                    {
                        let text = tree.new_node_raw(NodeType::NodeText);
                        text.set_tokens(start.as_bytes().to_vec());
                        node.insert_before(text);
                    }
                    if !end.is_empty()
                        && !node
                            .next()
                            .is_some_and(|n| n.text().starts_with([' ', '　']))
                    {
                        let text = tree.new_node_raw(NodeType::NodeText);
                        text.set_tokens(end.as_bytes().to_vec());
                        node.insert_after(text);
                    }
                    content = content.trim().to_string();
                    if !options.keep_escaped && !node.is_text_mark_type("code") {
                        content = html_util::unescape_html_str(&content);
                    }
                }
                if node.parent_is(&[NodeType::NodeTableCell]) && node.is_text_mark_type("code") {
                    content = content.replace('|', "&#124;");
                }
                node.data_mut().text_mark_text_content = content;
                if data_type == "u" {
                    let mut source = node.data().text_mark_text_content.clone();
                    if let Some(&first) = dom.nodes[idx].children.first() {
                        if dom.attr(first, "data-type") == "a" {
                            source = format!("[{source}]({})", dom.attr(first, "data-href"));
                        }
                    }
                    let inline = crate::parse::inline("", source.as_bytes(), options.clone());
                    if let Some(paragraph) = inline
                        .root_raw()
                        .first_child()
                        .filter(|p| p.first_child().is_some())
                    {
                        node.data_mut().text_mark_text_content =
                            html_util::escape_html_str(&paragraph.content())
                                .replace("&quot;", "\"");
                        if let Some(link) = paragraph
                            .first_child()
                            .filter(|n| n.next().is_none() && n.typ() == NodeType::NodeLink)
                        {
                            node.data_mut().text_mark_type.push_str(" a");
                            if let Some(dest) = link.child_by_type(NodeType::NodeLinkDest) {
                                node.data_mut().text_mark_a_href = dest.tokens_str();
                            }
                        }
                    }
                }
            }
        }
    }
    set_span_ial(tree, node, dom, idx);
}

/// Go util.GetTextMarkTextDataWithoutEscapeQuote; DOM text is decoded once by the HTML parser.
fn text_mark_text_data(text: &str) -> String {
    let text = text.replace(editor::ZWSP, "");
    let text = text.strip_suffix('\n').unwrap_or(&text);
    let mut content = html_util::escape_html_str(text).replace("&quot;", "\"");
    while content.contains("\n\n") {
        content = content.replace("\n\n", "\n");
    }
    content
}

fn text_mark_inline_math_data(dom: &Dom, idx: usize) -> String {
    let content = dom
        .attr(idx, "data-content")
        .replace('\n', editor::IAL_VAL_ESC_NEWLINE);
    html_util::unescape_html_str(&content).replace(editor::ZWSP, "")
}

/// Go wrapTabItemFragments: allocate temporary syntax containers in the same tree arena.
pub(crate) fn wrap_tab_item_fragments(tree: &mut Tree) -> std::collections::HashSet<String> {
    let mut wrappers = std::collections::HashSet::new();
    let mut current = tree.root_raw().first_child();
    while let Some(node) = current {
        current = node.next();
        if node.typ() != NodeType::NodeTabItem {
            continue;
        }
        if node.data().id.is_empty() {
            let id = crate::ast::new_node_id();
            node.data_mut().id = id.clone();
            node.set_ial_attr("id", &id);
        }
        let wrapper = tree.new_node_raw(NodeType::NodeTabs);
        let id = crate::ast::new_node_id();
        wrapper.data_mut().id = id.clone();
        wrapper.set_ial_attr("id", &id);
        node.insert_before(wrapper);
        wrapper.append_child(node);
        if let Some(ial) = current.filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL) {
            current = ial.next();
            wrapper.append_child(ial);
        } else {
            let ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(ial2_tokens(&node.data().kramdown_ial));
            wrapper.append_child(ial);
        }
        let ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        ial.set_tokens(ial2_tokens(&wrapper.data().kramdown_ial));
        wrapper.insert_after(ial);
        wrappers.insert(id);
    }
    wrappers
}

pub(crate) fn unwrap_tab_item_fragments(tree: &Tree, wrappers: &std::collections::HashSet<String>) {
    let mut current = tree.root_raw().first_child();
    while let Some(node) = current {
        current = node.next();
        if node.typ() != NodeType::NodeTabs || !wrappers.contains(&node.data().id) {
            continue;
        }
        while let Some(child) = node.first_child() {
            node.insert_before(child);
        }
        if let Some(ial) = current.filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL) {
            current = ial.next();
            ial.unlink();
        }
        node.unlink();
    }
}

/// Go normalizeSpinCaretNewline: recover the caret's paragraph position before Markdown parsing.
pub(crate) fn normalize_spin_caret_newline(tree: &Tree) {
    crate::ast::walk_raw(tree.root_raw(), &mut |paragraph, entering| {
        if !entering || paragraph.typ() != NodeType::NodeParagraph {
            return crate::ast::WalkStatus::WalkContinue;
        }
        let mut child = paragraph.first_child();
        while let Some(node) = child {
            child = node.next();
            if node.typ() != NodeType::NodeText {
                continue;
            }
            let mut content = node.tokens_str();
            let caret_newline = format!("{}\n", editor::CARET);
            if node.previous().is_none()
                && content.starts_with(&caret_newline)
                && spin_has_content_after(node, &content[caret_newline.len()..])
                && !spin_starts_tabs_fence(&content[caret_newline.len()..])
            {
                content = format!("{}{}", editor::CARET, &content[caret_newline.len()..]);
            }
            let pattern = format!("\n{}\n", editor::CARET);
            let mut offset = 0;
            while let Some(index) = content[offset..].find(&pattern) {
                let index = offset + index;
                let after = index + pattern.len();
                if spin_has_content_after(node, &content[after..])
                    && !spin_starts_tabs_fence(&content[after..])
                {
                    let replacement = format!("\n\n{}", editor::CARET);
                    content.replace_range(index..after, &replacement);
                    offset = index + replacement.len();
                } else {
                    offset = after;
                }
            }
            node.set_tokens(content.into_bytes());
        }
        crate::ast::WalkStatus::WalkSkipChildren
    });
}

fn spin_starts_tabs_fence(content: &str) -> bool {
    let content = content.split('\n').next().unwrap_or_default().trim();
    for marker in ["@tab", "@tab:active"] {
        if content == marker
            || content
                .strip_prefix(marker)
                .is_some_and(|suffix| suffix.starts_with([' ', '\t']))
        {
            return true;
        }
    }
    let count = content.bytes().take_while(|&byte| byte == b':').count();
    count >= 3
        && (count == content.len()
            || content[count..].starts_with([' ', '\t']) && content[count..].trim() == "tabs")
}

fn spin_has_content_after(node: Node, content: &str) -> bool {
    fn has_content(content: &str) -> bool {
        !content
            .replace(editor::CARET, "")
            .replace(editor::ZWSP, "")
            .trim()
            .is_empty()
    }
    if has_content(content) {
        return true;
    }
    let mut next = node.next();
    while let Some(node) = next {
        if has_content(&node.text()) {
            return true;
        }
        next = node.next();
    }
    false
}

/// Go ast.Node.ReviveFromDataTask preserves arbitrary single-byte task states.
fn revive_task_marker(node: Node, data_task: &str, checked: bool) {
    let marker = if data_task.len() == 1 {
        data_task.as_bytes()[0]
    } else if checked {
        b'X'
    } else {
        b' '
    };
    node.revive_from_marker(marker);
}

#[cfg(test)]
mod inline_dom_contract_tests {
    use super::*;

    fn paragraph(html: &str, text_mark: bool) -> (Tree, Node) {
        let options = ParseOptions {
            text_mark,
            ..ParseOptions::new()
        };
        let tree = blockdom2tree(
            &format!(
                "<div data-type=\"NodeParagraph\"><div contenteditable=\"true\">{html}</div></div>"
            ),
            options,
        );
        let paragraph = tree.root_raw().first_child().unwrap();
        (tree, paragraph)
    }

    #[test]
    fn native_tag_and_span_preserve_text_mark_metadata() {
        for tag in [
            "code", "kbd", "strong", "em", "sub", "sup", "b", "i", "del", "strike",
        ] {
            let (_tree, p) = paragraph(
                &format!(
                    "<{tag} style=\" color: red; background: blue; \" custom-x=\"v\">value</{tag}>"
                ),
                true,
            );
            let mark = p.first_child().unwrap();
            assert_eq!(mark.typ(), NodeType::NodeTextMark, "{tag}");
            assert_eq!(mark.data().text_mark_text_content, "value", "{tag}");
            assert_eq!(mark.ial_attr("style"), "color: red; background: blue;");
            assert_eq!(mark.ial_attr("custom-x"), "v");
            assert_eq!(mark.next().unwrap().typ(), NodeType::NodeKramdownSpanIAL);
        }
        let (_tree, p) = paragraph("<span style=\"color:red\">value</span>", false);
        assert_eq!(p.first_child().unwrap().typ(), NodeType::NodeTextMark);
    }

    #[test]
    fn empty_and_caret_only_inlines_do_not_leave_marks() {
        for tag in ["code", "kbd", "strong", "em", "sub", "sup", "u"] {
            let (_tree, p) = paragraph(&format!("<{tag}></{tag}><{tag}><wbr></{tag}>"), true);
            let text = p.first_child().unwrap();
            assert_eq!(text.typ(), NodeType::NodeText, "{tag}");
            assert_eq!(text.tokens_str(), editor::CARET);
            assert!(text.next().is_none());
        }
        let (_tree, p) = paragraph("<span data-type=\"a\" data-href=\"url\"><wbr></span><span data-type=\"block-ref\" data-id=\"id\"> </span>", true);
        assert_eq!(p.first_child().unwrap().tokens_str(), editor::CARET);
        assert!(p.first_child().unwrap().next().is_none());
    }

    #[test]
    fn reference_nodes_respect_text_mark_option_and_default_subtype() {
        for text_mark in [true, false] {
            let (_tree, p) = paragraph(
                "<span data-type=\"block-ref\" data-id=\"ref\">label</span>",
                text_mark,
            );
            let reference = p.first_child().unwrap();
            if text_mark {
                assert_eq!(reference.typ(), NodeType::NodeTextMark);
                assert_eq!(reference.data().text_mark_block_ref_subtype, "s");
                assert_eq!(reference.data().text_mark_block_ref_id, "ref");
            } else {
                assert_eq!(reference.typ(), NodeType::NodeBlockRef);
                assert_eq!(
                    reference
                        .child_by_type(NodeType::NodeBlockRefID)
                        .unwrap()
                        .tokens_str(),
                    "ref"
                );
                assert_eq!(
                    reference
                        .child_by_type(NodeType::NodeBlockRefText)
                        .unwrap()
                        .tokens_str(),
                    "label"
                );
            }
        }
    }

    #[test]
    fn recursive_children_by_type_includes_self_in_document_order() {
        let (mut tree, p) = paragraph("<strong>outer</strong>", true);
        let nested = tree.new_node_raw(NodeType::NodeParagraph);
        p.append_child(nested);
        assert_eq!(p.children_by_type(NodeType::NodeParagraph), vec![p, nested]);
    }

    #[test]
    fn math_type_suppresses_style_text_and_boundary_movement() {
        let (_tree, p) = paragraph(
            "<span data-type=\"strong inline-math\" data-content=\"x\"> value </span>",
            true,
        );
        let math = p.first_child().unwrap();
        assert_eq!(math.data().text_mark_inline_math_content, "x");
        assert_eq!(math.data().text_mark_text_content, "");
        assert!(math.next().is_none());
    }

    #[test]
    fn style_boundary_movement_uses_go_blank_character_set() {
        for (input, moved) in [
            (" value ", true),
            ("\tvalue\t", true),
            ("　value　", true),
            ("\u{00a0}value\u{00a0}", false),
        ] {
            // Call metadata normalization directly: the separate BlockDOM
            // postpass intentionally normalizes NBSP on plain text nodes.
            let dom = crate::dom::parse(&format!("<span data-type=\"strong\">{input}</span>"));
            let span = dom.nodes[dom.root].children[0];
            let mut tree = Tree::new();
            let p = tree.new_node_raw(NodeType::NodeParagraph);
            tree.root_raw().append_child(p);
            let mark = tree.new_node_raw(NodeType::NodeTextMark);
            p.append_child(mark);
            set_text_mark_node(&mut tree, mark, &dom, span, &ParseOptions::new());
            assert_eq!(mark.previous().is_some(), moved, "{input:?}");
            assert_eq!(mark.next().is_some(), moved, "{input:?}");
            assert_eq!(mark.data().text_mark_text_content, "value");
        }
    }
}

#[cfg(test)]
mod go_editor_structural_tests {
    use crate::{
        ast::{walk_raw, NodeType, WalkStatus},
        editor::{WORD_JOINER, ZWSP},
        lute::Lute,
        parse::parse,
    };
    fn engine() -> Lute {
        let mut l = Lute::new();
        l.parse_options.protyle_wysiwyg = true;
        l.render_options.protyle_wysiwyg = true;
        l.parse_options.kramdown_block_ial = true;
        l.parse_options.kramdown_span_ial = true;
        l.render_options.kramdown_block_ial = true;
        l.render_options.kramdown_span_ial = true;
        l
    }
    fn placeholder_engine() -> Lute {
        let mut l = engine();
        l.parse_options.tag = true;
        l.parse_options.text_mark = true;
        l.parse_options.html_tag2_text_mark = true;
        l.render_options.auto_space = false;
        l.parse_options.spin = true;
        l
    }
    fn placeholder_dom(s: &str) -> String {
        format!(
            r#"<div data-node-id="20260830120000-abcdefg" data-node-index="1" data-type="NodeParagraph" class="p" updated="20260830120000"><div contenteditable="true" spellcheck="false">{s}</div><div class="protyle-attr" contenteditable="false">{ZWSP}</div></div>"#
        )
    }
    // Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderCompatibility
    #[test]
    fn go_placeholder_text_mark_content() {
        let l = placeholder_engine();
        for marker in [ZWSP, "\u{feff}", WORD_JOINER] {
            for (open, close) in [
                (r#"<span data-type="code">"#, "</span>"),
                (r#"<span data-type="strong code">"#, "</span>"),
                (r#"<span data-type="kbd">"#, "</span>"),
                ("<kbd>", "</kbd>"),
                (r#"<span data-type="tag">"#, "</span>"),
                ("<code>", "</code>"),
            ] {
                let dom = placeholder_dom(&format!(
                    "outside{WORD_JOINER}{open}{marker}value{WORD_JOINER}tail{close}"
                ));
                let t = l.blockdom2tree(&dom);
                let mut content = String::new();
                walk_raw(t.root_raw(), &mut |n, e| {
                    if e && n.typ() == NodeType::NodeTextMark && content.is_empty() {
                        content = n.data().text_mark_text_content.clone();
                    }
                    WalkStatus::WalkContinue
                });
                assert_eq!(content, format!("value{WORD_JOINER}tail"));
            }
        }
    }
    // Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderPreservesAttributeWordJoiners
    #[test]
    fn go_placeholder_attribute_joiners() {
        let l = placeholder_engine();
        let href = format!("before{WORD_JOINER}after");
        let math = format!("x{WORD_JOINER}y");
        let dom = placeholder_dom(&format!(
            r#"<span data-type="a" data-href="{href}">link</span><span data-type="inline-math" data-content="{math}"></span>"#
        ));
        let t = l.blockdom2tree(&dom);
        let (mut got_href, mut got_math) = (String::new(), String::new());
        walk_raw(t.root_raw(), &mut |n, e| {
            if e && n.typ() == NodeType::NodeTextMark {
                if n.is_text_mark_type("a") {
                    got_href = n.data().text_mark_a_href.clone();
                }
                if n.is_text_mark_type("inline-math") {
                    got_math = n.data().text_mark_inline_math_content.clone();
                }
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(got_href, href);
        assert_eq!(got_math, math);
    }
    // Go: test/custom_block_test.go::TestCustomBlockOptionAndFence
    #[test]
    fn go_custom_block_info() {
        let mut l = Lute::new();
        l.parse_options.custom_block = true;
        let t = parse(
            "",
            b";;;plugin:test\ncontent\n;;;\nafter",
            l.parse_options.clone(),
        );
        assert_eq!(
            t.root_raw().first_child().unwrap().data().custom_block_info,
            "plugin:test"
        );
    }
    fn callout_engine() -> Lute {
        let mut l = Lute::new();
        l.parse_options.callout = true;
        l.parse_options.protyle_wysiwyg = true;
        l.render_options.protyle_wysiwyg = true;
        l.render_options.auto_space = true;
        l
    }
    fn check_callout(l: &Lute, md: &str, icon_type: i32, icon: &str, title: &str) {
        let t = parse("callout-image-icon", md.as_bytes(), l.parse_options.clone());
        let n = t.root_raw().first_child().unwrap();
        assert_eq!(n.typ(), NodeType::NodeCallout, "{md}");
        let d = n.data();
        assert_eq!(d.callout_icon_type, icon_type, "{md}");
        assert_eq!(d.callout_icon, icon, "{md}");
        assert_eq!(d.callout_title, title, "{md}");
    }
    // Go: test/callout_icon_test.go::TestCalloutImageIconParse
    #[test]
    fn go_callout_image_parse() {
        let l = callout_engine();
        for (icon, title) in [
            ("/emojis/1/b3log.png", "Local"),
            (
                "api/icon/getDynamicIcon?type=1&color=%23d23f31&date=20260807",
                "Dynamic",
            ),
            ("http://example.com/avatar", "HTTP"),
            ("https://example.com/a(b)?foo=bar&baz=qux#fragment", "HTTPS"),
            ("https://example.com/icon%20name.png", "Encoded"),
        ] {
            check_callout(
                &l,
                &format!("> [!NOTE] ![callout-icon](<{icon}>) {title}\n> Content"),
                1,
                icon,
                title,
            );
        }
        check_callout(
            &l,
            "> [!NOTE] ![callout-icon](https://example.com/icon.png) Plain\n> Content",
            1,
            "https://example.com/icon.png",
            "Plain",
        );
    }
    // Go: test/callout_icon_test.go::TestCalloutImageIconDoesNotConsumeTitle
    #[test]
    fn go_callout_icon_preserves_title() {
        let l = callout_engine();
        for title in [
            "https://example.com/icon.png Note",
            "![cover](<https://example.com/icon.png>) Note",
            "![callout-icon](<javascript:alert(1)>) Note",
            "![callout-icon](<data:image/png;base64,AA>) Note",
            "![callout-icon](<//example.com/icon.png>) Note",
            "![callout-icon](<https://>) Note",
        ] {
            check_callout(
                &l,
                &format!("> [!NOTE] {title}\n> Content"),
                0,
                crate::parse::callout::get_callout_icon("NOTE"),
                title,
            );
        }
    }
    // Go: test/callout_icon_test.go::TestCalloutImageIconRoundTrip
    #[test]
    fn go_callout_image_roundtrip() {
        let icon = "https://b3logfile.com/avatar/1734703705652.png?size=64&theme=light";
        let dom = format!(
            r#"<div data-subtype="NOTE" data-node-id="20260807120000-abcdefg" data-node-index="1" data-type="NodeCallout" class="callout"><div class="callout-info" contenteditable="false"><span class="callout-icon"><img class="callout-img" src="{icon}"></span><span contenteditable="true" spellcheck="false" class="callout-title">Note</span></div><div class="callout-content"><div data-node-id="20260807120001-abcdefg" data-type="NodeParagraph" class="p"><div contenteditable="true" spellcheck="false">Content</div><div class="protyle-attr" contenteditable="false">&ZeroWidthSpace;</div></div></div><div class="protyle-attr" contenteditable="false">&ZeroWidthSpace;</div></div>"#
        );
        let l = callout_engine();
        let md = l.blockdom2stdmd(&dom);
        assert_eq!(
            md,
            format!("> [!NOTE] ![callout-icon](<{icon}>) Note\n> Content\n")
        );
        let formatted = l.format("callout-image-icon", &md);
        let again = l.format("callout-image-icon", &formatted);
        assert_eq!(formatted, again);
        check_callout(&l, &again, 1, icon, "Note");
        let out = l.md2blockdom(&again, true);
        assert!(out.contains(
            r#"src="https://b3logfile.com/avatar/1734703705652.png?size=64&amp;theme=light""#
        ));
        assert!(out.contains(
            r#"<span contenteditable="true" spellcheck="false" class="callout-title">Note</span>"#
        ));
    }
    // Go: test/callout_icon_test.go::TestCalloutEmptyTitleRoundTrip
    #[test]
    fn go_callout_empty_title() {
        let l = callout_engine();
        let md = "> [!NOTE]\n> Content";
        let t = parse("callout-image-icon", md.as_bytes(), l.parse_options.clone());
        assert!(
            !t.root_raw()
                .first_child()
                .unwrap()
                .data()
                .callout_title_explicit
        );
        assert!(l
            .md2blockdom(md, true)
            .contains(r#"class="callout-title">Note</span>"#));
        let md = "> [!NOTE] ✏️\n> Content\n";
        let t = parse("callout-image-icon", md.as_bytes(), l.parse_options.clone());
        let n = t.root_raw().first_child().unwrap();
        assert!(n.data().callout_title_explicit);
        assert_eq!(n.data().callout_title, "");
        let dom = l.md2blockdom(md, true);
        assert!(dom.contains(r#"class="callout-title"></span>"#));
        assert_eq!(l.blockdom2stdmd(&dom), md);
    }
    fn recursive_engine() -> Lute {
        let mut l = engine();
        l.render_options.data_task = true;
        l
    }
    fn list_types(l: &Lute, dom: &str) -> Vec<i32> {
        let t = l.blockdom2tree(dom);
        let mut ret = vec![];
        walk_raw(t.root_raw(), &mut |n, e| {
            if e && n.typ() == NodeType::NodeList {
                ret.push(n.data().list_data.as_ref().unwrap().typ);
            }
            WalkStatus::WalkContinue
        });
        ret
    }
    fn task_count(l: &Lute, dom: &str) -> usize {
        let t = l.blockdom2tree(dom);
        let mut ret = 0;
        walk_raw(t.root_raw(), &mut |n, e| {
            if e && n.typ() == NodeType::NodeTaskListItemMarker {
                ret += 1;
            }
            WalkStatus::WalkContinue
        });
        ret
    }
    fn paragraph_texts(l: &Lute, dom: &str) -> Vec<String> {
        let t = l.blockdom2tree(dom);
        let mut ret = vec![];
        walk_raw(t.root_raw(), &mut |n, e| {
            if e && n.typ() == NodeType::NodeParagraph {
                ret.push(n.text());
            }
            WalkStatus::WalkContinue
        });
        ret
    }
    fn ordered_numbers(l: &Lute, dom: &str) {
        let t = l.blockdom2tree(dom);
        walk_raw(t.root_raw(), &mut |n, e| {
            if e && n.typ() == NodeType::NodeList && n.data().list_data.as_ref().unwrap().typ == 1 {
                let mut num = 1;
                for li in n.children() {
                    if li.typ() == NodeType::NodeListItem {
                        assert_eq!(li.data().list_data.as_ref().unwrap().num, num);
                        num += 1;
                    }
                }
            }
            WalkStatus::WalkContinue
        });
    }
    // Go: test/recursive_list_conversion_test.go::TestConvertListType
    #[test]
    fn go_convert_list_type() {
        let l = recursive_engine();
        let input = l.md2blockdom(
            "1. outer\n   1. nested\n      1. deep\n   - [ ] task\n     1. ordered in task",
            false,
        );
        let converted = l.convert_list_type(&input, "u");
        assert_eq!(list_types(&l, &converted), [0, 0, 0, 0, 0]);
        assert_eq!(task_count(&l, &converted), 0);
        assert_eq!(l.convert_list_type(&input, "invalid"), input);
        let input = l.md2blockdom("- outer\n  1. nested", false);
        assert_eq!(list_types(&l, &l.convert_list_type(&input, "u")), [0, 0]);
    }
    // Go: test/recursive_list_conversion_test.go::TestConvertListTypePairs
    #[test]
    fn go_convert_list_pairs() {
        let l = recursive_engine();
        for (md, target, typ) in [
            ("- outer\n  - nested", "o", 1),
            ("- outer\n  - nested", "t", 3),
            ("1. outer\n   1. nested", "u", 0),
            ("1. outer\n   1. nested", "t", 3),
            ("- [ ] outer\n  - [x] nested", "u", 0),
            ("- [ ] outer\n  - [x] nested", "o", 1),
        ] {
            let converted = l.convert_list_type(&l.md2blockdom(md, false), target);
            assert_eq!(list_types(&l, &converted), [typ, typ], "{md} -> {target}");
            assert_eq!(task_count(&l, &converted), if typ == 3 { 2 } else { 0 });
            if typ == 1 {
                ordered_numbers(&l, &converted);
            }
        }
    }
    // Go: test/recursive_list_conversion_test.go::TestConvertTaskListType
    #[test]
    fn go_convert_task_list() {
        let l = recursive_engine();
        let input = l.md2blockdom(
            "- [x] outer\n  - [ ] nested\n    1. ordered\n       - [ ] deep task",
            false,
        );
        let converted = l.convert_list_type(&input, "o");
        assert_eq!(list_types(&l, &converted), [1, 1, 1, 1]);
        assert_eq!(task_count(&l, &converted), 0);
        ordered_numbers(&l, &converted);
    }
    // Go: test/recursive_list_conversion_test.go::TestCancelListRecursively
    #[test]
    fn go_cancel_list_recursive() {
        let l = recursive_engine();
        let input = l.md2blockdom(
            "- outer\n  - nested\n    1. ordered\n       - deep unordered\n- tail",
            false,
        );
        assert_eq!(list_types(&l, &l.cancel_list(&input)), [0, 1, 0]);
        let converted = l.cancel_list_recursively(&input);
        assert_eq!(list_types(&l, &converted), [1]);
        assert_eq!(
            paragraph_texts(&l, &converted),
            ["outer", "nested", "ordered", "deep unordered", "tail"]
        );
    }
    // Go: test/recursive_list_conversion_test.go::TestCancelListKeepsExistingMultipleRootOrder
    #[test]
    fn go_cancel_list_root_order() {
        let l = recursive_engine();
        let input = l.md2blockdom("- list item\n\ntrailing paragraph", false);
        assert_eq!(
            paragraph_texts(&l, &l.cancel_list(&input)),
            ["trailing paragraph", "list item"]
        );
        assert_eq!(
            paragraph_texts(&l, &l.cancel_list_recursively(&input)),
            ["list item", "trailing paragraph"]
        );
    }
    // Go: test/protyle_mindmap_code_test.go::TestProtyleMindmapIsOrdinaryCode
    #[test]
    fn go_mindmap_ordinary_code() {
        let l = engine();
        for source in [
            "",
            "1\n",
            "- Root\n  - Child\n",
            "\n  &lt;script&gt; &amp; `code`  \n\n",
        ] {
            let md = format!("```mindmap\n{source}```\n");
            let (dom, tree) = l.md2blockdomtree(&md, false);
            for out in [
                dom.clone(),
                l.spin_blockdom(&dom),
                l.render_node_blockdom(&tree.root().first_child().unwrap()),
            ] {
                assert!(
                    out.contains(r#"class="code-block""#) && out.contains(">mindmap</span>"),
                    "{out}"
                );
                assert!(
                    !out.contains(r#"class="render-node""#)
                        && !out.contains(r#"data-subtype="mindmap""#),
                    "{out}"
                );
                let parsed = l.blockdom2tree(&out);
                let n = parsed.root_raw().first_child().unwrap();
                assert_eq!(n.typ(), NodeType::NodeCodeBlock);
                assert_eq!(
                    n.child_by_type(NodeType::NodeCodeBlockCode)
                        .unwrap()
                        .tokens_str(),
                    source
                );
                assert_eq!(
                    n.child_by_type(NodeType::NodeCodeBlockFenceInfoMarker)
                        .unwrap()
                        .data()
                        .code_block_info,
                    b"mindmap"
                );
            }
            let preview = String::from_utf8(crate::render::Renderer::render(
                &mut crate::render::protyle_preview::ProtylePreviewRenderer::new(
                    &tree,
                    l.render_options.clone(),
                    l.parse_options.clone(),
                ),
            ))
            .unwrap();
            assert!(!preview.contains(r#"data-subtype="mindmap""#));
            assert!(preview.contains(r#"class="code-block"#), "{preview}");
            let md = l.blockdom2stdmd(&dom);
            assert!(md.contains(source) && md.contains("```mindmap"), "{md}");
            for out in [l.blockdom2html(&dom), l.blockdom2richhtml(&dom)] {
                assert!(!out.contains("data-code=") && out.contains("<pre"), "{out}");
            }
        }
        let caret = l.md2blockdom(
            &format!("```mindmap\n1{}2\n```", crate::editor::CARET),
            false,
        );
        let out = l.spin_blockdom(&caret);
        assert!(
            out.contains("<wbr>") && !out.contains(r#"class="render-node""#),
            "{out}"
        );
    }
}

#[cfg(test)]
mod go_list_metadata_tests {
    use crate::{
        ast::{walk_raw, NodeType, RawNode, WalkStatus},
        lute::Lute,
    };

    // Go: test/m2p_test.go::TestMd2BlockDOMTaskMarkerFollowedByLink
    #[test]
    fn go_task_marker_followed_by_link_keeps_original_list_kind() {
        let mut engine = Lute::new();
        engine.parse_options.protyle_wysiwyg = true;
        engine.render_options.protyle_wysiwyg = true;
        engine.parse_options.arbitrary_task_list_item_marker = true;
        for (markdown, expected) in [
            ("- [1](siyuan://blocks/20260802201342-8lzm8cy)", 0),
            ("- [1]", 3),
            ("- [x] task", 3),
            ("- [/] task", 3),
        ] {
            let (_, tree) = engine.try_md2blockdomtree(markdown, true).unwrap();
            let list = tree.root_raw().first_child().unwrap();
            assert_eq!(
                list.data().list_data.as_ref().unwrap().typ,
                expected,
                "{markdown}"
            );
        }
    }
    fn contains_text(node: RawNode) -> bool {
        (node.typ() == NodeType::NodeText && !node.tokens().is_empty())
            || node.children().any(contains_text)
    }
    // Go: test/list_item_nested_test.go::TestIssue17890EmptyListItemNoDuplicateParagraph
    #[test]
    fn go_empty_list_items_have_at_most_one_empty_paragraph() {
        for markdown in [
            "- 1\n\n  -\n- 3\n- 4\n",
            "-\n  -\n- bar\n",
            "-\n  -\n  - baz\n",
        ] {
            let mut engine = Lute::new();
            engine.parse_options.protyle_wysiwyg = true;
            engine.render_options.protyle_wysiwyg = true;
            engine.parse_options.kramdown_block_ial = true;
            engine.parse_options.kramdown_span_ial = true;
            engine.render_options.kramdown_block_ial = true;
            engine.render_options.kramdown_span_ial = true;
            engine.parse_options.spin = true;
            engine.parse_options.ensure_list_item_paragraph = true;
            let (_, tree) = engine.try_md2blockdomtree(markdown, false).unwrap();
            let mut offenders = Vec::new();
            walk_raw(tree.root_raw(), &mut |node, entering| {
                if entering && node.typ() == NodeType::NodeListItem {
                    let empty = node
                        .children()
                        .filter(|child| {
                            child.typ() == NodeType::NodeParagraph && !contains_text(*child)
                        })
                        .count();
                    if empty > 1 {
                        offenders.push(node.id_owned());
                    }
                }
                WalkStatus::WalkContinue
            });
            assert!(
                offenders.is_empty(),
                "{markdown:?}: duplicate empty paragraphs in {offenders:?}"
            );
        }
    }
}
