// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 对外 API，对应 Go 版 lute.Lute 与 JS 版 Lute 对象。

use std::collections::HashMap;

use crate::ast::NodeType;
use crate::options::{ParseOptions, RenderOptions};
use crate::parse;
use crate::render;
use crate::render::format::FormatRenderer;
use crate::render::Renderer;

/// Lute 引擎实例。
pub struct Lute {
    pub parse_options: ParseOptions,
    pub render_options: RenderOptions,
    /// 用户自定义渲染器，按渲染器分组名索引（HTML2Md / Md2BlockDOM 等）。
    pub js_renderers: HashMap<String, render::ExtRendererFunc>,
}

impl Default for Lute {
    fn default() -> Self {
        Self::new()
    }
}

impl Lute {
    pub fn new() -> Self {
        Self {
            parse_options: ParseOptions::new(),
            render_options: RenderOptions::new(),
            js_renderers: HashMap::new(),
        }
    }

    /// 对应 Go 版 Lute.SetJSRenderers（已解析为分组名到回调的映射）。
    pub fn set_js_renderers(&mut self, renderers: HashMap<String, render::ExtRendererFunc>) {
        self.js_renderers = renderers;
    }

    fn ext(&self, group: &str) -> Option<render::ExtRendererFunc> {
        self.js_renderers.get(group).cloned()
    }

