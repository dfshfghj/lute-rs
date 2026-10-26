// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Protyle Block DOM 渲染器，对齐 Go 版 render.ProtyleRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::editor;
use crate::html as html_util;
use crate::options::{ParseOptions, RenderOptions};

use super::{BaseRenderer, Renderer};

/// Protyle WYSIWYG Block DOM 渲染器。
pub struct ProtyleRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
    pub node_index: i32,
}

impl ProtyleRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        let node_index = options.node_index_start;
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
            node_index,
        }
    }

    pub(crate) fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some((s, st)) = self.base.ext_render(n, entering) {
            self.base.write_str(&s);
            return st;
        }
        match n.typ() {
            NodeType::NodeDocument => WalkStatus::WalkContinue,
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => self.render_text(n, entering),
            NodeType::NodeCodeSpan => self.render_code_span(n, entering),
            NodeType::NodeCodeSpanOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "code")], false);
                    self.base.write_str(editor::WORD_JOINER);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanContent => {
                if entering {
                    self.base
                        .write(html_util::escape_html(&n.tokens_str()).as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanCloseMarker => {
                if entering {
                    self.base.write_str("</span>");
                    self.base.write_str(editor::ZWSP);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlock => self.render_code_block(n, entering),
            NodeType::NodeCodeBlockFenceOpenMarker
            | NodeType::NodeCodeBlockFenceInfoMarker
            | NodeType::NodeCodeBlockFenceCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeCodeBlockCode => self.render_code_block_code(n, entering),
            NodeType::NodeMathBlock => self.render_math_block(n, entering),
            NodeType::NodeMathBlockOpenMarker
            | NodeType::NodeMathBlockContent
            | NodeType::NodeMathBlockCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeInlineMath => self.render_inline_math(n, entering),
            NodeType::NodeInlineMathOpenMarker => {
                if entering {
                    let mut tokens = n.next().map(|x| x.tokens_vec()).unwrap_or_default();
                    tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
                    let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
                    let content = escaped.replace('"', "&quot;");
                    self.tag(
                        "span",
                        &[
                            ("data-type", "inline-math"),
                            ("data-subtype", "math"),
                            ("data-content", &content),
                            ("contenteditable", "false"),
                            ("class", "render-node"),
                        ],
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathContent => WalkStatus::WalkContinue,
            NodeType::NodeInlineMathCloseMarker => {
                if entering {
                    self.base.write_str("</span>");
                    if n.previous()
                        .map(|p| {
                            p.tokens_vec()
                                .windows(editor::CARET.len())
                                .any(|w| w == editor::CARET.as_bytes())
                        })
                        .unwrap_or(false)
                    {
                        self.base.write_str(editor::CARET);
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmphasis => self.auto_space(n, entering),
            NodeType::NodeEmA6kOpenMarker | NodeType::NodeEmU8eOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "em")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmA6kCloseMarker | NodeType::NodeEmU8eCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrong => self.auto_space(n, entering),
            NodeType::NodeStrongA6kOpenMarker | NodeType::NodeStrongU8eOpenMarker => {
                if entering {
                    let mut attrs: Vec<(String, String)> =
                        vec![("data-type".into(), "strong".into())];
                    if let Some(parent) = n.parent() {
                        self.span_node_attrs(parent, &mut attrs);
                    }
                    self.tag_owned("span", &attrs, false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kCloseMarker | NodeType::NodeStrongU8eCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote => {
                if entering {
                    let mut attrs = Vec::new();
                    self.block_node_attrs(n, &mut attrs, "bq");
                    self.tag_owned("div", &attrs, false);
                } else {
                    self.render_ial(n);
                    self.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker | NodeType::NodeHeadingID => WalkStatus::WalkContinue,
            NodeType::NodeList | NodeType::NodeMindmap => {
                if entering {
                    let mut attrs = Vec::new();
                    match n.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0) {
                        0 => attrs.push(("data-subtype".into(), "u".into())),
                        1 => attrs.push(("data-subtype".into(), "o".into())),
                        3 => attrs.push(("data-subtype".into(), "t".into())),
                        _ => {}
                    }
                    let class = if NodeType::NodeMindmap == n.typ() {
                        "mindmap"
                    } else {
                        "list"
                    };
                    self.block_node_attrs(n, &mut attrs, class);
                    self.tag_owned("div", &attrs, false);
                } else {
                    self.render_ial(n);
                    self.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeListItem | NodeType::NodeMindmapItem => {
                self.render_list_item(n, entering)
            }
            NodeType::NodeThematicBreak => {
                if entering {
                    let mut attrs = Vec::new();
                    self.block_node_attrs(n, &mut attrs, "hr");
                    self.tag_owned("div", &attrs, false);
                    self.tag("div", &[], false);
                } else {
                    self.tag("/div", &[], false);
                    self.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHardBreak => self.render_br(n, entering),
            NodeType::NodeSoftBreak => {
                if entering {
                    self.base.write_byte(b'\n');
                    if let (Some(prev), Some(next)) = (n.previous(), n.next()) {
                        if matches!(
                            prev.typ(),
                            NodeType::NodeStrong
                                | NodeType::NodeEmphasis
                                | NodeType::NodeTag
                                | NodeType::NodeStrikethrough
                                | NodeType::NodeUnderline
                                | NodeType::NodeKramdownSpanIAL
                        ) && next.tokens_vec() == editor::CARET.as_bytes()
                        {
                            self.base.write_byte(b'\n');
                        }
                    }
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
            | NodeType::NodeLess
            | NodeType::NodeGreater
            | NodeType::NodeOpenBrace
            | NodeType::NodeCloseBrace
            | NodeType::NodeLinkSpace
            | NodeType::NodeLinkDest
            | NodeType::NodeLinkTitle => WalkStatus::WalkContinue,
            NodeType::NodeLinkText => {
                if entering && n.parent().map(|p| p.typ()) != Some(NodeType::NodeImage) {
                    self.base
                        .write(html_util::escape_html(&n.tokens_str()).as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough => self.auto_space(n, entering),
            NodeType::NodeStrikethrough1OpenMarker
            | NodeType::NodeStrikethrough1CloseMarker
            | NodeType::NodeStrikethrough2OpenMarker
            | NodeType::NodeStrikethrough2CloseMarker => {
                if entering {
                    if n.typ() == NodeType::NodeStrikethrough1OpenMarker
                        || n.typ() == NodeType::NodeStrikethrough2OpenMarker
                    {
                        self.tag("span", &[("data-type", "s")], false);
                    } else {
                        self.tag("/span", &[], false);
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTaskListItemMarker => self.render_task_list_item_marker(n, entering),
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead => self.render_table_head(n, entering),
            NodeType::NodeTableRow => {
                if entering {
                    self.tag("tr", &[], false);
                } else {
                    self.tag("/tr", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTableCell => self.render_table_cell(n, entering),
            NodeType::NodeEmoji | NodeType::NodeEmojiAlias => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                if entering {
                    self.base.write(&n.tokens_vec());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDefBlock => {
                if entering {
                    self.base.write_str("<div class=\"footnotes-defs-div\">");
                    self.base.write_str("<hr class=\"footnotes-defs-hr\" />\n");
                    self.base.write_str("<ol class=\"footnotes-defs-ol\">");
                } else {
                    self.base.write_str("</ol></div>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDef => {
                if entering {
                    // 脚注锚点由 lute::prepend_footnote_anchors 在渲染前插入，
                    // 与 Go 版在 renderFootnotesDef 中前置 NodeInlineHTML 的效果一致。
                    self.base.write_str("<li>");
                } else {
                    self.base.write_str("</li>\n");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesRef => self.render_footnotes_ref(n, entering),
            NodeType::NodeToC => WalkStatus::WalkContinue,
            NodeType::NodeBackslash => {
                if entering {
                    self.base.write_str("<span data-type=\"backslash\">");
                } else {
                    self.base.write_str("</span>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBackslashContent => {
                if entering {
                    self.base
                        .write(html_util::escape_html(&n.tokens_str()).as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.base
                        .write(html_util::escape_html(&n.tokens_str()).as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatter => {
                if entering {
                    self.base
                        .write_str("<div class=\"protyle-wysiwyg__block\" data-type=\"yaml-front-matter\" data-block=\"0\">");
                } else {
                    self.base.write_str("</div>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterOpenMarker | NodeType::NodeYamlFrontMatterCloseMarker => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                if entering {
                    let preview = n.tokens_vec();
                    let preview = trim_ascii_space(&preview);
                    self.tag("pre", &[], false);
                    self.tag("code", &[("data-type", "yaml-front-matter")], false);
                    if preview.is_empty()
                        || (preview.len() == editor::CARET.len()
                            && preview == editor::CARET.as_bytes())
                    {
                        self.base.write_str(editor::FRONT_END_CARET);
                        self.base.write_str("\n");
                    } else {
                        self.base.write(
                            html_util::escape_html(&String::from_utf8_lossy(preview)).as_bytes(),
                        );
                    }
                    self.base.write_str("</code></pre>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRef => self.render_block_ref(n, entering),
            NodeType::NodeBlockRefID
            | NodeType::NodeBlockRefSpace
            | NodeType::NodeBlockRefText
            | NodeType::NodeBlockRefDynamicText => WalkStatus::WalkContinue,
            NodeType::NodeFileAnnotationRef => self.render_file_annotation_ref(n, entering),
            NodeType::NodeFileAnnotationRefID
            | NodeType::NodeFileAnnotationRefSpace
            | NodeType::NodeFileAnnotationRefText => WalkStatus::WalkContinue,
            NodeType::NodeMark => self.auto_space(n, entering),
            NodeType::NodeMark1OpenMarker | NodeType::NodeMark2OpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "mark")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark1CloseMarker | NodeType::NodeMark2CloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSup => WalkStatus::WalkContinue,
            NodeType::NodeSupOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "sup")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSupCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSub => WalkStatus::WalkContinue,
            NodeType::NodeSubOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "sub")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSubCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownBlockIAL => WalkStatus::WalkContinue,
            NodeType::NodeKramdownSpanIAL => {
                if !entering {
                    if let (Some(prev), Some(next)) = (n.previous(), n.next()) {
                        if NodeType::NodeImage == prev.typ() && NodeType::NodeImage == next.typ() {
                            self.base.write_str(editor::ZWSP);
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockQueryEmbed => self.render_block_query_embed(n, entering),
            NodeType::NodeBlockQueryEmbedScript => WalkStatus::WalkContinue,
            NodeType::NodeTag => self.render_tag(n, entering),
            NodeType::NodeTagOpenMarker => {
                if entering {
                    let mut content = n.parent().map(|p| p.text()).unwrap_or_default();
                    content = content.replace(editor::CARET, "");
                    self.tag(
                        "span",
                        &[
                            ("data-type", "tag"),
                            ("data-content", &html_util::escape_html(&content)),
                        ],
                        false,
                    );
                    self.base.write_str(editor::WORD_JOINER);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTagCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                    self.base.write_str(editor::ZWSP);
                }
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
            NodeType::NodeLinkRefDef => WalkStatus::WalkContinue,
            NodeType::NodeSuperBlock => self.render_super_block(n, entering),
            NodeType::NodeSuperBlockOpenMarker
            | NodeType::NodeSuperBlockLayoutMarker
            | NodeType::NodeSuperBlockCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeGitConflict => WalkStatus::WalkContinue,
            NodeType::NodeGitConflictContent => {
                if entering {
                    let mut attrs = Vec::new();
                    self.block_node_attrs(n, &mut attrs, "git-conflict");
                    self.tag_owned("div", &attrs, false);
                    self.tag(
                        "div",
                        &[("contenteditable", "false"), ("spellcheck", "false")],
                        false,
                    );
                    let raw = n.tokens_vec();
                    let tokens = trim_ascii_space(&raw);
                    self.base
                        .write(html_util::escape_html(&String::from_utf8_lossy(tokens)).as_bytes());
                } else {
                    self.tag("/div", &[], false);
                    self.render_ial(n);
                    self.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGitConflictOpenMarker | NodeType::NodeGitConflictCloseMarker => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeIFrame => self.render_iframe(n, entering),
            NodeType::NodeWidget => self.render_widget(n, entering),
            NodeType::NodeVideo => self.render_video(n, entering),
            NodeType::NodeAudio => self.render_audio(n, entering),
            NodeType::NodeKbd => WalkStatus::WalkContinue,
            NodeType::NodeKbdOpenMarker => {
                if entering {
                    self.ensure_inline_boundary_zwsp();
                    self.tag("span", &[("data-type", "kbd")], false);
                    self.base.write_str(editor::WORD_JOINER);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKbdCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                    self.base.write_str(editor::ZWSP);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeUnderline => WalkStatus::WalkContinue,
            NodeType::NodeUnderlineOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "u")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeUnderlineCloseMarker => {
                if entering {
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBr => self.render_br(n, entering),
            NodeType::NodeTextMark => self.render_text_mark(n, entering),
            NodeType::NodeAttributeView => self.render_attribute_view(n, entering),
            NodeType::NodeCustomBlock => self.render_custom_block(n, entering),
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => self.render_tabs(n, entering),
            _ => WalkStatus::WalkContinue,
        }
    }

    fn auto_space(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.text_auto_space_previous(n);
        } else {
            self.base.text_auto_space_next(n);
        }
        WalkStatus::WalkContinue
    }

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if n != self.root && is_tab_title_block(n) {
            return WalkStatus::WalkSkipChildren;
        }
        if entering {
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "p");
            self.tag_owned("div", &attrs, false);
            let mut attrs: Vec<(String, String)> = Vec::new();
            if "true" == n.ial_attr("tabs-title") {
                attrs.push(("class".into(), "tab-item-title callout-title".into()));
            }
            self.contenteditable(n, &mut attrs);
            self.spellcheck(&mut attrs);
            self.tag_owned("div", &attrs, false);
        } else {
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let mut tokens = if (crate::CAP_AUTO_SPACE && self.base.options.auto_space)
            && n.parent().map(|p| p.typ()) != Some(NodeType::NodeKbd)
        {
            self.base.space(&n.tokens_vec())
        } else {
            n.tokens_vec()
        };

        let zwsp_space = format!(" {}{}", editor::ZWSP, "");
        if tokens.starts_with(zwsp_space.as_bytes())
            && self.base.out.ends_with(editor::ZWSP.as_bytes())
        {
            let mut replaced = Vec::new();
            replaced.extend_from_slice(b" ");
            replaced.extend_from_slice(&tokens[zwsp_space.len()..]);
            tokens = replaced;
        }

        if n.parent().map(|p| p.typ()) == Some(NodeType::NodeTextMark) {
            if n.parent().map(|p| p.tokens_str()) == Some("code".to_string()) {
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    tokens = replace_all(&tokens, b"\\|", b"|");
                }
                self.base
                    .write(html_util::escape_html(&String::from_utf8_lossy(&tokens)).as_bytes());
            } else {
                self.base.write(&tokens);
            }
        } else {
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            if n.parent_is(&[NodeType::NodeTableCell]) {
                self.base.write(escaped.replace('|', "&#124;").as_bytes());
            } else {
                self.base.write(escaped.as_bytes());
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                if let Some(text) = previous_node_text(n) {
                    if let Some(c) = text.chars().last() {
                        if c.is_alphanumeric() {
                            self.base.write_byte(b' ');
                        }
                    }
                }
            }
            self.ensure_inline_boundary_zwsp();
        } else if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
            if let Some(text) = next_node_text(n) {
                if let Some(c) = text.chars().next() {
                    if c.is_alphanumeric() {
                        self.base.write_byte(b' ');
                    }
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        let mut no_hl = false;
        let mut language = String::new();
        if let Some(first) = n.first_child() {
            if let Some(info) = first.next() {
                if !info.data().code_block_info.is_empty() {
                    language = String::from_utf8_lossy(&info.data().code_block_info).to_string();
                    language = language.replace(editor::CARET, "");
                    no_hl = language != "mindmap" && no_highlight(&language);
                }
            }
        }

        if entering {
            if no_hl {
                let Some(first) = n.first_child() else {
                    return WalkStatus::WalkContinue;
                };
                let Some(info) = first.next() else {
                    return WalkStatus::WalkContinue;
                };
                let Some(code) = info.next() else {
                    return WalkStatus::WalkContinue;
                };
                let mut attrs = Vec::new();
                self.block_node_attrs(n, &mut attrs, "render-node");
                let mut tokens = code.tokens_vec();
                tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
                let tokens = trim_ascii_space(&tokens).to_vec();
                let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
                attrs.push(("data-content".into(), escaped));
                attrs.push(("data-subtype".into(), language));
                self.tag_owned("div", &attrs, false);
                self.tag("div", &[("spin", "1")], false);
                self.tag("/div", &[], false);
                self.render_ial(n);
                self.tag("/div", &[], false);
                return WalkStatus::WalkSkipChildren;
            }
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "code-block");
            self.tag_owned("div", &attrs, false);
        } else {
            if no_hl {
                return WalkStatus::WalkSkipChildren;
            }
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_code(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        self.tag("div", &[("class", "protyle-action")], false);
        let code_len = n.tokens_vec().len();
        let code_is_empty = code_len < 1
            || (code_len == editor::CARET.len() && n.tokens_vec() == editor::CARET.as_bytes());
        let mut language = String::new();
        let mut caret_in_info = false;
        if let Some(prev) = n.previous() {
            caret_in_info = prev
                .data()
                .code_block_info
                .windows(editor::CARET.len())
                .any(|w| w == editor::CARET.as_bytes());
            if caret_in_info {
                let cleaned =
                    replace_all(&prev.data().code_block_info, editor::CARET.as_bytes(), b"");
                prev.data_mut().code_block_info = cleaned;
            }
        }
        let attrs = [
            ("class", "protyle-action--first protyle-action__language"),
            ("contenteditable", "false"),
        ];
        if let Some(prev) = n.previous() {
            if !prev.data().code_block_info.is_empty() {
                let info = String::from_utf8_lossy(&prev.data().code_block_info).to_string();
                language = info.split(' ').next().unwrap_or("").to_string();
            }
        }
        self.tag("span", &attrs, false);
        self.base.write_str(&language);
        self.tag("/span", &[], false);
        self.base.write_str("<span class=\"fn__flex-1\"></span>");
        self.tag(
            "span",
            &[
                (
                    "class",
                    "ariaLabel protyle-icon protyle-icon--first protyle-action__copy",
                ),
                ("data-position", "4north"),
            ],
            false,
        );
        self.base
            .write_str("<svg><use xlink:href=\"#iconCopy\"></use></svg>");
        self.tag("/span", &[], false);
        self.base.write_str(
            "<span class=\"ariaLabel protyle-icon protyle-icon--last protyle-action__menu\" data-position=\"4north\"><svg><use xlink:href=\"#iconMore\"></use></svg></span>",
        );
        self.tag("/div", &[], false);

        self.tag("div", &[("class", "hljs")], false);
        self.tag("div", &[], false);
        self.tag("/div", &[], false);
        let mut attrs: Vec<(String, String)> = Vec::new();
        self.contenteditable(n, &mut attrs);
        attrs.push(("style".into(), "flex: 1".into()));
        attrs.push(("spellcheck".into(), "false".into()));
        self.tag_owned("div", &attrs, false);
        if code_is_empty {
            if caret_in_info {
                self.base.write_str(editor::FRONT_END_CARET);
            }
        } else {
            let escaped = html_util::escape_html(&n.tokens_str());
            let escaped = escaped
                .replace("__@mark__", "<span data-type=\"search-mark\">")
                .replace("__mark@__", "</span>");
            self.base.write_str(&escaped);
        }
        self.tag("/div", &[], false);
        self.tag("/div", &[], false);
        WalkStatus::WalkContinue
    }

    fn render_math_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let Some(first) = n.first_child() else {
            return WalkStatus::WalkContinue;
        };
        let Some(content) = first.next() else {
            return WalkStatus::WalkContinue;
        };
        let mut attrs = Vec::new();
        self.block_node_attrs(n, &mut attrs, "render-node");
        let mut tokens = content.tokens_vec();
        tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
        let tokens = trim_ascii_space(&tokens).to_vec();
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        attrs.push(("data-content".into(), escaped));
        attrs.push(("data-subtype".into(), "math".into()));
        self.tag_owned("div", &attrs, false);
        self.tag("div", &[("spin", "1")], false);
        self.tag("/div", &[], false);
        self.render_ial(n);
        self.tag("/div", &[], false);
        WalkStatus::WalkContinue
    }

    fn render_inline_math(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                if let Some(text) = previous_node_text(n) {
                    if let Some(c) = text.chars().last() {
                        if c.is_alphanumeric() {
                            self.base.write_byte(b' ');
                        }
                    }
                }
            }
        } else if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
            if let Some(text) = next_node_text(n) {
                if let Some(c) = text.chars().next() {
                    if c.is_alphanumeric() {
                        self.base.write_byte(b' ');
                    }
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_br(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if n.parent_is(&[NodeType::NodeParagraph, NodeType::NodeHeading]) {
                self.base.write_byte(b'\n');
            } else {
                self.base.write_str("<br />");
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs = Vec::new();
            let mut level = n.data().heading_level;
            if 6 < level {
                level = 6;
                n.data_mut().heading_level = 6;
            }
            let level = level.max(1) as usize;
            let tag = format!("h{}", level);
            attrs.push(("data-subtype".into(), tag.clone()));
            self.block_node_attrs(n, &mut attrs, &tag);
            self.tag_owned("div", &attrs, false);
            let mut attrs: Vec<(String, String)> = Vec::new();
            self.contenteditable(n, &mut attrs);
            self.spellcheck(&mut attrs);
            self.tag_owned("div", &attrs, false);
        } else {
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_list_item(&mut self, n: Node, entering: bool) -> WalkStatus {
        let typ = n.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0);
        let num = n.data().list_data.as_ref().map(|d| d.num).unwrap_or(0);
        if entering {
            let mut class = if NodeType::NodeMindmapItem == n.typ() {
                "mindmap-item".to_string()
            } else {
                "li".to_string()
            };
            let mut attrs: Vec<(String, String)> = Vec::new();
            match typ {
                0 => {
                    attrs.push(("data-marker".into(), "*".into()));
                    attrs.push(("data-subtype".into(), "u".into()));
                }
                1 => {
                    attrs.push(("data-marker".into(), format!("{}.", num)));
                    attrs.push(("data-subtype".into(), "o".into()));
                }
                3 => {
                    attrs.push(("data-marker".into(), "*".into()));
                    attrs.push(("data-subtype".into(), "t".into()));
                    if let Some(first) = n.first_child() {
                        if NodeType::NodeTaskListItemMarker == first.typ() {
                            if first.data().task_list_item_checked {
                                class.push_str(" protyle-task--done");
                            }
                            if self.base.options.data_task {
                                attrs.push((
                                    "data-task".into(),
                                    first.effective_task_list_item_marker(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
            self.block_node_attrs(n, &mut attrs, &class);
            self.tag_owned("div", &attrs, false);

            let draggable = if self.base.options.protyle_contenteditable {
                "true"
            } else {
                "false"
            };
            if 0 == typ {
                self.tag(
                    "div",
                    &[("class", "protyle-action"), ("draggable", draggable)],
                    false,
                );
                self.base
                    .write_str("<svg><use xlink:href=\"#iconDot\"></use></svg>");
                self.tag("/div", &[], false);
            } else if 1 == typ {
                self.tag(
                    "div",
                    &[
                        ("class", "protyle-action protyle-action--order"),
                        ("contenteditable", "false"),
                        ("draggable", draggable),
                    ],
                    false,
                );
                self.base.write_str(&format!("{}.", num));
                self.tag("/div", &[], false);
            }
        } else {
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_task_list_item_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let draggable = if self.base.options.protyle_contenteditable {
            "true"
        } else {
            "false"
        };
        let checked = normalized_task_list_item_checked(&self.base.options, n);
        let icon = if checked {
            "#iconCheck"
        } else {
            "#iconUncheck"
        };
        self.base.write_str(&format!(
            "<div class=\"protyle-action protyle-action--task\" draggable=\"{}\"><svg><use xlink:href=\"{}\"></use></svg></div>",
            draggable, icon
        ));
        WalkStatus::WalkContinue
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if n.first_child().is_none() {
            return WalkStatus::WalkSkipChildren;
        }
        if entering {
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "table");
            self.tag_owned("div", &attrs, false);
            self.tag("div", &[("contenteditable", "false")], false);
            let mut attrs: Vec<(String, String)> = Vec::new();
            self.contenteditable(n, &mut attrs);
            self.spellcheck(&mut attrs);
            self.tag_owned("table", &attrs, false);
            let mut caption = n.ial_attr("caption");
            if !caption.is_empty() {
                if self.base.options.sanitize {
                    caption =
                        String::from_utf8_lossy(&super::sanitizer::sanitize(caption.as_bytes()))
                            .to_string();
                }
                self.base.write_str(&html_util::unescape_string(&caption));
            }
        } else {
            if n.first_child().and_then(|f| f.next()).is_some() {
                self.tag("/tbody", &[], false);
            }
            self.tag("/table", &[], false);
            self.base.write_str(
                "<div class=\"protyle-action__table\"><div class=\"table__resize\"></div><div class=\"table__select\"></div></div>",
            );
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_table_head(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("colgroup", &[], false);
            let colgroup = n
                .parent()
                .map(|p| p.ial_attr("colgroup"))
                .unwrap_or_default();
            if colgroup.is_empty() {
                if let Some(first) = n.first_child() {
                    let mut th = first.first_child();
                    while let Some(cell) = th {
                        if NodeType::NodeTableCell == cell.typ() {
                            let style = table_cell_width_style(&cell.ial_attr("style"));
                            if !style.is_empty() {
                                self.tag("col", &[("style", &style)], true);
                            } else {
                                self.tag("col", &[], true);
                            }
                        }
                        th = cell.next();
                    }
                }
            } else {
                for style in colgroup.split('|') {
                    if !style.is_empty() {
                        self.tag("col", &[("style", style)], true);
                    } else {
                        self.tag("col", &[], true);
                    }
                }
            }
            self.tag("/colgroup", &[], false);
            self.tag("thead", &[], false);
        } else {
            self.tag("/thead", &[], false);
            if n.next().is_some() {
                self.tag("tbody", &[], false);
            }
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
            self.span_node_attrs(n, &mut attrs);
            append_table_cell_align_style(n, &mut attrs);
            if let Some(rich) = n.data().table_cell_rich.as_ref() {
                attrs.push((
                    crate::ast::TABLE_CELL_RICH_ATTRIBUTE.to_string(),
                    rich.encode(),
                ));
                attrs.push(("contenteditable".into(), "false".into()));
            }
            self.tag_owned(tag, &attrs, false);
            if n.data().table_cell_rich.is_some() {
                self.tag(
                    "div",
                    &[("class", "table__cell-rich"), ("contenteditable", "false")],
                    false,
                );
                if let Some(content) = crate::render::table_rich::table_cell_rich_html(
                    n.data().table_cell_rich.as_ref().unwrap(),
                    &self.base.options,
                ) {
                    self.base.write(&content);
                }
                self.tag("/div", &[], false);
                return WalkStatus::WalkSkipChildren;
            }
        } else {
            self.tag(&format!("/{}", tag), &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_link(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) else {
                return WalkStatus::WalkContinue;
            };
            let mut dest_tokens = dest.tokens_vec();
            if self.base.options.sanitize {
                dest_tokens = trim_ascii_space(&dest_tokens).to_vec();
                dest_tokens = super::sanitizer::sanitize(&dest_tokens);
                let lower = String::from_utf8_lossy(&dest_tokens).to_lowercase();
                if lower.starts_with("javascript:") {
                    dest_tokens.clear();
                }
            }
            dest_tokens = self.base.link_path(&dest_tokens);
            let caret_in_dest = dest_tokens
                .windows(editor::CARET.len())
                .any(|w| w == editor::CARET.as_bytes());
            if caret_in_dest {
                if let Some(text) = n.child_by_type(NodeType::NodeLinkText) {
                    let mut tokens = text.tokens_vec();
                    tokens.extend_from_slice(editor::CARET.as_bytes());
                    text.set_tokens(tokens);
                }
                dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
            }
            let mut attrs: Vec<(String, String)> = vec![
                ("data-type".into(), "a".into()),
                (
                    "data-href".into(),
                    String::from_utf8_lossy(&dest_tokens).to_string(),
                ),
            ];
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.data().tokens.is_empty() {
                    attrs.push(("data-title".into(), escape_ref_text(&title.tokens_str())));
                }
            }
            self.tag_owned("span", &attrs, false);
        } else {
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_image(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let prev_is_caret = n
                .previous()
                .map(|p| p.text() == editor::CARET)
                .unwrap_or(true);
            let table_first = n.parent_is(&[NodeType::NodeTableCell])
                && n.previous().is_some()
                && n.previous().and_then(|p| p.previous()).is_none();
            if prev_is_caret || table_first {
                self.base.write_str(editor::ZWSP);
            }
            let mut attrs: Vec<(String, String)> = vec![
                ("contenteditable".into(), "false".into()),
                ("data-type".into(), "img".into()),
                ("class".into(), "img".into()),
            ];
            let mut parent_style = n.ial_attr("parent-style");
            if !parent_style.is_empty() {
                parent_style = parent_style.replace("display: block;", "");
                parent_style = parent_style.trim().to_string();
                if !parent_style.is_empty() {
                    attrs.push(("style".into(), parent_style));
                }
            }
            if self.base.last_out == b'\n' {
                self.base.write_str(editor::ZWSP);
            }
            self.tag_owned("span", &attrs, false);
            self.tag("span", &[], false);
            self.base.write_str(" ");
            self.tag("/span", &[], false);
            let mut attrs: Vec<(String, String)> = Vec::new();
            let style = n.ial_attr("style");
            if !style.is_empty() {
                let mut width = String::new();
                for s in style.split(';') {
                    if s.contains("width") {
                        width = s.to_string();
                        break;
                    }
                }
                width = width.replace("vw", "%").trim().to_string();
                if !width.is_empty() {
                    attrs.push(("style".into(), format!("{};", width)));
                }
            }
            self.tag_owned("span", &attrs, false);
            self.tag("span", &[("class", "protyle-action protyle-icons")], false);
            self.base.write_str(
                "<span class=\"protyle-icon protyle-icon--only\"><svg class=\"svg\"><use xlink:href=\"#iconMore\"></use></svg></span>",
            );
            self.tag("/span", &[], false);
        } else {
            let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) else {
                return WalkStatus::WalkContinue;
            };
            let mut dest_tokens = dest.tokens_vec();
            if self.base.options.sanitize {
                dest_tokens = super::sanitizer::sanitize(&dest_tokens);
            }
            dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
            let data_src = String::from_utf8_lossy(&dest_tokens).to_string();
            let src = String::from_utf8_lossy(&self.base.link_path(&dest_tokens)).to_string();
            let mut attrs: Vec<(String, String)> = vec![
                ("src".into(), src),
                ("data-src".into(), data_src),
                ("loading".into(), "lazy".into()),
            ];
            if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                if !alt.data().tokens.is_empty() {
                    attrs.push(("alt".into(), alt.tokens_str()));
                }
            }
            let mut title_tokens: Vec<u8> = Vec::new();
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.data().tokens.is_empty() {
                    title_tokens = escape_ref_text(&title.tokens_str()).into_bytes();
                    attrs.push((
                        "title".into(),
                        String::from_utf8_lossy(&title_tokens).to_string(),
                    ));
                }
            }
            let style = n.ial_attr("style");
            if !style.is_empty() {
                let mut width = String::new();
                for s in style.split(';') {
                    if s.contains("width") {
                        width = s.to_string();
                    }
                }
                let mut style = style.replace(&format!("{};", width), "");
                style = style.replace("flex: 0 0 auto;", "");
                style = style.replace("display: block;", "");
                style = style.trim().to_string();
                if !style.is_empty() {
                    attrs.push(("style".into(), style));
                }
            }
            self.tag_owned("img", &attrs, true);

            // 与 Go 版一致：对已写出的 <img ...> 片段做 sanitize/路径处理。
            let buf = std::mem::take(&mut self.base.out);
            let idx = find_last(&buf, b"<img src=").unwrap_or(0);
            let mut img_buf = buf[idx..].to_vec();
            let head = buf[..idx].to_vec();
            if self.base.options.sanitize {
                img_buf = super::sanitizer::sanitize(&img_buf);
            }
            img_buf = self.tag_src_path(&img_buf);
            self.base.out = head;
            self.base.write(&img_buf);

            self.tag("span", &[("class", "protyle-action__drag")], false);
            self.tag("/span", &[], false);

            if self.base.options.protyle_mark_net_img
                && !dest_tokens.starts_with(b"assets/")
                && !dest_tokens.starts_with(b"api/icon/")
            {
                self.base.write_str(
                    "<span class=\"img__net\"><svg><use xlink:href=\"#iconLanguage\"></use></svg></span>",
                );
            }

            self.tag("span", &[("class", "protyle-action__title")], false);
            self.tag("span", &[], false);
            self.base.write(&title_tokens);
            self.tag("/span", &[], false);
            self.tag("/span", &[], false);
            self.tag("/span", &[], false);
            self.tag("span", &[], false);
            self.base.write_str(" ");
            self.tag("/span", &[], false);
            self.tag("/span", &[], false);

            let next_is_caret = n
                .next()
                .map(|x| x.text() == editor::CARET || NodeType::NodeImage == x.typ())
                .unwrap_or(true);
            if next_is_caret {
                self.base.write_str(editor::ZWSP);
                return WalkStatus::WalkContinue;
            }
            if let Some(next) = n.next() {
                if NodeType::NodeKramdownSpanIAL == next.typ()
                    && next
                        .next()
                        .map(|x| x.text() == editor::CARET)
                        .unwrap_or(true)
                {
                    self.base.write_str(editor::ZWSP);
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let mut attrs = Vec::new();
        self.block_node_attrs(n, &mut attrs, "render-node");
        let mut tokens = n.tokens_vec();
        tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
        attrs.push(("data-subtype".into(), "block".into()));
        self.tag_owned("div", &attrs, false);
        self.base.write_str("<div class=\"protyle-icons\">");
        self.base.write_str(
            "<span class=\"ariaLabel protyle-icon protyle-icon--first protyle-action__edit\" data-position=\"4north\"><svg><use xlink:href=\"#iconEdit\"></use></svg></span><span class=\"ariaLabel protyle-icon protyle-action__menu protyle-icon--last\" data-position=\"4north\"><svg><use xlink:href=\"#iconMore\"></use></svg></span>",
        );
        self.base.write_str("</div><div>");
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.tag("protyle-html", &[("data-content", &escaped)], false);
        self.tag("/protyle-html", &[], false);
        self.base.write_str("<span style=\"position: absolute\">");
        self.base.write_str(editor::ZWSP);
        self.base.write_str("</span>");
        self.base.write_str("</div>");
        self.render_ial(n);
        self.tag("/div", &[], false);
        WalkStatus::WalkContinue
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
        if tokens == b"<u>" || tokens == b"</u>" {
            self.base.write(&tokens);
            return WalkStatus::WalkContinue;
        }
        n.data_mut().typ = NodeType::NodeHTMLBlock;
        self.render_html(n, entering)
    }

    fn render_callout(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs: Vec<(String, String)> = vec![(
                "data-subtype".into(),
                html_util::escape_html(&n.data().callout_type),
            )];
            self.block_node_attrs(n, &mut attrs, "callout");
            self.tag_owned("div", &attrs, false);
            self.base.write_str(
                "<div class=\"callout-info\" contenteditable=\"false\"><span class=\"callout-icon\">",
            );
            match n.data().callout_icon_type {
                0 => self.base.write_str(&n.data().callout_icon.clone()),
                1 => {
                    self.base.write_str("<img class=\"callout-img\" src=\"");
                    self.base
                        .write_str(&html_util::escape_html(&n.data().callout_icon));
                    self.base.write_str("\" />");
                }
                _ => {}
            }
            self.base
                .write_str("</span><span contenteditable=\"true\" spellcheck=\"");
            self.base.write_str(if self.base.options.spellcheck {
                "true"
            } else {
                "false"
            });
            self.base.write_str("\" class=\"callout-title\">");
            let title = callout_display_title(n);
            let title_tree = crate::parse::inline("", title.as_bytes(), self.parse_options.clone());
            if let Some(first) = title_tree.root_raw().first_child() {
                let mut inlines: Vec<Node> = Vec::new();
                let mut c = first.first_child();
                while let Some(cur) = c {
                    inlines.push(cur);
                    c = cur.next();
                }
                first.unlink();
                for inline in inlines {
                    title_tree.root_raw().append_child(inline);
                }
                let mut r = ProtyleRenderer::new(
                    &title_tree,
                    self.base.options.clone(),
                    self.parse_options.clone(),
                );
                let data = r.render();
                self.base.write(&data);
            } else {
                self.base.write_str(&title);
            }
            self.base.write_str("</span></div>");
            self.base.write_str("<div class=\"callout-content\">");
        } else {
            self.base.write_str("</div>");
            self.render_ial(n);
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    /// Corresponds to ProtyleRenderer.renderTabs in render/tabs.go. Title blocks
    /// render through their own root so the normal traversal can skip them.
    fn render_tabs(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs = Vec::new();
            let is_item = n.typ() == NodeType::NodeTabItem;
            self.block_node_attrs(n, &mut attrs, if is_item { "tab-item" } else { "tabs" });
            self.tag_owned("div", &attrs, false);
            if !is_item {
                self.base.write_str(
                    "<div class=\"tabs-header protyle-action\" contenteditable=\"false\"></div>",
                );
            } else {
                self.base.write_str(
                    "<div class=\"tab-item-info callout-info\" contenteditable=\"false\">",
                );
                let title_block = n
                    .children()
                    .find(|c| c.typ() != NodeType::NodeKramdownBlockIAL)
                    .filter(|c| {
                        c.typ() == NodeType::NodeParagraph && c.ial_attr("tabs-title") == "true"
                    });
                if let Some(title) = title_block {
                    let tree = Tree::new();
                    let mut renderer = ProtyleRenderer::new(
                        &tree,
                        self.base.options.clone(),
                        self.parse_options.clone(),
                    );
                    renderer.root = title;
                    renderer._owner = title.owner();
                    self.base.write(&renderer.render());
                } else {
                    self.base.write_str("<span class=\"tab-item-title callout-title\" contenteditable=\"true\" spellcheck=\"");
                    self.base.write_str(if self.base.options.spellcheck {
                        "true"
                    } else {
                        "false"
                    });
                    self.base.write_str("\">");
                    let tree = crate::parse::inline(
                        "",
                        n.data().tab_item_title.as_bytes(),
                        self.parse_options.clone(),
                    );
                    if let Some(paragraph) = tree.root_raw().first_child() {
                        let inlines: Vec<_> = paragraph.children().collect();
                        paragraph.unlink();
                        for inline in inlines {
                            tree.root_raw().append_child(inline);
                        }
                    }
                    let mut renderer = ProtyleRenderer::new(
                        &tree,
                        self.base.options.clone(),
                        self.parse_options.clone(),
                    );
                    self.base.write(&renderer.render());
                    self.base.write_str("</span>");
                }
                self.base
                    .write_str("</div><div class=\"tab-item-content\">");
            }
        } else {
            if n.typ() == NodeType::NodeTabItem {
                self.base.write_str("</div>");
            }
            self.render_ial(n);
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_custom_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs: Vec<(String, String)> = vec![("contenteditable".into(), "false".into())];
            attrs.push((
                "data-info".into(),
                html_util::escape_html_str(&n.data().custom_block_info),
            ));
            attrs.push((
                "data-content".into(),
                html_util::escape_html_str(&n.tokens_str()),
            ));
            self.block_node_attrs(n, &mut attrs, "custom-block");
            self.tag_owned("div", &attrs, false);
            self.tag("div", &[("class", "custom-block__content")], false);
            self.tag("pre", &[], false);
            self.base
                .write_str(&html_util::escape_html(&n.tokens_str()));
            self.tag("/pre", &[], false);
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_attribute_view(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs: Vec<(String, String)> = vec![
                ("contenteditable".into(), "false".into()),
                ("data-av-id".into(), n.data().attribute_view_id.clone()),
                ("data-av-type".into(), n.data().attribute_view_type.clone()),
            ];
            self.block_node_attrs(n, &mut attrs, "av");
            self.tag_owned("div", &attrs, false);
            let mut attrs: Vec<(String, String)> = Vec::new();
            self.spellcheck(&mut attrs);
            self.tag_owned("div", &attrs, false);
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_block_query_embed(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let Some(script) = n.child_by_type(NodeType::NodeBlockQueryEmbedScript) else {
                return WalkStatus::WalkContinue;
            };
            let tokens = replace_all(&script.tokens_vec(), editor::CARET.as_bytes(), b"");
            let content = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            let content = content.replace(editor::IAL_VAL_ESC_NEWLINE, "\n");
            let mut attrs: Vec<(String, String)> = vec![("data-content".into(), content)];
            self.block_node_attrs(n, &mut attrs, "render-node");
            self.tag_owned("div", &attrs, false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_super_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let Some(first) = n.first_child() else {
                return WalkStatus::WalkContinue;
            };
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "sb");
            let mut layout = first.next().map(|x| x.tokens_str()).unwrap_or_default();
            if layout.is_empty() {
                layout = "row".to_string();
            }
            attrs.push(("data-sb-layout".into(), layout.clone()));
            self.tag_owned("div", &attrs, false);

            if layout == "col" {
                let mut block_count = 0;
                let mut c = first.next().and_then(|x| x.next());
                while let Some(cur) = c {
                    if NodeType::NodeSuperBlockCloseMarker == cur.typ() {
                        break;
                    }
                    if NodeType::NodeKramdownBlockIAL != cur.typ() {
                        block_count += 1;
                    }
                    c = cur.next();
                }
                let mut block_idx = 0;
                let mut c = first.next().and_then(|x| x.next());
                while let Some(cur) = c {
                    if NodeType::NodeSuperBlockCloseMarker == cur.typ() {
                        break;
                    }
                    let me = &mut *self;
                    crate::ast::walk_raw(cur, &mut |nn, entering| me.render_node(nn, entering));
                    if NodeType::NodeKramdownBlockIAL != cur.typ() {
                        if block_idx < block_count - 1 {
                            self.tag(
                                "span",
                                &[("class", "sb__resize"), ("contenteditable", "false")],
                                false,
                            );
                            self.tag("/span", &[], false);
                        }
                        block_idx += 1;
                    }
                    c = cur.next();
                }
                return WalkStatus::WalkSkipChildren;
            }
        } else {
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_tag(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.text_auto_space_previous(n);
            let prev_is_tag = n
                .previous()
                .map(|p| NodeType::NodeTag == p.typ())
                .unwrap_or(false);
            let prev_is_caret_before_tag = n
                .previous()
                .map(|p| {
                    p.tokens_str() == editor::CARET
                        && p.previous()
                            .map(|pp| NodeType::NodeTag == pp.typ())
                            .unwrap_or(false)
                })
                .unwrap_or(false);
            if prev_is_tag || prev_is_caret_before_tag {
                self.base.write_byte(b' ');
            }
            self.ensure_inline_boundary_zwsp();
        } else {
            self.base.text_auto_space_next(n);
        }
        WalkStatus::WalkContinue
    }

    fn render_text_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        let contains = |types: &[&str]| -> bool {
            let d = n.data();
            let set: Vec<&str> = d.text_mark_type.split(' ').collect();
            types.iter().any(|t| set.contains(t))
        };
        if entering {
            if contains(&["code", "inline-math", "kbd"]) {
                if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                    if let Some(text) = previous_node_text(n) {
                        if let Some(c) = text.chars().last() {
                            if c.is_alphanumeric() {
                                self.base.write_byte(b' ');
                            }
                        }
                    }
                }
            } else {
                self.base.text_auto_space_previous(n);
            }
            let mut attrs = self.render_text_mark_attrs(n);
            self.span_node_attrs(n, &mut attrs);
            if contains(&["tag"]) {
                if let Some(prev) = n.previous() {
                    let prev_is_tag = prev.data().text_mark_type.split(' ').any(|t| t == "tag");
                    let prev_is_caret_before_tag = prev.tokens_str() == editor::CARET
                        && prev
                            .previous()
                            .map(|pp| pp.data().text_mark_type.split(' ').any(|t| t == "tag"))
                            .unwrap_or(false);
                    if prev_is_tag || prev_is_caret_before_tag {
                        self.base.write_byte(b' ');
                    }
                }
            }
            if contains(&["code", "kbd", "tag"]) {
                self.ensure_inline_boundary_zwsp();
            }
            if n.data().text_mark_type == "code" && self.base.options.spellcheck {
                attrs.push(("spellcheck".into(), "false".into()));
            }
            self.tag_owned("span", &attrs, false);
            if contains(&["code", "kbd", "tag"]) {
                self.base.write_str(editor::WORD_JOINER);
            }
            let mut text = n.data().text_mark_text_content.clone();
            if n.parent_is(&[NodeType::NodeTableCell]) {
                if n.data().text_mark_type == "code" {
                    text = text.replace('|', "&#124;");
                } else {
                    text = text.replace("\\|", "|");
                }
                text = text.replace('\n', "<br />");
            }
            self.base.write_str(&text);
        } else {
            self.base.write_str("</span>");
            if contains(&["code", "kbd", "tag"]) {
                match next_node_text(n) {
                    Some(text) if !text.starts_with(editor::ZWSP) => {
                        self.base.write_str(editor::ZWSP)
                    }
                    Some(_) => {}
                    None => self.base.write_str(editor::ZWSP),
                }
            }
            if contains(&["code", "inline-math", "kbd"]) {
                if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                    if let Some(text) = next_node_text(n) {
                        if let Some(c) = text.chars().next() {
                            if c.is_alphanumeric() {
                                self.base.write_byte(b' ');
                            }
                        }
                    }
                }
            } else {
                self.base.text_auto_space_next(n);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_footnotes_ref(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let Some((idx, _def)) = self.find_footnotes_def(&n.tokens_vec()) else {
            return WalkStatus::WalkContinue;
        };
        let idx_str = idx.to_string();
        let id = format!("footnotes-ref-{}", n.data().footnotes_ref_id);
        self.tag(
            "sup",
            &[("class", "footnotes-ref"), ("id", id.as_str())],
            false,
        );
        let href = format!("{}#footnotes-def-{}", self.base.options.link_base, idx_str);
        self.tag("a", &[("href", href.as_str())], false);
        self.base.write_str(&idx_str);
        self.tag("/a", &[], false);
        self.tag("/sup", &[], false);
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

    fn render_block_ref(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let Some(id_node) = n.child_by_type(NodeType::NodeBlockRefID) else {
            return WalkStatus::WalkContinue;
        };
        let mut subtype = "s";
        let mut ref_text = String::new();
        if let Some(text_node) = n.child_by_type(NodeType::NodeBlockRefText) {
            ref_text = text_node.text();
        } else if let Some(text_node) = n.child_by_type(NodeType::NodeBlockRefDynamicText) {
            ref_text = text_node.text();
            subtype = "d";
        }
        ref_text = escape_ref_text(&ref_text);
        let attrs = [
            ("data-type", "block-ref"),
            ("data-subtype", subtype),
            ("data-id", &id_node.tokens_str()),
        ];
        self.tag("span", &attrs, false);
        let ref_text = ref_text.replace("&amp;#124;", "|");
        self.base.write_str(&ref_text);
        self.tag("/span", &[], false);
        WalkStatus::WalkSkipChildren
    }

    fn render_file_annotation_ref(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let id = n
            .child_by_type(NodeType::NodeFileAnnotationRefID)
            .map(|x| x.tokens_str())
            .unwrap_or_default();
        let mut ref_text = id.clone();
        if let Some(text_node) = n.child_by_type(NodeType::NodeFileAnnotationRefText) {
            ref_text = text_node.text();
        }
        let ref_text = escape_ref_text(&ref_text);
        let escaped_id = html_util::escape_html(&id);
        self.tag(
            "span",
            &[
                ("data-type", "file-annotation-ref"),
                ("data-subtype", "s"),
                ("data-id", &escaped_id),
            ],
            false,
        );
        self.base.write_str(&ref_text);
        self.tag("/span", &[], false);
        WalkStatus::WalkSkipChildren
    }

    fn render_iframe(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.render_media(n, entering, "iframe")
    }

    fn render_widget(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.render_media(n, entering, "widget")
    }

    fn render_video(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "iframe");
            self.tag_owned("div", &attrs, false);
            self.tag("div", &[("class", "iframe-content")], false);
            self.base.write_str(editor::ZWSP);
            let mut tokens = replace_all(&n.tokens_vec(), editor::CARET.as_bytes(), b"");
            if self.base.options.sanitize {
                tokens = super::sanitizer::sanitize(&tokens);
            }
            tokens = self.media_controls(&tokens, "video");
            self.base.write(&tokens);
        } else {
            self.tag(
                "span",
                &[
                    ("class", "protyle-action__drag"),
                    ("contenteditable", "false"),
                ],
                false,
            );
            self.tag("/span", &[], false);
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_audio(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "iframe");
            self.tag_owned("div", &attrs, false);
            self.tag("div", &[("class", "iframe-content")], false);
            let mut tokens = replace_all(&n.tokens_vec(), editor::CARET.as_bytes(), b"");
            if self.base.options.sanitize {
                tokens = super::sanitizer::sanitize(&tokens);
            }
            tokens = self.media_controls(&tokens, "audio");
            self.base.write(&tokens);
            self.base.write_str(editor::ZWSP);
        } else {
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_media(&mut self, n: Node, entering: bool, kind: &str) -> WalkStatus {
        if entering {
            let mut tokens = replace_all(&n.tokens_vec(), editor::CARET.as_bytes(), b"");
            if self.base.options.sanitize {
                tokens = super::sanitizer::sanitize(&tokens);
            }
            let data_src = self.tag_src(&tokens);
            let src = self.base.link_path(&data_src);
            tokens = self.replace_src(&tokens, &String::from_utf8_lossy(&src));
            let mut attrs = Vec::new();
            self.block_node_attrs(n, &mut attrs, "iframe");
            let old_style = tokens_style(&tokens);
            if n.ial_attr("style").is_empty() && !old_style.is_empty() {
                let os = old_style.replace("pointer-events: none;", "");
                attrs.push(("style".into(), os));
            }
            let style_attr = format!("style=\"{}\"", old_style);
            tokens = replace_all(&tokens, style_attr.as_bytes(), b"");
            if kind == "widget" {
                attrs.push(("data-subtype".into(), "widget".into()));
            } else {
                attrs.push(("loading".into(), "lazy".into()));
            }
            self.tag_owned("div", &attrs, false);
            self.tag("div", &[("class", "iframe-content")], false);
            self.base.write(&tokens);
        } else {
            self.tag(
                "span",
                &[
                    ("class", "protyle-action__drag"),
                    ("contenteditable", "false"),
                ],
                false,
            );
            self.tag("/span", &[], false);
            self.tag("/div", &[], false);
            self.render_ial(n);
            self.tag("/div", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn media_controls(&self, tokens: &[u8], tag: &str) -> Vec<u8> {
        let text = String::from_utf8_lossy(tokens).to_string();
        let mut dom = crate::dom::parse(&text);
        let root = dom.root;
        let Some(first) = dom.nodes[root].children.first().copied() else {
            return tokens.to_vec();
        };
        if dom.nodes[first].data != tag {
            return tokens.to_vec();
        }
        let mut has_controls = false;
        let mut has_src = false;
        for (k, _) in &dom.nodes[first].attrs {
            has_controls = has_controls || k == "controls";
            has_src = has_src || k == "src";
        }
        if !has_controls {
            dom.nodes[first]
                .attrs
                .push(("controls".into(), "controls".into()));
        }
        if has_src {
            let src = dom.attr(first, "src");
            let path = String::from_utf8_lossy(&self.base.link_path(src.as_bytes())).to_string();
            dom.set_attr(first, "src", &path);
        }
        dom.html(first).into_bytes()
    }

    fn tag_src(&self, tokens: &[u8]) -> Vec<u8> {
        let text = String::from_utf8_lossy(tokens).to_string();
        let dom = crate::dom::parse(&text);
        let root = dom.root;
        if let Some(first) = dom.nodes[root].children.first().copied() {
            return dom.attr(first, "src").into_bytes();
        }
        Vec::new()
    }

    fn replace_src(&self, tokens: &[u8], src: &str) -> Vec<u8> {
        let text = String::from_utf8_lossy(tokens).to_string();
        let mut dom = crate::dom::parse(&text);
        let root = dom.root;
        if let Some(first) = dom.nodes[root].children.first().copied() {
            dom.set_attr(first, "src", src);
            return dom.html(first).into_bytes();
        }
        tokens.to_vec()
    }

    fn tag_src_path(&self, tokens: &[u8]) -> Vec<u8> {
        // 与 Go 版一致：只做字节替换，不能重新序列化 DOM，否则 <img ... /> 会变成 </img>。
        let marker = b"src=\"";
        let Some(src_index) = tokens.windows(marker.len()).position(|w| w == marker) else {
            return tokens.to_vec();
        };
        if 0 == src_index {
            return tokens.to_vec();
        }
        let src = &tokens[src_index + marker.len()..];
        let without_caret = replace_all(src, editor::CARET.as_bytes(), b"");
        if without_caret.is_empty() {
            return tokens.to_vec();
        }
        let target_src = self.base.link_path(src);
        let mut origin_src = String::from_utf8_lossy(&target_src).to_string();
        if target_src.starts_with(b"//") {
            origin_src = format!("https:{}", origin_src);
        }
        replace_all(tokens, src, origin_src.as_bytes())
    }

    fn render_ial(&mut self, n: Node) {
        self.tag(
            "div",
            &[("class", "protyle-attr"), ("contenteditable", "false")],
            false,
        );
        let attrs_of =
            |name: &str| -> String { n.ial_attr(name).replace(editor::IAL_VAL_ESC_NEWLINE, "\n") };

        let bookmark = attrs_of("bookmark");
        if !bookmark.is_empty() {
            self.tag("div", &[("class", "protyle-attr--bookmark")], false);
            self.base.write_str(&html_util::escape_html(&bookmark));
            self.tag("/div", &[], false);
        }
        let name = attrs_of("name");
        if !name.is_empty() {
            self.tag("div", &[("class", "protyle-attr--name")], false);
            self.base
                .write_str("<svg><use xlink:href=\"#iconN\"></use></svg>");
            self.base.write_str(&html_util::escape_html(&name));
            self.tag("/div", &[], false);
        }
        let alias = attrs_of("alias");
        if !alias.is_empty() {
            self.tag("div", &[("class", "protyle-attr--alias")], false);
            self.base
                .write_str("<svg><use xlink:href=\"#iconA\"></use></svg>");
            self.base.write_str(&html_util::escape_html(&alias));
            self.tag("/div", &[], false);
        }
        let memo = attrs_of("memo");
        if !memo.is_empty() {
            let memo = html_util::escape_html(&memo);
            self.tag(
                "div",
                &[
                    ("class", "protyle-attr--memo ariaLabel"),
                    ("aria-label", &memo),
                    ("data-position", "4north"),
                ],
                false,
            );
            self.base
                .write_str("<svg><use xlink:href=\"#iconM\"></use></svg>");
            self.tag("/div", &[], false);
        }
        let avs = attrs_of("custom-avs");
        if !avs.is_empty() {
            self.tag("div", &[("class", "protyle-attr--av")], false);
            self.base
                .write_str("<svg><use xlink:href=\"#iconDatabase\"></use></svg>");
            self.base.write_str(&n.ial_attr("av-names"));
            self.tag("/div", &[], false);
        }
        let refcount = attrs_of("refcount");
        if !refcount.is_empty() {
            self.tag(
                "div",
                &[("class", "protyle-attr--refcount popover__block")],
                false,
            );
            self.base.write_str(&html_util::escape_html(&refcount));
            self.tag("/div", &[], false);
        }
        self.base.write_str(editor::ZWSP);
        self.tag("/div", &[], false);
    }

    fn render_text_mark_attrs(&mut self, n: Node) -> Vec<(String, String)> {
        let mut attrs: Vec<(String, String)> =
            vec![("data-type".into(), n.data().text_mark_type.clone())];
        if !n.data().text_mark_flashcard_occlusion_id.is_empty() {
            attrs.push((
                "data-occlusion-id".into(),
                n.data().text_mark_flashcard_occlusion_id.clone(),
            ));
        }
        let types: Vec<String> = n
            .data()
            .text_mark_type
            .split(' ')
            .map(|s| s.to_string())
            .collect();
        for typ in types {
            if typ == "block-ref" {
                attrs.push((
                    "data-subtype".into(),
                    n.data().text_mark_block_ref_subtype.clone(),
                ));
                attrs.push(("data-id".into(), n.data().text_mark_block_ref_id.clone()));
                let cleaned = n.data().text_mark_text_content.replace(editor::ZWSP, "");
                if cleaned.trim().is_empty() {
                    n.data_mut().text_mark_text_content = n.data().text_mark_block_ref_id.clone();
                }
            } else if typ == "a" {
                let mut href = n.data().text_mark_a_href.clone();
                if self.base.options.sanitize && href.to_lowercase().starts_with("javascript:") {
                    href.clear();
                }
                let mut href =
                    String::from_utf8_lossy(&self.base.link_path(href.as_bytes())).to_string();
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    href = href.replace("\\|", "|");
                }
                href = href.replace('"', "&amp;quot;");
                attrs.push(("data-href".into(), href));
                let mut title = n.data().text_mark_a_title.clone();
                if !title.is_empty() {
                    title = title.replace('"', "&amp;quot;");
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        title = title.replace("\\|", "|");
                    }
                    attrs.push(("data-title".into(), title));
                }
            } else if typ == "inline-math" {
                attrs.push(("data-subtype".into(), "math".into()));
                let mut content = n.data().text_mark_inline_math_content.clone();
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    content = content.replace('|', "&#124;").replace('\n', "<br/>");
                }
                content = content.replace(editor::IAL_VAL_ESC_NEWLINE, "\n");
                content = content.replace('"', "&amp;quot;");
                attrs.push(("data-content".into(), content));
                attrs.push(("contenteditable".into(), "false".into()));
                attrs.push(("class".into(), "render-node".into()));
            } else if typ == "file-annotation-ref" {
                attrs.push((
                    "data-id".into(),
                    html_util::escape_html(&n.data().text_mark_file_annotation_ref_id),
                ));
            } else if typ == "inline-memo" {
                let content = n
                    .data()
                    .text_mark_inline_memo_content
                    .replace(editor::IAL_VAL_ESC_NEWLINE, "\n");
                attrs.push(("data-inline-memo-content".into(), content));
            }
        }
        attrs
    }

    fn ensure_inline_boundary_zwsp(&mut self) {
        if !self.base.out.ends_with(editor::ZWSP.as_bytes()) {
            self.base.write_str(editor::ZWSP);
        }
    }

    fn block_node_attrs(&mut self, n: Node, attrs: &mut Vec<(String, String)>, class: &str) {
        self.node_id(n, attrs);
        self.node_index(n, attrs);
        attrs.push(("data-type".into(), n.typ().as_str().to_string()));
        attrs.push(("class".into(), class.to_string()));
        for (k, v) in &n.data().kramdown_ial {
            if k == "id" {
                continue;
            }
            attrs.push((k.clone(), v.replace(editor::IAL_VAL_ESC_NEWLINE, "\n")));
        }
    }

    fn span_node_attrs(&self, n: Node, attrs: &mut Vec<(String, String)>) {
        for (k, v) in &n.data().kramdown_ial {
            attrs.push((k.clone(), v.clone()));
        }
    }

    fn node_id(&mut self, n: Node, attrs: &mut Vec<(String, String)>) {
        attrs.push(("data-node-id".into(), node_id(n)));
    }

    fn node_index(&mut self, n: Node, attrs: &mut Vec<(String, String)>) {
        if n.parent().map(|p| p.typ()) != Some(NodeType::NodeDocument) {
            return;
        }
        attrs.push(("data-node-index".into(), self.node_index.to_string()));
        self.node_index += 1;
    }

    fn spellcheck(&self, attrs: &mut Vec<(String, String)>) {
        attrs.push((
            "spellcheck".into(),
            if self.base.options.spellcheck {
                "true"
            } else {
                "false"
            }
            .to_string(),
        ));
    }

    fn contenteditable(&self, n: Node, attrs: &mut Vec<(String, String)>) {
        let ce = n.ial_attr("contenteditable");
        if !ce.is_empty() {
            attrs.push(("contenteditable".into(), ce));
        } else {
            attrs.push((
                "contenteditable".into(),
                if self.base.options.protyle_contenteditable {
                    "true"
                } else {
                    "false"
                }
                .to_string(),
            ));
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
}

impl Renderer for ProtyleRenderer {
    fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);
        let root = self.root;
        let me = &mut *self;
        crate::ast::walk_raw(root, &mut |n, entering| me.render_node(n, entering));
        std::mem::take(&mut self.base.out)
    }
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

fn is_tab_title_block(n: Node) -> bool {
    let Some(parent) = n.parent() else {
        return false;
    };
    if NodeType::NodeTabItem != parent.typ() {
        return false;
    }
    let mut child = parent.first_child();
    while let Some(c) = child {
        if NodeType::NodeKramdownBlockIAL == c.typ() {
            child = c.next();
            continue;
        }
        return n == c && NodeType::NodeParagraph == c.typ() && "true" == c.ial_attr("tabs-title");
    }
    false
}

fn append_table_cell_align_style(node: Node, attrs: &mut Vec<(String, String)>) {
    let align = match node.data().table_cell_align {
        1 => "left",
        2 => "center",
        3 => "right",
        _ => return,
    };
    for (k, v) in attrs.iter_mut() {
        if k != "style" {
            continue;
        }
        for declaration in v.split(';') {
            if let Some((property, _)) = declaration.split_once(':') {
                if property.trim().eq_ignore_ascii_case("text-align") {
                    return;
                }
            }
        }
        let mut style = v.trim().to_string();
        if !style.is_empty() && !style.ends_with(';') {
            style.push(';');
        }
        if !style.is_empty() {
            style.push(' ');
        }
        *v = format!("{style}text-align: {align};");
        return;
    }
    attrs.push(("style".to_string(), format!("text-align: {align};")));
}

fn table_cell_width_style(style: &str) -> String {
    let mut ret = String::new();
    for declaration in style.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        if let Some((property, _)) = declaration.split_once(':') {
            let property = property.trim();
            if property == "width" || property.ends_with("-width") {
                ret.push_str(declaration);
                ret.push(';');
            }
        }
    }
    ret
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

fn escape_ref_text(text: &str) -> String {
    text.replace('>', "&gt;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn normalized_task_list_item_checked(options: &RenderOptions, n: Node) -> bool {
    let mut marker = n.effective_task_list_item_marker();
    if options.export_normalize_task_list_marker && marker != " " && marker != "X" {
        marker = "X".to_string();
    }
    let marker = html_util::unescape_string(&marker);
    marker.as_bytes().first().copied().unwrap_or(b' ') != b' '
}

fn node_id(n: Node) -> String {
    for (k, v) in &n.data().kramdown_ial {
        if k == "id" {
            return v.clone();
        }
    }
    crate::ast::new_node_id()
}

fn previous_node_text(n: Node) -> Option<String> {
    let mut previous = n.previous()?;
    if previous.typ() == NodeType::NodeKramdownSpanIAL {
        previous = previous.previous()?;
    }
    Some(previous.text())
}

fn next_node_text(n: Node) -> Option<String> {
    n.next().map(Node::text)
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

fn trim_ascii_space(buf: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = buf.len();
    while start < end && matches!(buf[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    while end > start && matches!(buf[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    &buf[start..end]
}

fn find_last(buf: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > buf.len() {
        return None;
    }
    (0..=buf.len() - needle.len())
        .rev()
        .find(|&i| &buf[i..i + needle.len()] == needle)
}

fn tokens_style(tokens: &[u8]) -> String {
    let text = String::from_utf8_lossy(tokens);
    let Some(idx) = text.find("style=") else {
        return String::new();
    };
    let rest = &text[idx + 6..];
    let Some(quote) = rest.chars().next() else {
        return String::new();
    };
    let rest = &rest[quote.len_utf8()..];
    match rest.find(quote) {
        Some(end) => rest[..end].to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod source_neighbor_tests {
    use super::*;

    #[test]
    fn inline_spacing_stops_at_linebreaks_and_only_skips_previous_ial() {
        let mut tree = Tree::new();
        let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
        tree.root_raw().append_child(paragraph);
        let left = tree.new_node_raw(NodeType::NodeTextMark);
        left.data_mut().text_mark_type = "code".into();
        left.data_mut().text_mark_text_content = "left".into();
        let br = tree.new_node_raw(NodeType::NodeSoftBreak);
        let right = tree.new_node_raw(NodeType::NodeTextMark);
        right.data_mut().text_mark_type = "code".into();
        right.data_mut().text_mark_text_content = "right".into();
        paragraph.append_child(left);
        paragraph.append_child(br);
        paragraph.append_child(right);
        assert_eq!(previous_node_text(right), Some(String::new()));
        assert_eq!(next_node_text(left), Some(String::new()));
        let ial = tree.new_node_raw(NodeType::NodeKramdownSpanIAL);
        left.insert_after(ial);
        assert_eq!(previous_node_text(br), Some("left".into()));
        assert_eq!(next_node_text(left), Some(String::new()));
    }

    #[test]
    fn cell_alignment_keeps_explicit_css_and_separates_added_declaration() {
        let mut tree = Tree::new();
        let cell = tree.new_node_raw(NodeType::NodeTableCell);
        cell.data_mut().table_cell_align = 2;
        let mut explicit = vec![("style".into(), "color: red; TEXT-ALIGN: right;".into())];
        append_table_cell_align_style(cell, &mut explicit);
        assert_eq!(explicit[0].1, "color: red; TEXT-ALIGN: right;");
        let mut implicit = vec![("style".into(), "color: red".into())];
        append_table_cell_align_style(cell, &mut implicit);
        assert_eq!(implicit[0].1, "color: red; text-align: center;");
    }
}
