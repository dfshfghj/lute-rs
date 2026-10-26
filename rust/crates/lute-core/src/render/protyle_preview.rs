// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Protyle 预览渲染器，对齐 Go 版 render.ProtylePreviewRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::editor;
use crate::options::{ParseOptions, RenderOptions};

use super::{BaseRenderer, Renderer};

/// Protyle 导出预览 HTML 渲染器。
pub struct ProtylePreviewRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
    /// 脚注定义首个块需要前置的输出。
    pending_footnotes_marker: Option<String>,
}

impl ProtylePreviewRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
            pending_footnotes_marker: None,
        }
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        match n.typ() {
            NodeType::NodeDocument => WalkStatus::WalkContinue,
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => self.render_text(n, entering),
            NodeType::NodeCodeSpan => self.render_code_span(n, entering),
            NodeType::NodeCodeSpanOpenMarker => {
                if entering {
                    self.base.tag(
                        "code",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanContent => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanCloseMarker => {
                if entering {
                    self.base.tag("/code", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlock => self.render_code_block(n, entering),
            NodeType::NodeCodeBlockFenceOpenMarker
            | NodeType::NodeCodeBlockFenceInfoMarker
            | NodeType::NodeCodeBlockFenceCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeCodeBlockCode => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlock => self.render_math_block(n, entering),
            NodeType::NodeMathBlockOpenMarker
            | NodeType::NodeMathBlockContent
            | NodeType::NodeMathBlockCloseMarker => WalkStatus::WalkContinue,
            NodeType::NodeInlineMath => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathOpenMarker => {
                if entering {
                    let next_tokens = n.next().map(|x| x.tokens_vec()).unwrap_or_default();
                    let tokens = super::escape_html_bytes(&next_tokens);
                    self.base.tag(
                        "span",
                        &[
                            ("data-type".to_string(), "inline-math".to_string()),
                            ("data-subtype".to_string(), "math".to_string()),
                            (
                                "data-content".to_string(),
                                String::from_utf8_lossy(&tokens).to_string(),
                            ),
                        ],
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathContent => WalkStatus::WalkContinue,
            NodeType::NodeInlineMathCloseMarker => {
                if entering {
                    self.base.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmphasis | NodeType::NodeStrong | NodeType::NodeStrikethrough => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmA6kOpenMarker | NodeType::NodeEmU8eOpenMarker => {
                if entering {
                    self.base.tag(
                        "em",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmA6kCloseMarker | NodeType::NodeEmU8eCloseMarker => {
                if entering {
                    self.base.tag("/em", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kOpenMarker | NodeType::NodeStrongU8eOpenMarker => {
                if entering {
                    self.base.tag(
                        "strong",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kCloseMarker | NodeType::NodeStrongU8eCloseMarker => {
                if entering {
                    self.base.tag("/strong", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough1OpenMarker | NodeType::NodeStrikethrough2OpenMarker => {
                if entering {
                    self.base.tag(
                        "del",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough1CloseMarker | NodeType::NodeStrikethrough2CloseMarker => {
                if entering {
                    self.base.tag("/del", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote => {
                if entering {
                    self.base.newline();
                    self.base.tag("blockquote", &n.ial_raw(), false);
                    self.base.newline();
                } else {
                    self.base.newline();
                    self.base.write_str("</blockquote>");
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker | NodeType::NodeHeadingID => WalkStatus::WalkContinue,
            NodeType::NodeList => self.render_list(n, entering),
            NodeType::NodeListItem => self.render_list_item(n, entering),
            NodeType::NodeMindmap => self.render_list(n, entering),
            NodeType::NodeMindmapItem => self.render_list_item(n, entering),
            NodeType::NodeTaskListItemMarker => self.render_task_list_item_marker(n, entering),
            NodeType::NodeThematicBreak => {
                if entering {
                    self.base.newline();
                    self.base.tag("hr", &[], true);
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHardBreak => {
                if entering {
                    self.base.tag("br", &[], true);
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSoftBreak => {
                if entering {
                    if self.base.options.soft_break2_hard_break {
                        self.base.tag("br", &[], true);
                        self.base.newline();
                    } else {
                        self.base.newline();
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
                if entering
                    && NodeType::NodeImage
                        != n.parent()
                            .map(|p| p.typ())
                            .unwrap_or(NodeType::NodeDocument)
                {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead => self.render_table_head(n, entering),
            NodeType::NodeTableRow => {
                if entering {
                    self.base.tag("tr", &[], false);
                    self.base.newline();
                } else {
                    self.base.tag("/tr", &[], false);
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTableCell => self.render_table_cell(n, entering),
            NodeType::NodeEmoji | NodeType::NodeEmojiAlias => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDefBlock => self.render_footnotes_def_block(n, entering),
            NodeType::NodeFootnotesDef => self.render_footnotes_def(n, entering),
            NodeType::NodeFootnotesRef => self.render_footnotes_ref(n, entering),
            NodeType::NodeToC => self.render_toc(n, entering),
            NodeType::NodeBackslash => WalkStatus::WalkContinue,
            NodeType::NodeBackslashContent => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatter => {
                self.base.newline();
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterOpenMarker => {
                if entering {
                    let mut attrs =
                        vec![("class".to_string(), "vditor-yml-front-matter".to_string())];
                    if let Some(p) = n.parent() {
                        attrs.extend(p.ial_raw());
                    }
                    self.base.tag("pre", &attrs, false);
                    self.base.write_str("<code class=\"language-yaml\">");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterCloseMarker => {
                if entering {
                    self.base.write_str("</code></pre>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRef | NodeType::NodeBlockRefID | NodeType::NodeBlockRefSpace => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefText => {
                if entering {
                    self.base.write_byte(b'"');
                    self.base.write(n.tokens());
                } else {
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefDynamicText => {
                if entering {
                    self.base.write_byte(b'\'');
                    self.base.write(n.tokens());
                } else {
                    self.base.write_byte(b'\'');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFileAnnotationRef
            | NodeType::NodeFileAnnotationRefID
            | NodeType::NodeFileAnnotationRefSpace => WalkStatus::WalkContinue,
            NodeType::NodeFileAnnotationRefText => {
                if entering {
                    self.base.write_byte(b'"');
                    self.base.write(n.tokens());
                } else {
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark1OpenMarker | NodeType::NodeMark2OpenMarker => {
                if entering {
                    self.base.tag(
                        "mark",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark1CloseMarker | NodeType::NodeMark2CloseMarker => {
                if entering {
                    self.base.tag("/mark", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSup => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSupOpenMarker => {
                if entering {
                    self.base.tag("sup", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSupCloseMarker => {
                if entering {
                    self.base.tag("/sup", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSub => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSubOpenMarker => {
                if entering {
                    self.base.tag("sub", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSubCloseMarker => {
                if entering {
                    self.base.tag("/sub", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTag => {
                if entering {
                    self.base.text_auto_space_previous(n);
                } else {
                    self.base.text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTagOpenMarker => {
                if entering {
                    self.base.tag(
                        "em",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                    self.base.write_byte(b'#');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTagCloseMarker => {
                if entering {
                    self.base.write_byte(b'#');
                    self.base.tag("/em", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownBlockIAL
            | NodeType::NodeKramdownSpanIAL
            | NodeType::NodeLinkRefDefBlock
            | NodeType::NodeLinkRefDef => WalkStatus::WalkSkipChildren,
            NodeType::NodeBlockQueryEmbed => {
                if entering {
                    self.base.newline();
                    self.base.tag("div", &[], false);
                } else {
                    self.base.tag("/div", &[], false);
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockQueryEmbedScript => {
                if entering {
                    self.base.write_byte(b'"');
                    self.base.write(n.tokens());
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCustomBlock => {
                if entering {
                    self.base.newline();
                    let attrs = vec![
                        ("data-type".to_string(), "NodeCustomBlock".to_string()),
                        (
                            "data-info".to_string(),
                            crate::html::escape_html_str(&n.data().custom_block_info),
                        ),
                        (
                            "data-content".to_string(),
                            String::from_utf8_lossy(&super::escape_html_bytes(n.tokens()))
                                .to_string(),
                        ),
                    ];
                    self.base.tag("div", &attrs, false);
                    self.base.tag("pre", &[], false);
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                    self.base.write_str("</pre>");
                    self.base.write_str("</div>");
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeAttributeView => {
                if entering {
                    self.base.newline();
                    self.base.tag(
                        "div",
                        &[
                            ("data-type".to_string(), "NodeAttributeView".to_string()),
                            ("data-av-id".to_string(), n.data().attribute_view_id.clone()),
                            (
                                "data-av-type".to_string(),
                                n.data().attribute_view_type.clone(),
                            ),
                        ],
                        false,
                    );
                    self.base.write_str("</div>");
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeVideo | NodeType::NodeAudio => {
                if entering {
                    self.base
                        .tag("div", &[("class".to_string(), "iframe".to_string())], false);
                    let tokens = self.tag_src_path(n.tokens());
                    self.base.write(&tokens);
                    self.base.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeIFrame => {
                if entering {
                    let mut attrs = vec![("class".to_string(), "iframe".to_string())];
                    let style = n.ial_attr("style");
                    if !style.is_empty() {
                        attrs.push(("style".to_string(), style));
                    }
                    self.base.tag("div", &attrs, false);
                    let tokens = self.tag_src_path(n.tokens());
                    self.base.write(&tokens);
                    self.base.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeWidget => {
                if entering {
                    let mut attrs = n.ial_raw();
                    attrs.push(("class".to_string(), "iframe".to_string()));
                    self.base.tag("div", &attrs, false);
                    let tokens = self.tag_src_path(n.tokens());
                    self.base.write(&tokens);
                    self.base.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGitConflict => {
                self.base.newline();
                if entering {
                    let mut attrs =
                        vec![("class".to_string(), "language-git-conflict".to_string())];
                    attrs.extend(n.ial_raw());
                    self.base.tag("div", &attrs, false);
                } else {
                    self.base.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGitConflictOpenMarker | NodeType::NodeGitConflictCloseMarker => {
                if entering {
                    self.base.write(n.tokens());
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGitConflictContent => {
                if entering {
                    self.base.write(&super::escape_html_bytes(n.tokens()));
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSuperBlock => WalkStatus::WalkContinue,
            NodeType::NodeSuperBlockOpenMarker
            | NodeType::NodeSuperBlockLayoutMarker
            | NodeType::NodeSuperBlockCloseMarker => WalkStatus::WalkSkipChildren,
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => {
                super::html::render_tabs_html(&mut self.base, n, entering, &self.parse_options)
            }
            NodeType::NodeTextMark => self.render_text_mark(n, entering),
            NodeType::NodeBr => {
                if entering {
                    self.base.write_str("<br />");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeUnderline => WalkStatus::WalkContinue,
            NodeType::NodeUnderlineOpenMarker => {
                if entering {
                    self.base.write_str("<u>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeUnderlineCloseMarker => {
                if entering {
                    self.base.write_str("</u>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKbd => WalkStatus::WalkContinue,
            NodeType::NodeKbdOpenMarker => {
                if entering {
                    self.base.tag(
                        "kbd",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKbdCloseMarker => {
                if entering {
                    self.base.tag("/kbd", &[], false);
                }
                WalkStatus::WalkContinue
            }
            _ => WalkStatus::WalkContinue,
        }
    }

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if n != self.root && is_tab_title_block(n) {
            return WalkStatus::WalkSkipChildren;
        }
        if entering {
            self.base.newline();
            self.base.tag("p", &n.ial_raw(), false);
            if let Some(marker) = self.pending_footnotes_marker.take() {
                self.base.write_str(&marker);
            }
            if self.base.options.chinese_paragraph_beginning_space
                && n.parent().map(|p| p.typ()) == Some(NodeType::NodeDocument)
                && !self.paragraph_contain_img_only(n)
            {
                self.base.write_str("　　");
            }
        } else {
            self.base.tag("/p", &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn paragraph_contain_img_only(&self, paragraph: Node) -> bool {
        let mut ret = true;
        let mut contain_img = false;
        crate::ast::walk_raw(paragraph, &mut |n, entering| {
            if !entering {
                return WalkStatus::WalkContinue;
            }
            if NodeType::NodeText == n.typ() {
                if !super::is_empty_str(&n.tokens_str()) {
                    ret = false;
                    return WalkStatus::WalkStop;
                }
            } else if NodeType::NodeTextMark == n.typ() {
                ret = false;
                return WalkStatus::WalkStop;
            } else if NodeType::NodeImage == n.typ() {
                contain_img = true;
            }
            WalkStatus::WalkContinue
        });
        contain_img && ret
    }

    fn render_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut tokens = n.tokens_vec();
            if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                tokens = self.base.space(&tokens);
            }
            self.base.write(&super::escape_html_bytes(&tokens));
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                let text = n.previous().map(|p| p.text()).unwrap_or_default();
                if !text.is_empty() {
                    let lastc = text.chars().next_back().unwrap_or('\u{FFFD}');
                    if lastc.is_alphabetic() || lastc.is_numeric() {
                        self.base.write_byte(b' ');
                    }
                }
            }
        } else if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
            let text = n.next().map(|p| p.text()).unwrap_or_default();
            if !text.is_empty() {
                let firstc = text.chars().next().unwrap_or('\u{FFFD}');
                if firstc.is_alphabetic() || firstc.is_numeric() {
                    self.base.write_byte(b' ');
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.base.newline();
        let mut no_highlight = false;
        let mut language = String::new();
        let info = n
            .first_child()
            .and_then(|c| c.next())
            .map(|c| c.data().code_block_info.clone())
            .unwrap_or_default();
        if !info.is_empty() {
            language = String::from_utf8_lossy(&info).to_string();
            no_highlight = language != "mindmap" && no_highlight_lang(&language);
        }

        if entering {
            if no_highlight {
                let tokens = n
                    .first_child()
                    .and_then(|c| c.next())
                    .and_then(|c| c.next())
                    .map(|c| c.tokens_vec())
                    .unwrap_or_default();
                let mut tokens = super::escape_html_bytes(&tokens);
                tokens = super::replace_bytes(&tokens, editor::CARET.as_bytes(), b"");
                let tokens = trim_space(&tokens);
                let mut attrs = vec![
                    (
                        "data-content".to_string(),
                        String::from_utf8_lossy(&tokens).to_string(),
                    ),
                    ("data-subtype".to_string(), language),
                ];
                attrs.extend(n.ial_raw());
                self.base.tag("div", &attrs, false);
                self.base
                    .tag("div", &[("spin".to_string(), "1".to_string())], false);
                self.base.tag("/div", &[], false);
                self.base.tag("/div", &[], false);
                return WalkStatus::WalkSkipChildren;
            }

            let mut attrs = vec![
                ("class".to_string(), "code-block".to_string()),
                ("data-language".to_string(), language),
            ];
            attrs.extend(n.ial_raw());
            self.base.tag("pre", &attrs, false);
            self.base.write_str("<code class=\"hljs\">");
        } else {
            if no_highlight {
                return WalkStatus::WalkSkipChildren;
            }
            self.base.tag("/code", &[], false);
            self.base.tag("/pre", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_math_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.base.newline();
        if entering {
            let tokens = n
                .first_child()
                .and_then(|c| c.next())
                .map(|c| c.tokens_vec())
                .unwrap_or_default();
            let mut tokens = super::escape_html_bytes(&tokens);
            tokens = super::replace_bytes(&tokens, editor::CARET.as_bytes(), b"");
            let tokens = trim_space(&tokens);
            let mut attrs = vec![
                (
                    "data-content".to_string(),
                    String::from_utf8_lossy(&tokens).to_string(),
                ),
                ("data-subtype".to_string(), "math".to_string()),
            ];
            attrs.extend(n.ial_raw());
            self.base.tag("div", &attrs, false);
            self.base
                .tag("div", &[("spin".to_string(), "1".to_string())], false);
            self.base.tag("/div", &[], false);
            self.base.tag("/div", &[], false);
            self.base.newline();
        }
        WalkStatus::WalkSkipChildren
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        let level = n.data().heading_level.max(0) as usize;
        if entering {
            self.base.newline();
            self.base.write_str(&format!("<h{}", level));
            let mut id = n.id().to_string();
            if id.is_empty() {
                id = super::heading_id(n);
            }
            if self.base.options.toc
                || self.base.options.heading_id
                || (crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial)
            {
                self.base.write_str(&format!(" id=\"{}\"", id));
                if crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial {
                    let ials = n.ial_raw();
                    if "id" != self.base.options.kramdown_ial_id_render_name && !ials.is_empty() {
                        self.base.write_str(&format!(
                            " {}=\"{}\"",
                            self.base.options.kramdown_ial_id_render_name,
                            n.data().heading_normalized_id
                        ));
                    }
                    if 1 < ials.len() {
                        for (k, v) in ials.iter() {
                            if "id" == k {
                                continue;
                            }
                            self.base.write_str(&format!(" {}=\"{}\"", k, v));
                        }
                    }
                }
            }
            self.base.write_str(">");
        } else {
            if self.base.options.heading_anchor {
                let id = super::heading_id(n);
                self.base.tag(
                    "a",
                    &[
                        ("id".to_string(), format!("vditorAnchor-{}", id)),
                        ("class".to_string(), "vditor-anchor".to_string()),
                        ("href".to_string(), format!("#{}", id)),
                    ],
                    false,
                );
                self.base.write_str(HEADING_ANCHOR_SVG);
                self.base.tag("/a", &[], false);
            }
            self.base.write_str(&format!("</h{}>", level));
            self.base.newline();
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
            self.base.newline();
            let mut attrs = Vec::new();
            self.render_list_style(n, &mut attrs);
            if 0 == ld.bullet_char && 1 != ld.start {
                attrs.push(("start".to_string(), ld.start.to_string()));
            }
            attrs.extend(n.ial_raw());
            self.base.tag(tag, &attrs, false);
            self.base.newline();
        } else {
            self.base.newline();
            self.base.tag(&format!("/{}", tag), &[], false);
            self.base.newline();
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
                "data-style".to_string(),
                String::from_utf8_lossy(&ld.marker).to_string(),
            )),
            1 => attrs.push((
                "data-style".to_string(),
                format!("{}{}", ld.num, ld.delimiter as char),
            )),
            3 => {
                if 0 == ld.bullet_char {
                    attrs.push((
                        "data-style".to_string(),
                        format!("{}{}", ld.num, ld.delimiter as char),
                    ));
                } else {
                    attrs.push((
                        "data-style".to_string(),
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
            let mut attrs = n.ial_raw();
            if 3 == ld.typ {
                if let Some(first) = n.first_child() {
                    let mut marker = if NodeType::NodeTaskListItemMarker == first.typ() {
                        Some(first)
                    } else {
                        first
                            .first_child()
                            .filter(|c| NodeType::NodeTaskListItemMarker == c.typ())
                    };
                    if let Some(m) = marker.take() {
                        let mut task_class = "protyle-task".to_string();
                        if m.data().task_list_item_checked {
                            task_class.push_str(" protyle-task--done");
                        }
                        attrs.push(("class".to_string(), task_class));
                    }
                }
            }
            self.base.tag("li", &attrs, false);
        } else {
            self.base.tag("/li", &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_task_list_item_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs: Vec<(String, String)> = Vec::new();
            if self.normalized_task_list_item_checked(n) {
                attrs.push(("checked".to_string(), String::new()));
            }
            attrs.push(("disabled".to_string(), String::new()));
            attrs.push(("type".to_string(), "checkbox".to_string()));
            if self.base.options.data_task {
                attrs.push(("data-task".to_string(), n.effective_task_list_item_marker()));
            }
            self.base.tag("input", &attrs, true);
        }
        WalkStatus::WalkContinue
    }

    fn normalized_task_list_item_marker(&self, n: Node) -> String {
        let mut marker = n.effective_task_list_item_marker();
        if self.base.options.export_normalize_task_list_marker && marker != " " && marker != "X" {
            marker = "X".to_string();
        }
        crate::html::unescape_html_str(&marker)
    }

    fn normalized_task_list_item_checked(&self, n: Node) -> bool {
        self.normalized_task_list_item_marker(n) != " "
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if need_use_html_table(n) {
            if entering {
                self.render_table_by_html(n);
                return WalkStatus::WalkSkipChildren;
            }
            return WalkStatus::WalkContinue;
        }

        if entering {
            let ials: Vec<(String, String)> = n
                .ial_raw()
                .into_iter()
                .filter(|(k, _)| !matches!(k.as_str(), "id" | "caption" | "updated" | "colgroup"))
                .collect();
            self.base.tag("table", &ials, false);
            self.base.newline();
            let caption = n.ial_attr("caption");
            if !caption.is_empty() {
                let caption = crate::html::unescape_html_str(&caption)
                    .replace("contenteditable=\"false\" ", "");
                self.base.write_str(&caption);
            }
            self.base.newline();
        } else {
            if n.first_child().and_then(|c| c.next()).is_some() {
                self.base.tag("/tbody", &[], false);
            }
            self.base.newline();
            self.base.tag("/table", &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_table_head(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.tag("colgroup", &[], false);
            let colgroup = n
                .parent()
                .map(|p| p.ial_attr("colgroup"))
                .unwrap_or_default();
            if colgroup.is_empty() {
                if let Some(row) = n.first_child() {
                    let mut th = row.first_child();
                    while let Some(cell) = th {
                        if NodeType::NodeTableCell == cell.typ() {
                            let style = cell.ial_attr("style");
                            if !style.is_empty() {
                                self.base.tag("col", &[("style".to_string(), style)], true);
                            } else {
                                self.base.tag("col", &[], true);
                            }
                        }
                        th = cell.next();
                    }
                }
            } else {
                for style in colgroup.split('|') {
                    if !style.is_empty() {
                        self.base
                            .tag("col", &[("style".to_string(), style.to_string())], true);
                    } else {
                        self.base.tag("col", &[], true);
                    }
                }
            }
            self.base.tag("/colgroup", &[], false);
            self.base.tag("thead", &[], false);
        } else {
            self.base.tag("/thead", &[], false);
            if n.next().is_some() {
                self.base.tag("tbody", &[], false);
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_table_cell(&mut self, n: Node, entering: bool) -> WalkStatus {
        for (k, v) in n.ial_raw() {
            if "class" == k && v.contains("fn__none") {
                return WalkStatus::WalkContinue;
            }
        }
        let tag = if n
            .parent()
            .and_then(|p| p.parent())
            .map(|p| NodeType::NodeTableHead == p.typ())
            .unwrap_or(false)
        {
            "th"
        } else {
            "td"
        };
        if entering {
            let mut attrs = n.ial_raw();
            append_table_cell_align_style(n, &mut attrs);
            self.base.tag(tag, &attrs, false);
        } else {
            self.base.tag(&format!("/{}", tag), &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_table_by_html(&mut self, table: Node) {
        let mut ials: Vec<(String, String)> = table
            .ial_raw()
            .into_iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "id" | "caption" | "updated" | "colgroup" | "custom-sy-table-rich"
                )
            })
            .collect();
        ials.sort_by(|a, b| a.0.cmp(&b.0));
        self.base.tag("table", &ials, false);
        self.base.newline();
        let caption = table.ial_attr("caption");
        if !caption.is_empty() {
            let caption =
                crate::html::unescape_html_str(&caption).replace("contenteditable=\"false\" ", "");
            self.base.write_str(&caption);
        }
        self.base.newline();

        self.base.tag("colgroup", &[], false);
        let colgroup = table.ial_attr("colgroup");
        if colgroup.is_empty() {
            if let Some(head) = table.child_by_type(NodeType::NodeTableHead) {
                if let Some(row) = head.first_child() {
                    let mut th = row.first_child();
                    while let Some(cell) = th {
                        if NodeType::NodeTableCell == cell.typ() {
                            let style = cell.ial_attr("style");
                            if !style.is_empty() {
                                self.base.tag("col", &[("style".to_string(), style)], true);
                            } else {
                                self.base.tag("col", &[], true);
                            }
                        }
                        th = cell.next();
                    }
                }
            }
        } else {
            for style in colgroup.split('|') {
                if !style.is_empty() {
                    self.base
                        .tag("col", &[("style".to_string(), style.to_string())], true);
                } else {
                    self.base.tag("col", &[], true);
                }
            }
        }
        self.base.tag("/colgroup", &[], false);

        let mut has_thead = false;
        let mut has_tbody = false;
        let mut child = table.first_child();
        while let Some(c) = child {
            if NodeType::NodeTableHead == c.typ() {
                self.base.tag("thead", &[], false);
                self.render_table_rows(c);
                self.base.tag("/thead", &[], false);
                has_thead = true;
            } else if NodeType::NodeTableRow == c.typ() {
                if !has_tbody {
                    self.base.tag("tbody", &[], false);
                    self.base.newline();
                    has_tbody = true;
                }
                self.render_table_row_html(c);
            }
            child = c.next();
        }
        if has_thead && !has_tbody {
            self.base.tag("tbody", &[], false);
            self.base.newline();
        }
        if has_tbody {
            self.base.tag("/tbody", &[], false);
            self.base.newline();
        }
        self.base.tag("/table", &[], false);
        self.base.newline();
    }

    fn render_table_rows(&mut self, thead: Node) {
        let mut row = thead.first_child();
        while let Some(r) = row {
            if NodeType::NodeTableRow == r.typ() {
                self.render_table_row_html(r);
            }
            row = r.next();
        }
    }

    fn render_table_row_html(&mut self, row: Node) {
        self.base.tag("tr", &[], false);
        self.base.newline();
        let mut cell = row.first_child();
        while let Some(c) = cell {
            if NodeType::NodeTableCell != c.typ() {
                cell = c.next();
                continue;
            }
            let tag = if c
                .parent()
                .and_then(|p| p.parent())
                .map(|p| NodeType::NodeTableHead == p.typ())
                .unwrap_or(false)
            {
                "th"
            } else {
                "td"
            };
            let mut attrs: Vec<(String, String)> = c
                .ial_raw()
                .into_iter()
                .filter(|(k, _)| k == "style")
                .collect();
            append_table_cell_align_style(c, &mut attrs);
            self.base.tag(tag, &attrs, false);
            let mut child = c.first_child();
            while let Some(ch) = child {
                self.render_node(ch, true);
                self.render_node(ch, false);
                child = ch.next();
            }
            self.base.tag(&format!("/{}", tag), &[], false);
            self.base.newline();
            cell = c.next();
        }
    }

    fn render_link(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.link_text_auto_space_previous(n);
            let mut dest_tokens = n
                .child_by_type(NodeType::NodeLinkDest)
                .map(|d| d.tokens_vec())
                .unwrap_or_default();
            if self.base.options.sanitize {
                let tokens = trim_space(&dest_tokens);
                let lower = String::from_utf8_lossy(&tokens).to_lowercase();
                if lower.starts_with("javascript:") {
                    dest_tokens = Vec::new();
                }
            }
            let dest_tokens = self.base.link_path(&dest_tokens);
            let mut attrs = vec![(
                "href".to_string(),
                String::from_utf8_lossy(&super::escape_html_bytes(&dest_tokens)).to_string(),
            )];
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.tokens().is_empty() {
                    attrs.push((
                        "title".to_string(),
                        String::from_utf8_lossy(&super::escape_html_bytes(title.tokens()))
                            .to_string(),
                    ));
                }
            }
            self.base.tag("a", &attrs, false);
        } else {
            self.base.tag("/a", &[], false);
            self.base.link_text_auto_space_next(n);
        }
        WalkStatus::WalkContinue
    }

    fn render_image(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut attrs = vec![
                ("contenteditable".to_string(), "false".to_string()),
                ("data-type".to_string(), "img".to_string()),
                ("class".to_string(), "img".to_string()),
            ];
            let mut parent_style = n.ial_attr("parent-style");
            if !parent_style.is_empty() {
                parent_style = parent_style.replace("display: block;", "");
                parent_style = parent_style.trim().to_string();
                if !parent_style.is_empty() {
                    attrs.push(("style".to_string(), parent_style));
                }
            }
            if b'\n' == self.base.last_out {
                self.base.write_str(editor::ZWSP);
            }
            self.base.tag("span", &attrs, false);
            self.base.tag("span", &[], false);
            self.base.write_str(" ");
            self.base.tag("/span", &[], false);

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
                let width = width.replace("vw", "%");
                let width = width.trim().to_string();
                if !width.is_empty() {
                    attrs.push(("style".to_string(), format!("{};", width)));
                }
            }
            self.base.tag("span", &attrs, false);
            self.base.tag(
                "span",
                &[(
                    "class".to_string(),
                    "protyle-action protyle-icons".to_string(),
                )],
                false,
            );
            self.base.write_str("<span class=\"protyle-icon protyle-icon--only\"><svg class=\"svg\"><use xlink:href=\"#iconMore\"></use></svg></span>");
            self.base.tag("/span", &[], false);
        } else {
            let mut dest_tokens = n
                .child_by_type(NodeType::NodeLinkDest)
                .map(|d| d.tokens_vec())
                .unwrap_or_default();
            if self.base.options.sanitize {
                dest_tokens = super::sanitizer::sanitize(&dest_tokens);
            }
            dest_tokens = super::replace_bytes(&dest_tokens, editor::CARET.as_bytes(), b"");
            let data_src_tokens = dest_tokens.clone();
            let data_src = String::from_utf8_lossy(&data_src_tokens).to_string();
            let src = String::from_utf8_lossy(&self.base.link_path(&dest_tokens)).to_string();
            let mut attrs = vec![("src".to_string(), src), ("data-src".to_string(), data_src)];
            if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                if !alt.tokens().is_empty() {
                    attrs.push((
                        "alt".to_string(),
                        String::from_utf8_lossy(alt.tokens()).to_string(),
                    ));
                }
            }

            let mut title_tokens: Vec<u8> = Vec::new();
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.tokens().is_empty() {
                    title_tokens = title.tokens_vec();
                    attrs.push((
                        "title".to_string(),
                        escape_ref_text(&String::from_utf8_lossy(&title_tokens)),
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
                let style = style
                    .replace(&format!("{};", width), "")
                    .replace("flex: 0 0 auto;", "")
                    .replace("display: block;", "");
                let style = style.trim().to_string();
                if !style.is_empty() {
                    attrs.push(("style".to_string(), style));
                }
            }
            self.base.tag("img", &attrs, true);

            self.sanitize_last_img();

            self.base.tag(
                "span",
                &[("class".to_string(), "protyle-action__drag".to_string())],
                false,
            );
            self.base.tag("/span", &[], false);

            if self.base.options.protyle_mark_net_img
                && !data_src_tokens.starts_with(b"assets/")
                && !data_src_tokens.starts_with(b"api/icon/")
            {
                self.base.write_str("<span class=\"img__net\"><svg><use xlink:href=\"#iconLanguage\"></use></svg></span>");
            }

            self.base.tag(
                "span",
                &[("class".to_string(), "protyle-action__title".to_string())],
                false,
            );
            self.base.tag("span", &[], false);
            self.base.write(&super::escape_html_bytes(&title_tokens));
            self.base.tag("/span", &[], false);
            self.base.tag("/span", &[], false);
            self.base.tag("/span", &[], false);
            self.base.tag("span", &[], false);
            self.base.write_str(" ");
            self.base.tag("/span", &[], false);
            self.base.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn sanitize_last_img(&mut self) {
        let buf = &self.base.out;
        let marker = b"<img src=";
        let Some(idx) = find_last_sub(buf, marker) else {
            return;
        };
        let mut img_buf = buf[idx..].to_vec();
        if self.base.options.sanitize {
            img_buf = super::sanitizer::sanitize(&img_buf);
        }
        img_buf = self.tag_src_path(&img_buf);
        self.base.out.truncate(idx);
        self.base.write(&img_buf);
    }

    fn render_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            let mut tokens = n.tokens_vec();
            if self.base.options.sanitize {
                tokens = super::sanitizer::sanitize(&tokens);
            }
            let tokens = self.tag_src_path(&tokens);
            self.base.write(&tokens);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_inline_html(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let mut tokens = n.tokens_vec();
            if self.base.options.sanitize {
                tokens = super::sanitizer::sanitize(&tokens);
            }
            self.base.write(&tokens);
        }
        WalkStatus::WalkContinue
    }

    fn tag_src_path(&self, tokens: &[u8]) -> Vec<u8> {
        if self.base.options.link_base.is_empty() {
            return tokens.to_vec();
        }
        let marker = b"src=\"";
        let Some(idx) = find_sub(tokens, marker) else {
            return tokens.to_vec();
        };
        let src = &tokens[idx + marker.len()..];
        let Some(end) = src.iter().position(|&b| b == b'"') else {
            return tokens.to_vec();
        };
        let src = &src[..end];
        if src.is_empty() {
            return tokens.to_vec();
        }
        let target = self.base.link_path(src);
        let mut origin = String::from_utf8_lossy(&target).to_string();
        if origin.starts_with("//") {
            origin = format!("https:{}", origin);
        }
        let mut ret = tokens[..idx + marker.len()].to_vec();
        ret.extend_from_slice(origin.as_bytes());
        ret.extend_from_slice(&tokens[idx + marker.len() + src.len()..]);
        ret
    }

    fn render_callout(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            {
                let d = n.data_mut();
                d.kramdown_ial
                    .push(("data-type".to_string(), "callout".to_string()));
                d.kramdown_ial.push((
                    "data-subtype".to_string(),
                    crate::html::escape_html_str(&d.callout_type),
                ));
            }
            self.render_blockquote(n, entering);
            self.base.write_str("<p>");
            let mut title = callout_display_title(n);
            if !n.data().callout_icon.is_empty() {
                title = format!("{} {}", n.data().callout_icon, title);
            }
            if !title.trim().is_empty() {
                self.base.write_str(&title);
                self.base.newline();
            }
            self.base.write_str("</p>");
        } else {
            self.render_blockquote(n, entering);
        }
        WalkStatus::WalkContinue
    }

    fn render_blockquote(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.tag("blockquote", &n.ial_raw(), false);
            self.base.newline();
        } else {
            self.base.newline();
            self.base.write_str("</blockquote>");
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_text_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let d = n.data();
        let mut text_content = d.text_mark_text_content.clone();
        if n.parent_is(&[NodeType::NodeTableCell]) {
            if is_text_mark_type(n, "code") {
                text_content = text_content.replace('|', "&#124;");
            } else {
                text_content = text_content.replace("\\|", "|");
            }
            text_content = text_content.replace('\n', "<br />");
        }

        if is_text_mark_type(n, "a") {
            let mut attrs = vec![("href".to_string(), d.text_mark_a_href.clone())];
            if !d.text_mark_a_title.is_empty() {
                attrs.push(("title".to_string(), d.text_mark_a_title.clone()));
            }
            attrs.extend(n.ial_raw());
            if !d.text_mark_type.is_empty() {
                attrs.push(("data-type".to_string(), d.text_mark_type.clone()));
            }
            if !d.text_mark_block_ref_subtype.is_empty() {
                attrs.push((
                    "data-subtype".to_string(),
                    d.text_mark_block_ref_subtype.clone(),
                ));
            }
            self.base.tag("a", &attrs, false);
            self.base.write_str(&text_content);
            self.base.write_str("</a>");
        } else if is_text_mark_type(n, "inline-memo") {
            self.base.write_str(&text_content);
            if is_next_same_inline_memo(n) {
                return WalkStatus::WalkContinue;
            }
            if !d.text_mark_inline_memo_content.is_empty() {
                let last_rune = d
                    .text_mark_text_content
                    .chars()
                    .next_back()
                    .unwrap_or('\u{FFFD}');
                let memo = d
                    .text_mark_inline_memo_content
                    .replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                if is_cjk(last_rune) {
                    self.base.write_str("<sup>（");
                    self.base.write_str(&memo);
                    self.base.write_str("）</sup>");
                } else {
                    self.base.write_str("<sup>(");
                    self.base.write_str(&memo);
                    self.base.write_str(")</sup>");
                }
            }
        } else {
            let mut attrs = text_mark_attrs(n);
            attrs.extend(n.ial_raw());
            self.base.tag("span", &attrs, false);
            self.base.write_str(&text_content);
            self.base.write_str("</span>");
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
        let ref_id = n.data().footnotes_ref_id.clone();
        self.base.tag(
            "sup",
            &[
                ("class".to_string(), "footnotes-ref".to_string()),
                ("id".to_string(), format!("footnotes-ref-{}", ref_id)),
            ],
            false,
        );
        self.base.tag(
            "a",
            &[(
                "href".to_string(),
                format!("{}#footnotes-def-{}", self.base.options.link_base, idx_str),
            )],
            false,
        );
        self.base.write_str(&idx_str);
        self.base.tag("/a", &[], false);
        self.base.tag("/sup", &[], false);
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

    fn render_footnotes_def_block(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if !self.exist_footnotes_def() {
            return WalkStatus::WalkContinue;
        }
        if entering {
            self.base.write_str("<div class=\"footnotes-defs-div\">");
            self.base.write_str("<hr class=\"footnotes-defs-hr\" />\n");
            self.base.write_str("<ol class=\"footnotes-defs-ol\">");
        } else {
            self.base.write_str("</ol></div>");
        }
        WalkStatus::WalkContinue
    }

    fn exist_footnotes_def(&self) -> bool {
        let mut ret = false;
        crate::ast::walk_raw(self.root, &mut |n, entering| {
            if entering && NodeType::NodeFootnotesDef == n.typ() {
                ret = true;
                return WalkStatus::WalkStop;
            }
            WalkStatus::WalkContinue
        });
        ret
    }

    fn render_footnotes_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.write_str("<li>");
            let refs = n.data().footnotes_refs.clone();
            if !refs.is_empty() && n.first_child().is_some() {
                let ref_id = refs[0].data().footnotes_ref_id.clone();
                self.pending_footnotes_marker =
                    Some(format!("<span id=\"footnotes-def-{}\"></span>", ref_id));
            }
        } else {
            self.base.write_str("</li>\n");
        }
        WalkStatus::WalkContinue
    }

    fn render_toc(&mut self, _n: Node, entering: bool) -> WalkStatus {
        if entering {
            let headings = super::headings(self.root, &self.base.options);
            self.base.write_str("<div class=\"vditor-toc\" data-block=\"0\" data-type=\"toc-block\" contenteditable=\"false\">");
            if !headings.top.is_empty() {
                self.base.write_str("<ul>");
                for i in headings.top.clone() {
                    self.render_toc0(&headings.all, i);
                }
                self.base.write_str("</ul>");
            } else {
                self.base.write_str("[toc]<br>");
            }
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn render_toc0(&mut self, headings: &[super::Heading], idx: usize) {
        self.base.write_str("<li>");
        self.base.tag(
            "span",
            &[("data-target-id".to_string(), headings[idx].id.clone())],
            false,
        );
        self.base.write_str(&headings[idx].content);
        self.base.tag("/span", &[], false);
        if !headings[idx].children.is_empty() {
            self.base.write_str("<ul>");
            for child in headings[idx].children.clone() {
                self.render_toc0(headings, child);
            }
            self.base.write_str("</ul>");
        }
        self.base.write_str("</li>");
    }
}

impl Renderer for ProtylePreviewRenderer {
    fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);
        let root = self.root;
        crate::ast::walk_raw(root, &mut |n, entering| self.render_node(n, entering));
        std::mem::take(&mut self.base.out)
    }
}

const HEADING_ANCHOR_SVG: &str = "<svg viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\"><path fill-rule=\"evenodd\" d=\"M4 9h1v1H4c-1.5 0-3-1.69-3-3.5S2.55 3 4 3h4c1.45 0 3 1.69 3 3.5 0 1.41-.91 2.72-2 3.25V8.59c.58-.45 1-1.27 1-2.09C10 5.22 8.98 4 8 4H4c-.98 0-2 1.22-2 2.5S3 9 4 9zm9-3h-1v1h1c1 0 2 1.22 2 2.5S13.98 12 13 12H9c-.98 0-2-1.22-2-2.5 0-.83.42-1.64 1-2.09V6.25c-1.09.53-2 1.84-2 3.25C6 11.31 7.55 13 9 13h4c1.45 0 3-1.69 3-3.5S14.5 6 13 6z\"></path></svg>";

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
        *v = format!("{}text-align: {};", style, align);
        return;
    }
    attrs.push(("style".to_string(), format!("text-align: {};", align)));
}

fn need_use_html_table(table: Node) -> bool {
    let mut ret = false;
    crate::ast::walk_raw(table, &mut |n, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if NodeType::NodeTable == n.typ() {
            if !n.ial_attr("caption").is_empty() {
                ret = true;
                return WalkStatus::WalkStop;
            }
        } else if NodeType::NodeTableCell == n.typ() {
            for (k, v) in n.ial_raw() {
                if ("colspan" == k || "rowspan" == k) && "1" != v {
                    ret = true;
                    return WalkStatus::WalkStop;
                }
            }
        }
        WalkStatus::WalkContinue
    });
    ret
}

fn no_highlight_lang(language: &str) -> bool {
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

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
}

fn find_last_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).rposition(|w| w == sub)
}

fn trim_space(buf: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(buf);
    let trimmed = s.trim_matches(|c: char| c.is_whitespace());
    trimmed.as_bytes().to_vec()
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

fn is_text_mark_type(n: Node, typ: &str) -> bool {
    n.data().text_mark_type.split(' ').any(|t| t == typ)
}

fn is_next_same_inline_memo(n: Node) -> bool {
    let mut next_inline_memo = None;
    let mut node = n.next();
    while let Some(cur) = node {
        if n.next().is_none()
            || NodeType::NodeKramdownSpanIAL == cur.typ()
            || cur.next().is_none()
            || cur
                .next()
                .map(|x| NodeType::NodeKramdownSpanIAL == x.typ())
                .unwrap_or(true)
        {
            node = cur.next();
            continue;
        }
        if NodeType::NodeTextMark == cur.typ() && is_text_mark_type(cur, "inline-memo") {
            next_inline_memo = Some(cur);
            break;
        }
        node = cur.next();
    }
    match next_inline_memo {
        Some(m) => n.data().text_mark_inline_memo_content == m.data().text_mark_inline_memo_content,
        None => false,
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF)
}

fn text_mark_attrs(n: Node) -> Vec<(String, String)> {
    let d = n.data();
    let mut attrs = vec![("data-type".to_string(), d.text_mark_type.clone())];
    for typ in d.text_mark_type.split(' ') {
        if "block-ref" == typ {
            attrs.push((
                "data-subtype".to_string(),
                d.text_mark_block_ref_subtype.clone(),
            ));
            attrs.push(("data-id".to_string(), d.text_mark_block_ref_id.clone()));
        } else if "a" == typ {
            attrs.push(("data-href".to_string(), d.text_mark_a_href.clone()));
            if !d.text_mark_a_title.is_empty() {
                attrs.push(("data-title".to_string(), d.text_mark_a_title.clone()));
            }
        } else if "inline-math" == typ {
            attrs.push(("data-subtype".to_string(), "math".to_string()));
            let mut content = d.text_mark_inline_math_content.clone();
            if n.parent_is(&[NodeType::NodeTableCell]) {
                content = content.replace('|', "&#124;");
                content = content.replace('\n', "<br/>");
            }
            content = content.replace(editor::IAL_VAL_ESC_NEWLINE, "\n");
            attrs.push(("data-content".to_string(), content));
            attrs.push(("contenteditable".to_string(), "false".to_string()));
            attrs.push(("class".to_string(), "render-node".to_string()));
        } else if "file-annotation-ref" == typ {
            attrs.push((
                "data-id".to_string(),
                crate::html::escape_html_str(&d.text_mark_file_annotation_ref_id),
            ));
        } else if "inline-memo" == typ {
            let content = d
                .text_mark_inline_memo_content
                .replace(editor::IAL_VAL_ESC_NEWLINE, "\n");
            attrs.push(("data-inline-memo-content".to_string(), content));
        }
    }
    attrs
}

fn escape_ref_text(ref_text: &str) -> String {
    ref_text
        .replace('>', "&gt;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
