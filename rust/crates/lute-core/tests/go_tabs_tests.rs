//! Ports of Go test/tabs_{test,export_test,whitespace_test,title_block_test,task_test}.go.
//! Expectations come from the Go assertions, never from Rust snapshots. Public owning
//! Node handles are used throughout. Renderer APIs absent in Rust are documented below.
use lute_core::{
    ast::{walk, Node, NodeType, Tree, WalkStatus},
    html::{escape_attr_val, unescape_html_str},
    lute::Lute,
    parse::{parse, parse_block},
    render::{format::FormatRenderer, protyle::ProtyleRenderer, Renderer},
};

fn engine() -> Lute {
    let mut l = Lute::new();
    l.parse_options.tabs = true;
    l.parse_options.kramdown_block_ial = true;
    l.parse_options.block_ref = true;
    l.parse_options.super_block = true;
    l.parse_options.text_mark = true;
    l.parse_options.protyle_wysiwyg = true;
    l.render_options.protyle_wysiwyg = true;
    l.render_options.kramdown_block_ial = true;
    l
}
fn nodes(tree: &Tree) -> (Vec<Node>, Vec<Node>) {
    let (mut tabs, mut items) = (vec![], vec![]);
    walk(&tree.root(), &mut |n, entering| {
        if entering {
            match n.typ() {
                NodeType::NodeTabs => tabs.push(n),
                NodeType::NodeTabItem => items.push(n),
                _ => (),
            }
        }
        WalkStatus::WalkContinue
    });
    (tabs, items)
}
fn parsed(l: &Lute, md: &str) -> (Vec<Node>, Vec<Node>) {
    nodes(&parse("", md.as_bytes(), l.parse_options.clone()))
}
fn counts(l: &Lute, md: &str, expected: (usize, usize)) {
    let (tabs, items) = parsed(l, md);
    assert_eq!((tabs.len(), items.len()), expected, "{md}");
}
fn title_block(item: &Node) -> Option<Node> {
    item.children().find(|n| n.ial_attr("tabs-title") == "true")
}

