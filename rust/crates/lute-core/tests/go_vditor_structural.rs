//! Original Go editor round-trip and DOM structure assertions. No Rust snapshots.
use lute_core::{
    dom::{self, DomType},
    lute::Lute,
};

fn set_editor(engine: &mut Lute, mode: &str) {
    match mode {
        "wysiwyg" => {
            engine.parse_options.vditor_wysiwyg = true;
            engine.render_options.vditor_wysiwyg = true;
        }
        "sv" => {
            engine.parse_options.vditor_sv = true;
            engine.render_options.vditor_sv = true;
        }
        _ => unreachable!(),
    }
}
fn browser_text(dom: &dom::Dom, node: usize) -> String {
    if dom.nodes[node].typ == DomType::Text {
        return dom.nodes[node].data.clone();
    }
    dom.nodes[node]
        .children
        .iter()
        .map(|&c| browser_text(dom, c))
        .collect()
}

// Go: test/vditor_table_br_test.go::TestVditorTableCellBrRoundTrip
#[test]
fn go_test_vditor_table_cell_br_round_trip() {
    let l = Lute::new();
    for (name, md, content, dom_count, md_count) in [
        ("middle", "| c |\n| - |\n| 1<br />2 |\n", "1<br />2", 1, 1),
        (
            "compact",
            "| c |\n| - |\n| 1<br>2<br/>3 |\n",
            "1<br />2<br />3",
            2,
            2,
        ),
        (
            "uppercase",
            "| c |\n| - |\n| 1<BR />2 |\n",
            "1<br />2",
            1,
            1,
        ),
        ("trailing", "| c |\n| - |\n| 1<br /> |\n", "1<br />", 2, 1),
        (
            "consecutive",
            "| c |\n| - |\n| 1<br /><br /> |\n",
            "1<br /><br />",
            3,
            2,
        ),
    ] {
        let dom = l.md2vditor_ir_dom(md);
        assert_eq!(dom.matches("<br />").count(), dom_count, "{name}: {dom}");
        assert!(!dom.contains("data-type=\"html-inline\""), "{name}: {dom}");
        let markdown = l.vditor_ir_dom2md(&dom);
        assert_eq!(
            markdown.matches("<br />").count(),
            md_count,
            "{name}: {markdown}"
        );
        assert!(markdown.contains(content), "{name}: {markdown}");
    }
    assert!(l
        .md2vditor_ir_dom("1<br />2")
        .contains("data-type=\"html-inline\""));
}
// Go: test/vditor_table_br_test.go::TestSpinVditorTableCellBr
#[test]
fn go_test_spin_vditor_table_cell_br() {
    let l = Lute::new();
    for (name, cell, content, dom_count, md_count) in [
        ("trailing", "1<br><wbr><br>", "1<br /><wbr><br />", 2, 1),
        (
            "consecutive",
            "1<br><br><wbr><br>",
            "1<br /><br /><wbr><br />",
            3,
            2,
        ),
        ("deleted", "1<wbr><br>", "<td>1<wbr></td>", 0, 0),
    ] {
        let input=format!("<table data-block=\"0\"><thead><tr><th>c</th></tr></thead><tbody><tr><td>{cell}</td></tr></tbody></table>");
        let dom = l.spin_vditor_ir_dom(&input);
        assert!(dom.contains(content), "{name}: {dom}");
        assert_eq!(dom.matches("<br />").count(), dom_count, "{name}: {dom}");
        assert!(!dom.contains("data-type=\"html-inline\""), "{name}: {dom}");
        let md = l.vditor_ir_dom2md(&dom);
        assert_eq!(md.matches("<br />").count(), md_count, "{name}: {md}");
        assert_eq!(l.spin_vditor_ir_dom(&dom), dom, "{name}");
    }
}
// Go: test/list_setext_continuation_test.go::TestFormatListSetextContinuation
#[test]
fn go_test_format_list_setext_continuation() {
    let l = Lute::new();
    for (md, want) in [
        ("* a\n* b\n=", "* a\n* b\n  \\=\n"),
        ("* a\n* b\n--", "* a\n* b\n  \\--\n"),
        ("1. a\n2. b\n===", "1. a\n2. b\n   \\===\n"),
        ("* a\n  * b\n=", "* a\n  * b\n    \\=\n"),
        ("* a\n* b\n =", "* a\n* b\n  \\=\n"),
    ] {
        let out = l.format("", md);
        assert_eq!(out, want, "{md:?}");
        assert_eq!(l.format("", &out), out);
        assert_eq!(l.markdown("", md), l.markdown("", &out));
    }
    assert_eq!(l.format("heading", "* a\n* b\n  ="), "* a\n* b\n  =\n");
}
// Go: test/list_setext_continuation_test.go::TestVditorDOMListSetextContinuation
#[test]
fn go_test_vditor_dom_list_setext_continuation() {
    let mut l = Lute::new();
    set_editor(&mut l, "wysiwyg");
    let dom="<ul data-tight=\"true\" data-marker=\"*\" data-block=\"0\"><li data-marker=\"*\">a</li><li data-marker=\"*\">b\n-<wbr></li></ul>";
    assert_eq!(l.vditor_dom2md(dom), "* a\n* b\n  \\-\n");
    let spun = l.spin_vditor_dom(dom);
    assert!(
        !spun.contains("<h2") && spun.contains("data-type=\"backslash\""),
        "{spun}"
    );
}
// Go: test/list_setext_continuation_test.go::TestSpinVditorSVListSetextContinuation
#[test]
fn go_test_spin_vditor_sv_list_setext_continuation() {
    let mut l = Lute::new();
    set_editor(&mut l, "sv");
    for marker in ["-", "--", "=", "==="] {
        let spun = l.spin_vditor_sv_dom(&format!("* a\n* b\n{marker}‸"));
        assert!(
            !spun.contains("heading-marker") && spun.contains(&format!("\\{marker}")),
            "{marker}: {spun}"
        );
        let dom = dom::parse(&spun);
        let second = l.spin_vditor_sv_dom(&dom.text(dom.root));
        assert!(!second.contains("heading-marker"), "{marker}: {second}");
    }
}
// Go: test/list_setext_continuation_test.go::TestSpinVditorSVThematicBreakAtDocumentStart
#[test]
fn go_test_spin_vditor_sv_thematic_break_at_document_start() {
    let mut l = Lute::new();
    set_editor(&mut l, "sv");
    let spun = l.spin_vditor_sv_dom("***\n‸");
    assert!(
        !spun.contains("yaml-front-matter") && spun.contains(">***</span>"),
        "{spun}"
    );
    let dom = dom::parse(&spun);
    let second = l.spin_vditor_sv_dom(&dom.text(dom.root));
    assert!(
        !second.contains("yaml-front-matter") && second.contains(">***</span>"),
        "{second}"
    );
}
// Go: test/vditor_callout_test.go::TestVditorCalloutRoundTrip
#[test]
fn go_test_vditor_callout_round_trip() {
    for md in [
        "> [!NOTE]\n> Content",
        "> [!TIP] Custom **title**\n> First\n>\n> Second",
        "> [!CUSTOM] ✂️ Custom title\n> * List\n>   > [!WARNING]\n>   > Nested",
        "> [!IMPORTANT] ![callout-icon](<https://example.com/icon.png>) Image\n> Content",
    ] {
        let mut l = Lute::new();
        l.parse_options.callout = true;
        l.render_options.sanitize = true;
        let want = l.markdown("", md);
        let wysiwyg = l.md2vditor_dom(md);
        assert!(
            wysiwyg.contains("data-type=\"callout\"")
                && !wysiwyg.contains("not found render function"),
            "{md}: {wysiwyg}"
        );
        assert_eq!(l.markdown("", &l.vditor_dom2md(&wysiwyg)), want, "{md}");
        let ir = l.md2vditor_ir_dom(md);
        assert!(
            ir.contains("data-type=\"callout\"") && !ir.contains("not found render function"),
            "{md}: {ir}"
        );
        assert_eq!(l.markdown("", &l.vditor_ir_dom2md(&ir)), want, "{md}");
        let sv = l.md2vditor_sv_dom(md);
        assert!(
            !sv.contains("not found render function") && sv.contains("[!"),
            "{md}: {sv}"
        );
        let dom = dom::parse(&sv);
        assert_eq!(l.markdown("", &browser_text(&dom, dom.root)), want, "{md}");
    }
}
// Go: test/vditor_callout_test.go::TestCalloutTypeAttributeEscaping
#[test]
fn go_test_callout_type_attribute_escaping() {
    let mut l = Lute::new();
    l.parse_options.callout = true;
    l.render_options.sanitize = true;
    let md = "> [!\" onclick=\"alert(1)] Title\n> Content";
    let bd = l.md2blockdom(md, false);
    for (name, output) in [
        ("HTML", l.markdown("", md)),
        ("WYSIWYG", l.md2vditor_dom(md)),
        ("IR", l.md2vditor_ir_dom(md)),
        ("BlockDOM", bd.clone()),
        ("Protyle preview", l.protyle_preview_str("", md)),
        ("Protyle export", l.blockdom2html(&bd)),
    ] {
        assert!(!output.contains(" onclick=\""), "{name}: {output}");
    }
}
// Go: test/spin_wysiwyg_test.go::TestSpinVditorDOMTableInList
#[test]
fn go_test_spin_vditor_dom_table_in_list() {
    let mut l = Lute::new();
    set_editor(&mut l, "wysiwyg");
    for (tag, marker) in [("ul", "*"), ("ol", "1.")] {
        let input=format!("<{tag} data-marker=\"{marker}\" data-block=\"0\"><li data-marker=\"{marker}\"><p data-block=\"0\">test</p><table data-block=\"0\"><thead><tr><th>col1</th></tr></thead><tbody><tr><td>12<wbr></td></tr></tbody></table></li></{tag}>");
        let want=format!("<{tag} data-tight=\"true\" data-marker=\"{marker}\" data-block=\"0\"><li data-marker=\"{marker}\">test<table data-block=\"0\"><thead><tr><th>col1</th></tr></thead><tbody><tr><td>12<wbr></td></tr></tbody></table></li></{tag}>");
        let once = l.spin_vditor_dom(&input);
        assert_eq!(once, want);
        assert_eq!(l.spin_vditor_dom(&once), want);
    }
}
// Go: test/h2v_test.go::TestHTML2VditorDOMToC
#[test]
fn go_test_html2vditor_dom_toc() {
    let mut l = Lute::new();
    l.parse_options.toc = true;
    l.render_options.toc = true;
    let input = "<p>[ToC]</p><h2>教_程</h2><h2>array_contains</h2>";
    let expected="<div class=\"vditor-toc\" data-block=\"0\" data-type=\"toc-block\" contenteditable=\"false\"><ul><li><span data-target-id=\"教程\">教_程</span></li><li><span data-target-id=\"arraycontains\">array_contains</span></li></ul></div><h2 data-block=\"0\" id=\"wysiwyg-教程\" data-marker=\"#\">教<span data-type=\"backslash\"><span>\\</span>_</span>程</h2><h2 data-block=\"0\" id=\"wysiwyg-arraycontains\" data-marker=\"#\">array<span data-type=\"backslash\"><span>\\</span>_</span>contains</h2>";
    assert_eq!(l.html2vditor_dom(input), expected);
}
// Go: test/html_test.go::TestHTMLParse
#[test]
fn go_test_html_parse() {
    let dom = dom::parse("<p>foo</p>");
    assert_eq!(dom.nodes[dom.root].children.len(), 1);
    assert!(dom.is_element(dom.nodes[dom.root].children[0], "p"));
}
// Go: test/html_test.go::TestHTMLParse0
#[test]
fn go_test_html_parse0() {
    let dom = dom::parse("<p>&& &nbsp;</p>");
    assert_eq!(dom.nodes[dom.root].children.len(), 1);
    assert!(dom.is_element(dom.nodes[dom.root].children[0], "p"));
}

