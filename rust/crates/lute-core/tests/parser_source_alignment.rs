use lute_core::{
    ast::{walk, NodeType, WalkStatus},
    options::ParseOptions,
    parse::parse,
};

fn tokens_of(input: &str, options: ParseOptions, typ: NodeType) -> Vec<Vec<u8>> {
    let tree = parse("", input.as_bytes(), options);
    let mut found = Vec::new();
    walk(&tree.root(), &mut |node, entering| {
        if entering && node.typ() == typ {
            found.push(node.tokens());
        }
        WalkStatus::WalkContinue
    });
    found
}

#[test]
fn editor_caret_in_html_tags_round_trips() {
    for mode in 0..3 {
        let mut options = ParseOptions::new();
        options.vditor_wysiwyg = mode == 0;
        options.vditor_ir = mode == 1;
        options.vditor_sv = mode == 2;
        for tag in ["<bar ‸>", "<img src=\"‸\">", "<!-- ‸ -->"] {
            assert_eq!(
                tokens_of(
                    &format!("foo {tag}"),
                    options.clone(),
                    NodeType::NodeInlineHTML
                ),
                vec![tag.as_bytes().to_vec()]
            );
        }
    }
}

#[test]
fn editor_caret_closes_fences_and_setext_headings() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        setext: true,
        ..ParseOptions::new()
    };
    assert_eq!(
        tokens_of(
            "```\nfoo\n```‸\n",
            options.clone(),
            NodeType::NodeCodeBlockCode
        ),
        vec!["foo‸".as_bytes().to_vec()]
    );
    let tree = parse("", "foo\n=‸\n".as_bytes(), options);
    assert_eq!(
        tree.root().first_child().unwrap().typ(),
        NodeType::NodeHeading
    );
    assert_eq!(
        tree.root()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .tokens(),
        "foo‸".as_bytes()
    );
}

#[test]
fn protyle_unconverted_html_is_literal_text() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        ..ParseOptions::new()
    };
    assert_eq!(
        tokens_of("<u>、<sub>foo", options.clone(), NodeType::NodeText),
        vec!["<u>、<sub>foo".as_bytes().to_vec()]
    );
    assert!(tokens_of(
        "<span id=\"tag\">foo</span>",
        options,
        NodeType::NodeInlineHTML
    )
    .is_empty());
}

#[test]
fn table_cell_metadata_is_extracted_without_span_ial_option() {
    let options = ParseOptions {
        kramdown_span_ial: false,
        ..ParseOptions::new()
    };
    let tree = parse(
        "",
        b"| a {: colspan=\"2\" style=\"text-align: right\"} | b |\n| --- | --- |",
        options,
    );
    let cell = tree
        .root()
        .first_child()
        .unwrap()
        .first_child()
        .unwrap()
        .first_child()
        .unwrap()
        .first_child()
        .unwrap();
    assert_eq!(cell.ial_attr("colspan"), "2");
    assert_eq!(cell.ial_attr("style"), "text-align: right");
}

#[test]
fn protyle_code_span_reparses_inner_inline_syntax() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        ..ParseOptions::new()
    };
    for (input, expected) in [
        ("` ``foo`` `", "`foo`"),
        ("`foo &amp; bar`", "foo &amp; bar"),
        ("`<span data-type=\"strong\">foo</span>`", "foo"),
    ] {
        assert_eq!(
            tokens_of(input, options.clone(), NodeType::NodeCodeSpanContent),
            vec![expected.as_bytes().to_vec()],
            "{input}"
        );
    }
}

#[test]
fn editor_link_title_caret_moves_across_closing_parenthesis() {
    let options = ParseOptions {
        vditor_ir: true,
        ..ParseOptions::new()
    };
    for (input, title) in [
        ("[foo](bar \"b\"‸)", "b"),
        ("[foo](bar \"\"‸)", "‸"),
        ("[foo](bar \"\")‸", "‸"),
    ] {
        assert_eq!(
            tokens_of(input, options.clone(), NodeType::NodeLinkTitle),
            vec![title.as_bytes().to_vec()],
            "{input}"
        );
    }
}

