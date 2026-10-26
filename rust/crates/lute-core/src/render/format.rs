// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 格式化渲染器（Markdown 输出），对应 Go 版 render.FormatRenderer。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::editor;
use crate::options::{ParseOptions, RenderOptions};
use crate::parse::ial::ial_to_tokens;

use super::table::{need_use_html_table, HtmlTableRenderer};
use super::{is_letter_or_digit, BaseRenderer};

/// 格式化渲染器。
pub struct FormatRenderer {
    pub base: BaseRenderer,
    pub(crate) root: Node,
    _owner: NodeOwner,
    pub parse_options: ParseOptions,
    /// 节点输出缓冲栈（保存外层 writer）。
    stack: Vec<Vec<u8>>,
    /// TextBundle 模式下的链接前缀。
    pub link_prefixes: Option<Vec<String>>,
    /// TextBundle 模式下被替换掉的原始链接。
    pub original_links: Vec<String>,
    /// 对应 Go 版 render.ProtyleExportMdRenderer：文本标记输出 Markdown 标记符而非 span。
    protyle_export: bool,
    tabs_fence_lengths: Option<std::collections::HashMap<Node, usize>>,
}

impl FormatRenderer {
    pub fn new(tree: &Tree, options: RenderOptions, parse_options: ParseOptions) -> Self {
        Self {
            base: BaseRenderer::new(options),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            parse_options,
            stack: Vec::new(),
            link_prefixes: None,
            original_links: Vec::new(),
            protyle_export: false,
            tabs_fence_lengths: None,
        }
    }

    /// 对应 Go 版 render.NewProtyleExportMdRenderer。
    ///
    /// 与 FormatRenderer 共用绝大部分实现，差异集中在文本标记、表格单元格 IAL、
    /// 段落与文本的零宽空格处理等处，用 protyle_export 开关区分。
    pub fn new_protyle_export(
        tree: &Tree,
        options: RenderOptions,
        parse_options: ParseOptions,
    ) -> Self {
        let mut ret = Self::new(tree, options, parse_options);
        ret.protyle_export = true;
        ret.base.table_cell_rich_inline = true;
        ret
    }

    /// TextBundle 渲染器，对应 Go 版 render.NewTextBundleRenderer。
    pub fn new_text_bundle(
        tree: &Tree,
        link_prefixes: Vec<String>,
        options: RenderOptions,
        parse_options: ParseOptions,
    ) -> Self {
        let mut ret = Self::new(tree, options, parse_options);
        ret.link_prefixes = Some(link_prefixes);
        ret
    }

    pub fn render(&mut self) -> Vec<u8> {
        self.base.last_out = b'\n';
        self.base.out = Vec::with_capacity(4096);
        let root = self.root;
        crate::ast::walk_raw(root, &mut |n, entering| self.render_node(n, entering));
        std::mem::take(&mut self.base.out)
    }

    fn push_writer(&mut self) {
        let cur = std::mem::take(&mut self.base.out);
        self.stack.push(cur);
    }