#[test]
fn go_test_tabs_round_trip() {
    let l = engine();
    let md = ":::: tabs\n@tab **First**\n{: id=\"20260905120000-item001\"}\n\nOne\n::: tabs\n@tab Nested\n```\n@tab literal\n:::\n```\n:::\n@tab:active Second\n{: id=\"20260905120000-item002\"}\n\nTwo\n::::\n{: id=\"20260905120000-tabs001\" tabs-active-id=\"20260905120000-item002\" tabs-position=\"left\"}\n";
    let mut dom = l.md2blockdom(md, false);
    for _ in 0..3 {
        let (tabs, items) = nodes(&l.blockdom2tree(&dom));
        assert_eq!((tabs.len(), items.len()), (2, 3), "{dom}");
        assert!(dom.contains("data-type=\"strong\">First"), "{dom}");
        assert_eq!(items[2].id(), "20260905120000-item002");
        assert_eq!(tabs[0].ial_attr("tabs-active-id"), items[2].id());
        assert_eq!(tabs[0].ial_attr("tabs-position"), "left");
        assert!(dom.contains("literal") && dom.contains("Two"));
        dom = l.spin_blockdom(&dom);
    }
}
#[test]
fn go_test_tabs_containers_and_fences() {
    let l = engine();
    for md in [
        "::: tabs\n@tab A\nbody\n:::\n",
        ":::\ttabs\n@tab\tA\nbody\n:::\n",
        "> ::: tabs\n> @tab A\n> body\n> :::\n",
        "- ::: tabs\n  @tab A\n  body\n  :::\n",
        "{{{row\n::: tabs\n@tab A\nbody\n:::\n}}}\n",
        "::: tabs\n@tab A\nbody",
    ] {
        let (tabs, items) = parsed(&l, md);
        assert_eq!((tabs.len(), items.len()), (1, 1), "{md}");
        assert_eq!(items[0].parent(), Some(tabs[0].clone()));
        counts(&l, &l.format("", md), (1, 1));
    }
    for md in [
        "@tab stray\nbody\n:::\n",
        "@tab:active stray\nbody\n:::\n",
        ":::tabs\n:::tab A\nbody\n:::\n:::\n",
        ":: tabs\n@tab A\nbody\n::\n",
        "::: tabs extra\n@tab A\nbody\n:::\n",
        "```\n::: tabs\n@tab A\n:::\n```\n",
    ] {
        assert!(parsed(&l, md).0.is_empty(), "{md}");
    }
}
const FRAGMENT: &str = r#"<div data-node-id="20260905120000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info callout-info"><span class="tab-item-title callout-title"><span data-type="strong">Title</span></span></div><div class="tab-item-content"><div data-node-id="20260905120000-para001" data-type="NodeParagraph" class="p"><div contenteditable="true">Body</div></div></div></div>"#;
#[test]
fn go_test_tab_item_fragment_and_export() {
    let l = engine();
    let spun = l.spin_blockdom(FRAGMENT);
    let (tabs, items) = nodes(&l.blockdom2tree(&spun));
    assert_eq!((tabs.len(), items.len()), (0, 1));
    assert_eq!(items[0].id(), "20260905120000-item001");
    assert!(spun.contains("data-type=\"strong\">Title"));
    assert_eq!(
        items[0].first_child().unwrap().id(),
        "20260905120000-para001"
    );
    let md = l.blockdom2stdmd(FRAGMENT);
    for expected in ["::: tabs", "@tab", "**Title**", "Body"] {
        assert!(md.contains(expected), "{md}");
    }
}
#[test]
fn go_test_tabs_html_and_empty_input() {
    let l = engine();
    let md = "::: tabs\n@tab **Title**\nBody\n@tab\n:::\n";
    let html = l.markdown("", md);
    let back = l.html2md(&html);
    let (tabs, items) = parsed(&l, &back);
    assert_eq!((tabs.len(), items.len()), (1, 2), "{back}");
    assert!(back.contains("Title"));
    assert!(items[1].first_child().is_some());
    assert!(!html.contains("contenteditable") && !html.contains("display: none"));
    assert!(l
        .blockdom2stdmd(&l.md2blockdom(md, false))
        .contains("::: tabs"));
}
#[test]
fn go_test_tabs_empty_paragraph_ids() {
    let l = engine();
    let md = "::: tabs\n@tab\n{: id=\"20260905120000-item001\"}\n\n{: id=\"20260905120000-empty01\"}\n\n{: id=\"20260905120000-empty02\"}\n:::\n{: id=\"20260905120000-tabs001\"}\n";
    let mut dom = l.md2blockdom(md, false);
    for _ in 0..3 {
        let (tabs, items) = nodes(&l.blockdom2tree(&dom));
        assert_eq!((tabs.len(), items.len()), (1, 1));
        assert_eq!(tabs[0].id(), "20260905120000-tabs001");
        assert_eq!(items[0].id(), "20260905120000-item001");
        let ids: Vec<_> = items[0]
            .children()
            .filter(|n| n.typ() == NodeType::NodeParagraph)
            .map(|n| n.id())
            .collect();
        assert_eq!(ids, ["20260905120000-empty01", "20260905120000-empty02"]);
        dom = l.spin_blockdom(&dom);
    }
}
#[test]
fn go_test_tabs_slash_preset() {
    let l = engine();
    let markdown = "::: tabs\n@tab\n‸\n@tab\n\n:::\n";
    let dom = l.md2blockdom(markdown, false);
    assert!(!dom.contains(":::"));
    let (tabs, items) = nodes(&l.blockdom2tree(&dom));
    assert_eq!((tabs.len(), items.len()), (1, 2));
    assert!(items.iter().all(|n| n.first_child().is_some()));
    let paragraph = format!(
        r#"<div data-node-id="20260905134000-paragraph" data-type="NodeParagraph" class="p"><div contenteditable="true">{markdown}</div><div class="protyle-attr" contenteditable="false"></div></div>"#
    );
    let spun = l.spin_blockdom(&paragraph);
    assert!(!spun.contains(":::") && spun.contains("<wbr>"), "{spun}");
}
#[test]
fn go_test_tabs_nested_fence_lengths_and_indent() {
    let l = engine();
    for indent in ["", "  ", "    "] {
        let mut md = format!("::::: tabs\n@tab Outer\n{indent}:::: tabs\n{indent}@tab Middle\n{indent}{indent}::: tabs\n{indent}{indent}@tab Inner\n{indent}{indent}body\n{indent}{indent}:::\n{indent}@tab Middle sibling\n{indent}body\n{indent}::::\n@tab Outer sibling\nbody\n:::::\n");
        for _ in 0..3 {
            let (tabs, items) = parsed(&l, &md);
            assert_eq!((tabs.len(), items.len()), (3, 5), "{md}");
            for (child, parent) in [
                (&tabs[1], &items[0]),
                (&tabs[2], &items[1]),
                (&items[3], &tabs[1]),
                (&items[4], &tabs[0]),
            ] {
                assert_eq!(child.parent(), Some(parent.clone()));
            }
            md = l.format("", &md);
        }
    }
    for md in [
        "::: tabs\n@tab A\n::: tabs\nbody\n@tab B\n:::\n",
        "::: tabs\n@tab A\n:::: tabs\nbody\n@tab B\n:::\n",
        ":::: tabs\n@tab A\n:::\nbody\n@tab B\n::::\n",
        "::: tabs\n@tab A\n::::\nbody\n@tab B\n:::\n",
    ] {
        counts(&l, md, (1, 2));
        assert!(l.format("", md).contains("@tab B\n"));
    }
}
#[test]
fn go_test_tabs_canonical_fence_lengths() {
    let l = engine();
    for (md,expected) in [
        (":::::::: tabs\n@tab A\nbody\n::::::::\n",vec!["::: tabs",":::"]),
        (":::::::: tabs\n@tab A\n:::::: tabs\n@tab B\nbody\n::::::\n::::::::\n",vec![":::: tabs","::: tabs",":::","::::"]),
        (":::::::: tabs\n@tab A\n:::::: tabs\n@tab B\n:::: tabs\n@tab C\nbody\n::::\n::::::\n::::::::\n",vec!["::::: tabs",":::: tabs","::: tabs",":::","::::",":::::"])] {
        let formatted=l.format("",md);
        let fences:Vec<_>=formatted.lines().map(str::trim).filter(|s|s.starts_with(":::")).collect();
        assert_eq!(fences,expected,"{formatted}");
    }
}
#[test]
fn go_test_tabs_deep_nesting() {
    let l = engine();
    let depth = 12;
    let mut md = String::new();
    for level in 0..depth {
        md += &format!(
            "{}{} tabs\n{}@tab Level {}\n",
            " ".repeat(2 * level),
            ":".repeat(depth - level + 2),
            " ".repeat(2 * level),
            char::from(b'A' + level as u8)
        );
    }
    md += &format!("{}deep body\n", " ".repeat(2 * (depth - 1)));
    for level in (0..depth).rev() {
        md += &format!(
            "{}{}\n",
            " ".repeat(2 * level),
            ":".repeat(depth - level + 2)
        );
    }
    for _ in 0..3 {
        let (tabs, items) = parsed(&l, &md);
        assert_eq!((tabs.len(), items.len()), (depth, depth));
        for level in 1..depth {
            assert_eq!(tabs[level].parent(), Some(items[level - 1].clone()));
        }
        assert!(items[depth - 1].content().contains("deep body"));
        md = l.format("", &md);
    }
}
#[test]
fn go_test_tabs_literal_markers() {
    let l = engine();
    for body in [
        "```\n@tab Literal\n:::\n```\n",
        "~~~\n@tab Literal\n:::\n~~~\n",
        "    @tab Literal\n    :::\n",
        "> @tab Literal\n> :::\n",
        "- @tab Literal\n  :::\n",
        "\\@tab Literal\n\\@tab:active Literal\n",
        "\\::: tabs\n\\:::\nLiteral\n",
        "Literal first line\n\\@tab Literal\n\\@tab:active Literal\n",
        "@table Literal\n@tab:activeX Literal\n@tab:other Literal\n",
    ] {
        let mut md = format!("::: tabs\n@tab A\n\n{body}\n@tab B\nbody\n:::\n");
        for _ in 0..3 {
            let (_, items) = parsed(&l, &md);
            counts(&l, &md, (1, 2));
            assert!(items[0].content().contains("Literal"));
            assert!(l.format("", &md).contains("@tab B\n"));
            md = l.format("", &md);
        }
    }
}
#[test]
fn go_test_tabs_literal_markers_spin() {
    let l = engine();
    for body in [
        "\\@tab Literal\n\\@tab:active Literal\n",
        "Literal first line\n\\@tab Literal\n\\@tab:active Literal\n",
        "\\::: tabs\n\\:::\nLiteral\n",
        "Literal first line\n\\::: tabs\n\\:::\n",
    ] {
        let mut dom = l.md2blockdom(
            &format!("::: tabs\n@tab A\n\n{body}\n@tab B\nbody\n:::\n"),
            false,
        );
        let mut expected = None;
        for _ in 0..4 {
            let (tabs, items) = nodes(&l.blockdom2tree(&dom));
            assert_eq!((tabs.len(), items.len()), (1, 2));
            let content = items[0].content();
            if let Some(ref expected) = expected {
                assert_eq!(&content, expected);
            } else {
                expected = Some(content);
            }
            dom = l.spin_blockdom(&dom);
        }
    }
}
#[test]
fn go_test_tabs_empty_and_leading_body() {
    let l = engine();
    for (input, n, leading) in [
        ("::: tabs\n:::\n", 1, ""),
        ("::: tabs", 1, ""),
        ("::: tabs\n@tab", 1, ""),
        ("::: tabs\n@tab\n@tab\n:::\n", 2, ""),
        (
            "::: tabs\nLeading body\n@tab Titled\nbody\n:::\n",
            2,
            "Leading body",
        ),
        ("::: tabs\nLeading body", 1, "Leading body"),
    ] {
        let mut md = input.to_owned();
        for _ in 0..3 {
            let (tabs, items) = parsed(&l, &md);
            assert_eq!((tabs.len(), items.len()), (1, n), "{md}");
            if !leading.is_empty() {
                assert!(items[0].text().contains(leading));
            }
            assert!(items.iter().all(|n| n.first_child().is_some()));
            md = l.format("", &md);
        }
    }
}
#[test]
fn go_test_tabs_active_selection() {
    for ial in [false, true] {
        for block in [false, true] {
            for (name, markers, group_ial, mut selected) in [
                ("default", ["@tab", "@tab", "@tab"], "", 0),
                ("second", ["@tab", "@tab:active", "@tab"], "", 1),
                (
                    "first marker wins",
                    ["@tab", "@tab:active", "@tab:active"],
                    "",
                    1,
                ),
                (
                    "marker overrides IAL",
                    ["@tab", "@tab:active", "@tab"],
                    " tabs-active-id=\"20260905120000-item003\"",
                    1,
                ),
                (
                    "valid IAL",
                    ["@tab", "@tab", "@tab"],
                    " tabs-active-id=\"20260905120000-item003\"",
                    2,
                ),
                (
                    "invalid IAL",
                    ["@tab", "@tab", "@tab"],
                    " tabs-active-id=\"20260905120000-missing\"",
                    0,
                ),
            ] {
                let mut l = engine();
                l.parse_options.kramdown_block_ial = ial;
                l.render_options.kramdown_block_ial = ial;
                let mut md = "::: tabs\n".to_owned();
                for (i, marker) in markers.iter().enumerate() {
                    md += &format!("{marker} Title\n");
                    if ial {
                        md += &format!("{{: id=\"20260905120000-item00{}\"}}\n", i + 1);
                    }
                    md += "\nbody\n";
                }
                md += ":::\n";
                if ial {
                    md += &format!("{{: id=\"20260905120000-tabs001\"{group_ial}}}\n");
                }
                if !ial && name == "valid IAL" {
                    selected = 0;
                }
                for round in 0..4 {
                    let tree = if block {
                        parse_block("", md.as_bytes(), l.parse_options.clone())
                    } else {
                        parse("", md.as_bytes(), l.parse_options.clone())
                    };
                    let (tabs, items) = nodes(&tree);
                    assert_eq!((tabs.len(), items.len()), (1, 3), "{name} {md}");
                    assert!(!items[selected].id().is_empty());
                    assert_eq!(
                        tabs[0].ial_attr("tabs-active-id"),
                        items[selected].id(),
                        "{name} round {round}"
                    );
                    md = l.format("", &md);
                    assert_eq!(md.matches("@tab:active").count(), 1, "{md}");
                }
            }
        }
    }
}
#[test]
fn go_test_tabs_nested_active_selection() {
    let l = engine();
    let mut md=":::: tabs\n@tab Outer first\n::: tabs\n@tab Inner first\nbody\n@tab:active Inner second\nbody\n:::\n@tab:active Outer second\nbody\n::::\n".to_owned();
    for _ in 0..3 {
        let (tabs, items) = parsed(&l, &md);
        assert_eq!((tabs.len(), items.len()), (2, 4));
        assert_eq!(tabs[0].ial_attr("tabs-active-id"), items[3].id());
        assert_eq!(tabs[1].ial_attr("tabs-active-id"), items[2].id());
        md = l.format("", &md);
    }
}
#[test]
fn go_test_tabs_item_attribute_placement() {
    let l = engine();
    let mut md="::: tabs\n@tab A\n{: id=\"20260905120000-item001\" custom-item=\"kept\"}\n\nbody\n{: id=\"20260905120000-para001\" custom-body=\"kept\"}\n@tab B\n{: id=\"20260905120000-item002\"}\n\n{: id=\"20260905120000-empty01\"}\n:::\n{: id=\"20260905120000-tabs001\" tabs-position=\"left\"}\n".to_owned();
    for _ in 0..3 {
        let (tabs, items) = parsed(&l, &md);
        assert_eq!((tabs.len(), items.len()), (1, 2));
        assert_eq!(tabs[0].id(), "20260905120000-tabs001");
        assert_eq!(tabs[0].ial_attr("tabs-position"), "left");
        assert_eq!(items[0].id(), "20260905120000-item001");
        assert_eq!(items[0].ial_attr("custom-item"), "kept");
        assert_eq!(items[1].id(), "20260905120000-item002");
        let body = items[0].first_child().unwrap();
        assert_eq!(body.id(), "20260905120000-para001");
        assert_eq!(body.ial_attr("custom-body"), "kept");
        assert_eq!(
            items[1].first_child().unwrap().id(),
            "20260905120000-empty01"
        );
        md = l.format("", &md);
    }
}
#[test]
fn go_test_tabs_title_whitespace() {
    let l = engine();
    for title in [
        " leading",
        "trailing ",
        "two  spaces",
        "   ",
        " <span data-type=\"strong\">bold</span> ",
    ] {
        let mut dom = format!(
            r#"<div data-node-id="20260909000000-tabs001" data-type="NodeTabs" class="tabs"><div data-node-id="20260909000000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info"><span class="tab-item-title">{title}</span></div><div class="tab-item-content"><div data-node-id="20260909000000-para001" data-type="NodeParagraph" class="p"><div contenteditable="true">Body</div></div></div></div></div>"#
        );
        // Public Node intentionally has no mutable raw metadata access. Compare the
        // same inline title by extracting the re-rendered title span from DOM.
        for _ in 0..3 {
            let tree = l.blockdom2tree(&dom);
            assert_eq!(nodes(&tree).1.len(), 1);
            let rendered = String::from_utf8(
                ProtyleRenderer::new(&tree, l.render_options.clone(), l.parse_options.clone())
                    .render(),
            )
            .unwrap();
            let dom_tree = lute_core::dom::parse(&rendered);
            let title_index = dom_tree
                .nodes
                .iter()
                .position(|n| {
                    n.attrs.iter().any(|(key, value)| {
                        key == "class" && value == "tab-item-title callout-title"
                    })
                })
                .unwrap();
            let got = unescape_html_str(&dom_tree.text(title_index));
            let expected = title
                .replace("<span data-type=\"strong\">", "")
                .replace("</span>", "");
            assert_eq!(got, expected, "{rendered}");
            dom = l.spin_blockdom(&rendered);
        }
    }
}
#[test]
fn go_test_tabs_literal_marker_caret_spin() {
    let l = engine();
    for marker in [
        "@tab Literal body",
        "@tab:active Literal body",
        "::: tabs",
        ":::",
    ] {
        let markdown = format!("::: tabs\n@tab First\n\\{marker}\n@tab Second\nMore body\n:::\n");
        let mut dom = l.md2blockdom(&markdown, false);
        dom = dom.replace(
            &format!("<span data-type=\"backslash\">{}</span>", &marker[..1]),
            &format!("<wbr>{}", &marker[..1]),
        );
        for _ in 0..3 {
            dom = l.spin_blockdom(&dom);
            let (tabs, items) = nodes(&l.blockdom2tree(&dom));
            assert_eq!((tabs.len(), items.len()), (1, 2));
            assert!(l.blockdom2stdmd(&dom).contains("@tab Second\n"));
            assert_eq!(dom.matches("<wbr>").count(), 1);
            assert!(!dom.contains('\\'));
        }
    }
}
#[test]
fn go_test_tabs_clipboard_nesting_and_literal_ial() {
    let l = engine();
    let md=":::: tabs\n@tab First\n\n::: tabs\n@tab Inner\nA\n@tab:active Selected\nB\n:::\n@tab:active Second\n\n```\n{: id=\"literal\"}\n@tab literal\n::: tabs\n```\n::::\n";
    let copied = l.blockdom2stdmd(&l.md2blockdom(md, false));
    let (tabs, items) = parsed(&l, &copied);
    assert_eq!((tabs.len(), items.len()), (2, 4));
    assert!(copied.contains(":::: tabs"));
    assert_eq!(copied.matches("@tab:active").count(), 2);
    for group in tabs {
        let last = group
            .children()
            .filter(|n| n.typ() == NodeType::NodeTabItem)
            .last()
            .unwrap();
        assert_eq!(group.ial_attr("tabs-active-id"), last.id());
    }
    assert!(copied.contains("{: id=\"literal\"}\n@tab literal\n::: tabs"));
    assert_eq!(copied.matches("{: ").count(), 1);
}
#[test]
fn go_test_tabs_empty_and_multiline_title_identity() {
    let l = engine();
    for content in [
        "",
        "First<br />Second",
        "😀😀😀😀😀😀😀😀😀😀😀😀😀",
        "<span data-type=\"code\">@tab literal</span>",
    ] {
        let mut dom = format!(
            r#"<div data-node-id="20260905120000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info"><div data-node-id="20260905120000-title01" data-type="NodeParagraph" class="p" tabs-title="true"><div class="tab-item-title callout-title" contenteditable="true">{content}</div></div></div><div class="tab-item-content"><div data-node-id="20260905120000-empty01" data-type="NodeParagraph" class="p" tabs-placeholder="true"><div contenteditable="true"></div></div></div></div>"#
        );
        for _ in 0..3 {
            dom = l.spin_blockdom(&dom);
            let (_, items) = nodes(&l.blockdom2tree(&dom));
            assert_eq!(items.len(), 1);
            assert_eq!(
                title_block(&items[0]).unwrap().id(),
                "20260905120000-title01"
            );
            assert!(dom.contains("tabs-placeholder=\"true\""));
        }
    }
}
#[test]
fn go_test_tabs_task_round_trip() {
    for marker in [" ", "X", "x", "/", "-", "?", "!", "\"", "&", "<"] {
        let l = engine();
        let mut dom = l.md2blockdom("::: tabs\n@tab **Task**\nBody\n:::\n", false);
        dom = dom
            .replacen(
                "class=\"tab-item\"",
                &format!(
                    "class=\"tab-item\" tabs-task=\"{}\"",
                    escape_attr_val(marker)
                ),
                1,
            )
            .replacen("class=\"tabs\"", "class=\"tabs\" tabs-task=\"true\"", 1);
        for _ in 0..3 {
            for tree in [
                l.blockdom2tree(&dom),
                l.blockdom2tree(&l.html2blockdom(&l.blockdom2html(&dom))),
                parse(
                    "",
                    l.blockdom2stdmd(&dom).as_bytes(),
                    l.parse_options.clone(),
                ),
            ] {
                let (groups, items) = nodes(&tree);
                assert_eq!((groups.len(), items.len()), (1, 1));
                assert_eq!(groups[0].ial_attr("tabs-task"), "true");
                assert_eq!(items[0].ial_attr("tabs-task"), marker);
            }
            dom = l.md2blockdom(&l.blockdom2stdmd(&dom), false);
        }
    }
}
#[test]
fn go_test_custom_task_status_spin_round_trip() {
    let mut l = engine();
    l.render_options.data_task = true;
    l.parse_options.arbitrary_task_list_item_marker = true;
    l.render_options.export_normalize_task_list_marker = true;
    for marker in [" ", "X", "x", "/", "-", "?", "!", "\"", "&", "<"] {
        let mut dom = l.md2blockdom("* [ ] Task\n", false).replacen(
            "data-task=\" \"",
            &format!("data-task=\"{}\"", escape_attr_val(marker)),
            1,
        );
        for _ in 0..3 {
            dom = l.spin_blockdom(&dom);
            let expected = if marker == "x" { "X" } else { marker };
            assert!(
                dom.contains(&format!("data-task=\"{}\"", escape_attr_val(expected))),
                "{marker}: {dom}"
            );
        }
    }
}

