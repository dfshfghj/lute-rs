// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 节点模型：单个结构体 + 类型枚举 + 指针句柄。
//!
//! 与 Go 版保持一致：节点身份稳定，允许就地修改类型和父子关系。
//! 对外 Node 自动持有 arena；内部 RawNode 为 Copy 句柄。
//! 跨树链接会合并 arena 的生命周期。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use self::RawNode as Node;
use super::walk::{walk_raw as walk, WalkStatus};
use super::NodeType;
use crate::editor::IAL_VAL_ESC_NEWLINE;
use crate::html;

/// 测试环境使用的固定节点 ID，对应 Go 版 ast.Testing 分支。
pub const TESTING_NODE_ID: &str = "20060102150405-1a2b3c4";

/// 对应 Go 版 ast.Testing。
pub static TESTING: AtomicBool = AtomicBool::new(false);

/// 列表附加数据，对应 Go 版 ast.ListData。
#[derive(Clone, Debug, Default)]
pub struct ListData {
    /// 0：无序列表，1：有序列表，3：任务列表。
    pub typ: i32,
    pub tight: bool,
    pub bullet_char: u8,
    pub start: i32,
    pub delimiter: u8,
    /// 列表内部缩进空格数（包含标识符长度，即规范中的 W+N）。
    pub padding: i32,
    /// 标识符相对缩进空格数。
    pub marker_offset: i32,
    pub checked: bool,
    pub marker: Vec<u8>,
    /// 有序列表项修正过的序号。
    pub num: i32,
}

/// 表格单元格富文本源，对应 Go 版 ast.TableCellRich。
#[derive(Clone, Debug, Default)]
pub struct TableCellRich {
    pub spec: i32,
    pub format: String,
    pub content: String,
    pub raw: String,
    pub invalid: bool,
}

/// 富文本单元格载荷的版本与格式，对应 Go 版 ast.TableCellRichSpec / TableCellRichFormat。
pub const TABLE_CELL_RICH_SPEC: i32 = 1;
/// 富文本单元格载荷的存储语法。
pub const TABLE_CELL_RICH_FORMAT: &str = "kramdown";
/// 富文本单元格在 BlockDOM 中的属性名。
pub const TABLE_CELL_RICH_ATTRIBUTE: &str = "data-sy-table-cell-rich";
/// 富文本单元格在 Kramdown IAL 中的属性名。
pub const TABLE_CELL_RICH_IAL: &str = "table-cell-rich";

impl TableCellRich {
    /// 对应 Go 版 TableCellRich.Validate。
    ///
    /// Go 还会拒绝包含非法 UTF-8 的 `Content`；Rust 的 `String` 无法表示该状态，
    /// 非法字节在解码阶段就被拒绝。
    pub fn validate(&self) -> Result<(), String> {
        if self.invalid
            || !self.raw.is_empty()
            || TABLE_CELL_RICH_SPEC != self.spec
            || TABLE_CELL_RICH_FORMAT != self.format
        {
            return Err(format!(
                "unsupported table cell rich text format [{}, {}]",
                self.spec, self.format
            ));
        }
        if self.content.contains('\0') {
            return Err("table cell rich text contains invalid text".to_string());
        }
        Ok(())
    }

    /// 对应 Go 版 TableCellRich.Encode。
    pub fn encode(&self) -> String {
        if self.invalid || !self.raw.is_empty() {
            return self.raw.clone();
        }
        let json = format!(
            "{{\"spec\":{},\"format\":\"{}\",\"content\":\"{}\"}}",
            self.spec,
            crate::render::json::escape_json_string(&self.format),
            crate::render::json::escape_json_string(&self.content)
        );
        crate::base64::encode_raw_url(json.as_bytes())
    }
}

/// 对应 Go 版 ast.DecodeTableCellRich。
///
/// 无法解码的载荷保留原文并标记为非法，禁止调用方把它当作普通单元格静默覆盖。
pub fn decode_table_cell_rich(encoded: &str) -> TableCellRich {
    fn invalid(encoded: &str) -> TableCellRich {
        TableCellRich {
            raw: encoded.to_string(),
            invalid: true,
            ..Default::default()
        }
    }

    let Some(data) = crate::base64::decode_raw_url(encoded) else {
        return invalid(encoded);
    };
    let Ok(text) = std::str::from_utf8(&data) else {
        return invalid(encoded);
    };
    // Go 解码到 map[string]json.RawMessage：重复键保留最后一次出现，
    // 且不接受非对象、缺失字段或 null 值。
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str::<serde_json::Value>(text)
    else {
        return invalid(encoded);
    };
    if 3 != fields.len() {
        return invalid(encoded);
    }
    let (Some(spec), Some(format), Some(content)) = (
        fields.get("spec").and_then(|value| value.as_i64()),
        fields.get("format").and_then(|value| value.as_str()),
        fields.get("content").and_then(|value| value.as_str()),
    ) else {
        return invalid(encoded);
    };
    // 小数与指数形式在 Go 中无法解码到 int，as_i64 同样拒绝。
    let Ok(spec) = i32::try_from(spec) else {
        return invalid(encoded);
    };
    TableCellRich {
        spec,
        format: format.to_string(),
        content: content.to_string(),
        raw: String::new(),
        invalid: false,
    }
}

