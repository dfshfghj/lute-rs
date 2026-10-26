// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Vditor 分屏预览（Split-View）DOM 渲染器，对齐 Go 版 render.VditorSVRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::dom::{Dom, DomType};
use crate::editor;
use crate::html as html_util;
use crate::options::{ParseOptions, RenderOptions};

use super::format::escape_setext_heading_markers_in_list_paragraph;
use super::{BaseRenderer, Renderer};

/// 分屏预览模式下的换行标记。
pub const NEWLINE_SV: &[u8] =
    b"<span data-type=\"newline\"><br /><span style=\"display: none\">\n</span></span>";

const BLOCKQUOTE_MARKER: &[u8] =
    b"<span data-type=\"blockquote-marker\" class=\"vditor-sv__marker\">&gt; </span>";

/// Vditor 分屏预览 DOM 渲染器。
pub struct VditorSVRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
    /// 节点输出缓冲栈（保存外层 writer）。
    stack: Vec<Vec<u8>>,
    /// 最新输出的 newline 长度个字节。
    last_out: Vec<u8>,
}

impl VditorSVRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
            stack: Vec::new(),
            last_out: Vec::new(),
        }
    }

    // ---- 输出 -------------------------------------------------------------

    fn write_byte(&mut self, c: u8) {
        self.base.out.push(c);
        self.last_out.push(c);
        if 1024 < self.last_out.len() {
            self.last_out.drain(..512);
        }
    }

    fn write(&mut self, content: &[u8]) {
        if content.is_empty() {
            return;
        }
        self.base.out.extend_from_slice(content);
        self.last_out.extend_from_slice(content);
        if 1024 < self.last_out.len() {
            self.last_out.drain(..512);
        }
    }

    fn write_str(&mut self, content: &str) {
        self.write(content.as_bytes());
    }

    fn newline(&mut self) {
        if !self.last_out.ends_with(NEWLINE_SV) {
            self.base.out.extend_from_slice(NEWLINE_SV);
            self.last_out = NEWLINE_SV.to_vec();
        }
    }

    fn tag(&mut self, name: &str, attrs: &[(&str, &str)], selfclosing: bool) {
        let owned: Vec<(String, String)> = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.base.tag(name, &owned, selfclosing);
    }

    /// 输出一个 span 标记元素。
    fn marker(&mut self, class: &str, content: &str) {
        self.tag("span", &[("class", class)], false);
        self.write_str(content);
        self.tag("/span", &[], false);
    }

    fn marker_byte(&mut self, class: &str, c: u8) {
        self.tag("span", &[("class", class)], false);
        self.write_byte(c);
        self.tag("/span", &[], false);
    }

    fn push_writer(&mut self) {
        let cur = std::mem::take(&mut self.base.out);
        self.stack.push(cur);
    }

    fn pop_writer(&mut self) {
        let child = std::mem::take(&mut self.base.out);
        self.base.out = self.stack.pop().unwrap_or_default();
        self.base.out.extend_from_slice(&child);
    }

    fn pop_write_class(&mut self, node: Node, class: &str) {
        let child = std::mem::take(&mut self.base.out);
        self.base.out = self.stack.pop().unwrap_or_default();
        let classed = apply_class(child, node, class);
        self.base.out.extend_from_slice(&classed);
    }

    fn render_class(&mut self, node: Node, class: &str) {
        let buf = std::mem::take(&mut self.base.out);
        self.base.out = apply_class(buf, node, class);
    }

    /// 引述块 / 标注块收尾：给每一行加上引述标记。
    fn close_quote(&mut self) {
        let child = std::mem::take(&mut self.base.out);
        self.base.out = self.stack.pop().unwrap_or_default();

        let mut buf = Vec::with_capacity(BLOCKQUOTE_MARKER.len() + child.len());
        buf.extend_from_slice(BLOCKQUOTE_MARKER);
        buf.extend_from_slice(&child);
        while buf.ends_with(NEWLINE_SV) {
            buf.truncate(buf.len() - NEWLINE_SV.len());
        }
        let replaced = replace_bytes(&buf, NEWLINE_SV, &concat_newline_marker());
        self.base.out.extend_from_slice(&replaced);

        let whole = std::mem::take(&mut self.base.out);
        self.write(&whole);
        self.newline();
        self.write(NEWLINE_SV);
    }

    // ---- 渲染 -------------------------------------------------------------

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some((s, st)) = self.base.ext_render(n, entering) {
            self.write_str(&s);
            return st;
        }
        match n.typ() {
            NodeType::NodeDocument => self.render_document(n, entering),
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => self.render_text(n, entering),
            NodeType::NodeCodeSpan => WalkStatus::WalkContinue,
            NodeType::NodeCodeSpanOpenMarker => self.render_code_span_open_marker(n, entering),
            NodeType::NodeCodeSpanContent => self.render_code_span_content(n, entering),
            NodeType::NodeCodeSpanCloseMarker => self.render_code_span_close_marker(n, entering),
            NodeType::NodeCodeBlock => self.render_code_block(n, entering),
            NodeType::NodeCodeBlockFenceOpenMarker => {
                self.render_code_block_open_marker(n, entering)
            }
            NodeType::NodeCodeBlockFenceInfoMarker => {
                self.render_code_block_info_marker(n, entering)
            }
            NodeType::NodeCodeBlockCode => self.render_code_block_code(n, entering),
            NodeType::NodeCodeBlockFenceCloseMarker => {
                self.render_code_block_close_marker(n, entering)
            }
            NodeType::NodeMathBlock => WalkStatus::WalkContinue,
            NodeType::NodeMathBlockOpenMarker => self.render_math_block_open_marker(n, entering),
            NodeType::NodeMathBlockContent => self.render_math_block_content(n, entering),
            NodeType::NodeMathBlockCloseMarker => self.render_math_block_close_marker(n, entering),
            NodeType::NodeInlineMath => WalkStatus::WalkContinue,
            NodeType::NodeInlineMathOpenMarker => self.render_inline_math_open_marker(n, entering),
            NodeType::NodeInlineMathContent => self.render_inline_math_content(n, entering),
            NodeType::NodeInlineMathCloseMarker => {
                self.render_inline_math_close_marker(n, entering)
            }
            NodeType::NodeEmphasis => self.render_emphasis(n, entering),
            NodeType::NodeEmA6kOpenMarker => self.render_marker_bi(n, "*", entering),
            NodeType::NodeEmA6kCloseMarker => self.render_marker_bi(n, "*", entering),
            NodeType::NodeEmU8eOpenMarker => self.render_marker_bi(n, "_", entering),
            NodeType::NodeEmU8eCloseMarker => self.render_marker_bi(n, "_", entering),
            NodeType::NodeStrong => self.render_strong(n, entering),
            NodeType::NodeStrongA6kOpenMarker => self.render_marker_bi(n, "**", entering),
            NodeType::NodeStrongA6kCloseMarker => self.render_marker_bi(n, "**", entering),
            NodeType::NodeStrongU8eOpenMarker => self.render_marker_bi(n, "__", entering),
            NodeType::NodeStrongU8eCloseMarker => self.render_marker_bi(n, "__", entering),
            NodeType::NodeBlockquote => self.render_blockquote(n, entering),
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeadingID => self.render_heading_id(n, entering),
            NodeType::NodeList => self.render_list(n, entering),
            NodeType::NodeListItem => self.render_list_item(n, entering),
            NodeType::NodeMindmap => self.render_list(n, entering),
            NodeType::NodeMindmapItem => self.render_list_item(n, entering),
            NodeType::NodeThematicBreak => self.render_thematic_break(n, entering),
            NodeType::NodeHardBreak => self.render_break(n, entering),
            NodeType::NodeSoftBreak => self.render_break(n, entering),
            NodeType::NodeHTMLBlock => self.render_html(n, entering),
            NodeType::NodeInlineHTML => self.render_inline_html(n, entering),
            NodeType::NodeLink => WalkStatus::WalkContinue,
            NodeType::NodeImage => self.render_image(n, entering),
            NodeType::NodeBang => self.render_bang(n, entering),
            NodeType::NodeOpenBracket => self.render_open_bracket(n, entering),
            NodeType::NodeCloseBracket => self.render_close_bracket(n, entering),
            NodeType::NodeOpenParen => self.render_open_paren(n, entering),
            NodeType::NodeCloseParen => self.render_close_paren(n, entering),
            NodeType::NodeOpenBrace => self.render_open_brace(n, entering),
            NodeType::NodeCloseBrace => self.render_close_brace(n, entering),
            NodeType::NodeLinkText => self.render_link_text(n, entering),
            NodeType::NodeLinkSpace => self.render_link_space(n, entering),
            NodeType::NodeLinkDest => self.render_link_dest(n, entering),
            NodeType::NodeLinkTitle => self.render_link_title(n, entering),
            NodeType::NodeStrikethrough => self.render_strikethrough(n, entering),
            NodeType::NodeStrikethrough1OpenMarker => self.render_marker(n, "~", entering),
            NodeType::NodeStrikethrough1CloseMarker => self.render_marker(n, "~", entering),
            NodeType::NodeStrikethrough2OpenMarker => self.render_marker(n, "~~", entering),
            NodeType::NodeStrikethrough2CloseMarker => self.render_marker(n, "~~", entering),
            NodeType::NodeTaskListItemMarker => self.render_task_list_item_marker(n, entering),
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead | NodeType::NodeTableRow | NodeType::NodeTableCell => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmoji => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode => WalkStatus::WalkContinue,
            NodeType::NodeEmojiImg => WalkStatus::WalkContinue,
            NodeType::NodeEmojiAlias => {
                if entering {
                    let tokens = n.tokens_vec();
                    self.write(&tokens);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDefBlock => WalkStatus::WalkContinue,
            NodeType::NodeFootnotesDef => self.render_footnotes_def(n, entering),
            NodeType::NodeFootnotesRef => self.render_footnotes_ref(n, entering),
            NodeType::NodeToC => self.render_toc(n, entering),
            NodeType::NodeBackslash => self.render_backslash(n, entering),
            NodeType::NodeBackslashContent => self.render_backslash_content(n, entering),
            NodeType::NodeHTMLEntity => self.render_html_entity(n, entering),
            NodeType::NodeYamlFrontMatter => WalkStatus::WalkContinue,
            NodeType::NodeYamlFrontMatterOpenMarker => {
                self.render_yaml_front_matter_open_marker(n, entering)
            }
            NodeType::NodeYamlFrontMatterContent => {
                self.render_yaml_front_matter_content(n, entering)
            }
            NodeType::NodeYamlFrontMatterCloseMarker => {
                self.render_yaml_front_matter_close_marker(n, entering)
            }
            NodeType::NodeMark => self.render_mark(n, entering),
            NodeType::NodeMark1OpenMarker => self.render_marker(n, "=", entering),
            NodeType::NodeMark1CloseMarker => self.render_marker(n, "=", entering),
            NodeType::NodeMark2OpenMarker => self.render_marker(n, "==", entering),
            NodeType::NodeMark2CloseMarker => self.render_marker(n, "==", entering),
            NodeType::NodeSup => self.render_sup(n, entering),
            NodeType::NodeSupOpenMarker => self.render_marker(n, "^", entering),
            NodeType::NodeSupCloseMarker => self.render_marker(n, "^", entering),
            NodeType::NodeSub => self.render_sub(n, entering),
            NodeType::NodeSubOpenMarker => self.render_marker(n, "~", entering),
            NodeType::NodeSubCloseMarker => self.render_marker(n, "~", entering),
            NodeType::NodeKramdownBlockIAL => self.render_kramdown_block_ial(n, entering),
            NodeType::NodeLinkRefDefBlock => WalkStatus::WalkContinue,
            NodeType::NodeLinkRefDef => self.render_link_ref_def(n, entering),
            _ => WalkStatus::WalkContinue,
        }
    }

    fn render_document(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.stack.pop();
            let buf = trim_bytes(&self.base.out, b" \t\n");
            self.base.out.clear();
            self.write(&buf);
        }
        WalkStatus::WalkContinue
    }

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.newline();
            let grandparent = n.parent().and_then(|p| p.parent());
            let in_tight_list = grandparent.is_some_and(|g| {
                NodeType::NodeList == g.typ()
                    && g.data().list_data.as_ref().is_some_and(|ld| ld.tight)
            });
            if !in_tight_list {
                self.write(NEWLINE_SV);
            }
            self.pop_writer();
        }
        WalkStatus::WalkContinue
    }

    fn render_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if n.parent_is(&[NodeType::NodeTableCell]) {
            return WalkStatus::WalkContinue;
        }
        if entering {
            let mut tokens = n.tokens_vec();
            if self.base.options.fix_term_typo {
                self.base.fix_term_typo(&mut tokens);
            }
            tokens = escape_setext_heading_markers_in_list_paragraph(n, &tokens);
            self.tag("span", &[("data-type", "text")], false);
            while tokens.last() == Some(&b'\n') {
                tokens.pop();
            }
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.write_str(&escaped);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_open_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let marker_len = n.parent().map_or(0, |p| p.data().code_marker_len).max(0) as usize;
            let marker = "`".repeat(marker_len);
            self.tag("span", &[("class", "vditor-sv__marker")], false);
            self.write_str(&marker);
            if n.next().is_some_and(|nx| nx.tokens().starts_with(b"`")) {
                self.write_byte(b' ');
            }
            self.tag("/span", &[], false);
            self.tag("span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let escaped = html_util::escape_html(&n.tokens_str());
            self.write_str(&escaped);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_close_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let marker_len = n.parent().map_or(0, |p| p.data().code_marker_len).max(0) as usize;
            let marker = "`".repeat(marker_len);
            self.tag("/span", &[], false);
            self.tag("span", &[("class", "vditor-sv__marker")], false);
            if n.previous().is_some_and(|p| p.tokens().ends_with(b"`")) {
                self.write_byte(b' ');
            }
            self.write_str(&marker);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        let fenced = n.data().is_fenced_code_block;
        if entering {
            if !fenced {
                self.tag(
                    "span",
                    &[
                        ("data-type", "code-block-open-marker"),
                        ("class", "vditor-sv__marker"),
                    ],
                    false,
                );
                self.write_str("```");
                self.tag("/span", &[], false);
                self.newline();
            }
        } else {
            if !fenced {
                self.newline();
                self.tag(
                    "span",
                    &[
                        ("class", "vditor-sv__marker--info"),
                        ("data-type", "code-block-info"),
                    ],
                    false,
                );
                self.write_str("```");
                self.tag("/span", &[], false);
            }
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_open_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[
                    ("data-type", "code-block-open-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_info_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[
                    ("class", "vditor-sv__marker--info"),
                    ("data-type", "code-block-info"),
                ],
                false,
            );
            let info = n.data().code_block_info.clone();
            self.write(&info);
            self.tag("/span", &[], false);
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_close_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline();
            self.tag(
                "span",
                &[
                    ("data-type", "code-block-close-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.tag("/span", &[], false);
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_code(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("data-type", "text")], false);
            let tokens =
                html_util::escape_html(&String::from_utf8_lossy(&trim_space(&n.tokens_vec())));
            let mut newline = b"<span data-type=\"padding\"></span>".to_vec();
            newline.extend_from_slice(NEWLINE_SV);
            let tokens = replace_bytes(tokens.as_bytes(), b"\n", &newline);
            self.write(&tokens);
            self.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    fn render_math_block_open_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[
                    ("data-type", "math-block-open-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            self.write_str("$$");
            self.tag("/span", &[], false);
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_math_block_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("data-type", "text")], false);
            let tokens =
                html_util::escape_html(&String::from_utf8_lossy(&trim_space(&n.tokens_vec())));
            let mut newline = b"<span data-type=\"padding\"></span>".to_vec();
            newline.extend_from_slice(NEWLINE_SV);
            let tokens = replace_bytes(tokens.as_bytes(), b"\n", &newline);
            self.write(&tokens);
            self.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    fn render_math_block_close_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline();
            self.tag(
                "span",
                &[
                    ("data-type", "math-block-close-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            self.write_str("$$");
            self.tag("/span", &[], false);
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_math_open_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.marker_byte("vditor-sv__marker", b'$');
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_math_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let escaped = html_util::escape_html(&n.tokens_str());
            self.write_str(&escaped);
            self.tag("/code", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_math_close_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.marker_byte("vditor-sv__marker", b'$');
        }
        WalkStatus::WalkContinue
    }

    fn render_emphasis(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "em");
        }
        WalkStatus::WalkContinue
    }

    fn render_marker_bi(&mut self, _n: Node, marker: &str, entering: bool) -> WalkStatus {
        if entering {
            self.marker("vditor-sv__marker--bi", marker);
        }
        WalkStatus::WalkContinue
    }

    fn render_marker(&mut self, _n: Node, marker: &str, entering: bool) -> WalkStatus {
        if entering {
            self.marker("vditor-sv__marker", marker);
        }
        WalkStatus::WalkContinue
    }

    fn render_strong(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "strong");
        }
        WalkStatus::WalkContinue
    }

    fn render_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "mark");
        }
        WalkStatus::WalkContinue
    }

    fn render_sup(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "sup");
        }
        WalkStatus::WalkContinue
    }

    fn render_sub(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "sub");
        }
        WalkStatus::WalkContinue
    }

    fn render_strikethrough(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.pop_write_class(n, "s");
        }
        WalkStatus::WalkContinue
    }

    fn render_blockquote(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            self.close_quote();
        }
        WalkStatus::WalkContinue
    }

    fn render_callout(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
            self.tag(
                "span",
                &[
                    ("data-type", "text"),
                    ("class", "vditor-sv__marker--callout"),
                ],
                false,
            );
            let callout_type = n.data().callout_type.clone();
            let mut info = String::from("[!");
            info.push_str(&html_util::escape_html_str(&callout_type));
            info.push(']');
            self.write_str(&info);
            let title = callout_editable_info(n);
            if !title.is_empty() {
                self.write_byte(b' ');
                let escaped = html_util::escape_html(&title);
                self.write_str(&escaped);
            }
            self.tag("/span", &[], false);
            self.newline();
        } else {
            self.close_quote();
        }
        WalkStatus::WalkContinue
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        let level = n.data().heading_level;
        let setext = n.data().heading_setext;
        if entering {
            self.push_writer();
            if !setext {
                self.tag(
                    "span",
                    &[
                        ("class", "vditor-sv__marker--heading"),
                        ("data-type", "heading-marker"),
                    ],
                    false,
                );
                let mut marker = "#".repeat(level.max(0) as usize);
                marker.push(' ');
                self.write_str(&marker);
                self.tag("/span", &[], false);
            }
        } else {
            if setext {
                self.newline();
                self.tag(
                    "span",
                    &[
                        ("class", "vditor-sv__marker--heading"),
                        ("data-type", "heading-marker"),
                    ],
                    false,
                );
                let content_len = self.setext_heading_len(n);
                let ch = if 1 == level { '=' } else { '-' };
                let marker: String = std::iter::repeat_n(ch, content_len).collect();
                self.write_str(&marker);
                self.tag("/span", &[], false);
            }

            let class = format!("h{}", level);
            self.render_class(n, &class);
            self.newline();
            self.write(NEWLINE_SV);
            self.pop_writer();
        }
        WalkStatus::WalkContinue
    }

    fn setext_heading_len(&self, n: Node) -> usize {
        let mut buf = String::new();
        crate::ast::walk_raw(n, &mut |cn, entering| {
            if entering
                && (NodeType::NodeText == cn.typ()
                    || NodeType::NodeLinkText == cn.typ()
                    || NodeType::NodeSoftBreak == cn.typ())
            {
                buf.push_str(&cn.tokens_str());
            }
            WalkStatus::WalkContinue
        });
        let content = buf.replace(editor::CARET, "");
        let last_line = content.split('\n').next_back().unwrap_or("");
        let mut ret = 0usize;
        for c in last_line.chars() {
            if 0x80 <= c as u32 {
                ret += 2;
            } else {
                ret += 1;
            }
        }
        if 0 == ret {
            ret = 3;
        }
        ret
    }

    fn render_heading_id(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("class", "vditor-sv__marker")], false);
            let mut s = String::from(" {");
            s.push_str(&n.tokens_str());
            s.push('}');
            self.write_str(&s);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_list(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if !entering {
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_list_item(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
        } else {
            let child = std::mem::take(&mut self.base.out);
            self.base.out = self.stack.pop().unwrap_or_default();

            let ld = n.data().list_data.clone().unwrap_or_default();
            let marker_str = if 1 == ld.typ || (3 == ld.typ && 0 == ld.bullet_char) {
                format!("{}{}", ld.num, ld.delimiter as char)
            } else {
                String::from_utf8_lossy(&ld.marker).to_string()
            };
            let mut marker = format!(
                "<span data-type=\"li-marker\" class=\"vditor-sv__marker\">{} </span>",
                marker_str
            )
            .into_bytes();
            marker.extend_from_slice(&child);

            while marker.ends_with(NEWLINE_SV) {
                marker.truncate(marker.len() - NEWLINE_SV.len());
            }
            let padding = format!(
                "<span data-type=\"padding\">{}</span>",
                " ".repeat(ld.padding.max(0) as usize)
            );
            let mut with_padding = NEWLINE_SV.to_vec();
            with_padding.extend_from_slice(padding.as_bytes());
            let marker = replace_bytes(&marker, NEWLINE_SV, &with_padding);

            self.base.out.extend_from_slice(&marker);
            let whole = std::mem::take(&mut self.base.out);
            self.write(&whole);
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_task_list_item_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let marker = n.effective_task_list_item_marker();
        let checked = n.data().task_list_item_checked;

        self.tag(
            "span",
            &[
                ("data-type", "task-marker"),
                ("class", "vditor-sv__marker--bi"),
            ],
            false,
        );
        self.write_byte(b'[');
        self.tag("/span", &[], false);
        if checked {
            self.tag(
                "span",
                &[
                    ("data-type", "task-marker"),
                    ("class", "vditor-sv__marker--strong"),
                ],
                false,
            );
            self.write_str(&marker);
            self.tag("/span", &[], false);
        } else {
            self.tag(
                "span",
                &[
                    ("data-type", "task-marker"),
                    ("class", "vditor-sv__marker--bi"),
                ],
                false,
            );
            self.write_str(&marker);
            self.tag("/span", &[], false);
        }
        self.tag(
            "span",
            &[
                ("data-type", "task-marker"),
                ("class", "vditor-sv__marker--bi"),
            ],
            false,
        );
        self.write_str("] ");
        self.tag("/span", &[], false);

        if let Some(next) = n.next() {
            if next.tokens().starts_with(b" ") {
                next.set_tokens(next.tokens()[1..].to_vec());
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_thematic_break(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.marker("vditor-sv__marker", "***");
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_break(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("class", "vditor-sv__marker")], false);
            let tokens =
                html_util::escape_html(&String::from_utf8_lossy(&trim_space(&n.tokens_vec())));
            let mut newline = b"<span data-type=\"padding\"></span>".to_vec();
            newline.extend_from_slice(NEWLINE_SV);
            let tokens = replace_bytes(tokens.as_bytes(), b"\n", &newline);
            self.write(&tokens);
            self.write_str("</span>");
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("class", "vditor-sv__marker")], false);
            let escaped = html_util::escape_html(&n.tokens_str());
            self.write_str(&escaped);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_image(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering && 3 == n.data().link_type {
            if let Some(c) = n.child_by_type(NodeType::NodeOpenParen) {
                c.unlink();
            }
            if let Some(c) = n.child_by_type(NodeType::NodeLinkDest) {
                c.unlink();
            }
            if let Some(link_space) = n.child_by_type(NodeType::NodeLinkSpace) {
                link_space.unlink();
                if let Some(c) = n.child_by_type(NodeType::NodeLinkTitle) {
                    c.unlink();
                }
            }
            if let Some(c) = n.child_by_type(NodeType::NodeCloseParen) {
                c.unlink();
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_bang(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.marker_byte("vditor-sv__marker", b'!');
        }
        WalkStatus::WalkContinue
    }

    fn render_open_bracket(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.marker_byte("vditor-sv__marker--bracket", b'[');
        }
        WalkStatus::WalkContinue
    }

    fn render_close_bracket(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        self.marker_byte("vditor-sv__marker--bracket", b']');
        let parent = match n.parent() {
            Some(p) => p,
            None => return WalkStatus::WalkContinue,
        };
        if 3 == parent.data().link_type {
            let link_text = parent.child_by_type(NodeType::NodeLinkText);
            let need = match link_text {
                None => true,
                Some(t) => !super::eq_fold(&parent.data().link_ref_label, t.tokens()),
            };
            if need {
                self.tag("span", &[("class", "vditor-sv__marker--link")], false);
                self.write_byte(b'[');
                let label = parent.data().link_ref_label.clone();
                self.write(&label);
                self.write_byte(b']');
                self.tag("/span", &[], false);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_open_paren(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.marker_byte("vditor-sv__marker--paren", b'(');
        }
        WalkStatus::WalkContinue
    }

    fn render_close_paren(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.marker_byte("vditor-sv__marker--paren", b')');
        }
        WalkStatus::WalkContinue
    }

    fn render_open_brace(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.marker_byte("vditor-sv__marker--brace", b'{');
        }
        WalkStatus::WalkContinue
    }

    fn render_close_brace(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.marker_byte("vditor-sv__marker--brace", b'}');
        }
        WalkStatus::WalkContinue
    }

    fn render_link_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let parent = match n.parent() {
            Some(p) => p,
            None => return WalkStatus::WalkContinue,
        };
        if NodeType::NodeImage == parent.typ() {
            self.tag("span", &[("class", "vditor-sv__marker--bracket")], false);
        } else if 3 == parent.data().link_type {
            self.tag("span", &[], false);
        } else {
            self.tag(
                "span",
                &[
                    ("class", "vditor-sv__marker--bracket"),
                    ("data-type", "link-text"),
                ],
                false,
            );
        }
        let tokens = n.tokens_vec();
        self.write(&tokens);
        self.tag("/span", &[], false);
        WalkStatus::WalkContinue
    }

    fn render_link_space(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.write_byte(b' ');
        }
        WalkStatus::WalkContinue
    }

    fn render_link_dest(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.tag("span", &[("class", "vditor-sv__marker--link")], false);
            let mut dest = n.tokens_vec();
            if self.base.options.sanitize {
                let trimmed = trim_space(&dest);
                let lower = String::from_utf8_lossy(&trimmed).to_lowercase();
                if lower.starts_with("javascript:") {
                    dest = Vec::new();
                }
            }
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&dest));
            self.write_str(&escaped);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_link_title(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent().is_some_and(|p| 3 == p.data().link_type) {
                return WalkStatus::WalkContinue;
            }
            self.tag("span", &[("class", "vditor-sv__marker--title")], false);
            self.write_byte(b'"');
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.write_byte(b'"');
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("data-type", "table")], false);
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.newline();
            self.write(NEWLINE_SV);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkSkipChildren
    }

    fn render_footnotes_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if self.base.rendering_footnotes {
            return WalkStatus::WalkContinue;
        }

        self.marker_byte("vditor-sv__marker--bracket", b'[');
        self.tag(
            "span",
            &[
                ("class", "vditor-sv__marker--link"),
                ("data-type", "footnotes-link"),
            ],
            false,
        );
        let tokens = n.tokens_vec();
        self.write(&tokens);
        self.tag("/span", &[], false);
        self.marker_byte("vditor-sv__marker--bracket", b']');
        self.write_str("<span>: </span>");

        let first = n.first_child();
        for c in n.children() {
            crate::ast::walk_raw(c, &mut |nn, ent| {
                if ent
                    && Some(nn) != first
                    && (nn.is_block()
                        || NodeType::NodeCodeBlockCode == nn.typ()
                        || NodeType::NodeCodeBlockFenceCloseMarker == nn.typ())
                {
                    let indent = "<span data-type=\"padding\">    </span>";
                    if NodeType::NodeCodeBlockFenceCloseMarker == nn.typ() {
                        let mut t = indent.as_bytes().to_vec();
                        t.extend_from_slice(nn.tokens());
                        nn.set_tokens(t);
                    } else {
                        self.write_str(indent);
                    }
                }
                self.render_node(nn, ent)
            });
        }
        WalkStatus::WalkSkipChildren
    }

    fn render_footnotes_ref(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let Some((_idx, def)) = self.find_footnotes_def(&n.tokens_vec()) else {
            return WalkStatus::WalkContinue;
        };
        let label = def.text();
        let aria = sub_str(&escape_go_string(&label), 24);
        let footnote_label = String::from_utf8_lossy(&n.data().footnotes_ref_label).to_string();

        self.tag("span", &[("class", "sup")], false);
        self.marker_byte("vditor-sv__marker--bracket", b'[');
        self.tag("span", &[("class", "vditor-sv__marker--link")], false);
        let tokens = n.tokens_vec();
        self.write(&tokens);
        self.tag("/span", &[], false);
        self.marker_byte("vditor-sv__marker--bracket", b']');
        self.tag("/span", &[], false);
        let _ = aria;
        let _ = footnote_label;
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

    fn render_toc(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.write_str(
                "<span class=\"vditor-toc\" data-type=\"toc-block\" contenteditable=\"false\">",
            );
            self.write_str("[toc]");
            self.write_str("</span>");
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_backslash(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.write_str("<span data-type=\"backslash\">");
            self.write_str("<span class=\"vditor-sv__marker\">");
            self.write_byte(b'\\');
            self.write_str("</span>");
        } else {
            self.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    fn render_backslash_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let escaped = html_util::escape_html(&n.tokens_str());
            self.write_str(&escaped);
        }
        WalkStatus::WalkContinue
    }

    fn render_html_entity(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[
                    ("class", "vditor-sv__marker--pre"),
                    ("data-type", "html-entity"),
                ],
                false,
            );
            let escaped =
                html_util::escape_html(&String::from_utf8_lossy(&n.data().html_entity_tokens));
            self.write_str(&escaped);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter_open_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[
                    ("data-type", "yaml-front-matter-open-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            self.write_str("---");
            self.tag("/span", &[], false);
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("data-type", "text")], false);
            let tokens =
                html_util::escape_html(&String::from_utf8_lossy(&trim_space(&n.tokens_vec())));
            let mut newline = b"<span data-type=\"padding\"></span>".to_vec();
            newline.extend_from_slice(NEWLINE_SV);
            let tokens = replace_bytes(tokens.as_bytes(), b"\n", &newline);
            self.write(&tokens);
            self.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter_close_marker(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline();
            self.tag(
                "span",
                &[
                    ("data-type", "yaml-front-matter-close-marker"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            self.write_str("---");
            self.tag("/span", &[], false);
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkContinue
    }

    fn render_kramdown_block_ial(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline();
            self.tag(
                "span",
                &[
                    ("data-type", "kramdown-ial"),
                    ("class", "vditor-sv__marker"),
                ],
                false,
            );
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.tag("/span", &[], false);
            self.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_link_ref_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let dest = n
                .first_child()
                .and_then(|c| c.child_by_type(NodeType::NodeLinkDest))
                .map(|d| d.tokens_vec())
                .unwrap_or_default();
            let dest_str = String::from_utf8_lossy(&dest).to_string();

            self.marker_byte("vditor-sv__marker--bracket", b'[');
            self.tag(
                "span",
                &[
                    ("class", "vditor-sv__marker--link"),
                    ("data-type", "link-ref-defs-block"),
                ],
                false,
            );
            let tokens = n.tokens_vec();
            self.write(&tokens);
            self.tag("/span", &[], false);
            self.marker_byte("vditor-sv__marker--bracket", b']');
            self.write_str("<span>:");
            if editor::CARET != dest_str {
                self.write_str(" ");
            }
            self.write_str("</span>");
            self.write_str(&dest_str);
            self.newline();
            self.write(NEWLINE_SV);
        }
        WalkStatus::WalkSkipChildren
    }
}

impl Renderer for VditorSVRenderer {
    fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);
        self.last_out.clear();
        let root = self.root;
        crate::ast::walk_raw(root, &mut |n, entering| self.render_node(n, entering));
        std::mem::take(&mut self.base.out)
    }
}

