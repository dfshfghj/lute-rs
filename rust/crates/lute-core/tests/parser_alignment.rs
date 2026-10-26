//! Go inline_attribute_list.go and math_block.go alignment regressions.
use lute_core::ast::NodeType;
use lute_core::options::ParseOptions;
use lute_core::parse::parse;

#[test]
fn span_ial_entities_and_identical_duplicate() {
    let mut options = ParseOptions::new();
    options.kramdown_span_ial = true;
    let tree = parse(
        "",
        b"*foo*{: title=\"a &quot;b&quot; &amp; c\"}{: title=\"a &quot;b&quot; &amp; c\"} tail",
        options,
    );
    let emphasis = tree.root().first_child().unwrap().first_child().unwrap();
    assert_eq!(emphasis.ial_attr("title"), "a \"b\" & c");
    let ial = emphasis.next().unwrap();
    assert_eq!(ial.typ(), NodeType::NodeKramdownSpanIAL);
    assert_eq!(ial.tokens(), b"{: title=\"a &quot;b&quot; &amp; c\"}");
    assert_eq!(ial.next().unwrap().tokens(), b" tail");
    assert!(ial.next().unwrap().next().is_none());
}

#[test]
fn span_ial_preserves_different_duplicate_and_unclosed_source() {
    let mut options = ParseOptions::new();
    options.kramdown_span_ial = true;
    let tree = parse("", b"*foo*{: title=\"a\"}{: title=\"b\"}", options.clone());
    let emphasis = tree.root().first_child().unwrap().first_child().unwrap();
    assert_eq!(emphasis.ial_attr("title"), "a");
    assert_eq!(
        emphasis.next().unwrap().next().unwrap().tokens(),
        b"{: title=\"b\"}"
    );
    let tree = parse("", b"*foo*{: title=\"a &amp; b\"", options);
    let emphasis = tree.root().first_child().unwrap().first_child().unwrap();
    assert!(emphasis.ial_raw().is_empty());
    assert_eq!(
        emphasis.next().unwrap().next().unwrap().typ(),
        NodeType::NodeHTMLEntity
    );
}

#[test]
fn math_block_ial_closes_before_following_paragraph() {
    let mut options = ParseOptions::new();
    options.kramdown_block_ial = true;
    let tree = parse(
        "",
        b"$$\nx\n{: id=\"20260101000000-abcdefg\"}\nafter\n",
        options,
    );
    let math = tree.root().first_child().unwrap();
    assert_eq!(math.typ(), NodeType::NodeMathBlock);
    assert_eq!(math.ial_attr("id"), "20260101000000-abcdefg");
    assert_eq!(math.first_child().unwrap().next().unwrap().tokens(), b"x");
    let ial = math.next().unwrap();
    assert_eq!(ial.typ(), NodeType::NodeKramdownBlockIAL);
    assert_eq!(ial.next().unwrap().content(), "after");
}

#[test]
fn math_finalize_moves_caret_and_extracts_span_content() {
    for mode in 0..4 {
        let mut options = ParseOptions::new();
        options.vditor_wysiwyg = mode == 0;
        options.vditor_ir = mode == 1;
        options.vditor_sv = mode == 2;
        options.protyle_wysiwyg = mode == 3;
        let tree = parse("", "$$x$$‸".as_bytes(), options);
        let content = tree
            .root()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(content.tokens(), "x‸".as_bytes());
    }
    let mut options = ParseOptions::new();
    options.protyle_wysiwyg = true;
    let tree = parse(
        "",
        b"$$<span data-type=\"text\">x &amp; y</span>$$",
        options,
    );
    let content = tree
        .root()
        .first_child()
        .unwrap()
        .first_child()
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(content.tokens(), b"x & y");
}

#[test]
fn span_ial_applies_to_supported_inline_nodes_and_removes_caret() {
    for markdown in [
        "*x*", "**x**", "`x`", "~~x~~", "#x#", "==x==", "$x$", "![x](a)",
    ] {
        let mut options = ParseOptions::new();
        options.kramdown_span_ial = true;
        options.tag = true;
        options.mark = true;
        let input = format!("{markdown}{{: title=\"a‸b\"}}");
        let tree = parse("", input.as_bytes(), options);
        let node = tree.root().first_child().unwrap().first_child().unwrap();
        assert_eq!(node.ial_attr("title"), "ab", "{markdown}");
        assert_eq!(node.next().unwrap().typ(), NodeType::NodeKramdownSpanIAL);
    }
}

#[test]
fn math_block_ial_decodes_id_and_retains_last_duplicate_key() {
    let mut options = ParseOptions::new();
    options.kramdown_block_ial = true;
    let tree = parse("", b"$$\nx\n{: id=\"first\" id=\"a&amp;b\"}\n", options);
    assert_eq!(tree.root().first_child().unwrap().id(), "a&b");
}
