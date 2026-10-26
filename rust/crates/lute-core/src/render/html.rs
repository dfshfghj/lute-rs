// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! HTML 渲染器，对齐 Go 版 render.HtmlRenderer（JavaScript 构建行为：不做语法高亮）。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::lex;
use crate::options::RenderOptions;

use super::table::{append_table_cell_align_style, need_use_html_table, HtmlTableRenderer};

use super::{
    collect_footnotes_defs, eq_fold, escape_html_bytes, handle_kramdown_block_ial, heading_id,
    is_empty_str, last_deepest_child, BaseRenderer, Renderer,
};

/// HTML 渲染器。
pub struct HtmlRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    /// 解析选项，对应 Go 版 HtmlRenderer.ParseOptions。
    pub parse_options: crate::options::ParseOptions,
    footnotes_all: Vec<Node>,
    /// 对应 Go 版 HtmlRenderer.SetTextMarkStandardTag：用标准标签渲染文本标记。
    text_mark_standard_tag: bool,
}

impl HtmlRenderer {
    /// 创建 HTML 渲染器。
    pub fn new(tree: &Tree, options: RenderOptions) -> Self {
        Self::with_parse_options(tree, options, crate::options::ParseOptions::new())
    }

    /// 创建 HTML 渲染器，同时指定解析选项（提示块标题需要再次解析行级内容）。
    pub fn with_parse_options(
        tree: &Tree,
        options: RenderOptions,
        parse_options: crate::options::ParseOptions,
    ) -> Self {
        let root = tree.root_raw();
        Self {
            base: BaseRenderer::new(options),
            root,
            _owner: root.owner(),
            parse_options,
            footnotes_all: collect_footnotes_defs(root),
            text_mark_standard_tag: false,
        }
    }

