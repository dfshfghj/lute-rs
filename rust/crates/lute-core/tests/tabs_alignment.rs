//! Source-aligned tabs container parsing and HTML rendering regressions.
use lute_core::{
    ast::NodeType,
    options::{ParseOptions, RenderOptions},
    parse::parse,
    render::{HtmlRenderer, Renderer},
};

fn options() -> ParseOptions {
    let mut o = ParseOptions::new();
    o.tabs = true;
    o.kramdown_block_ial = true;
    o
}
#[test]
fn tabs_items_titles_and_active_identity() {
    let tree = parse("", b"::: tabs\n@tab First\n{: id=\"first\"}\n\none\n@tab:active **Second**\n{: id=\"second\"}\n\ntwo\n:::\nafter\n", options());
    let tabs = tree.root().first_child().unwrap();
    assert_eq!(tabs.typ(), NodeType::NodeTabs);
    assert_eq!(tabs.ial_attr("tabs-active-id"), "second");
    let items: Vec<_> = tabs
        .children()
        .filter(|n| n.typ() == NodeType::NodeTabItem)
        .collect();
    assert_eq!(items.len(), 2);
    assert_eq!(tabs.next().unwrap().text(), "after");
    let html = String::from_utf8(
        HtmlRenderer::with_parse_options(&tree, RenderOptions::new(), options()).render(),
    )
    .unwrap();
    assert!(
        html.contains(
            "<span class=\"tab-item-title callout-title\"><strong>Second</strong></span>"
        ),
        "{html}"
    );
    assert!(html.contains("tabs-active-id=\"second\""), "{html}");
}
#[test]
fn nested_fences_and_code_markers_stay_in_their_containers() {
    let tree = parse(
        "",
        b":::: tabs\n@tab Outer\n::: tabs\n@tab Inner\n```\n@tab literal\n:::\n```\n:::\n::::\n",
        options(),
    );
    let outer = tree.root().first_child().unwrap();
    assert_eq!(outer.typ(), NodeType::NodeTabs);
    let inner = outer.first_child().unwrap().first_child().unwrap();
    assert_eq!(inner.typ(), NodeType::NodeTabs);
    let code = inner.first_child().unwrap().first_child().unwrap();
    assert_eq!(code.typ(), NodeType::NodeCodeBlock);
    assert_eq!(
        code.child_by_type(NodeType::NodeCodeBlockCode)
            .unwrap()
            .tokens(),
        b"@tab literal\n:::\n"
    );
}
#[test]
fn empty_and_implicit_items_have_body_paragraphs() {
    for markdown in [
        "::: tabs\n:::\n",
        "::: tabs\nbody\n:::\n",
        "::: tabs\n@tab Empty\n:::\n",
    ] {
        let tree = parse("", markdown.as_bytes(), options());
        let tabs = tree.root().first_child().unwrap();
        assert_eq!(tabs.typ(), NodeType::NodeTabs);
        let item = tabs.first_child().unwrap();
        assert_eq!(item.typ(), NodeType::NodeTabItem);
        assert_eq!(item.first_child().unwrap().typ(), NodeType::NodeParagraph);
        assert!(!tabs.ial_attr("tabs-active-id").is_empty());
    }
}

fn tab_counts(markdown: &str) -> (usize, usize) {
    let tree = parse("", markdown.as_bytes(), options());
    let mut counts = (0, 0);
    lute_core::ast::walk(&tree.root(), &mut |node, entering| {
        if entering {
            if node.typ() == NodeType::NodeTabs {
                counts.0 += 1;
            }
            if node.typ() == NodeType::NodeTabItem {
                counts.1 += 1;
            }
        }
        lute_core::ast::WalkStatus::WalkContinue
    });
    counts
}

