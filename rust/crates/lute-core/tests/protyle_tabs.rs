use lute_core::ast::{NodeType, Tree};
use lute_core::options::{ParseOptions, RenderOptions};
use lute_core::render::{protyle::ProtyleRenderer, Renderer};

#[test]
fn protyle_tab_title_block_is_rendered_once_and_body_is_preserved() {
    let mut tree = Tree::new();
    let tabs = tree.new_node(NodeType::NodeTabs);
    let item = tree.new_node(NodeType::NodeTabItem);
    let title = tree.new_node(NodeType::NodeParagraph);
    title.set_ial_attr("tabs-title", "true");
    let title_text = tree.new_node(NodeType::NodeText);
    title_text.set_tokens(b"Title".to_vec());
    title.append_child(&title_text);
    let body = tree.new_node(NodeType::NodeParagraph);
    let body_text = tree.new_node(NodeType::NodeText);
    body_text.set_tokens(b"Body".to_vec());
    body.append_child(&body_text);
    tree.root().append_child(&tabs);
    tabs.append_child(&item);
    item.append_child(&title);
    item.append_child(&body);
    let mut renderer = ProtyleRenderer::new(&tree, RenderOptions::new(), ParseOptions::new());
    let html = String::from_utf8(renderer.render()).unwrap();
    assert!(html.contains("class=\"tabs\""), "{html}");
    assert!(html.contains("class=\"tab-item\""), "{html}");
    assert!(
        html.contains("class=\"tab-item-title callout-title\""),
        "{html}"
    );
    assert_eq!(html.matches("Title").count(), 1, "{html}");
    assert!(html.contains("Body"), "{html}");
}

#[test]
fn caret_normalization_belongs_to_tree_wrapper_not_direct_markdown_rendering() {
    let lute = lute_core::lute::Lute::new();
    let direct = lute.md2blockdom("`‸`", false);
    assert!(direct.contains('‸'), "{direct}");
    assert!(!direct.contains("<wbr>"), "{direct}");
    let tree = lute_core::parse::parse("", "`‸`".as_bytes(), ParseOptions::new());
    let wrapped = lute.tree2blockdom(&tree);
    assert!(wrapped.contains("<wbr>"), "{wrapped}");
    assert!(!wrapped.contains('‸'), "{wrapped}");
}