/// 节点的全部数据。字段与 Go 版 ast.Node 一一对应。
///
/// M1 只实现了树结构，字段要等 M2 解析器落地后才会被读写。
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct NodeData {
    owner: Weak<RefCell<NodeArena>>,
    pub id: String,
    pub box_: String,
    pub path: String,
    pub spec: String,

    pub typ: NodeType,
    pub parent: Option<Node>,
    pub previous: Option<Node>,
    pub next: Option<Node>,
    pub first_child: Option<Node>,
    pub last_child: Option<Node>,
    pub children: Vec<Node>,
    pub tokens: Vec<u8>,
    /// Go distinguishes a nil token slice from an allocated empty slice while
    /// deciding whether an empty paragraph survives inline parsing.
    pub tokens_initialized: bool,
    pub type_str: String,
    pub data: String,

    pub close: bool,
    pub last_line_blank: bool,
    pub last_line_checked: bool,

    pub code_marker_len: i32,

    pub is_fenced_code_block: bool,
    pub code_block_fence_char: u8,
    pub code_block_fence_len: i32,
    pub code_block_fence_offset: i32,
    pub code_block_open_fence: Vec<u8>,
    pub code_block_info: Vec<u8>,
    pub code_block_close_fence: Vec<u8>,

    pub html_block_type: i32,

    pub list_data: Option<ListData>,

    pub task_list_item_checked: bool,
    pub task_list_item_marker: u8,

    pub table_aligns: Vec<i32>,
    pub table_cell_align: i32,
    pub table_cell_content_width: i32,
    pub table_cell_content_max_width: i32,
    pub table_cell_rich: Option<TableCellRich>,
    /// 外部 Markdown 导入时短暂保留的单元格原文，对应 Go 版 Context.TableCellSources。
    /// 解析收尾后由 normalize_markdown_table_cells 消费并清空。
    pub table_cell_source: String,

    pub link_type: i32,
    pub link_ref_label: Vec<u8>,

    pub heading_level: i32,
    pub heading_setext: bool,
    pub heading_normalized_id: String,

    pub math_block_dollar_offset: i32,

    pub footnotes_ref_label: Vec<u8>,
    pub footnotes_ref_id: String,
    pub footnotes_refs: Vec<Node>,

    pub html_entity_tokens: Vec<u8>,

    pub kramdown_ial: Vec<(String, String)>,
    pub properties: HashMap<String, String>,

    pub text_mark_type: String,
    pub text_mark_a_href: String,
    pub text_mark_a_title: String,
    pub text_mark_inline_math_content: String,
    pub text_mark_inline_memo_content: String,
    pub text_mark_block_ref_id: String,
    pub text_mark_block_ref_subtype: String,
    pub text_mark_file_annotation_ref_id: String,
    pub text_mark_flashcard_occlusion_id: String,
    pub text_mark_text_content: String,

    pub attribute_view_id: String,
    pub attribute_view_type: String,

    pub custom_block_fence_offset: i32,
    pub custom_block_info: String,

    pub callout_type: String,
    pub callout_title: String,
    pub callout_title_explicit: bool,
    pub tab_item_title: String,
    pub callout_icon: String,
    pub callout_icon_type: i32,
}

/// 指向节点数据的句柄。
///
/// 句柄是 Copy 的，可以像 Go 的 *ast.Node 一样随意传递。
/// 安全约束：使用句柄时必须持有对应的 Tree 或 NodeOwner。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RawNode(pub(crate) *mut NodeData);

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_null() {
            return f.write_str("Node(null)");
        }
        f.debug_struct("Node")
            .field("type", &self.typ())
            .field("id", &self.id())
            .finish()
    }
}

impl Node {
    /// 保留节点及其关联 arena，供跨语言回调等需要逃逸 Tree 生命周期的句柄使用。
    /// 必须在节点仍由 Tree / NodeOwner 持有时调用。
    pub fn owner(self) -> NodeOwner {
        NodeOwner(self.data().owner.upgrade().expect("node owner must exist"))
    }

    #[inline]
    pub(crate) fn data(&self) -> &NodeData {
        debug_assert!(!self.0.is_null(), "null node");
        unsafe { &*self.0 }
    }

    /// 与 Go 版直接改字段等价：句柄是 Copy 的，可变性由调用方保证独占。
    ///
    /// 这是 ADR-0001 的既定设计（单线程 + arena 所有权保活 + 稳定地址），
    /// 不使用 RefCell 以避免 90 处 ast.Walk 期间改树触发 panic。
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub(crate) fn data_mut(&self) -> &mut NodeData {
        debug_assert!(!self.0.is_null(), "null node");
        unsafe { &mut *self.0 }
    }

    // ---- 基础字段 ----

    #[inline]
    pub fn typ(self) -> NodeType {
        self.data().typ
    }

    #[inline]
    pub fn close(self) -> bool {
        self.data().close
    }

    /// 对应 Go 版 Node.ParentIs。
    pub fn parent_is(self, types: &[NodeType]) -> bool {
        let mut p = self.parent();
        let mut deep = 0;
        while let Some(node) = p {
            if types.contains(&node.typ()) {
                return true;
            }
            deep += 1;
            if 128 < deep {
                break;
            }
            p = node.parent();
        }
        false
    }

    /// 对应 Go 版 Node.AcceptLines。
    pub fn accept_lines(self) -> bool {
        matches!(
            self.typ(),
            NodeType::NodeParagraph
                | NodeType::NodeCodeBlock
                | NodeType::NodeHTMLBlock
                | NodeType::NodeMathBlock
                | NodeType::NodeYamlFrontMatter
                | NodeType::NodeBlockQueryEmbed
                | NodeType::NodeGitConflict
                | NodeType::NodeIFrame
                | NodeType::NodeWidget
                | NodeType::NodeVideo
                | NodeType::NodeAudio
                | NodeType::NodeAttributeView
                | NodeType::NodeCustomBlock
        )
    }

    /// 对应 Go 版 Node.CanContain。
    pub fn can_contain(self, node_type: NodeType) -> bool {
        if NodeType::NodeTabItem == node_type {
            return NodeType::NodeTabs == self.typ();
        }
        if NodeType::NodeMindmapItem == node_type {
            return NodeType::NodeMindmap == self.typ();
        }
        match self.typ() {
            NodeType::NodeTabs => NodeType::NodeKramdownBlockIAL == node_type,
            NodeType::NodeTabItem => {
                NodeType::NodeDocument != node_type
                    && NodeType::NodeListItem != node_type
                    && NodeType::NodeFootnotesDef != node_type
            }
            NodeType::NodeMindmap => NodeType::NodeMindmapItem == node_type,
            NodeType::NodeMindmapItem => {
                NodeType::NodeDocument != node_type
                    && NodeType::NodeList != node_type
                    && NodeType::NodeListItem != node_type
                    && NodeType::NodeFootnotesDef != node_type
            }
            NodeType::NodeCodeBlock
            | NodeType::NodeHTMLBlock
            | NodeType::NodeParagraph
            | NodeType::NodeThematicBreak
            | NodeType::NodeTable
            | NodeType::NodeMathBlock
            | NodeType::NodeYamlFrontMatter
            | NodeType::NodeGitConflict
            | NodeType::NodeIFrame
            | NodeType::NodeWidget
            | NodeType::NodeVideo
            | NodeType::NodeAudio
            | NodeType::NodeAttributeView
            | NodeType::NodeCustomBlock => false,
            NodeType::NodeList => NodeType::NodeListItem == node_type,
            NodeType::NodeFootnotesDefBlock => NodeType::NodeFootnotesDef == node_type,
            NodeType::NodeFootnotesDef => NodeType::NodeFootnotesDef != node_type,
            NodeType::NodeSuperBlock => {
                if let Some(last) = self.last_child() {
                    if NodeType::NodeSuperBlockCloseMarker == last.typ() {
                        return false; // 超级块已闭合
                    }
                }
                true
            }
            _ => NodeType::NodeListItem != node_type,
        }
    }

