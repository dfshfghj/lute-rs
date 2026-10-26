// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 块级解析器。结构对齐 Go 版 parse 包：行驱动 + 容器栈 + 末梢节点。

mod block_query_embed;
mod blockquote;
pub(crate) mod callout;
mod code_block;
mod custom_block;
mod footnotes;
mod git_conflict;
mod heading;
mod html_block;
pub(crate) mod html_text_style;
pub(crate) mod ial;
pub(crate) mod inline;
mod link;
mod list;
mod math_block;
mod paragraph;
mod super_block;
mod table;
pub mod table_rich;
mod tabs;
mod thematic_break;
mod tlds;
pub use tlds::add_auto_link_domain_suffix;
mod transform;
mod yaml_front_matter;

pub use transform::{
    nested_inlines2flatted_spans, nested_inlines2flatted_spans_hybrid, text_marks2inlines,
    text_marks2inlines_with_nested_syntax,
};

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::lex::{self, Lexer};
use crate::options::ParseOptions;

/// 块起始函数的返回值。
pub const BLOCK_START_NONE: i32 = 0;
pub const BLOCK_START_CONTAINER: i32 = 1;
pub const BLOCK_START_LEAF: i32 = 2;
pub const BLOCK_START_CLOSED: i32 = 3;

type BlockStart = for<'a> fn(&mut Parser<'a>, Node) -> i32;

/// 块起始函数表，顺序与 Go 版 blockStarts() 一致。
///
/// 尚未移植的块类型暂时返回 0，随 M2 推进逐个补齐。
const BLOCK_STARTS: &[BlockStart] = &[
    git_conflict::git_conflict_start,
    tabs::tabs_start,
    callout::callout_start,
    blockquote::blockquote_start,
    heading::atx_heading_start,
    code_block::fence_code_block_start,
    custom_block::custom_block_start,
    heading::setext_heading_start,
    html_block::html_block_start,
    yaml_front_matter::yaml_front_matter_start,
    thematic_break::thematic_break_start,
    list::list_start,
    math_block::math_block_start,
    code_block::indent_code_block_start,
    footnotes::footnotes_start,
    ial::ial_start,
    block_query_embed::block_query_embed_start,
    super_block::super_block_start,
];

/// 块级解析上下文，对应 Go 版 parse.Context。
#[derive(Default)]
pub(crate) struct Context {
    pub tip: Option<Node>,
    pub oldtip: Option<Node>,
    pub current_line: Vec<u8>,
    pub current_line_len: usize,
    pub offset: usize,
    pub column: usize,
    pub next_nonspace: usize,
    pub next_nonspace_column: usize,
    pub indent: usize,
    pub indented: bool,
    pub blank: bool,
    pub partially_consumed_tab: bool,
    pub all_closed: bool,
    pub last_matched_container: Option<Node>,
    /// 文档级 IAL 节点，对应 Go 版 Context.rootIAL。
    pub root_ial: Option<Node>,
    tabs: Vec<(Node, tabs::TabsParseState)>,
    tabs_item_ial: Option<Node>,
}

impl Context {
    /// 移动 count 个字符位置；columns 为 true 时按列补偿 tab。
    pub fn advance_offset(&mut self, mut count: usize, columns: bool) {
        while 0 < count {
            let c = self.current_line[self.offset];
            if lex::ITEM_TAB == c {
                let chars_to_tab = 4 - (self.column % 4);
                if columns {
                    self.partially_consumed_tab = chars_to_tab > count;
                    let chars_to_advance = if self.partially_consumed_tab {
                        count
                    } else {
                        self.offset += 1;
                        chars_to_tab
                    };
                    self.column += chars_to_advance;
                    count -= chars_to_advance;
                } else {
                    self.partially_consumed_tab = false;
                    self.column += chars_to_tab;
                    self.offset += 1;
                    count -= 1;
                }
            } else {
                self.partially_consumed_tab = false;
                self.offset += 1;
                self.column += 1; // 块起始标记符都是 ASCII
                count -= 1;
            }
        }
    }

