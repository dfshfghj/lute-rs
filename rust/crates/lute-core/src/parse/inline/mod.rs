// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 行级解析器。结构对齐 Go 版 parse 包的 `parseInline` / `walkParseInline`。
//!
//! 分隔符栈使用 arena + 下标模拟 Go 的双向链表，避免 `Rc<RefCell>` 带来的运行时开销。

// 本模块与 Go 版逐行对齐，保留 Go 风格的边界比较等写法。
#![allow(
    clippy::len_zero,
    clippy::int_plus_one,
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::manual_range_contains,
    clippy::byte_char_slices,
    clippy::type_complexity,
    clippy::never_loop
)]

mod autolink;
mod case_fold;
mod code_span;
mod emoji;
mod emphasis;
mod entity;
pub(crate) mod html;
mod index;
pub(crate) mod link;
mod math;
mod misc;
pub(crate) use misc::parse_span_ial;
mod text;
pub(crate) mod text_mark_delimiters;

use std::collections::HashMap;

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::options::ParseOptions;

use index::LinkRefDef;

/// 行级解析上下文，对应 Go 版 parse.InlineContext。
pub(crate) struct InlineContext {
    pub tokens: Vec<u8>,
    pub tokens_len: usize,
    pub pos: usize,
    /// 分隔符 arena；`delimiter_top` 是栈顶下标。
    delimiters: Vec<Delimiter>,
    delimiter_top: Option<usize>,
    /// 方括号 arena；`bracket_top` 是栈顶下标。
    brackets: Vec<Delimiter>,
    bracket_top: Option<usize>,
}

impl InlineContext {
    fn new(tokens: Vec<u8>) -> Self {
        let tokens_len = tokens.len();
        Self {
            tokens,
            tokens_len,
            pos: 0,
            delimiters: Vec::new(),
            delimiter_top: None,
            brackets: Vec::new(),
            bracket_top: None,
        }
    }

    #[inline]
    fn delimiter(&self, idx: usize) -> &Delimiter {
        &self.delimiters[idx]
    }

    #[inline]
    fn delimiter_mut(&mut self, idx: usize) -> &mut Delimiter {
        &mut self.delimiters[idx]
    }

    #[inline]
    fn bracket(&self, idx: usize) -> &Delimiter {
        &self.brackets[idx]
    }

    #[inline]
    fn bracket_mut(&mut self, idx: usize) -> &mut Delimiter {
        &mut self.brackets[idx]
    }

    /// 对应 Go 版 Tree.addBracket。
    fn add_bracket(&mut self, node: Node, index: usize, image: bool) {
        if let Some(top) = self.bracket_top {
            self.bracket_mut(top).bracket_after = true;
        }
        let previous = self.bracket_top;
        let previous_delimiter = self.delimiter_top;
        let idx = self.brackets.len();
        self.brackets.push(Delimiter {
            node,
            typ: 0,
            num: 0,
            original_num: 0,
            unit_len: 0,
            can_open: false,
            can_close: false,
            previous,
            next: None,
            active: true,
            image,
            bracket_after: false,
            index,
            previous_delimiter,
        });
        self.bracket_top = Some(idx);
    }

    /// 对应 Go 版 Tree.removeDelimiter。
    fn remove_delimiter(&mut self, idx: usize) {
        let (previous, next) = {
            let d = &self.delimiters[idx];
            (d.previous, d.next)
        };
        if let Some(prev) = previous {
            self.delimiters[prev].next = next;
        }
        match next {
            None => self.delimiter_top = previous,
            Some(n) => self.delimiters[n].previous = previous,
        }
    }

    /// 对应 Go 版 Tree.removeBracket。
    fn remove_bracket(&mut self) {
        self.bracket_top = self.bracket_top.and_then(|idx| self.bracket(idx).previous);
    }
}