// Go: test/callout_test.go::TestHTML2MdCallout
#[test]
fn go_test_html2md_callout() {
    let mut l = Lute::new();
    l.parse_options.callout = true;
    for (name, html, expected) in [
        (
            "nested",
            r#"<div class="callout extra" data-subtype="NOTE"><div class="callout-info"><span class="callout-icon">✏️</span><span class="callout-title">Note</span></div><div class="callout-content"><p>Outer</p><div class="callout" data-subtype="WARNING"><div class="callout-info"><span class="callout-icon">⚠️</span><span class="callout-title">Warning</span></div><div class="callout-content"><p>Inner</p></div></div></div></div>"#,
            "> [!NOTE]\n> Outer\n>\n>> [!WARNING]\n>> Inner\n>>\n",
        ),
        (
            "custom title",
            r#"<div class="callout" data-subtype="TIP"><div class="callout-info"><span class="callout-icon">✨</span><span class="callout-title"><strong>Custom</strong> title</span></div><div class="callout-content"><p>First</p><ul><li>Second</li></ul></div></div>"#,
            "> [!TIP] ✨ **Custom** title\n> First\n>\n> * Second\n",
        ),
        (
            "image icon",
            r#"<div class="callout" data-subtype="IMPORTANT"><div class="callout-info"><span class="callout-icon"><img class="callout-img" src="https://example.com/icon.png"></span><span class="callout-title">Custom</span></div><div class="callout-content"><p>Content</p></div></div>"#,
            "> [!IMPORTANT] ![callout-icon](<https://example.com/icon.png>) Custom\n> Content\n",
        ),
    ] {
        assert_eq!(l.html2md(html), expected, "{name}");
        let dom = l.html2vditor_dom(html);
        assert!(dom.contains("data-type=\"callout\""), "{name}: {dom}");
    }
}