    /// 预移动到下一个非空字符位置。
    pub fn advance_next_nonspace(&mut self) {
        self.offset = self.next_nonspace;
        self.column = self.next_nonspace_column;
        self.partially_consumed_tab = false;
    }

    /// 查找下一个非空字符。
    pub fn find_next_nonspace(&mut self) {
        let mut i = self.offset;
        let mut cols = self.column;

        loop {
            let token = self.current_line[i];
            if lex::ITEM_SPACE == token {
                i += 1;
                cols += 1;
            } else if lex::ITEM_TAB == token {
                i += 1;
                cols += 4 - (cols % 4);
            } else {
                break;
            }
        }

        self.blank = lex::ITEM_NEWLINE == self.current_line[i];
        self.next_nonspace = i;
        self.next_nonspace_column = cols;
        self.indent = self.next_nonspace_column - self.column;
        self.indented = 4 <= self.indent;
    }
}

/// 解析器。持有树的独占可变引用，避免 Context 与 Tree 的借用冲突。
pub(crate) struct Parser<'a> {
    pub tree: &'a mut Tree,
    pub ctx: Context,
    lexer: Lexer,
    pub options: ParseOptions,
}

/// 将 markdown 解析为语法树，对应 Go 版 parse.Parse。
pub fn parse(name: &str, markdown: &[u8], options: ParseOptions) -> Tree {
    parse_with_task_list_probe(name, markdown, options, false)
}

/// Go parse(..., taskListProbe): a plain paragraph needs no second inline pass.
fn parse_with_task_list_probe(
    name: &str,
    markdown: &[u8],
    options: ParseOptions,
    task_list_probe: bool,
) -> Tree {
    let mut tree = Tree::new_with_name(name);
    let mut parser = Parser::new(&mut tree, markdown, options.clone());
    parser.parse_blocks();
    if task_list_probe
        && parser
            .tree
            .root_raw()
            .first_child()
            .is_some_and(|n| n.typ() == NodeType::NodeParagraph && !n.tokens().is_empty())
    {
        return tree;
    }
    let oldtip_is_table = parser
        .ctx
        .oldtip
        .map(|n| NodeType::NodeTable == n.typ())
        .unwrap_or(false);
    inline::parse_inlines(parser.tree, &options, oldtip_is_table);
    parser.final_parse_block_ial();
    parser.final_parse_tabs();
    table_rich::final_parse_table_cell_rich(parser.tree, &options);
    tree
}

/// 解析行级片段，对应 Go 版 parse.Inline。
pub fn inline(name: &str, markdown: &[u8], options: ParseOptions) -> Tree {
    let mut tree = Tree::new_with_name(name);
    let p = tree.new_node_raw(NodeType::NodeParagraph);
    p.set_tokens(markdown.to_vec());
    tree.root_raw().append_child(p);
    inline::parse_inlines(&mut tree, &options, false);
    tree
}

/// 只做块级解析，对应 Go 版 parse.Block。
pub fn parse_block(name: &str, markdown: &[u8], options: ParseOptions) -> Tree {
    let mut tree = Tree::new_with_name(name);
    let mut parser = Parser::new(&mut tree, markdown, options);
    parser.parse_blocks();
    parser.final_parse_block_ial();
    parser.final_parse_tabs();
    tree
}

impl<'a> Parser<'a> {
    pub fn new(tree: &'a mut Tree, markdown: &[u8], options: ParseOptions) -> Self {
        let root = tree.root_raw();
        Self {
            tree,
            ctx: Context {
                tip: Some(root),
                ..Context::default()
            },
            lexer: Lexer::new(markdown),
            options,
        }
    }

