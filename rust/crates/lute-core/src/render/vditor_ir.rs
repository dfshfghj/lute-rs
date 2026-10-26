// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Vditor Instant-Rendering DOM 渲染器，对齐 Go 版 render.VditorIRRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::editor;
use crate::html as html_util;
use crate::options::{ParseOptions, RenderOptions};

use super::{BaseRenderer, Renderer};

pub struct VditorIRRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
}

impl VditorIRRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
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
            NodeType::NodeCodeSpan => self.render_wrap_span(n, entering),
            NodeType::NodeCodeSpanOpenMarker => self.render_code_span_open_marker(n, entering),
            NodeType::NodeCodeSpanContent => {
                if entering {
                    let escaped = html_util::escape_html(&n.tokens_str());
                    self.base.write(escaped.as_bytes());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanCloseMarker => self.render_code_span_close_marker(n, entering),
            NodeType::NodeCodeBlock => self.render_div_node(n, entering),
            NodeType::NodeCodeBlockFenceOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "code-block-open-marker")], false);
                    self.base.write(&n.tokens_vec());
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockFenceInfoMarker => {
                if entering {
                    self.tag(
                        "span",
                        &[
                            ("class", "vditor-ir__marker vditor-ir__marker--info"),
                            ("data-type", "code-block-info"),
                        ],
                        false,
                    );
                    self.base.write_str(editor::ZWSP);
                    self.base.write(&n.data().code_block_info.clone());
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockFenceCloseMarker => {
                if entering {
                    self.tag("span", &[("data-type", "code-block-close-marker")], false);
                    self.base.write(&n.tokens_vec());
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockCode => self.render_code_block_code(n, entering),
            NodeType::NodeMathBlock => self.render_div_node(n, entering),
            NodeType::NodeMathBlockOpenMarker => {
                if entering {
                    self.tag("span", &[("data-type", "math-block-open-marker")], false);
                    self.base.write_str("$$");
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlockContent => self.render_math_block_content(n, entering),
            NodeType::NodeMathBlockCloseMarker => {
                if entering {
                    self.tag("span", &[("data-type", "math-block-close-marker")], false);
                    self.base.write_str("$$");
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMath => self.render_wrap_span(n, entering),
            NodeType::NodeInlineMathOpenMarker => {
                if entering {
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    self.base.write_byte(b'$');
                    self.tag("/span", &[], false);
                    self.tag(
                        "code",
                        &[
                            ("data-newline", "1"),
                            ("class", "vditor-ir__marker vditor-ir__marker--pre"),
                            ("data-type", "math-inline"),
                        ],
                        false,
                    );
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathContent => self.render_inline_math_content(n, entering),
            NodeType::NodeInlineMathCloseMarker => {
                if entering {
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    self.base.write_byte(b'$');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmphasis => self.render_wrap_span(n, entering),
            NodeType::NodeEmA6kOpenMarker => {
                if entering {
                    self.render_bi_marker("*", "em", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmA6kCloseMarker => {
                if entering {
                    self.render_bi_marker("*", "em", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmU8eOpenMarker => {
                if entering {
                    self.render_bi_marker("_", "em", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmU8eCloseMarker => {
                if entering {
                    self.render_bi_marker("_", "em", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrong => self.render_wrap_span(n, entering),
            NodeType::NodeStrongA6kOpenMarker => {
                if entering {
                    self.render_bi_marker("**", "strong", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kCloseMarker => {
                if entering {
                    self.render_bi_marker("**", "strong", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongU8eOpenMarker => {
                if entering {
                    self.render_bi_marker("__", "strong", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongU8eCloseMarker => {
                if entering {
                    self.render_bi_marker("__", "strong", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote => {
                if entering {
                    self.base.write_str("<blockquote data-block=\"0\">");
                } else {
                    self.base.write_str("</blockquote>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeCallout => self.render_callout(n, entering),
            NodeType::NodeTabs | NodeType::NodeTabItem => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeadingID => {
                if entering {
                    self.tag(
                        "span",
                        &[("data-type", "heading-id"), ("class", "vditor-ir__marker")],
                        false,
                    );
                    let mut s = String::from(" {");
                    s.push_str(&n.tokens_str());
                    s.push('}');
                    self.base.write_str(&s);
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeList => self.render_list(n, entering),
            NodeType::NodeListItem => self.render_list_item(n, entering),
            NodeType::NodeMindmap => self.render_list(n, entering),
            NodeType::NodeMindmapItem => self.render_list_item(n, entering),
            NodeType::NodeThematicBreak => {
                if entering {
                    self.tag("hr", &[("data-block", "0")], true);
                }
                WalkStatus::WalkContinue
            }
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
            NodeType::NodeLink => self.render_wrap_span(n, entering),
            NodeType::NodeImage => self.render_image(n, entering),
            NodeType::NodeBang => {
                if entering {
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    self.base.write_byte(b'!');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeOpenBracket => {
                if entering {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--bracket")],
                        false,
                    );
                    self.base.write_byte(b'[');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseBracket => self.render_close_bracket(n, entering),
            NodeType::NodeOpenParen => {
                if entering && !self.is_link_ref(n) {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--paren")],
                        false,
                    );
                    self.base.write_byte(b'(');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseParen => {
                if entering && !self.is_link_ref(n) {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--paren")],
                        false,
                    );
                    self.base.write_byte(b')');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeOpenBrace => {
                if entering {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--brace")],
                        false,
                    );
                    self.base.write_byte(b'{');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseBrace => {
                if entering {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--brace")],
                        false,
                    );
                    self.base.write_byte(b'}');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkText => self.render_link_text(n, entering),
            NodeType::NodeLinkSpace => {
                if entering && !self.is_link_ref(n) {
                    self.base.write_byte(b' ');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkDest => {
                if entering && !self.is_link_ref(n) {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--link")],
                        false,
                    );
                    let mut dest = n.tokens_vec();
                    if self.base.options.sanitize {
                        let tokens = trim_ascii_space(&dest).to_vec();
                        let lower = String::from_utf8_lossy(&tokens).to_lowercase();
                        if lower.starts_with("javascript:") {
                            dest.clear();
                        }
                    }
                    let escaped = html_util::escape_html(&String::from_utf8_lossy(&dest));
                    self.base.write(escaped.as_bytes());
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkTitle => {
                if entering && !self.is_link_ref(n) {
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--title")],
                        false,
                    );
                    self.base.write_byte(b'"');
                    self.base.write(&n.tokens_vec());
                    self.base.write_byte(b'"');
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough => self.render_wrap_span(n, entering),
            NodeType::NodeStrikethrough1OpenMarker => {
                if entering {
                    self.render_s_marker("~", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough1CloseMarker => {
                if entering {
                    self.render_s_marker("~", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough2OpenMarker => {
                if entering {
                    self.render_s_marker("~~", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough2CloseMarker => {
                if entering {
                    self.render_s_marker("~~", false);
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
            NodeType::NodeEmoji => {
                if entering {
                    let mut previous_node_text = previous_node_text(n).unwrap_or_default();
                    previous_node_text = previous_node_text.replace(editor::CARET, "");
                    if previous_node_text.is_empty() {
                        self.base.write_str(editor::ZWSP);
                    }
                    self.render_span_node(n);
                } else {
                    self.base.write_str("</span>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                if entering {
                    self.base.write_str("<span data-render=\"2\">");
                    self.base.write(&n.tokens_vec());
                    self.base.write_str("</span>");
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    if let Some(first) = n.first_child() {
                        self.base.write(&first.tokens_vec());
                    }
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmojiAlias => WalkStatus::WalkContinue,
            NodeType::NodeFootnotesDefBlock => {
                if entering {
                    self.base
                        .write_str("<div data-block=\"0\" data-type=\"footnotes-block\">");
                } else {
                    self.base.write_str("</div>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDef => self.render_footnotes_def(n, entering),
            NodeType::NodeFootnotesRef => self.render_footnotes_ref(n, entering),
            NodeType::NodeToC => self.render_toc(n, entering),
            NodeType::NodeBackslash => {
                if entering {
                    self.render_wrap_span(n, true);
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__marker vditor-ir__marker--bi")],
                        false,
                    );
                    self.base.write_byte(b'\\');
                    self.tag("/span", &[], false);
                } else {
                    self.render_wrap_span(n, false);
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
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.render_span_node(n);
                    self.tag(
                        "code",
                        &[
                            ("data-newline", "1"),
                            ("class", "vditor-ir__marker vditor-ir__marker--pre"),
                            ("data-type", "html-entity"),
                        ],
                        false,
                    );
                    let escaped = html_util::escape_html(&String::from_utf8_lossy(
                        &n.data().html_entity_tokens,
                    ));
                    self.base.write(escaped.as_bytes());
                    self.tag("/code", &[], false);
                    self.tag(
                        "span",
                        &[("class", "vditor-ir__preview"), ("data-render", "2")],
                        false,
                    );
                    self.tag("code", &[], false);
                    self.base.write(&n.data().html_entity_tokens);
                    self.tag("/code", &[], false);
                    self.tag("/span", &[], false);
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatter => self.render_div_node(n, entering),
            NodeType::NodeYamlFrontMatterOpenMarker => {
                if entering {
                    self.tag(
                        "span",
                        &[("data-type", "yaml-front-matter-open-marker")],
                        false,
                    );
                    self.base.write_str("---");
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                self.render_yaml_front_matter_content(n, entering)
            }
            NodeType::NodeYamlFrontMatterCloseMarker => {
                if entering {
                    self.tag(
                        "span",
                        &[("data-type", "yaml-front-matter-close-marker")],
                        false,
                    );
                    self.base.write_str("---");
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark => self.render_wrap_span(n, entering),
            NodeType::NodeMark1OpenMarker => {
                if entering {
                    self.render_mark_marker("=", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark1CloseMarker => {
                if entering {
                    self.render_mark_marker("=", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark2OpenMarker => {
                if entering {
                    self.render_mark_marker("==", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark2CloseMarker => {
                if entering {
                    self.render_mark_marker("==", false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSup => self.render_wrap_span(n, entering),
            NodeType::NodeSupOpenMarker => {
                if entering {
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    self.base.write_str("^");
                    self.tag("/span", &[], false);
                    self.tag("sup", &[("data-newline", "1")], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSupCloseMarker => {
                if entering {
                    self.tag("/sup", &[], false);
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    self.base.write_str("^");
                    self.tag("/span", &[], false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSub => self.render_wrap_span(n, entering),
            NodeType::NodeSubOpenMarker => {
                if entering {
                    self.render_bi_marker("~", "sub", true);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSubCloseMarker => {
                if entering {
                    self.render_bi_marker("~", "sub", false);
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
            NodeType::NodeLinkRefDef => {
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
            _ => WalkStatus::WalkContinue,
        }
    }

    fn is_link_ref(&self, n: Node) -> bool {
        n.parent()
            .map(|p| NodeType::NodeLink == p.typ() && 3 == p.data().link_type)
            .unwrap_or(false)
    }

    fn render_wrap_span(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.render_span_node(n);
        } else {
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    /// 删除线标记：与 Go 版一致，使用普通 marker（不是 --bi）。
    fn render_s_marker(&mut self, marker: &str, open: bool) -> WalkStatus {
        if open {
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            self.base.write_str(marker);
            self.tag("/span", &[], false);
            self.tag("s", &[("data-newline", "1")], false);
        } else {
            self.tag("/s", &[], false);
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            self.base.write_str(marker);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_bi_marker(&mut self, marker: &str, tag: &str, open: bool) -> WalkStatus {
        if open {
            self.tag(
                "span",
                &[("class", "vditor-ir__marker vditor-ir__marker--bi")],
                false,
            );
            self.base.write_str(marker);
            self.tag("/span", &[], false);
            self.tag(tag, &[("data-newline", "1")], false);
        } else {
            let s = format!("/{}", tag);
            self.tag(&s, &[], false);
            self.tag(
                "span",
                &[("class", "vditor-ir__marker vditor-ir__marker--bi")],
                false,
            );
            self.base.write_str(marker);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_mark_marker(&mut self, marker: &str, open: bool) -> WalkStatus {
        if open {
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            self.base.write_str(marker);
            self.tag("/span", &[], false);
            self.tag("mark", &[("data-newline", "1")], false);
        } else {
            self.tag("/mark", &[], false);
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            self.base.write_str(marker);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_span_node(&mut self, n: Node) {
        let text = self.text(n);
        let mut attrs: Vec<(String, String)> = Vec::new();
        let data_type = match n.typ() {
            NodeType::NodeEmphasis => "em",
            NodeType::NodeStrong => "strong",
            NodeType::NodeStrikethrough => "s",
            NodeType::NodeMark => "mark",
            NodeType::NodeSup => "sup",
            NodeType::NodeSub => "sub",
            NodeType::NodeLink => {
                if 3 != n.data().link_type {
                    "a"
                } else {
                    "link-ref"
                }
            }
            NodeType::NodeImage => "img",
            NodeType::NodeCodeSpan => "code",
            NodeType::NodeEmoji => "emoji",
            NodeType::NodeInlineHTML => "html-inline",
            NodeType::NodeHTMLEntity => "html-entity",
            NodeType::NodeBackslash => "backslash",
            _ => "inline-node",
        };
        attrs.push(("data-type".into(), data_type.into()));

        if text.contains(editor::CARET) {
            attrs.push((
                "class".into(),
                "vditor-ir__node vditor-ir__node--expand".into(),
            ));
        } else {
            let pre_text = previous_node_text(n).unwrap_or_default();
            let next_text = next_node_text(n).unwrap_or_default();
            if pre_text.ends_with(editor::CARET) || next_text.starts_with(editor::CARET) {
                attrs.push((
                    "class".into(),
                    "vditor-ir__node vditor-ir__node--expand".into(),
                ));
            } else {
                attrs.push(("class".into(), "vditor-ir__node".into()));
            }
        }
        self.tag_owned("span", &attrs, false);
    }

    fn render_div_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let text = self.text(n);
            let mut attrs: Vec<(String, String)> = vec![("data-block".into(), "0".into())];
            let data_type = match n.typ() {
                NodeType::NodeCodeBlock => Some("code-block"),
                NodeType::NodeHTMLBlock => Some("html-block"),
                NodeType::NodeMathBlock => Some("math-block"),
                NodeType::NodeYamlFrontMatter => Some("yaml-front-matter"),
                _ => None,
            };
            if let Some(t) = data_type {
                attrs.push(("data-type".into(), t.into()));
            }
            if text.contains(editor::CARET) {
                attrs.push((
                    "class".into(),
                    "vditor-ir__node vditor-ir__node--expand".into(),
                ));
            } else {
                attrs.push(("class".into(), "vditor-ir__node".into()));
            }
            self.tag_owned("div", &attrs, false);
        } else {
            self.base.write_str("</div>");
        }
        WalkStatus::WalkContinue
    }

    fn text(&self, n: Node) -> String {
        let mut ret = String::new();
        crate::ast::walk_raw(n, &mut |n, entering| {
            if entering {
                match n.typ() {
                    NodeType::NodeText
                    | NodeType::NodeLinkText
                    | NodeType::NodeLinkDest
                    | NodeType::NodeLinkTitle
                    | NodeType::NodeCodeBlockCode
                    | NodeType::NodeCodeSpanContent
                    | NodeType::NodeInlineMathContent
                    | NodeType::NodeMathBlockContent
                    | NodeType::NodeYamlFrontMatterContent
                    | NodeType::NodeHTMLBlock
                    | NodeType::NodeInlineHTML
                    | NodeType::NodeEmojiAlias => ret.push_str(&n.tokens_str()),
                    NodeType::NodeCodeBlockFenceInfoMarker => {
                        ret.push_str(&String::from_utf8_lossy(&n.data().code_block_info))
                    }
                    NodeType::NodeLink if 3 == n.data().link_type => {
                        ret.push_str(&String::from_utf8_lossy(&n.data().link_ref_label));
                    }
                    _ => {}
                }
            }
            WalkStatus::WalkContinue
        });
        ret
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
        let mut caret_in_info = false;
        let mut code_block_info: Vec<u8> = Vec::new();
        if is_fenced {
            if let Some(prev) = n.previous() {
                code_block_info = prev.data().code_block_info.clone();
                if contains(&code_block_info, editor::CARET.as_bytes()) {
                    caret_in_info = true;
                    code_block_info = replace_all(&code_block_info, editor::CARET.as_bytes(), b"");
                }
            }
        }
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut language = String::new();
        if is_fenced && !code_block_info.is_empty() {
            let info_words: Vec<&[u8]> = code_block_info
                .split(|&b| b == b' ')
                .filter(|w| !w.is_empty())
                .collect();
            if let Some(first) = info_words.first() {
                language = String::from_utf8_lossy(first).to_string();
                attrs.push(("class".into(), format!("language-{}", language)));
                if "mindmap" == language {
                    let data_code =
                        super::mindmap::echarts_mindmap_str(&String::from_utf8_lossy(&tokens));
                    attrs.push(("data-code".into(), data_code));
                }
            }
        }

        let mut class = "vditor-ir__marker--pre".to_string();
        if self.base.options.vditor_code_block_preview {
            class.push_str(" vditor-ir__marker");
        }
        self.tag("pre", &[("class", &class)], false);
        self.tag_owned("code", &attrs, false);
        if code_is_empty {
            if !caret_in_info {
                self.base.write_str(editor::FRONT_END_CARET);
            }
            self.base.write_byte(b'\n');
        } else {
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
            self.base.newline();
        }
        self.base.write_str("</code></pre>");

        if self.base.options.vditor_code_block_preview {
            self.tag(
                "pre",
                &[("class", "vditor-ir__preview"), ("data-render", "2")],
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

    fn render_math_block_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = trim_ascii_space(&n.tokens_vec()).to_vec();
        let code_len = tokens.len();
        let code_is_empty = code_len < 1
            || (editor::CARET.len() == code_len && editor::CARET.as_bytes() == &tokens[..]);
        let mut class = "vditor-ir__marker--pre".to_string();
        if self.base.options.vditor_math_block_preview {
            class.push_str(" vditor-ir__marker");
        }
        self.tag("pre", &[("class", &class)], false);
        self.tag(
            "code",
            &[("data-type", "math-block"), ("class", "language-math")],
            false,
        );
        if code_is_empty {
            self.base.write_str(editor::FRONT_END_CARET);
            self.base.write_str("\n");
        } else {
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
        }
        self.base.write_str("</code></pre>");

        if self.base.options.vditor_math_block_preview {
            self.tag(
                "pre",
                &[("class", "vditor-ir__preview"), ("data-render", "2")],
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

    fn render_inline_math_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = n.tokens_vec();
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        self.tag("/code", &[], false);
        self.tag(
            "span",
            &[("class", "vditor-ir__preview"), ("data-render", "2")],
            false,
        );
        self.tag("span", &[("class", "language-math")], false);
        // 与 Go 版一致：预览使用已转义的 tokens（因此会显示 &gt; 等实体）。
        let mut tokens = escaped.into_bytes();
        tokens = replace_all(&tokens, editor::CARET.as_bytes(), b"");
        if n.parent_is(&[NodeType::NodeTableCell]) {
            tokens = replace_all(&tokens, b"\\|", b"|");
        }
        self.base.write(&tokens);
        self.tag("/span", &[], false);
        self.tag("/span", &[], false);
        WalkStatus::WalkContinue
    }

    fn render_yaml_front_matter_content(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let tokens = trim_ascii_space(&n.tokens_vec()).to_vec();
        let code_len = tokens.len();
        let code_is_empty = code_len < 1
            || (editor::CARET.len() == code_len && editor::CARET.as_bytes() == &tokens[..]);
        self.tag("pre", &[("class", "vditor-ir__marker--pre")], false);
        self.tag(
            "code",
            &[
                ("data-type", "yaml-front-matter"),
                ("class", "language-yaml"),
            ],
            false,
        );
        if code_is_empty {
            self.base.write_str(editor::FRONT_END_CARET);
            self.base.write_str("\n");
        } else {
            let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
            self.base.write(escaped.as_bytes());
        }
        self.base.write_str("</code></pre>");
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
            self.tag(
                "table",
                &[("data-block", "0"), ("data-type", "table")],
                false,
            );
        } else {
            if n.first_child().and_then(|c| c.next()).is_some() {
                self.tag("/tbody", &[], false);
            }
            self.tag("/table", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_image(&mut self, n: Node, entering: bool) -> WalkStatus {
        let need_reset_caret = n
            .next()
            .map(|x| {
                NodeType::NodeText == x.typ() && x.tokens().starts_with(editor::CARET.as_bytes())
            })
            .unwrap_or(false);

        if entering {
            if 3 == n.data().link_type {
                if let Some(x) = n.child_by_type(NodeType::NodeOpenParen) {
                    x.unlink();
                }
                if let Some(x) = n.child_by_type(NodeType::NodeLinkDest) {
                    x.unlink();
                }
                if let Some(space) = n.child_by_type(NodeType::NodeLinkSpace) {
                    space.unlink();
                    if let Some(x) = n.child_by_type(NodeType::NodeLinkTitle) {
                        x.unlink();
                    }
                }
                if let Some(x) = n.child_by_type(NodeType::NodeCloseParen) {
                    x.unlink();
                }
            }

            let text = self.text(n);
            let mut class = "vditor-ir__node".to_string();
            if text.contains(editor::CARET) || need_reset_caret {
                class.push_str(" vditor-ir__node--expand");
            }
            self.tag("span", &[("class", &class), ("data-type", "img")], false);
        } else {
            if need_reset_caret {
                self.base.write_str(editor::CARET);
                if let Some(next) = n.next() {
                    let tokens = replace_all(&next.tokens_vec(), editor::CARET.as_bytes(), b"");
                    next.set_tokens(tokens);
                }
            }

            let link = if 3 == n.data().link_type {
                self.find_link_ref_def_link(&n.data().link_ref_label)
                    .unwrap_or(n)
            } else {
                n
            };
            let dest_tokens = link
                .child_by_type(NodeType::NodeLinkDest)
                .map(|d| d.tokens_vec())
                .unwrap_or_default();
            let dest_tokens = self.base.link_path(&dest_tokens);
            let dest_tokens = replace_all(&dest_tokens, editor::CARET.as_bytes(), b"");
            let mut attrs: Vec<(String, String)> = vec![(
                "src".into(),
                String::from_utf8_lossy(&dest_tokens).to_string(),
            )];
            if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                if !alt.tokens().is_empty() {
                    let alt_tokens = replace_all(&alt.tokens_vec(), editor::CARET.as_bytes(), b"");
                    attrs.push((
                        "alt".into(),
                        String::from_utf8_lossy(&alt_tokens).to_string(),
                    ));
                }
            }
            self.tag_owned("img", &attrs, true);

            let idx = match find_last(&self.base.out, b"<img src=") {
                Some(i) => i,
                None => {
                    self.tag("/span", &[], false);
                    return WalkStatus::WalkContinue;
                }
            };
            let img_buf = self.base.out[idx..].to_vec();
            let img_buf = if self.base.options.sanitize {
                super::sanitizer::sanitize(&img_buf)
            } else {
                img_buf
            };
            self.base.out.truncate(idx);
            self.base.write(&img_buf);

            self.tag("/span", &[], false);
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
        self.render_div_node(n, true);
        let tokens = trim_ascii_space(&n.tokens_vec()).to_vec();
        self.base
            .write_str("<pre class=\"vditor-ir__marker--pre vditor-ir__marker\">");
        self.tag("code", &[("data-type", "html-block")], false);
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        self.base.write_str("</code></pre>");

        self.tag(
            "pre",
            &[("class", "vditor-ir__preview"), ("data-render", "2")],
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
        if is_vditor_table_cell_br(n) {
            ensure_vditor_table_cell_br_placeholder(n);
            self.tag("br", &[], true);
            return WalkStatus::WalkContinue;
        }

        let tokens = n.tokens_vec();
        let open_kbd = tokens == b"<kbd>";
        let close_kbd = tokens == b"</kbd>";
        if open_kbd || close_kbd {
            if open_kbd {
                if self.tag_match_close("kbd", n) {
                    self.render_span_node(n);
                    self.tag("code", &[("class", "vditor-ir__marker")], false);
                    let escaped = html_util::escape_html(&n.tokens_str());
                    self.base.write(escaped.as_bytes());
                    self.tag("/code", &[], false);
                    self.tag("/span", &[], false);
                    self.tag("kbd", &[], false);
                } else {
                    self.render_span_node(n);
                    self.tag("code", &[("class", "vditor-ir__marker")], false);
                    let escaped = html_util::escape_html(&n.tokens_str());
                    self.base.write(escaped.as_bytes());
                    self.tag("/code", &[], false);
                    self.tag("/span", &[], false);
                }
            } else if self.tag_match_open("kbd", n) {
                self.tag("/kbd", &[], false);
                self.render_span_node(n);
                self.tag("code", &[("class", "vditor-ir__marker")], false);
                let escaped = html_util::escape_html(&n.tokens_str());
                self.base.write(escaped.as_bytes());
                self.tag("/code", &[], false);
                self.tag("/span", &[], false);
            } else {
                self.render_span_node(n);
                self.tag("code", &[("class", "vditor-ir__marker")], false);
                let escaped = html_util::escape_html(&n.tokens_str());
                self.base.write(escaped.as_bytes());
                self.tag("/code", &[], false);
                self.tag("/span", &[], false);
            }
        } else {
            self.render_span_node(n);
            self.tag("code", &[("class", "vditor-ir__marker")], false);
            let escaped = html_util::escape_html(&n.tokens_str());
            self.base.write(escaped.as_bytes());
            self.tag("/code", &[], false);
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn tag_match_close(&self, tag: &str, n: Node) -> bool {
        let close = format!("</{}>", tag);
        let mut cur = n.next();
        while let Some(x) = cur {
            if NodeType::NodeInlineHTML == x.typ() && x.tokens_str() == close {
                return true;
            }
            cur = x.next();
        }
        false
    }

    fn tag_match_open(&self, tag: &str, n: Node) -> bool {
        let open = format!("<{}>", tag);
        let mut cur = n.previous();
        while let Some(x) = cur {
            if NodeType::NodeInlineHTML == x.typ() && x.tokens_str() == open {
                return true;
            }
            cur = x.previous();
        }
        false
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
            self.tag("p", &[("data-block", "0")], false);
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
        let caret_zwsp = format!("{}{}", editor::CARET, editor::ZWSP);
        if !super::eq_fold(&tokens, caret_zwsp.as_bytes()) {
            tokens = replace_all(&tokens, editor::ZWSP.as_bytes(), b"");
        }
        let escaped = html_util::escape_html(&String::from_utf8_lossy(&tokens));
        self.base.write(escaped.as_bytes());
        WalkStatus::WalkContinue
    }

    fn render_code_span_open_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            let len = n.parent().map(|p| p.data().code_marker_len).unwrap_or(1);
            self.base.write_str(&"\u{0060}".repeat(len.max(1) as usize));
            if let Some(next) = n.next() {
                if next.tokens().starts_with(&[0x60u8]) {
                    self.base.write_byte(b' ');
                }
            }
            self.tag("/span", &[], false);
            self.tag("code", &[("data-newline", "1")], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_code_span_close_marker(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag("/code", &[], false);
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            if let Some(prev) = n.previous() {
                if prev.tokens().ends_with(&[0x60u8]) {
                    self.base.write_byte(b' ');
                }
            }
            let len = n.parent().map(|p| p.data().code_marker_len).unwrap_or(1);
            self.base.write_str(&"\u{0060}".repeat(len.max(1) as usize));
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_link_text(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            let parent = n.parent();
            if parent
                .map(|p| NodeType::NodeImage == p.typ())
                .unwrap_or(false)
            {
                self.tag(
                    "span",
                    &[("class", "vditor-ir__marker vditor-ir__marker--bracket")],
                    false,
                );
            } else if parent.map(|p| 3 == p.data().link_type).unwrap_or(false) {
                self.tag("span", &[], false);
            } else {
                self.tag("span", &[("class", "vditor-ir__link")], false);
            }
            self.base.write(&n.tokens_vec());
            self.tag("/span", &[], false);
        }
        WalkStatus::WalkContinue
    }

    fn render_close_bracket(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.tag(
                "span",
                &[("class", "vditor-ir__marker vditor-ir__marker--bracket")],
                false,
            );
            self.base.write_byte(b']');
            self.tag("/span", &[], false);

            if let Some(parent) = n.parent() {
                if 3 == parent.data().link_type {
                    let link_text = parent.child_by_type(NodeType::NodeLinkText);
                    let same = link_text
                        .map(|t| super::eq_fold(&parent.data().link_ref_label, t.tokens()))
                        .unwrap_or(false);
                    if !same {
                        self.tag(
                            "span",
                            &[("class", "vditor-ir__marker vditor-ir__marker--link")],
                            false,
                        );
                        self.base.write_byte(b'[');
                        self.base.write(&parent.data().link_ref_label);
                        self.base.write_byte(b']');
                        self.tag("/span", &[], false);
                    }
                }
            }
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
            let mut class = "callout-info vditor-ir__node".to_string();
            let all = format!(
                "{}{}{}",
                n.data().callout_type,
                n.data().callout_icon,
                n.data().callout_title
            );
            if all.contains(editor::CARET) {
                class.push_str(" vditor-ir__node--expand");
            }
            self.tag("p", &[("data-block", "0"), ("class", &class)], false);
            self.tag("span", &[("class", "vditor-ir__marker")], false);
            let mut s = String::from("[!");
            s.push_str(&subtype);
            s.push_str("] ");
            self.base.write_str(&s);
            self.tag("/span", &[], false);
            if !n.data().callout_icon.is_empty() {
                if 1 == n.data().callout_icon_type {
                    self.tag("span", &[("data-render", "2")], false);
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
                    self.tag("/span", &[], false);
                    self.tag("span", &[("class", "vditor-ir__marker")], false);
                    let editable = html_util::escape_html(&callout_editable_icon(n));
                    self.base.write_str(&editable);
                    self.tag("/span", &[], false);
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
                let mut renderer = VditorIRRenderer::new(
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
            let text = self.text(n);
            let heading_id = n.child_by_type(NodeType::NodeHeadingID);
            let expand = text.contains(editor::CARET)
                || heading_id
                    .map(|h| contains(&h.tokens_vec(), editor::CARET.as_bytes()))
                    .unwrap_or(false);
            let mut s = format!("<h{} data-block=\"0\" class=\"vditor-ir__node", level);
            if expand {
                s.push_str(" vditor-ir__node--expand");
            }
            s.push('"');
            let mut id = heading_id.map(|h| h.tokens_str()).unwrap_or_default();
            if id.is_empty() {
                id = super::heading_id(n);
            }
            s.push_str(" id=\"ir-");
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
            if !n.data().heading_setext {
                self.tag(
                    "span",
                    &[
                        ("class", "vditor-ir__marker vditor-ir__marker--heading"),
                        ("data-type", "heading-marker"),
                    ],
                    false,
                );
                self.base.write_str(&"#".repeat(level));
                self.base.write_str(" ");
                self.tag("/span", &[], false);
            }
        } else {
            if n.data().heading_setext {
                self.tag(
                    "span",
                    &[
                        ("class", "vditor-ir__marker vditor-ir__marker--heading"),
                        ("data-type", "heading-marker"),
                        ("data-render", "2"),
                    ],
                    false,
                );
                self.base.newline();
                let content_len = self.setext_heading_len(n);
                if 1 == level {
                    self.base.write_str(&"=".repeat(content_len));
                } else {
                    self.base.write_str(&"-".repeat(content_len));
                }
                self.tag("/span", &[], false);
            }
            let s = format!("</h{}>", level);
            self.base.write_str(&s);
        }
        WalkStatus::WalkContinue
    }

    fn setext_heading_len(&self, n: Node) -> usize {
        let mut buf = String::new();
        crate::ast::walk_raw(n, &mut |n, entering| {
            if (NodeType::NodeText == n.typ()
                || NodeType::NodeLinkText == n.typ()
                || NodeType::NodeSoftBreak == n.typ())
                && entering
            {
                buf.push_str(&n.tokens_str());
            }
            WalkStatus::WalkContinue
        });
        let content = buf.replace(editor::CARET, "");
        let last_line = content.split('\n').next_back().unwrap_or("");
        let mut ret = 0;
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

    fn render_footnotes_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            if self.base.rendering_footnotes {
                return WalkStatus::WalkContinue;
            }
            self.base.write_str("<div data-type=\"footnotes-def\">");
            let mut s = String::from("[");
            s.push_str(&n.tokens_str());
            s.push_str("]: ");
            self.base.write_str(&s);
            let mut c = n.first_child();
            while let Some(child) = c {
                let me = &mut *self;
                crate::ast::walk_raw(child, &mut |nn, en| me.render_node(nn, en));
                c = child.next();
            }
            self.base.write_str("</div>");
            return WalkStatus::WalkSkipChildren;
        }
        WalkStatus::WalkContinue
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
        let text = n.text();
        let expand = text.contains(editor::CARET);
        let mut attrs: Vec<(String, String)> = vec![("data-type".into(), "footnotes-ref".into())];
        if expand {
            attrs.push((
                "class".into(),
                "vditor-ir__node vditor-ir__node--expand vditor-tooltipped vditor-tooltipped__s"
                    .into(),
            ));
        } else {
            attrs.push((
                "class".into(),
                "vditor-ir__node vditor-tooltipped vditor-tooltipped__s".into(),
            ));
        }
        attrs.push((
            "aria-label".into(),
            sub_str(&html_util::escape_string(&label), 24),
        ));
        attrs.push((
            "data-footnotes-label".into(),
            String::from_utf8_lossy(&n.data().footnotes_ref_label).to_string(),
        ));
        self.tag_owned("sup", &attrs, false);
        self.tag(
            "span",
            &[("class", "vditor-ir__marker vditor-ir__marker--bracket")],
            false,
        );
        self.base.write_byte(b'[');
        self.tag("/span", &[], false);
        self.tag(
            "span",
            &[("class", "vditor-ir__marker vditor-ir__marker--link")],
            false,
        );
        self.base.write(&n.tokens_vec());
        self.tag("/span", &[], false);
        self.tag(
            "span",
            &[("class", "vditor-ir__marker--hide"), ("data-render", "1")],
            false,
        );
        self.base.write_str(&idx.to_string());
        self.tag("/span", &[], false);
        self.tag(
            "span",
            &[("class", "vditor-ir__marker vditor-ir__marker--bracket")],
            false,
        );
        self.base.write_byte(b']');
        self.tag("/span", &[], false);
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
            if self.base.options.vditor_ir {
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
}

impl Renderer for VditorIRRenderer {
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

fn is_vditor_table_cell_br(node: Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if NodeType::NodeTableCell != parent.typ() || NodeType::NodeInlineHTML != node.typ() {
        return false;
    }
    let raw = node.tokens_vec();
    let tokens = trim_ascii_space(&raw);
    super::eq_fold(tokens, b"<br>")
        || super::eq_fold(tokens, b"<br/>")
        || super::eq_fold(tokens, b"<br />")
}

fn ensure_vditor_table_cell_br_placeholder(node: Node) {
    let Some(parent) = node.parent() else {
        return;
    };
    let mut last = parent.last_child();
    if let Some(l) = last {
        if NodeType::NodeText == l.typ()
            && trim_ascii_space(&replace_all(&l.tokens_vec(), editor::CARET.as_bytes(), b""))
                .is_empty()
        {
            last = l.previous();
        }
    }
    if Some(node) != last {
        return;
    }

    let mut allocation = Tree::new();
    let placeholder = allocation.new_node_raw(NodeType::NodeHardBreak);
    if let Some(next) = node.next().filter(|next| {
        NodeType::NodeText == next.typ()
            && trim_ascii_space(&replace_all(
                &next.tokens_vec(),
                editor::CARET.as_bytes(),
                b"",
            ))
            .is_empty()
    }) {
        next.insert_after(placeholder);
    } else {
        node.insert_after(placeholder);
    }
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

fn callout_editable_icon(node: Node) -> String {
    let d = node.data();
    if 1 == d.callout_icon_type {
        if crate::parse::callout::is_valid_callout_image_src(&d.callout_icon) {
            let dest = crate::html::encode_destination(d.callout_icon.as_bytes());
            return format!("![callout-icon](<{}>)", String::from_utf8_lossy(&dest));
        }
        return String::new();
    }
    d.callout_icon.clone()
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
    let mut prev = n.previous();
    if let Some(p) = prev {
        if NodeType::NodeKramdownSpanIAL == p.typ() {
            prev = p.previous();
        }
    }
    prev.map(|p| p.text())
}

fn next_node_text(n: Node) -> Option<String> {
    n.next().map(|p| p.text())
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
