// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Vditor WYSIWYG DOM 渲染器，对齐 Go 版 render.VditorRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::editor;
use crate::html as html_util;
use crate::options::{ParseOptions, RenderOptions};

use super::{BaseRenderer, Renderer};

/// Vditor 所见即所得 DOM 渲染器。
pub struct VditorRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
    pub comment_stack_depth: i32,
}

impl VditorRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
            comment_stack_depth: 0,
        }
    }

    fn tag(&mut self, name: &str, attrs: &[(&str, &str)], selfclosing: bool) {
        let owned: Vec<(String, String)> = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.tag_owned(name, &owned, selfclosing);
    }

    fn tag_owned(&mut self, name: &str, attrs: &[(String, String)], selfclosing: bool) {
        self.base.tag(name, attrs, selfclosing);
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some((s, st)) = self.base.ext_render(n, entering) {
            self.base.write_str(&s);
            return st;
        }
        match n.typ() {
            NodeType::NodeDocument => WalkStatus::WalkContinue,
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => self.render_text(n, entering),
            NodeType::NodeCodeSpan => self.render_code_span(n, entering),
            NodeType::NodeCodeSpanOpenMarker => WalkStatus::WalkContinue,
            NodeType::NodeCodeSpanContent => self.render_code_span_content(n, entering),
            NodeType::NodeCodeSpanCloseMarker => self.render_code_span_close_marker(n, entering),
            NodeType::NodeCodeBlock => self.render_code_block(n, entering),
            NodeType::NodeCodeBlockFenceOpenMarker
            | NodeType::NodeCodeBlockFenceInfoMarker
            | NodeType::NodeCodeBlockFenceCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeCodeBlockCode => self.render_code_block_code(n, entering),
            NodeType::NodeMathBlock => self.render_math_block(n, entering),
            NodeType::NodeMathBlockOpenMarker => WalkStatus::WalkContinue,
            NodeType::NodeMathBlockContent => self.render_math_block_content(n, entering),
            NodeType::NodeMathBlockCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeInlineMath => self.render_inline_math(n, entering),
            NodeType::NodeInlineMathOpenMarker => WalkStatus::WalkContinue,
            NodeType::NodeInlineMathContent => self.render_inline_math_content(n, entering),
            NodeType::NodeInlineMathCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeEmphasis => WalkStatus::WalkContinue,
            NodeType::NodeEmA6kOpenMarker => {
                if entering {
                    self.tag("em", &[("data-marker", "*")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmA6kCloseMarker => {
                if entering {
                    self.tag("/em", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmU8eOpenMarker => {
                if entering {
                    self.tag("em", &[("data-marker", "_")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmU8eCloseMarker => {
                if entering {
                    self.tag("/em", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrong => WalkStatus::WalkContinue,
            NodeType::NodeStrongA6kOpenMarker => {
                if entering {
                    self.tag("strong", &[("data-marker", "**")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kCloseMarker => {
                if entering {
                    self.tag("/strong", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongU8eOpenMarker => {
                if entering {
                    self.tag("strong", &[("data-marker", "__")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongU8eCloseMarker => {
                if entering {
                    self.tag("/strong", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote => self.render_blockquote(n, entering),
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker | NodeType::NodeHeadingID => WalkStatus::WalkContinue,
            NodeType::NodeList => self.render_list(n, entering),
            NodeType::NodeListItem => self.render_list_item(n, entering),
            NodeType::NodeMindmap => self.render_list(n, entering),
            NodeType::NodeMindmapItem => self.render_list_item(n, entering),
            NodeType::NodeThematicBreak => self.render_thematic_break(n, entering),
            NodeType::NodeHardBreak => {
                if entering {
                    self.tag("br", &[], true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSoftBreak => {
                if entering {
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLBlock => self.render_html(n, entering),
            NodeType::NodeInlineHTML => self.render_inline_html(n, entering),
            NodeType::NodeLink => self.render_link(n, entering),
            NodeType::NodeImage => self.render_image(n, entering),
            NodeType::NodeBang
            | NodeType::NodeOpenBracket
            | NodeType::NodeCloseBracket
            | NodeType::NodeOpenParen
            | NodeType::NodeCloseParen
            | NodeType::NodeOpenBrace
            | NodeType::NodeCloseBrace
            | NodeType::NodeLinkSpace
            | NodeType::NodeLinkDest
            | NodeType::NodeLinkTitle => WalkStatus::WalkContinue,
            NodeType::NodeLinkText => {
                if entering {
                    self.base.write(&n.tokens_vec());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough => WalkStatus::WalkContinue,
            NodeType::NodeStrikethrough1OpenMarker => {
                if entering {
                    self.tag("s", &[("data-marker", "~")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough1CloseMarker => {
                if entering {
                    self.tag("/s", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough2OpenMarker => {
                if entering {
                    self.tag("s", &[("data-marker", "~~")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough2CloseMarker => {
                if entering {
                    self.tag("/s", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTaskListItemMarker => self.render_task_list_item_marker(n, entering),
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead => self.render_table_head(n, entering),
            NodeType::NodeTableRow => self.render_table_row(n, entering),
            NodeType::NodeTableCell => self.render_table_cell(n, entering),
            NodeType::NodeEmoji => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                if entering {
                    self.base.write(&n.tokens_vec());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmojiAlias => WalkStatus::WalkContinue,
            NodeType::NodeFootnotesDefBlock => {
                if entering {
                    self.base
                        .write_str("<div data-block=\"0\" data-type=\"footnotes-block\">");
                    self.base.write_str("<ol data-type=\"footnotes-defs-ol\">");
                } else {
                    self.base.write_str("</ol></div>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDef => self.render_footnotes_def(n, entering),
            NodeType::NodeFootnotesRef => self.render_footnotes_ref(n, entering),
            NodeType::NodeToC => self.render_toc(n, entering),
            NodeType::NodeBackslash => {
                if entering {
                    self.base.write_str("<span data-type=\"backslash\">");
                    self.base.write_str("<span>");
                    self.base.write_byte(b'\\');
                    self.base.write_str("</span>");
                } else {
                    self.base.write_str("</span>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBackslashContent => {
                if entering {
                    let escaped = html_util::escape_html(&n.tokens_str());
                    self.base.write(escaped.as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLEntity => self.render_html_entity(n, entering),
            NodeType::NodeYamlFrontMatter => self.render_yaml_front_matter(n, entering),
            NodeType::NodeYamlFrontMatterOpenMarker | NodeType::NodeYamlFrontMatterCloseMarker => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                self.render_yaml_front_matter_content(n, entering)
            }
            NodeType::NodeMark => self.render_mark(n, entering),
            NodeType::NodeMark1OpenMarker => {
                if entering {
                    self.tag("mark", &[("data-marker", "=")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark1CloseMarker => {
                if entering {
                    self.tag("/mark", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark2OpenMarker => {
                if entering {
                    self.tag("mark", &[("data-marker", "==")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark2CloseMarker => {
                if entering {
                    self.tag("/mark", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSup => WalkStatus::WalkContinue,
            NodeType::NodeSupOpenMarker => {
                if entering {
                    self.tag("sup", &[("data-marker", "^")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSupCloseMarker => {
                if entering {
                    self.tag("/sup", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSub => WalkStatus::WalkContinue,
            NodeType::NodeSubOpenMarker => {
                if entering {
                    self.tag("sub", &[("data-marker", "~")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSubCloseMarker => {
                if entering {
                    self.tag("/sub", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownBlockIAL | NodeType::NodeKramdownSpanIAL => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkRefDefBlock => {
                if entering {
                    self.base
                        .write_str("<div data-block=\"0\" data-type=\"link-ref-defs-block\">");
                } else {
                    self.base.write_str("</div>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkRefDef => self.render_link_ref_def(n, entering),
            _ => WalkStatus::WalkContinue,
        }
    }

    fn render_link_ref_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if let Some(first) = n.first_child() {
                if let Some(dest) = first.child_by_type(NodeType::NodeLinkDest) {
                    let dest_str = dest.tokens_str();
                    let mut s = String::from("[");
                    s.push_str(&n.tokens_str());
                    s.push_str("]:");
                    if editor::CARET != dest_str {
                        s.push(' ');
                    }
                    s.push_str(&dest_str);
                    s.push('\n');
                    self.base.write_str(&s);
                }
            }
            return WalkStatus::WalkSkipChildren;
        }
        WalkStatus::WalkContinue
    }

    fn render_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut previous_node_text = previous_node_text(n).unwrap_or_default();
            previous_node_text = previous_node_text.replace(editor::CARET, "");
            if previous_node_text.is_empty() {
                self.base.write_str(editor::ZWSP);
            }
        } else {
            self.base.write_str(editor::ZWSP);
        }
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.write_str("<div class=\"vditor-wysiwyg__block\" data-type=\"yaml-front-matter\" data-block=\"0\">");
        } else {
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let tokens = n.tokens_vec();
            let preview_tokens = trim_ascii_space(&tokens).to_vec();
            let code_len = preview_tokens.len();
            let code_is_empty = code_len < 1
                || (editor::CARET.len() == code_len && editor::CARET.as_bytes() == &tokens[..]);
            self.tag("pre", &[], false);
            self.tag("code", &[("data-type", "yaml-front-matter")], false);
            if code_is_empty {
                self.base.write_str(editor::FRONT_END_CARET);
                self.base.write_str("\n");
            } else {
                let escaped = html_util::escape_html(&String::from_utf8_lossy(&preview_tokens));
                self.base.write(escaped.as_bytes());
            }
            self.base.write_str("</code></pre>");
        }
        WalkStatus::WalkContinue
    }

    fn render_html_entity(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }

        let mut previous_node_text = previous_node_text(n).unwrap_or_default();
        previous_node_text = previous_node_text.replace(editor::CARET, "");
        if previous_node_text.is_empty() {
            self.base.write_str(editor::ZWSP);
        }

        self.base
            .write_str("<span class=\"vditor-wysiwyg__block\" data-type=\"html-entity\">");
        self.tag(
            "code",
            &[("data-type", "html-entity"), ("style", "display: none")],
            false,
        );
        let mut tokens = Vec::new();
        tokens.extend_from_slice(editor::ZWSP.as_bytes());
        tokens.extend_from_slice(&n.data().html_entity_tokens);
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        self.base.write_str("</code>");

        self.tag(
            "span",
            &[("class", "vditor-wysiwyg__preview"), ("data-render", "2")],
            false,
        );
        self.tag("code", &[], false);
        let preview_tokens =
            replace_all(&n.data().html_entity_tokens, editor::CARET.as_bytes(), b"");
        self.base.write(&preview_tokens);
        self.tag("/code", &[], false);
        self.tag("/span", &[], false);
        self.base.write_str("</span>");
        self.base.write_str(editor::ZWSP);
        WalkStatus::WalkContinue
    }

    fn render_toc(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            let headings = self.headings();
            self.base.write_str("<div class=\"vditor-toc\" data-block=\"0\" data-type=\"toc-block\" contenteditable=\"false\">");
            if !headings.is_empty() {
                self.base.write_str("<ul>");
                for child in &headings {
                    self.render_toc0(child);
                }
                self.base.write_str("</ul>");
            } else {
                self.base.write_str("[toc]<br>");
            }
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_toc0(&mut self, heading: &TocHeading) {
        self.base.write_str("<li>");
        self.tag("span", &[("data-target-id", &heading.id)], false);
        self.base.write_str(&heading.content);
        self.tag("/span", &[], false);
        if !heading.children.is_empty() {
            self.base.write_str("<ul>");
            for child in &heading.children {
                self.render_toc0(child);
            }
            self.base.write_str("</ul>");
        }
        self.base.write_str("</li>");
    }

    fn headings(&self) -> Vec<TocHeading> {
        let mut ret: Vec<TocHeading> = Vec::new();
        let mut levels: Vec<i32> = Vec::new();
        let mut path: Vec<usize> = Vec::new();
        for child in self.root.children() {
            if NodeType::NodeHeading != child.typ() {
                continue;
            }
            let mut id = super::heading_id(child);
            if self.base.options.vditor_wysiwyg {
                id = format!("wysiwyg-{}", id);
            } else if self.base.options.vditor_ir {
                id = format!("ir-{}", id);
            }
            if crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial {
                for (k, v) in &child.data().kramdown_ial {
                    if "id" == k {
                        id = v.clone();
                        break;
                    }
                }
            }
            let level = child.data().heading_level;
            while let Some(&lvl) = levels.last() {
                if lvl < level {
                    break;
                }
                levels.pop();
                path.pop();
            }
            let h = TocHeading {
                id,
                content: heading_text(child),
                children: Vec::new(),
            };
            let container = toc_container(&mut ret, &path);
            container.push(h);
            let idx = container.len() - 1;
            path.push(idx);
            levels.push(level);
        }
        ret
    }

    fn render_footnotes_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if self.base.rendering_footnotes {
            return WalkStatus::WalkContinue;
        }
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if let Some(prev) = n.previous() {
            if super::eq_fold(prev.tokens(), n.tokens()) {
                return WalkStatus::WalkContinue;
            }
        }

        let mut s = String::from("<li data-type=\"footnotes-li\" data-marker=\"");
        s.push_str(&n.tokens_str());
        s.push_str("\">");
        self.base.write_str(&s);
        let mut c = n.first_child();
        while let Some(child) = c {
            let me = &mut *self;
            crate::ast::walk_raw(child, &mut |nn, en| me.render_node(nn, en));
            c = child.next();
        }
        self.base.write_str("</li>");
        WalkStatus::WalkSkipChildren
    }

    fn render_footnotes_ref(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let mut previous_node_text = previous_node_text(n).unwrap_or_default();
        previous_node_text = previous_node_text.replace(editor::CARET, "");
        if previous_node_text.is_empty() {
            self.base.write_str(editor::ZWSP);
        }
        let Some((idx, def)) = self.find_footnotes_def(&n.tokens_vec()) else {
            return WalkStatus::WalkContinue;
        };
        let label = def.text();
        let escaped_label = html_util::escape_string(&label);
        let aria = sub_str(&escaped_label, 24);
        let footnote_label = String::from_utf8_lossy(&n.data().footnotes_ref_label).to_string();
        self.tag(
            "sup",
            &[
                ("data-type", "footnotes-ref"),
                ("data-footnotes-label", &footnote_label),
                ("class", "vditor-tooltipped vditor-tooltipped__s"),
                ("aria-label", &aria),
            ],
            false,
        );
        self.base.write_str(&idx.to_string());
        self.base.write_str("</sup>");
        self.base.write_str(editor::ZWSP);
        WalkStatus::WalkContinue
    }

    fn find_footnotes_def(&self, label: &[u8]) -> Option<(usize, Node)> {
        let mut idx = 0usize;
        let mut ret = None;
        crate::ast::walk_raw(self.root, &mut |n, entering| {
            if entering && NodeType::NodeFootnotesDef == n.typ() {
                idx += 1;
                if ret.is_none() && super::eq_fold(n.tokens(), label) {
                    ret = Some((idx, n));
                }
            }
            WalkStatus::WalkContinue
        });
        ret
    }

    fn render_inline_math(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut previous_node_text = previous_node_text(n).unwrap_or_default();
            previous_node_text = previous_node_text.replace(editor::CARET, "");
            if previous_node_text.is_empty() {
                self.base.write_str(editor::ZWSP);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_math_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = replace_all(&n.tokens_vec(), editor::ZWSP.as_bytes(), b"");
        let mut preview_tokens = tokens.clone();
        let mut code_attrs: Vec<(String, String)> =
            vec![("data-type".into(), "math-inline".into())];
        if !contains(&preview_tokens, editor::CARET.as_bytes()) {
            code_attrs.push(("style".into(), "display: none".into()));
        }
        self.base
            .write_str("<span class=\"vditor-wysiwyg__block\" data-type=\"math-inline\">");
        self.tag_owned("code", &code_attrs, false);
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        let mut out = Vec::new();
        out.extend_from_slice(editor::ZWSP.as_bytes());
        out.extend_from_slice(escaped.as_bytes());
        self.base.write(&out);
        self.base.write_str("</code>");

        self.tag(
            "span",
            &[("class", "vditor-wysiwyg__preview"), ("data-render", "2")],
            false,
        );
        self.tag("span", &[("class", "language-math")], false);
        preview_tokens = replace_all(&preview_tokens, editor::CARET.as_bytes(), b"");
        if n.parent_is(&[NodeType::NodeTableCell]) {
            preview_tokens = replace_all(&preview_tokens, b"\\|", b"|");
        }
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&preview_tokens));
        self.base.write(escaped.as_bytes());
        self.tag("/span", &[], false);
        self.tag("/span", &[], false);
        self.base.write_str("</span>");
        self.base.write_str(editor::ZWSP);
        WalkStatus::WalkContinue
    }

    fn render_math_block(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.write_str(
                "<div class=\"vditor-wysiwyg__block\" data-type=\"math-block\" data-block=\"0\">",
            );
        } else {
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_math_block_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = n.tokens_vec();
        let preview_tokens = trim_ascii_space(&tokens).to_vec();
        let mut pre_attrs: Vec<(String, String)> = Vec::new();
        if !contains(&preview_tokens, editor::CARET.as_bytes())
            && self.base.options.vditor_math_block_preview
        {
            pre_attrs.push(("style".into(), "display: none".into()));
        }
        let code_len = preview_tokens.len();
        let code_is_empty = code_len < 1
            || (editor::CARET.len() == code_len && editor::CARET.as_bytes() == &tokens[..]);
        self.tag_owned("pre", &pre_attrs, false);
        self.tag("code", &[("data-type", "math-block")], false);
        if code_is_empty {
            self.base.write_str(editor::FRONT_END_CARET);
            self.base.write_str("\n");
        } else {
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&preview_tokens));
            self.base.write(escaped.as_bytes());
        }
        self.base.write_str("</code></pre>");

        if self.base.options.vditor_math_block_preview {
            self.tag(
                "pre",
                &[("class", "vditor-wysiwyg__preview"), ("data-render", "2")],
                false,
            );
            self.tag(
                "div",
                &[("data-type", "math-block"), ("class", "language-math")],
                false,
            );
            let tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
            self.base.write_str("</div></pre>");
        }
        WalkStatus::WalkContinue
    }

    fn render_table_cell(&mut self, n: Node, entering: bool) -> WalkStatus {
        let tag = if n.parent().and_then(|p| p.parent()).map(|p| p.typ())
            == Some(NodeType::NodeTableHead)
        {
            "th"
        } else {
            "td"
        };
        if entering {
            let mut attrs: Vec<(String, String)> = Vec::new();
            match n.data().table_cell_align {
                1 => attrs.push(("align".into(), "left".into())),
                2 => attrs.push(("align".into(), "center".into())),
                3 => attrs.push(("align".into(), "right".into())),
                _ => {}
            }
            self.tag_owned(tag, &attrs, false);
            if n.first_child().is_none() {
                // 空单元格补一个空格，保证编辑器里可以落光标。
                self.base.write_str(" ");
            } else if let Some(first) = n.first_child() {
                if first.tokens() == editor::CARET.as_bytes() {
                    first.set_tokens(format!("{} ", editor::CARET));
                }
            }
        } else {
            self.tag(&format!("/{}", tag), &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_table_row(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("tr", &[], false);
        } else {
            self.tag("/tr", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_table_head(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("thead", &[], false);
        } else {
            self.tag("/thead", &[], false);
            if n.next().is_some() {
                self.tag("tbody", &[], false);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("table", &[("data-block", "0")], false);
        } else {
            if n.first_child().and_then(|c| c.next()).is_some() {
                self.tag("/tbody", &[], false);
            }
            self.tag("/table", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_image(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if 3 == n.data().link_type {
                let mut previous_node_text = previous_node_text(n).unwrap_or_default();
                previous_node_text = previous_node_text.replace(editor::CARET, "");
                if previous_node_text.is_empty() {
                    self.base.write_str(editor::ZWSP);
                }
                self.base.write_str("<img src=\"");
                let link = self.find_link_ref_def_link(&n.data().link_ref_label);
                let dest_tokens = link
                    .and_then(|l| l.child_by_type(NodeType::NodeLinkDest))
                    .map(|d| d.tokens_vec())
                    .unwrap_or_default();
                let dest_tokens = self.base.link_path(&dest_tokens);
                let dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
                self.base.write(&dest_tokens);
                self.base.write_str("\" alt=\"");
                if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                    let alt_tokens = replace_all(&alt.tokens_vec(), editor::CARET.as_bytes(), b"");
                    self.base.write(&alt_tokens);
                }
                self.base.write_byte(b'"');
                if let Some(title) = link.and_then(|l| l.child_by_type(NodeType::NodeLinkTitle)) {
                    if !title.tokens().is_empty() {
                        self.base.write_str(" title=\"");
                        let tokens =
                            replace_all(&title.tokens_vec(), editor::CARET.as_bytes(), b"");
                        self.base.write(&tokens);
                        self.base.write_byte(b'"');
                    }
                }
                let label = String::from_utf8_lossy(&n.data().link_ref_label).to_string();
                self.base
                    .write_str(" data-type=\"link-ref\" data-link-label=\"");
                self.base.write_str(&label);
                self.base.write_str("\"");
                self.base.write_str(" />");

                self.sanitize_last_img();
                return WalkStatus::WalkSkipChildren;
            }

            if 0 == self.base.disable_tags {
                self.base.write_str("<img src=\"");
                let dest_tokens = n
                    .child_by_type(NodeType::NodeLinkDest)
                    .map(|d| d.tokens_vec())
                    .unwrap_or_default();
                let dest_tokens = self.base.link_path(&dest_tokens);
                let dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
                self.base.write(&dest_tokens);
                self.base.write_str("\" alt=\"");
                if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                    if contains(&alt.tokens_vec(), editor::CARET.as_bytes()) {
                        let tokens = replace_all(&alt.tokens_vec(), editor::CARET.as_bytes(), b"");
                        alt.set_tokens(tokens);
                    }
                }
            }
            self.base.disable_tags += 1;
            return WalkStatus::WalkContinue;
        }

        self.base.disable_tags -= 1;
        if 0 == self.base.disable_tags {
            self.base.write_byte(b'"');
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.tokens().is_empty() {
                    self.base.write_str(" title=\"");
                    let tokens = replace_all(&title.tokens_vec(), editor::CARET.as_bytes(), b"");
                    self.base.write(&tokens);
                    self.base.write_byte(b'"');
                }
            }
            self.base.write_str(" />");
            self.sanitize_last_img();
        }
        WalkStatus::WalkContinue
    }

    fn sanitize_last_img(&mut self) {
        let idx = match find_last(&self.base.out, b"<img src=") {
            Some(i) => i,
            None => return,
        };
        let img_buf = self.base.out[idx..].to_vec();
        let img_buf = if self.base.options.sanitize {
            super::sanitizer::sanitize(&img_buf)
        } else {
            img_buf
        };
        self.base.out.truncate(idx);
        self.base.write(&img_buf);
    }

    fn render_link(&mut self, n: Node, entering: bool) -> WalkStatus {
        if 3 == n.data().link_type {
            if entering {
                let mut previous_node_text = previous_node_text(n).unwrap_or_default();
                previous_node_text = previous_node_text.replace(editor::CARET, "");
                if previous_node_text.is_empty() {
                    self.base.write_str(editor::ZWSP);
                }
                let link_text = n.children_by_type(NodeType::NodeLinkText);
                let text = link_text
                    .first()
                    .map(|t| t.tokens_str())
                    .unwrap_or_default();
                let label = String::from_utf8_lossy(&n.data().link_ref_label).to_string();
                self.tag(
                    "span",
                    &[("data-type", "link-ref"), ("data-link-label", &label)],
                    false,
                );
                self.base.write_str(&text);
                self.tag("/span", &[], false);
                self.base.write_str(editor::ZWSP);
                return WalkStatus::WalkSkipChildren;
            }
            return WalkStatus::WalkContinue;
        }

        if entering {
            let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) else {
                return WalkStatus::WalkContinue;
            };
            let mut dest_tokens = dest.tokens_vec();
            if self.base.options.sanitize {
                let tokens = trim_ascii_space(&dest_tokens).to_vec();
                let lower = String::from_utf8_lossy(&tokens).to_lowercase();
                if lower.starts_with("javascript:") {
                    dest_tokens.clear();
                }
            }
            dest_tokens = self.base.link_path(&dest_tokens);
            let caret_in_dest = contains(&dest_tokens, editor::CARET.as_bytes());
            if caret_in_dest {
                if let Some(text) = n.child_by_type(NodeType::NodeLinkText) {
                    let mut tokens = text.tokens_vec();
                    tokens.extend_from_slice(editor::CARET.as_bytes());
                    text.set_tokens(tokens);
                }
                dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
            }
            let mut attrs: Vec<(String, String)> = vec![(
                "href".into(),
                String::from_utf8_lossy(&dest_tokens).to_string(),
            )];
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.tokens().is_empty() {
                    let tokens = replace_all(&title.tokens_vec(), editor::CARET.as_bytes(), b"");
                    attrs.push(("title".into(), String::from_utf8_lossy(&tokens).to_string()));
                }
            }
            self.tag_owned("a", &attrs, false);
        } else {
            self.tag("/a", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn find_link_ref_def_link(&self, label: &[u8]) -> Option<Node> {
        let mut ret = None;
        crate::ast::walk_raw(self.root, &mut |n, entering| {
            if entering && NodeType::NodeLinkRefDef == n.typ() && super::eq_fold(n.tokens(), label)
            {
                ret = n.first_child();
                return WalkStatus::WalkStop;
            }
            WalkStatus::WalkContinue
        });
        ret
    }

    fn render_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        self.base.write_str(
            "<div class=\"vditor-wysiwyg__block\" data-type=\"html-block\" data-block=\"0\">",
        );
        let tokens = trim_ascii_space(&n.tokens_vec()).to_vec();
        self.base.write_str("<pre>");
        self.tag("code", &[], false);
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        self.base.write_str("</code></pre>");

        self.tag(
            "pre",
            &[("class", "vditor-wysiwyg__preview"), ("data-render", "2")],
            false,
        );
        let mut tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
        if self.base.options.sanitize {
            tokens = super::sanitizer::sanitize(&tokens);
        }
        tokens = self.tag_src_path(&tokens);
        self.base.write(&tokens);
        self.base.write_str("</pre></div>");
        WalkStatus::WalkContinue
    }

    fn tag_src_path(&self, tokens: &[u8]) -> Vec<u8> {
        if let Some(src_index) = find(tokens, b"src=\"") {
            if src_index > 0 {
                let src = &tokens[src_index + 5..];
                let without_caret = replace_all(src, editor::CARET.as_bytes(), b"");
                if without_caret.is_empty() {
                    return tokens.to_vec();
                }
                let target_src = self.base.link_path(src);
                let mut origin_src = String::from_utf8_lossy(&target_src).to_string();
                if origin_src.starts_with("//") {
                    origin_src = format!("https:{}", origin_src);
                }
                return replace_all(tokens, src, origin_src.as_bytes());
            }
        }
        tokens.to_vec()
    }

    fn render_inline_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }

        let tokens = n.tokens_vec();
        if tokens == b"<br />" && n.parent_is(&[NodeType::NodeTableCell]) {
            self.base.write(&tokens);
            return WalkStatus::WalkContinue;
        }

        if contains(&tokens, b"<span class=\"vditor-comment") {
            self.comment_stack_depth += 1;
            self.base.write(&tokens);
            return WalkStatus::WalkContinue;
        }

        if tokens == b"</span>" && 0 < self.comment_stack_depth {
            self.comment_stack_depth -= 1;
            self.base.write(&tokens);
            return WalkStatus::WalkContinue;
        }

        let mut previous_node_text = previous_node_text(n).unwrap_or_default();
        previous_node_text = previous_node_text.replace(editor::CARET, "");
        if editor::ZWSP == previous_node_text || previous_node_text.is_empty() {
            self.base.write_str(editor::ZWSP);
        }

        let mut tokens = replace_all(&tokens, editor::ZWSP.as_bytes(), b"");
        let mut out = Vec::new();
        out.extend_from_slice(editor::ZWSP.as_bytes());
        out.extend_from_slice(&tokens);
        tokens = out;

        self.tag("code", &[("data-type", "html-inline")], false);
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        self.base.write_str("</code>");
        WalkStatus::WalkContinue
    }

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some(grandparent) = n.parent().and_then(|p| p.parent()) {
            if NodeType::NodeList == grandparent.typ()
                && grandparent
                    .data()
                    .list_data
                    .as_ref()
                    .map(|d| d.tight)
                    .unwrap_or(false)
            {
                return WalkStatus::WalkContinue;
            }
        }

        if entering {
            let mut attrs: Vec<(String, String)> = vec![("data-block".into(), "0".into())];
            attrs.extend(n.data().kramdown_ial.iter().cloned());
            self.tag_owned("p", &attrs, false);
        } else {
            self.tag("/p", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let mut tokens = n.tokens_vec();
        if self.base.options.fix_term_typo {
            self.base.fix_term_typo(&mut tokens);
        }
        while tokens.last() == Some(&b'\n') {
            tokens.pop();
        }
        let caret_zwsp = format!("{}{}", editor::CARET, editor::ZWSP);
        if !super::eq_fold(&tokens, caret_zwsp.as_bytes()) {
            tokens = replace_all(&tokens, editor::ZWSP.as_bytes(), b"");
        }
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        WalkStatus::WalkContinue
    }

    fn render_code_span(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut previous_node_text = previous_node_text(n).unwrap_or_default();
            previous_node_text = previous_node_text.replace(editor::CARET, "");
            if previous_node_text.is_empty() {
                self.base.write_str(editor::ZWSP);
            } else if let Some(last) = previous_node_text.chars().next_back() {
                if super::is_letter_or_digit(last) {
                    self.base.write_byte(b' ');
                }
            }
            let marker = "\u{0060}".repeat(n.data().code_marker_len.max(1) as usize);
            self.tag("code", &[("data-marker", &marker)], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let tokens = replace_all(&n.tokens_vec(), editor::ZWSP.as_bytes(), b"");
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            let mut out = Vec::new();
            out.extend_from_slice(editor::ZWSP.as_bytes());
            out.extend_from_slice(escaped.as_bytes());
            self.base.write(&out);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_close_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.write_str("</code>");
            let code_span = n.parent();
            if let Some(parent) = code_span.and_then(|p| p.parent()) {
                if NodeType::NodeLink == parent.typ() {
                    return WalkStatus::WalkContinue;
                }
            }
            self.base.write_str(editor::ZWSP);
        }
        WalkStatus::WalkContinue
    }

    fn render_blockquote(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.write_str("<blockquote data-block=\"0\">");
        } else {
            self.base.write_str("</blockquote>");
        }
        WalkStatus::WalkContinue
    }

    fn render_callout(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let subtype = html_util::escape_html(&n.data().callout_type);
            self.tag(
                "blockquote",
                &[
                    ("data-block", "0"),
                    ("data-type", "callout"),
                    ("data-subtype", &subtype),
                    ("class", "callout"),
                ],
                false,
            );
            self.tag(
                "p",
                &[("data-block", "0"), ("class", "callout-info")],
                false,
            );
            self.tag(
                "span",
                &[("class", "vditor-wysiwyg__callout-marker")],
                false,
            );
            let mut s = String::from("[!");
            s.push_str(&subtype);
            s.push_str("] ");
            self.base.write_str(&s);
            self.tag("/span", &[], false);
            if !n.data().callout_icon.is_empty() {
                if 1 == n.data().callout_icon_type {
                    let icon = html_util::escape_html(&n.data().callout_icon);
                    self.tag(
                        "img",
                        &[
                            ("src", &icon),
                            ("alt", "callout-icon"),
                            ("class", "callout-img"),
                        ],
                        true,
                    );
                } else {
                    let icon = html_util::escape_html(&n.data().callout_icon);
                    self.base.write_str(&icon);
                }
                self.base.write_byte(b' ');
            }
            let title = callout_display_title(n);
            let tree = crate::parse::inline("", title.as_bytes(), self.parse_options.clone());
            let root = tree.root_raw();
            if let Some(paragraph) = root.first_child() {
                let inlines: Vec<_> = paragraph.children().collect();
                paragraph.unlink();
                for inline in inlines {
                    root.append_child(inline);
                }
                let mut renderer = VditorRenderer::new(
                    &tree,
                    self.base.options.clone(),
                    self.parse_options.clone(),
                );
                self.base.write(&renderer.render());
            }
            self.tag("/p", &[], false);
        } else {
            self.tag("/blockquote", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        let level = n.data().heading_level.clamp(1, 6) as usize;
        if entering {
            let mut s = format!("<h{} data-block=\"0\"", level);
            let mut id = n
                .child_by_type(NodeType::NodeHeadingID)
                .map(|x| x.tokens_str())
                .unwrap_or_default();
            if self.base.options.heading_id && !id.is_empty() {
                s.push_str(" data-id=\"");
                s.push_str(&id);
                s.push('"');
            }
            if id.is_empty() {
                id = super::heading_id(n);
            }
            s.push_str(" id=\"wysiwyg-");
            s.push_str(&id);
            s.push('"');
            if !n.data().heading_setext {
                s.push_str(" data-marker=\"#\">");
            } else if 1 == level {
                s.push_str(" data-marker=\"=\">");
            } else {
                s.push_str(" data-marker=\"-\">");
            }
            self.base.write_str(&s);
            if self.base.options.heading_anchor {
                let anchor_id = super::heading_id(n);
                let anchor = format!("vditorAnchor-{}", anchor_id);
                let href = format!("#{}", anchor_id);
                self.tag(
                    "a",
                    &[("id", &anchor), ("class", "vditor-anchor"), ("href", &href)],
                    false,
                );
                self.base.write_str("<svg viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\"><path fill-rule=\"evenodd\" d=\"M4 9h1v1H4c-1.5 0-3-1.69-3-3.5S2.55 3 4 3h4c1.45 0 3 1.69 3 3.5 0 1.41-.91 2.72-2 3.25V8.59c.58-.45 1-1.27 1-2.09C10 5.22 8.98 4 8 4H4c-.98 0-2 1.22-2 2.5S3 9 4 9zm9-3h-1v1h1c1 0 2 1.22 2 2.5S13.98 12 13 12H9c-.98 0-2-1.22-2-2.5 0-.83.42-1.64 1-2.09V6.25c-1.09.53-2 1.84-2 3.25C6 11.31 7.55 13 9 13h4c1.45 0 3-1.69 3-3.5S14.5 6 13 6z\"></path></svg>");
                self.tag("/a", &[], false);
            }
        } else {
            let s = format!("</h{}>", level);
            self.base.write_str(&s);
        }
        WalkStatus::WalkContinue
    }

    fn render_list(&mut self, n: Node, entering: bool) -> WalkStatus {
        let ld = n.data().list_data.clone().unwrap_or_default();
        let tag = if 1 == ld.typ || (3 == ld.typ && 0 == ld.bullet_char) {
            "ol"
        } else {
            "ul"
        };
        if entering {
            let mut attrs: Vec<(String, String)> = Vec::new();
            if ld.tight {
                attrs.push(("data-tight".into(), "true".into()));
            }
            if 0 == ld.bullet_char && 1 != ld.start {
                attrs.push(("start".into(), ld.start.to_string()));
            }
            match ld.typ {
                0 => attrs.push((
                    "data-marker".into(),
                    String::from_utf8_lossy(&ld.marker).to_string(),
                )),
                1 => attrs.push((
                    "data-marker".into(),
                    format!("{}{}", ld.num, ld.delimiter as char),
                )),
                3 => {
                    if 0 == ld.bullet_char {
                        attrs.push((
                            "data-marker".into(),
                            format!("{}{}", ld.num, ld.delimiter as char),
                        ));
                    } else {
                        attrs.push((
                            "data-marker".into(),
                            String::from_utf8_lossy(&ld.marker).to_string(),
                        ));
                    }
                }
                _ => {}
            }
            attrs.push(("data-block".into(), "0".into()));
            self.render_list_style(n, &mut attrs);
            self.tag_owned(tag, &attrs, false);
        } else {
            self.tag(&format!("/{}", tag), &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_list_style(&self, n: Node, attrs: &mut Vec<(String, String)>) {
        if !self.base.options.render_list_style {
            return;
        }
        let ld = n.data().list_data.clone().unwrap_or_default();
        match ld.typ {
            0 => attrs.push((
                "data-style".into(),
                String::from_utf8_lossy(&ld.marker).to_string(),
            )),
            1 => attrs.push((
                "data-style".into(),
                format!("{}{}", ld.num, ld.delimiter as char),
            )),
            3 => {
                if 0 == ld.bullet_char {
                    attrs.push((
                        "data-style".into(),
                        format!("{}{}", ld.num, ld.delimiter as char),
                    ));
                } else {
                    attrs.push((
                        "data-style".into(),
                        String::from_utf8_lossy(&ld.marker).to_string(),
                    ));
                }
            }
            _ => {}
        }
    }

    fn render_list_item(&mut self, n: Node, entering: bool) -> WalkStatus {
        let ld = n.data().list_data.clone().unwrap_or_default();
        if entering {
            let mut attrs: Vec<(String, String)> = Vec::new();
            match ld.typ {
                0 => attrs.push((
                    "data-marker".into(),
                    String::from_utf8_lossy(&ld.marker).to_string(),
                )),
                1 => attrs.push((
                    "data-marker".into(),
                    format!("{}{}", ld.num, ld.delimiter as char),
                )),
                3 => {
                    if 0 == ld.bullet_char {
                        attrs.push((
                            "data-marker".into(),
                            format!("{}{}", ld.num, ld.delimiter as char),
                        ));
                    } else {
                        attrs.push((
                            "data-marker".into(),
                            String::from_utf8_lossy(&ld.marker).to_string(),
                        ));
                    }
                    if let Some(first) = n.first_child() {
                        if let Some(inner) = first.first_child() {
                            if NodeType::NodeTaskListItemMarker == inner.typ() {
                                attrs.push((
                                    "class".into(),
                                    self.base.options.gfm_task_list_item_class.clone(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
            self.tag_owned("li", &attrs, false);
            if n.first_child().is_none() {
                self.base.write_str(editor::ZWSP);
            }
        } else {
            self.tag("/li", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_task_list_item_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs: Vec<(String, String)> = Vec::new();
            if n.data().task_list_item_checked {
                attrs.push(("checked".into(), String::new()));
            }
            attrs.push(("type".into(), "checkbox".into()));
            if self.base.options.data_task {
                attrs.push(("data-task".into(), n.effective_task_list_item_marker()));
            }
            self.tag_owned("input", &attrs, true);
        }
        WalkStatus::WalkContinue
    }

    fn render_thematic_break(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("hr", &[("data-block", "0")], true);
            if !n.tokens().is_empty() {
                self.tag("p", &[("data-block", "0")], false);
                self.base.write(&n.tokens_vec());
                self.base.write_byte(b'\n');
                self.tag("/p", &[], false);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut marker = "\u{0060}\u{0060}\u{0060}".to_string();
            if let Some(first) = n.first_child() {
                if first.tokens().starts_with(&[0x60u8, 0x60, 0x60]) {
                    marker = first.tokens_str();
                }
            }
            self.base.write_str("<div class=\"vditor-wysiwyg__block\" data-type=\"code-block\" data-block=\"0\" data-marker=\"");
            self.base.write_str(&marker);
            self.base.write_str("\">");
        } else {
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_code(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = n.tokens_vec();
        let code_len = tokens.len();
        let code_is_empty = code_len < 1
            || (editor::CARET.len() == code_len && editor::CARET.as_bytes() == &tokens[..]);
        let is_fenced = n
            .parent()
            .map(|p| p.data().is_fenced_code_block)
            .unwrap_or(false);
        let mut language = String::new();
        let mut caret_in_info = false;
        let mut attrs: Vec<(String, String)> = Vec::new();
        if is_fenced {
            if let Some(prev) = n.previous() {
                let mut code_block_info = prev.data().code_block_info.clone();
                if contains(&code_block_info, editor::CARET.as_bytes()) {
                    caret_in_info = true;
                    code_block_info = replace_all(&code_block_info, editor::CARET.as_bytes(), b"");
                }
                if !code_block_info.is_empty() {
                    let info_words: Vec<&[u8]> = code_block_info
                        .split(|&b| b == b' ')
                        .filter(|w| !w.is_empty())
                        .collect();
                    if let Some(first) = info_words.first() {
                        language = String::from_utf8_lossy(first).to_string();
                        attrs.push(("class".into(), format!("language-{}", language)));
                        if "mindmap" == language {
                            let data_code = super::mindmap::echarts_mindmap_str(
                                &String::from_utf8_lossy(&tokens),
                            );
                            attrs.push(("data-code".into(), data_code));
                        }
                    }
                }
            }
        }
        let mut pre_attrs: Vec<(String, String)> =
            vec![("class".into(), "vditor-wysiwyg__pre".into())];
        if !contains(&tokens, editor::CARET.as_bytes())
            && !caret_in_info
            && self.base.options.vditor_code_block_preview
        {
            pre_attrs.push(("style".into(), "display: none".into()));
        }
        self.tag_owned("pre", &pre_attrs, false);
        self.tag_owned("code", &attrs, false);

        if code_is_empty {
            self.base.write_str(editor::FRONT_END_CARET);
            self.base.write_str("\n");
        } else {
            if caret_in_info {
                self.base.write_str(editor::FRONT_END_CARET);
            }
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
            self.base.newline();
        }
        self.base.write_str("</code></pre>");

        if self.base.options.vditor_code_block_preview {
            self.tag(
                "pre",
                &[("class", "vditor-wysiwyg__preview"), ("data-render", "2")],
                false,
            );
            let pre_div = no_highlight(&language);
            if pre_div {
                self.tag_owned("div", &attrs, false);
            } else {
                self.tag_owned("code", &attrs, false);
            }
            let tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
            if pre_div {
                self.base.write_str("</div></pre>");
            } else {
                self.base.write_str("</code></pre>");
            }
        }
        WalkStatus::WalkContinue
    }
}

impl Renderer for VditorRenderer {
    fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);
        let root = self.root;
        let me = &mut *self;
        crate::ast::walk_raw(root, &mut |n, entering| me.render_node(n, entering));
        std::mem::take(&mut self.base.out)
    }
}

struct TocHeading {
    id: String,
    content: String,
    children: Vec<TocHeading>,
}

fn toc_container<'a>(headings: &'a mut Vec<TocHeading>, path: &[usize]) -> &'a mut Vec<TocHeading> {
    let mut cur = headings;
    for &i in path {
        cur = &mut cur[i].children;
    }
    cur
}

fn heading_text(n: Node) -> String {
    let mut buf = String::new();
    crate::ast::walk_raw(n, &mut |n, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        match n.typ() {
            NodeType::NodeLinkText
            | NodeType::NodeBlockRefText
            | NodeType::NodeBlockRefDynamicText
            | NodeType::NodeFileAnnotationRefText
            | NodeType::NodeBackslashContent => buf.push_str(&n.tokens_str()),
            NodeType::NodeInlineMathContent => {
                buf.push_str("<span class=\"language-math\">");
                buf.push_str(&html_util::escape_html(&n.tokens_str()));
                buf.push_str("</span>");
            }
            NodeType::NodeCodeSpanContent => {
                buf.push_str("<code>");
                buf.push_str(&html_util::escape_html(&n.tokens_str()));
                buf.push_str("</code>");
            }
            NodeType::NodeText => {
                if n.parent_is(&[NodeType::NodeStrong]) {
                    buf.push_str("<strong>");
                    buf.push_str(&html_util::escape_html(&n.tokens_str()));
                    buf.push_str("</strong>");
                } else if n.parent_is(&[NodeType::NodeEmphasis]) {
                    buf.push_str("<em>");
                    buf.push_str(&html_util::escape_html(&n.tokens_str()));
                    buf.push_str("</em>");
                } else if let Some(prev) = n.previous() {
                    if NodeType::NodeInlineHTML == prev.typ() {
                        if prev.tokens().starts_with(b"<font ") {
                            buf.push_str(&prev.tokens_str());
                            buf.push_str(&n.tokens_str());
                        }
                        if let Some(next) = n.next() {
                            if next.tokens() == b"</font>" {
                                buf.push_str(&next.tokens_str());
                            }
                        }
                    } else {
                        buf.push_str(&html_util::escape_html(&n.tokens_str()));
                    }
                } else {
                    buf.push_str(&html_util::escape_html(&n.tokens_str()));
                }
            }
            _ => {}
        }
        WalkStatus::WalkContinue
    });
    buf
}

fn callout_display_title(node: Node) -> String {
    let d = node.data();
    let mut title = d.callout_title.clone();
    if title.is_empty() && !d.callout_title_explicit {
        title = crate::parse::callout::get_callout_title(&d.callout_type).to_string();
    }
    if title.is_empty() && !d.callout_title_explicit {
        title = d.callout_type.clone();
    }
    title
}

fn no_highlight(language: &str) -> bool {
    matches!(
        language,
        "mermaid"
            | "echarts"
            | "abc"
            | "graphviz"
            | "mindmap"
            | "flowchart"
            | "plantuml"
            | "infographic"
    )
}

fn previous_node_text(n: Node) -> Option<String> {
    // 与 Go 版一致：只看紧邻的前一个节点（跳过 KramdownSpanIAL），不跳过空节点。
    // 空节点（如 SoftBreak）会让调用方补零宽空格，跳过会漏掉边界。
    let mut prev = n.previous();
    if let Some(p) = prev {
        if NodeType::NodeKramdownSpanIAL == p.typ() {
            prev = p.previous();
        }
    }
    prev.map(|p| p.text())
}

fn replace_all(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    super::replace_bytes(tokens, old, new)
}

fn contains(tokens: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    tokens.windows(needle.len()).any(|w| w == needle)
}

fn find(tokens: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    tokens.windows(needle.len()).position(|w| w == needle)
}

fn find_last(tokens: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(tokens.len());
    }
    tokens.windows(needle.len()).rposition(|w| w == needle)
}

fn trim_ascii_space(tokens: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end
        && (tokens[start] == b' '
            || tokens[start] == b'\t'
            || tokens[start] == b'\n'
            || tokens[start] == b'\r')
    {
        start += 1;
    }
    while end > start
        && (tokens[end - 1] == b' '
            || tokens[end - 1] == b'\t'
            || tokens[end - 1] == b'\n'
            || tokens[end - 1] == b'\r')
    {
        end -= 1;
    }
    &tokens[start..end]
}

fn sub_str(s: &str, length: usize) -> String {
    let mut ret = String::new();
    for (count, c) in s.chars().enumerate() {
        ret.push(c);
        if length <= count + 1 {
            break;
        }
    }
    ret
}