    /// 块级语法树文本导出，格式与上游 Go 工具 `astdump` 一致。
    pub fn dump(self) -> String {
        let mut out = String::new();
        self.dump_to(&mut out, 0);
        out
    }

    fn dump_to(self, out: &mut String, depth: usize) {
        use std::fmt::Write as _;

        let d = self.data();
        for _ in 0..depth {
            out.push_str("  ");
        }
        let _ = write!(
            out,
            "{} tokens={}",
            d.typ.as_str(),
            go_quote(&String::from_utf8_lossy(&d.tokens))
        );
        match d.typ {
            NodeType::NodeHeading => {
                let _ = write!(
                    out,
                    " level={} setext={}",
                    d.heading_level, d.heading_setext
                );
            }
            NodeType::NodeCodeBlock => {
                let _ = write!(out, " fenced={}", d.is_fenced_code_block);
                if d.is_fenced_code_block {
                    let _ = write!(
                        out,
                        " fence_char={} fence_len={} fence_offset={} open={} info={} close={}",
                        d.code_block_fence_char,
                        d.code_block_fence_len,
                        d.code_block_fence_offset,
                        go_quote(&String::from_utf8_lossy(&d.code_block_open_fence)),
                        go_quote(&String::from_utf8_lossy(&d.code_block_info)),
                        go_quote(&String::from_utf8_lossy(&d.code_block_close_fence))
                    );
                }
            }
            NodeType::NodeHTMLBlock
            | NodeType::NodeIFrame
            | NodeType::NodeVideo
            | NodeType::NodeAudio
            | NodeType::NodeWidget => {
                let _ = write!(out, " html_block_type={}", d.html_block_type);
            }
            NodeType::NodeListItem => {
                let _ = write!(
                    out,
                    " task_marker={} checked={}",
                    d.task_list_item_marker, d.task_list_item_checked
                );
            }
            NodeType::NodeCodeSpan => {
                let _ = write!(out, " marker_len={}", d.code_marker_len);
            }
            NodeType::NodeLink | NodeType::NodeImage => {
                let _ = write!(
                    out,
                    " link_type={} ref={}",
                    d.link_type,
                    go_quote(&String::from_utf8_lossy(&d.link_ref_label))
                );
            }
            NodeType::NodeHTMLEntity => {
                let _ = write!(
                    out,
                    " entity={}",
                    go_quote(&String::from_utf8_lossy(&d.html_entity_tokens))
                );
            }
            NodeType::NodeFootnotesRef => {
                let _ = write!(
                    out,
                    " ref_id={} ref_label={}",
                    go_quote(&d.footnotes_ref_id),
                    go_quote(&String::from_utf8_lossy(&d.footnotes_ref_label))
                );
            }
            _ => {}
        }
        if let Some(ld) = &d.list_data {
            let _ = write!(
                out,
                " list={{typ:{} tight:{} bullet:{} start:{} delim:{} padding:{} marker_offset:{} checked:{} marker:{} num:{}}}",
                ld.typ,
                ld.tight,
                ld.bullet_char,
                ld.start,
                ld.delimiter,
                ld.padding,
                ld.marker_offset,
                ld.checked,
                go_quote(&String::from_utf8_lossy(&ld.marker)),
                ld.num
            );
        }
        out.push('\n');

        let mut child = d.first_child;
        while let Some(c) = child {
            c.dump_to(out, depth + 1);
            child = c.data().next;
        }
    }

    /// 对应 Go 版 Node.ReviveFromMarker。
    pub fn revive_from_marker(self, marker: u8) {
        let d = self.data_mut();
        d.task_list_item_marker = marker;
        d.task_list_item_checked = marker != b' ' && marker != 0;
    }

    #[inline]
    pub fn set_typ(self, typ: NodeType) {
        self.data_mut().typ = typ;
    }

    /// 节点 ID。返回值借用句柄，句柄是局部变量时可直接使用。
    #[inline]
    pub fn id(&self) -> &str {
        &self.data().id
    }

    pub fn id_owned(&self) -> String {
        self.data().id.clone()
    }

    #[inline]
    pub fn set_id(self, id: impl Into<String>) {
        self.data_mut().id = id.into();
    }

    /// 原始 tokens。返回值借用句柄，句柄是局部变量时可直接使用。
    #[inline]
    pub fn tokens(&self) -> &[u8] {
        &self.data().tokens
    }

    pub fn tokens_vec(self) -> Vec<u8> {
        self.data().tokens.clone()
    }

    pub fn tokens_str(self) -> String {
        String::from_utf8_lossy(&self.data().tokens).into_owned()
    }

    pub fn set_tokens(self, tokens: impl Into<Vec<u8>>) {
        self.data_mut().tokens = tokens.into();
        self.data_mut().tokens_initialized = true;
    }

    pub(crate) fn clear_tokens(self) {
        self.data_mut().tokens.clear();
        self.data_mut().tokens_initialized = false;
    }

    pub fn append_tokens(self, tokens: &[u8]) {
        self.data_mut().tokens.extend_from_slice(tokens);
        self.data_mut().tokens_initialized |= !tokens.is_empty();
    }

    pub fn prepend_tokens(self, tokens: &[u8]) {
        let mut merged = Vec::with_capacity(tokens.len() + self.data().tokens.len());
        merged.extend_from_slice(tokens);
        merged.extend_from_slice(&self.data().tokens);
        self.data_mut().tokens = merged;
        self.data_mut().tokens_initialized |= !tokens.is_empty();
    }

    pub fn token_len(self) -> usize {
        let mut ret = 0;
        walk(self, &mut |n, entering| {
            if entering {
                ret += bytes_show_length(&n.data().tokens);
            }
            WalkStatus::WalkContinue
        });
        ret
    }

    // ---- 树结构 ----

    #[inline]
    pub fn parent(self) -> Option<Node> {
        self.data().parent
    }

    #[inline]
    pub fn previous(self) -> Option<Node> {
        self.data().previous
    }

    #[inline]
    pub fn next(self) -> Option<Node> {
        self.data().next
    }

    #[inline]
    pub fn first_child(self) -> Option<Node> {
        self.data().first_child
    }

    #[inline]
    pub fn last_child(self) -> Option<Node> {
        self.data().last_child
    }

    pub fn children(self) -> RawChildren {
        RawChildren {
            next: self.data().first_child,
        }
    }

    /// 对应 Go 版 Node.AppendChild。
    pub fn append_child(self, child: Node) {
        self.owner().merge(&child.owner());
        child.unlink();
        child.data_mut().parent = Some(self);
        let last_child = self.data().last_child;
        match last_child {
            Some(last) => {
                last.data_mut().next = Some(child);
                child.data_mut().previous = Some(last);
                self.data_mut().last_child = Some(child);
            }
            None => {
                self.data_mut().first_child = Some(child);
                self.data_mut().last_child = Some(child);
            }
        }
    }