    fn pop_writer(&mut self) -> Vec<u8> {
        let child = std::mem::take(&mut self.base.out);
        self.base.out = self.stack.pop().unwrap_or_default();
        child
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        if let Some((s, st)) = self.base.ext_render(n, entering) {
            self.base.write_str(&s);
            return st;
        }
        let typ = n.typ();
        match typ {
            NodeType::NodeDocument => {
                if entering {
                    self.push_writer();
                } else {
                    self.stack.pop();
                    let mut buf = std::mem::take(&mut self.base.out);
                    if self.base.options.keep_paragraph_beginning_space {
                        let start = buf.iter().position(|&b| b != b'\n').unwrap_or(buf.len());
                        let mut end = buf.len();
                        while end > start && matches!(buf[end - 1], b' ' | b'\t' | b'\n') {
                            end -= 1;
                        }
                        buf = buf[start..end].to_vec();
                    } else {
                        let start = buf
                            .iter()
                            .position(|&b| b != b' ' && b != b'\t' && b != b'\n')
                            .unwrap_or(buf.len());
                        let mut end = buf.len();
                        while end > start && matches!(buf[end - 1], b' ' | b'\t' | b'\n') {
                            end -= 1;
                        }
                        buf = buf[start..end].to_vec();
                    }
                    self.base.out.clear();
                    self.base.write(&buf);
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeParagraph => self.render_paragraph(n, entering),
            NodeType::NodeText => {
                if entering {
                    if self.protyle_export {
                        // 去掉导出 Markdown 时残留的零宽空格
                        let mut tokens = n.tokens_vec();
                        let next_text = n.next().map(|x| x.text()).unwrap_or_default();
                        while tokens.starts_with(editor::ZWSP.as_bytes())
                            && next_text.is_empty()
                            && tokens.len() > editor::ZWSP.len()
                        {
                            tokens = tokens[editor::ZWSP.len()..].to_vec();
                        }
                        if n.previous().is_none() && tokens.starts_with(editor::ZWSP.as_bytes()) {
                            tokens = tokens[editor::ZWSP.len()..].to_vec();
                        }
                        if tokens.is_empty() {
                            return WalkStatus::WalkContinue;
                        }
                        n.set_tokens(tokens);
                    }
                    let mut tokens = n.tokens_vec();
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        tokens = self.base.space(&tokens);
                    }
                    if self.base.options.fix_term_typo {
                        self.base.fix_term_typo(&mut tokens);
                    }
                    tokens = escape_setext_heading_markers_in_list_paragraph(n, &tokens);
                    if self.parse_options.tabs {
                        tokens = escape_tabs_paragraph_markers(n, &tokens, !self.protyle_export);
                    }
                    if n.previous().is_none()
                        || n.previous().map(|p| p.typ()) == Some(NodeType::NodeTaskListItemMarker)
                    {
                        if let Some(gp) = n.parent().and_then(|p| p.parent()) {
                            if let Some(ld) = gp.data().list_data.as_ref() {
                                if 3 == ld.typ && b' ' == self.base.last_out {
                                    tokens = trim_prefix(&tokens, b" ");
                                    let caret_space = format!("{} ", editor::CARET).into_bytes();
                                    if tokens.starts_with(&caret_space) {
                                        tokens = trim_prefix(&tokens, &caret_space);
                                        let mut nt = editor::CARET.as_bytes().to_vec();
                                        nt.extend_from_slice(&tokens);
                                        tokens = nt;
                                    }
                                }
                            }
                        }
                    }
                    self.base.write(&tokens);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpan => {
                self.auto_space_letter_digit(n, entering);
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanOpenMarker => {
                if entering {
                    self.base.write_byte(b'`');
                    if 1 < n.parent().map(|p| p.data().code_marker_len).unwrap_or(0) {
                        self.base.write_byte(b'`');
                        let text = n.next().map(|x| x.tokens_str()).unwrap_or_default();
                        if text.starts_with('`') {
                            self.base.write_byte(b' ');
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanContent => {
                if entering {
                    let mut tokens = n.tokens_vec();
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        tokens = replace_bytes(&tokens, b"\\|", b"|");
                        tokens = replace_bytes(&tokens, b"|", b"\\|");
                        tokens = replace_bytes(&tokens, b"<br/>", b"");
                    }
                    self.base.write(&tokens);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeSpanCloseMarker => {
                if entering {
                    if 1 < n.parent().map(|p| p.data().code_marker_len).unwrap_or(0) {
                        let text = n.previous().map(|x| x.tokens_str()).unwrap_or_default();
                        if text.ends_with('`') {
                            self.base.write_byte(b' ');
                        }
                        self.base.write_byte(b'`');
                    }
                    self.base.write_byte(b'`');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlock => {
                if entering {
                    self.base.newline();
                    if !n.data().is_fenced_code_block {
                        self.base.write(b"```");
                        self.base.write_byte(b'\n');
                        if let Some(first) = n.first_child() {
                            self.base.write(first.tokens());
                        }
                        self.base.write(b"```");
                        self.base.newline();
                        if !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                            self.base.write_byte(b'\n');
                        }
                        return WalkStatus::WalkSkipChildren;
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockFenceOpenMarker => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockFenceInfoMarker => {
                if entering {
                    self.base.write(&n.data().code_block_info);
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockCode => {
                if entering {
                    if self.protyle_export {
                        let mut tokens = n.tokens().to_vec();
                        if let Some(parent) = n.parent() {
                            if let Some(info) =
                                parent.child_by_type(NodeType::NodeCodeBlockFenceInfoMarker)
                            {
                                let lang = String::from_utf8_lossy(&info.data().code_block_info)
                                    .to_string();
                                // 不需要高亮的语言（mermaid 等）导出时还原 HTML 实体。
                                if "mindmap" != lang && no_highlight_language(&lang) {
                                    tokens = crate::html::unescape_bytes(&tokens);
                                }
                            }
                        }
                        self.base.write(&tokens);
                    } else {
                        self.base.write(n.tokens());
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlockFenceCloseMarker => {
                if entering {
                    self.base.newline();
                    self.base.write(n.tokens());
                    self.base.newline();
                    if !self.is_last_node(n) {
                        if let Some(parent) = n.parent() {
                            if self.without_kramdown_block_ial(parent) {
                                self.base.write_byte(b'\n');
                            }
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlock => {
                self.base.newline();
                if !entering && !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlockOpenMarker | NodeType::NodeMathBlockCloseMarker => {
                if entering {
                    self.base.write(b"$$");
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMathBlockContent => {
                if entering {
                    self.base.write(n.tokens());
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMath => {
                self.auto_space_letter_digit(n, entering);
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathOpenMarker => {
                if entering {
                    self.base.write_byte(b'$');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathContent => {
                if entering {
                    if self.protyle_export {
                        let mut content =
                            crate::html::unescape_string(&String::from_utf8_lossy(n.tokens()));
                        if n.parent_is(&[NodeType::NodeTableCell]) {
                            content = crate::lex::repeat_backslash_before_pipe(&content);
                            content = content.replace('\n', "<br/>");
                        }
                        content = content.replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                        self.base.write_str(&content);
                    } else {
                        self.base.write(n.tokens());
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineMathCloseMarker => {
                if entering {
                    self.base.write_byte(b'$');
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
            NodeType::NodeEmA6kOpenMarker | NodeType::NodeEmA6kCloseMarker => {
                if entering {
                    self.base.write_byte(b'*');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmU8eOpenMarker | NodeType::NodeEmU8eCloseMarker => {
                if entering {
                    self.base.write_byte(b'_');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongA6kOpenMarker | NodeType::NodeStrongA6kCloseMarker => {
                if entering {
                    self.base.write(b"**");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrongU8eOpenMarker | NodeType::NodeStrongU8eCloseMarker => {
                if entering {
                    self.base.write(b"__");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote => self.render_blockquote(n, entering),
            NodeType::NodeBlockquoteMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeading => self.render_heading(n, entering),
            NodeType::NodeHeadingC8hMarker => WalkStatus::WalkContinue,
            NodeType::NodeHeadingID => {
                if entering {
                    self.base.write_str(" {");
                    self.base.write(n.tokens());
                    self.base.write_str("}");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeList | NodeType::NodeMindmap => self.render_list(n, entering),
            NodeType::NodeListItem | NodeType::NodeMindmapItem => {
                self.render_list_item(n, entering)
            }
            NodeType::NodeThematicBreak => {
                if entering {
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        self.base.write_str("<hr/>");
                    } else {
                        self.base.write_str("---");
                        if self.without_kramdown_block_ial(n) {
                            self.base.write_byte(b'\n');
                            self.base.write_byte(b'\n');
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHardBreak => {
                if entering {
                    if !self.base.options.soft_break2_hard_break {
                        self.base.write(b"\\\n");
                    } else if n.parent_is(&[NodeType::NodeTableCell]) {
                        self.base.write_str("<br/>");
                    } else {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSoftBreak => {
                if entering {
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLBlock => {
                if entering {
                    if self.protyle_export {
                        self.base.newline();
                    } else {
                        self.newline_before_block(n);
                    }
                    let tokens = self.tag_src_path(n.tokens());
                    self.base.write(&tokens);
                    self.base.newline();
                    if !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeInlineHTML => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLink => {
                if entering {
                    self.base.link_text_auto_space_previous(n);
                    if 3 == n.data().link_type {
                        let text = n
                            .child_by_type(NodeType::NodeLinkText)
                            .map(|t| t.tokens_vec())
                            .unwrap_or_default();
                        let label = n.data().link_ref_label.clone();
                        if text == label {
                            self.base.write_str("[");
                            self.base.write(&text);
                            self.base.write_str("]");
                        } else {
                            self.base.write_str("[");
                            self.base.write(&text);
                            self.base.write_str("][");
                            self.base.write(&label);
                            self.base.write_str("]");
                        }
                        return WalkStatus::WalkSkipChildren;
                    }
                    if 1 == n.data().link_type {
                        if let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) {
                            let tokens = dest.tokens_vec();
                            self.base.write(&tokens);
                        }
                        return WalkStatus::WalkSkipChildren;
                    }
                } else {
                    self.base.link_text_auto_space_next(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeImage => {
                if self.protyle_export && self.base.options.img_tag && entering {
                    let mut attrs = Vec::new();
                    if let Some(alt) = n.child_by_type(NodeType::NodeLinkText) {
                        attrs.push((
                            "alt".into(),
                            alt.tokens_str().replace('[', "\\[").replace(']', "\\]"),
                        ));
                    }
                    if let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) {
                        let path = self.base.link_path(dest.tokens());
                        let mut path = String::from_utf8_lossy(&path).into_owned();
                        if !self.base.options.prevent_encode_link_space {
                            path = path.replace(' ', "%20");
                        }
                        attrs.push(("src".into(), path));
                    }
                    if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                        attrs.push(("title".into(), title.tokens_str().replace('"', "&quot;")));
                    }
                    let style = n.ial_attr("style");
                    if !style.is_empty() {
                        attrs.push(("style".into(), style));
                    }
                    self.base.tag("img", &attrs, true);
                    return WalkStatus::WalkSkipChildren;
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBang => {
                if entering {
                    self.base.write_byte(b'!');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeOpenBracket => {
                if entering {
                    self.base.write_byte(b'[');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseBracket => {
                if entering {
                    self.base.write_byte(b']');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeOpenParen => {
                if entering {
                    self.base.write_byte(b'(');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseParen => {
                if entering {
                    self.base.write_byte(b')');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLess => {
                if entering {
                    self.base.write_byte(b'<');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGreater => {
                if entering {
                    self.base.write_byte(b'>');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeOpenBrace => {
                if entering {
                    self.base.write_byte(b'{');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCloseBrace => {
                if entering {
                    self.base.write_byte(b'}');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkText => {
                if entering {
                    let mut tokens = n.tokens_vec();
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        tokens = self.base.space(&tokens);
                    }
                    if self.protyle_export {
                        // 导出 Markdown 时链接文本中的方括号需要转义。
                        let text = String::from_utf8_lossy(&tokens)
                            .replace('[', "\\[")
                            .replace(']', "\\]");
                        self.base.write_str(&text);
                    } else {
                        self.base.write(&tokens);
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkSpace => {
                if entering {
                    self.base.write_byte(b' ');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkDest => {
                if entering {
                    if let Some(prefixes) = self.link_prefixes.clone() {
                        let mut dest = n.tokens_vec();
                        for prefix in prefixes {
                            if !prefix.is_empty() && dest.starts_with(prefix.as_bytes()) {
                                self.original_links
                                    .push(String::from_utf8_lossy(&dest).to_string());
                                let mut nd = b"assets".to_vec();
                                nd.extend_from_slice(&dest[prefix.len()..]);
                                dest = nd;
                            }
                        }
                        self.base.write(&dest);
                    } else {
                        let tokens = self.base.link_path(n.tokens());
                        if self.protyle_export {
                            let href = if self.base.options.prevent_encode_link_space {
                                String::from_utf8_lossy(&tokens).to_string()
                            } else {
                                String::from_utf8_lossy(&tokens).replace(' ', "%20")
                            };
                            self.base.write_str(&href);
                        } else {
                            self.base.write(&tokens);
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkTitle => {
                if entering {
                    self.base.write_byte(b'"');
                    if self.protyle_export {
                        // 只转义双引号，其余保持原样
                        let tokens = replace_bytes(n.tokens(), b"\"", b"&quot;");
                        self.base.write(&tokens);
                    } else {
                        let tokens = super::escape_html_bytes(n.tokens());
                        self.base.write(&tokens);
                    }
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough1OpenMarker | NodeType::NodeStrikethrough1CloseMarker => {
                if entering {
                    self.base.write_byte(b'~');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeStrikethrough2OpenMarker | NodeType::NodeStrikethrough2CloseMarker => {
                if entering {
                    self.base.write(b"~~");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTaskListItemMarker => {
                if entering {
                    let marker = self.normalized_task_list_item_marker(n);
                    self.base.write_byte(b'[');
                    self.base.write_byte(marker);
                    self.base.write_byte(b']');
                } else {
                    self.base.write_byte(b' ');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTable => self.render_table(n, entering),
            NodeType::NodeTableHead => self.render_table_head(n, entering),
            NodeType::NodeTableRow => {
                if !entering {
                    self.base.write_str("|\n");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTableCell => self.render_table_cell(n, entering),
            NodeType::NodeEmoji | NodeType::NodeEmojiUnicode | NodeType::NodeEmojiImg => {
                WalkStatus::WalkContinue
            }
            NodeType::NodeEmojiAlias => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFootnotesDefBlock => WalkStatus::WalkContinue,
            NodeType::NodeFootnotesDef => self.render_footnotes_def(n, entering),
            NodeType::NodeFootnotesRef => {
                if entering {
                    self.base.write_byte(b'[');
                    self.base.write(n.tokens());
                    self.base.write_byte(b']');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeToC => {
                if entering {
                    self.base.write_str("[toc]\n\n");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBackslash => {
                if entering {
                    self.base.write_byte(b'\\');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBackslashContent => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.base.write(&n.data().html_entity_tokens);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatter => {
                if entering {
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterOpenMarker | NodeType::NodeYamlFrontMatterCloseMarker => {
                if entering {
                    self.base.write(b"---");
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeYamlFrontMatterContent => {
                if entering {
                    self.base.write(n.tokens());
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRef => WalkStatus::WalkContinue,
            NodeType::NodeBlockRefID => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefSpace => {
                if entering {
                    self.base.write_byte(b' ');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefText => {
                if entering {
                    self.base.write_byte(b'"');
                    let tokens =
                        replace_bytes(&super::escape_html_bytes(n.tokens()), b"'", b"&apos;");
                    self.base.write(&tokens);
                    self.base.write_byte(b'"');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockRefDynamicText => {
                if entering {
                    self.base.write_byte(b'\'');
                    let tokens =
                        replace_bytes(&super::escape_html_bytes(n.tokens()), b"'", b"&apos;");
                    self.base.write(&tokens);
                    self.base.write_byte(b'\'');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFileAnnotationRef => WalkStatus::WalkContinue,
            NodeType::NodeFileAnnotationRefID => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFileAnnotationRefSpace => {
                if entering {
                    self.base.write_byte(b' ');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeFileAnnotationRefText => {
                if entering {
                    self.base.write_byte(b'"');
                    let tokens =
                        replace_bytes(&super::escape_html_bytes(n.tokens()), b"'", b"&apos;");
                    self.base.write(&tokens);
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
            NodeType::NodeMark1OpenMarker | NodeType::NodeMark1CloseMarker => {
                if entering {
                    self.base.write_byte(b'=');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeMark2OpenMarker | NodeType::NodeMark2CloseMarker => {
                if entering {
                    self.base.write(b"==");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSup => WalkStatus::WalkContinue,
            NodeType::NodeSupOpenMarker | NodeType::NodeSupCloseMarker => {
                if entering {
                    if self.protyle_export {
                        // 导出 Markdown 时上标用 HTML 标签，避免与脚注引用冲突。
                        if NodeType::NodeSupOpenMarker == typ {
                            self.base.write_str("<sup>");
                        } else {
                            self.base.write_str("</sup>");
                        }
                    } else {
                        self.base.write_byte(b'^');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSub => WalkStatus::WalkContinue,
            NodeType::NodeSubOpenMarker | NodeType::NodeSubCloseMarker => {
                if entering {
                    if self.protyle_export {
                        if NodeType::NodeSubOpenMarker == typ {
                            self.base.write_str("<sub>");
                        } else {
                            self.base.write_str("</sub>");
                        }
                    } else {
                        self.base.write_byte(b'~');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownBlockIAL => self.render_kramdown_block_ial(n, entering),
            NodeType::NodeKramdownSpanIAL => self.render_kramdown_span_ial(n, entering),
            NodeType::NodeBlockQueryEmbed => {
                self.base.newline();
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockQueryEmbedScript => {
                if entering {
                    self.base.write(n.tokens());
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
            NodeType::NodeTagOpenMarker | NodeType::NodeTagCloseMarker => {
                if entering {
                    self.base.write_byte(b'#');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeLinkRefDefBlock => WalkStatus::WalkContinue,
            NodeType::NodeLinkRefDef => {
                if entering {
                    self.base.write_byte(b'[');
                    self.base.write(n.tokens());
                    self.base.write_str("]: ");
                } else {
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSuperBlock => {
                if entering {
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSuperBlockOpenMarker => {
                if entering && self.base.options.super_block {
                    self.base.write(b"{{{");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSuperBlockLayoutMarker => {
                if entering && self.base.options.super_block {
                    self.base.write(n.tokens());
                    self.base.write_byte(b'\n');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeSuperBlockCloseMarker => {
                if entering {
                    if self.base.options.super_block {
                        self.base.newline();
                        self.base.write(b"}}}");
                        self.base.newline();
                    }
                    if !self.is_last_node(n) {
                        if let Some(parent) = n.parent() {
                            if self.without_kramdown_block_ial(parent) {
                                self.base.write_byte(b'\n');
                            }
                        }
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeGitConflict => {
                if entering {
                    self.base.newline();
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
                if !entering {
                    self.base.write(n.tokens());
                    self.base.newline();
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeIFrame
            | NodeType::NodeWidget
            | NodeType::NodeVideo
            | NodeType::NodeAudio => {
                if entering {
                    self.base.newline();
                    let tokens = self.tag_src_path(n.tokens());
                    self.base.write(&tokens);
                    self.base.newline();
                    if !self.is_last_node(n) {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKbd => WalkStatus::WalkContinue,
            NodeType::NodeKbdOpenMarker => {
                if entering {
                    self.base.write_str("<kbd>");
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKbdCloseMarker => {
                if entering {
                    self.base.write_str("</kbd>");
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
            NodeType::NodeBr => {
                if entering {
                    if self.protyle_export || n.parent_is(&[NodeType::NodeTableCell]) {
                        self.base.write_str("<br />");
                    } else {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTextMark => {
                if self.protyle_export {
                    self.render_text_mark_export(n, entering)
                } else {
                    self.render_text_mark(n, entering)
                }
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
                    if !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCustomBlock => {
                if entering {
                    self.base.newline();
                    self.base.write_str(";;;");
                    self.base.write_str(&n.data().custom_block_info);
                    self.base.newline();
                    self.base.write(n.tokens());
                    self.base.newline();
                    self.base.write_str(";;;");
                    if !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                        self.base.write_byte(b'\n');
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHTMLTag => WalkStatus::WalkContinue,
            NodeType::NodeHTMLTagOpen | NodeType::NodeHTMLTagClose => {
                if entering {
                    self.base.write(n.tokens());
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCallout => {
                if entering {
                    self.render_blockquote(n, true);
                    self.base.write_str("[!");
                    self.base.write_str(&n.data().callout_type);
                    self.base.write_byte(b']');
                    let title = super::callout_title(n);
                    if !title.is_empty() {
                        self.base.write_byte(b' ');
                        self.base.write_str(&title);
                    }
                    self.base.newline();
                } else {
                    self.render_blockquote(n, false);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTabs | NodeType::NodeTabItem => self.render_tabs(n, entering),
            _ => WalkStatus::WalkContinue,
        }
    }

    fn auto_space_letter_digit(&mut self, n: Node, entering: bool) {
        if !(crate::CAP_AUTO_SPACE && self.base.options.auto_space) {
            return;
        }
        let text = if entering {
            let mut prev = n.previous();
            if prev.map(|p| p.typ()) == Some(NodeType::NodeKramdownSpanIAL) {
                prev = prev.and_then(|p| p.previous());
            }
            prev.map(|p| p.text()).unwrap_or_default()
        } else {
            n.next().map(|p| p.text()).unwrap_or_default()
        };
        if text.is_empty() {
            return;
        }
        let c = if entering {
            text.chars().next_back().unwrap_or('\u{FFFD}')
        } else {
            text.chars().next().unwrap_or('\u{FFFD}')
        };
        if is_letter_or_digit(c) {
            self.base.write_byte(b' ');
        }
    }

    fn render_paragraph(&mut self, n: Node, entering: bool) -> WalkStatus {
        if self.protyle_export
            && self.base.options.tabs_markdown
            && n != self.root
            && n.parent().and_then(tab_title_block) == Some(n)
        {
            return WalkStatus::WalkSkipChildren;
        }
        if !entering {
            if !self.base.options.keep_paragraph_beginning_space {
                if let Some(first) = n.first_child() {
                    let tokens = first.tokens_vec();
                    first.set_tokens(trim_space(&tokens));
                }
            }
            if n.parent_is(&[NodeType::NodeTableCell]) {
                if let Some(next) = n.next() {
                    if NodeType::NodeText != next.typ() {
                        self.base.write_str("<br /><br />");
                    }
                }
                return WalkStatus::WalkContinue;
            }
            if self.without_kramdown_block_ial(n) {
                self.base.newline();
            }

            let mut in_tight_list = false;
            let mut last_list_item_last_para = false;
            if let Some(parent) = n.parent() {
                if NodeType::NodeListItem == parent.typ()
                    || NodeType::NodeMindmapItem == parent.typ()
                {
                    let list_item = parent;
                    if let Some(list_parent) = list_item.parent() {
                        if let Some(ld) = list_parent.data().list_data.as_ref() {
                            in_tight_list = ld.tight;
                            if list_item.next().is_none() {
                                last_list_item_last_para = n.next().is_none();
                            }
                        } else {
                            in_tight_list = true;
                        }
                    } else {
                        in_tight_list = true;
                    }
                }
            }
            if (!in_tight_list || last_list_item_last_para) && self.without_kramdown_block_ial(n) {
                self.base.write_byte(b'\n');
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_blockquote(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline_before_block(n);
            self.push_writer();
        } else {
            let mut child = self.pop_writer();
            let mut lines: Vec<&[u8]> = child.split(|&b| b == b'\n').collect();
            let length = lines.len();
            if 2 < length && is_blank(lines[length - 1]) && is_blank(lines[length - 2]) {
                lines.pop();
            }
            if self.stack.len() <= 1 {
                let length = lines.len();
                if 1 < length && is_blank(lines[length - 1]) {
                    lines.pop();
                }
            }
            let mut blockquote_lines = Vec::new();
            for line in lines {
                if line.is_empty() {
                    blockquote_lines.extend_from_slice(b">\n");
                    continue;
                }
                if b'>' == line[0] {
                    blockquote_lines.push(b'>');
                } else {
                    blockquote_lines.extend_from_slice(b"> ");
                }
                blockquote_lines.extend_from_slice(line);
                blockquote_lines.push(b'\n');
            }
            child = trim_space(&blockquote_lines);
            self.base.write(&child);
            let trimmed = trim_space(&self.base.out);
            self.base.out.clear();
            self.base.write(&trimmed);
            if !n.parent_is(&[NodeType::NodeTableCell]) && self.without_kramdown_block_ial(n) {
                self.base.write_str("\n\n");
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_heading(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline_before_block(n);
            if !n.data().heading_setext {
                let level = n.data().heading_level.max(0) as usize;
                for _ in 0..level {
                    self.base.write_byte(b'#');
                }
                self.base.write_byte(b' ');
            }
        } else {
            if n.data().heading_setext {
                self.base.write_byte(b'\n');
                let content_len = self.setext_heading_len(n);
                if 1 == n.data().heading_level {
                    self.base.write(&vec![b'='; content_len]);
                } else if 2 == n.data().heading_level {
                    self.base.write(&vec![b'-'; content_len]);
                }
            }
            if !n.parent_is(&[NodeType::NodeTableCell]) && self.without_kramdown_block_ial(n) {
                self.base.newline();
                self.base.write_byte(b'\n');
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_list(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.newline_before_block(n);
            self.push_writer();
        } else {
            let child = self.pop_writer();
            self.base.write(&child);
            let trimmed = trim_space(&self.base.out);
            self.base.out.clear();
            self.base.write(&trimmed);
            if !n.parent_is(&[NodeType::NodeTableCell]) && self.without_kramdown_block_ial(n) {
                self.base.write_str("\n\n");
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_list_item(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
            if crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial {
                if let Some(next) = n.next() {
                    if NodeType::NodeKramdownBlockIAL == next.typ() {
                        self.base.write(next.tokens());
                    }
                }
            }
            if let Some(first) = n.first_child() {
                if NodeType::NodeList == first.typ() || NodeType::NodeMindmap == first.typ() {
                    self.base.newline();
                }
            }
        } else {
            let mut child = self.pop_writer();
            let ld = n.data().list_data.clone().unwrap_or_default();
            let mut indent = ld.marker.len() + 1;
            if 1 == ld.typ && 3 > indent {
                indent += 1;
            }
            if 3 == ld.typ {
                let first_next_is_callout = n.first_child().and_then(|f| f.next()).map(|x| x.typ())
                    == Some(NodeType::NodeCallout);
                if 0 == ld.bullet_char
                    || ((b'*' == ld.bullet_char || b'-' == ld.bullet_char) && first_next_is_callout)
                {
                    indent += 4;
                }
            }

            if self.protyle_export && child.starts_with(b"* ") {
                let mut empty = b" \n\n".to_vec();
                empty.extend_from_slice(&child);
                child = empty;
            }
            let indent_spaces = vec![b' '; indent];
            let mut indented = Vec::new();
            for line in child.split(|&b| b == b'\n') {
                if line.is_empty() {
                    indented.push(b'\n');
                    continue;
                }
                indented.extend_from_slice(&indent_spaces);
                indented.extend_from_slice(line);
                indented.push(b'\n');
            }
            child = if indent < indented.len() {
                indented[indent..].to_vec()
            } else {
                Vec::new()
            };

            let mut list_item_buf = Vec::new();
            if 1 == ld.typ || (3 == ld.typ && 0 == ld.bullet_char) {
                if 0 == ld.num && 0 == ld.delimiter {
                    list_item_buf.extend_from_slice(&ld.marker);
                } else {
                    list_item_buf.extend_from_slice(ld.num.to_string().as_bytes());
                    list_item_buf.push(ld.delimiter);
                }
            } else if !self.base.options.unordered_list_marker.is_empty() {
                list_item_buf.extend_from_slice(self.base.options.unordered_list_marker.as_bytes());
            } else {
                list_item_buf.extend_from_slice(&ld.marker);
            }
            list_item_buf.push(b' ');
            list_item_buf.extend_from_slice(&child);
            child = list_item_buf;
            if n.parent_is(&[NodeType::NodeTableCell]) {
                child = replace_bytes(&child, b"\n", b"");
            }
            self.base.write(&child);
            let buf = trim_space(&self.base.out);
            self.base.out.clear();
            self.base.write(&buf);
            if !n.parent_is(&[NodeType::NodeTableCell]) {
                self.base.write_str("\n");
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_table(&mut self, n: Node, entering: bool) -> WalkStatus {
        if self.protyle_export && need_use_html_table(n) {
            if entering {
                self.render_table_by_html(n);
                return WalkStatus::WalkSkipChildren;
            }
            return WalkStatus::WalkContinue;
        }
        if entering {
            let mut cells: Vec<Vec<Node>> = vec![Vec::new()];
            let head_row = match n.child_by_type(NodeType::NodeTableHead) {
                Some(h) => h,
                None => return WalkStatus::WalkSkipChildren,
            };
            if head_row.first_child().is_none() || n.first_child().is_none() {
                return WalkStatus::WalkSkipChildren;
            }
            if let Some(first_row) = head_row.first_child() {
                let mut cur = first_row.first_child();
                while let Some(c) = cur {
                    cells[0].push(c);
                    cur = c.next();
                }
            }
            let mut i = 1;
            let mut row = n.first_child().and_then(|f| f.next());
            while let Some(table_row) = row {
                cells.push(Vec::new());
                let mut cur = table_row.first_child();
                while let Some(c) = cur {
                    cells[i].push(c);
                    cur = c.next();
                }
                i += 1;
                row = table_row.next();
            }

            let mut max_width = 0;
            for col in 0..cells[0].len() {
                for row in &cells {
                    let Some(&cell) = row.get(col) else {
                        break;
                    };
                    let mut width = cell.token_len() as i32;
                    if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                        let mut extra = 0i32;
                        crate::ast::walk_raw(cell, &mut |cn, entering| {
                            if entering {
                                extra += self.base.space(cn.tokens()).len() as i32
                                    - cn.tokens().len() as i32;
                            }
                            WalkStatus::WalkContinue
                        });
                        width += extra;
                    }
                    cell.data_mut().table_cell_content_width = width;
                    if max_width < width {
                        max_width = width;
                    }
                }
                for row in &cells {
                    let Some(cell) = row.get(col) else {
                        break;
                    };
                    cell.data_mut().table_cell_content_max_width = max_width;
                }
                max_width = 0;
            }
        } else {
            self.base.newline();
            if !self.is_last_node(n) && self.without_kramdown_block_ial(n) {
                self.base.write_byte(b'\n');
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_table_head(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !entering {
            if let Some(head_row) = n.first_child() {
                let mut th = head_row.first_child();
                while let Some(cell) = th {
                    if NodeType::NodeKramdownSpanIAL == cell.typ() {
                        th = cell.next();
                        continue;
                    }
                    let align = cell.data().table_cell_align;
                    let max_width = cell.data().table_cell_content_max_width;
                    match align {
                        0 => {
                            self.base.write_str("| -");
                            let padding = max_width - 1;
                            if 0 < padding {
                                self.base.write(&vec![b'-'; padding as usize]);
                            }
                            if !self.base.options.protyle_wysiwyg {
                                self.base.write_byte(b' ');
                            }
                        }
                        1 => {
                            self.base.write_str("| :-");
                            let padding = max_width - 2;
                            if 0 < padding {
                                self.base.write(&vec![b'-'; padding as usize]);
                            }
                            if !self.base.options.protyle_wysiwyg {
                                self.base.write_byte(b' ');
                            }
                        }
                        2 => {
                            self.base.write_str("| :-");
                            let padding = max_width - 3;
                            if 0 < padding {
                                self.base.write(&vec![b'-'; padding as usize]);
                            }
                            self.base.write_str(": ");
                        }
                        3 => {
                            self.base.write_str("| -");
                            let padding = max_width - 2;
                            if 0 < padding {
                                self.base.write(&vec![b'-'; padding as usize]);
                            }
                            self.base.write_str(": ");
                        }
                        _ => {}
                    }
                    th = cell.next();
                }
            }
            self.base.write_str("|\n");
        }
        WalkStatus::WalkContinue
    }

    fn render_table_cell(&mut self, n: Node, entering: bool) -> WalkStatus {
        let padding = n.data().table_cell_content_max_width - n.data().table_cell_content_width;
        if entering {
            self.base.write_byte(b'|');
            // ProtyleExportMdRenderer 不写出内部 IAL，改为输出片段静态 HTML。
            if !self.protyle_export && n.data().table_cell_rich.is_some() {
                let mut ial = n.ial_raw();
                if let Some(rich) = n.data().table_cell_rich.as_ref() {
                    ial.push(("table-cell-rich".to_string(), rich.encode()));
                }
                self.base.write(&ial_to_tokens(&ial));
            }
            if !self.base.options.protyle_wysiwyg {
                self.base.write_byte(b' ');
                match n.data().table_cell_align {
                    2 => {
                        self.base.write(&vec![b' '; (padding / 2).max(0) as usize]);
                    }
                    3 => {
                        self.base.write(&vec![b' '; padding.max(0) as usize]);
                    }
                    _ => {}
                }
            }
            if self.protyle_export {
                if let Some(content) =
                    crate::render::table_rich::table_cell_rich_inline_html(n, &self.base.options)
                {
                    self.base.write(&content);
                    return WalkStatus::WalkSkipChildren;
                }
            }
            let include_ial = !self.protyle_export || !self.base.options.omit_table_cell_ial;
            if include_ial
                && self.base.options.kramdown_span_ial
                && n.data().table_cell_rich.is_none()
            {
                if let Some(tokens) = table_cell_full_ial_tokens(n) {
                    self.base.write(&tokens);
                }
            }
            if include_ial
                && !self.base.options.kramdown_span_ial
                && n.data().table_cell_rich.is_none()
            {
                if let Some(tokens) = table_cell_struct_ial_tokens(n) {
                    self.base.write(&tokens);
                }
            }
        } else if !self.base.options.protyle_wysiwyg {
            match n.data().table_cell_align {
                2 => {
                    self.base.write(&vec![b' '; (padding / 2).max(0) as usize]);
                }
                3 => {}
                _ => {
                    self.base.write(&vec![b' '; padding.max(0) as usize]);
                }
            }
            self.base.write_byte(b' ');
        }
        WalkStatus::WalkContinue
    }

    fn render_footnotes_def(&mut self, n: Node, entering: bool) -> WalkStatus {
        if entering {
            self.push_writer();
            self.base.write_byte(b'[');
            self.base.write(n.tokens());
            self.base.write_str("]: ");
        } else {
            let child = self.pop_writer();
            let buf = String::from_utf8_lossy(&child).to_string();
            let lines: Vec<&str> = buf.split('\n').collect();
            let mut content = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if 0 == i {
                    content.extend_from_slice(line.as_bytes());
                    content.push(b'\n');
                } else if line.is_empty() {
                    content.push(b'\n');
                } else {
                    content.extend_from_slice(b"    ");
                    content.extend_from_slice(line.as_bytes());
                    content.push(b'\n');
                }
            }
            self.base.write(&content);
        }
        WalkStatus::WalkContinue
    }

    fn render_kramdown_block_ial(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !(crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial) {
            return WalkStatus::WalkContinue;
        }
        if let Some(prev) = n.previous() {
            if NodeType::NodeListItem == prev.typ()
                || NodeType::NodeMindmapItem == prev.typ()
                || (!self.protyle_export && NodeType::NodeTabItem == prev.typ())
            {
                return WalkStatus::WalkContinue;
            }
        }
        if entering {
            self.base.newline();
            if super::is_doc_ial(n.tokens()) {
                self.base.write_byte(b'\n');
            }
            self.base.write(n.tokens());
        } else {
            let parent = n.parent();
            let is_list = parent
                .map(|p| {
                    matches!(
                        p.typ(),
                        NodeType::NodeListItem
                            | NodeType::NodeMindmapItem
                            | NodeType::NodeList
                            | NodeType::NodeMindmap
                    )
                })
                .unwrap_or(false);
            if is_list {
                if let Some(p) = parent {
                    if let Some(ld) = p.data().list_data.as_ref() {
                        if !ld.tight {
                            self.base.newline();
                        }
                    }
                }
            } else {
                self.base.newline();
            }
            self.base.write_byte(b'\n');
        }
        WalkStatus::WalkContinue
    }

    fn render_kramdown_span_ial(&mut self, n: Node, entering: bool) -> WalkStatus {
        if !self.base.options.kramdown_span_ial || is_table_cell_ial_node(n) {
            return WalkStatus::WalkContinue;
        }
        if entering {
            self.base.write(n.tokens());
        } else if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
            if let Some(prev) = n.previous() {
                if super::contain_text_mark(prev, &["code", "inline-math", "kbd"]) {
                    if let Some(next) = n.next() {
                        let mut text = next.text();
                        if text.starts_with(editor::ZWSP) {
                            text = text.trim_start_matches(editor::ZWSP).to_string();
                        }
                        if let Some(c) = text.chars().next() {
                            if is_letter_or_digit(c) {
                                self.base.write_byte(b' ');
                            }
                        }
                    }
                }
            }
        }
        WalkStatus::WalkContinue
    }

    fn render_text_mark(&mut self, n: Node, entering: bool) -> WalkStatus {
        let special = super::contain_text_mark(n, &["code", "inline-math", "kbd"]);
        if entering {
            if special {
                if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                    let mut text = n.previous().map(|p| p.text()).unwrap_or_default();
                    if text.ends_with(editor::ZWSP) {
                        text = text.trim_end_matches(editor::ZWSP).to_string();
                    }
                    if let Some(c) = text.chars().next_back() {
                        if is_letter_or_digit(c) {
                            self.base.write_byte(b' ');
                        }
                    }
                }
            } else {
                self.base.text_auto_space_previous(n);
            }

            let attrs = self.render_text_mark_attrs(n);
            self.base.tag("span", &attrs, false);
            let mut text_content = n.data().text_mark_text_content.clone();
            if n.parent_is(&[NodeType::NodeTableCell]) {
                text_content = text_content.replace("\\|", "|");
                if !n.is_text_mark_type("code") {
                    text_content = text_content.replace('|', "\\|");
                } else {
                    text_content = text_content.replace('|', "&#124;");
                }
                text_content = text_content.replace('\n', "<br/>");
                if n.data().text_mark_type.contains("code") {
                    text_content = text_content.replace("<br/>", "");
                }
            }
            if (crate::CAP_AUTO_SPACE && self.base.options.auto_space)
                && !super::contain_text_mark(n, &["block-ref", "code", "inline-math", "kbd", "tag"])
            {
                text_content =
                    String::from_utf8_lossy(&self.base.space(text_content.as_bytes())).to_string();
            }
            self.base.write_str(&text_content);
        } else {
            self.base.write_str("</span>");
            if special {
                if crate::CAP_AUTO_SPACE && self.base.options.auto_space {
                    if let Some(next) = n.next() {
                        let mut text = next.text();
                        if text.starts_with(editor::ZWSP) {
                            text = text.trim_start_matches(editor::ZWSP).to_string();
                        }
                        if let Some(c) = text.chars().next() {
                            if is_letter_or_digit(c) {
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

    /// 对应 Go 版 ProtyleExportMdRenderer.renderTextMark。
    fn render_text_mark_export(&mut self, n: Node, entering: bool) -> WalkStatus {
        let is_strong_em = super::contain_text_mark(n, &["strong", "em", "s"])
            && !n.is_text_mark_type("inline-math");
        let text_mark_text_content = n.data().text_mark_text_content.clone();
        let has_backtick = text_mark_text_content.contains('`');

        if entering {
            let marker = self.render_md_marker(n, entering);
            let special = n.is_text_mark_type("a")
                || n.is_text_mark_type("inline-memo")
                || n.is_text_mark_type("block-ref")
                || n.is_text_mark_type("file-annotation-ref")
                || n.is_text_mark_type("inline-math");
            if !special {
                let mut text_content = text_mark_text_content.clone();
                if n.is_text_mark_type("code") {
                    text_content = crate::html::unescape_string(&text_content);
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        text_content = crate::lex::repeat_backslash_before_pipe(&text_content);
                    }
                }
                if is_strong_em {
                    // 填充空格以满足 Markdown 语法
                    if let Some(first_rune) = text_content.chars().next() {
                        let before_is_whitespace = crate::lex::is_unicode_whitespace(first_rune);
                        let before_is_punct = crate::char_class::is_punct_or_symbol(first_rune);
                        if before_is_whitespace || before_is_punct {
                            self.base.write_byte(b' ');
                        }
                    }
                }
                self.base.write_str(&marker);
                if has_backtick {
                    self.base.write_byte(b' ');
                }
                self.base.write_str(&text_content);
            } else {
                self.base.write_str(&marker);
                if has_backtick {
                    self.base.write_byte(b' ');
                }
            }
        } else {
            let marker = self.render_md_marker(n, entering);
            if has_backtick {
                self.base.write_byte(b' ');
            }
            self.base.write_str(&marker);
            if let Some(next) = n.next() {
                if NodeType::NodeTextMark == next.typ() {
                    // 通过零宽空格来区隔相邻的 Markdown 标记符
                    self.base.write_str(editor::ZWSP);
                } else if is_strong_em {
                    if let Some(last_rune) = text_mark_text_content.chars().next_back() {
                        let after_is_whitespace = crate::lex::is_unicode_whitespace(last_rune);
                        let after_is_punct = crate::char_class::is_punct_or_symbol(last_rune);
                        if after_is_whitespace || after_is_punct {
                            self.base.write_byte(b' ');
                        }
                    }
                }
            }
        }
        WalkStatus::WalkContinue
    }

    /// 对应 Go 版 ProtyleExportMdRenderer.renderMdMarker。
    fn render_md_marker(&mut self, n: Node, entering: bool) -> String {
        let mut types: Vec<String> = n
            .data()
            .text_mark_type
            .split(' ')
            .map(|s| s.to_string())
            .collect();
        if 1 == types.len() {
            let typ = types[0].clone();
            return self.render_md_marker0(n, &typ, entering);
        }

        // 重新排序：把 a / inline-memo / block-ref / file-annotation-ref / inline-math 放最前，code 放最后。
        let mut tmp: Vec<String> = Vec::new();
        let mut code = String::new();
        let mut i = 0;
        while i < types.len() {
            let typ = types[i].clone();
            if "a" == typ
                || "inline-memo" == typ
                || "block-ref" == typ
                || "file-annotation-ref" == typ
                || "inline-math" == typ
            {
                tmp.push(typ);
                types.remove(i);
                break;
            }
            if "code" == typ {
                code = typ;
                types.remove(i);
                break;
            }
            if "text" == typ {
                i += 1;
                continue;
            }
            i += 1;
        }
        tmp.extend(types);
        if !code.is_empty() {
            tmp.push(code.clone());
        }
        // 过滤掉 text 类型
        let mut types: Vec<String> = tmp.into_iter().filter(|t| "text" != t).collect();
        if types.is_empty() {
            return String::new();
        }

        let mut ret = String::new();
        let typ = types[0].clone();
        if "a" == typ
            || "inline-memo" == typ
            || "block-ref" == typ
            || "file-annotation-ref" == typ
            || "inline-math" == typ
        {
            types.remove(0);
            if entering {
                for t in &types {
                    if "code" != t {
                        ret.push_str(&self.render_md_marker1(n, t, entering));
                    }
                }
                match typ.as_str() {
                    "a" => {
                        ret.push('[');
                        for t in &types {
                            if "code" == t {
                                ret.push_str(&self.render_md_marker1(n, t, entering));
                            }
                        }
                        return ret;
                    }
                    "block-ref" => {
                        let content = n.data().text_mark_text_content.replace('\'', "&apos;");
                        ret.push_str("((");
                        ret.push_str(&n.data().text_mark_block_ref_id);
                        if "s" == n.data().text_mark_block_ref_subtype {
                            ret.push_str(" \"");
                            ret.push_str(&content);
                            ret.push('"');
                        } else {
                            ret.push_str(" '");
                            ret.push_str(&content);
                            ret.push('\'');
                        }
                        ret.push_str("))");
                    }
                    "file-annotation-ref" => {
                        let content = n.data().text_mark_text_content.replace('\'', "&apos;");
                        ret.push_str("<<");
                        ret.push_str(&n.data().text_mark_file_annotation_ref_id);
                        ret.push_str(" \"");
                        ret.push_str(&content);
                        ret.push('"');
                        ret.push_str(">>");
                    }
                    "inline-memo" => {
                        ret.push_str(&n.data().text_mark_text_content);
                        if n.is_next_same_inline_memo() {
                            return ret;
                        }
                        let content = n
                            .data()
                            .text_mark_inline_memo_content
                            .replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                        if !content.is_empty() {
                            let last_rune = n
                                .data()
                                .text_mark_text_content
                                .chars()
                                .next_back()
                                .unwrap_or('\u{FFFD}');
                            if crate::char_class::is_cjk(last_rune) {
                                ret.push_str("<sup>（");
                                ret.push_str(&content);
                                ret.push_str("）</sup>");
                            } else {
                                ret.push_str("<sup>(");
                                ret.push_str(&content);
                                ret.push_str(")</sup>");
                            }
                        }
                    }
                    "inline-math" => {
                        let mut content = n.data().text_mark_inline_math_content.clone();
                        if n.parent_is(&[NodeType::NodeTableCell]) {
                            content = crate::lex::repeat_backslash_before_pipe(&content);
                            content = content.replace('\n', "<br/>");
                        }
                        content = content.replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                        ret.push('$');
                        ret.push_str(&content);
                        ret.push('$');
                    }
                    _ => {}
                }
            } else {
                if typ == "a" {
                    let href = self.export_link_href(n);
                    ret.push_str(&String::from_utf8_lossy(
                        &crate::lex::escape_protyle_markers(
                            n.data().text_mark_text_content.as_bytes(),
                        ),
                    ));
                    for t in &types {
                        if "code" == t {
                            ret.push_str(&self.render_md_marker1(n, t, entering));
                        }
                    }
                    ret.push_str("](");
                    ret.push_str(&href);
                    if !n.data().text_mark_a_title.is_empty() {
                        ret.push_str(" \"");
                        ret.push_str(&crate::html::unescape_html_str(&n.data().text_mark_a_title));
                        ret.push('"');
                    }
                    ret.push(')');
                }
                for t in &types {
                    if "code" != t {
                        ret.push_str(&self.render_md_marker1(n, t, entering));
                    }
                }
            }
        } else {
            if !entering {
                types.reverse();
            }
            for (i, t) in types.iter().enumerate() {
                ret.push_str(&self.render_md_marker1(n, t, entering));
                if entering && !code.is_empty() && types.len() >= 2 && types.len() - 2 == i {
                    // 最内层是 code 时，渲染 code 标记符前加零宽空格
                    ret.push_str(editor::ZWSP);
                }
                if !entering && !code.is_empty() && 0 == i {
                    ret.push_str(editor::ZWSP);
                }
            }
        }
        ret
    }

    /// 对应 Go 版 ProtyleExportMdRenderer.renderMdMarker0。
    fn render_md_marker0(&mut self, n: Node, typ: &str, entering: bool) -> String {
        let mut ret = String::new();
        match typ {
            "a" => {
                let href = self.export_link_href(n);
                if entering {
                    let mut content = n.data().text_mark_text_content.replace('[', "\\[");
                    content = content.replace(']', "\\]");
                    content = crate::html::unescape_html_str(&content);
                    ret.push('[');
                    ret.push_str(&content);
                    ret.push_str("](");
                    ret.push_str(&href);
                    if !n.data().text_mark_a_title.is_empty() {
                        let title = crate::html::unescape_html_str(&n.data().text_mark_a_title)
                            .replace("&lt;", "<")
                            .replace("&gt;", ">");
                        ret.push_str(" \"");
                        ret.push_str(&title);
                        ret.push('"');
                    }
                    ret.push(')');
                }
            }
            "block-ref" => {
                if entering {
                    let content = n.data().text_mark_text_content.replace('\'', "&apos;");
                    ret.push_str("((");
                    ret.push_str(&n.data().text_mark_block_ref_id);
                    if "s" == n.data().text_mark_block_ref_subtype {
                        ret.push_str(" \"");
                        ret.push_str(&content);
                        ret.push('"');
                    } else {
                        ret.push_str(" '");
                        ret.push_str(&content);
                        ret.push('\'');
                    }
                    ret.push_str("))");
                }
            }
            "file-annotation-ref" => {
                if entering {
                    let content = n.data().text_mark_text_content.replace('\'', "&apos;");
                    ret.push_str("<<");
                    ret.push_str(&n.data().text_mark_file_annotation_ref_id);
                    ret.push_str(" \"");
                    ret.push_str(&content);
                    ret.push('"');
                    ret.push_str(">>");
                }
            }
            "inline-memo" => {
                if entering {
                    ret.push_str(&n.data().text_mark_text_content);
                    if n.is_next_same_inline_memo() {
                        return ret;
                    }
                    let content = n
                        .data()
                        .text_mark_inline_memo_content
                        .replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                    if !content.is_empty() {
                        let last_rune = n
                            .data()
                            .text_mark_text_content
                            .chars()
                            .next_back()
                            .unwrap_or('\u{FFFD}');
                        if crate::char_class::is_cjk(last_rune) {
                            ret.push_str("<sup>（");
                            ret.push_str(&content);
                            ret.push_str("）</sup>");
                        } else {
                            ret.push_str("<sup>(");
                            ret.push_str(&content);
                            ret.push_str(")</sup>");
                        }
                    }
                }
            }
            "inline-math" => {
                if entering {
                    let mut content = n.data().text_mark_inline_math_content.clone();
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        content = crate::lex::repeat_backslash_before_pipe(&content);
                        content = content.replace('\n', "<br/>");
                    }
                    content = content.replace(editor::IAL_VAL_ESC_NEWLINE, " ");
                    ret.push('$');
                    ret.push_str(&content);
                } else {
                    ret.push('$');
                }
            }
            _ => ret.push_str(&self.render_md_marker1(n, typ, entering)),
        }
        ret
    }

    /// 对应 Go 版 ProtyleExportMdRenderer.renderMdMarker1。
    fn render_md_marker1(&self, n: Node, typ: &str, entering: bool) -> String {
        let mut ret = String::new();
        match typ {
            "strong" => ret.push_str("**"),
            "em" => ret.push('*'),
            "code" => {
                let content = &n.data().text_mark_text_content;
                if content.contains("``") {
                    ret.push('`');
                } else if content.contains('`') {
                    ret.push_str("``");
                } else {
                    ret.push('`');
                }
            }
            "tag" => ret.push('#'),
            "s" => ret.push_str("~~"),
            "mark" => ret.push_str("=="),
            "u" => ret.push_str(if entering { "<u>" } else { "</u>" }),
            "sup" => ret.push_str(if entering { "<sup>" } else { "</sup>" }),
            "sub" => ret.push_str(if entering { "<sub>" } else { "</sub>" }),
            "kbd" => ret.push_str(if entering { "<kbd>" } else { "</kbd>" }),
            "text" => {
                if entering {
                    ret.push_str("<span data-type=\"text\"");
                    for (k, v) in &n.data().kramdown_ial {
                        ret.push(' ');
                        ret.push_str(k);
                        ret.push_str("=\"");
                        ret.push_str(v);
                        ret.push('"');
                    }
                    ret.push('>');
                } else {
                    ret.push_str("</span>");
                }
            }
            _ => {}
        }
        ret
    }

    /// 对应 Go 版 BaseRenderer.EncodeLinkSpace + LinkPath。
    fn export_link_href(&self, n: Node) -> String {
        let href = self.base.link_path(n.data().text_mark_a_href.as_bytes());
        let mut href = String::from_utf8_lossy(&href).to_string();
        href = crate::html::unescape_html_str(&href);
        if self.base.options.prevent_encode_link_space {
            href
        } else {
            href.replace(' ', "%20")
        }
    }

    fn render_text_mark_attrs(&self, n: Node) -> Vec<(String, String)> {
        let d = n.data();
        let mut attrs = vec![("data-type".to_string(), d.text_mark_type.clone())];
        if !d.text_mark_flashcard_occlusion_id.is_empty() {
            attrs.push((
                "data-occlusion-id".to_string(),
                d.text_mark_flashcard_occlusion_id.clone(),
            ));
        }
        for typ in d.text_mark_type.split(' ') {
            if "block-ref" == typ {
                attrs.push((
                    "data-subtype".to_string(),
                    d.text_mark_block_ref_subtype.clone(),
                ));
                attrs.push(("data-id".to_string(), d.text_mark_block_ref_id.clone()));
            } else if "a" == typ {
                let mut href =
                    String::from_utf8_lossy(&self.base.link_path(d.text_mark_a_href.as_bytes()))
                        .to_string();
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    href = href.replace("\\|", "|");
                    href = href.replace('|', "\\|");
                }
                attrs.push(("data-href".to_string(), href));
                if !d.text_mark_a_title.is_empty() {
                    let mut title = d.text_mark_a_title.clone();
                    if n.parent_is(&[NodeType::NodeTableCell]) {
                        title = title.replace("\\|", "|");
                        title = title.replace('|', "\\|");
                    }
                    attrs.push(("data-title".to_string(), title));
                }
            } else if "inline-math" == typ {
                attrs.push(("data-subtype".to_string(), "math".to_string()));
                let mut content = d.text_mark_inline_math_content.clone();
                if n.parent_is(&[NodeType::NodeTableCell]) {
                    content = content.replace('|', "&#124;");
                    content = content.replace('\n', "<br/>");
                }
                content = super::escape_html_str(&content);
                attrs.push(("data-content".to_string(), content));
                attrs.push(("contenteditable".to_string(), "false".to_string()));
                attrs.push(("class".to_string(), "render-node".to_string()));
            } else if "file-annotation-ref" == typ {
                attrs.push((
                    "data-id".to_string(),
                    super::escape_html_str(&d.text_mark_file_annotation_ref_id),
                ));
            } else if "inline-memo" == typ {
                attrs.push((
                    "data-inline-memo-content".to_string(),
                    d.text_mark_inline_memo_content.clone(),
                ));
            }
        }
        attrs
    }

    /// Shared dispatch for Go FormatRenderer.renderTabs and ProtyleExportMdRenderer.renderTabs.
    fn render_tabs(&mut self, n: Node, entering: bool) -> WalkStatus {
        self.base.newline();
        if self.protyle_export && !self.base.options.tabs_markdown {
            if entering
                && n.typ() == NodeType::NodeTabItem
                && tab_title_block(n).is_none()
                && !n.data().tab_item_title.is_empty()
            {
                let title = self.export_tab_title(n);
                self.base.write_str(&title);
                self.base.write_str("\n\n");
            }
            return WalkStatus::WalkContinue;
        }
        if n.typ() == NodeType::NodeTabs {
            let fence_length = self.tabs_fence_length(n);
            self.base.write_str(&":".repeat(fence_length));
            if entering {
                self.base.write_str(" tabs");
            }
            if self.protyle_export {
                self.base.write_str("\n\n");
            } else {
                self.base.newline();
            }
            if !entering
                && (self.protyle_export
                    || !(crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial))
                && n.ial_attr("tabs-task") == "true"
            {
                self.base.write_str("{: tabs-task=\"true\"}\n");
                if self.protyle_export {
                    self.base.write_byte(b'\n');
                }
            }
            return WalkStatus::WalkContinue;
        }
        if !entering {
            return WalkStatus::WalkContinue;
        }
        self.base.write_str("@tab");
        if n.parent().is_some_and(|parent| {
            parent.typ() == NodeType::NodeTabs
                && !n.data().id.is_empty()
                && n.data().id == parent.ial_attr("tabs-active-id")
        }) {
            self.base.write_str(":active");
        }
        let title = if self.protyle_export {
            self.export_tab_title(n).trim().to_string()
        } else if tab_title_block(n).is_none() {
            n.data().tab_item_title.clone()
        } else {
            String::new()
        };
        if !title.is_empty() {
            self.base.write_byte(b' ');
            self.base.write_str(&title.replace(['\r', '\n'], " "));
        }
        if self.protyle_export {
            let task = n.ial_attr("tabs-task");
            if !task.is_empty() {
                self.base.newline();
                self.base.write(&ial_to_tokens(&[(
                    "tabs-task".into(),
                    crate::html::escape_attr_val(&task),
                )]));
            }
            self.base.write_str("\n\n");
        } else {
            self.base.newline();
            if crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial {
                if !n.data().kramdown_ial.is_empty() {
                    self.base.write(&ial_to_tokens(&n.data().kramdown_ial));
                } else if let Some(ial) = n
                    .next()
                    .filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL)
                {
                    self.base.write(ial.tokens());
                }
                self.base.newline();
            } else {
                let task = n.ial_attr("tabs-task");
                if !task.is_empty() {
                    self.base.write(&ial_to_tokens(&[(
                        "tabs-task".into(),
                        crate::html::escape_attr_val(&task),
                    )]));
                    self.base.newline();
                }
            }
            self.base.write_byte(b'\n');
        }
        WalkStatus::WalkContinue
    }

    fn tabs_fence_length(&mut self, n: Node) -> usize {
        if self.tabs_fence_lengths.is_none() {
            let mut lengths = std::collections::HashMap::new();
            let mut depths = std::collections::HashMap::<Node, usize>::new();
            crate::ast::walk_raw(self.root, &mut |node, entering| {
                if !entering {
                    let mut depth = *depths.get(&node).unwrap_or(&0);
                    if node.typ() == NodeType::NodeTabs {
                        depth += 1;
                        lengths.insert(node, depth + 2);
                    }
                    if let Some(parent) = node.parent() {
                        let parent_depth = depths.entry(parent).or_default();
                        *parent_depth = (*parent_depth).max(depth);
                    }
                }
                WalkStatus::WalkContinue
            });
            self.tabs_fence_lengths = Some(lengths);
        }
        self.tabs_fence_lengths
            .as_ref()
            .and_then(|lengths| lengths.get(&n))
            .copied()
            .unwrap_or(3)
    }

    fn export_tab_title(&self, n: Node) -> String {
        // Keep the original title paragraph as the render root so suppression does not hide it.
        let mut tree = crate::parse::inline(
            "",
            n.data().tab_item_title.as_bytes(),
            self.parse_options.clone(),
        );
        if let Some(paragraph) = tree.root_raw().first_child() {
            while let Some(inline) = paragraph.first_child() {
                tree.root_raw().append_child(inline);
            }
            paragraph.unlink();
        }
        if !self.base.options.tabs_markdown {
            crate::parse::text_marks2inlines(&mut tree, &self.parse_options);
        }
        let mut renderer =
            Self::new_protyle_export(&tree, self.base.options.clone(), self.parse_options.clone());
        if let Some(title) = tab_title_block(n) {
            renderer.root = title;
            renderer._owner = title.owner();
        }
        String::from_utf8_lossy(&renderer.render()).into_owned()
    }

    fn setext_heading_len(&self, n: Node) -> usize {
        let mut buf = Vec::new();
        crate::ast::walk_raw(n, &mut |cn, entering| {
            if entering
                && (NodeType::NodeText == cn.typ()
                    || NodeType::NodeLinkText == cn.typ()
                    || NodeType::NodeSoftBreak == cn.typ())
            {
                buf.extend_from_slice(cn.tokens());
            }
            WalkStatus::WalkContinue
        });
        let content = String::from_utf8_lossy(&buf).replace(editor::CARET, "");
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

    fn normalized_task_list_item_marker(&self, n: Node) -> u8 {
        let d = n.data();
        let marker: u8 = if 0 != d.task_list_item_marker {
            if b'x' == d.task_list_item_marker {
                b'X'
            } else {
                d.task_list_item_marker
            }
        } else if d.task_list_item_checked {
            b'X'
        } else {
            b' '
        };
        if self.base.options.export_normalize_task_list_marker && marker != b' ' && marker != b'X' {
            return b'X';
        }
        marker
    }

    fn tag_src_path(&self, tokens: &[u8]) -> Vec<u8> {
        if let Some(src_index) = find_sub(tokens, b"src=\"") {
            if 0 < src_index {
                let src = &tokens[src_index + 5..];
                if src.is_empty() {
                    return tokens.to_vec();
                }
                let target_src = self.base.link_path(src);
                let mut origin_src = target_src.clone();
                if target_src.starts_with(b"//") {
                    let mut s = b"https:".to_vec();
                    s.extend_from_slice(&target_src);
                    origin_src = s;
                }
                return replace_bytes(tokens, src, &origin_src);
            }
        }
        tokens.to_vec()
    }

    fn without_kramdown_block_ial(&self, n: Node) -> bool {
        !(crate::CAP_KRAMDOWN_BLOCK_IAL && self.base.options.kramdown_block_ial)
            || n.ial_raw().is_empty()
            || n.next().map(|x| x.typ()) != Some(NodeType::NodeKramdownBlockIAL)
    }

    fn newline_before_block(&mut self, n: Node) {
        if self.protyle_export {
            return;
        }
        if n.parent_is(&[NodeType::NodeTableCell]) {
            return;
        }
        if let Some(prev) = n.previous() {
            if !prev.is_block()
                && NodeType::NodeKramdownBlockIAL != prev.typ()
                && NodeType::NodeTaskListItemMarker != prev.typ()
            {
                self.base.newline();
            }
        }
    }

    fn is_last_node(&self, n: Node) -> bool {
        if self.root == n || n.parent().is_none() {
            return true;
        }
        if n.next().is_some() {
            return false;
        }
        let parent = n.parent().unwrap();
        if NodeType::NodeDocument == parent.typ() {
            return self.root.last_child() == Some(n);
        }
        let mut cur = parent;
        loop {
            let p = match cur.parent() {
                Some(p) => p,
                None => return true,
            };
            if NodeType::NodeDocument == p.typ() {
                break;
            }
            cur = p;
        }
        self.root.last_child() == Some(cur)
    }
}

/// 对应 Go 版 render.NoHighlight：这些语言不需要语法高亮。
fn no_highlight_language(language: &str) -> bool {
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

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| matches!(b, b' ' | b'\t'))
}

fn trim_space(buf: &[u8]) -> Vec<u8> {
    let start = buf
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\t' | b'\n'))
        .unwrap_or(buf.len());
    let mut end = buf.len();
    while end > start && matches!(buf[end - 1], b' ' | b'\t' | b'\n') {
        end -= 1;
    }
    buf[start..end].to_vec()
}

fn trim_prefix(tokens: &[u8], prefix: &[u8]) -> Vec<u8> {
    if tokens.starts_with(prefix) {
        tokens[prefix.len()..].to_vec()
    } else {
        tokens.to_vec()
    }
}

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
}

fn replace_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
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

fn table_head_column_align(node: Node) -> i32 {
    let parent = match node.parent() {
        Some(p) => p,
        None => return 0,
    };
    let grand = match parent.parent() {
        Some(p) => p,
        None => return 0,
    };
    let mut column = 0;
    let mut prev = node.previous();
    while let Some(p) = prev {
        if NodeType::NodeTableCell == p.typ() {
            column += 1;
        }
        prev = p.previous();
    }
    let mut table = grand;
    if NodeType::NodeTableHead == table.typ() {
        table = match table.parent() {
            Some(p) => p,
            None => return 0,
        };
    }
    let first_child = match table.first_child() {
        Some(c) => c,
        None => return 0,
    };
    if NodeType::NodeTableHead != first_child.typ() {
        return 0;
    }
    let first_row = match first_child.first_child() {
        Some(c) => c,
        None => return 0,
    };
    let mut cell = first_row.first_child();
    while let Some(c) = cell {
        if NodeType::NodeTableCell == c.typ() {
            if 0 == column {
                return c.data().table_cell_align;
            }
            column -= 1;
        }
        cell = c.next();
    }
    0
}

fn table_cell_struct_ial_tokens(node: Node) -> Option<Vec<u8>> {
    let mut structural: Vec<(String, String)> = Vec::new();
    let mut has_text_align = false;
    let mut style_index: isize = -1;
    for (k, v) in node.ial_raw() {
        if "colspan" == k || "rowspan" == k || ("class" == k && "fn__none" == v) {
            structural.push((k, v));
        } else if "style" == k {
            let mut keep_style = false;
            for declaration in v.split(';') {
                if let Some(idx) = declaration.find(':') {
                    let property = declaration[..idx].trim();
                    if property.eq_ignore_ascii_case("text-align") {
                        has_text_align = true;
                        keep_style = true;
                    } else if property.eq_ignore_ascii_case("vertical-align") {
                        keep_style = true;
                    }
                }
            }
            if keep_style {
                style_index = structural.len() as isize;
                structural.push((k, v));
            }
        }
    }
    if !has_text_align && node.data().table_cell_align != table_head_column_align(node) {
        let align = match node.data().table_cell_align {
            1 => "left",
            2 => "center",
            3 => "right",
            _ => "start",
        };
        if 0 <= style_index {
            let idx = style_index as usize;
            let mut style = structural[idx].1.trim().to_string();
            if !style.is_empty() && !style.ends_with(';') {
                style.push(';');
            }
            structural[idx] = ("style".to_string(), format!("{style} text-align: {align};"));
        } else {
            structural.push(("style".to_string(), format!("text-align: {align};")));
        }
    }
    if structural.is_empty() {
        return None;
    }
    Some(ial_to_tokens(&structural))
}

fn table_cell_full_ial_tokens(node: Node) -> Option<Vec<u8>> {
    let mut ial = node.ial_raw();
    if node.data().table_cell_align != table_head_column_align(node) {
        let mut style_index: isize = -1;
        let mut has_text_align = false;
        for (i, (k, v)) in ial.iter().enumerate() {
            if "style" != k {
                continue;
            }
            style_index = i as isize;
            for declaration in v.split(';') {
                if let Some(idx) = declaration.find(':') {
                    if declaration[..idx].trim().eq_ignore_ascii_case("text-align") {
                        has_text_align = true;
                    }
                }
            }
        }
        if !has_text_align {
            let align = match node.data().table_cell_align {
                1 => "left",
                2 => "center",
                3 => "right",
                _ => "start",
            };
            if 0 <= style_index {
                let idx = style_index as usize;
                let mut style = ial[idx].1.trim().to_string();
                if !style.is_empty() && !style.ends_with(';') {
                    style.push(';');
                }
                ial[idx].1 = format!("{style} text-align: {align};");
            } else {
                ial.push(("style".to_string(), format!("text-align: {align};")));
            }
        }
    }
    if ial.is_empty() {
        return None;
    }
    Some(ial_to_tokens(&ial))
}

fn is_table_cell_ial_node(node: Node) -> bool {
    if let Some(parent) = node.parent() {
        if NodeType::NodeTableCell == parent.typ()
            && parent.first_child() == Some(node)
            && !parent.ial_raw().is_empty()
        {
            return true;
        }
    }
    if let Some(prev) = node.previous() {
        if NodeType::NodeTableCell == prev.typ()
            && !prev.ial_raw().is_empty()
            && node.tokens() == ial_to_tokens(&prev.ial_raw())
        {
            return true;
        }
    }
    false
}

fn setext_marker_offset_in_list_paragraph(node: Node) -> isize {
    if NodeType::NodeText != node.typ() {
        return -1;
    }
    let parent = match node.parent() {
        Some(p) => p,
        None => return -1,
    };
    if NodeType::NodeParagraph != parent.typ() {
        return -1;
    }
    if parent.parent().map(|p| p.typ()) != Some(NodeType::NodeListItem) {
        return -1;
    }

    let mut line_start = node;
    let mut found_break = false;
    let mut prev = node.previous();
    while let Some(p) = prev {
        if NodeType::NodeSoftBreak == p.typ() || NodeType::NodeHardBreak == p.typ() {
            found_break = true;
            break;
        }
        line_start = p;
        prev = p.previous();
    }
    if !found_break {
        return -1;
    }

    let mut leading_whitespace = 0;
    let mut marker = 0u8;
    let mut marker_count = 0;
    let mut trailing_whitespace = false;
    let mut marker_node: Option<Node> = None;
    let mut marker_offset: isize = -1;
    let mut current = Some(line_start);
    while let Some(cur) = current {
        if NodeType::NodeSoftBreak == cur.typ() || NodeType::NodeHardBreak == cur.typ() {
            break;
        }
        if NodeType::NodeVditorCaret == cur.typ() {
            current = cur.next();
            continue;
        }
        if NodeType::NodeText != cur.typ() {
            return -1;
        }
        let tokens = cur.tokens();
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i..].starts_with(editor::CARET.as_bytes()) {
                i += editor::CARET.len();
                continue;
            }
            let token = tokens[i];
            if 0 == marker {
                if b' ' == token || b'\t' == token {
                    leading_whitespace += 1;
                    if 3 < leading_whitespace {
                        return -1;
                    }
                    i += 1;
                    continue;
                }
                if b'-' != token && b'=' != token {
                    return -1;
                }
                marker = token;
                marker_count = 1;
                marker_node = Some(cur);
                marker_offset = i as isize;
                i += 1;
                continue;
            }
            if marker == token && !trailing_whitespace {
                marker_count += 1;
                i += 1;
                continue;
            }
            if b' ' == token || b'\t' == token {
                trailing_whitespace = true;
                i += 1;
                continue;
            }
            return -1;
        }
        current = cur.next();
    }

    if 0 == marker_count || marker_node != Some(node) {
        return -1;
    }
    marker_offset
}

pub(super) fn escape_setext_heading_markers_in_list_paragraph(
    node: Node,
    tokens: &[u8],
) -> Vec<u8> {
    if NodeType::NodeText != node.typ() {
        return tokens.to_vec();
    }
    let parent = match node.parent() {
        Some(p) => p,
        None => return tokens.to_vec(),
    };
    if NodeType::NodeParagraph != parent.typ()
        || parent.parent().map(|p| p.typ()) != Some(NodeType::NodeListItem)
    {
        return tokens.to_vec();
    }

    let mut offsets: Vec<usize> = Vec::new();
    let offset = setext_marker_offset_in_list_paragraph(node);
    if 0 <= offset {
        offsets.push(offset as usize);
    }
    let mut line_start = match tokens.iter().position(|&b| b == b'\n') {
        Some(p) => p + 1,
        None => 0,
    };
    while 0 < line_start && line_start <= tokens.len() {
        let line_end = match tokens[line_start..].iter().position(|&b| b == b'\n') {
            Some(p) => line_start + p,
            None => tokens.len(),
        };
        let offset = setext_marker_offset(&tokens[line_start..line_end]);
        if 0 <= offset {
            offsets.push(line_start + offset as usize);
        }
        if tokens.len() <= line_end {
            break;
        }
        line_start = line_end + 1;
    }
    if offsets.is_empty() {
        return tokens.to_vec();
    }

    let mut escaped = tokens.to_vec();
    for &off in offsets.iter().rev() {
        escaped.insert(off, b'\\');
    }
    escaped
}

fn setext_marker_offset(tokens: &[u8]) -> isize {
    let mut leading_whitespace = 0;
    let mut marker = 0u8;
    let mut marker_offset: isize = -1;
    let mut trailing_whitespace = false;
    let mut marker_count = 0;
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i..].starts_with(editor::CARET.as_bytes()) {
            i += editor::CARET.len();
            continue;
        }
        let token = tokens[i];
        if 0 == marker {
            if b' ' == token || b'\t' == token {
                leading_whitespace += 1;
                if 3 < leading_whitespace {
                    return -1;
                }
                i += 1;
                continue;
            }
            if b'-' != token && b'=' != token {
                return -1;
            }
            marker = token;
            marker_offset = i as isize;
            marker_count = 1;
            i += 1;
            continue;
        }
        if marker == token && !trailing_whitespace {
            marker_count += 1;
            i += 1;
            continue;
        }
        if b' ' == token || b'\t' == token {
            trailing_whitespace = true;
            i += 1;
            continue;
        }
        return -1;
    }
    if 0 == marker_count {
        return -1;
    }
    marker_offset
}

fn is_tabs_paragraph_marker(line: &[u8]) -> bool {
    let mut line = trim_space(line);
    if line.starts_with(editor::CARET.as_bytes()) {
        line = line[editor::CARET.len()..].to_vec();
    }
    for marker in ["@tab", "@tab:active"] {
        if line == marker.as_bytes()
            || line.starts_with(format!("{marker} ").as_bytes())
            || line.starts_with(format!("{marker}\t").as_bytes())
        {
            return true;
        }
    }
    let mut count = 0;
    while count < line.len() && b':' == line[count] {
        count += 1;
    }
    if count < 3 {
        return false;
    }
    if count == line.len() {
        return true;
    }
    (b' ' == line[count] || b'\t' == line[count])
        && String::from_utf8_lossy(&trim_space(&line[count..])) == "tabs"
}

fn escape_tabs_paragraph_markers(node: Node, tokens: &[u8], within_tabs_only: bool) -> Vec<u8> {
    if !node.parent_is(&[NodeType::NodeParagraph])
        && (within_tabs_only || !node.parent_is(&[NodeType::NodeDocument]))
    {
        return tokens.to_vec();
    }
    if within_tabs_only {
        let mut tabs = None;
        let mut parent = node.parent().and_then(|p| p.parent());
        while let Some(p) = parent {
            if NodeType::NodeTabs == p.typ() {
                tabs = Some(p);
                break;
            }
            parent = p.parent();
        }
        if tabs.is_none() {
            return tokens.to_vec();
        }
    }

    let line_start = node.previous().is_none()
        || node.previous().map(|p| p.typ()) == Some(NodeType::NodeSoftBreak)
        || node.previous().map(|p| p.typ()) == Some(NodeType::NodeHardBreak);
    let mut lines: Vec<Vec<u8>> = tokens.split(|&b| b == b'\n').map(|l| l.to_vec()).collect();
    let mut changed = false;
    for (i, line) in lines.iter_mut().enumerate() {
        if (0 < i || line_start) && is_tabs_paragraph_marker(line) {
            let leading = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(line.len());
            let mut escaped = line[..leading].to_vec();
            escaped.push(b'\\');
            escaped.extend_from_slice(&line[leading..]);
            *line = escaped;
            changed = true;
        }
    }
    if !changed {
        return tokens.to_vec();
    }
    let mut ret = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if 0 < i {
            ret.push(b'\n');
        }
        ret.extend_from_slice(line);
    }
    ret
}

fn tab_title_block(n: Node) -> Option<Node> {
    if n.typ() != NodeType::NodeTabItem {
        return None;
    }
    let mut child = n.first_child();
    while let Some(node) = child {
        if node.typ() != NodeType::NodeKramdownBlockIAL {
            return (node.typ() == NodeType::NodeParagraph
                && node.ial_attr("tabs-title") == "true")
                .then_some(node);
        }
        child = node.next();
    }
    None
}

impl HtmlTableRenderer for FormatRenderer {
    fn table_base(&mut self) -> &mut BaseRenderer {
        &mut self.base
    }
    fn render_table_node(&mut self, node: Node, entering: bool) -> WalkStatus {
        self.render_node(node, entering)
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn standard_export_omits_cell_ial_without_changing_roundtrip_format() {
        let mut tree = Tree::new();
        let cell = tree.new_node_raw(NodeType::NodeTableCell);
        cell.set_ial_attr("style", "text-align: right;");
        cell.set_ial_attr("colspan", "2");
        tree.root_raw().append_child(cell);
        for kramdown_span_ial in [false, true] {
            for omit_table_cell_ial in [false, true] {
                let options = RenderOptions {
                    protyle_wysiwyg: true,
                    kramdown_span_ial,
                    omit_table_cell_ial,
                    ..RenderOptions::default()
                };
                let mut export = FormatRenderer::new_protyle_export(
                    &tree,
                    options.clone(),
                    ParseOptions::default(),
                );
                export.render_table_cell(cell, true);
                assert_eq!(export.base.out.contains(&b'{'), !omit_table_cell_ial);
                let mut roundtrip = FormatRenderer::new(&tree, options, ParseOptions::default());
                roundtrip.render_table_cell(cell, true);
                assert!(roundtrip.base.out.contains(&b'{'));
            }
        }
    }

    #[test]
    fn combined_link_markers_keep_label_between_enter_and_exit() {
        let mut tree = Tree::new();
        let mark = tree.new_node_raw(NodeType::NodeTextMark);
        mark.data_mut().text_mark_type = "strong a code".into();
        mark.data_mut().text_mark_text_content = "label".into();
        mark.data_mut().text_mark_a_href = "some path".into();
        mark.data_mut().text_mark_a_title = "title".into();
        tree.root_raw().append_child(mark);
        let mut renderer = FormatRenderer::new_protyle_export(
            &tree,
            RenderOptions::default(),
            ParseOptions::default(),
        );
        assert_eq!(renderer.render_md_marker(mark, true), "**[`");
        assert_eq!(
            renderer.render_md_marker(mark, false),
            "label`](some%20path \"title\")**"
        );
    }

    #[test]
    fn export_image_tag_escapes_attributes_and_skips_markdown_children() {
        let mut tree = Tree::new();
        let image = tree.new_node_raw(NodeType::NodeImage);
        tree.root_raw().append_child(image);
        image.set_ial_attr("style", "width:20px");
        for (typ, tokens) in [
            (NodeType::NodeLinkText, "[alt]"),
            (NodeType::NodeLinkDest, "some path"),
            (NodeType::NodeLinkTitle, "a\"b"),
        ] {
            let child = tree.new_node_raw(typ);
            child.set_tokens(tokens.as_bytes().to_vec());
            image.append_child(child);
        }
        let options = RenderOptions {
            img_tag: true,
            ..RenderOptions::default()
        };
        let mut renderer =
            FormatRenderer::new_protyle_export(&tree, options, ParseOptions::default());
        assert_eq!(
            renderer.render_node(image, true),
            WalkStatus::WalkSkipChildren
        );
        assert_eq!(
            String::from_utf8(renderer.base.out).unwrap(),
            "<img alt=\"\\[alt\\]\" src=\"some%20path\" title=\"a&quot;b\" style=\"width:20px\" />"
        );
    }
}