    fn parse_blocks(&mut self) {
        self.ctx.tip = Some(self.tree.root_raw());
        while let Some(mut line) = self.lexer.next_line() {
            if (self.options.vditor_wysiwyg
                || self.options.vditor_ir
                || self.options.vditor_sv
                || (crate::PROTYLE && self.options.protyle_wysiwyg))
                && line != crate::editor::CARET_NEWLINE.as_bytes()
                && self
                    .ctx
                    .tip
                    .is_some_and(|tip| tip.parent_is(&[NodeType::NodeListItem]))
                && line.starts_with(crate::editor::CARET.as_bytes())
            {
                let tip = self.ctx.tip.expect("tip");
                if tip.typ() == NodeType::NodeListItem {
                    let text = self.tree.new_node_raw(NodeType::NodeText);
                    text.set_tokens(line);
                    tip.append_child(text);
                    break;
                }
                let tokens = tip.tokens();
                let mut tokens = tokens.strip_suffix(b"\n").unwrap_or(tokens).to_vec();
                tokens.extend_from_slice(crate::editor::CARET_NEWLINE.as_bytes());
                tip.set_tokens(tokens);
                line.drain(..crate::editor::CARET.len());
            }
            self.incorporate_line(line);
        }
        while self.ctx.tip.is_some() {
            let tip = self.ctx.tip.unwrap();
            self.finalize(tip);
        }
    }

