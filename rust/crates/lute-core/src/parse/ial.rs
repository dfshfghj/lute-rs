// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Kramdown 块级内联属性列表（{: attrs}），对应 Go 版 parse/inline_attribute_list.go。

use crate::ast::{NodeType, RawNode as Node};
use crate::parse::inline::html::tag_attr;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 属性值中的换行转义，对应 Go 版 editor.IALValEscNewLine。
pub(crate) const IAL_VAL_ESC_NEWLINE: &str = "_esc_newline_";

/// 对应 Go 版 IALStart。
pub fn ial_start(p: &mut Parser, _container: Node) -> i32 {
    if !(crate::CAP_KRAMDOWN_BLOCK_IAL && p.options.kramdown_block_ial) || p.ctx.indented {
        return BLOCK_START_NONE;
    }

    if let Some(tip) = p.ctx.tip {
        if NodeType::NodeListItem == tip.typ() && tip.first_child().is_none() {
            // 在列表最终化过程中处理
            return BLOCK_START_NONE;
        }
    }

    let ial = match p.parse_kramdown_block_ial() {
        Some(ial) => ial,
        None => return BLOCK_START_NONE,
    };
    p.close_unmatched_blocks();
    // 整行过
    p.ctx.offset = p.ctx.current_line_len;

    if is_doc_ial2(&ial) {
        // 文档块 IAL
        let tokens = p.ctx.current_line[p.ctx.next_nonspace..].to_vec();
        let root_ial = p.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        root_ial.set_tokens(tokens);
        p.ctx.root_ial = Some(root_ial);
        let root = p.tree.root_raw();
        root.data_mut().kramdown_ial = ial.clone();
        let id = ial.first().map(|kv| kv.1.clone()).unwrap_or_default();
        root.set_id(id.clone());
        p.tree.id = id;
        return BLOCK_START_LEAF;
    }

    let mut last_matched = match p.ctx.last_matched_container {
        Some(n) => n,
        None => return BLOCK_START_NONE,
    };
    if p.ctx.all_closed
        && matches!(
            last_matched.typ(),
            NodeType::NodeDocument
                | NodeType::NodeListItem
                | NodeType::NodeBlockquote
                | NodeType::NodeCallout
                | NodeType::NodeSuperBlock
                | NodeType::NodeTabs
                | NodeType::NodeTabItem
        )
    {
        // 挂到最后一个子块上
        let tip = p.ctx.tip.expect("tip");
        last_matched = match tip.last_child() {
            Some(last) => last,
            None => p
                .ctx
                .last_matched_container
                .expect("last matched container"),
        };
        if matches!(
            last_matched.typ(),
            NodeType::NodeSuperBlockLayoutMarker | NodeType::NodeKramdownBlockIAL
        ) && last_matched.parent().is_some()
        {
            // 两个连续的 IAL
            let tokens = ial_to_tokens(&ial);
            if !last_matched.tokens().starts_with(&tokens) {
                // 有的块解析已经做过打断处理
                // 在两个连续的 IAL 之间插入空段落，这样能够保持空段落
                let para = p.tree.new_node_raw(NodeType::NodeParagraph);
                para.set_tokens(b" ".to_vec());
                last_matched.insert_after(para);
                p.ctx.tip = Some(para);
                last_matched = para;
            }
        } else if NodeType::NodeBlockquoteMarker == last_matched.typ() {
            // 引述块下没有段落子块，需要构建一个空的段落块挂上去
            let para = p.tree.new_node_raw(NodeType::NodeParagraph);
            para.set_tokens(b" ".to_vec());
            last_matched.insert_after(para);
            p.ctx.tip = Some(para);
            last_matched = para;
        } else if NodeType::NodeDocument == last_matched.typ()
            || (NodeType::NodeTabItem == last_matched.typ() && !last_matched.close())
        {
            // 文档或页签正文的第一个节点是 IAL 时，保留带有该属性的空段落。
            let para = p.tree.new_node_raw(NodeType::NodeParagraph);
            para.set_tokens(b" ".to_vec());
            last_matched.append_child(para);
            p.ctx.tip = Some(para);
            last_matched = para;
        }
    }

    last_matched.data_mut().kramdown_ial = ial.clone();
    let id = ial2_map_un_esc(&ial).get("id").cloned().unwrap_or_default();
    last_matched.set_id(id);
    let node = p.add_child(NodeType::NodeKramdownBlockIAL);
    node.set_tokens(p.ctx.current_line[p.ctx.next_nonspace..].to_vec());
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    /// 对应 Go 版 Tree.parseKramdownBlockIAL。
    pub(super) fn parse_kramdown_block_ial(&self) -> Option<Vec<(String, String)>> {
        let tokens = &self.ctx.current_line[self.ctx.next_nonspace..];
        parse_kramdown_block_ial_tokens(tokens)
    }
}