    /// 对应 Go 版 Node.PrependChild。
    pub fn prepend_child(self, child: Node) {
        self.owner().merge(&child.owner());
        child.unlink();
        child.data_mut().parent = Some(self);
        let first_child = self.data().first_child;
        match first_child {
            Some(first) => {
                first.data_mut().previous = Some(child);
                child.data_mut().next = Some(first);
                self.data_mut().first_child = Some(child);
            }
            None => {
                self.data_mut().first_child = Some(child);
                self.data_mut().last_child = Some(child);
            }
        }
    }

    /// 对应 Go 版 Node.InsertBefore。
    pub fn insert_before(self, sibling: Node) {
        self.owner().merge(&sibling.owner());
        sibling.unlink();
        let previous = self.data().previous;
        sibling.data_mut().previous = previous;
        if let Some(prev) = previous {
            prev.data_mut().next = Some(sibling);
        }
        sibling.data_mut().next = Some(self);
        self.data_mut().previous = Some(sibling);
        let parent = self.data().parent;
        sibling.data_mut().parent = parent;
        if let Some(p) = parent {
            if sibling.data().previous.is_none() {
                p.data_mut().first_child = Some(sibling);
            }
        }
    }

    /// 对应 Go 版 Node.InsertAfter。
    pub fn insert_after(self, sibling: Node) {
        self.owner().merge(&sibling.owner());
        sibling.unlink();
        let next = self.data().next;
        sibling.data_mut().next = next;
        if let Some(n) = next {
            n.data_mut().previous = Some(sibling);
        }
        sibling.data_mut().previous = Some(self);
        self.data_mut().next = Some(sibling);
        let parent = self.data().parent;
        sibling.data_mut().parent = parent;
        if let Some(p) = parent {
            if sibling.data().next.is_none() && p.data().last_child.is_some() {
                p.data_mut().last_child = Some(sibling);
            }
        }
    }

    /// 对应 Go 版 Node.Unlink。
    pub fn unlink(self) {
        let parent = self.data().parent;
        let previous = self.data().previous;
        let next = self.data().next;

        match previous {
            Some(prev) => prev.data_mut().next = next,
            None => {
                if let Some(p) = parent {
                    p.data_mut().first_child = next;
                }
            }
        }
        match next {
            Some(n) => n.data_mut().previous = previous,
            None => {
                if let Some(p) = parent {
                    p.data_mut().last_child = previous;
                }
            }
        }

        let d = self.data_mut();
        d.parent = None;
        d.next = None;
        d.previous = None;
    }

    // ---- 查询 ----

    /// 对应 Go 版 Node.ChildByType。
    pub fn child_by_type(self, child_type: NodeType) -> Option<Node> {
        self.children().find(|c| c.typ() == child_type)
    }

    /// 对应 Go 版 Node.ChildrenByType。
    pub fn children_by_type(self, child_type: NodeType) -> Vec<Node> {
        let mut matches = Vec::new();
        walk(self, &mut |node, entering| {
            if entering && node.typ() == child_type {
                matches.push(node);
            }
            WalkStatus::WalkContinue
        });
        matches
    }

    /// 对应 Go 版 Node.List。
    pub fn list(self) -> Vec<Node> {
        let mut ret = Vec::with_capacity(512);
        walk(self, &mut |n, entering| {
            if entering {
                ret.push(n);
            }
            WalkStatus::WalkContinue
        });
        ret
    }

    // ---- 文本 ----

    /// 对应 Go 版 Node.Text。
    pub fn text(self) -> String {
        let mut buf = String::new();
        walk(self, &mut |n, entering| {
            if !entering {
                return WalkStatus::WalkContinue;
            }
            match n.typ() {
                NodeType::NodeText
                | NodeType::NodeLinkText
                | NodeType::NodeBlockRefText
                | NodeType::NodeBlockRefDynamicText
                | NodeType::NodeFileAnnotationRefText
                | NodeType::NodeFootnotesRef => {
                    buf.push_str(&n.tokens_str());
                }
                NodeType::NodeTextMark => {
                    buf.push_str(&n.data().text_mark_text_content);
                }
                _ => {}
            }
            WalkStatus::WalkContinue
        });
        buf
    }

    /// 对应 Go 版 Node.TextLen，返回字符数。
    pub fn text_len(self) -> usize {
        let mut buf = String::new();
        walk(self, &mut |n, entering| {
            if !entering {
                return WalkStatus::WalkContinue;
            }
            match n.typ() {
                NodeType::NodeText
                | NodeType::NodeLinkText
                | NodeType::NodeBlockRefText
                | NodeType::NodeBlockRefDynamicText
                | NodeType::NodeFileAnnotationRefText
                | NodeType::NodeFootnotesRef => {
                    buf.push_str(&n.tokens_str());
                }
                NodeType::NodeTextMark => {
                    buf.push_str(&n.data().text_mark_text_content);
                }
                _ => {}
            }
            WalkStatus::WalkContinue
        });
        buf.chars().count()
    }

    /// 对应 Go 版 Node.Content。
    /// 对应 Go 版 Node.EscapeMarkerContent。
    pub fn escape_marker_content(self) -> String {
        let content = self.content();
        String::from_utf8_lossy(&crate::lex::escape_protyle_markers(content.as_bytes())).to_string()
    }

    pub fn content(self) -> String {
        let mut buf = String::new();
        walk(self, &mut |n, entering| {
            if !entering {
                let has_next_sibling = n.next().is_some();
                let has_next_next = n.next().and_then(|x| x.next()).is_some();
                if has_next_sibling
                    && has_next_next
                    && buf.len() > 1
                    && n.is_block()
                    && !buf.ends_with('\n')
                {
                    buf.push('\n');
                }
                return WalkStatus::WalkContinue;
            }

            match n.typ() {
                NodeType::NodeText
                | NodeType::NodeLinkText
                | NodeType::NodeBlockRefText
                | NodeType::NodeBlockRefDynamicText
                | NodeType::NodeFileAnnotationRefText
                | NodeType::NodeFootnotesRef
                | NodeType::NodeCodeSpanContent
                | NodeType::NodeCodeBlockCode
                | NodeType::NodeInlineMathContent
                | NodeType::NodeMathBlockContent
                | NodeType::NodeHTMLEntity
                | NodeType::NodeEmojiAlias
                | NodeType::NodeEmojiUnicode
                | NodeType::NodeBackslashContent
                | NodeType::NodeYamlFrontMatterContent
                | NodeType::NodeGitConflictContent => {
                    buf.push_str(&n.tokens_str());
                }
                NodeType::NodeTextMark => {
                    let d = n.data();
                    if !d.text_mark_text_content.is_empty() {
                        buf.push_str(&html::unescape_string(&d.text_mark_text_content));
                    } else if !d.text_mark_inline_math_content.is_empty() {
                        buf.push_str(
                            &d.text_mark_inline_math_content
                                .replace(IAL_VAL_ESC_NEWLINE, " "),
                        );
                    }
                    if !d.text_mark_inline_memo_content.is_empty() {
                        buf.push_str(
                            &d.text_mark_inline_memo_content
                                .replace(IAL_VAL_ESC_NEWLINE, " "),
                        );
                    }
                }
                _ => {}
            }
            WalkStatus::WalkContinue
        });
        buf
    }

