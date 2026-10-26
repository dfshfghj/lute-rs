//! Migrated from Go parse/performance_regression_test.go.
//! The original structural assertions are preserved; no wall-clock thresholds.
use lute_core::ast::{walk, NodeType, WalkStatus};
use lute_core::options::ParseOptions;
use lute_core::parse::parse;

fn count_nodes(input: &str, options: ParseOptions, typ: NodeType) -> usize {
    let tree = parse("", input.as_bytes(), options);
    let mut count = 0;
    walk(&tree.root(), &mut |n, entering| {
        if entering && n.typ() == typ {
            count += 1;
        }
        WalkStatus::WalkContinue
    });
    count
}

/// Go: TestTableCandidateCompatibility.
#[test]
fn go_table_candidate_compatibility() {
    for (input, setext, want, rows) in [
        ("foo\n---", false, NodeType::NodeParagraph, 0),
        ("foo\n---", true, NodeType::NodeHeading, 0),
        ("foo\n-:", false, NodeType::NodeTable, 1),
        ("foo\n::\nbar", false, NodeType::NodeTable, 2),
        ("a | b\n--- | ---\nx | y", false, NodeType::NodeTable, 2),
        ("a | b\n---\nx | y", false, NodeType::NodeParagraph, 0),
        (
            "alpha | beta\ngamma | delta",
            false,
            NodeType::NodeParagraph,
            0,
        ),
        (
            "前文\na | b\n--- | ---\nx | y",
            false,
            NodeType::NodeParagraph,
            2,
        ),
    ] {
        let mut options = ParseOptions::new();
        options.setext = setext;
        let tree = parse("", input.as_bytes(), options.clone());
        assert_eq!(tree.root().first_child().unwrap().typ(), want, "{input:?}");
        assert_eq!(
            count_nodes(input, options, NodeType::NodeTableRow),
            rows,
            "{input:?}"
        );
    }
}

/// Go: TestAutoLinkCandidateCompatibility.
#[test]
fn go_auto_link_candidate_compatibility() {
    let long = "a".repeat(4096);
    let cases: &[(&str, &[&str])] = &[
        (&long, &[]),
        ("foo://", &[]),
        ("foo://bar", &["foo://bar"]),
        ("中文foo://bar", &["foo://bar"]),
        ("abc-def://host", &["def://host"]),
        ("prefix: foo://bar", &["foo://bar"]),
        ("foo://bar baz://qux", &["baz://qux"]),
        ("foo://bar https://example.com", &["https://example.com"]),
        (
            "https://example.com foo://bar",
            &["https://example.com", "foo://bar"],
        ),
        ("www.example.com", &["http://www.example.com"]),
        ("http://example.com/a).", &["http://example.com/a)"]),
        (
            "https://example.com:8080/a:b ftp://example.com/path",
            &["https://example.com:8080/a:b", "ftp://example.com/path"],
        ),
    ];
    for &(input, want) in cases {
        let mut options = ParseOptions::new();
        options.emoji = false;
        let tree = parse("", input.as_bytes(), options);
        let mut got = Vec::new();
        walk(&tree.root(), &mut |n, entering| {
            if entering && n.typ() == NodeType::NodeLinkDest {
                got.push(String::from_utf8(n.tokens()).unwrap());
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(got, want, "{input:?}");
    }
}

/// Go: TestTaskListProbeCompatibility.
#[test]
fn go_task_list_probe_compatibility() {
    let cases: &[(&str, &[(NodeType, usize)])] = &[
        (
            "- [ ] alpha **beta** and `code`\n",
            &[
                (NodeType::NodeList, 1),
                (NodeType::NodeParagraph, 1),
                (NodeType::NodeStrong, 1),
                (NodeType::NodeCodeSpan, 1),
            ],
        ),
        ("- [ ] # heading\n", &[(NodeType::NodeHeading, 1)]),
        (
            "- [ ] > quote\n",
            &[(NodeType::NodeBlockquote, 1), (NodeType::NodeParagraph, 1)],
        ),
        (
            "- [ ] - nested\n",
            &[(NodeType::NodeList, 2), (NodeType::NodeParagraph, 1)],
        ),
        (
            "- [ ] ```go\n  code\n  ```\n",
            &[(NodeType::NodeCodeBlock, 2)],
        ),
    ];
    for protyle in [false, true] {
        for &(input, types) in cases {
            let mut options = ParseOptions::new();
            options.protyle_wysiwyg = protyle;
            options.kramdown_block_ial = protyle;
            options.kramdown_span_ial = protyle;
            assert_eq!(
                count_nodes(input, options.clone(), NodeType::NodeTaskListItemMarker),
                1,
                "{input:?}, protyle={protyle}"
            );
            for &(typ, want) in types {
                assert_eq!(
                    count_nodes(input, options.clone(), typ),
                    want,
                    "{input:?}, protyle={protyle}, {typ:?}"
                );
            }
        }
    }
}

/// Additional boundaries confirmed with Go parse.Parse (2026-10-24):
/// editor-only empty markers, marker placement, and probe/inline pass separation.
#[test]
fn task_probe_empty_marker_and_plain_paragraph_boundaries() {
    for protyle in [false, true] {
        let mut options = ParseOptions::new();
        options.protyle_wysiwyg = protyle;
        options.kramdown_block_ial = protyle;
        options.kramdown_span_ial = protyle;
        assert_eq!(
            count_nodes("- [ ]\n", options.clone(), NodeType::NodeTaskListItemMarker),
            usize::from(protyle)
        );
        let tree = parse("", b"- [ ] **bold**\n", options);
        let mut markers = 0;
        let mut strong = 0;
        let mut text = Vec::new();
        walk(&tree.root(), &mut |n, entering| {
            if entering {
                match n.typ() {
                    NodeType::NodeTaskListItemMarker => {
                        markers += 1;
                        assert_eq!(n.tokens(), b"[ ]");
                        assert_eq!(
                            n.parent().unwrap().typ(),
                            if protyle {
                                NodeType::NodeListItem
                            } else {
                                NodeType::NodeParagraph
                            }
                        );
                    }
                    NodeType::NodeStrong => strong += 1,
                    NodeType::NodeText => text.push(n.tokens()),
                    _ => {}
                }
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(markers, 1);
        assert_eq!(strong, 1);
        assert_eq!(
            text,
            if protyle {
                vec![b"bold".to_vec()]
            } else {
                vec![b" ".to_vec(), b"bold".to_vec()]
            }
        );
    }
}

/// Go TestSpec 553/566/584: empty explicit reference labels consume `[]`.
#[test]
fn collapsed_references_consume_empty_labels_and_preserve_shortcuts() {
    let lute = lute_core::lute::Lute::new();
    for (input, want) in [
        (
            "[foo][]\n\n[foo]: /url1\n",
            "<p><a href=\"/url1\">foo</a></p>\n",
        ),
        (
            "![foo][]\n\n[foo]: /url \"title\"\n",
            "<p><img src=\"/url\" alt=\"foo\" title=\"title\" /></p>\n",
        ),
        (
            "[foo]\n\n[foo]: /url1\n",
            "<p><a href=\"/url1\">foo</a></p>\n",
        ),
        (
            "[foo][missing]\n\n[foo]: /url1\n",
            "<p>[foo][missing]</p>\n",
        ),
    ] {
        assert_eq!(lute.markdown("", input), want, "{input:?}");
    }
}
