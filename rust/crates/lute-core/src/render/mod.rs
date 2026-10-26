// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 渲染器基础设施，对应 Go 版 render 包的 BaseRenderer。

pub mod format;
pub mod html;
pub mod json;
pub mod mindmap;
pub mod protyle;
pub mod protyle_preview;
pub mod sanitizer;
pub mod space;
mod table;
pub mod table_rich;
pub mod term_typographer;
pub mod vditor;
pub mod vditor_ir;
pub mod vditor_sv;

pub use html::HtmlRenderer;

use crate::ast::{NodeType, RawNode as Node, WalkStatus};
use crate::html as html_util;
use crate::options::RenderOptions;

/// 渲染器接口。
pub trait Renderer {
    /// 渲染输出。
    fn render(&mut self) -> Vec<u8>;
}

/// 用户自定义渲染器函数。返回 None 表示走默认渲染。
pub type ExtRendererFunc =
    std::rc::Rc<dyn Fn(crate::ast::Node, bool) -> Option<(String, WalkStatus)>>;

/// 渲染器基类，对应 Go 版 BaseRenderer。
pub struct BaseRenderer {
    pub options: RenderOptions,
    pub out: Vec<u8>,
    pub last_out: u8,
    /// 标签嵌套计数器，用于判断不可能出现标签嵌套的情况。
    pub disable_tags: i32,
    /// 用户自定义渲染器，命中时直接输出并接管遍历状态。
    pub ext_renderer: Option<ExtRendererFunc>,
    /// 被引用的脚注定义集，按引用顺序排列。
    pub(crate) footnotes_defs: Vec<Node>,
    /// 是否正在渲染脚注定义。
    pub rendering_footnotes: bool,
    /// 对应 Go 版 BaseRenderer.tableCellRichInline：HTML 表格降级时单元格富文本
    /// 片段需要转义管道符与换行，Markdown 输出才使用该形式。
    pub table_cell_rich_inline: bool,
}

impl BaseRenderer {
    pub fn new(options: RenderOptions) -> Self {
        Self {
            options,
            out: Vec::with_capacity(4096),
            last_out: b'\n',
            disable_tags: 0,
            footnotes_defs: Vec::new(),
            rendering_footnotes: false,
            table_cell_rich_inline: false,
            ext_renderer: None,
        }
    }

    /// 调用用户自定义渲染器；未注册或未命中返回 None。
    #[inline]
    pub(crate) fn ext_render(&self, n: Node, entering: bool) -> Option<(String, WalkStatus)> {
        self.ext_renderer
            .as_ref()
            .and_then(|f| f(crate::ast::Node::from_raw(n), entering))
    }

    /// 输出一个字节。
    #[inline]
    pub fn write_byte(&mut self, c: u8) {
        self.out.push(c);
        self.last_out = c;
    }

    /// 输出字节数组。
    #[inline]
    pub fn write(&mut self, content: &[u8]) {
        if !content.is_empty() {
            self.out.extend_from_slice(content);
            self.last_out = content[content.len() - 1];
        }
    }

    /// 输出字符串。
    #[inline]
    pub fn write_str(&mut self, content: &str) {
        self.write(content.as_bytes());
    }

    /// 在最新内容不是换行符时输出一个换行符。
    pub fn newline(&mut self) {
        if b'\n' != self.last_out {
            self.out.push(b'\n');
            self.last_out = b'\n';
        }
    }

    /// 输出标签，对应 Go 版 BaseRenderer.Tag。
    pub fn tag(&mut self, name: &str, attrs: &[(String, String)], selfclosing: bool) {
        if self.disable_tags > 0 {
            return;
        }

        self.write_str("<");
        self.write_str(name);
        for (k, v) in attrs {
            self.write_str(" ");
            self.write_str(k);
            self.write_str("=\"");
            self.write_str(v);
            self.write_str("\"");
        }
        if selfclosing {
            self.write_str(" /");
        }
        self.write_str(">");
    }

    /// 节点上除 id 以外的属性串，对应 Go 版 NodeAttrsStr。
    pub(crate) fn node_attrs_str(&self, node: Node) -> String {
        let mut ret = String::new();
        for (k, v) in node.ial_raw() {
            if "id" == k {
                continue;
            }
            ret.push_str(&k);
            ret.push_str("=\"");
            ret.push_str(&v);
            ret.push_str("\" ");
        }
        if !ret.is_empty() {
            ret.pop();
        }
        ret
    }