// Go: test/parse_test.go::TestCase1 (original smoke/logging test, no HTML equality assertion).
#[test]
fn go_test_case1_smoke() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let input = std::fs::read_to_string(root.join("conformance/fixtures/case1.md")).unwrap();
    let _html = Lute::new().markdown("case1.md", &input);
}

// Go: test/footnotes_test.go::TestFootnoteLabelInlineTokens
#[test]
fn go_test_footnote_label_inline_tokens() {
    let reference = r##"<sup class="footnotes-ref" id="footnotes-ref-1"><a href="#footnotes-def-1">1</a></sup>"##;
    for protyle in [false, true] {
        for sup in [false, true] {
            for label in [
                "a_b", "a*b", "a&b", "a<b", "a=b", "a~b", "a^b", "a$b", "a#b", "a!b", "a(b)",
                "a`b", "a&amp;b",
            ] {
                let mut engine = Lute::new();
                engine.parse_options.protyle_wysiwyg = protyle;
                engine.render_options.protyle_wysiwyg = protyle;
                engine.parse_options.sup = sup;
                for (source, expected) in [
                    (
                        format!("Text[^{label}] and *after*."),
                        format!("<p>Text{reference} and <em>after</em>.</p>\n"),
                    ),
                    (
                        format!("*before[^{label}] after*."),
                        format!("<p><em>before{reference} after</em>.</p>\n"),
                    ),
                    (
                        format!("Text[caption_*][^{label}]."),
                        format!("<p>Text{reference}.</p>\n"),
                    ),
                ] {
                    let markdown = format!("{source}\n\n[^{label}]: body\n");
                    let actual = engine.markdown("", &markdown);
                    assert!(actual.starts_with(&expected),"protyle={protyle} sup={sup} label={label:?}\nexpected prefix={expected:?}\nactual={actual:?}");
                }
            }
        }
    }
}