    /// 处理一行并把生成的块级节点挂到树上。
    fn incorporate_line(&mut self, line: Vec<u8>) {
        let item_ial = self.ctx.tabs_item_ial.take();
        self.ctx.oldtip = self.ctx.tip;
        self.ctx.offset = 0;
        self.ctx.column = 0;
        self.ctx.blank = false;
        self.ctx.partially_consumed_tab = false;
        self.ctx.current_line = line;
        self.ctx.current_line_len = self.ctx.current_line.len();

        let mut all_matched = true;
        let mut container = self.tree.root_raw();
        loop {
            let last_child = match container.last_child() {
                Some(child) if !child.close() => child,
                _ => break,
            };
            container = last_child;
            self.ctx.find_next_nonspace();

            match self.continue_block(container) {
                0 => {}
                1 => all_matched = false,
                2 => return,
                3 => {
                    // 匹配超级块闭合，处理下一行
                    self.close_super_block_children();
                    let tip = self.ctx.tip.expect("tip");
                    if NodeType::NodeSuperBlock != tip.typ() {
                        let sb = tip.parent().expect("super block");
                        sb.data_mut().close = true;
                        let marker = self.tree.new_node_raw(NodeType::NodeSuperBlockCloseMarker);
                        sb.append_child(marker);
                        self.ctx.tip = sb.parent();
                        self.ctx.last_matched_container = Some(sb);
                    } else {
                        let marker = self.tree.new_node_raw(NodeType::NodeSuperBlockCloseMarker);
                        tip.append_child(marker);
                        tip.data_mut().close = true;
                        self.ctx.tip = tip.parent();
                        self.ctx.last_matched_container = self.ctx.tip;
                    }
                    return;
                }
                _ => unreachable!(),
            }

            if !all_matched {
                container = container.parent().expect("container must have parent");
                break;
            }
        }

        self.ctx.all_closed = Some(container) == self.ctx.oldtip;
        self.ctx.last_matched_container = Some(container);
        if item_ial == Some(container)
            && (crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial)
            && !self.ctx.indented
        {
            if let Some(ial) = self.parse_kramdown_block_ial() {
                if !ial::is_doc_ial2(&ial) {
                    container.set_id(
                        ial::ial2_map_un_esc(&ial)
                            .get("id")
                            .cloned()
                            .unwrap_or_default(),
                    );
                    container.data_mut().kramdown_ial = ial;
                    return;
                }
            }
        }

        let mut matched_leaf =
            NodeType::NodeParagraph != container.typ() && container.accept_lines();

        while !matched_leaf {
            self.ctx.find_next_nonspace();

            // 只有潜在的块标记符才继续迭代，其余按段落文本处理
            let maybe_marker = self.ctx.current_line[self.ctx.next_nonspace];
            if !self.ctx.indented
                && lex::ITEM_HYPHEN != maybe_marker
                && lex::ITEM_ASTERISK != maybe_marker
                && lex::ITEM_PLUS != maybe_marker
                && !lex::is_digit(maybe_marker)
                && lex::ITEM_BACKTICK != maybe_marker
                && lex::ITEM_TILDE != maybe_marker
                && lex::ITEM_SEMICOLON != maybe_marker
                && lex::ITEM_COLON != maybe_marker
                && b'@' != maybe_marker
                && lex::ITEM_CROSSHATCH != maybe_marker
                && lex::ITEM_GREATER != maybe_marker
                && lex::ITEM_LESS != maybe_marker
                && lex::ITEM_UNDERSCORE != maybe_marker
                && lex::ITEM_EQUAL != maybe_marker
                && lex::ITEM_DOLLAR != maybe_marker
                && lex::ITEM_OPEN_BRACKET != maybe_marker
                && lex::ITEM_OPEN_BRACE != maybe_marker
                && lex::ITEM_CLOSE_BRACE != maybe_marker
                && lex::ITEM_BANG != maybe_marker
                && "！".as_bytes()[0] != maybe_marker
                && crate::editor::CARET.as_bytes()[0] != maybe_marker
            {
                self.ctx.advance_next_nonspace();
                break;
            }

            let mut i = 0;
            while i < BLOCK_STARTS.len() {
                let res = BLOCK_STARTS[i](self, container);
                if BLOCK_START_CLOSED == res {
                    return;
                }
                if BLOCK_START_CONTAINER == res {
                    container = self.ctx.tip.unwrap();
                    break;
                }
                if BLOCK_START_LEAF == res {
                    container = self.ctx.tip.unwrap();
                    matched_leaf = true;
                    break;
                }
                i += 1;
            }

            if i == BLOCK_STARTS.len() {
                self.ctx.advance_next_nonspace();
                break;
            }
        }

        if !self.ctx.all_closed
            && !self.ctx.blank
            && self.ctx.tip.map(|n| n.typ()) == Some(NodeType::NodeParagraph)
        {
            self.add_line();
        } else {
            self.close_unmatched_blocks();

            if self.ctx.blank {
                if let Some(last) = container.last_child() {
                    last.data_mut().last_line_blank = true;
                }
            }

            let typ = container.typ();
            let is_fenced = NodeType::NodeCodeBlock == typ && container.data().is_fenced_code_block;

            let last_line_blank = self.ctx.blank
                && !(typ == NodeType::NodeFootnotesDef
                    || typ == NodeType::NodeBlockquote
                    || typ == NodeType::NodeCallout
                    || (typ == NodeType::NodeCodeBlock && is_fenced)
                    || typ == NodeType::NodeCustomBlock
                    || typ == NodeType::NodeMathBlock
                    || typ == NodeType::NodeGitConflict
                    || (typ == NodeType::NodeListItem && container.first_child().is_none()));
            let mut cont = Some(container);
            while let Some(c) = cont {
                c.data_mut().last_line_blank = last_line_blank;
                cont = c.parent();
            }

            if container.accept_lines() {
                self.add_line();
                if NodeType::NodeHTMLBlock == typ {
                    // HTML 块（类型 1-5）需要检查是否满足闭合条件
                    let html_type = container.data().html_block_type;
                    if (1..=5).contains(&html_type) {
                        let tokens = self.ctx.current_line[self.ctx.offset..].to_vec();
                        if self.is_html_block_close(&tokens, html_type) {
                            self.finalize(container);
                        }
                    }
                } else if NodeType::NodeMathBlock == typ {
                    // $$foo$$ 形式需要判断右边结尾的闭合标记符
                    let tokens = container.tokens_vec();
                    if 3 <= tokens.len() {
                        let first = find_sub(&tokens, b"$$");
                        let last = rfind_sub(&tokens, b"$$");
                        if let (Some(first), Some(last)) = (first, last) {
                            if last != first {
                                let tails = &tokens[last..];
                                if tails.starts_with(b"$$")
                                    && tails.ends_with(b"\n")
                                    && lex::trim_whitespace(tails) == b"$$"
                                {
                                    self.finalize(container);
                                }
                            }
                        }
                    }
                }
            } else if self.ctx.offset < self.ctx.current_line_len && !self.ctx.blank {
                self.add_child(NodeType::NodeParagraph);
                self.ctx.advance_next_nonspace();
                self.add_line();
            }
        }
    }