// ---- 辅助函数 -------------------------------------------------------------

fn concat_newline_marker() -> Vec<u8> {
    let mut ret = NEWLINE_SV.to_vec();
    ret.extend_from_slice(BLOCKQUOTE_MARKER);
    ret
}

fn callout_editable_info(node: Node) -> String {
    let d = node.data();
    let mut icon = d.callout_icon.clone();
    if 1 == d.callout_icon_type {
        if crate::parse::callout::is_valid_callout_image_src(&icon) {
            let dest = html_util::encode_destination(icon.as_bytes());
            icon = format!("![callout-icon](<{}>)", String::from_utf8_lossy(&dest));
        } else {
            icon = String::new();
        }
    }
    let mut title = d.callout_title.clone();
    if title.is_empty() && !d.callout_title_explicit {
        title = crate::parse::callout::get_callout_title(&d.callout_type).to_string();
    }
    if title.is_empty() && !d.callout_title_explicit {
        title = d.callout_type.clone();
    }
    let mut info = icon;
    info.push(' ');
    info.push_str(&title);
    info.trim().to_string()
}

fn trim_bytes(buf: &[u8], cutset: &[u8]) -> Vec<u8> {
    let start = buf
        .iter()
        .position(|b| !cutset.contains(b))
        .unwrap_or(buf.len());
    let mut end = buf.len();
    while end > start && cutset.contains(&buf[end - 1]) {
        end -= 1;
    }
    buf[start..end].to_vec()
}

