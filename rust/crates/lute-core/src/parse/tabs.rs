//! Tabs fences, item ownership and finalization, aligned with Go parse/tabs.go.
use super::{Parser, BLOCK_START_CLOSED};
use crate::{
    ast::{new_node_id, NodeType, RawNode as Node},
    lex,
};

#[derive(Default)]
pub(super) struct TabsParseState {
    fence_len: usize,
    indent: usize,
    active: Option<Node>,
}

fn tabs_container(mut container: Option<Node>) -> Option<Node> {
    while let Some(n) = container {
        match n.typ() {
            NodeType::NodeTabs => return Some(n),
            NodeType::NodeParagraph
            | NodeType::NodeTabItem
            | NodeType::NodeList
            | NodeType::NodeKramdownBlockIAL => container = n.parent(),
            _ => return None,
        }
    }
    None
}

pub(super) fn tabs_start(p: &mut Parser, container: Node) -> i32 {
    if !p.options.tabs {
        return 0;
    }
    let line = lex::trim_whitespace(&p.ctx.current_line[p.ctx.next_nonspace..]).to_vec();
    let fence_len = line.iter().take_while(|&&c| c == b':').count();
    if fence_len >= 3 {
        if fence_len == line.len() {
            if p.ctx.indented {
                return 0;
            }
            let mut target = tabs_container(Some(container));
            while let Some(n) = target {
                if p.ctx
                    .tabs
                    .iter()
                    .any(|(tabs, state)| *tabs == n && state.fence_len == fence_len)
                {
                    p.close_unmatched_blocks();
                    while p.ctx.tip != Some(n) {
                        p.finalize(p.ctx.tip.expect("tabs tip"));
                    }
                    p.finalize(n);
                    return BLOCK_START_CLOSED;
                }
                target = tabs_container(n.parent());
            }
            return 0;
        }
        if matches!(line[fence_len], b' ' | b'\t')
            && lex::trim_whitespace(&line[fence_len..]) == b"tabs"
        {
            let mut nested = false;
            let mut parent = Some(container);
            while let Some(n) = parent {
                if let Some((_, state)) = p.ctx.tabs.iter().find(|(tabs, _)| *tabs == n) {
                    nested = true;
                    if fence_len >= state.fence_len {
                        return 0;
                    }
                }
                parent = n.parent();
            }
            if p.ctx.indented && !nested {
                return 0;
            }
            p.close_unmatched_blocks();
            let node = p.add_child(NodeType::NodeTabs);
            p.ctx.tabs.push((
                node,
                TabsParseState {
                    fence_len,
                    indent: p.ctx.indent,
                    active: None,
                },
            ));
            return BLOCK_START_CLOSED;
        }
        return 0;
    }
    if p.ctx.indented {
        return 0;
    }
    let active = line.starts_with(b"@tab:active");
    let marker: &[u8] = if active { b"@tab:active" } else { b"@tab" };
    if line != marker
        && !(line.starts_with(marker) && matches!(line.get(marker.len()), Some(b' ' | b'\t')))
    {
        return 0;
    }
    let Some(tabs) = tabs_container(Some(container)) else {
        return 0;
    };
    p.close_unmatched_blocks();
    while p.ctx.tip != Some(tabs) {
        p.finalize(p.ctx.tip.expect("tabs tip"));
    }
    let item = p.add_child(NodeType::NodeTabItem);
    item.data_mut().tab_item_title = String::from_utf8_lossy(&line[marker.len()..])
        .trim()
        .to_owned();
    p.ctx.tabs_item_ial = Some(item);
    if active {
        if let Some((_, state)) = p.ctx.tabs.iter_mut().find(|(n, _)| *n == tabs) {
            if state.active.is_none() {
                state.active = Some(item);
            }
        }
    }
    BLOCK_START_CLOSED
}