    // ---- 类型判断 ----

    /// 对应 Go 版 Node.IsBlock。
    pub fn is_block(self) -> bool {
        matches!(
            self.typ(),
            NodeType::NodeDocument
                | NodeType::NodeParagraph
                | NodeType::NodeHeading
                | NodeType::NodeThematicBreak
                | NodeType::NodeBlockquote
                | NodeType::NodeList
                | NodeType::NodeListItem
                | NodeType::NodeHTMLBlock
                | NodeType::NodeCodeBlock
                | NodeType::NodeTable
                | NodeType::NodeMathBlock
                | NodeType::NodeFootnotesDefBlock
                | NodeType::NodeFootnotesDef
                | NodeType::NodeToC
                | NodeType::NodeYamlFrontMatter
                | NodeType::NodeBlockQueryEmbed
                | NodeType::NodeKramdownBlockIAL
                | NodeType::NodeSuperBlock
                | NodeType::NodeGitConflict
                | NodeType::NodeAudio
                | NodeType::NodeVideo
                | NodeType::NodeIFrame
                | NodeType::NodeWidget
                | NodeType::NodeAttributeView
                | NodeType::NodeCustomBlock
                | NodeType::NodeCallout
                | NodeType::NodeTabs
                | NodeType::NodeTabItem
                | NodeType::NodeMindmap
                | NodeType::NodeMindmapItem
        )
    }

    /// 对应 Go 版 Node.IsContainerBlock。
    pub fn is_container_block(self) -> bool {
        matches!(
            self.typ(),
            NodeType::NodeDocument
                | NodeType::NodeBlockquote
                | NodeType::NodeList
                | NodeType::NodeListItem
                | NodeType::NodeFootnotesDefBlock
                | NodeType::NodeFootnotesDef
                | NodeType::NodeSuperBlock
                | NodeType::NodeCallout
                | NodeType::NodeTabs
                | NodeType::NodeTabItem
                | NodeType::NodeMindmap
                | NodeType::NodeMindmapItem
        )
    }

    pub(crate) fn is_empty_block_ial(self) -> bool {
        if self.typ() != NodeType::NodeKramdownBlockIAL
            || self
                .tokens()
                .windows(b"type=\"doc\"".len())
                .any(|s| s == b"type=\"doc\"")
        {
            return false;
        }
        if let Some(previous) = self.previous() {
            return previous.typ() == NodeType::NodeKramdownBlockIAL;
        }
        self.parent()
            .is_none_or(|parent| parent.is_container_block())
    }

    /// 对应 Go 版 Node.IsMarker。
    pub fn is_marker(self) -> bool {
        matches!(
            self.typ(),
            NodeType::NodeHeadingC8hMarker
                | NodeType::NodeBlockquoteMarker
                | NodeType::NodeCodeBlockFenceOpenMarker
                | NodeType::NodeCodeBlockFenceCloseMarker
                | NodeType::NodeCodeBlockFenceInfoMarker
                | NodeType::NodeEmA6kOpenMarker
                | NodeType::NodeEmA6kCloseMarker
                | NodeType::NodeEmU8eOpenMarker
                | NodeType::NodeEmU8eCloseMarker
                | NodeType::NodeStrongA6kOpenMarker
                | NodeType::NodeStrongA6kCloseMarker
                | NodeType::NodeStrongU8eOpenMarker
                | NodeType::NodeStrongU8eCloseMarker
                | NodeType::NodeCodeSpanOpenMarker
                | NodeType::NodeCodeSpanCloseMarker
                | NodeType::NodeTaskListItemMarker
                | NodeType::NodeStrikethrough1OpenMarker
                | NodeType::NodeStrikethrough1CloseMarker
                | NodeType::NodeStrikethrough2OpenMarker
                | NodeType::NodeStrikethrough2CloseMarker
                | NodeType::NodeMathBlockOpenMarker
                | NodeType::NodeMathBlockCloseMarker
                | NodeType::NodeInlineMathOpenMarker
                | NodeType::NodeInlineMathCloseMarker
                | NodeType::NodeYamlFrontMatterOpenMarker
                | NodeType::NodeYamlFrontMatterCloseMarker
                | NodeType::NodeMark1OpenMarker
                | NodeType::NodeMark1CloseMarker
                | NodeType::NodeMark2OpenMarker
                | NodeType::NodeMark2CloseMarker
                | NodeType::NodeTagOpenMarker
                | NodeType::NodeTagCloseMarker
                | NodeType::NodeSuperBlockOpenMarker
                | NodeType::NodeSuperBlockLayoutMarker
                | NodeType::NodeSuperBlockCloseMarker
                | NodeType::NodeSupOpenMarker
                | NodeType::NodeSupCloseMarker
                | NodeType::NodeSubOpenMarker
                | NodeType::NodeSubCloseMarker
        )
    }

    // ---- IAL ----

    pub fn ial_raw(self) -> Vec<(String, String)> {
        self.data().kramdown_ial.clone()
    }

    /// 对应 Go 版 Node.SetIALAttr。
    pub fn set_ial_attr(self, name: &str, value: &str) {
        let escaped = html::escape_attr_val(value);
        let d = self.data_mut();
        for kv in d.kramdown_ial.iter_mut() {
            if kv.0 == name {
                kv.1 = escaped;
                return;
            }
        }
        d.kramdown_ial.push((name.to_string(), escaped));
    }

    /// 对应 Go 版 Node.RemoveIALAttr。
    pub fn remove_ial_attr(self, name: &str) {
        self.data_mut().kramdown_ial.retain(|kv| kv.0 != name);
    }

    /// 对应 Go 版 Node.RemoveIALAttrsByPrefix。
    pub fn remove_ial_attrs_by_prefix(self, prefix: &str) {
        self.data_mut()
            .kramdown_ial
            .retain(|kv| !kv.0.starts_with(prefix));
    }