/// 对应 Go 版 Context.parseKramdownBlockIAL。
pub(crate) fn parse_kramdown_block_ial_tokens(tokens: &[u8]) -> Option<Vec<(String, String)>> {
    let start = find_sub(tokens, b"{:")?;
    if 0 != start {
        return None;
    }
    let tokens = &tokens[start + 2..];
    let end = rfind_sub(tokens, b"}")?;
    if 3 > end {
        return None;
    }
    if !tokens[end..].starts_with(b"}\n") {
        // IAL 后不能存在其他内容，必须独占一行
        return None;
    }
    Some(tokens2_ial(tokens))
}

/// 对应 Go 版 Tokens2IAL。
pub(crate) fn tokens2_ial(tokens: &[u8]) -> Vec<(String, String)> {
    // tokens 开头必须是空格
    let mut tokens = tokens.to_vec();
    while let Some(&last) = tokens.last() {
        if b' ' == last || b'\n' == last {
            tokens.pop();
        } else {
            break;
        }
    }
    if tokens.starts_with(b"{:") {
        tokens.drain(..2);
    }
    if tokens.ends_with(b"}") {
        tokens.pop();
    }
    let tokens = replace_bytes(&tokens, b"\n", IAL_VAL_ESC_NEWLINE.as_bytes());
    let mut ret = Vec::new();
    let mut remains: Vec<u8> = tokens;
    loop {
        let (valid, next, attr, name, val) = tag_attr(&remains);
        if !valid {
            break;
        }
        remains = next;
        if attr.is_empty() {
            break;
        }
        let val = replace_bytes(&val, IAL_VAL_ESC_NEWLINE.as_bytes(), b"\n");
        ret.push((
            String::from_utf8_lossy(&name).to_string(),
            String::from_utf8_lossy(&val).to_string(),
        ));
    }
    ret
}

/// 对应 Go 版 IAL2Tokens。
pub(crate) fn ial_to_tokens(ial: &[(String, String)]) -> Vec<u8> {
    let mut buf = b"{: ".to_vec();
    for (i, (k, v)) in ial.iter().enumerate() {
        buf.extend_from_slice(k.as_bytes());
        buf.extend_from_slice(b"=\"");
        buf.extend_from_slice(v.as_bytes());
        buf.push(b'"');
        if i < ial.len() - 1 {
            buf.push(b' ');
        }
    }
    buf.push(b'}');
    buf
}

/// 对应 Go 版 IAL2MapUnEsc。
pub(crate) fn ial2_map_un_esc(
    ial: &[(String, String)],
) -> std::collections::HashMap<String, String> {
    let mut ret = std::collections::HashMap::with_capacity(ial.len());
    for (k, v) in ial {
        ret.insert(k.clone(), v.clone());
    }
    ret
}

/// 对应 Go 版 util.IsDocIAL2。
pub(super) fn is_doc_ial2(ial: &[(String, String)]) -> bool {
    ial.iter().any(|(k, v)| "type" == k && "doc" == v)
}

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
}

fn rfind_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).rposition(|w| w == sub)
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