    /// 中英文之间自动空格，对应 Go 版 BaseRenderer.Space。
    pub fn space(&self, tokens: &[u8]) -> Vec<u8> {
        space::space(tokens)
    }

    /// 修正术语拼写，对应 Go 版 BaseRenderer.FixTermTypo。
    pub fn fix_term_typo(&self, tokens: &mut [u8]) {
        term_typographer::fix_term_typo(tokens, &self.options.terms);
    }

    /// 自动空格辅助：判断节点首字符前是否需要空格，对应 Go 版 TextAutoSpacePrevious。
    pub(crate) fn text_auto_space_previous(&mut self, node: Node) {
        if !(crate::CAP_AUTO_SPACE && self.options.auto_space) {
            return;
        }
        let tokens = auto_space_tokens(node);
        if tokens.is_empty() {
            return;
        }
        if let Some(previous) = node.previous() {
            if NodeType::NodeText == previous.typ() {
                let prev_last = decode_last_rune(previous.tokens());
                let first = decode_first_rune(&tokens);
                if space::allow_space(prev_last, first) {
                    self.write_byte(b' ');
                }
            }
        }
    }

    /// 自动空格辅助：判断节点末字符后是否需要空格，对应 Go 版 TextAutoSpaceNext。
    pub(crate) fn text_auto_space_next(&mut self, node: Node) {
        if !(crate::CAP_AUTO_SPACE && self.options.auto_space) {
            return;
        }
        let tokens = auto_space_tokens(node);
        if tokens.is_empty() {
            return;
        }
        if let Some(mut next) = node.next() {
            if NodeType::NodeText == next.typ() {
                let next_first = decode_first_rune(next.tokens());
                let last = decode_last_rune(&tokens);
                if space::allow_space(last, next_first) {
                    self.write_byte(b' ');
                }
            } else if NodeType::NodeKramdownSpanIAL == next.typ() {
                // 优化排版未处理样式文本 https://github.com/siyuan-note/siyuan/issues/6305
                if let Some(next2) = next.next() {
                    next = next2;
                    if NodeType::NodeText == next.typ() {
                        let next_first = decode_first_rune(next.tokens());
                        let last = decode_last_rune(&tokens);
                        if space::allow_space(last, next_first) {
                            let mut new_tokens = Vec::with_capacity(next.tokens().len() + 1);
                            new_tokens.push(b' ');
                            new_tokens.extend_from_slice(next.tokens());
                            next.set_tokens(new_tokens);
                        }
                    }
                }
            }
        }
    }

    /// 链接文本前的自动空格，对应 Go 版 LinkTextAutoSpacePrevious。
    pub(crate) fn link_text_auto_space_previous(&mut self, node: Node) {
        if !(crate::CAP_AUTO_SPACE && self.options.auto_space) {
            return;
        }
        if let Some(text) = node.child_by_type(NodeType::NodeLinkText) {
            let text_tokens = text.tokens();
            if !text_tokens.is_empty() {
                if let Some(previous) = node.previous() {
                    if NodeType::NodeText == previous.typ() {
                        let prev_last = decode_last_rune(previous.tokens());
                        let first = decode_first_rune(text_tokens);
                        if space::allow_space(prev_last, first) {
                            self.write_byte(b' ');
                        }
                    }
                }
            }
        }
    }

    /// 链接文本后的自动空格，对应 Go 版 LinkTextAutoSpaceNext。
    pub(crate) fn link_text_auto_space_next(&mut self, node: Node) {
        if !(crate::CAP_AUTO_SPACE && self.options.auto_space) {
            return;
        }
        if let Some(text) = node.child_by_type(NodeType::NodeLinkText) {
            let text_tokens = text.tokens();
            if !text_tokens.is_empty() {
                if let Some(next) = node.next() {
                    if NodeType::NodeText == next.typ() {
                        let next_first = decode_first_rune(next.tokens());
                        let last = decode_last_rune(text_tokens);
                        if space::allow_space(last, next_first) {
                            self.write_byte(b' ');
                        }
                    }
                }
            }
        }
    }

    /// 链接、图片路径处理，对应 Go 版 LinkPath。
    pub fn link_path(&self, dest: &[u8]) -> Vec<u8> {
        let dest = self.relative_path(dest);
        self.prefix_path(&dest)
    }