    /// 对应 Go 版 Node.IALAttr。
    /// 对应 Go 版 Node.IsTextMarkType。
    pub fn is_text_mark_type(self, typ: &str) -> bool {
        self.data().text_mark_type.split(' ').any(|t| t == typ)
    }

    /// 对应 Go 版 Node.IsNextSameInlineMemo。
    pub fn is_next_same_inline_memo(self) -> bool {
        let mut next_inline_memo = None;
        let mut node = self.next();
        while let Some(cur) = node {
            if self.next().is_none()
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
            if NodeType::NodeTextMark == cur.typ() && cur.is_text_mark_type("inline-memo") {
                next_inline_memo = Some(cur);
                break;
            }
            node = cur.next();
        }
        match next_inline_memo {
            Some(m) => {
                self.data().text_mark_inline_memo_content == m.data().text_mark_inline_memo_content
            }
            None => false,
        }
    }

    /// 对应 Go 版 Node.IsSameTextMarkType。
    pub fn is_same_text_mark_type(self, other: Node) -> bool {
        let a_data = self.data();
        let b_data = other.data();
        if a_data.text_mark_type.is_empty() || b_data.text_mark_type.is_empty() {
            return false;
        }
        if a_data.text_mark_flashcard_occlusion_id != b_data.text_mark_flashcard_occlusion_id {
            return false;
        }
        let mut a: Vec<&str> = a_data.text_mark_type.split(' ').collect();
        let mut b: Vec<&str> = b_data.text_mark_type.split(' ').collect();
        if a.len() != b.len() {
            return false;
        }
        a.sort_unstable();
        b.sort_unstable();
        a.iter().zip(&b).all(|(left, right)| {
            left == right
                && match *left {
                    "block-ref" => a_data.text_mark_block_ref_id == b_data.text_mark_block_ref_id,
                    "a" => a_data.text_mark_a_href == b_data.text_mark_a_href,
                    "inline-memo" => {
                        a_data.text_mark_inline_memo_content == b_data.text_mark_inline_memo_content
                    }
                    _ => true,
                }
        })
    }

    /// 对应 Go 版 Node.SortTextMarkDataTypes。
    pub fn sort_text_mark_data_types(self) {
        if self.data().text_mark_text_content.is_empty() {
            return;
        }
        let mut types: Vec<&str> = self.data().text_mark_type.split(' ').collect();
        types.sort_unstable();
        self.data_mut().text_mark_type = types.join(" ");
    }

    pub fn ial_attr(self, name: &str) -> String {
        for (k, v) in self.data().kramdown_ial.iter() {
            if k == name {
                return html::unescape_attr_val(v);
            }
        }
        String::new()
    }

    // ---- 任务列表 ----

    /// 对应 Go 版 Node.EffectiveTaskListItemMarker。
    pub fn effective_task_list_item_marker(self) -> String {
        let marker = self.data().task_list_item_marker;
        if marker != 0 {
            if marker == b'x' {
                return "X".to_string();
            }
            return html::escape_html_str(&(marker as char).to_string());
        }
        if self.data().task_list_item_checked {
            return "X".to_string();
        }
        " ".to_string()
    }
}

/// 子节点迭代器。
pub(crate) struct RawChildren {
    next: Option<Node>,
}

impl Iterator for RawChildren {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        let current = self.next?;
        self.next = current.data().next;
        Some(current)
    }
}

/// 可克隆的节点内存所有者。持有它会保留同一 arena 中的全部节点。
///
/// 链接跨树节点时合并 arena，旧所有者通过单向 parent 保活合并后的 arena。
/// 节点仅弱引用 arena，不形成 Rc 环；最后一个 Tree / NodeOwner 释放时回收。
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeOwner(Rc<RefCell<NodeArena>>);

#[derive(Debug, Default)]
struct NodeArena {
    // 必须装箱：Vec 扩容不能移动 NodeData。
    #[allow(clippy::vec_box)]
    nodes: Vec<Box<NodeData>>,
    parent: Option<NodeOwner>,
}

impl NodeOwner {
    fn root(&self) -> Self {
        let mut owner = self.clone();
        loop {
            let parent = owner.0.borrow().parent.clone();
            match parent {
                Some(parent) => owner = parent,
                None => return owner,
            }
        }
    }

    fn merge(&self, other: &Self) {
        let mut target = self.root();
        let mut source = other.root();
        if Rc::ptr_eq(&target.0, &source.0) {
            return;
        }
        // 移动较小 arena，避免反复跨树插入导致平方级所有权更新。
        if target.0.borrow().nodes.len() < source.0.borrow().nodes.len() {
            std::mem::swap(&mut target, &mut source);
        }
        let mut source_arena = source.0.borrow_mut();
        for node in &mut source_arena.nodes {
            node.owner = Rc::downgrade(&target.0);
        }
        target.0.borrow_mut().nodes.append(&mut source_arena.nodes);
        source_arena.parent = Some(target.clone());
    }
}

/// 节点树，与逃逸节点句柄共享 arena 的内存所有权。
///
/// 节点地址保持稳定；跨树链接后，任一关联 Tree / NodeOwner 都能保活全部节点。
pub struct Tree {
    owner: NodeOwner,
    root: Option<Node>,
    /// 文档名，对应 Go 版 parse.Tree.Name。
    pub name: String,
    /// 文档 ID，对应 Go 版 parse.Tree.ID。
    pub id: String,
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    pub fn new() -> Self {
        let mut tree = Tree {
            owner: NodeOwner::default(),
            root: None,
            name: String::new(),
            id: String::new(),
        };
        let root = tree.new_node_raw(NodeType::NodeDocument);
        tree.root = Some(root);
        tree
    }

    /// 分配一个新节点。节点在最后一个关联 Tree / NodeOwner 释放时回收。
    pub(crate) fn new_node_raw(&mut self, typ: NodeType) -> Node {
        let owner = self.owner.root();
        let mut data = Box::new(NodeData {
            typ,
            owner: Rc::downgrade(&owner.0),
            ..NodeData::default()
        });
        let ptr = &mut *data as *mut NodeData;
        owner.0.borrow_mut().nodes.push(data);
        Node(ptr)
    }

    /// Allocate a node whose handle retains its arena.
    pub fn new_node(&mut self, typ: NodeType) -> super::Node {
        super::Node::from_raw(self.new_node_raw(typ))
    }

    /// 共享另一棵树的节点所有权，保留两侧已有句柄的有效性。
    pub(crate) fn adopt(&mut self, other: &mut Tree) {
        self.owner.merge(&other.owner);
    }

    /// 新建带文档名的树。
    pub fn new_with_name(name: &str) -> Self {
        let mut tree = Self::new();
        tree.name = name.to_string();
        tree
    }

