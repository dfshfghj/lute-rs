//! Direct ports of Go helper/security tests. Fixture expectations are copied
//! from the named Go tests, never generated from the Rust implementation.
use lute_core::{
    ast::{self, NodeType, WalkStatus},
    html,
    lex::Lexer,
    lute::Lute,
    parse,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    go_file: String,
    go_test: String,
    name: String,
    api: String,
    source: String,
    expected: String,
}

#[test]
fn go_helper_fixture_cases() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/go_helpers.json")).unwrap();
    assert_eq!(38, cases.len());
    let mut failures = Vec::new();
    for case in cases {
        let mut engine = Lute::new();
        let actual = match case.api.as_str() {
            "markdown" => engine.markdown(&case.name, &case.source),
            "sanitize_markdown" => {
                engine.render_options.sanitize = true;
                engine.markdown(&case.name, &case.source)
            }
            "sanitize_vditor" => {
                engine.parse_options.vditor_wysiwyg = true;
                engine.render_options.vditor_wysiwyg = true;
                engine.render_options.sanitize = true;
                engine.spin_vditor_dom(&case.source)
            }
            "term_markdown" | "term_format" => {
                engine.render_options.auto_space = true;
                engine.render_options.fix_term_typo = true;
                if case.api == "term_markdown" {
                    engine.put_terms([("customtest".into(), "CUSTOMtest".into())].into());
                    engine.markdown(&case.name, &case.source)
                } else {
                    engine.format(&case.name, &case.source)
                }
            }
            _ => panic!("unknown fixture API"),
        };
        if case.expected != actual {
            failures.push(format!(
                "{}::{}/{} source={:?}\nexpected={:?}\nactual={:?}",
                case.go_file, case.go_test, case.name, case.source, case.expected, actual
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// lex/lexer_test.go: TestLexerNormalizeLines (including invalid UTF-8).
#[test]
fn go_test_lexer_normalize_lines() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"a", b"a\n"),
        (b"a\n", b"a\n"),
        (b"a\r", b"a\n"),
        (b"a\r\nb\rc\n", b"a\nb\nc\n"),
        (b"\r\r\n\n", b"\n\n\n"),
        (b"\0", "�\n".as_bytes()),
        (b"a\0\0b\r\nc\0\rd", "a��b\nc�\nd\n".as_bytes()),
        ("中文\r\n日文\0\r".as_bytes(), "中文\n日文�\n".as_bytes()),
        (b"\xff\r\n\xfe\0", b"\xff\n\xfe\xef\xbf\xbd\n"),
    ];
    for &(input, expected) in cases {
        let mut lexer = Lexer::new(input);
        let mut actual = Vec::new();
        while let Some(line) = lexer.next_line() {
            actual.extend(line);
        }
        assert_eq!(expected, actual, "input={input:?}");
        assert!(lexer.next_line().is_none());
    }
}

/// test/html_esc_test.go: TestEscapeAttrVal, TestEscapeHTMLStr, TestEscapeString.
#[test]
fn go_test_escape_roundtrips() {
    let source = "foo\n<bar>'baz&中 文";
    assert_eq!(
        source,
        html::unescape_attr_val(&html::escape_attr_val(source))
    );
    assert_eq!(
        source,
        html::unescape_html_str(&html::escape_html_str(source))
    );
    assert_eq!(source, html::unescape_string(&html::escape_string(source)));
}

/// test/html_escape_test.go: TestHTML2MarkdownEscapedText.
#[test]
fn go_test_html2markdown_escaped_text() {
    let mut engine = Lute::new();
    engine.parse_options.protyle_wysiwyg = true;
    engine.render_options.protyle_wysiwyg = true;
    let mut failures = Vec::new();
    for (name, source, expected) in [
        ("plain", r#"<p>a = b \=</p>"#, "a \\= b \\\\\\=\n"),
        ("rawSpan", "<span>$x=2$</span>", "$x=2$\n"),
        (
            "attributedSpan",
            r#"<span class="">$x=2$</span>"#,
            "\\$x\\=2\\$\n",
        ),
        (
            "link",
            r#"<a href="https://example.com">a = b</a>"#,
            "[a = b](https://example.com)\n",
        ),
        ("code", r#"<code>a = b \=</code>"#, "`a = b \\=`\n"),
        ("math", "<p>$$x=2$$</p>", "$$\nx=2\n$$\n"),
    ] {
        let (actual, error) = engine.html2markdown(source);
        assert!(error.is_none(), "{name}: {error:?}");
        if expected != actual {
            failures.push(format!("{name}: expected={expected:?}, actual={actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// test/html_escape_test.go: TestHTML2TreeEscapedText, both syntax settings.
#[test]
fn go_test_html2tree_escaped_text() {
    let source = r#"a = b \= \ *x* _y_ ~z~ $m$ ^n^ <t> `c`"#;
    for enabled in [true, false] {
        let mut engine = Lute::new();
        let options = &mut engine.parse_options;
        options.protyle_wysiwyg = true;
        options.text_mark = true;
        options.html_tag2_text_mark = true;
        options.inline_asterisk = enabled;
        options.inline_underscore = enabled;
        options.gfm_strikethrough = enabled;
        options.mark = enabled;
        options.sup = enabled;
        options.sub = enabled;
        engine.render_options.protyle_wysiwyg = true;
        for tag in ["strong", "em", "s", "mark", "sup", "sub"] {
            let mut tree =
                engine.html2tree(&format!("<{tag}>{}</{tag}>", html::escape_html(source)));
            parse::text_marks2inlines(&mut tree, &engine.parse_options);
            parse::nested_inlines2flatted_spans_hybrid(&mut tree, false);
            let mut actual = String::new();
            ast::walk(&tree.root(), &mut |node, entering| {
                if entering && node.typ() == NodeType::NodeTextMark {
                    assert!(node.is_text_mark_type(tag), "{tag}/syntax={enabled}");
                    actual.push_str(&html::unescape_string(&node.text()));
                }
                WalkStatus::WalkContinue
            });
            assert_eq!(source, actual, "{tag}/syntax={enabled}");
        }
    }
}

/// test/sanitizer_test.go: TestSanitize.
#[test]
fn go_test_sanitize() {
    assert_eq!(
        r#"<img src="foo" />"#,
        Lute::sanitize(r#"<img src="foo" onload="alert(1)" onerror="alert(2)"/>"#)
    );
    assert_eq!(
        "![a](&#34;<img src=\"xss\">)\n",
        Lute::sanitize("![a](&quot;<img src=xss onerror=alert(1)>)\n")
    );
}

/// h2m.go: escapeHTMLTextMarkers must split AST nodes, preserving text and links.
#[test]
fn html2tree_marker_escaping_preserves_ast_structure() {
    let mut engine = Lute::new();
    engine.parse_options.protyle_wysiwyg = true;
    engine.render_options.protyle_wysiwyg = true;
    let tree = engine.html2tree("<p>a=$x$</p>");
    let paragraph = tree.root().first_child().unwrap();
    let children: Vec<_> = paragraph.children().collect();
    assert_eq!(
        vec![
            NodeType::NodeText,
            NodeType::NodeBackslash,
            NodeType::NodeBackslash,
            NodeType::NodeText,
            NodeType::NodeBackslash
        ],
        children.iter().map(|node| node.typ()).collect::<Vec<_>>()
    );
    assert_eq!(b"a", children[0].tokens().as_slice());
    assert_eq!(b"x", children[3].tokens().as_slice());
    for (index, marker) in [(1, b'='), (2, b'$'), (4, b'$')] {
        let content = children[index].first_child().unwrap();
        assert_eq!(NodeType::NodeBackslashContent, content.typ());
        assert_eq!(vec![marker], content.tokens());
        assert_eq!(Some(children[index].clone()), content.parent());
        assert!(content.next().is_none());
    }
    for (index, child) in children.iter().enumerate() {
        assert_eq!(Some(paragraph.clone()), child.parent());
        assert_eq!(
            index.checked_sub(1).map(|i| children[i].clone()),
            child.previous()
        );
        assert_eq!(children.get(index + 1).cloned(), child.next());
    }
    assert_eq!("a\\=\\$x\\$\n", engine.html2md("<p>a=$x$</p>"));
    assert_eq!("$x=2$\n", engine.html2md("<span>$x=2$</span>"));
    assert_eq!(
        "[a = b](https://example.com)\n",
        engine.html2md("<a href=\"https://example.com\">a = b</a>")
    );
}