    fn prefix_path(&self, dest: &[u8]) -> Vec<u8> {
        if self.options.link_prefix.is_empty() {
            return dest.to_vec();
        }
        let mut ret = self.options.link_prefix.as_bytes().to_vec();
        ret.extend_from_slice(dest);
        ret
    }

    fn relative_path(&self, dest: &[u8]) -> Vec<u8> {
        if self.options.link_base.is_empty() {
            return dest.to_vec();
        }
        if !is_relative_path(dest) {
            return dest.to_vec();
        }

        if !dest.starts_with(b"assets/")
            && (self
                .options
                .link_base
                .starts_with("https://assets.b3logfile.com/siyuan/")
                || self
                    .options
                    .link_base
                    .starts_with("https://assets.liuyun.io/siyuan/"))
        {
            return dest.to_vec();
        }

        let dest = replace_bytes(dest, b"%5C", b"\\");
        let mut link_base = self.options.link_base.as_bytes().to_vec();
        if !link_base.ends_with(b"/") {
            link_base.push(b'/');
        }
        let mut ret = link_base.clone();
        ret.extend_from_slice(&dest);
        if link_base == ret {
            return Vec::new();
        }
        ret
    }
}

/// 取自动空格判断用的文本，对应 Go 版 TextAutoSpace* 中的 tokens 获取逻辑。
fn auto_space_tokens(node: Node) -> Vec<u8> {
    if NodeType::NodeTextMark == node.typ() {
        return node.data().text_mark_text_content.as_bytes().to_vec();
    }
    node.child_by_type(NodeType::NodeText)
        .map(|t| t.tokens().to_vec())
        .unwrap_or_default()
}

/// 解码最后一个字符，空输入返回 U+FFFD，对应 Go 版 utf8.DecodeLastRuneInString。
fn decode_last_rune(tokens: &[u8]) -> char {
    String::from_utf8_lossy(tokens)
        .chars()
        .next_back()
        .unwrap_or('\u{FFFD}')
}

/// 解码第一个字符，空输入返回 U+FFFD，对应 Go 版 utf8.DecodeRuneInString。
fn decode_first_rune(tokens: &[u8]) -> char {
    String::from_utf8_lossy(tokens)
        .chars()
        .next()
        .unwrap_or('\u{FFFD}')
}