    /// 在当前末梢节点上追加本行剩余内容。
    fn add_line(&mut self) {
        if self.ctx.partially_consumed_tab {
            self.ctx.offset += 1;
            let chars_to_tab = 4 - (self.ctx.column % 4);
            let spaces = vec![b' '; chars_to_tab];
            if let Some(tip) = self.ctx.tip {
                tip.append_tokens(&spaces);
            }
        }

        let start_with_space = 1 < self.ctx.current_line_len
            && (b' ' == self.ctx.current_line[0] || b'\t' == self.ctx.current_line[0]);
        let doc_child_para = self
            .ctx
            .tip
            .and_then(|n| n.parent())
            .map(|p| NodeType::NodeDocument == p.typ())
            .unwrap_or(false);

        let tokens = if self.options.paragraph_beginning_space && start_with_space && doc_child_para
        {
            self.ctx.current_line.clone()
        } else {
            self.ctx.current_line[self.ctx.offset..].to_vec()
        };
        if let Some(tip) = self.ctx.tip {
            tip.append_tokens(&tokens);
        }
    }

    /// 最终化所有未匹配的块节点。
    fn close_unmatched_blocks(&mut self) {
        if !self.ctx.all_closed {
            while self.ctx.oldtip != self.ctx.last_matched_container {
                let oldtip = self.ctx.oldtip.expect("oldtip");
                let parent = oldtip.parent();
                self.finalize(oldtip);
                self.ctx.oldtip = parent;
            }
            self.ctx.all_closed = true;
        }
    }

    /// 执行块的最终化处理，并把末梢置为块的父节点。
    pub fn finalize(&mut self, block: Node) {
        let parent = block.parent();
        block.data_mut().close = true;

        match block.typ() {
            NodeType::NodeCodeBlock => self.code_block_finalize(block),
            NodeType::NodeHTMLBlock
            | NodeType::NodeIFrame
            | NodeType::NodeVideo
            | NodeType::NodeAudio
            | NodeType::NodeWidget => self.html_block_finalize(block),
            NodeType::NodeParagraph => {
                if self.paragraph_finalize(block) {
                    return;
                }
            }
            NodeType::NodeList => self.list_finalize(block),
            NodeType::NodeYamlFrontMatter => self.yaml_front_matter_finalize(block),
            NodeType::NodeMathBlock => self.math_block_finalize(block),
            NodeType::NodeGitConflict => self.git_conflict_finalize(block),
            NodeType::NodeCustomBlock => self.custom_block_finalize(block),
            NodeType::NodeSuperBlock => self.super_block_finalize(block),
            NodeType::NodeCallout => self.callout_finalize(block),
            NodeType::NodeTabs | NodeType::NodeTabItem => self.tabs_finalize(block),
            NodeType::NodeBlockquote => {}
            _ => {}
        }

        self.ctx.tip = parent;
    }

    /// 构造一个节点并添加到末梢节点上。
    pub fn add_child(&mut self, typ: NodeType) -> Node {
        let tip = self.ctx.tip.expect("tip");
        if tip.typ() == NodeType::NodeTabs && !tip.can_contain(typ) {
            let item = self.tree.new_node_raw(NodeType::NodeTabItem);
            tip.append_child(item);
            self.ctx.tip = Some(item);
        }
        while !self.ctx.tip.expect("tip").can_contain(typ) {
            let tip = self.ctx.tip.expect("tip");
            self.finalize(tip);
        }

        let ret = self.tree.new_node_raw(typ);
        self.ctx.tip.expect("tip").append_child(ret);
        self.ctx.tip = Some(ret);
        ret
    }

    /// 构造一个闭合的标记符节点并添加到末梢节点上。
    pub fn add_child_marker(&mut self, typ: NodeType, tokens: &[u8]) -> Node {
        let ret = self.tree.new_node_raw(typ);
        ret.set_tokens(tokens.to_vec());
        ret.data_mut().close = true;
        self.ctx.tip.expect("tip").append_child(ret);
        ret
    }

