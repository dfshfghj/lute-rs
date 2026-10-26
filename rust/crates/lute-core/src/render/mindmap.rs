// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! ECharts / KityMinder JSON 渲染器，对应 Go 版 render/echarts_json_renderer.go
//! 与 render/kityminder_json_renderer.go。

use crate::ast::{NodeOwner, NodeType, RawNode as Node, Tree, WalkStatus};

/// ECharts 思维导图 JSON 渲染器。
pub struct EChartsJsonRenderer {
    out: Vec<u8>,
    root: Node,
    tree_id: String,
    _owner: NodeOwner,
}

impl EChartsJsonRenderer {
    pub fn new(tree: &Tree) -> Self {
        Self {
            out: Vec::new(),
            root: tree.root_raw(),
            tree_id: tree.id.clone(),
            _owner: tree.root_raw().owner(),
        }
    }

    pub fn render(&mut self) -> Vec<u8> {
        let root = self.root;
        crate::ast::walk_raw(root, &mut |n, entering| self.render_node(n, entering));
        std::mem::take(&mut self.out)
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        match n.typ() {
            NodeType::NodeDocument => {
                if entering {
                    self.out.push(b'[');
                    self.open_obj();
                    self.val("Document", n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj(n);
                    self.out.push(b']');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeParagraph => self.container(n, entering, "Paragraph\np"),
            NodeType::NodeText => {
                if entering {
                    let text = n.tokens_str();
                    let mut i = 0;
                    let mut summary = String::new();
                    for r in text.chars() {
                        i += 1;
                        summary.push(r);
                        if 4 < i {
                            summary.push_str("...");
                            break;
                        }
                    }
                    self.open_obj();
                    self.val(&format!("Text\n{summary}"), n);
                    self.close_obj(n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeCodeSpan => {
                if entering {
                    self.leaf("Code Span\ncode", n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeCodeBlock => {
                if entering {
                    self.leaf("Code Block\npre.code", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeMathBlock => {
                if entering {
                    self.leaf("Math Block\ndiv", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeInlineMath => {
                if entering {
                    self.leaf("Inline Math\nspan", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeEmphasis => self.container(n, entering, "Emphasis\nem"),
            NodeType::NodeStrong => self.container(n, entering, "Strong\nstrong"),
            NodeType::NodeBlockquote => self.container(n, entering, "Blockquote\nblockquote"),
            NodeType::NodeHeading => {
                if entering {
                    self.open_obj();
                    let level = n.data().heading_level.clamp(1, 6);
                    self.val(&format!("Heading\nh{level}"), n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeList | NodeType::NodeMindmap => {
                if entering {
                    self.open_obj();
                    let ld = n.data().list_data.clone().unwrap_or_default();
                    let list = if 1 == ld.typ || (3 == ld.typ && 0 == ld.bullet_char) {
                        "ol"
                    } else {
                        "ul"
                    };
                    self.val(&format!("List\n{list}"), n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeListItem | NodeType::NodeMindmapItem => {
                if entering {
                    self.open_obj();
                    let marker = n
                        .data()
                        .list_data
                        .as_ref()
                        .map(|ld| String::from_utf8_lossy(&ld.marker).to_string())
                        .unwrap_or_default();
                    self.val(&format!("List Item\nli {marker}"), n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeThematicBreak => {
                if entering {
                    self.leaf("Thematic Break\nhr", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeHardBreak => {
                if entering {
                    self.leaf("Hard Break\nbr", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeSoftBreak => {
                if entering {
                    self.leaf("Soft Break\n", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeHTMLBlock => {
                if entering {
                    self.leaf("HTML Block\n", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeInlineHTML => {
                if entering {
                    self.leaf("Inline HTML\n", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeLink => self.container(n, entering, "Link\na"),
            NodeType::NodeImage => self.container(n, entering, "Image\nimg"),
            NodeType::NodeStrikethrough => {
                if entering {
                    self.leaf("Strikethrough\ndel", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeTaskListItemMarker => {
                if entering {
                    self.open_obj();
                    let marker = effective_task_marker(n);
                    self.val(&format!("Task List Item Marker\n[{marker}]"), n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeTable => self.container(n, entering, "Table\ntable"),
            NodeType::NodeTableHead => {
                if entering {
                    self.leaf("Table Head\nthead", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeTableRow => {
                if entering {
                    self.leaf("Table Row\ntr", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeTableCell => {
                if entering {
                    self.leaf("Table Cell\ntd", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeEmoji => WalkStatus::WalkContinue,
            NodeType::NodeEmojiUnicode => {
                if entering {
                    self.leaf("Emoji Unicode\n", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeEmojiImg => {
                if entering {
                    self.leaf("Emoji Img\n", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeEmojiAlias => WalkStatus::WalkSkipChildren,
            NodeType::NodeFootnotesDef => self.container(n, entering, "Footnotes Def\np"),
            NodeType::NodeFootnotesRef => {
                if entering {
                    self.leaf("Footnotes Ref\ndiv", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeToC => {
                if entering {
                    self.leaf("ToC\ndiv", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeBackslash => {
                if entering {
                    self.leaf("Blackslash\ndiv", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeBackslashContent => WalkStatus::WalkSkipChildren,
            NodeType::NodeHTMLEntity => {
                if entering {
                    self.leaf("HTML Entity\nspan", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeYamlFrontMatter => {
                if entering {
                    self.leaf("Front Matter\nYAML", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeBlockRef => {
                if entering {
                    self.leaf("BlockRef\n((id))", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeFileAnnotationRef => {
                if entering {
                    self.leaf("FileAnnotationRef\n<<id>>", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeMark => {
                if entering {
                    self.leaf("Mark\nmark", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeSup => {
                if entering {
                    self.leaf("Sup\nsup", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeSub => {
                if entering {
                    self.leaf("Sub\nsub", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeBlockQueryEmbed => {
                if entering {
                    self.leaf("BlockQueryEmbed\n{{script}}", n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeKramdownBlockIAL => {
                if entering {
                    if let Some(prev) = n.previous() {
                        let id = if super::is_doc_ial(n.tokens()) {
                            self.tree_id.clone()
                        } else {
                            node_id(prev)
                        };
                        self.leaf(&format!("Block IAL\n{{: {id}}}"), n);
                    }
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownSpanIAL => {
                if entering {
                    if let Some(prev) = n.previous() {
                        let id = node_id(prev);
                        self.leaf(&format!("Span IAL\n{{: {id}}}"), n);
                    }
                }
                WalkStatus::WalkContinue
            }
            _ => WalkStatus::WalkContinue,
        }
    }

    fn container(&mut self, n: Node, entering: bool, val: &str) -> WalkStatus {
        if entering {
            self.open_obj();
            self.val(val, n);
            self.open_children(n);
        } else {
            self.close_children(n);
            self.close_obj(n);
        }
        WalkStatus::WalkContinue
    }

    fn leaf(&mut self, val: &str, n: Node) {
        self.open_obj();
        self.val(val, n);
        self.close_obj(n);
    }

    fn val(&mut self, val: &str, _n: Node) {
        let val = val
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace(['"', '\''], "");
        self.out.extend_from_slice(b"\"name\":\"");
        self.out.extend_from_slice(val.as_bytes());
        self.out.push(b'"');
    }

    fn open_obj(&mut self) {
        self.out.push(b'{');
    }

    fn close_obj(&mut self, n: Node) {
        self.out.push(b'}');
        if !ignore(n.next()) {
            self.out.push(b',');
        }
    }

    fn open_children(&mut self, n: Node) {
        if n.first_child().is_some() {
            self.out.extend_from_slice(b",\"children\":[");
        }
    }

    fn close_children(&mut self, n: Node) {
        if n.first_child().is_some() {
            self.out.push(b']');
        }
    }
}

fn ignore(node: Option<Node>) -> bool {
    let n = match node {
        Some(n) => n,
        None => return true,
    };
    matches!(
        n.typ(),
        NodeType::NodeBlockquoteMarker
            | NodeType::NodeEmA6kOpenMarker
            | NodeType::NodeEmA6kCloseMarker
            | NodeType::NodeEmU8eOpenMarker
            | NodeType::NodeEmU8eCloseMarker
            | NodeType::NodeStrongA6kOpenMarker
            | NodeType::NodeStrongA6kCloseMarker
            | NodeType::NodeStrongU8eOpenMarker
            | NodeType::NodeStrongU8eCloseMarker
            | NodeType::NodeStrikethrough1OpenMarker
            | NodeType::NodeStrikethrough1CloseMarker
            | NodeType::NodeStrikethrough2OpenMarker
            | NodeType::NodeStrikethrough2CloseMarker
            | NodeType::NodeMathBlockOpenMarker
            | NodeType::NodeMathBlockContent
            | NodeType::NodeMathBlockCloseMarker
            | NodeType::NodeInlineMathOpenMarker
            | NodeType::NodeInlineMathContent
            | NodeType::NodeInlineMathCloseMarker
            | NodeType::NodeYamlFrontMatterOpenMarker
            | NodeType::NodeYamlFrontMatterCloseMarker
            | NodeType::NodeYamlFrontMatterContent
    )
}

fn effective_task_marker(n: Node) -> String {
    let d = n.data();
    if 0 != d.task_list_item_marker {
        if b'x' == d.task_list_item_marker {
            "X".to_string()
        } else {
            (d.task_list_item_marker as char).to_string()
        }
    } else if d.task_list_item_checked {
        "X".to_string()
    } else {
        " ".to_string()
    }
}

fn node_id(n: Node) -> String {
    let id = n.ial_attr("id");
    if id.is_empty() {
        crate::ast::new_node_id()
    } else {
        id
    }
}

/// KityMinder JSON 渲染器。
pub struct KityMinderJsonRenderer {
    out: Vec<u8>,
    root: Node,
    _owner: NodeOwner,
    name: String,
}

impl KityMinderJsonRenderer {
    pub fn new(tree: &Tree) -> Self {
        Self {
            out: Vec::new(),
            root: tree.root_raw(),
            _owner: tree.root_raw().owner(),
            name: tree.name.clone(),
        }
    }

    pub fn render(&mut self) -> Vec<u8> {
        let root = self.root;
        crate::ast::walk_raw(root, &mut |n, entering| self.render_node(n, entering));
        std::mem::take(&mut self.out)
    }

    fn render_node(&mut self, n: Node, entering: bool) -> WalkStatus {
        match n.typ() {
            NodeType::NodeDocument => {
                if entering {
                    self.out.push(b'{');
                    self.out.extend_from_slice(b"\"root\":");
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.out.push(b'}');
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeParagraph => {
                if let Some(gp) = n.parent().and_then(|p| p.parent()) {
                    if NodeType::NodeList == gp.typ()
                        && gp
                            .data()
                            .list_data
                            .as_ref()
                            .map(|d| d.tight)
                            .unwrap_or(false)
                    {
                        if let Some(parent) = n.parent() {
                            if parent.first_child() == Some(n) && parent.last_child() == Some(n) {
                                return WalkStatus::WalkContinue;
                            }
                        }
                    }
                }
                if entering {
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.comma(n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeCodeBlock
            | NodeType::NodeMathBlock
            | NodeType::NodeHTMLBlock
            | NodeType::NodeToC
            | NodeType::NodeYamlFrontMatter
            | NodeType::NodeBlockQueryEmbed
            | NodeType::NodeThematicBreak => {
                if entering {
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.comma(n);
                }
                WalkStatus::WalkSkipChildren
            }
            NodeType::NodeTable => {
                if entering {
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.comma(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeBlockquote
            | NodeType::NodeSuperBlock
            | NodeType::NodeList
            | NodeType::NodeMindmap
            | NodeType::NodeListItem
            | NodeType::NodeMindmapItem => {
                if entering {
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.comma(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeHeading => {
                if entering {
                    self.open_obj();
                    self.data(n);
                    self.open_children(n);

                    // 与 Go 版保持一致：unlink 会把 c.Next 置空，循环因此只移除第一个子节点。
                    let mut child = n.first_child();
                    while let Some(c) = child {
                        c.unlink();
                        child = c.next();
                    }
                    for c in heading_children(n) {
                        n.append_child(c);
                    }
                } else {
                    self.close_children(n);
                    self.close_obj();
                    self.comma(n);
                }
                WalkStatus::WalkContinue
            }
            NodeType::NodeKramdownBlockIAL => WalkStatus::WalkSkipChildren,
            _ => WalkStatus::WalkContinue,
        }
    }

    fn data(&mut self, n: Node) {
        self.out.extend_from_slice(b"\"data\":");
        self.open_obj();

        let mut text = String::new();
        match n.typ() {
            NodeType::NodeDocument => text = self.name.clone(),
            NodeType::NodeList => {
                let typ = n.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0);
                if 0 == typ {
                    self.out.extend_from_slice(b"\"priority\": \"iconList\",");
                } else if 1 == typ {
                    self.out
                        .extend_from_slice(b"\"priority\": \"iconOrderedList\",");
                } else {
                    self.out.extend_from_slice(b"\"priority\": \"iconCheck\",");
                }
            }
            NodeType::NodeBlockquote => self.out.extend_from_slice(b"\"priority\": \"iconQuote\","),
            NodeType::NodeSuperBlock => self.out.extend_from_slice(b"\"priority\": \"iconSuper\","),
            _ => {
                let mut buf: Vec<u8> = Vec::new();
                crate::ast::walk_raw(n, &mut |cn, entering| {
                    if !entering {
                        return WalkStatus::WalkContinue;
                    }
                    if NodeType::NodeTag == cn.typ() {
                        buf.extend_from_slice(b"#");
                        buf.extend_from_slice(cn.text().as_bytes());
                        buf.extend_from_slice(b"#");
                        return WalkStatus::WalkSkipChildren;
                    }
                    if matches!(
                        cn.typ(),
                        NodeType::NodeText
                            | NodeType::NodeLinkText
                            | NodeType::NodeBlockRefText
                            | NodeType::NodeBlockRefDynamicText
                            | NodeType::NodeCodeSpanContent
                            | NodeType::NodeCodeBlockCode
                            | NodeType::NodeLinkTitle
                            | NodeType::NodeMathBlockContent
                            | NodeType::NodeInlineMathContent
                            | NodeType::NodeYamlFrontMatterContent
                    ) {
                        buf.extend_from_slice(cn.tokens());
                    }
                    WalkStatus::WalkContinue
                });
                text = String::from_utf8_lossy(&buf).to_string();
            }
        }

        let mut cleaned = String::with_capacity(text.len());
        for c in text.chars() {
            if matches!(c, '\\' | '\n' | '"' | '\t' | '\'') {
                continue;
            }
            cleaned.push(c);
        }
        if 16 < cleaned.chars().count() {
            let truncated: String = cleaned.chars().take(16).collect();
            cleaned = format!("{truncated}...");
        }
        if NodeType::NodeDocument == n.typ() {
            self.out.extend_from_slice(b"\"layout\":\"right\",");
        }
        self.out.extend_from_slice(b"\"text\":\"");
        self.out.extend_from_slice(cleaned.as_bytes());
        self.out.extend_from_slice(b"\",");
        self.out.extend_from_slice(b"\"id\":\"");
        self.out.extend_from_slice(n.ial_attr("id").as_bytes());
        self.out.extend_from_slice(b"\",");
        self.out.extend_from_slice(b"\"type\":\"");
        self.out.extend_from_slice(n.typ().as_str().as_bytes());
        self.out.extend_from_slice(b"\",");
        self.out.extend_from_slice(b"\"isContainer\":");
        self.out.extend_from_slice(if n.is_container_block() {
            b"true"
        } else {
            b"false"
        });
        self.close_obj();
    }

    fn open_obj(&mut self) {
        self.out.push(b'{');
    }

    fn close_obj(&mut self) {
        self.out.push(b'}');
    }

    fn open_children(&mut self, n: Node) {
        if n.first_child().is_some() {
            self.out.extend_from_slice(b",\"children\":[");
        }
    }

    fn close_children(&mut self, n: Node) {
        if n.first_child().is_some() {
            self.out.push(b']');
        }
    }

    fn comma(&mut self, n: Node) {
        if let Some(mut next) = n.next() {
            while NodeType::NodeKramdownBlockIAL == next.typ() {
                match next.next() {
                    Some(x) => next = x,
                    None => return,
                }
            }
            if next.is_block() {
                self.out.push(b',');
            }
        }
    }
}

fn heading_children(heading: Node) -> Vec<Node> {
    let mut start = heading;
    if let Some(next) = heading.next() {
        if NodeType::NodeKramdownBlockIAL == next.typ() {
            if let Some(n2) = next.next() {
                start = n2;
            }
        }
    }
    let current_level = heading.data().heading_level;
    let mut ret = Vec::new();
    // 注意：Go 版从 start 本身开始遍历（start 默认就是 heading），
    // 因此没有 IAL 时首轮即命中 break，返回空列表。这里保持逐字节一致。
    let mut n = Some(start);
    while let Some(cur) = n {
        if NodeType::NodeHeading == cur.typ() && current_level >= cur.data().heading_level {
            break;
        }
        if NodeType::NodeKramdownBlockIAL == cur.typ() {
            if !cur.tokens().windows(10).any(|w| w == b"type=\"doc\"") {
                ret.push(cur);
            }
        } else {
            ret.push(cur);
        }
        n = cur.next();
    }
    ret
}

/// 对应 Go 版 render.EChartsMindmapStr。
pub fn echarts_mindmap_str(list_content: &str) -> String {
    let content = list_content.replace(crate::editor::CARET, "");
    let tree = crate::parse::parse("", content.as_bytes(), crate::options::ParseOptions::new());
    let root = tree.root_raw();
    if root.first_child().is_none() || NodeType::NodeList != root.first_child().unwrap().typ() {
        return "{}".to_string();
    }

    // 移除非列表节点
    let mut to_remove = Vec::new();
    let mut c = root.first_child();
    while let Some(cur) = c {
        if NodeType::NodeList != cur.typ() {
            to_remove.push(cur);
        }
        c = cur.next();
    }
    for n in to_remove {
        n.unlink();
    }

    let mut out = String::new();
    crate::ast::walk_raw(root, &mut |n, entering| {
        match n.typ() {
            NodeType::NodeDocument => {
                if entering {
                    if need_root(root) {
                        out.push_str("{\"name\": \"Root\", \"children\": [");
                    }
                } else if need_root(root) {
                    out.push_str("]}");
                }
            }
            NodeType::NodeList => {}
            NodeType::NodeListItem => {
                let children = n.child_by_type(NodeType::NodeList).is_some();
                if entering {
                    let first = n.first_child();
                    let text = first.map(mindmap_text).unwrap_or_default();
                    out.push_str("{\"name\": \"");
                    out.push_str(&text);
                    out.push('"');
                    if children {
                        out.push_str(", \"children\": [");
                    }
                } else {
                    if children {
                        out.push(']');
                    }
                    out.push('}');
                    let parent_next = n.parent().and_then(|p| p.next()).is_some();
                    if n.next().is_some() || parent_next {
                        out.push_str(", ");
                    }
                }
            }
            _ => {}
        }
        WalkStatus::WalkContinue
    });
    out
}

fn mindmap_text(node: Node) -> String {
    let mut buf: Vec<u8> = Vec::new();
    crate::ast::walk_raw(node, &mut |n, entering| {
        if NodeType::NodeList == n.typ() || NodeType::NodeListItem == n.typ() {
            return WalkStatus::WalkContinue;
        }
        if entering && (NodeType::NodeText == n.typ() || NodeType::NodeLinkText == n.typ()) {
            buf.extend_from_slice(n.tokens());
        }
        WalkStatus::WalkContinue
    });
    let text = String::from_utf8_lossy(&buf).to_string();
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(crate::editor::CARET, "")
}

fn need_root(root: Node) -> bool {
    let mut count = 0;
    let mut c = root.first_child();
    while let Some(cur) = c {
        if NodeType::NodeList == cur.typ() {
            count += 1;
        }
        c = cur.next();
    }
    if 1 < count {
        return true;
    }
    if 0 == count {
        return true;
    }

    let mut count = 0;
    if let Some(list) = root.first_child() {
        let mut c = list.first_child();
        while let Some(cur) = c {
            if NodeType::NodeListItem == cur.typ() {
                count += 1;
            }
            c = cur.next();
        }
    }
    1 < count
}

#[cfg(test)]
mod source_ial_tests {
    use super::EChartsJsonRenderer;
    use crate::ast::{NodeType, Tree};

    #[test]
    fn document_ial_uses_tree_id_while_block_ial_uses_previous_node_id() {
        let mut tree = Tree::new();
        tree.id = "document-id".into();
        let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
        paragraph.set_id("paragraph-id");
        paragraph.set_ial_attr("id", "paragraph-id");
        tree.root_raw().append_child(paragraph);
        let block_ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        block_ial.set_tokens(br#"{: id="paragraph-id"}"#.to_vec());
        tree.root_raw().append_child(block_ial);
        let doc_ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        doc_ial.set_tokens(br#"{: id="document-id" type="doc"}"#.to_vec());
        tree.root_raw().append_child(doc_ial);
        let output = String::from_utf8(EChartsJsonRenderer::new(&tree).render()).unwrap();
        assert!(
            output.contains(r#""name":"Block IAL\n{: paragraph-id}""#),
            "{output}"
        );
        assert!(
            output.contains(r#""name":"Block IAL\n{: document-id}""#),
            "{output}"
        );
    }
}