    /// 对应 Go 版 HtmlRenderer.SetTextMarkStandardTag。
    pub fn set_text_mark_standard_tag(&mut self) {
        self.text_mark_standard_tag = true;
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some((s, st)) = self.base.ext_render(n, entering) {
            self.base.write_str(&s);
            return st;
        }
        let typ = n.typ();
        match typ {
            NodeType::NodeDocument => WalkStatus::WalkContinue,
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => {
                if entering {
                    let mut tokens = n.tokens_vec();
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        tokens = self.base.space(&tokens);
                    }
                    if self.base.options.fix_term_typo {
                        self.base.fix_term_typo(&mut tokens);
                    }
                    self.base.write(&escape_html_bytes(&tokens));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTextMark => {
                if self.text_mark_standard_tag {
                    self.render_text_mark_standard_tag(n, entering)
                } else {
                    self.render_text_mark(n, entering)
                }
            }
            NodeType::NodeCodeSpan => {
                if entering {
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        let mut prev = n.previous();
                        if prev.map(|p| p.typ()) == Some(NodeType::NodeKramdownSpanIAL) {
                            prev = prev.and_then(|p| p.previous());
                        }
                        let text = prev.map(|p| p.text()).unwrap_or_default();
                        if !text.is_empty() {
                            let lastc = text.chars().next_back().unwrap_or('\u{FFFD}');
                            if super::is_letter_or_digit(lastc) {
                                self.base.write_byte(b' ');
                            }
                        }
                    }
                } else if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                    let text = n.next().map(|p| p.text()).unwrap_or_default();
                    if !text.is_empty() {
                        let firstc = text.chars().next().unwrap_or('\u{FFFD}');
                        if super::is_letter_or_digit(firstc) {
                            self.base.write_byte(b' ');
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
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
                    self.base.write(&escape_html_bytes(n.tokens()));
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
            NodeType::NodeCodeBlockCode => self.render_code_block_code(n, entering),
            NodeType::NodeMathBlock => {
                self.base.newline();
                if entering {
                    handle_kramdown_block_ial(n, &self.base.options);
                    let mut attrs = vec![("class".to_string(), "language-math".to_string())];
                    attrs.extend(n.ial_raw());
                    self.base.tag("div", &attrs, false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlockOpenMarker => WalkStatus::WalkContinue,
            NodeType::NodeMathBlockContent => {
                if entering {
                    self.base.write(&escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlockCloseMarker => {
                if entering {
                    self.base.tag("/div", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMath => WalkStatus::WalkContinue,
            NodeType::NodeInlineMathOpenMarker => {
                if entering {
                    self.base.tag(
                        "span",
                        &[("class".to_string(), "language-math".to_string())],
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathContent => {
                if entering {
                    let mut tokens = n.tokens_vec();
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        tokens = super::replace_bytes(&tokens, b"\\|", b"|");
                    }
                    self.base.write(&escape_html_bytes(&tokens));
                }
                WalkStatus::WalkContinue
            }
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
            NodeType::NodeTag | NodeType::NodeMark => {
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
            NodeType::NodeBlockquote => {
                if entering {
                    self.base.newline();
                    handle_kramdown_block_ial(n, &self.base.options);
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
            NodeType::NodeTaskListItemMarker => {
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
            NodeType::NodeHTMLBlock => {
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
            NodeType::NodeInlineHTML => {
                if entering {
                    if self.base.options.sanitize {
                        let tokens = super::sanitizer::sanitize(n.tokens());
                        self.base.write(&tokens);
                    } else {
                        self.base.write(n.tokens());
                    }
                }
                WalkStatus::WalkContinue
            }
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
                if entering {
                    let mut tokens = n.tokens_vec();
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        tokens = self.base.space(&tokens);
                    }
                    self.base.write(&escape_html_bytes(&tokens));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmoji | NodeType::NodeEmojiAlias => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDefBlock => WalkStatus::WalkContinue,
            NodeType::NodeFootnotesDef => {
                if entering && !self.base.rendering_footnotes {
                    let mut found = false;
                    for d in &self.base.footnotes_defs {
                        if eq_fold(d.tokens(), n.tokens()) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        self.base.footnotes_defs.push(n);
                    }
                    return WalkStatus::WalkSkipChildren;
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesRef => {
                if entering {
                    let mut idx = 0usize;
                    let mut found = false;
                    for (i, d) in self.footnotes_all.iter().enumerate() {
                        if eq_fold(d.tokens(), n.tokens()) {
                            idx = i + 1;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        let idx_str = idx.to_string();
                        let ref_id = n.data().footnotes_ref_id.clone();
                        self.base.tag(
                            "sup",
                            &[
                                ("class".to_string(), "footnotes-ref".to_string()),
                                ("id".to_string(), format!("footnotes-ref-{ref_id}")),
                            ],
                            false,
                        );
                        self.base.tag(
                            "a",
                            &[(
                                "href".to_string(),
                                format!("{}#footnotes-def-{idx_str}", self.base.options.link_base),
                            )],
                            false,
                        );
                        self.base.write_str(&idx_str);
                        self.base.tag("/a", &[], false);
                        self.base.tag("/sup", &[], false);
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeToC => {
                if entering {
                    self.render_toc();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBackslash => WalkStatus::WalkContinue,
            NodeType::NodeBackslashContent => {
                if entering {
                    self.base.write(&escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.base.write(&escape_html_bytes(n.tokens()));
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
                    if let Some(parent) = n.parent() {
                        attrs.extend(parent.ial_raw());
                    }
                    self.base.tag("pre", &attrs, false);
                    self.base.write_str("<code class=\"language-yaml\">");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                if entering {
                    self.base.write(&escape_html_bytes(n.tokens()));
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterCloseMarker => {
                if entering {
                    self.base.write_str("</code></pre>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead => {
                if entering {
                    self.base.tag("thead", &[], false);
                    self.base.newline();
                } else {
                    self.base.tag("/thead", &[], false);
                    self.base.newline();
                    if n.next().is_some() {
                        self.base.tag("tbody", &[], false);
                    }
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
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
            NodeType::NodeKramdownBlockIAL
            | NodeType::NodeKramdownSpanIAL
            | NodeType::NodeLinkRefDef
            | NodeType::NodeLinkRefDefBlock => WalkStatus::WalkSkipChildren,
            NodeType::NodeGitConflict => {
                self.base.newline();
                if entering {
                    handle_kramdown_block_ial(n, &self.base.options);
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
                    self.base.write(&escape_html_bytes(n.tokens()));
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeIFrame => self.render_iframe(n, entering),
            NodeType::NodeWidget => self.render_widget(n, entering),
            NodeType::NodeVideo => self.render_video(n, entering),
            NodeType::NodeAudio => self.render_audio(n, entering),
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
            NodeType::NodeUnderline => WalkStatus::WalkContinue,
            NodeType::NodeUnderlineOpenMarker => {
                if entering {
                    self.base.tag(
                        "u",
                        &n.parent().map(|p| p.ial_raw()).unwrap_or_default(),
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeUnderlineCloseMarker => {
                if entering {
                    self.base.tag("/u", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBr => {
                if entering {
                    self.base.tag("br", &[], true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => {
                super::html::render_tabs_html(&mut self.base, n, entering, &self.parse_options)
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
                            String::from_utf8_lossy(&escape_html_bytes(n.tokens())).to_string(),
                        ),
                    ];
                    self.base.tag("div", &attrs, false);
                    self.base.tag("pre", &[], false);
                    self.base.write(&escape_html_bytes(n.tokens()));
                    self.base.write_str("</pre></div>");
                    self.base.newline();
                }
                WalkStatus::WalkSkipChildren
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
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeSuperBlock => WalkStatus::WalkContinue,
            NodeType::NodeSuperBlockOpenMarker
            | NodeType::NodeSuperBlockLayoutMarker
            | NodeType::NodeSuperBlockCloseMarker => WalkStatus::WalkSkipChildren,
            NodeType::NodeBlockRef | NodeType::NodeBlockRefID | NodeType::NodeBlockRefSpace => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefText => {
                if entering {
                    self.base.write_byte(b'"');
                    self.base.write(&escape_html_bytes(n.tokens()));
                } else {
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefDynamicText => {
                if entering {
                    self.base.write_byte(b'\'');
                    self.base.write(&escape_html_bytes(n.tokens()));
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
                    self.base.write(&escape_html_bytes(n.tokens()));
                } else {
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
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
            _ => WalkStatus::WalkContinue,
        }
    }

    /// 对应 Go 版 HtmlRenderer.renderCallout。
    fn render_callout(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.tag(
                "div",
                &[
                    ("class".to_string(), "callout".to_string()),
                    (
                        "data-subtype".to_string(),
                        crate::html::escape_html_str(&n.data().callout_type),
                    ),
                ],
                false,
            );
            self.base.newline();
            self.base
                .write_str("<div class=\"callout-info\"><span class=\"callout-icon\">");
            let icon_type = n.data().callout_icon_type;
            let icon = n.data().callout_icon.clone();
            if 0 == icon_type {
                self.base.write_str(&icon);
            } else if 1 == icon_type {
                self.base
                    .write_str("<img class=\"callout-img\" alt=\"\" src=\"");
                self.base.write_str(&crate::html::escape_html_str(&icon));
                self.base.write_str("\" />");
            }
            self.base.write_str("</span><span class=\"callout-title\">");
            let title = callout_display_title(n);
            let tree = crate::parse::inline("", title.as_bytes(), self.parse_options.clone());
            if tree.root_raw().first_child().is_some() {
                let first = tree.root_raw().first_child().unwrap();
                let inlines = first.children().collect::<Vec<_>>();
                first.unlink();
                for inline in inlines {
                    tree.root_raw().append_child(inline);
                }
                let mut r = HtmlRenderer::with_parse_options(
                    &tree,
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
            self.base.newline();
            self.base.write_str("</div>");
            self.base.newline();
            self.base.write_str("</div>");
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    /// 对应 Go 版 BaseRenderer.renderToC。
    fn render_toc(&mut self) {
        let headings = super::headings(self.root, &self.base.options);
        self.base
            .write_str("<div class=\"vditor-toc\" data-block=\"0\" data-type=\"toc-block\" contenteditable=\"false\">");
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

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if n != self.root && is_tab_title_block(n) {
            return WalkStatus::WalkSkipChildren;
        }
        if let Some(gp) = n.parent().and_then(|p| p.parent()) {
            if NodeType::NodeList == gp.typ()
                && gp
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
            self.base.newline();
            handle_kramdown_block_ial(n, &self.base.options);
            let mut attrs = n.ial_raw();
            if self.base.options.chinese_paragraph_beginning_space
                && n.parent().map(|p| p.typ()) == Some(NodeType::NodeDocument)
                && !self.paragraph_contain_img_only(n)
            {
                attrs.push(("class".to_string(), "indent--2".to_string()));
            }
            self.base.tag("p", &attrs, false);
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
                if !is_empty_str(&n.tokens_str()) {
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

    /// 对应 Go 版 HtmlRenderer.renderTextMark。
    fn render_text_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }

        let mut text_content = n.data().text_mark_text_content.clone();
        if n.parent_is(&[NodeType::NodeTableCell]) {
            if n.is_text_mark_type("code") {
                text_content = text_content.replace('|', "&#124;");
            } else {
                text_content = text_content.replace("\\|", "|");
            }
            text_content = text_content.replace('\n', "<br />");
        }

        if n.is_text_mark_type("a") {
            let mut attrs = vec![("href".to_string(), n.data().text_mark_a_href.clone())];
            if !n.data().text_mark_a_title.is_empty() {
                attrs.push(("title".to_string(), n.data().text_mark_a_title.clone()));
            }
            self.span_node_attrs(n, &mut attrs);
            self.base.tag("a", &attrs, false);
            self.base.write_str(&text_content);
            self.base.write_str("</a>");
        } else if n.is_text_mark_type("inline-memo") {
            self.base.write_str(&text_content);
            let last_rune = n
                .data()
                .text_mark_text_content
                .chars()
                .next_back()
                .unwrap_or('\u{FFFD}');
            if !n.data().text_mark_inline_memo_content.is_empty() {
                let memo = n.data().text_mark_inline_memo_content.clone();
                if crate::char_class::is_cjk(last_rune) {
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
            let mut attrs = self.render_text_mark_attrs(n);
            self.span_node_attrs(n, &mut attrs);
            self.base.tag("span", &attrs, false);
            self.base.write_str(&text_content);
            self.base.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    /// 对应 Go 版 HtmlRenderer.renderTextMarkStandardTag。
    fn render_text_mark_standard_tag(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }

        let style = n.ial_attr("style");
        if !style.is_empty() {
            self.base.tag(
                "span",
                &[("style".to_string(), crate::html::escape_attr_val(&style))],
                false,
            );
        }

        let types: Vec<String> = n
            .data()
            .text_mark_type
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        for typ in &types {
            match typ.as_str() {
                "a" => {
                    let href = self.base.link_path(n.data().text_mark_a_href.as_bytes());
                    let mut attrs = vec![(
                        "href".to_string(),
                        String::from_utf8_lossy(&href).to_string(),
                    )];
                    if !n.data().text_mark_a_title.is_empty() {
                        attrs.push(("title".to_string(), n.data().text_mark_a_title.clone()));
                    }
                    self.base.tag("a", &attrs, false);
                }
                "strong" | "em" | "code" | "s" | "mark" | "u" | "sup" | "sub" | "kbd" => {
                    self.base.tag(typ, &[], false);
                }
                "tag" => self.base.write_str("<mark>#"),
                "inline-math" => self.base.write_str("<span class=\"language-math\">$"),
                _ => {}
            }
        }

        let mut text_content = n.data().text_mark_text_content.clone();
        if n.is_text_mark_type("inline-math") {
            text_content = n.data().text_mark_inline_math_content.clone();
        }
        if n.parent_is(&[NodeType::NodeTableCell]) {
            if n.is_text_mark_type("code") {
                text_content = text_content.replace('|', "&#124;");
            } else {
                text_content = text_content.replace("\\|", "|");
            }
            text_content = text_content.replace('\n', "<br />");
        }
        self.base.write_str(&text_content);

        for typ in types.iter().rev() {
            match typ.as_str() {
                "a" | "strong" | "em" | "code" | "s" | "mark" | "u" | "sup" | "sub" | "kbd" => {
                    self.base.write_str("</");
                    self.base.write_str(typ);
                    self.base.write_str(">");
                }
                "tag" => self.base.write_str("#</mark>"),
                "inline-math" => self.base.write_str("$</span>"),
                _ => {}
            }
        }

        if n.is_text_mark_type("inline-memo") && !n.data().text_mark_inline_memo_content.is_empty()
        {
            let last_rune = n
                .data()
                .text_mark_text_content
                .chars()
                .next_back()
                .unwrap_or('\u{FFFD}');
            let memo = n.data().text_mark_inline_memo_content.clone();
            if crate::char_class::is_cjk(last_rune) {
                self.base.write_str("<sup>（");
                self.base.write_str(&memo);
                self.base.write_str("）</sup>");
            } else {
                self.base.write_str("<sup>(");
                self.base.write_str(&memo);
                self.base.write_str(")</sup>");
            }
        }

        if !style.is_empty() {
            self.base.write_str("</span>");
        }
        WalkStatus::WalkContinue
    }

    /// 对应 Go 版 HtmlRenderer.renderTextMarkAttrs。
    fn render_text_mark_attrs(&self, n: Node) -> Vec<(String, String)> {
        let mut attrs = vec![("data-type".to_string(), n.data().text_mark_type.clone())];
        for typ in n.data().text_mark_type.split(' ') {
            if "block-ref" == typ {
                attrs.push((
                    "data-subtype".to_string(),
                    n.data().text_mark_block_ref_subtype.clone(),
                ));
                attrs.push((
                    "data-id".to_string(),
                    n.data().text_mark_block_ref_id.clone(),
                ));
            } else if "a" == typ {
                let href = self.base.link_path(n.data().text_mark_a_href.as_bytes());
                attrs.push((
                    "data-href".to_string(),
                    String::from_utf8_lossy(&href).to_string(),
                ));
                if !n.data().text_mark_a_title.is_empty() {
                    attrs.push(("data-title".to_string(), n.data().text_mark_a_title.clone()));
                }
            } else if "inline-math" == typ {
                attrs.push(("data-subtype".to_string(), "math".to_string()));
                let mut content = n.data().text_mark_inline_math_content.clone();
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    content = content.replace('|', "&#124;");
                    content = content.replace('\n', "<br/>");
                }
                content = content.replace(crate::editor::IAL_VAL_ESC_NEWLINE, "\n");
                attrs.push(("data-content".to_string(), content));
                attrs.push(("contenteditable".to_string(), "false".to_string()));
                attrs.push(("class".to_string(), "render-node".to_string()));
            } else if "file-annotation-ref" == typ {
                attrs.push((
                    "data-id".to_string(),
                    crate::html::escape_html_str(&n.data().text_mark_file_annotation_ref_id),
                ));
            } else if "inline-memo" == typ {
                let content = n
                    .data()
                    .text_mark_inline_memo_content
                    .replace(crate::editor::IAL_VAL_ESC_NEWLINE, "\n");
                attrs.push(("data-inline-memo-content".to_string(), content));
            }
        }
        attrs
    }

    /// 对应 Go 版 HtmlRenderer.spanNodeAttrs。
    fn span_node_attrs(&self, n: Node, attrs: &mut Vec<(String, String)>) {
        for (k, v) in &n.data().kramdown_ial {
            attrs.push((k.clone(), v.clone()));
        }
    }

    fn render_code_block(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.base.newline();
        if !n.data().is_fenced_code_block {
            if entering {
                self.base.write_str("<pre><code>");
                let tokens = n.first_child().map(|c| c.tokens_vec()).unwrap_or_default();
                self.base.write(&escape_html_bytes(&tokens));
                self.base.write_str("</code></pre>");
                self.base.newline();
                return WalkStatus::WalkSkipChildren;
            }
            return WalkStatus::WalkContinue;
        }
        WalkStatus::WalkContinue
    }

    fn render_code_block_code(&mut self, n: Node, entering: bool) -> WalkStatus {
        let mut language = String::new();
        let info = n
            .previous()
            .map(|p| p.data().code_block_info.clone())
            .unwrap_or_default();
        if !info.is_empty() {
            let words = lex::split(&info, b' ');
            if let Some(first) = words.first() {
                language = String::from_utf8_lossy(first).to_string();
            }
        }
        let pre_div = no_highlight(&language)
            && !(self.base.options.protyle_wysiwyg && language == "mindmap");

        if entering {
            self.base.newline();
            handle_kramdown_block_ial(n, &self.base.options);
            let attrs = n.ial_raw();
            if !pre_div {
                self.base.tag("pre", &attrs, false);
            }
            let tokens = n.tokens_vec();
            if !info.is_empty() {
                if pre_div && language == "mindmap" {
                    let json =
                        super::mindmap::echarts_mindmap_str(&String::from_utf8_lossy(&tokens));
                    self.base.write_str("<div data-code=\"");
                    self.base
                        .write(&crate::html::encode_destination(json.as_bytes()));
                    self.base.write_str("\" class=\"language-mindmap\">");
                } else if pre_div {
                    self.base.write_str("<div class=\"language-");
                    self.base.write_str(&language);
                    self.base.write_str("\">");
                } else {
                    self.base.write_str("<code class=\"language-");
                    self.base.write_str(&language);
                    self.base.write_str("\">");
                }
                self.base.write(&escape_html_bytes(&tokens));
            } else {
                self.base.write_str("<code>");
                self.base.write(&escape_html_bytes(&tokens));
            }
        } else {
            if pre_div {
                self.base.write_str("</div>");
            } else {
                self.base.write_str("</code></pre>");
            }
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        let level = n.data().heading_level.clamp(1, 6) as usize;
        if entering {
            self.base.newline();
            self.base.write_str(&format!("<h{level}"));
            let id = heading_id(n);
            if self.base.options.toc
                || self.base.options.heading_id
                || (crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial)
            {
                self.base.write_str(" id=\"");
                self.base.write_str(&id);
                self.base.write_str("\"");
                if crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial {
                    let ial = n.data().kramdown_ial.clone();
                    let id_render_name = self.base.options.kramdown_ial_id_render_name.clone();
                    if "id" != id_render_name && !ial.is_empty() {
                        self.base.write_str(" ");
                        self.base.write_str(&id_render_name);
                        self.base.write_str("=\"");
                        self.base.write_str(&ial[0].1);
                        self.base.write_str("\"");
                    }
                    if 1 < ial.len() {
                        for (k, v) in &ial {
                            if "id" == k {
                                continue;
                            }
                            self.base.write_str(" ");
                            self.base.write_str(k);
                            self.base.write_str("=\"");
                            self.base.write_str(v);
                            self.base.write_str("\"");
                        }
                    }
                }
            }
            self.base.write_str(">");
        } else {
            if self.base.options.heading_anchor {
                let id = heading_id(n);
                self.base.tag(
                    "a",
                    &[
                        ("id".to_string(), format!("vditorAnchor-{id}")),
                        ("class".to_string(), "vditor-anchor".to_string()),
                        ("href".to_string(), format!("#{id}")),
                    ],
                    false,
                );
                self.base.write_str("<svg viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\"><path fill-rule=\"evenodd\" d=\"M4 9h1v1H4c-1.5 0-3-1.69-3-3.5S2.55 3 4 3h4c1.45 0 3 1.69 3 3.5 0 1.41-.91 2.72-2 3.25V8.59c.58-.45 1-1.27 1-2.09C10 5.22 8.98 4 8 4H4c-.98 0-2 1.22-2 2.5S3 9 4 9zm9-3h-1v1h1c1 0 2 1.22 2 2.5S13.98 12 13 12H9c-.98 0-2-1.22-2-2.5 0-.83.42-1.64 1-2.09V6.25c-1.09.53-2 1.84-2 3.25C6 11.31 7.55 13 9 13h4c1.45 0 3-1.69 3-3.5S14.5 6 13 6z\"></path></svg>");
                self.base.tag("/a", &[], false);
            }
            self.base.write_str(&format!("</h{level}>"));
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
            let mut attrs: Vec<(String, String)> = Vec::new();
            if self.base.options.render_list_style {
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
            if 0 == ld.bullet_char && 1 != ld.start {
                attrs.push(("start".to_string(), ld.start.to_string()));
            }
            handle_kramdown_block_ial(n, &self.base.options);
            attrs.extend(n.ial_raw());
            self.base.tag(tag, &attrs, false);
            self.base.newline();
        } else {
            self.base.newline();
            self.base.tag(&format!("/{tag}"), &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_list_item(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            handle_kramdown_block_ial(n, &self.base.options);
            let mut attrs = n.ial_raw();
            let ld = n.data().list_data.clone().unwrap_or_default();
            if 3 == ld.typ
                && !self.base.options.gfm_task_list_item_class.is_empty()
                && n.first_child().is_some()
            {
                let first = n.first_child().unwrap();
                let marker = if NodeType::NodeTaskListItemMarker == first.typ() {
                    Some(first)
                } else {
                    first
                        .first_child()
                        .filter(|c| NodeType::NodeTaskListItemMarker == c.typ())
                };
                if let Some(marker) = marker {
                    let mut task_class = self.base.options.gfm_task_list_item_class.clone();
                    if marker.data().task_list_item_checked {
                        task_class.push_str(" vditor-task--done");
                    }
                    attrs.push(("class".to_string(), task_class));
                }
            }
            self.base.tag("li", &attrs, false);
        } else {
            self.base.tag("/li", &[], false);
            self.base.newline();
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
        let marker = self.normalized_task_list_item_marker(n);
        marker != " "
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if need_use_html_table(n) {
            if entering {
                self.base.newline();
                handle_kramdown_block_ial(n, &self.base.options);
                self.render_table_by_html(n);
                return WalkStatus::WalkSkipChildren;
            }
            return WalkStatus::WalkContinue;
        }

        if entering {
            handle_kramdown_block_ial(n, &self.base.options);
            self.base.tag("table", &n.ial_raw(), false);
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

    fn render_table_cell(&mut self, n: Node, entering: bool) -> WalkStatus {
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
            let mut attrs: Vec<(String, String)> = n
                .ial_raw()
                .into_iter()
                .filter(|(k, _)| k == "style")
                .collect();
            append_table_cell_align_style(n, &mut attrs);
            self.base.tag(tag, &attrs, false);
        } else {
            self.base.tag(&format!("/{tag}"), &[], false);
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_link(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.link_text_auto_space_previous(n);
            let mut dest_tokens = n
                .child_by_type(NodeType::NodeLinkDest)
                .map(|d| d.tokens_vec())
                .unwrap_or_default();
            if self.base.options.sanitize {
                let lower = String::from_utf8_lossy(&dest_tokens).trim().to_lowercase();
                if lower.starts_with("javascript:") {
                    dest_tokens = Vec::new();
                }
            }
            let dest_tokens = self.base.link_path(&dest_tokens);
            let mut attrs = vec![(
                "href".to_string(),
                String::from_utf8_lossy(&escape_html_bytes(&dest_tokens)).to_string(),
            )];
            if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                if !title.tokens().is_empty() {
                    attrs.push((
                        "title".to_string(),
                        String::from_utf8_lossy(&escape_html_bytes(title.tokens())).to_string(),
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
            if 0 == self.base.disable_tags {
                let parent_style = n.ial_attr("parent-style");
                if !parent_style.is_empty() {
                    self.base
                        .tag("span", &[("style".to_string(), parent_style)], false);
                }

                self.base.write_str("<img src=\"");
                let mut dest_tokens = n
                    .child_by_type(NodeType::NodeLinkDest)
                    .map(|d| d.tokens_vec())
                    .unwrap_or_default();
                dest_tokens = self.base.link_path(&dest_tokens);
                if !self.base.options.image_lazy_loading.is_empty() {
                    self.base.write(&escape_html_bytes(
                        self.base.options.image_lazy_loading.as_bytes(),
                    ));
                    self.base.write_str("\" data-src=\"");
                }
                self.base.write(&escape_html_bytes(&dest_tokens));
                self.base.write_str("\" alt=\"");
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
                    self.base.write(&escape_html_bytes(title.tokens()));
                    self.base.write_byte(b'"');
                }
            }
            let ial = self.base.node_attrs_str(n);
            if !ial.is_empty() {
                self.base.write_str(" ");
                self.base.write_str(&ial);
            }
            self.base.write_str(" />");
            if !n.ial_attr("style").is_empty() {
                self.base.tag("/span", &[], false);
            }

            if self.base.options.sanitize {
                let out = std::mem::take(&mut self.base.out);
                let idx = out
                    .windows(b"<img src=".len())
                    .rposition(|w| w == b"<img src=");
                self.base.out = out;
                if let Some(idx) = idx {
                    let img_buf = super::sanitizer::sanitize(&self.base.out[idx..]);
                    self.base.out.truncate(idx);
                    self.base.write(&img_buf);
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_iframe(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.write(n.tokens());
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_widget(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.write(n.tokens());
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_video(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.tag(
                "video",
                &[
                    ("src".to_string(), n.tokens_str()),
                    ("controls".to_string(), "controls".to_string()),
                ],
                false,
            );
            self.base.write_str("</video>");
            self.base.newline();
        }
        WalkStatus::WalkContinue
    }

    fn render_audio(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.base.newline();
            self.base.tag(
                "audio",
                &[
                    ("src".to_string(), n.tokens_str()),
                    ("controls".to_string(), "controls".to_string()),
                ],
                false,
            );
            self.base.write_str("</audio>");
            self.base.newline();
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
            origin = format!("https:{origin}");
        }
        let mut ret = tokens[..idx + marker.len()].to_vec();
        ret.extend_from_slice(origin.as_bytes());
        ret.extend_from_slice(&tokens[idx + marker.len() + src.len()..]);
        ret
    }

    /// 对应 Go 版 RenderFootnotes。
    fn render_footnotes(&mut self) -> Vec<u8> {
        if self.base.footnotes_defs.is_empty() {
            return Vec::new();
        }

        let mut buf = Vec::new();
        buf.extend_from_slice(b"<div class=\"footnotes-defs-div\">");
        buf.extend_from_slice(b"<hr class=\"footnotes-defs-hr\" />\n");
        buf.extend_from_slice(b"<ol class=\"footnotes-defs-ol\">");
        let defs = self.base.footnotes_defs.clone();
        for (i, def) in defs.iter().enumerate() {
            buf.extend_from_slice(format!("<li id=\"footnotes-def-{}\">", i + 1).as_bytes());

            let mut tree = Tree::new();
            tree.root_raw().append_child(*def);
            let lc = last_deepest_child(tree.root_raw());
            let refs = def.data().footnotes_refs.clone();
            for r in refs.iter().rev() {
                let ref_id = r.data().footnotes_ref_id.clone();
                let goto_ref = format!(
                    " <a href=\"#footnotes-ref-{ref_id}\" class=\"vditor-footnotes__goto-ref\">↩</a>"
                );
                let link = tree.new_node_raw(NodeType::NodeInlineHTML);
                link.set_tokens(goto_ref.into_bytes());
                lc.insert_after(link);
            }

            let mut r = HtmlRenderer::new(&tree, self.base.options.clone());
            r.base.rendering_footnotes = true;
            let content = r.render();
            buf.extend_from_slice(&content);
            buf.extend_from_slice(b"</li>\n");
        }
        buf.extend_from_slice(b"</ol></div>");
        buf
    }
}

impl HtmlTableRenderer for HtmlRenderer {
    fn table_base(&mut self) -> &mut BaseRenderer {
        &mut self.base
    }
    fn render_table_node(&mut self, node: Node, entering: bool) -> WalkStatus {
        self.render_node(node, entering)
    }
}

impl Renderer for HtmlRenderer {
    fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);

        let root = self.root;
        let me = &mut *self;
        crate::ast::walk_raw(root, &mut |n, entering| me.render_node(n, entering));

        let mut output = std::mem::take(&mut self.base.out);
        output.extend(self.render_footnotes());
        output
    }
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

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
}

/// 对应 Go 版 render.calloutDisplayTitle。
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

/// Shared HTML/preview implementation, corresponding to Go BaseRenderer.renderTabsHTML.
pub(super) fn render_tabs_html(
    base: &mut super::BaseRenderer,
    node: Node,
    entering: bool,
    parse_options: &crate::options::ParseOptions,
) -> WalkStatus {
    if entering {
        let class = if node.typ() == NodeType::NodeTabs {
            "tabs"
        } else {
            "tab-item"
        };
        let mut attrs = vec![
            ("class".to_owned(), class.to_owned()),
            ("data-type".to_owned(), node.typ().as_str().to_owned()),
        ];
        if !node.id().is_empty() {
            attrs.push(("data-node-id".to_owned(), node.id_owned()));
            attrs.push(("id".to_owned(), node.id_owned()));
        }
        for name in ["tabs-active-id", "tabs-position", "tabs-task"] {
            let value = node.ial_attr(name);
            if !value.is_empty() {
                attrs.push((name.to_owned(), crate::html::escape_html_str(&value)));
            }
        }
        base.tag("div", &attrs, false);
        if node.typ() == NodeType::NodeTabs {
            base.write_str("<div class=\"tabs-header protyle-action\"></div>");
        } else {
            let title = node
                .children()
                .find(|n| n.typ() != NodeType::NodeKramdownBlockIAL)
                .filter(|n| {
                    n.typ() == NodeType::NodeParagraph && n.ial_attr("tabs-title") == "true"
                });
            if let Some(title) = title {
                base.write_str("<div class=\"tab-item-info callout-info\"><div class=\"tab-item-title callout-title\">");
                let tree = Tree::new();
                let mut renderer = HtmlRenderer::with_parse_options(
                    &tree,
                    base.options.clone(),
                    parse_options.clone(),
                );
                renderer.root = title;
                renderer._owner = title.owner();
                base.write(&renderer.render());
                base.write_str("</div></div><div class=\"tab-item-content\">");
            } else {
                base.write_str("<div class=\"tab-item-info callout-info\"><span class=\"tab-item-title callout-title\">");
                let tree = crate::parse::inline(
                    "",
                    node.data().tab_item_title.as_bytes(),
                    parse_options.clone(),
                );
                if let Some(paragraph) = tree.root_raw().first_child() {
                    let inlines = paragraph.children().collect::<Vec<_>>();
                    paragraph.unlink();
                    for inline in inlines {
                        tree.root_raw().append_child(inline);
                    }
                }
                let mut renderer = HtmlRenderer::with_parse_options(
                    &tree,
                    base.options.clone(),
                    parse_options.clone(),
                );
                base.write(&renderer.render());
                base.write_str("</span></div><div class=\"tab-item-content\">");
            }
        }
    } else {
        if node.typ() == NodeType::NodeTabItem {
            base.write_str("</div>");
        }
        base.write_str("</div>");
        base.newline();
    }
    WalkStatus::WalkContinue
}

#[cfg(test)]
mod code_block_source_tests {
    use super::HtmlRenderer;
    use crate::ast::{NodeType, Tree};
    use crate::lute::Lute;
    use crate::options::RenderOptions;
    use crate::render::table::HtmlTableRenderer;

    #[test]
    fn html_table_row_closes_after_its_cells() {
        let mut tree = Tree::new();
        let row = tree.new_node_raw(NodeType::NodeTableRow);
        let cell = tree.new_node_raw(NodeType::NodeTableCell);
        let text = tree.new_node_raw(NodeType::NodeText);
        text.set_tokens(b"value".to_vec());
        tree.root_raw().append_child(row);
        row.append_child(cell);
        cell.append_child(text);
        let mut renderer = HtmlRenderer::new(&tree, RenderOptions::default());
        renderer.render_table_row_html(row);
        assert_eq!(renderer.base.out, b"<tr>\n<td>value</td>\n</tr>\n");
    }

    #[test]
    fn mindmap_code_uses_encoded_tree_data_and_preserves_source() {
        let engine = Lute::new();
        assert_eq!(
            engine.markdown("", "```mindmap\n* a & b\n```"),
            "<div data-code=\"%7B%22name%22:%20%22a%20&%20b%22%7D\" class=\"language-mindmap\">* a &amp; b\n</div>\n"
        );
        assert_eq!(
            engine.markdown("", "```mindmap\nordinary text\n```"),
            "<div data-code=\"%7B%7D\" class=\"language-mindmap\">ordinary text\n</div>\n"
        );
    }

    #[test]
    fn protyle_mindmap_code_keeps_standard_code_container() {
        let mut engine = Lute::new();
        engine.render_options.protyle_wysiwyg = true;
        assert_eq!(
            engine.markdown("", "```mindmap\n* foo\n```"),
            "<pre><code class=\"language-mindmap\">* foo\n</code></pre>\n"
        );
    }
}