    /// 对应 Go 版 Lute.MarkdownStr。
    pub fn markdown(&self, name: &str, markdown: &str) -> String {
        let tree = parse::parse(name, markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::HtmlRenderer::with_parse_options(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        // Go 版 Markdown 会把 Md2HTMLRendererFuncs 逐个装进 HTML 渲染器。
        r.base.ext_renderer = self.ext("Md2HTML");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.FormatStr。
    pub fn format(&self, name: &str, markdown: &str) -> String {
        let tree = parse::parse(name, markdown.as_bytes(), self.parse_options.clone());
        let mut r = FormatRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.TextBundleStr。
    pub fn text_bundle(
        &self,
        name: &str,
        markdown: &str,
        link_prefixes: Vec<String>,
    ) -> (String, Option<Vec<String>>) {
        let tree = parse::parse(name, markdown.as_bytes(), self.parse_options.clone());
        let mut r = FormatRenderer::new_text_bundle(
            &tree,
            link_prefixes,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        let out = String::from_utf8_lossy(&r.render()).to_string();
        let links = if r.original_links.is_empty() {
            None
        } else {
            Some(r.original_links.clone())
        };
        (out, links)
    }

    /// 对应 Go 版 Lute.RenderJSON。
    pub fn render_json(&self, markdown: &str) -> String {
        let tree = parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        render::json::render_json(&tree, &self.render_options)
    }

    /// 对应 Go 版 Lute.RenderEChartsJSON。
    pub fn render_echarts_json(&self, markdown: &str) -> String {
        let tree = parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::mindmap::EChartsJsonRenderer::new(&tree);
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.RenderKityMinderJSON。
    pub fn render_kityminder_json(&self, markdown: &str) -> String {
        let tree = parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::mindmap::KityMinderJsonRenderer::new(&tree);
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 render.EChartsMindmapStr。
    pub fn echarts_mindmap_str(list_content: &str) -> String {
        render::mindmap::echarts_mindmap_str(list_content)
    }

    /// 对应 Go 版 render.Sanitize。
    pub fn sanitize(s: &str) -> String {
        render::sanitizer::sanitize_str(s)
    }

    pub fn ol2ul(&self, html: &str) -> String {
        crate::structure::ol2ul(html, &self.render_options, &self.parse_options)
    }
    pub fn ul2ol(&self, html: &str) -> String {
        crate::structure::ul2ol(html, &self.render_options, &self.parse_options)
    }
    pub fn ul2tl(&self, html: &str) -> String {
        crate::structure::to_task_list(html, &self.render_options, &self.parse_options)
    }
    pub fn ol2tl(&self, html: &str) -> String {
        crate::structure::to_task_list(html, &self.render_options, &self.parse_options)
    }
    pub fn tl2ul(&self, html: &str) -> String {
        crate::structure::tl2ul(html, &self.render_options, &self.parse_options)
    }
    pub fn tl2ol(&self, html: &str) -> String {
        crate::structure::tl2ol(html, &self.render_options, &self.parse_options)
    }
    pub fn convert_list_type(&self, html: &str, target: &str) -> String {
        crate::structure::convert_list_type(html, target, &self.render_options, &self.parse_options)
    }
    pub fn blockquote2callout(&self, html: &str) -> String {
        crate::structure::blockquote2callout(html, &self.render_options, &self.parse_options)
    }
    pub fn callout2blockquote(&self, html: &str) -> String {
        crate::structure::callout2blockquote(html, &self.render_options, &self.parse_options)
    }
    pub fn cancel_blockquote(&self, html: &str) -> String {
        crate::structure::cancel_blockquote(html, &self.render_options, &self.parse_options)
    }
    pub fn cancel_callout(&self, html: &str) -> String {
        crate::structure::cancel_callout(html, &self.render_options, &self.parse_options)
    }
    pub fn cancel_list(&self, html: &str) -> String {
        crate::structure::cancel_list(html, &self.render_options, &self.parse_options)
    }
    pub fn cancel_list_recursively(&self, html: &str) -> String {
        crate::structure::cancel_list_recursively(html, &self.render_options, &self.parse_options)
    }
    pub fn cancel_superblock(&self, html: &str) -> String {
        crate::structure::cancel_superblock(html, &self.render_options, &self.parse_options)
    }
    pub fn blocks2ps(&self, html: &str) -> String {
        crate::structure::blocks2ps(html, &self.render_options, &self.parse_options)
    }
    pub fn blocks2hs(&self, html: &str, level: &str) -> String {
        crate::structure::blocks2hs(html, level, &self.render_options, &self.parse_options)
    }

    pub fn spin_blockdom(&self, blockdom: &str) -> String {
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        let mut tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            options.clone(),
            &self.render_options,
        );
        let mut mindmap_types = std::collections::HashMap::new();
        crate::ast::walk_raw(tree.root_raw(), &mut |node, entering| {
            if entering
                && !node.data().id.is_empty()
                && matches!(
                    node.typ(),
                    NodeType::NodeMindmap | NodeType::NodeMindmapItem
                )
            {
                mindmap_types.insert(node.data().id.clone(), node.typ());
            }
            crate::ast::WalkStatus::WalkContinue
        });
        let wrappers = crate::blockdom::wrap_tab_item_fragments(&mut tree);
        crate::blockdom::normalize_spin_caret_newline(&tree);
        let mut markdown = crate::blockdom::tree2md_with_render_options(
            &mut tree,
            options.clone(),
            &self.render_options,
        );
        markdown = markdown.replace(crate::editor::ZWSP, "");
        let mut tree = crate::parse::parse("", markdown.as_bytes(), options.clone());
        crate::ast::walk_raw(tree.root_raw(), &mut |node, entering| {
            if entering {
                if let Some(&typ) = mindmap_types.get(&node.data().id) {
                    if (typ == NodeType::NodeMindmap && node.typ() == NodeType::NodeList)
                        || (typ == NodeType::NodeMindmapItem
                            && node.typ() == NodeType::NodeListItem)
                    {
                        node.data_mut().typ = typ;
                    }
                }
            }
            crate::ast::WalkStatus::WalkContinue
        });
        crate::blockdom::unwrap_tab_item_fragments(&tree, &wrappers);

        if let Some(first) = tree.root_raw().first_child() {
            if let Some(ial) = tree
                .root_raw()
                .last_child()
                .and_then(|last| last.previous())
            {
                if first.typ() == NodeType::NodeParagraph
                    && first.data().id.is_empty()
                    && ial.typ() == NodeType::NodeKramdownBlockIAL
                {
                    if let Some(previous) = ial.previous().filter(|previous| *previous != first) {
                        first.data_mut().id = std::mem::take(&mut previous.data_mut().id);
                        first.data_mut().kramdown_ial =
                            std::mem::take(&mut previous.data_mut().kramdown_ial);
                        first.insert_after(ial);
                    }
                }
            }
            if first.typ() == NodeType::NodeKramdownBlockIAL
                && first.next().is_some_and(|next| {
                    next.typ() == NodeType::NodeKramdownBlockIAL
                        && crate::render::is_doc_ial(next.tokens())
                })
            {
                let ial = crate::parse::ial::tokens2_ial(first.tokens());
                let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
                paragraph.data_mut().id = ial
                    .iter()
                    .find(|(name, _)| name == "id")
                    .map(|(_, id)| id.clone())
                    .unwrap_or_default();
                paragraph.data_mut().kramdown_ial = ial;
                first.insert_before(paragraph);
            }
        }
        crate::parse::text_marks2inlines_with_nested_syntax(&mut tree, &options);
        crate::parse::nested_inlines2flatted_spans_hybrid(&mut tree, false);
        crate::blockdom::tree2blockdom(&tree, self.render_options.clone(), options)
    }

    /// 对应 Go 版 Lute.Md2BlockDOM。
    pub fn md2blockdom(&self, markdown: &str, reserve_empty_paragraph: bool) -> String {
        match self.try_md2blockdom(markdown, reserve_empty_paragraph) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        }
    }

    /// Md2BlockDOM 的可失败版本。
    ///
    /// Go 版在 Protyle 渲染链接引用定义时会解引用 nil 指针；wasm 侧需要把同样的
    /// 错误抛给 JS，而不是让 Rust panic 变成不可捕获的 trap。
    pub fn try_md2blockdom(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<String, String> {
        self.md2blockdomtree_impl(markdown, reserve_empty_paragraph, false)
            .map(|(html, _)| html)
    }

    /// 对应 Go 版 Lute.BlockDOM2Tree + BlockDOM2Md。
    pub fn blockdom2md(&self, blockdom: &str) -> String {
        // 对应 Go 版 BlockDOM2Md：转换后去掉全部零宽空格。
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        crate::blockdom::blockdom2md_with_render_options(blockdom, options, &self.render_options)
            .replace(crate::editor::ZWSP, "")
    }

    /// 对应 Go 版私有方法 blockDOM2Md（保留零宽空格，供 BlockDOM2HTML 等复用）。
    fn blockdom2md_raw(&self, blockdom: &str) -> String {
        crate::blockdom::blockdom2md_with_render_options(
            blockdom,
            self.parse_options.clone(),
            &self.render_options,
        )
    }

    /// 对应 Go 版 Lute.BlockDOM2StdMd。
    pub fn blockdom2stdmd(&self, blockdom: &str) -> String {
        let mut parse_options = self.parse_options.clone();
        parse_options.keep_escaped = false;
        let mut tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            parse_options.clone(),
            &self.render_options,
        );
        crate::blockdom::wrap_tab_item_fragments(&mut tree);
        // 对应 Go 版 removeBlockDOMZwsp：渲染前清掉占位零宽空格。
        crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering {
                let tokens = n.tokens();
                if tokens
                    .windows(crate::editor::ZWSP.len())
                    .any(|w| w == crate::editor::ZWSP.as_bytes())
                {
                    n.set_tokens(crate::vditor::replace_all(
                        tokens,
                        crate::editor::ZWSP.as_bytes(),
                        b"",
                    ));
                }
                let content = n.data().text_mark_text_content.clone();
                if content.contains(crate::editor::ZWSP) {
                    n.data_mut().text_mark_text_content = content.replace(crate::editor::ZWSP, "");
                }
            }
            crate::ast::WalkStatus::WalkContinue
        });
        crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering
                && (NodeType::NodeKramdownBlockIAL == n.typ()
                    || NodeType::NodeKramdownSpanIAL == n.typ())
            {
                n.set_tokens(Vec::new());
            }
            crate::ast::WalkStatus::WalkContinue
        });
        let mut options = RenderOptions::new();
        options.auto_space = false;
        options.fix_term_typo = false;
        options.kramdown_block_ial = true;
        options.kramdown_span_ial = true;
        options.keep_paragraph_beginning_space = true;
        options.omit_table_cell_ial = true;
        options.tabs_markdown = true;
        options.unordered_list_marker = self.render_options.unordered_list_marker.clone();
        options.export_normalize_task_list_marker =
            self.render_options.export_normalize_task_list_marker;
        let mut r = FormatRenderer::new_protyle_export(&tree, options, parse_options);
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.BlockDOM2HTML。
    pub fn blockdom2html(&self, blockdom: &str) -> String {
        let markdown = self.blockdom2md_raw(blockdom);
        self.markdown("", &markdown)
    }

    /// 对应 Go 版 Lute.BlockDOM2Text。
    pub fn blockdom2text(&self, blockdom: &str) -> String {
        let tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            self.parse_options.clone(),
            &self.render_options,
        );
        tree.root_raw().text()
    }

    /// 对应 Go 版 Lute.BlockDOM2TextLen。
    pub fn blockdom2textlen(&self, blockdom: &str) -> usize {
        let tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            self.parse_options.clone(),
            &self.render_options,
        );
        tree.root_raw().text_len()
    }