#[test]
fn go_test_tabs_standard_export_literal_fences() {
    for markdown in [
        "::: tabs\n@tab First\n\\::: tabs\nLiteral body\n\\:::\n@tab Second\nMore body\n:::\n",
        "::: tabs\n@tab \\::: tabs\nLiteral body\n@tab Second\nMore body\n:::\n",
    ] {
        for plain_dom in [false, true] {
            let l = engine();
            let mut dom = l.md2blockdom(markdown, false);
            if plain_dom {
                dom = dom.replace("<span data-type=\"backslash\">:</span>", ":");
            }
            let tree = l.blockdom2tree(&dom);
            let exported = String::from_utf8(
                FormatRenderer::new_protyle_export(
                    &tree,
                    lute_core::options::RenderOptions::new(),
                    l.parse_options.clone(),
                )
                .render(),
            )
            .unwrap();
            assert!(exported.contains("\\::: tabs"), "{exported}");
            assert!(!exported.contains("\\\\::: tabs"), "{exported}");
            assert!(parsed(&l, &exported).0.is_empty());
            let html = l.markdown("", &exported);
            for content in ["::: tabs", "Literal body", "Second", "More body"] {
                assert!(html.contains(content), "{exported}");
            }
        }
    }
}
#[test]
fn go_test_tabs_nested_slash_preset() {
    let l = engine();
    let markdown = "::: tabs\n@tab\n\n‸\n\n@tab\n\n:::\n";
    let paragraph = format!(
        r#"<div data-node-id="20260905134000-para001" data-type="NodeParagraph" class="p"><div contenteditable="true">{markdown}</div><div class="protyle-attr" contenteditable="false"></div></div>"#
    );
    let inner = l.spin_blockdom(&paragraph);
    let (tabs, items) = nodes(&l.blockdom2tree(&inner));
    assert_eq!((tabs.len(), items.len()), (1, 2));
    assert!(inner.contains("<wbr>"));
    let mut dom = format!(
        r#"<div data-node-id="20260905120000-tabs001" data-type="NodeTabs" class="tabs"><div data-node-id="20260905120000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info callout-info"><span class="tab-item-title callout-title">Outer</span></div><div class="tab-item-content">{inner}</div></div></div>"#
    );
    for _ in 0..3 {
        dom = l.spin_blockdom(&dom);
        let (tabs, items) = nodes(&l.blockdom2tree(&dom));
        assert_eq!((tabs.len(), items.len()), (2, 3));
        assert_eq!(tabs[1].parent(), Some(items[0].clone()));
        assert!(dom.contains("<wbr>"));
    }
}
#[test]
fn go_test_tabs_original_title_block() {
    let l = engine();
    let mut dom=r#"<div data-node-id="20260905120000-tabs001" data-type="NodeTabs" class="tabs" tabs-active-id="20260905120000-item001"><div data-node-id="20260905120000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info callout-info"><div data-node-id="20260905120000-title01" data-type="NodeParagraph" class="p" tabs-title="true" custom-test="kept"><div class="tab-item-title callout-title" contenteditable="true"><span data-type="strong">abcdefghijklmnop</span> <span data-type="block-ref" data-id="20260905120000-ref0001" data-subtype="d">Reference</span></div></div></div><div class="tab-item-content"><div data-node-id="20260905120000-para001" data-type="NodeParagraph" class="p"><div contenteditable="true">Body</div></div></div></div></div>"#.to_owned();
    for _ in 0..4 {
        let tree = l.blockdom2tree(&dom);
        let (_, items) = nodes(&tree);
        assert_eq!(items.len(), 1);
        let title = title_block(&items[0]).unwrap();
        assert_eq!(title.id(), "20260905120000-title01");
        assert_eq!(title.ial_attr("custom-test"), "kept");
        let mut count = 0;
        walk(&tree.root(), &mut |n, entering| {
            if entering && n.id() == title.id() {
                count += 1;
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(count, 1);
        let md = l.blockdom2stdmd(&dom);
        assert_eq!(md.matches("abcdefghijklmnop").count(), 1);
        assert!(md.contains("@tab:active **abcdefghijklmnop**"), "{md}");
        for forbidden in ["custom-test", "tabs-title", "{: "] {
            assert!(!md.contains(forbidden), "{md}");
        }
        counts(&l, &md, (1, 1));
        let flat = String::from_utf8(
            FormatRenderer::new_protyle_export(
                &tree,
                lute_core::options::RenderOptions::new(),
                l.parse_options.clone(),
            )
            .render(),
        )
        .unwrap();
        assert_eq!(flat.matches("abcdefghijklmnop").count(), 1);
        assert!(!flat.contains("@tab"));
        let html = l.blockdom2html(&dom);
        assert_eq!(html.matches("abcdefghijklmnop").count(), 1);
        assert!(html.contains("Body"));
        let back = l.html2md(&html);
        assert_eq!(parsed(&l, &back).1.len(), 1);
        assert_eq!(back.matches("abcdefghijklmnop").count(), 1);
        assert!(!back.contains("@tab:active {: "));
        dom = l.spin_blockdom(&dom);
        assert_eq!(
            dom.matches("data-node-id=\"20260905120000-title01\"")
                .count(),
            1
        );
        assert!(dom.contains("class=\"tab-item-title callout-title\""));
    }
    // Go's PDF/HTML ProtyleExportRenderer and standalone title rendering have
    // no public Rust counterpart; their assertions remain an explicit API gap.
}

/// Source branch coverage: Go parse/tabs.go finalization must retain the
/// selected item's identity when its siblings are empty and EOF closes the group.
/// This combines TestTabsActiveSelection and TestTabsEmptyAndLeadingBody states.
#[test]
fn tabs_finalize_active_empty_item_at_eof() {
    let l = engine();
    for suffix in ["", "\n", "\n:::\n"] {
        let mut md =
            format!("::: tabs\n@tab\n@tab:active\n{{: id=\"20260905120000-selected\"}}{suffix}");
        for _ in 0..3 {
            let (tabs, items) = parsed(&l, &md);
            assert_eq!((tabs.len(), items.len()), (1, 2));
            assert_eq!(tabs[0].ial_attr("tabs-active-id"), items[1].id());
            assert!(items.iter().all(|n| n.first_child().is_some()));
            md = l.format("", &md);
        }
    }
}

// Migration gap: test/tabs_export_test.go::TestTabsExportNestedTitleBlocks
// exclusively exercises Go ProtyleExportRenderer (PDF/export HTML), for which
// Rust currently exposes no counterpart. TestTabsOriginalTitleBlock's two
// assertions for that renderer (whole tree and standalone title root) are also
// deferred. Ordinary ProtyleRenderer output is not a substitute for this API.
