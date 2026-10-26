//! Public AST handles must retain their storage without help from callers.
use std::{cell::RefCell, rc::Rc};

use lute_core::{
    ast::{walk, Node, NodeType, Tree, WalkStatus},
    options::{ParseOptions, RenderOptions},
    parse::parse,
    render::{HtmlRenderer, Renderer},
};

fn require_owning_handles() {
    // Fail safely on the former Copy handle before exercising escaped nodes.
    assert!(std::mem::needs_drop::<Node>());
}

#[test]
fn root_children_and_parent_survive_tree_drop() {
    require_owning_handles();
    let (root, paragraph, text) = {
        let tree = parse("", b"retained", ParseOptions::new());
        let root = tree.root();
        let paragraph = root.first_child().unwrap();
        let text = paragraph.first_child().unwrap();
        (root, paragraph, text)
    };
    assert_eq!(root.typ(), NodeType::NodeDocument);
    assert_eq!(root.first_child().unwrap(), paragraph);
    assert_eq!(text.tokens(), b"retained");
    assert_eq!(text.parent().unwrap(), paragraph);
    assert_eq!(paragraph.parent().unwrap(), root);
    let clone = text.clone();
    drop(text);
    drop(root);
    drop(paragraph);
    assert_eq!(clone.parent().unwrap().content(), "retained");
}

#[test]
fn children_iterator_retains_storage_after_tree_and_node_drop() {
    require_owning_handles();
    let mut children = {
        let tree = parse("", b"first\n\nsecond", ParseOptions::new());
        tree.root().children()
    };
    assert_eq!(children.next().unwrap().content(), "first");
    let second = children.next().unwrap();
    drop(children);
    assert_eq!(second.content(), "second");
    assert_eq!(second.previous().unwrap().content(), "first");
}

#[test]
fn cross_tree_links_survive_every_tree_and_original_handle_drop() {
    require_owning_handles();
    let moved = {
        let source = parse("", b"moved", ParseOptions::new());
        let target = Tree::new();
        let paragraph = source.root().first_child().unwrap();
        target.root().append_child(&paragraph);
        paragraph.first_child().unwrap()
    };
    let paragraph = moved.parent().unwrap();
    assert_eq!(moved.tokens(), b"moved");
    let target_root = paragraph.parent().unwrap();
    assert_eq!(target_root.typ(), NodeType::NodeDocument);
    assert_eq!(target_root.first_child().unwrap(), paragraph);
    paragraph.unlink();
    drop(target_root);
    assert_eq!(moved.parent().unwrap().content(), "moved");
}

#[test]
fn every_link_operation_retains_nodes_from_multiple_arenas() {
    require_owning_handles();
    let root = {
        let mut target = Tree::new();
        let middle = target.new_node(NodeType::NodeText);
        middle.set_tokens(b"middle".to_vec());
        target.root().append_child(&middle);

        let mut first_tree = Tree::new();
        let first = first_tree.new_node(NodeType::NodeText);
        first.set_tokens(b"first".to_vec());
        middle.insert_before(&first);

        let mut last_tree = Tree::new();
        let last = last_tree.new_node(NodeType::NodeText);
        last.set_tokens(b"last".to_vec());
        middle.insert_after(&last);

        let mut prefix_tree = Tree::new();
        let prefix = prefix_tree.new_node(NodeType::NodeText);
        prefix.set_tokens(b"prefix".to_vec());
        target.root().prepend_child(&prefix);
        target.root()
    };
    assert_eq!(
        root.children().map(|n| n.tokens_str()).collect::<Vec<_>>(),
        ["prefix", "first", "middle", "last"]
    );
    let last = root.last_child().unwrap();
    drop(root);
    assert_eq!(last.previous().unwrap().tokens(), b"middle");
    assert_eq!(
        last.parent().unwrap().first_child().unwrap().tokens(),
        b"prefix"
    );
}

#[test]
fn traversal_callback_nodes_can_escape_the_tree() {
    require_owning_handles();
    let mut saved = Vec::new();
    {
        let tree = parse("", b"saved", ParseOptions::new());
        walk(&tree.root(), &mut |node, entering| {
            if entering {
                saved.push(node);
            }
            WalkStatus::WalkContinue
        });
    }
    assert_eq!(saved.len(), 3);
    assert_eq!(saved[2].tokens(), b"saved");
    assert_eq!(saved[2].parent().unwrap(), saved[1]);
}