#[test]
fn tabs_obey_explicit_containers_and_preserve_literal_markers() {
    for md in [
        ":::\ttabs\n@tab\tA\nbody\n:::\n",
        "> ::: tabs\n> @tab A\n> body\n> :::\n",
        "- ::: tabs\n  @tab A\n  body\n  :::\n",
        "::: tabs\n@tab A\nbody",
    ] {
        assert_eq!(tab_counts(md), (1, 1), "{md}");
    }
    for md in [
        "@tab stray\nbody\n:::\n",
        ":::tabs\n@tab A\nbody\n:::\n",
        ":: tabs\n@tab A\n::\n",
        "::: tabs extra\n@tab A\n:::\n",
        "```\n::: tabs\n@tab A\n:::\n```\n",
    ] {
        assert_eq!(tab_counts(md), (0, 0), "{md}");
    }
    for body in [
        "```\n@tab Literal\n:::\n```\n",
        "    @tab Literal\n    :::\n",
        "> @tab Literal\n> :::\n",
        "- @tab Literal\n  :::\n",
        "\\@tab Literal\n\\@tab:active Literal\n",
        "@table Literal\n@tab:activeX Literal\n@tab:other Literal\n",
    ] {
        let md = format!("::: tabs\n@tab A\n\n{body}\n@tab B\nbody\n:::\n");
        assert_eq!(tab_counts(&md), (1, 2), "{md}");
    }
}

#[test]
fn nested_tabs_consume_only_each_fences_relative_indent() {
    for indent in ["", "  ", "    "] {
        let md = format!("::::: tabs\n@tab Outer\n{indent}:::: tabs\n{indent}@tab Middle\n{indent}{indent}::: tabs\n{indent}{indent}@tab Inner\n{indent}{indent}body\n{indent}{indent}:::\n{indent}@tab Middle sibling\n{indent}body\n{indent}::::\n@tab Outer sibling\nbody\n:::::\n");
        assert_eq!(tab_counts(&md), (3, 5), "{md}");
    }
    for md in [
        "::: tabs\n@tab A\n::: tabs\nbody\n@tab B\n:::\n",
        "::: tabs\n@tab A\n:::: tabs\nbody\n@tab B\n:::\n",
        ":::: tabs\n@tab A\n:::\nbody\n@tab B\n::::\n",
    ] {
        assert_eq!(tab_counts(md), (1, 2), "{md}");
    }
}

#[test]
fn tab_title_block_renders_once_and_keeps_body_in_html_and_preview() {
    let tree = parse("", b"::: tabs\n@tab\n{: id=\"item\"}\n\nTitle\n{: id=\"title\" tabs-title=\"true\"}\n\nBody\n:::\n", options());
    let options = options();
    let mut html_renderer =
        HtmlRenderer::with_parse_options(&tree, RenderOptions::new(), options.clone());
    let mut preview_renderer = lute_core::render::protyle_preview::ProtylePreviewRenderer::new(
        &tree,
        RenderOptions::new(),
        options,
    );
    for output in [html_renderer.render(), preview_renderer.render()] {
        let html = String::from_utf8(output).unwrap();
        assert_eq!(html.matches("Title").count(), 1, "{html}");
        assert_eq!(html.matches("Body").count(), 1, "{html}");
        assert!(
            html.contains("<div class=\"tab-item-title callout-title\"><p"),
            "{html}"
        );
    }
}

#[test]
fn block_ial_finalization_sets_editor_ids_and_retains_document_ial() {
    let mut opts = options();
    opts.protyle_wysiwyg = true;
    let tree = parse(
        "",
        b"body\n\n>\n\n{: id=\"20260905120000-document\" type=\"doc\"}\n",
        opts,
    );
    let body = tree.root().first_child().unwrap();
    assert!(!body.id().is_empty());
    assert_eq!(body.ial_attr("id"), body.id());
    assert_eq!(body.ial_attr("updated"), &body.id()[..14]);
    assert_eq!(tree.id, "20260905120000-document");
    assert_eq!(
        tree.root().last_child().unwrap().typ(),
        NodeType::NodeKramdownBlockIAL
    );
    let quote = body.next().unwrap();
    assert_eq!(quote.typ(), NodeType::NodeBlockquote);
    assert_eq!(quote.last_child().unwrap().typ(), NodeType::NodeParagraph);
}

#[test]
fn spin_finalization_moves_existing_identity_to_new_preceding_block() {
    let mut opts = options();
    opts.protyle_wysiwyg = true;
    opts.spin = true;
    let tree = parse(
        "",
        b"new\n\nold\n{: id=\"20260905120000-existing\" custom-test=\"kept\"}\n",
        opts,
    );
    let first = tree.root().first_child().unwrap();
    assert_eq!(first.id(), "20260905120000-existing");
    assert_eq!(first.ial_attr("custom-test"), "kept");
    let old = first.next().unwrap().next().unwrap();
    assert_eq!(old.text(), "old");
    assert_ne!(old.id(), first.id());
    assert_eq!(old.ial_attr("custom-test"), "");
    assert_eq!(old.ial_attr("id"), old.id());
}