/// 分隔符/方括号节点，对应 Go 版 parse.delimiter。
struct Delimiter {
    node: Node,
    typ: u8,
    num: i32,
    original_num: i32,
    unit_len: usize,
    can_open: bool,
    can_close: bool,
    previous: Option<usize>,
    next: Option<usize>,

    active: bool,
    image: bool,
    bracket_after: bool,
    index: usize,
    previous_delimiter: Option<usize>,
}

/// 行级解析器。
pub(crate) struct InlineParser<'a> {
    pub tree: &'a mut Tree,
    pub options: &'a ParseOptions,
    /// 块级解析结束时 `oldtip` 是否为表格（供转义 `<br />` 判断使用）。
    pub oldtip_is_table: bool,
    link_ref_defs: HashMap<String, LinkRefDef>,
    link_ref_defs_folded: HashMap<String, LinkRefDef>,
    link_ref_defs_indexed: bool,
    footnotes_defs: Vec<Node>,
    footnotes_defs_indexed: bool,
}

impl<'a> InlineParser<'a> {
    fn new(tree: &'a mut Tree, options: &'a ParseOptions, oldtip_is_table: bool) -> Self {
        Self {
            tree,
            options,
            oldtip_is_table,
            link_ref_defs: HashMap::new(),
            link_ref_defs_folded: HashMap::new(),
            link_ref_defs_indexed: false,
            footnotes_defs: Vec::new(),
            footnotes_defs_indexed: false,
        }
    }

    #[inline]
    fn new_node_raw(&mut self, typ: NodeType) -> Node {
        self.tree.new_node_raw(typ)
    }

    #[inline]
    fn new_text(&mut self, tokens: &[u8]) -> Node {
        let n = self.tree.new_node_raw(NodeType::NodeText);
        n.set_tokens(tokens.to_vec());
        n
    }