    /// 对应 Go 版 Lute.BlockDOM2Content。
    pub fn blockdom2content(&self, blockdom: &str) -> String {
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        let tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            options,
            &self.render_options,
        );
        tree.root_raw().content()
    }

    /// 对应 Go 版 Lute.BlockDOM2RichHTML。
    pub fn blockdom2richhtml(&self, blockdom: &str) -> String {
        let tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            self.parse_options.clone(),
            &self.render_options,
        );
        let mut r = render::HtmlRenderer::with_parse_options(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.set_text_mark_standard_tag();
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.BlockDOM2InlineBlockDOM。
    pub fn blockdom2inline(&self, blockdom: &str) -> String {
        let markdown = self.blockdom2md_raw(blockdom);
        let options = self.parse_options.clone();
        let mut tree = crate::parse::parse("", markdown.as_bytes(), options.clone());
        enum Item {
            Node(crate::ast::RawNode),
            Text(Vec<u8>),
        }
        let mut items: Vec<Item> = Vec::new();
        let root = tree.root_raw();
        crate::ast::walk_raw(root, &mut |n, entering| {
            if !entering {
                return crate::ast::WalkStatus::WalkContinue;
            }
            if NodeType::NodeTableCell == n.typ() {
                let space = tree.new_node_raw(NodeType::NodeText);
                space.set_tokens(b" ".to_vec());
                n.append_child(space);
                return crate::ast::WalkStatus::WalkContinue;
            }
            if !n.is_block() {
                if !matches!(
                    n.typ(),
                    NodeType::NodeCodeBlockCode
                        | NodeType::NodeMathBlockContent
                        | NodeType::NodeTaskListItemMarker
                        | NodeType::NodeTableHead
                        | NodeType::NodeTableRow
                        | NodeType::NodeTableCell
                ) {
                    if NodeType::NodeBlockquoteMarker == n.typ() {
                        items.push(Item::Text(b">".to_vec()));
                        return crate::ast::WalkStatus::WalkSkipChildren;
                    }
                    items.push(Item::Node(n));
                    return crate::ast::WalkStatus::WalkSkipChildren;
                }
            } else if NodeType::NodeHTMLBlock == n.typ() {
                items.push(Item::Text(n.tokens_vec()));
                return crate::ast::WalkStatus::WalkSkipChildren;
            }
            crate::ast::WalkStatus::WalkContinue
        });
        let mut inlines = Vec::new();
        for item in items {
            match item {
                Item::Node(n) => inlines.push(n),
                Item::Text(t) => {
                    let text = tree.new_node_raw(NodeType::NodeText);
                    text.set_tokens(t);
                    inlines.push(text);
                }
            }
        }
        let mut children = Vec::new();
        let mut c = tree.root_raw().first_child();
        while let Some(cur) = c {
            children.push(cur);
            c = cur.next();
        }
        for n in children {
            n.unlink();
        }
        for n in inlines {
            tree.root_raw().append_child(n);
        }
        let mut r =
            render::protyle::ProtyleRenderer::new(&tree, self.render_options.clone(), options);
        let out = r.render();
        String::from_utf8_lossy(&out).trim().to_string()
    }

    /// 对应 Go 版 Lute.Md2BlockDOMWithAutoLink。
    pub fn md2blockdom_with_auto_link(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> String {
        match self.try_md2blockdom_with_auto_link(markdown, reserve_empty_paragraph) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        }
    }

    /// Md2BlockDOMWithAutoLink 的可失败版本。
    pub fn try_md2blockdom_with_auto_link(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<String, String> {
        self.md2blockdomtree_impl(markdown, reserve_empty_paragraph, true)
            .map(|(html, _)| html)
    }

    /// 对应 Go 版 Lute.InlineMd2BlockDOM。
    pub fn inline_md2blockdom(&self, markdown: &str) -> String {
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        let mut tree = crate::parse::inline("", markdown.as_bytes(), options.clone());
        crate::parse::nested_inlines2flatted_spans_hybrid(&mut tree, false);
        let mut r =
            render::protyle::ProtyleRenderer::new(&tree, self.render_options.clone(), options);
        r.base.ext_renderer = self.ext("Md2BlockDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.BlockDOM2EscapeMarkerContent。
    pub fn blockdom2_escape_marker_content(&self, blockdom: &str) -> String {
        let tree = crate::blockdom::blockdom2tree_with_render_options(
            blockdom,
            self.parse_options.clone(),
            &self.render_options,
        );
        tree.root_raw().escape_marker_content()
    }

    /// 对应 Go 版 Lute.ProtylePreviewStr。
    pub fn protyle_preview_str(&self, name: &str, markdown: &str) -> String {
        let tree = crate::parse::parse(name, markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::protyle_preview::ProtylePreviewRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 JS 全局 Lute.BlockDOM2Content 使用的固定选项集。
    pub fn blockdom2content_global(dom: &str) -> String {
        let mut lute = Lute::new();
        lute.parse_options.protyle_wysiwyg = true;
        lute.render_options.protyle_wysiwyg = true;
        lute.parse_options.block_ref = true;
        lute.parse_options.file_annotation_ref = true;
        lute.parse_options.kramdown_block_ial = true;
        lute.parse_options.kramdown_span_ial = true;
        lute.render_options.kramdown_block_ial = true;
        lute.render_options.kramdown_span_ial = true;
        lute.parse_options.tag = true;
        lute.parse_options.super_block = true;
        lute.render_options.super_block = true;
        lute.parse_options.img_path_allow_space = true;
        lute.parse_options.git_conflict = true;
        lute.parse_options.mark = true;
        lute.parse_options.sup = true;
        lute.parse_options.sub = true;
        lute.parse_options.inline_math_allow_digit_after_open_marker = true;
        lute.parse_options.footnotes = false;
        lute.parse_options.toc = false;
        lute.render_options.toc = false;
        lute.parse_options.indent_code_block = false;
        lute.parse_options.paragraph_beginning_space = true;
        lute.render_options.keep_paragraph_beginning_space = true;
        lute.render_options.auto_space = false;
        lute.parse_options.heading_id = false;
        lute.render_options.heading_id = false;
        lute.parse_options.setext = false;
        lute.parse_options.yaml_front_matter = false;
        lute.parse_options.link_ref = false;
        lute.render_options.code_syntax_highlight = false;
        lute.render_options.sanitize = true;
        lute.blockdom2content(dom)
    }

    /// 对应 Go 版 Lute.GetLinkBase。
    pub fn get_link_base(&self) -> String {
        self.render_options.link_base.clone()
    }

    /// 对应 Go 版 Lute.Md2BlockDOMTree。
    pub fn md2blockdomtree(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> (String, crate::ast::Tree) {
        match self.try_md2blockdomtree(markdown, reserve_empty_paragraph) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        }
    }

    /// Md2BlockDOMTree 的可失败版本。
    pub fn try_md2blockdomtree(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<(String, crate::ast::Tree), String> {
        self.md2blockdomtree_impl(markdown, reserve_empty_paragraph, false)
    }

    fn md2blockdomtree_impl(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
        auto_link: bool,
    ) -> Result<(String, crate::ast::Tree), String> {
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        options.preserve_table_cell_source = true;
        if auto_link {
            options.protyle_wysiwyg_auto_link = true;
        }
        let mut tree = crate::parse::parse("", markdown.as_bytes(), options.clone());
        crate::parse::table_rich::normalize_markdown_table_cells(&mut tree);
        crate::parse::text_marks2inlines(&mut tree, &options);
        crate::parse::nested_inlines2flatted_spans_hybrid(&mut tree, false);
        prepend_footnote_anchors(&mut tree);
        if reserve_empty_paragraph {
            let mut empty = Vec::new();
            crate::ast::walk_raw(tree.root_raw(), &mut |node, entering| {
                if entering && node.is_empty_block_ial() {
                    empty.push(node);
                }
                crate::ast::WalkStatus::WalkContinue
            });
            for ial in empty {
                let attrs = crate::parse::ial::tokens2_ial(ial.tokens());
                let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
                paragraph.data_mut().id = attrs
                    .iter()
                    .find(|(key, _)| key == "id")
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default();
                paragraph.data_mut().kramdown_ial = attrs;
                ial.insert_before(paragraph);
            }
        }
        if let Some(err) = protyle_nil_pointer_error(&tree) {
            return Err(err);
        }
        let html = crate::blockdom::tree2blockdom_ext(
            &tree,
            self.render_options.clone(),
            options,
            self.ext("Md2BlockDOM"),
        );
        Ok((html, tree))
    }

    /// 对应 Go 版 Lute.HTML2BlockDOM。
    pub fn html2blockdom(&self, html: &str) -> String {
        let markdown = crate::html2md::html2markdown_ext(
            html,
            self.parse_options.clone(),
            self.render_options.clone(),
            self.ext("HTML2Md"),
        );
        let mut options = self.parse_options.clone();
        options.keep_escaped = true;
        options.preserve_table_cell_source = true;
        let tree = crate::parse::parse("", markdown.as_bytes(), options.clone());
        crate::blockdom::tree2blockdom_ext(
            &tree,
            self.render_options.clone(),
            options,
            self.ext("HTML2BlockDOM"),
        )
    }

    /// 对应 Go 版 Lute.Md2VditorDOM。
    pub fn md2vditor_dom(&self, markdown: &str) -> String {
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor::VditorRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.HTML2VditorDOM。
    pub fn html2vditor_dom(&self, html: &str) -> String {
        let markdown = self.html2markdown(html).0;
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor::VditorRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("HTML2VditorDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.VditorDOM2HTML。
    pub fn vditor_dom2html(&self, vhtml: &str) -> String {
        let markdown = self.vditor_dom2md(vhtml);
        self.markdown("", &markdown)
    }

    /// 对应 Go 版 Lute.VditorDOM2Md。
    pub fn vditor_dom2md(&self, html: &str) -> String {
        let html = html.replace(crate::editor::ZWSP, "");
        let markdown = crate::vditor::dom2md_with_options(
            &html,
            &self.parse_options,
            &self.render_options,
            false,
        );
        markdown.replace(crate::editor::ZWSP, "")
    }

    /// 对应 Go 版 Lute.SpinVditorDOM。
    pub fn spin_vditor_dom(&self, html: &str) -> String {
        let html = html.replace(crate::editor::FRONT_END_CARET, crate::editor::CARET);
        let markdown = crate::vditor::dom2md_with_options(
            &html,
            &self.parse_options,
            &self.render_options,
            false,
        );
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor::VditorRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorDOM");
        String::from_utf8_lossy(&r.render())
            .replace(crate::editor::CARET, crate::editor::FRONT_END_CARET)
    }

    /// 对应 Go 版 Lute.Md2VditorIRDOM。
    pub fn md2vditor_ir_dom(&self, markdown: &str) -> String {
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_ir::VditorIRRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorIRDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.HTML2VditorIRDOM。
    pub fn html2vditor_ir_dom(&self, html: &str) -> String {
        let markdown = self.html2markdown(html).0;
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_ir::VditorIRRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("HTML2VditorIRDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.VditorIRDOM2HTML。
    pub fn vditor_ir_dom2html(&self, vhtml: &str) -> String {
        let markdown = self.vditor_ir_dom2md(vhtml);
        self.markdown("", &markdown)
    }

    /// 对应 Go 版 Lute.VditorIRDOM2Md。
    pub fn vditor_ir_dom2md(&self, html: &str) -> String {
        let html = html.replace(crate::editor::ZWSP, "");
        let markdown = crate::vditor::dom2md_with_options(
            &html,
            &self.parse_options,
            &self.render_options,
            true,
        );
        markdown.replace(crate::editor::ZWSP, "")
    }

    /// 对应 Go 版 Lute.SpinVditorIRDOM。
    pub fn spin_vditor_ir_dom(&self, html: &str) -> String {
        let html = html.replace(crate::editor::FRONT_END_CARET, crate::editor::CARET);
        let markdown = crate::vditor::dom2md_with_options(
            &html,
            &self.parse_options,
            &self.render_options,
            true,
        );
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_ir::VditorIRRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorIRDOM");
        String::from_utf8_lossy(&r.render())
            .replace(crate::editor::CARET, crate::editor::FRONT_END_CARET)
    }

    /// 对应 Go 版 Lute.Md2VditorSVDOM。
    pub fn md2vditor_sv_dom(&self, markdown: &str) -> String {
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_sv::VditorSVRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorSVDOM");
        String::from_utf8_lossy(&r.render())
            .replace(crate::editor::CARET, crate::editor::FRONT_END_CARET)
    }

    /// 对应 Go 版 Lute.HTML2VditorSVDOM。
    pub fn html2vditor_sv_dom(&self, html: &str) -> String {
        let markdown = self.html2markdown(html).0;
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_sv::VditorSVRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("HTML2VditorSVDOM");
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.SpinVditorSVDOM。
    pub fn spin_vditor_sv_dom(&self, markdown: &str) -> String {
        if crate::editor::CARET == markdown.trim() {
            let mut ret = String::from("<span data-type=\"text\"><wbr></span>");
            ret.push_str(&String::from_utf8_lossy(render::vditor_sv::NEWLINE_SV));
            return ret;
        }
        let tree = crate::parse::parse("", markdown.as_bytes(), self.parse_options.clone());
        let mut r = render::vditor_sv::VditorSVRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2VditorSVDOM");
        String::from_utf8_lossy(&r.render())
            .replace(crate::editor::CARET, crate::editor::FRONT_END_CARET)
    }

    /// 对应 Go 版 Lute.HTML2Markdown。
    pub fn html2markdown(&self, dom: &str) -> (String, Option<String>) {
        (
            crate::html2md::html2markdown_ext(
                dom,
                self.parse_options.clone(),
                self.render_options.clone(),
                self.ext("HTML2Md"),
            ),
            None,
        )
    }

    /// 对应 Go 版 Lute.HTML2Md。
    pub fn html2md(&self, dom: &str) -> String {
        self.html2markdown(dom).0
    }

    /// 对应 Go 版 Lute.HTML2Text。
    pub fn html2text(&self, dom: &str) -> String {
        let tree = self.html2tree(dom);
        tree.root_raw().text()
    }

    /// 对应 Go 版 Lute.Space。
    pub fn space(text: &str) -> String {
        render::space::space0(text)
    }

    /// 对应 Go 版 Lute.IsValidLinkDest。
    pub fn is_valid_link_dest(&self, s: &str) -> bool {
        let s = s.trim();
        if s.starts_with('[') {
            return false;
        }
        let mut options = ParseOptions::new();
        options.gfm_auto_link = true;
        let tree = parse::parse("", s.as_bytes(), options);
        let root = tree.root_raw();
        root.first_child().is_some_and(|first| {
            first == root.last_child().unwrap()
                && first
                    .first_child()
                    .is_some_and(|n| n.typ() == NodeType::NodeLink)
        })
    }

    /// 对应 Go 版 Lute.GetLinkDest。
    pub fn get_link_dest(&self, s: &str) -> String {
        self.link_dest(s).unwrap_or_default()
    }

    fn link_dest(&self, s: &str) -> Option<String> {
        let s = s.trim();
        if s.starts_with("file://") {
            return Some(s.to_string());
        }

        let mut options = ParseOptions::new();
        options.gfm_auto_link = true;
        let tree = parse::parse("", s.as_bytes(), options);
        let root = tree.root_raw();
        let first = root.first_child()?;
        let link = first.first_child()?;
        if root.last_child() != root.first_child() {
            return None;
        }
        if NodeType::NodeLink != link.typ() {
            return None;
        }
        let dest = link
            .child_by_type(NodeType::NodeLinkDest)
            .map(|d| d.tokens_str())
            .unwrap_or_default();
        Some(dest)
    }

    // ------------------------------------------------------------ 语法树 API

    /// 对应 Go 版 Lute.HTML2Tree。
    pub fn html2tree(&self, html: &str) -> crate::ast::Tree {
        crate::html2md::html2tree_with_render_options(
            html,
            self.parse_options.clone(),
            self.render_options.clone(),
        )
    }

    /// 对应 Go 版 Lute.BlockDOM2Tree。
    pub fn blockdom2tree(&self, html: &str) -> crate::ast::Tree {
        crate::blockdom::blockdom2tree_with_render_options(
            html,
            self.parse_options.clone(),
            &self.render_options,
        )
    }

    /// 对应 Go 版 Lute.Tree2HTML。
    pub fn tree2html(&self, tree: &crate::ast::Tree) -> String {
        let mut r = render::HtmlRenderer::with_parse_options(
            tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        String::from_utf8_lossy(&r.render()).to_string()
    }

    /// 对应 Go 版 Lute.Tree2BlockDOM。
    pub fn tree2blockdom(&self, tree: &crate::ast::Tree) -> String {
        let out = crate::blockdom::tree2blockdom_ext(
            tree,
            self.render_options.clone(),
            self.parse_options.clone(),
            self.ext("Md2BlockDOM"),
        );
        out.replace(crate::editor::CARET, "<wbr>")
    }

    /// 对应 Go 版 Lute.RenderNodeBlockDOM。
    pub fn render_node_blockdom(&self, node: &crate::ast::Node) -> String {
        let node = node.raw();
        let tree = crate::ast::Tree::new();
        let mut r = render::protyle::ProtyleRenderer::new(
            &tree,
            self.render_options.clone(),
            self.parse_options.clone(),
        );
        r.base.ext_renderer = self.ext("Md2BlockDOM");
        r.base.last_out = b'\n';
        r.base.out = Vec::with_capacity(4096);
        let me = &mut r;
        crate::ast::walk_raw(node, &mut |n, entering| me.render_node(n, entering));
        String::from_utf8_lossy(&r.base.out).to_string()
    }

    /// Merge adjacent spans while the public handle retains their arena.
    pub fn merge_same_span(&self, node: &crate::ast::Node) {
        self.merge_same_span_raw(node.raw());
    }

    /// Merge adjacent text marks while the public handle retains their arena.
    pub fn merge_same_text_mark(&self, node: &crate::ast::Node) {
        self.merge_same_text_mark_raw(node.raw());
    }

    /// 对应 Go 版 Lute.MergeSameSpan。
    pub(crate) fn merge_same_span_raw(&self, n: crate::ast::RawNode) {
        let Some(next) = n.next() else { return };
        if n.typ() != next.typ() {
            return;
        }
        if let Some(nn) = next.next() {
            if NodeType::NodeKramdownSpanIAL == nn.typ() {
                return;
            }
        }
        let (Some(open), Some(close)) = (next.first_child(), next.last_child()) else {
            return;
        };
        open.unlink();
        close.unlink();

        let mut span_children = Vec::new();
        let mut c = next.first_child();
        while let Some(x) = c {
            span_children.push(x);
            c = x.next();
        }
        if let Some(last) = n.last_child() {
            for c in span_children {
                last.insert_before(c);
            }
        }
        next.unlink();
    }

    /// 对应 Go 版 Lute.MergeSameTextMark。
    pub(crate) fn merge_same_text_mark_raw(&self, n: crate::ast::RawNode) {
        use crate::editor;

        let Some(prev) = n.previous() else { return };
        let mut merge_with_ial = false;
        let mut merge_with_zwsp = false;
        if NodeType::NodeKramdownSpanIAL == prev.typ() {
            let (Some(next), Some(prev_prev)) = (n.next(), prev.previous()) else {
                return;
            };
            if NodeType::NodeKramdownSpanIAL != next.typ() {
                return;
            }
            if prev.tokens() != next.tokens() {
                return;
            }
            if !n.is_same_text_mark_type(prev_prev) {
                return;
            }
            merge_with_ial = true;
        } else {
            let preview_text = prev.tokens_str();
            let stripped = preview_text
                .replace(editor::ZWSP, "")
                .replace(editor::CARET, "");
            if NodeType::NodeText == prev.typ()
                && !preview_text.contains('　')
                && !preview_text.contains(' ')
                && !preview_text.contains('\n')
                && !preview_text.contains('\t')
                && stripped.trim().is_empty()
                && prev.previous().is_some()
                && n.is_same_text_mark_type(prev.previous().unwrap())
            {
                if !n.data().text_mark_type.contains("tag") || preview_text.contains(editor::CARET)
                {
                    merge_with_zwsp = true;
                }
            } else if n.typ() != prev.typ() || !n.is_same_text_mark_type(prev) {
                return;
            }
        }

        let mut rest: Vec<&str> = n.data().text_mark_type.split(' ').collect();
        rest.retain(|t| {
            !matches!(
                *t,
                "code"
                    | "em"
                    | "strong"
                    | "s"
                    | "mark"
                    | "u"
                    | "sub"
                    | "sup"
                    | "kbd"
                    | "text"
                    | "tag"
                    | "block-ref"
                    | "a"
                    | "inline-memo"
            )
        });
        if !rest.is_empty() {
            return;
        }

        let content = n.data().text_mark_text_content.clone();
        if merge_with_ial || merge_with_zwsp {
            let pp = prev.previous().unwrap();
            let mut merged = pp.data().text_mark_text_content.clone();
            if prev.tokens_str().contains(editor::CARET) {
                merged.push_str(editor::CARET);
            }
            merged.push_str(&content);
            n.data_mut().text_mark_text_content = merged;
            pp.unlink();
        } else {
            n.data_mut().text_mark_text_content =
                prev.data().text_mark_text_content.clone() + &content;
        }
        prev.unlink();
        n.sort_text_mark_data_types();
    }

    /// 对应 Go 版 render.HeadingID。
    pub fn heading_id(node: &crate::ast::Node) -> String {
        render::heading_id(node.raw())
    }

    /// 对应 Go 版 render.HeadingIDRaw。
    pub fn heading_id_raw(node: &crate::ast::Node) -> String {
        render::heading_id_raw(node.raw())
    }

    /// 对应 JS 版 SetEmojis（直接替换 AliasEmoji）。
    pub fn set_emojis(&mut self, map: HashMap<String, String>) {
        self.parse_options.alias_emoji = map;
    }

    /// 对应 Go 版 Lute.PutEmojis。
    pub fn put_emojis(&mut self, map: HashMap<String, String>) {
        for (k, v) in map {
            self.parse_options.alias_emoji.insert(k.clone(), v.clone());
            self.parse_options.emoji_alias.insert(v, k);
        }
    }

    /// 对应 Go 版 Lute.GetEmojis。
    pub fn get_emojis(&self) -> HashMap<String, String> {
        let mut ret = HashMap::with_capacity(self.parse_options.alias_emoji.len());
        for (k, v) in &self.parse_options.alias_emoji {
            ret.insert(
                k.clone(),
                v.replace("${emojiSite}", &self.parse_options.emoji_site),
            );
        }
        ret
    }

    /// 对应 Go 版 Lute.RemoveEmoji。
    pub fn remove_emoji(&self, s: &str) -> String {
        let mut ret = s.to_string();
        for u in self.parse_options.emoji_alias.keys() {
            ret = ret.replace(u, "");
        }
        ret.trim().to_string()
    }

    /// 对应 JS 版 SetTerms（直接替换 Terms）。
    pub fn set_terms(&mut self, map: HashMap<String, String>) {
        self.render_options.terms = map;
    }

    /// 对应 Go 版 Lute.PutTerms。
    pub fn put_terms(&mut self, map: HashMap<String, String>) {
        for (k, v) in map {
            self.render_options.terms.insert(k, v);
        }
    }

    /// 对应 Go 版 Lute.GetTerms。
    pub fn get_terms(&self) -> HashMap<String, String> {
        self.render_options.terms.clone()
    }
}

/// 复现 Go 版 ProtyleRenderer.renderLinkRefDef 的空指针崩溃。
///
/// 基准实现里 `node.FirstChild.ChildByType(NodeLinkDest)` 没有判空，
/// 任何链接引用定义都会触发 panic；这里显式返回同样的错误文本，
/// 由 wasm 绑定层抛给 JS。
/// 在脚注定义的首个子节点前插入 `<span id="footnotes-def-N"></span>`。
///
/// 对应 Go 版 ProtyleRenderer.renderFootnotesDef：li 上带 id 会让 Pandoc 转 Docx 出问题，
/// 所以锚点放在块内容里。必须在渲染前完成，节点归 Tree 所有，避免返回给 JS 的树悬空。
fn prepend_footnote_anchors(tree: &mut crate::ast::Tree) {
    let mut targets: Vec<(crate::ast::RawNode, String)> = Vec::new();
    crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering && NodeType::NodeFootnotesDef == n.typ() {
            if let (Some(first), Some(first_ref)) =
                (n.first_child(), n.data().footnotes_refs.first().copied())
            {
                targets.push((first, first_ref.data().footnotes_ref_id.clone()));
            }
        }
        crate::ast::WalkStatus::WalkContinue
    });
    for (first, ref_id) in targets {
        let anchor = tree.new_node_raw(NodeType::NodeInlineHTML);
        anchor.set_tokens(format!("<span id=\"footnotes-def-{}\"></span>", ref_id));
        first.prepend_child(anchor);
    }
}

fn protyle_nil_pointer_error(tree: &crate::ast::Tree) -> Option<String> {
    let mut broken = false;
    crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering && NodeType::NodeLinkRefDef == n.typ() && n.first_child().is_some() {
            broken = true;
        }
        crate::ast::WalkStatus::WalkContinue
    });
    if broken {
        Some("runtime error: invalid memory address or nil pointer dereference".to_string())
    } else {
        None
    }
}
