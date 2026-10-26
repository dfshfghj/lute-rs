//! SpinBlockDOM keeps the complete Go normalization/parse/transform pipeline.
use lute_core::{
    ast::{self, NodeType, WalkStatus},
    lute::Lute,
    options::ParseOptions,
    parse,
};

fn engine() -> Lute {
    let mut lute = Lute::new();
    lute.parse_options.kramdown_block_ial = true;
    lute.parse_options.tabs = true;
    lute
}

#[test]
fn spin_flattens_parsed_code_and_preserves_literal_quotes() {
    let lute = engine();
    let dom = r#"<div data-node-id="20261023000000-para001" data-type="NodeParagraph"><div contenteditable="true">`a&quot;b`</div></div>"#;
    let expected = "<div data-node-id=\"20261023000000-para001\" data-node-index=\"1\" data-type=\"NodeParagraph\" class=\"p\"><div contenteditable=\"true\" spellcheck=\"false\">\u{200b}<span data-type=\"code\">\u{2060}a\"b</span>\u{200b}</div><div class=\"protyle-attr\" contenteditable=\"false\">\u{200b}</div></div>";
    assert_eq!(lute.spin_blockdom(dom), expected);
}

#[test]
fn spin_restores_empty_paragraph_identity() {
    let lute = engine();
    let dom = r#"<div data-node-id="20261023000000-para001" data-type="NodeParagraph"><div contenteditable="true"></div></div>"#;
    let expected = "<div data-node-id=\"20261023000000-para001\" data-node-index=\"1\" data-type=\"NodeParagraph\" class=\"p\"><div contenteditable=\"true\" spellcheck=\"false\"></div><div class=\"protyle-attr\" contenteditable=\"false\">\u{200b}</div></div>";
    assert_eq!(lute.spin_blockdom(dom), expected);
}

#[test]
fn spin_normalizes_browser_caret_newline_before_reparsing() {
    let lute = engine();
    let dom = r#"<div data-node-id="20261023000000-para001" data-type="NodeParagraph"><div contenteditable="true">first
<wbr>
second</div></div>"#;
    let spun = lute.spin_blockdom(dom);
    assert_eq!(
        spun.matches("data-type=\"NodeParagraph\"").count(),
        2,
        "{spun}"
    );
    assert!(spun.contains(">first</div>"), "{spun}");
    assert!(spun.contains("<wbr>second</div>"), "{spun}");
    assert_eq!(spun.matches("20261023000000-para001").count(), 1, "{spun}");
}

#[test]
fn spin_keeps_mindmap_node_kinds_by_identity() {
    let lute = engine();
    let dom = r#"<div data-node-id="20261023000000-mind001" data-type="NodeMindmap" data-subtype="u"><div data-node-id="20261023000000-item001" data-type="NodeMindmapItem" data-marker="*" data-subtype="u"><div data-node-id="20261023000000-para001" data-type="NodeParagraph"><div contenteditable="true">body</div></div></div></div>"#;
    let spun = lute.spin_blockdom(dom);
    assert!(spun.contains("data-type=\"NodeMindmap\""), "{spun}");
    assert!(spun.contains("data-type=\"NodeMindmapItem\""), "{spun}");
    assert!(spun.contains("20261023000000-mind001"), "{spun}");
    assert!(spun.contains("20261023000000-item001"), "{spun}");
}

#[test]
fn delimiter_pairing_across_styles_uses_existing_inline_parser() {
    let mut tree = lute_core::blockdom::blockdom2tree(
        r#"<div data-type="NodeParagraph"><div contenteditable="true">*<span data-type="strong">hello</span>*</div></div>"#,
        ParseOptions::new(),
    );
    let paragraph = tree.root().first_child().unwrap();
    paragraph
        .first_child()
        .unwrap()
        .next()
        .unwrap()
        .set_ial_attr("style", "color: red");
    parse::text_marks2inlines_with_nested_syntax(&mut tree, &ParseOptions::new());
    assert_eq!(
        paragraph.first_child().unwrap().typ(),
        NodeType::NodeEmphasis
    );
    let mut found_style = false;
    ast::walk(&paragraph, &mut |node, entering| {
        if entering && node.typ() == NodeType::NodeTextMark {
            found_style = node.is_text_mark_type("strong")
                && node.text() == "hello"
                && node.ial_attr("style") == "color: red";
        }
        WalkStatus::WalkContinue
    });
    assert!(found_style);
}