fn is_relative_path(dest: &[u8]) -> bool {
    if dest.is_empty() {
        return true;
    }
    if b'/' == dest[0] {
        return false;
    }
    let lower = String::from_utf8_lossy(dest).to_lowercase();
    if lower.starts_with("data:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("sms:")
    {
        return false;
    }
    !dest.windows(2).any(|w| w == b":/")
        && !dest.windows(2).any(|w| w == b":\\")
        && !dest.windows(4).any(|w| w == b":%5C")
}

/// 对应 Go 版 render.HeadingIDRaw。
pub(crate) fn heading_id_raw(heading: Node) -> String {
    match heading.child_by_type(NodeType::NodeHeadingID) {
        Some(n) => n.tokens_str(),
        None => String::new(),
    }
}

/// 对应 Go 版 render.HeadingID。
pub(crate) fn heading_id(heading: Node) -> String {
    let d = heading.data();
    if d.heading_normalized_id.is_empty() {
        heading_id0(heading);
    }
    heading.data().heading_normalized_id.clone()
}

fn heading_id0(heading: Node) {
    let mut root = heading;
    while NodeType::NodeDocument != root.typ() {
        match root.parent() {
            Some(p) => root = p,
            None => break,
        }
    }

    let mut headings = Vec::new();
    crate::ast::walk_raw(root, &mut |n, entering| {
        if entering && NodeType::NodeHeading == n.typ() {
            headings.push(n);
        }
        WalkStatus::WalkContinue
    });

    let mut id_occurs: Vec<String> = Vec::new();
    for n in headings {
        let mut id = normalize_heading_id(n);
        while id_occurs.contains(&id) {
            id.push('-');
        }
        n.data_mut().heading_normalized_id = id.clone();
        id_occurs.push(id);
    }
}

fn normalize_heading_id(heading: Node) -> String {
    let mut id = match heading.child_by_type(NodeType::NodeHeadingID) {
        Some(n) => n.tokens_str(),
        None => String::new(),
    };
    if id.is_empty() {
        id = heading.text();
    }

    let id = id.trim_start_matches('#').replace(crate::editor::CARET, "");
    let mut buf = String::with_capacity(id.len());
    for r in id.chars() {
        if is_letter_or_digit(r) {
            buf.push(r);
        } else {
            buf.push('-');
        }
    }
    buf
}

pub(crate) fn is_letter_or_digit(c: char) -> bool {
    let cp = c as u32;
    crate::char_class::LETTER_OR_DIGIT
        .binary_search_by(|&(start, end)| {
            if cp < start {
                std::cmp::Ordering::Greater
            } else if cp > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// 对应 Go 版 bytes.EqualFold。
pub(crate) fn eq_fold(a: &[u8], b: &[u8]) -> bool {
    let a = String::from_utf8_lossy(a);
    let b = String::from_utf8_lossy(b);
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) => {
                if x.to_lowercase().ne(y.to_lowercase()) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

pub(crate) fn replace_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
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

/// 对应 Go 版 parse.ContainTextMark。
pub(crate) fn contain_text_mark(node: Node, types: &[&str]) -> bool {
    node.data()
        .text_mark_type
        .split(' ')
        .any(|t| types.contains(&t))
}

/// 对应 Go 版 util.IsDocIAL。
pub(crate) fn is_doc_ial(tokens: &[u8]) -> bool {
    tokens
        .windows(b"type=\"doc\"".len())
        .any(|w| w == b"type=\"doc\"")
}

/// 对应 Go 版 render.calloutTitle。
pub(crate) fn callout_title(node: Node) -> String {
    use crate::parse::callout::{
        get_callout_icon, get_callout_title, is_built_in_callout_type, is_valid_callout_image_src,
    };
    let d = node.data();
    if is_built_in_callout_type(&d.callout_type)
        && ((!d.callout_title_explicit && d.callout_title.is_empty())
            || d.callout_title == get_callout_title(&d.callout_type))
        && d.callout_icon == get_callout_icon(&d.callout_type)
    {
        return String::new();
    }

    let mut icon = d.callout_icon.clone();
    if 1 == d.callout_icon_type {
        if is_valid_callout_image_src(&icon) {
            let dest = crate::html::encode_destination(icon.as_bytes());
            icon = format!("![callout-icon](<{}>)", String::from_utf8_lossy(&dest));
        } else {
            icon.clear();
        }
    }
    format!("{icon} {}", d.callout_title).trim().to_string()
}

/// 对应 Go 版 util.IsEmptyStr。
pub(crate) fn is_empty_str(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace() || c == '\u{200B}')
}

/// 收集文档中所有脚注定义，对应 Go 版 indexFootnotesDefs。
pub(crate) fn collect_footnotes_defs(root: Node) -> Vec<Node> {
    let mut defs = Vec::new();
    crate::ast::walk_raw(root, &mut |n, entering| {
        if entering && NodeType::NodeFootnotesDef == n.typ() {
            defs.push(n);
        }
        WalkStatus::WalkContinue
    });
    defs
}

/// 返回最深的最后一个子节点，对应 Go 版 LastDeepestChild。
pub(crate) fn last_deepest_child(node: Node) -> Node {
    let mut n = node;
    while let Some(last) = n.last_child() {
        n = last;
    }
    n
}

/// 转义 HTML 后写入，对应 Go 版 html.EscapeHTML。
pub(crate) fn escape_html_bytes(tokens: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len() + 16);
    for &b in tokens {
        match b {
            b'&' => ret.extend_from_slice(b"&amp;"),
            b'<' => ret.extend_from_slice(b"&lt;"),
            b'>' => ret.extend_from_slice(b"&gt;"),
            b'"' => ret.extend_from_slice(b"&quot;"),
            _ => ret.push(b),
        }
    }
    ret
}

/// 保持与 html 模块一致的字符串转义入口。
pub(crate) fn escape_html_str(s: &str) -> String {
    html_util::escape_html(s)
}

/// 目录项，对应 Go 版 render.Heading。
pub struct Heading {
    pub id: String,
    pub content: String,
    pub level: i32,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// 目录树：`all` 保存全部标题，`top` 是顶层标题下标。
pub struct HeadingTree {
    pub all: Vec<Heading>,
    pub top: Vec<usize>,
}

/// 收集文档一级标题构成的目录树，对应 Go 版 BaseRenderer.headings。
pub(crate) fn headings(root: Node, options: &RenderOptions) -> HeadingTree {
    let mut all: Vec<Heading> = Vec::new();
    let mut top: Vec<usize> = Vec::new();
    let mut tip: Option<usize> = None;
    for heading in root.children_by_type(NodeType::NodeHeading) {
        let mut id = heading_id(heading);
        if options.vditor_wysiwyg {
            id = format!("wysiwyg-{id}");
        } else if options.vditor_ir {
            id = format!("ir-{id}");
        }
        if crate::CAP_KRAMDOWN_BLOCK_IAL && options.kramdown_block_ial {
            for (k, v) in heading.ial_raw() {
                if "id" == k {
                    id = v;
                    break;
                }
            }
        }

        let idx = all.len();
        all.push(Heading {
            id,
            content: heading_text(heading),
            level: heading.data().heading_level,
            parent: None,
            children: Vec::new(),
        });

        match tip {
            None => top.push(idx),
            Some(tip_idx) => {
                if all[tip_idx].level < all[idx].level {
                    all[tip_idx].children.push(idx);
                    all[idx].parent = Some(tip_idx);
                } else {
                    match parent_tip(&all, idx, tip_idx) {
                        None => top.push(idx),
                        Some(parent) => {
                            all[parent].children.push(idx);
                            all[idx].parent = all[tip_idx].parent;
                        }
                    }
                }
            }
        }
        tip = Some(idx);
    }
    HeadingTree { all, top }
}

fn parent_tip(headings: &[Heading], current: usize, tip: usize) -> Option<usize> {
    let mut parent = headings[tip].parent;
    while let Some(p) = parent {
        if headings[p].level < headings[current].level {
            return Some(p);
        }
        parent = headings[p].parent;
    }
    None
}

/// 标题纯文本，对应 Go 版 render.headingText。
pub(crate) fn heading_text(n: Node) -> String {
    let mut buf: Vec<u8> = Vec::new();
    crate::ast::walk_raw(n, &mut |n, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        match n.typ() {
            NodeType::NodeLinkText
            | NodeType::NodeBlockRefText
            | NodeType::NodeBlockRefDynamicText
            | NodeType::NodeFileAnnotationRefText
            | NodeType::NodeBackslashContent => buf.extend_from_slice(n.tokens()),
            NodeType::NodeInlineMathContent => {
                buf.extend_from_slice(b"<span class=\"language-math\">");
                buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                buf.extend_from_slice(b"</span>");
            }
            NodeType::NodeCodeSpanContent => {
                buf.extend_from_slice(b"<code>");
                buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                buf.extend_from_slice(b"</code>");
            }
            NodeType::NodeText => {
                if n.parent_is(&[NodeType::NodeStrong]) {
                    buf.extend_from_slice(b"<strong>");
                    buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                    buf.extend_from_slice(b"</strong>");
                } else if n.parent_is(&[NodeType::NodeEmphasis]) {
                    buf.extend_from_slice(b"<em>");
                    buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                    buf.extend_from_slice(b"</em>");
                } else if let Some(prev) = n.previous() {
                    if NodeType::NodeInlineHTML == prev.typ() {
                        if prev.tokens().starts_with(b"<font ") {
                            buf.extend_from_slice(prev.tokens());
                            buf.extend_from_slice(n.tokens());
                        }
                        if let Some(next) = n.next() {
                            if b"</font>" == next.tokens() {
                                buf.extend_from_slice(next.tokens());
                            }
                        }
                    } else {
                        buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                    }
                } else {
                    buf.extend_from_slice(&escape_html_bytes(n.tokens()));
                }
            }
            _ => {}
        }
        WalkStatus::WalkContinue
    });
    String::from_utf8_lossy(&buf).to_string()
}

/// 对应 Go 版 HtmlRenderer.handleKramdownBlockIAL：把 IAL 第一项的名字改成渲染名。
pub(crate) fn handle_kramdown_block_ial(n: Node, options: &RenderOptions) {
    if (crate::CAP_KRAMDOWN_BLOCK_IAL && options.kramdown_block_ial)
        && "id" != options.kramdown_ial_id_render_name
        && !n.data().kramdown_ial.is_empty()
    {
        let name = options.kramdown_ial_id_render_name.clone();
        n.data_mut().kramdown_ial[0].0 = name;
    }
}