impl Parser<'_> {
    pub(super) fn tabs_continue(&mut self, n: Node) -> i32 {
        if let Some((_, state)) = self.ctx.tabs.iter().find(|(tabs, _)| *tabs == n) {
            self.ctx
                .advance_offset(state.indent.min(self.ctx.indent), true);
        }
        0
    }

    pub(super) fn final_parse_tabs(&mut self) {
        for (tabs, state) in &self.ctx.tabs {
            let active = state
                .active
                .or_else(|| {
                    tabs.children().find(|n| {
                        n.typ() == NodeType::NodeTabItem
                            && !n.id().is_empty()
                            && n.id() == tabs.ial_attr("tabs-active-id")
                    })
                })
                .or_else(|| tabs.children().find(|n| n.typ() == NodeType::NodeTabItem));
            let Some(active) = active else {
                continue;
            };
            if active.id().is_empty() {
                active.set_id(new_node_id());
                active.set_ial_attr("id", active.id());
            }
            tabs.set_ial_attr("tabs-active-id", active.id());
            if let Some(ial) = tabs
                .next()
                .filter(|n| n.typ() == NodeType::NodeKramdownBlockIAL)
            {
                ial.set_tokens(super::ial::ial_to_tokens(&tabs.ial_raw()));
            }
        }
    }

    pub(super) fn tabs_finalize(&mut self, node: Node) {
        if node.typ() == NodeType::NodeTabs {
            if node.children().any(|n| n.typ() == NodeType::NodeTabItem) {
                return;
            }
            let item = self.tree.new_node_raw(NodeType::NodeTabItem);
            item.data_mut().close = true;
            let paragraph = self.tree.new_node_raw(NodeType::NodeParagraph);
            paragraph.data_mut().close = true;
            item.append_child(paragraph);
            node.append_child(item);
            return;
        }
        if !node.ial_raw().is_empty()
            && node
                .next()
                .is_none_or(|n| n.typ() != NodeType::NodeKramdownBlockIAL)
        {
            let ial = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(super::ial::ial_to_tokens(&node.ial_raw()));
            ial.data_mut().close = true;
            node.insert_after(ial);
        }
        if node.children().any(|n| {
            n.is_block()
                && n.typ() != NodeType::NodeKramdownBlockIAL
                && !(n.typ() == NodeType::NodeParagraph
                    && n.ial_attr("tabs-title") == "true"
                    && node
                        .children()
                        .find(|n| n.typ() != NodeType::NodeKramdownBlockIAL)
                        == Some(n))
        }) {
            return;
        }
        let paragraph = self.tree.new_node_raw(NodeType::NodeParagraph);
        paragraph.data_mut().close = true;
        node.append_child(paragraph);
    }
}

#[cfg(test)]
mod go_tab_metadata_tests {
    use crate::{
        ast::{walk_raw, NodeType, WalkStatus},
        lute::Lute,
    };

    // Go: test/tabs_test.go::TestTabsRoundTrip (original direct title/identity fields).
    #[test]
    fn go_tabs_round_trip_retains_original_title_metadata() {
        let mut engine = Lute::new();
        engine.parse_options.tabs = true;
        engine.parse_options.kramdown_block_ial = true;
        engine.render_options.kramdown_block_ial = true;
        engine.parse_options.block_ref = true;
        engine.parse_options.super_block = true;
        engine.render_options.super_block = true;
        engine.parse_options.text_mark = true;
        engine.parse_options.protyle_wysiwyg = true;
        engine.render_options.protyle_wysiwyg = true;
        let md=":::: tabs\n@tab **First**\n{: id=\"20260905120000-item001\"}\n\nOne\n::: tabs\n@tab Nested\n```\n@tab literal\n:::\n```\n:::\n@tab:active Second\n{: id=\"20260905120000-item002\"}\n\nTwo\n::::\n{: id=\"20260905120000-tabs001\" tabs-active-id=\"20260905120000-item002\" tabs-position=\"left\"}\n";
        let mut dom = engine.md2blockdom(md, false);
        for round in 0..3 {
            let tree = engine.blockdom2tree(&dom);
            let mut tabs = Vec::new();
            let mut items = Vec::new();
            walk_raw(tree.root_raw(), &mut |node, entering| {
                if entering {
                    match node.typ() {
                        NodeType::NodeTabs => tabs.push(node),
                        NodeType::NodeTabItem => items.push(node),
                        _ => {}
                    }
                }
                WalkStatus::WalkContinue
            });
            assert_eq!((tabs.len(), items.len()), (2, 3), "round {round}: {dom}");
            assert!(
                items[0]
                    .data()
                    .tab_item_title
                    .contains("data-type=\"strong\">First"),
                "round {round}: {:?}",
                items[0].data().tab_item_title
            );
            assert_eq!(items[2].id(), "20260905120000-item002");
            assert_eq!(tabs[0].ial_attr("tabs-active-id"), items[2].id());
            assert_eq!(tabs[0].ial_attr("tabs-position"), "left");
            assert!(dom.contains("literal") && dom.contains("Two"), "{dom}");
            dom = engine.spin_blockdom(&dom);
        }
    }
}
