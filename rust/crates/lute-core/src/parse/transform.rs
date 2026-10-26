// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 行级节点与文本标记之间的转换，对应 Go 版 parse/transform.go。
//!
//! - [`nested_inlines2flatted_spans_hybrid`]：把嵌套的行级节点（代码、链接、强调等）
//!   拆成平铺的 `NodeTextMark`，保留转义节点；
//! - [`text_marks2inlines`]：把可逆的文本标记还原为带标记符的行级节点。
//!
//! Protyle 模式依赖这两个转换：基准实现在解析阶段就把行内内容转义并写入
//! `TextMarkTextContent`，渲染阶段直接输出，因此转义、零宽空格等细节都必须在这里对齐。

use crate::ast::{self, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::html;
use crate::options::ParseOptions;

fn merge_text_mark_types(groups: &[&[&str]]) -> String {
    let mut merged: Vec<&str> = Vec::new();
    for group in groups {
        for typ in *group {
            if typ.is_empty() || "text" == *typ {
                continue;
            }
            if !merged.contains(typ) {
                merged.push(typ);
            }
        }
    }
    merged.join(" ")
}

fn escape_text_mark_content(tokens: &[u8]) -> String {
    let content = html::escape_html(&String::from_utf8_lossy(tokens));
    // 粘贴 Markdown 时行级元素中的双引号不再转换为实体
    // https://github.com/siyuan-note/siyuan/issues/14503
    content.replace("&quot;", "\"")
}

/// 对应 Go 版 processNestedNode。
fn process_nested_node(
    n: Node,
    tag: &str,
    tags: &mut Vec<String>,
    unlinks: &mut Vec<Node>,
    entering: bool,
) {
    if entering {
        tags.push(tag.to_string());
        return;
    }

    if !tags.is_empty() {
        tags.pop();
    }
    unlinks.push(n);

    let mut c = n.first_child();
    while let Some(cur) = c {
        let next = cur.next();
        if matches!(
            cur.typ(),
            NodeType::NodeTextMark | NodeType::NodeText | NodeType::NodeImage
        ) {
            n.insert_before(cur);
            if cur.ial_attr("style").is_empty() {
                cur.data_mut().kramdown_ial = n.data().kramdown_ial.clone();
            } else {
                // 平铺嵌套格式时保留文本自身的样式，并补齐祖先上的其他属性。
                for (name, value) in n.data().kramdown_ial.clone() {
                    if cur.ial_attr(&name).is_empty() {
                        cur.set_ial_attr(&name, &value);
                    }
                }
            }
            if NodeType::NodeLink == n.typ() && NodeType::NodeTextMark == cur.typ() {
                if let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) {
                    cur.data_mut().text_mark_a_href = dest.tokens_str();
                }
                if let Some(title) = n.child_by_type(NodeType::NodeLinkTitle) {
                    cur.data_mut().text_mark_a_title = title.tokens_str();
                }
            }
        } else if NodeType::NodeLinkDest == cur.typ() {
            if let Some(prev) = n.previous() {
                if NodeType::NodeTextMark == prev.typ() {
                    prev.data_mut().text_mark_a_href = cur.tokens_str();
                }
            }
        }
        c = next;
    }
}

/// 对应 Go 版 parse.NestedInlines2FlattedSpansHybrid。
pub fn nested_inlines2flatted_spans_hybrid(tree: &mut Tree, is_export_md: bool) {
    nested_inlines2flatted_spans_impl(tree, is_export_md, true);
}

/// 对应 Go 版 parse.NestedInlines2FlattedSpans，移除表格外的转义节点。
pub fn nested_inlines2flatted_spans(tree: &mut Tree, is_export_md: bool) {
    nested_inlines2flatted_spans_impl(tree, is_export_md, false);
}

fn nested_inlines2flatted_spans_impl(tree: &mut Tree, is_export_md: bool, hybrid: bool) {
    let root = tree.root_raw();
    nested_inlines2flatted_spans_at(tree, root, is_export_md, hybrid);
}