#[test]
fn protyle_ordered_marker_moves_caret_to_item_body() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        ..ParseOptions::new()
    };
    for input in ["‸1. foo", "1‸. foo"] {
        let tree = parse("", input.as_bytes(), options.clone());
        let list = tree.root().first_child().unwrap();
        assert_eq!(list.typ(), NodeType::NodeList);
        assert_eq!(
            tokens_of(input, options.clone(), NodeType::NodeText),
            vec!["‸foo".as_bytes().to_vec()]
        );
    }
}

#[test]
fn protyle_empty_paragraphs_and_list_item_ial_survive() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        kramdown_block_ial: true,
        ..ParseOptions::new()
    };
    for input in ["1. \n2. ", "{: id=\"20220426085736-e2v2fzx\"}\n"] {
        let tree = parse("", input.as_bytes(), options.clone());
        let mut paragraphs = 0;
        walk(&tree.root(), &mut |node, entering| {
            if entering && node.typ() == NodeType::NodeParagraph {
                paragraphs += 1;
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(paragraphs, if input.starts_with("1.") { 2 } else { 1 });
    }
    let tree = parse(
        "",
        b"* {: id=\"20220810110631-fdlccs9\"}\n  {: id=\"20220810110631-56rznx9\"}\n",
        options,
    );
    let item = tree.root().first_child().unwrap().first_child().unwrap();
    assert_eq!(item.id(), "20220810110631-fdlccs9");
    let paragraph = item.first_child().unwrap();
    assert_eq!(paragraph.typ(), NodeType::NodeParagraph);
    assert_eq!(paragraph.id(), "20220810110631-56rznx9");
}

#[test]
fn protyle_html_image_uses_width_before_height() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        ..ParseOptions::new()
    };
    for (attrs, style) in [
        ("width=\"871\" height=\"610\"", "width: 871px;"),
        ("height=\"50%\"", "height: 50%;"),
    ] {
        let tree = parse(
            "",
            format!("<img {attrs} src=\"a&amp;b\" alt=\"Image\" />").as_bytes(),
            options.clone(),
        );
        let image = tree.root().first_child().unwrap().first_child().unwrap();
        assert_eq!(image.typ(), NodeType::NodeImage);
        assert_eq!(image.ial_attr("style"), style);
        assert_eq!(
            image
                .child_by_type(NodeType::NodeLinkDest)
                .unwrap()
                .tokens(),
            b"a&b"
        );
    }
}

#[test]
fn unquoted_html_attribute_leaves_tag_closer_for_inline_parser() {
    let options = ParseOptions {
        vditor_wysiwyg: true,
        ..ParseOptions::new()
    };
    assert_eq!(
        tokens_of("<font size=12>t‸", options, NodeType::NodeInlineHTML),
        vec![b"<font size=12>".to_vec()]
    );
}

#[test]
fn editor_caret_at_nested_empty_list_item_stays_in_list() {
    let options = ParseOptions {
        vditor_sv: true,
        ..ParseOptions::new()
    };
    let tree = parse("", "* foo\n  * \n‸bar".as_bytes(), options);
    let outer = tree.root().first_child().unwrap();
    assert!(outer.next().is_none());
    let inner = outer.first_child().unwrap().last_child().unwrap();
    assert_eq!(inner.typ(), NodeType::NodeList);
    let item = inner.first_child().unwrap();
    let text = item.first_child().unwrap();
    assert_eq!(text.typ(), NodeType::NodeText);
    assert!(String::from_utf8(text.tokens())
        .unwrap()
        .starts_with("‸bar"));
}

#[test]
fn leading_table_ial_is_distinct_from_image_ial() {
    let options = ParseOptions {
        protyle_wysiwyg: true,
        kramdown_span_ial: true,
        ..ParseOptions::new()
    };
    let tree = parse(
        "",
        b"| a | b |\n| - | - |\n| {: style=\"width: 342px;\" colspan=\"2\"}![image](x){: style=\"max-width: 325px;\"} | |",
        options,
    );
    let table = tree.root().first_child().unwrap();
    let row = table.last_child().unwrap();
    let cell = row.first_child().unwrap();
    assert_eq!(cell.ial_attr("colspan"), "2");
    assert_eq!(cell.ial_attr("style"), "width: 342px;");
    let image = cell.child_by_type(NodeType::NodeImage).unwrap();
    assert_eq!(image.ial_attr("style"), "max-width: 325px;");
}