    /// 根节点（NodeDocument）。
    pub(crate) fn root_raw(&self) -> Node {
        self.root.expect("tree root must exist")
    }

    /// Return a root handle that remains valid after this tree is dropped.
    pub fn root(&self) -> super::Node {
        super::Node::from_raw(self.root_raw())
    }

    /// 共享 arena 已分配的节点数量（跨树链接后包括关联树的节点）。
    pub fn len(&self) -> usize {
        self.owner.root().0.borrow().nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.owner.root().0.borrow().nodes.is_empty()
    }
}

/// 与 Go 的 %q 尽量一致的字符串转义，仅用于差分测试输出。
fn go_quote(s: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\v"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || 0x7f == c as u32 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c if !is_print(c) => {
                if (c as u32) <= 0xffff {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                } else {
                    let _ = write!(out, "\\U{:08x}", c as u32);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 对应 Go 版 unicode.IsPrint。
fn is_print(c: char) -> bool {
    let cp = c as u32;
    crate::char_class::PRINTABLE
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

/// 对应 Go 版 lex.BytesShowLength：ASCII 记 1 列，其余 UTF-8 首字节记 2 列。
pub fn bytes_show_length(bytes: &[u8]) -> usize {
    let mut length = 0;
    for &b in bytes {
        if (b & 0xc0) != 0x80 {
            if b < 0x7f {
                length += 1;
            } else {
                length += 2;
            }
        }
    }
    length
}

/// 对应 Go 版 ast.NewNodeID。
pub fn new_node_id() -> String {
    if TESTING.load(Ordering::Relaxed) {
        return TESTING_NODE_ID.to_string();
    }
    let (y, m, d, hh, mm, ss) = now_utc_parts();
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}-{}",
        y,
        m,
        d,
        hh,
        mm,
        ss,
        rand_str(7)
    )
}

/// 当前 Unix 纳秒。wasm32-unknown-unknown 没有系统时钟，改用 JS 时间。
#[cfg(not(target_arch = "wasm32"))]
fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn now_nanos() -> u128 {
    (js_sys::Date::now() as u128) * 1_000_000
}

fn now_utc_parts() -> (i64, i64, i64, i64, i64, i64) {
    let secs = (now_nanos() / 1_000_000_000) as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    (y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Howard Hinnant 的 days-from-civil 逆运算。
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

static RNG_STATE: AtomicU64 = AtomicU64::new(0);

fn rand_str(length: usize) -> String {
    const LETTER: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut state = RNG_STATE.load(Ordering::Relaxed);
    if state == 0 {
        state = now_nanos() as u64 | 1;
    }
    let mut ret = String::with_capacity(length);
    for _ in 0..length {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let v = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ret.push(LETTER[(v % LETTER.len() as u64) as usize] as char);
    }
    RNG_STATE.store(state, Ordering::Relaxed);
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_slice_allocation_state_survives_empty_values() {
        let mut tree = Tree::new();
        let node = tree.new_node_raw(NodeType::NodeParagraph);
        assert!(!node.data().tokens_initialized);
        node.append_tokens(b"");
        assert!(!node.data().tokens_initialized);
        node.set_tokens(Vec::new());
        assert!(node.data().tokens_initialized);
        assert!(node.tokens().is_empty());
        node.clear_tokens();
        assert!(!node.data().tokens_initialized);
        node.prepend_tokens(b"x");
        assert!(node.data().tokens_initialized);
        assert_eq!(node.tokens(), b"x");
    }

    #[test]
    fn text_mark_equality_checks_semantic_metadata() {
        let mut tree = Tree::new();
        let a = tree.new_node_raw(NodeType::NodeTextMark);
        let b = tree.new_node_raw(NodeType::NodeTextMark);
        for typ in ["block-ref", "a", "inline-memo"] {
            a.data_mut().text_mark_type = typ.into();
            b.data_mut().text_mark_type = typ.into();
            assert!(a.is_same_text_mark_type(b));
            match typ {
                "block-ref" => b.data_mut().text_mark_block_ref_id = "other".into(),
                "a" => b.data_mut().text_mark_a_href = "other".into(),
                _ => b.data_mut().text_mark_inline_memo_content = "other".into(),
            }
            assert!(!a.is_same_text_mark_type(b));
        }
        // Go's current link comparison intentionally compares href alone.
        a.data_mut().text_mark_type = "a".into();
        b.data_mut().text_mark_type = "a".into();
        a.data_mut().text_mark_a_href = b.data().text_mark_a_href.clone();
        b.data_mut().text_mark_a_title = "different title".into();
        assert!(a.is_same_text_mark_type(b));
    }

    #[test]
    fn empty_block_ial_requires_container_placement() {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        let ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        ial.set_tokens(b"{: id=\"block\"}".to_vec());
        assert!(ial.is_empty_block_ial());
        root.append_child(ial);
        assert!(ial.is_empty_block_ial());
        let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
        root.prepend_child(paragraph);
        assert!(!ial.is_empty_block_ial());
        paragraph.append_child(ial);
        assert!(!ial.is_empty_block_ial());
        let previous = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        paragraph.prepend_child(previous);
        assert!(ial.is_empty_block_ial());
        ial.set_tokens(b"{: id=\"doc\" type=\"doc\"}".to_vec());
        assert!(!ial.is_empty_block_ial());
    }

    fn build_tree() -> (Tree, Node, Node, Node) {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        let p1 = tree.new_node_raw(NodeType::NodeParagraph);
        let p2 = tree.new_node_raw(NodeType::NodeParagraph);
        let text = tree.new_node_raw(NodeType::NodeText);
        text.set_tokens("hello".as_bytes().to_vec());
        root.append_child(p1);
        root.append_child(p2);
        p1.append_child(text);
        (tree, root, p1, p2)
    }

    #[test]
    fn owning_handles_reclaim_the_arena_after_the_last_handle() {
        let mut tree = Tree::new();
        let child = tree.new_node(NodeType::NodeText);
        tree.root().append_child(&child);
        let weak = Rc::downgrade(&tree.owner.0);
        let children = tree.root().children();
        drop(tree);
        assert!(weak.upgrade().is_some());
        drop(child);
        assert!(weak.upgrade().is_some());
        drop(children);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn retained_owner_outlives_tree_and_reclaims_arena() {
        let mut tree = Tree::new();
        let text = tree.new_node_raw(NodeType::NodeText);
        text.set_tokens(b"retained".to_vec());
        tree.root_raw().append_child(text);
        let owner = text.owner();
        let weak = Rc::downgrade(&owner.0);
        drop(tree);
        assert_eq!("retained", text.text());
        assert_eq!(Some(text), text.parent().unwrap().first_child());
        drop(owner);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn cross_tree_links_preserve_old_owners_without_cycles() {
        let linkers: [fn(Node, Node); 4] = [
            Node::append_child,
            Node::prepend_child,
            Node::insert_before,
            Node::insert_after,
        ];
        for link in linkers {
            let mut target = Tree::new();
            let mut source = Tree::new();
            let anchor = target.new_node_raw(NodeType::NodeParagraph);
            target.root_raw().append_child(anchor);
            let text = source.new_node_raw(NodeType::NodeText);
            text.set_tokens(b"source".to_vec());
            source.root_raw().append_child(text);
            let target_owner = anchor.owner();
            let source_owner = text.owner();
            let target_weak = Rc::downgrade(&target_owner.0);
            let source_weak = Rc::downgrade(&source_owner.0);
            link(anchor, text);
            // Linking the already joined arenas again must not create a parent cycle.
            link(anchor, text);
            drop(target);
            drop(source);
            assert_eq!("source", text.text());
            text.unlink();
            drop(source_owner);
            assert_eq!("source", text.text());
            drop(target_owner);
            assert!(target_weak.upgrade().is_none());
            assert!(source_weak.upgrade().is_none());
        }
    }

    #[test]
    fn arena_merge_keeps_old_tree_roots_and_future_allocations_alive() {
        let mut small = Tree::new();
        let mut large = Tree::new();
        for _ in 0..8 {
            large.new_node_raw(NodeType::NodeText);
        }
        let small_root = small.root_raw();
        let small_owner = small_root.owner();
        large.root_raw().append_child(small_root);
        let later = small.new_node_raw(NodeType::NodeText);
        later.set_tokens(b"later".to_vec());
        small_root.append_child(later);
        let root_weak = Rc::downgrade(&later.owner().0);
        drop(large);
        drop(small);
        assert_eq!("later", small_root.text());
        assert_eq!(Some(later), small_root.first_child());
        drop(small_owner);
        assert!(root_weak.upgrade().is_none());
    }

    #[test]
    fn append_and_navigate() {
        let (_tree, root, p1, p2) = build_tree();
        assert_eq!(Some(p1), root.first_child());
        assert_eq!(Some(p2), root.last_child());
        assert_eq!(Some(p2), p1.next());
        assert_eq!(Some(p1), p2.previous());
        assert_eq!(Some(root), p1.parent());
        assert_eq!(2, root.children().count());
    }

    #[test]
    fn insert_before_and_after() {
        let (mut tree, root, p1, p2) = build_tree();
        let mid = tree.new_node_raw(NodeType::NodeHeading);
        p2.insert_before(mid);
        assert_eq!(Some(mid), p1.next());
        assert_eq!(Some(p2), mid.next());

        let tail = tree.new_node_raw(NodeType::NodeThematicBreak);
        p2.insert_after(tail);
        assert_eq!(Some(tail), root.last_child());
    }

    #[test]
    fn unlink_repairs_neighbors() {
        let (_tree, root, p1, p2) = build_tree();
        p1.unlink();
        assert_eq!(Some(p2), root.first_child());
        assert_eq!(None, p2.previous());
        assert_eq!(None, p1.parent());
        p2.unlink();
        assert_eq!(None, root.first_child());
        assert_eq!(None, root.last_child());
    }

    #[test]
    fn text_and_content() {
        let (_tree, _root, p1, _p2) = build_tree();
        assert_eq!("hello", p1.text());
        assert_eq!(5, p1.text_len());
        assert_eq!("hello", p1.content());
    }

    #[test]
    fn content_separates_blocks_with_newline() {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        let p1 = tree.new_node_raw(NodeType::NodeParagraph);
        let t1 = tree.new_node_raw(NodeType::NodeText);
        t1.set_tokens("a".as_bytes().to_vec());
        p1.append_child(t1);
        let p2 = tree.new_node_raw(NodeType::NodeParagraph);
        let t2 = tree.new_node_raw(NodeType::NodeText);
        t2.set_tokens("b".as_bytes().to_vec());
        p2.append_child(t2);
        root.append_child(p1);
        root.append_child(p2);
        // Go 版要求 buf 长度大于 1 且存在下下个兄弟节点，这里刻意锁住该行为
        assert_eq!("ab", root.content());
    }

    #[test]
    fn ial_escapes_and_unescapes() {
        let (_tree, _root, p1, _p2) = build_tree();
        p1.set_ial_attr("id", "a\"b");
        assert_eq!("a\"b", p1.ial_attr("id"));
        p1.set_ial_attr("id", "c");
        assert_eq!(1, p1.ial_raw().len());
        assert_eq!("c", p1.ial_attr("id"));
        p1.set_ial_attr("custom-x", "1");
        p1.remove_ial_attrs_by_prefix("custom-");
        assert_eq!(1, p1.ial_raw().len());
        p1.remove_ial_attr("id");
        assert!(p1.ial_raw().is_empty());
    }

    #[test]
    fn task_marker() {
        let (mut tree, _root, _p1, _p2) = build_tree();
        let li = tree.new_node_raw(NodeType::NodeListItem);
        assert_eq!(" ", li.effective_task_list_item_marker());
        li.data_mut().task_list_item_checked = true;
        assert_eq!("X", li.effective_task_list_item_marker());
        li.data_mut().task_list_item_checked = false;
        li.data_mut().task_list_item_marker = b'/';
        assert_eq!("/", li.effective_task_list_item_marker());
    }

    #[test]
    fn block_classification() {
        let (mut tree, _root, _p1, _p2) = build_tree();
        let heading = tree.new_node_raw(NodeType::NodeHeading);
        let marker = tree.new_node_raw(NodeType::NodeHeadingC8hMarker);
        let text = tree.new_node_raw(NodeType::NodeText);
        assert!(heading.is_block());
        assert!(!heading.is_container_block());
        assert!(marker.is_marker());
        assert!(!text.is_block());
        assert!(!text.is_marker());
    }

    #[test]
    fn node_id_is_stable_in_testing_mode() {
        TESTING.store(true, Ordering::Relaxed);
        assert_eq!(TESTING_NODE_ID, new_node_id());
        TESTING.store(false, Ordering::Relaxed);
        let id = new_node_id();
        assert_eq!(22, id.len());
        assert_eq!(b'-', id.as_bytes()[14]);
        assert!(id[15..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    }

    #[test]
    fn civil_conversion() {
        // 2006-01-01 对应 Unix day 13149
        assert_eq!((2006, 1, 1), civil_from_days(13149));
        assert_eq!((2006, 1, 2), civil_from_days(13150));
        assert_eq!((1970, 1, 1), civil_from_days(0));
    }
}