pub(crate) fn nested_inlines2flatted_spans_hybrid_at(
    tree: &mut Tree,
    root: Node,
    is_export_md: bool,
) {
    nested_inlines2flatted_spans_at(tree, root, is_export_md, true);
}

fn nested_inlines2flatted_spans_at(tree: &mut Tree, root: Node, is_export_md: bool, hybrid: bool) {
    let mut unlinks: Vec<Node> = Vec::new();

    // 第一遍：整理超链接内部结构（转义节点、嵌套图片、空链接文本）。
    ast::walk_raw(root, &mut |n, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if !hybrid && NodeType::NodeBackslash == n.typ() {
            if !n.parent_is(&[NodeType::NodeTableCell]) {
                if let Some(content) = n.first_child() {
                    let tokens = html::unescape_html_str(&content.tokens_str()).into_bytes();
                    if let Some(previous) = n
                        .previous()
                        .filter(|p| matches!(p.typ(), NodeType::NodeText | NodeType::NodeLinkText))
                    {
                        previous.append_tokens(&tokens);
                    } else if let Some(next) = n
                        .next()
                        .filter(|p| matches!(p.typ(), NodeType::NodeText | NodeType::NodeLinkText))
                    {
                        next.prepend_tokens(&tokens);
                    } else {
                        let text = tree.new_node_raw(NodeType::NodeText);
                        text.set_tokens(tokens);
                        n.insert_before(text);
                    }
                    unlinks.push(n);
                }
            }
            return WalkStatus::WalkContinue;
        }
        if NodeType::NodeLink != n.typ() {
            return WalkStatus::WalkContinue;
        }

        let mut backslashes: Vec<Node> = Vec::new();
        ast::walk_raw(n, &mut |c, entering| {
            if !hybrid || !entering || NodeType::NodeBackslash != c.typ() {
                return WalkStatus::WalkContinue;
            }
            if let Some(cont) = c.child_by_type(NodeType::NodeBackslashContent) {
                let link_text = tree.new_node_raw(NodeType::NodeLinkText);
                link_text.set_tokens(cont.tokens_vec());
                c.insert_before(link_text);
            }
            backslashes.push(c);
            WalkStatus::WalkContinue
        });
        for backslash in backslashes {
            backslash.unlink();
        }

        // 超链接嵌套图片情况下，图片子节点移到超链接节点前面
        let Some(img) = n.child_by_type(NodeType::NodeImage) else {
            return WalkStatus::WalkContinue;
        };
        n.insert_before(img);
        let link_text = n.child_by_type(NodeType::NodeLinkText);
        let empty_link_text = match link_text {
            None => true,
            Some(t) => is_blank(t.tokens()),
        };
        if empty_link_text {
            if let Some(open_bracket) = n.child_by_type(NodeType::NodeOpenBracket) {
                if let Some(dest) = n.child_by_type(NodeType::NodeLinkDest) {
                    let text = tree.new_node_raw(NodeType::NodeLinkText);
                    text.set_tokens(dest.tokens_vec());
                    open_bracket.insert_after(text);
                }
            }
        }
        WalkStatus::WalkContinue
    });

    for n in unlinks.drain(..) {
        n.unlink();
    }

    // 第二遍：把嵌套的行级节点展开为平铺的文本标记。
    let mut tags: Vec<String> = Vec::new();
    let mut span: Option<Node> = None;
    ast::walk_raw(root, &mut |n, entering| {
        match n.typ() {
            NodeType::NodeBackslash if !hybrid => {
                if !n.parent_is(&[NodeType::NodeTableCell]) {
                    unlinks.push(n);
                }
            }
            NodeType::NodeCodeSpan => {
                process_nested_node(n, "code", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeTag => process_nested_node(n, "tag", &mut tags, &mut unlinks, entering),
            NodeType::NodeInlineMath => {
                process_nested_node(n, "inline-math", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeEmphasis => {
                process_nested_node(n, "em", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeStrong => {
                process_nested_node(n, "strong", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeStrikethrough => {
                process_nested_node(n, "s", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeMark => process_nested_node(n, "mark", &mut tags, &mut unlinks, entering),
            NodeType::NodeUnderline => {
                process_nested_node(n, "u", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeSub => process_nested_node(n, "sub", &mut tags, &mut unlinks, entering),
            NodeType::NodeSup => process_nested_node(n, "sup", &mut tags, &mut unlinks, entering),
            NodeType::NodeKbd => process_nested_node(n, "kbd", &mut tags, &mut unlinks, entering),
            NodeType::NodeLink => process_nested_node(n, "a", &mut tags, &mut unlinks, entering),
            NodeType::NodeBlockRef => {
                process_nested_node(n, "block-ref", &mut tags, &mut unlinks, entering)
            }
            NodeType::NodeText
            | NodeType::NodeCodeSpanContent
            | NodeType::NodeInlineMathContent
            | NodeType::NodeLinkText
            | NodeType::NodeBlockRefID
            | NodeType::NodeHTMLEntity
            | NodeType::NodeBackslash => {
                if tags.is_empty() {
                    return WalkStatus::WalkContinue;
                }
                if entering {
                    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                    let typ = merge_text_mark_types(&[&tag_refs]);
                    let sp = tree.new_node_raw(NodeType::NodeTextMark);
                    sp.data_mut().text_mark_type = typ;
                    sp.data_mut().text_mark_text_content = if hybrid {
                        escape_text_mark_content(n.tokens())
                    } else {
                        html::escape_html(&n.tokens_str())
                    };

                    if NodeType::NodeInlineMathContent == n.typ() {
                        sp.data_mut().text_mark_text_content.clear();
                        let mut math = html::escape_html(&String::from_utf8_lossy(n.tokens()));
                        if n.parent_is(&[NodeType::NodeTableCell]) && !is_export_md {
                            // Improve the handling of inline-math containing | in the table
                            // https://github.com/siyuan-note/siyuan/issues/9227
                            math = math.replace("\\|", "|");
                        }
                        sp.data_mut().text_mark_inline_math_content = math;
                    } else if NodeType::NodeBackslash == n.typ() {
                        if let Some(c) = n.child_by_type(NodeType::NodeBackslashContent) {
                            // Go 版此处只做 EscapeHTML，不做 &quot; 还原。
                            sp.data_mut().text_mark_text_content =
                                html::escape_html(&String::from_utf8_lossy(c.tokens()));
                        }
                    } else if NodeType::NodeBlockRefID == n.typ() {
                        let mut subtype = "s";
                        let mut text = n.tokens_str();
                        let mut ref_text = n
                            .parent()
                            .and_then(|p| p.child_by_type(NodeType::NodeBlockRefText));
                        if ref_text.is_none() {
                            ref_text = n
                                .parent()
                                .and_then(|p| p.child_by_type(NodeType::NodeBlockRefDynamicText));
                            subtype = "d";
                        }
                        if let Some(ref_text) = ref_text {
                            text = ref_text.tokens_str();
                        }
                        sp.data_mut().text_mark_block_ref_subtype = subtype.to_string();
                        sp.data_mut().text_mark_text_content = text;
                        sp.data_mut().text_mark_block_ref_id = n
                            .parent()
                            .and_then(|p| p.child_by_type(NodeType::NodeBlockRefID))
                            .map(|x| x.tokens_str())
                            .unwrap_or_default();
                    } else if n.parent_is(&[NodeType::NodeLink])
                        && !n.parent_is(&[NodeType::NodeImage])
                    {
                        if let Some(next) = n.next() {
                            if NodeType::NodeLinkText == next.typ() {
                                // 合并相邻的链接文本节点
                                next.prepend_tokens(n.tokens());
                                return WalkStatus::WalkContinue;
                            }
                        }

                        let mut link = None;
                        let mut p = n.parent();
                        while let Some(cur) = p {
                            if NodeType::NodeLink == cur.typ() {
                                link = Some(cur);
                                break;
                            }
                            p = cur.parent();
                        }
                        if let Some(link) = link {
                            if let Some(dest) = link.child_by_type(NodeType::NodeLinkDest) {
                                sp.data_mut().text_mark_a_href = dest.tokens_str();
                            }
                            if let Some(title) = link.child_by_type(NodeType::NodeLinkTitle) {
                                sp.data_mut().text_mark_a_title = title.tokens_str();
                            }
                        }
                    }
                    span = Some(sp);
                } else {
                    if let Some(next) = n.next() {
                        if NodeType::NodeLinkText == next.typ() {
                            return WalkStatus::WalkContinue;
                        }
                    }

                    let Some(sp) = span.take() else {
                        return WalkStatus::WalkContinue;
                    };
                    sp.data_mut().kramdown_ial = n
                        .parent()
                        .map(|p| p.data().kramdown_ial.clone())
                        .unwrap_or_default();
                    if n.is_marker() {
                        if let Some(parent) = n.parent() {
                            parent.insert_before(sp);
                        }
                    } else if hybrid
                        && n.parent().map(|p| p.typ()) == Some(NodeType::NodeHTMLTag)
                        && n.parent().and_then(|p| p.parent()).map(|p| p.typ())
                            == Some(NodeType::NodeLink)
                    {
                        // https://github.com/siyuan-note/siyuan/issues/14788#issuecomment-2882248074
                        if let Some(grand) = n.parent().and_then(|p| p.parent()) {
                            grand.insert_before(sp);
                        }
                    } else {
                        n.insert_before(sp);
                        if NodeType::NodeText == n.typ() {
                            unlinks.push(n);
                        }
                    }
                }
            }
            NodeType::NodeTextMark => {
                if tags.is_empty() {
                    return WalkStatus::WalkContinue;
                }
                if entering {
                    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                    let types: Vec<&str> = n.data().text_mark_type.split(' ').collect();
                    let merged = merge_text_mark_types(&[&tag_refs, &types]);
                    if !merged.is_empty() {
                        n.data_mut().text_mark_type = merged;
                    }
                }
            }
            _ => {}
        }
        WalkStatus::WalkContinue
    });

    for n in unlinks {
        n.unlink();
    }
}

/// 对应 Go 版 parse.TextMarks2Inlines。
pub fn text_marks2inlines(tree: &mut Tree, options: &ParseOptions) {
    text_marks2inlines_impl(tree, options, false);
}

/// 对应 Go 版 parse.TextMarks2InlinesWithNestedSyntax。
pub fn text_marks2inlines_with_nested_syntax(tree: &mut Tree, options: &ParseOptions) {
    super::inline::text_mark_delimiters::parse_text_mark_delimiters(tree, options);
    text_marks2inlines_impl(tree, options, true);
}

fn text_marks2inlines_impl(tree: &mut Tree, options: &ParseOptions, nested_syntax_only: bool) {
    let root = tree.root_raw();
    ast::walk_raw(root, &mut |n, entering| {
        if !entering
            || NodeType::NodeTextMark != n.typ()
            || !n.data().text_mark_flashcard_occlusion_id.is_empty()
        {
            return WalkStatus::WalkContinue;
        }

        let (node_type, open_type, close_type) = match n.data().text_mark_type.as_str() {
            "sup" => (
                NodeType::NodeSup,
                NodeType::NodeSupOpenMarker,
                NodeType::NodeSupCloseMarker,
            ),
            "sub" => (
                NodeType::NodeSub,
                NodeType::NodeSubOpenMarker,
                NodeType::NodeSubCloseMarker,
            ),
            "em" => (
                NodeType::NodeEmphasis,
                NodeType::NodeEmA6kOpenMarker,
                NodeType::NodeEmA6kCloseMarker,
            ),
            "strong" => (
                NodeType::NodeStrong,
                NodeType::NodeStrongA6kOpenMarker,
                NodeType::NodeStrongA6kCloseMarker,
            ),
            "mark" => (
                NodeType::NodeMark,
                NodeType::NodeMark2OpenMarker,
                NodeType::NodeMark2CloseMarker,
            ),
            "s" => (
                NodeType::NodeStrikethrough,
                NodeType::NodeStrikethrough2OpenMarker,
                NodeType::NodeStrikethrough2CloseMarker,
            ),
            _ => return WalkStatus::WalkContinue,
        };

        let content = n.data().text_mark_text_content.clone();
        let mut sub = crate::parse::inline("", content.as_bytes(), options.clone());
        tree.adopt(&mut sub);
        let mut nodes: Vec<Node> = Vec::new();
        let mut has_nested = false;
        let mut can_flatten = true;
        if let Some(p) = sub.root_raw().first_child() {
            let mut c = p.first_child();
            while let Some(cur) = c {
                nodes.push(cur);
                match cur.typ() {
                    NodeType::NodeCodeSpan
                    | NodeType::NodeTag
                    | NodeType::NodeInlineMath
                    | NodeType::NodeEmphasis
                    | NodeType::NodeStrong
                    | NodeType::NodeStrikethrough
                    | NodeType::NodeMark
                    | NodeType::NodeUnderline
                    | NodeType::NodeSub
                    | NodeType::NodeSup
                    | NodeType::NodeKbd
                    | NodeType::NodeLink
                    | NodeType::NodeBlockRef
                    | NodeType::NodeImage
                    | NodeType::NodeTextMark => has_nested = true,
                    NodeType::NodeText | NodeType::NodeHTMLEntity | NodeType::NodeBackslash => {}
                    _ => can_flatten = false,
                }
                c = cur.next();
            }
        }
        if nested_syntax_only && (!has_nested || !can_flatten) {
            return WalkStatus::WalkContinue;
        }

        n.set_typ(node_type);
        let open = tree.new_node_raw(open_type);
        n.prepend_child(open);
        for node in nodes {
            n.append_child(node);
        }
        let close = tree.new_node_raw(close_type);
        n.append_child(close);
        WalkStatus::WalkContinue
    });
}

/// 对应 Go 版 bytes.TrimSpace(...) 的判空语义（按 Unicode 空白判断）。
fn is_blank(tokens: &[u8]) -> bool {
    String::from_utf8_lossy(tokens).trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(tree: &mut Tree) -> Node {
        let paragraph = tree.new_node_raw(NodeType::NodeParagraph);
        tree.root_raw().append_child(paragraph);
        paragraph
    }

    fn escaped(tree: &mut Tree, parent: Node, content: &str) -> Node {
        let node = tree.new_node_raw(NodeType::NodeBackslash);
        let text = tree.new_node_raw(NodeType::NodeBackslashContent);
        text.set_tokens(content.as_bytes().to_vec());
        node.append_child(text);
        parent.append_child(node);
        node
    }

    #[test]
    fn nonhybrid_unescapes_and_merges_outside_tables_only() {
        let mut tree = Tree::new();
        let p = paragraph(&mut tree);
        let first = tree.new_node_raw(NodeType::NodeText);
        first.set_tokens(b"left".to_vec());
        p.append_child(first);
        escaped(&mut tree, p, "&amp;");
        let cell = tree.new_node_raw(NodeType::NodeTableCell);
        p.append_child(cell);
        let preserved = escaped(&mut tree, cell, "|");
        nested_inlines2flatted_spans(&mut tree, false);
        assert_eq!(first.tokens(), b"left&");
        assert_eq!(first.next(), Some(cell));
        assert_eq!(cell.first_child(), Some(preserved));
    }

    #[test]
    fn hybrid_and_export_preserve_distinct_quote_and_escape_semantics() {
        for hybrid in [true, false] {
            let mut tree = Tree::new();
            let p = paragraph(&mut tree);
            let strong = tree.new_node_raw(NodeType::NodeStrong);
            p.append_child(strong);
            let text = tree.new_node_raw(NodeType::NodeText);
            text.set_tokens(b"\"".to_vec());
            strong.append_child(text);
            escaped(&mut tree, strong, "*");
            nested_inlines2flatted_spans_impl(&mut tree, false, hybrid);
            let spans: Vec<_> = p.children().collect();
            assert_eq!(spans.len(), if hybrid { 2 } else { 1 });
            assert_eq!(spans[0].data().text_mark_type, "strong");
            assert_eq!(
                spans[0].data().text_mark_text_content,
                if hybrid { "\"" } else { "&quot;*" }
            );
            if hybrid {
                assert_eq!(spans[1].data().text_mark_text_content, "*");
            }
        }
    }

    #[test]
    fn flatten_preserves_child_style_and_fills_ancestor_attributes() {
        let mut tree = Tree::new();
        let p = paragraph(&mut tree);
        let link = tree.new_node_raw(NodeType::NodeLink);
        link.set_ial_attr("style", "color:red");
        link.set_ial_attr("custom", "parent");
        p.append_child(link);
        let mark = tree.new_node_raw(NodeType::NodeTextMark);
        mark.data_mut().text_mark_type = "text strong strong".into();
        mark.data_mut().text_mark_text_content = "content".into();
        mark.set_ial_attr("style", "color:blue");
        link.append_child(mark);
        let dest = tree.new_node_raw(NodeType::NodeLinkDest);
        dest.set_tokens(b"target".to_vec());
        link.append_child(dest);
        let title = tree.new_node_raw(NodeType::NodeLinkTitle);
        title.set_tokens(b"title".to_vec());
        link.append_child(title);
        nested_inlines2flatted_spans_hybrid(&mut tree, false);
        assert_eq!(p.first_child(), Some(mark));
        assert_eq!(mark.next(), None);
        assert_eq!(mark.data().text_mark_type, "a strong");
        assert_eq!(mark.ial_attr("style"), "color:blue");
        assert_eq!(mark.ial_attr("custom"), "parent");
        assert_eq!(mark.data().text_mark_a_href, "target");
        assert_eq!(mark.data().text_mark_a_title, "title");
    }

    #[test]
    fn nested_only_conversion_keeps_plain_and_occluded_marks() {
        for (content, occlusion, expected) in [
            ("plain", "", NodeType::NodeTextMark),
            ("*nested*", "", NodeType::NodeStrong),
            ("*nested*", "occlusion", NodeType::NodeTextMark),
        ] {
            let mut tree = Tree::new();
            let p = paragraph(&mut tree);
            let mark = tree.new_node_raw(NodeType::NodeTextMark);
            mark.data_mut().text_mark_type = "strong".into();
            mark.data_mut().text_mark_text_content = content.into();
            mark.data_mut().text_mark_flashcard_occlusion_id = occlusion.into();
            p.append_child(mark);
            text_marks2inlines_with_nested_syntax(&mut tree, &ParseOptions::default());
            assert_eq!(mark.typ(), expected, "{content} / {occlusion}");
            if expected == NodeType::NodeStrong {
                assert_eq!(
                    mark.first_child().unwrap().typ(),
                    NodeType::NodeStrongA6kOpenMarker
                );
                assert_eq!(
                    mark.last_child().unwrap().typ(),
                    NodeType::NodeStrongA6kCloseMarker
                );
                assert!(mark.children().any(|n| n.typ() == NodeType::NodeEmphasis));
            }
        }
    }
}