    /// 判断节点是否可以继续处理。0 可以，1 不可以，2 已闭合。
    fn continue_block(&mut self, n: Node) -> i32 {
        match n.typ() {
            NodeType::NodeCodeBlock => self.code_block_continue(n),
            NodeType::NodeHTMLBlock => self.html_block_continue(n),
            NodeType::NodeParagraph => paragraph::paragraph_continue(&self.ctx),
            NodeType::NodeListItem => self.list_item_continue(n),
            NodeType::NodeBlockquote => self.blockquote_continue(n),
            NodeType::NodeYamlFrontMatter => self.yaml_front_matter_continue(n),
            NodeType::NodeMathBlock => self.math_block_continue(n),
            NodeType::NodeGitConflict => self.git_conflict_continue(n),
            NodeType::NodeCustomBlock => self.custom_block_continue(n),
            NodeType::NodeFootnotesDef => self.footnotes_continue(n),
            NodeType::NodeSuperBlock => self.super_block_continue(n),
            NodeType::NodeCallout => self.callout_continue(n),
            NodeType::NodeTabs => self.tabs_continue(n),
            NodeType::NodeHeading
            | NodeType::NodeThematicBreak
            | NodeType::NodeKramdownBlockIAL
            | NodeType::NodeLinkRefDefBlock
            | NodeType::NodeBlockQueryEmbed
            | NodeType::NodeIFrame
            | NodeType::NodeVideo
            | NodeType::NodeAudio
            | NodeType::NodeWidget
            | NodeType::NodeAttributeView => 1,
            _ => 0,
        }
    }
}

fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn rfind_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(markdown: &str) -> Vec<NodeType> {
        let tree = parse("", markdown.as_bytes(), ParseOptions::new());
        tree.root_raw()
            .children()
            .map(|n| n.typ())
            .collect::<Vec<_>>()
    }

    #[test]
    fn paragraph_is_one_block() {
        assert_eq!(vec![NodeType::NodeParagraph], types("foo\nbar"));
    }

    #[test]
    fn atx_heading() {
        let tree = parse("", b"## foo ##\n", ParseOptions::new());
        let heading = tree.root_raw().first_child().unwrap();
        assert_eq!(NodeType::NodeHeading, heading.typ());
        assert_eq!(2, heading.data().heading_level);
        assert_eq!("foo", heading.text());
        assert_eq!(
            NodeType::NodeHeadingC8hMarker,
            heading.first_child().unwrap().typ()
        );
    }

    #[test]
    fn thematic_break() {
        assert_eq!(vec![NodeType::NodeThematicBreak], types("***\n"));
        assert_eq!(vec![NodeType::NodeThematicBreak], types(" - - -\n"));
    }

    #[test]
    fn fenced_code_block() {
        let tree = parse("", b"```go\nfmt.Println(1)\n```\n", ParseOptions::new());
        let cb = tree.root_raw().first_child().unwrap();
        assert_eq!(NodeType::NodeCodeBlock, cb.typ());
        assert!(cb.data().is_fenced_code_block);
        let code = cb.child_by_type(NodeType::NodeCodeBlockCode).unwrap();
        assert_eq!("fmt.Println(1)\n", code.tokens_str());
    }

    #[test]
    fn blockquote_nests_paragraph() {
        let tree = parse("", b"> foo\n", ParseOptions::new());
        let bq = tree.root_raw().first_child().unwrap();
        assert_eq!(NodeType::NodeBlockquote, bq.typ());
        assert_eq!(
            NodeType::NodeBlockquoteMarker,
            bq.first_child().unwrap().typ()
        );
        let p = bq.last_child().unwrap();
        assert_eq!(NodeType::NodeParagraph, p.typ());
        assert_eq!("foo", p.text());
    }

    #[test]
    fn list_items() {
        let tree = parse("", b"- a\n- b\n", ParseOptions::new());
        let list = tree.root_raw().first_child().unwrap();
        assert_eq!(NodeType::NodeList, list.typ());
        assert_eq!(2, list.children().count());
        assert!(list.data().list_data.as_ref().unwrap().tight);
    }

    #[test]
    fn loose_list() {
        let tree = parse("", b"- a\n\n- b\n", ParseOptions::new());
        let list = tree.root_raw().first_child().unwrap();
        assert!(!list.data().list_data.as_ref().unwrap().tight);
    }
}