#[test]
fn renderer_and_saved_callback_nodes_retain_the_tree() {
    require_owning_handles();
    let mut renderer = {
        let tree = parse("", b"rendered", ParseOptions::new());
        HtmlRenderer::new(&tree, RenderOptions::new())
    };
    let saved = Rc::new(RefCell::new(Vec::new()));
    let callback_saved = saved.clone();
    renderer.base.ext_renderer = Some(Rc::new(move |node, entering| {
        if entering {
            callback_saved.borrow_mut().push(node);
        }
        None
    }));
    assert_eq!(renderer.render(), b"<p>rendered</p>\n");
    drop(renderer);
    let nodes = saved.borrow();
    let text = nodes
        .iter()
        .find(|n| n.typ() == NodeType::NodeText)
        .unwrap();
    assert_eq!(text.tokens(), b"rendered");
    assert_eq!(text.parent().unwrap().content(), "rendered");
}

#[test]
fn id_and_tokens_are_snapshots_independent_of_mutation_and_storage() {
    require_owning_handles();
    let (id, tokens) = {
        let mut tree = Tree::new();
        let node = tree.new_node(NodeType::NodeText);
        node.set_id("original");
        node.set_tokens(b"original".to_vec());
        let alias = node.clone();
        let id = node.id();
        let tokens = node.tokens();
        alias.set_id("replacement".repeat(1024));
        alias.set_tokens(vec![b'x'; 8192]);
        assert_eq!(id, "original");
        assert_eq!(tokens, b"original");
        (id, tokens)
    };
    assert_eq!(id, "original");
    assert_eq!(tokens, b"original");
}

#[test]
fn debug_formatting_uses_snapshots_when_the_writer_mutates_a_node() {
    struct MutatingWriter {
        node: Node,
        output: String,
    }
    impl std::fmt::Write for MutatingWriter {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            if text == "\"" {
                self.node.set_id("changed".repeat(2048));
                self.node.set_tokens(vec![b'x'; 8192]);
            }
            self.output.push_str(text);
            Ok(())
        }
    }
    let node = {
        let mut tree = Tree::new();
        let node = tree.new_node(NodeType::NodeText);
        node.set_id("original");
        node
    };
    let mut writer = MutatingWriter {
        node: node.clone(),
        output: String::new(),
    };
    std::fmt::write(&mut writer, format_args!("{node:?}")).unwrap();
    assert_eq!(writer.output, "Node { type: NodeText, id: \"original\" }");
    assert_ne!(node.id(), "original");
}

#[test]
fn all_cached_root_renderers_retain_their_arena() {
    use lute_core::render::{
        format::FormatRenderer,
        mindmap::{EChartsJsonRenderer, KityMinderJsonRenderer},
        protyle::ProtyleRenderer,
        protyle_preview::ProtylePreviewRenderer,
        vditor::VditorRenderer,
        vditor_ir::VditorIRRenderer,
        vditor_sv::VditorSVRenderer,
    };
    macro_rules! capture {
        ($renderer:expr) => {{
            let mut renderer = $renderer;
            Box::new(move || renderer.render()) as Box<dyn FnMut() -> Vec<u8>>
        }};
    }
    let renderers: Vec<Box<dyn FnMut() -> Vec<u8>>> = {
        let tree = parse("", b"kept", ParseOptions::new());
        let render = RenderOptions::new();
        let parse = ParseOptions::new();
        vec![
            capture!(HtmlRenderer::new(&tree, render.clone())),
            capture!(FormatRenderer::new(&tree, render.clone(), parse.clone())),
            capture!(ProtyleRenderer::new(&tree, render.clone(), parse.clone())),
            capture!(ProtylePreviewRenderer::new(
                &tree,
                render.clone(),
                parse.clone(),
            )),
            capture!(VditorRenderer::new(&tree, render.clone(), parse.clone())),
            capture!(VditorIRRenderer::new(&tree, render.clone(), parse.clone())),
            capture!(VditorSVRenderer::new(&tree, render, parse)),
            capture!(EChartsJsonRenderer::new(&tree)),
            capture!(KityMinderJsonRenderer::new(&tree)),
        ]
    };
    for (index, mut renderer) in renderers.into_iter().enumerate() {
        let output = String::from_utf8(renderer()).unwrap();
        assert!(output.contains("kept"), "renderer {index}: {output}");
    }
}