/// 对应 Go 版 Context.parseKramdownIALInListItem。
pub(crate) fn parse_kramdown_ial_in_list_item(tokens: &[u8]) -> Vec<(String, String)> {
    let mut ret = Vec::new();
    let start = match find_sub(tokens, b"{:") {
        Some(0) => 0,
        _ => return ret,
    };
    let tokens = &tokens[start + 2..];
    let end = match find_sub(tokens, b"}") {
        Some(end) if 3 <= end => end,
        _ => return ret,
    };
    let mut tokens = tokens[..end].to_vec();
    loop {
        let (valid, remains, attr, name, val) = tag_attr(&tokens);
        if !valid {
            break;
        }
        tokens = remains;
        if attr.is_empty() {
            break;
        }
        ret.push((
            String::from_utf8_lossy(&name).to_string(),
            String::from_utf8_lossy(&val).to_string(),
        ));
    }
    ret
}

/// 对应 Go 版 mergeIALPreservingOrder：保持属性顺序合并，重复 key 最后一个值生效。
pub(crate) fn merge_ial_preserving_order(
    dst: &[(String, String)],
    src: &[(String, String)],
) -> Vec<(String, String)> {
    let mut ret: Vec<(String, String)> = Vec::with_capacity(dst.len() + src.len());
    for (name, value) in dst {
        let value = crate::html::unescape_attr_val(value);
        if let Some(idx) = ret.iter().position(|(k, _)| k == name) {
            ret[idx].1 = value;
            continue;
        }
        ret.push((name.clone(), value));
    }
    for (name, value) in src {
        let value = crate::html::unescape_attr_val(value);
        if let Some(idx) = ret.iter().position(|(k, _)| k == name) {
            ret[idx].1 = value;
            continue;
        }
        ret.push((name.clone(), value));
    }
    ret
}

impl Parser<'_> {
    /// Complete block identities and document IAL after inline parsing (Go finalParseBlockIAL).
    pub(super) fn final_parse_block_ial(&mut self) {
        if !(crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial) {
            return;
        }
        let mut appends = Vec::new();
        crate::ast::walk_raw(self.tree.root_raw(), &mut |node, entering| {
            use crate::ast::{new_node_id, WalkStatus};
            if !entering || !node.is_block() || node.typ() == NodeType::NodeKramdownBlockIAL {
                return WalkStatus::WalkContinue;
            }
            if node.typ() == NodeType::NodeBlockquote
                && node.first_child().is_some_and(|n| n.next().is_none())
            {
                appends.push(node);
            }
            if node.id().is_empty() {
                let id = node.ial_attr("id");
                node.set_id(if id.is_empty() { new_node_id() } else { id });
                if (crate::PROTYLE && self.options.protyle_wysiwyg)
                    && self.options.spin
                    && node.typ() != NodeType::NodeDocument
                {
                    if let Some(next) = node
                        .next()
                        .filter(|n| n.typ() != NodeType::NodeKramdownBlockIAL && !n.id().is_empty())
                    {
                        node.set_id(next.id_owned());
                        node.data_mut().kramdown_ial = next.ial_raw();
                        if node.ial_attr("updated").is_empty() {
                            node.set_ial_attr("updated", timestamp(node.id()));
                        }
                        next.set_id(new_node_id());
                        next.data_mut().kramdown_ial.clear();
                        next.set_ial_attr("id", next.id());
                        next.set_ial_attr("updated", timestamp(next.id()));
                        if let Some(ial) = next
                            .next()
                            .filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL)
                        {
                            ial.set_tokens(ial_to_tokens(&next.ial_raw()));
                        }
                        let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
                        ial.set_tokens(ial_to_tokens(&node.ial_raw()));
                        node.insert_after(ial);
                        return WalkStatus::WalkContinue;
                    }
                }
            }
            if let Some(ial) = node
                .next()
                .filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL)
            {
                node.data_mut().kramdown_ial = tokens2_ial(ial.tokens());
                if (crate::PROTYLE && self.options.protyle_wysiwyg)
                    && node.ial_attr("updated").is_empty()
                {
                    node.set_ial_attr("updated", timestamp(node.id()));
                    ial.set_tokens(ial_to_tokens(&node.ial_raw()));
                }
            } else if crate::PROTYLE && self.options.protyle_wysiwyg {
                node.set_ial_attr("id", node.id());
                node.set_ial_attr("updated", timestamp(node.id()));
            }
            WalkStatus::WalkContinue
        });
        for node in appends {
            let id = crate::ast::new_node_id();
            let paragraph = self.tree.new_node_raw(NodeType::NodeParagraph);
            paragraph.set_id(id.clone());
            paragraph.data_mut().kramdown_ial = vec![
                ("id".to_owned(), id.clone()),
                ("updated".to_owned(), timestamp(&id).to_owned()),
            ];
            let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(format!("{{: id=\"{id}\"}}").into_bytes());
            paragraph.insert_after(ial);
            if node
                .next()
                .is_some_and(|n| n.typ() == NodeType::NodeKramdownBlockIAL)
                && node.typ() == NodeType::NodeBlockquote
                && node.first_child().is_some_and(|n| {
                    n.typ() == NodeType::NodeBlockquoteMarker && n.next().is_none()
                })
            {
                let text = self.tree.new_node_raw(NodeType::NodeText);
                text.set_tokens(crate::editor::CARET.as_bytes().to_vec());
                paragraph.append_child(text);
            }
            node.append_child(paragraph);
        }
        let doc_ial = self.ctx.root_ial.unwrap_or_else(|| {
            let id = crate::ast::new_node_id();
            let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(
                format!(
                    "{{: id=\"{id}\" updated=\"{}\" type=\"doc\"}}",
                    timestamp(&id)
                )
                .into_bytes(),
            );
            self.tree.root_raw().set_id(id.clone());
            self.tree.id = id;
            ial
        });
        self.tree.root_raw().append_child(doc_ial);
    }
}