fn trim_space(buf: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(buf);
    let trimmed = s.trim_matches(|c: char| c.is_whitespace());
    trimmed.as_bytes().to_vec()
}

fn replace_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    if old.is_empty() {
        return tokens.to_vec();
    }
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

/// 对应 Go 版 html.EscapeString。
fn escape_go_string(s: &str) -> String {
    let mut ret = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ret.push_str("&amp;"),
            '\'' => ret.push_str("&#39;"),
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            '"' => ret.push_str("&#34;"),
            '\r' => ret.push_str("&#13;"),
            _ => ret.push(c),
        }
    }
    ret
}

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn render_dom_html(dom: &Dom, node: usize, out: &mut Vec<u8>) {
    let n = &dom.nodes[node];
    match n.typ {
        DomType::Text => out.extend_from_slice(escape_go_string(&n.data).as_bytes()),
        DomType::Comment => {
            out.extend_from_slice(b"<!--");
            out.extend_from_slice(n.data.as_bytes());
            out.extend_from_slice(b"-->");
        }
        DomType::Doctype => {
            out.extend_from_slice(b"<!DOCTYPE ");
            out.extend_from_slice(n.data.as_bytes());
            out.push(b'>');
        }
        DomType::Element => {
            out.push(b'<');
            out.extend_from_slice(n.data.as_bytes());
            for (k, v) in &n.attrs {
                out.push(b' ');
                out.extend_from_slice(k.as_bytes());
                out.extend_from_slice(b"=\"");
                out.extend_from_slice(escape_go_string(v).as_bytes());
                out.push(b'"');
            }
            if is_void_tag(&n.data) {
                out.extend_from_slice(b"/>");
                return;
            }
            out.push(b'>');
            for &c in &n.children {
                render_dom_html(dom, c, out);
            }
            out.extend_from_slice(b"</");
            out.extend_from_slice(n.data.as_bytes());
            out.push(b'>');
        }
        DomType::Document => {
            for &c in &n.children {
                render_dom_html(dom, c, out);
            }
        }
    }
}

/// 给缓冲区中的顶层节点追加 class，对应 Go 版 renderClass。
fn apply_class(buf: Vec<u8>, _node: Node, class: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(&buf).to_string();
    let mut dom = crate::dom::parse(&text);
    let root = dom.root;
    let children = dom.nodes[root].children.clone();
    let mut out = Vec::with_capacity(buf.len() + 16);
    for c in children {
        let clazz = dom.attr(c, "class");
        let clazz = if clazz.is_empty() {
            class.to_string()
        } else {
            format!("{} {}", clazz, class)
        };
        dom.set_attr(c, "class", &clazz);
        render_dom_html(&dom, c, &mut out);
    }
    out
}
