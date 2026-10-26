//! Pair delimiters across adjacent text/style nodes, preserving each segment's style.
//! Corresponds to Go parse/text_mark_delimiters.go and uses the existing emphasis parser.
use super::{InlineContext, InlineParser};
use crate::ast::{self, NodeType, RawNode as Node, Tree, WalkStatus};
use crate::options::ParseOptions;
use std::collections::HashMap;

pub(crate) fn parse_text_mark_delimiters(tree: &mut Tree, options: &ParseOptions) {
    let mut groups = Vec::new();
    ast::walk_raw(tree.root_raw(), &mut |node, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        let mut group = Vec::new();
        let mut child = node.first_child();
        while let Some(node) = child {
            if node.typ() == NodeType::NodeText || is_delimiter_text_mark(node) {
                group.push(node);
            } else {
                if group.len() > 1 {
                    groups.push(std::mem::take(&mut group));
                }
                group.clear();
            }
            child = node.next();
        }
        if group.len() > 1 {
            groups.push(group);
        }
        WalkStatus::WalkContinue
    });
    for group in groups {
        parse_group(tree, options, &group);
    }
}

fn is_delimiter_text_mark(node: Node) -> bool {
    node.typ() == NodeType::NodeTextMark
        && node.data().text_mark_flashcard_occlusion_id.is_empty()
        && !node.data().text_mark_type.is_empty()
        && node.data().text_mark_type.split_whitespace().all(|typ| {
            matches!(
                typ,
                "strong" | "em" | "s" | "mark" | "sup" | "sub" | "u" | "text"
            )
        })
}

fn parse_group(tree: &mut Tree, options: &ParseOptions, group: &[Node]) {
    let mut tokens = Vec::new();
    let mut owners = Vec::new();
    let mut has_style = false;
    for &node in group {
        let content = if node.typ() == NodeType::NodeTextMark {
            has_style = true;
            node.data().text_mark_text_content.as_bytes()
        } else {
            node.tokens()
        };
        if content.iter().any(|byte| b"\\`$&<>\n[]()".contains(byte)) {
            return;
        }
        tokens.extend_from_slice(content);
        owners.extend(std::iter::repeat_n(node, content.len()));
    }
    if !has_style || !tokens.iter().any(|byte| b"*_~=#^".contains(byte)) {
        return;
    }
    let block = tree.new_node_raw(NodeType::NodeParagraph);
    let mut parser = InlineParser::new(tree, options, false);
    let mut ctx = InlineContext::new(tokens);
    let mut markers = HashMap::new();
    while ctx.pos < ctx.tokens_len {
        let start = ctx.pos;
        if b"*_~=#^".contains(&ctx.tokens[start]) {
            parser.handle_delim(block, &mut ctx);
            if owners[start + 1..ctx.pos]
                .iter()
                .any(|owner| *owner != owners[start])
            {
                return;
            }
            if let Some(marker) = block.last_child() {
                markers.insert(marker, (owners[start], ctx.pos - start));
            }
        } else {
            ctx.pos += 1;
            while ctx.pos < ctx.tokens_len && !b"*_~=#^".contains(&ctx.tokens[ctx.pos]) {
                ctx.pos += 1;
            }
            let mut start = start;
            while start < ctx.pos {
                let mut next = start + 1;
                while next < ctx.pos && owners[next] == owners[start] {
                    next += 1;
                }
                let owner = owners[start];
                let node = parser.new_node_raw(owner.typ());
                if owner.typ() == NodeType::NodeTextMark {
                    node.data_mut().text_mark_type = owner.data().text_mark_type.clone();
                    node.data_mut().text_mark_text_content =
                        String::from_utf8_lossy(&ctx.tokens[start..next]).into_owned();
                    node.data_mut().kramdown_ial = owner.data().kramdown_ial.clone();
                } else {
                    node.set_tokens(ctx.tokens[start..next].to_vec());
                }
                block.append_child(node);
                start = next;
            }
        }
    }
    parser.process_emphasis(None, &mut ctx);
    let mut matched = false;
    for (marker, (owner, length)) in markers {
        if marker.parent().is_none() || marker.tokens().len() != length {
            matched = true;
        }
        if marker.parent().is_some() && owner.typ() == NodeType::NodeTextMark {
            marker.set_typ(NodeType::NodeTextMark);
            marker.data_mut().text_mark_type = owner.data().text_mark_type.clone();
            marker.data_mut().text_mark_text_content = marker.tokens_str();
            marker.data_mut().kramdown_ial = owner.data().kramdown_ial.clone();
            marker.clear_tokens();
        }
    }
    if !matched {
        return;
    }
    while let Some(child) = block.first_child() {
        group[0].insert_before(child);
    }
    for node in group {
        node.unlink();
    }
}