// Editor IDs normally start with a 14-byte timestamp. Preserve short imported IDs
// without introducing Go's out-of-bounds panic into safe Rust parsing.
fn timestamp(id: &str) -> &str {
    id.get(..14).unwrap_or(id)
}

#[cfg(test)]
mod go_ial_tests {
    use super::merge_ial_preserving_order;

    fn attrs(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    /// Go parse/inline_attribute_list_test.go: TestMergeIALPreservingOrder.
    #[test]
    fn go_merge_ial_preserving_order() {
        for (name, dst, src, want) in [
            (
                "updates_existing_attrs_and_appends_new_attrs",
                attrs(&[("id", "old"), ("class", "foo&amp;bar")]),
                attrs(&[("id", "new"), ("updated", "20260405"), ("title", "a&amp;b")]),
                attrs(&[
                    ("id", "new"),
                    ("class", "foo&bar"),
                    ("updated", "20260405"),
                    ("title", "a&b"),
                ]),
            ),
            (
                "appends_new_attr_after_existing_block_attrs",
                attrs(&[
                    ("data-node-id", "20250824233004-3qfd5gf"),
                    ("data-node-index", "1"),
                    ("data-type", "NodeBlockquote"),
                    ("class", "bq"),
                    ("updated", "20250824233509"),
                ]),
                attrs(&[("custom-b", "info")]),
                attrs(&[
                    ("data-node-id", "20250824233004-3qfd5gf"),
                    ("data-node-index", "1"),
                    ("data-type", "NodeBlockquote"),
                    ("class", "bq"),
                    ("updated", "20250824233509"),
                    ("custom-b", "info"),
                ]),
            ),
            (
                "collapses_duplicate_attrs_with_stable_order",
                attrs(&[("id", "old"), ("id", "shadow"), ("class", "foo")]),
                attrs(&[("class", "bar"), ("id", "new")]),
                attrs(&[("id", "new"), ("class", "bar")]),
            ),
        ] {
            assert_eq!(merge_ial_preserving_order(&dst, &src), want, "{name}");
        }
    }

    /// Additional function boundary assertions: empty sides and repeated source keys.
    #[test]
    fn merge_ial_empty_inputs_and_repeated_source_keys() {
        assert!(merge_ial_preserving_order(&[], &[]).is_empty());
        let source = attrs(&[
            ("title", "&#34;x&#34;"),
            ("id", "1"),
            ("title", "&lt;final&gt;"),
        ]);
        let want = attrs(&[("title", "<final>"), ("id", "1")]);
        assert_eq!(merge_ial_preserving_order(&[], &source), want);
        assert_eq!(merge_ial_preserving_order(&source, &[]), want);
        assert_eq!(
            source[0].1, "&#34;x&#34;",
            "merging must not mutate caller attributes"
        );
    }
}