    #[inline]
    fn editor_mode(&self) -> bool {
        self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg)
    }

    /// 行级解析主循环，对应 Go 版 Tree.parseInline。
    fn parse_inline(&mut self, block: Node, ctx: &mut InlineContext) {
        while ctx.pos < ctx.tokens_len {
            if self.options.full_width_strikethrough
                && ctx.tokens[ctx.pos..].starts_with(FULL_WIDTH_TILDE)
            {
                self.handle_delim_by_marker(block, ctx, FULL_WIDTH_TILDE, b'~', true);
                continue;
            }

            let token = ctx.tokens[ctx.pos];
            let n: Option<Node> = match token {
                b'\\' => self.parse_backslash(block, ctx),
                b'`' => Some(self.parse_code_span(block, ctx)),
                b'*' | b'_' | b'~' | b'=' | b'#' => {
                    self.handle_delim(block, ctx);
                    None
                }
                b'^' => {
                    if self.options.sup {
                        self.handle_delim(block, ctx);
                        None
                    } else {
                        Some(self.parse_text(ctx))
                    }
                }
                b'\n' => Some(self.parse_newline(block, ctx)),
                b'<' => {
                    let mut n = self.parse_autolink(ctx);
                    if n.is_none() {
                        n = self.parse_auto_email_link(ctx);
                    }
                    if n.is_none() {
                        n = self.parse_file_annotation_ref(ctx);
                    }
                    if n.is_none() {
                        let html = self.parse_inline_html(ctx);
                        if (crate::PROTYLE && self.options.protyle_wysiwyg)
                            && html.typ() == NodeType::NodeInlineHTML
                        {
                            let mut dom = crate::dom::Dom::new();
                            let span =
                                crate::parse::html_text_style::inline_style_tag(html, &mut dom)
                                    .is_some_and(|(node, _)| dom.nodes[node].data == "span");
                            if self.options.inline_code_html || !span {
                                html.set_typ(NodeType::NodeText);
                            }
                        }
                        n = Some(html);
                    }
                    n
                }
                b'[' => Some(self.parse_open_bracket(ctx)),
                b']' => Some(self.parse_close_bracket(ctx)),
                b'&' => Some(self.parse_entity(ctx)),
                b'!' => Some(self.parse_bang(ctx)),
                b'$' => Some(self.parse_inline_math(ctx)),
                b'{' => self.parse_heading_id(block, ctx),
                b'(' => Some(self.parse_block_ref(ctx)),
                _ => Some(self.parse_text(ctx)),
            };

            if let Some(n) = n {
                self.append_parsed(block, n);
            }
        }
        block.clear_tokens();
    }

    /// 把解析结果挂到块节点上。解析函数可能返回一串已经互相链接的节点。
    fn append_parsed(&mut self, block: Node, n: Node) {
        if n.previous().is_none() && n.next().is_none() {
            block.append_child(n);
            return;
        }

        let mut first = n;
        while let Some(prev) = first.previous() {
            first = prev;
        }
        let mut nodes = Vec::new();
        let mut cur = Some(first);
        while let Some(c) = cur {
            nodes.push(c);
            cur = c.next();
        }
        for node in nodes {
            block.append_child(node);
        }
    }

    /// 对应 Go 版 Tree.walkParseInline。
    fn walk_parse_inline(&mut self, node: Node) {
        let mut typ = node.typ();

        if NodeType::NodeSuperBlock == typ {
            if let Some(last) = node.last_child() {
                if NodeType::NodeSuperBlockLayoutMarker == last.typ() {
                    node.set_typ(NodeType::NodeParagraph);
                    let mut tokens = b"{{{".to_vec();
                    tokens.extend_from_slice(last.tokens());
                    node.set_tokens(tokens);
                    if let Some(first) = node.first_child() {
                        first.unlink();
                    }
                    if let Some(last) = node.last_child() {
                        last.unlink();
                    }
                    typ = NodeType::NodeParagraph;
                }
            }
        }

        if NodeType::NodeParagraph == typ
            || NodeType::NodeHeading == typ
            || NodeType::NodeTableCell == typ
        {
            let tokens = node.tokens_vec();
            if NodeType::NodeParagraph == typ
                && !node.data().tokens_initialized
                && node.first_child().is_none()
            {
                let parent_typ = node.parent().map(|p| p.typ());
                if parent_typ != Some(NodeType::NodeTabItem)
                    && (parent_typ != Some(NodeType::NodeListItem)
                        || self.options.vditor_wysiwyg
                        || self.options.vditor_ir
                        || self.options.vditor_sv)
                {
                    // 解析 GFM 表节点后段落内容 Tokens 可能会被置换为空，在此移除空段落节点
                    node.unlink();
                }
                return;
            }

            if NodeType::NodeParagraph == typ {
                let ial = crate::parse::ial::parse_kramdown_ial_in_list_item(&tokens);
                if !ial.is_empty() {
                    if let Some(previous) = node.previous() {
                        // 解析 kramdown 列表或者引述块时可能出现标记符后为空，此时 IAL 应该进行合并
                        for (k, v) in &ial {
                            previous.set_ial_attr(k, &crate::html::unescape_attr_val(v));
                        }
                        let next = node.next();
                        node.unlink();
                        node.data_mut().next = next;
                        if let Some(next) = next {
                            if NodeType::NodeKramdownBlockIAL == next.typ() {
                                let merged = crate::parse::ial::merge_ial_preserving_order(
                                    &crate::parse::ial::tokens2_ial(next.tokens()),
                                    &ial,
                                );
                                next.set_tokens(crate::parse::ial::ial_to_tokens(&merged));
                            }
                        }
                        return;
                    }
                }
            }

            if tokens.len() < 1 {
                return;
            }

            // 外部导入时保留单元格原文，供 NormalizeMarkdownTableCells 恢复块结构。
            if NodeType::NodeTableCell == typ && self.options.preserve_table_cell_source {
                node.data_mut().table_cell_source = String::from_utf8_lossy(&tokens).to_string();
            }

            let mut ctx = InlineContext::new(tokens);
            self.parse_inline(node, &mut ctx);
            self.process_emphasis(None, &mut ctx);
            self.merge_text(node);

            let protyle_auto_link = (crate::PROTYLE && self.options.protyle_wysiwyg)
                && self.options.protyle_wysiwyg_auto_link;
            if (self.options.gfm_auto_link && !self.editor_mode()) || protyle_auto_link {
                self.parse_gfm_auto_email_link(node);
                self.parse_gfm_auto_link(node);
            }

            if self.options.emoji {
                self.emoji(node);
            }
            return;
        } else if NodeType::NodeCodeBlock == typ {
            if node.data().is_fenced_code_block {
                let open_marker = self.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                open_marker.set_tokens(node.data().code_block_open_fence.clone());
                open_marker.data_mut().code_block_fence_len = node.data().code_block_fence_len;
                node.prepend_child(open_marker);

                let info = self.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                info.data_mut().code_block_info = node.data().code_block_info.clone();
                node.append_child(info);

                let code = self.new_node_raw(NodeType::NodeCodeBlockCode);
                code.set_tokens(node.tokens_vec());
                node.append_child(code);

                if node.data().code_block_close_fence.is_empty() {
                    node.data_mut().code_block_close_fence =
                        node.data().code_block_open_fence.clone();
                }
                let close_marker = self.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                close_marker.set_tokens(node.data().code_block_close_fence.clone());
                close_marker.data_mut().code_block_fence_len = node.data().code_block_fence_len;
                node.append_child(close_marker);
            } else {
                let code = self.new_node_raw(NodeType::NodeCodeBlockCode);
                code.set_tokens(node.tokens_vec());
                node.append_child(code);
            }
            node.clear_tokens();
        }

        let mut child = node.first_child();
        while let Some(c) = child {
            let next = c.next();
            self.walk_parse_inline(c);
            child = next;
        }
    }

    /// 合并连续的文本节点，对应 Go 版 Tree.mergeText。
    fn merge_text(&mut self, node: Node) {
        let mut child = node.first_child();
        while let Some(c) = child {
            let mut next = c.next();
            let typ = c.typ();
            if NodeType::NodeText == typ || NodeType::NodeLinkText == typ {
                if let Some(n) = next {
                    if n.typ() == typ {
                        let mut merged = c.tokens_vec();
                        let mut cur = Some(n);
                        while let Some(m) = cur {
                            if m.typ() != typ {
                                break;
                            }
                            merged.extend_from_slice(m.tokens());
                            let after = m.next();
                            m.unlink();
                            cur = after;
                        }
                        c.set_tokens(merged);
                        next = c.next();
                    }
                }
            } else {
                self.merge_text(c);
            }
            child = next;
        }
    }
}

/// 全角波浪线删除线标记。
const FULL_WIDTH_TILDE: &[u8] = "～".as_bytes();

/// 解析整棵语法树的行级子节点，对应 Go 版 Tree.parseInlines。
pub(crate) fn parse_inlines(tree: &mut Tree, options: &ParseOptions, oldtip_is_table: bool) {
    let root = tree.root_raw();
    let mut parser = InlineParser::new(tree, options, oldtip_is_table);
    parser.walk_parse_inline(root);

    if options.kramdown_span_ial {
        parser.parse_kramdown_span_ial();
    }
    if options.protyle_wysiwyg {
        super::html_text_style::normalize_inline_html_text_styles(parser.tree);
        crate::ast::walk_raw(root, &mut |node, entering| {
            if entering && node.typ() == NodeType::NodeInlineHTML {
                node.set_typ(NodeType::NodeText);
            }
            if !entering
                && matches!(
                    node.typ(),
                    NodeType::NodeParagraph | NodeType::NodeHeading | NodeType::NodeTableCell
                )
            {
                parser.merge_text(node);
            }
            crate::ast::WalkStatus::WalkContinue
        });
    }
}
