//! Original Go style/export tests. Expectations are copied from Go, never Rust output.
use lute_core::lute::Lute;

fn engine() -> Lute {
    let mut engine = Lute::new();
    engine.parse_options.protyle_wysiwyg = true;
    engine.render_options.protyle_wysiwyg = true;
    engine.parse_options.html_tag2_text_mark = true;
    engine.parse_options.kramdown_block_ial = true;
    engine.parse_options.kramdown_span_ial = true;
    engine.render_options.kramdown_block_ial = true;
    engine.render_options.kramdown_span_ial = true;
    engine
}

fn assert_contains(output: &str, expected: &[&str], context: &str) {
    for want in expected {
        assert!(
            output.contains(want),
            "{context}: missing {want:?} in {output}"
        );
    }
}

// Go: test/html_text_style_test.go::TestHTMLTextStyle
#[test]
fn go_html_text_style() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/go_style_structural.json")).unwrap();
    let cases = fixture["html_text_style"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    let engine = engine();
    let mut failures = Vec::new();
    for case in cases {
        let output = engine.html2blockdom(case["input"].as_str().unwrap());
        if output.contains("data-href=\"\"") {
            failures.push(format!("{}: empty link in {output}", case["name"]));
        }
        for want in case["expected"].as_array().unwrap() {
            if !output.contains(want.as_str().unwrap()) {
                failures.push(format!("{}: missing {want} in {output}", case["name"]));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Go: test/html_text_style_test.go::TestHTMLTextStyleRoundTrip
#[test]
fn go_html_text_style_round_trip() {
    let engine = engine();
    let mut dom = engine.html2blockdom(r#"<p><span style="color:red;font-family:Arial;font-size:24px">A<strong>B</strong><a href="https://example.com">C</a><small>D</small></span></p>"#);
    for round in 0..3 {
        dom = engine.spin_blockdom(&dom);
        assert_contains(
            &dom,
            &[
                "color: red;",
                "font-family: Arial;",
                "font-size: 24.000000px;",
                "font-size: 20.000000px;",
                "data-href=\"https://example.com\"",
            ],
            &format!("round {round}"),
        );
    }
}

// Go: test/wps_text_style_test.go::TestWPSTextStyle
#[test]
fn go_wps_text_style() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/go_style_structural.json")).unwrap();
    let engine = engine();
    let mut dom = engine.html2blockdom(fixture["wps"].as_str().unwrap());
    let mut failures = Vec::new();
    for round in 0..3 {
        for want in [
            "background-color: rgb(255,165,61);",
            "color: rgb(216,57,49);",
            "font-family: 宋体;",
            "font-size: 16.000000px;",
            "data-type=\"u\" style=\"font-family: 宋体;font-size: 16.000000px;\">4444</span>",
        ] {
            if !dom.contains(want) {
                failures.push(format!("round {round}: missing {want:?} in {dom}"));
            }
        }
        if dom.contains("{: style=") {
            failures.push(format!(
                "round {round}: style syntax leaked into content: {dom}"
            ));
        }
        dom = engine.spin_blockdom(&dom);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Go: test/p2h_test.go::TestBlockDOM2RichHTML
#[test]
fn go_blockdom2richhtml() {
    let mut engine = engine();
    // Original only sets ProtyleWYSIWYG, KramdownIAL and TextMark.
    engine.parse_options.html_tag2_text_mark = false;
    engine.parse_options.text_mark = true;
    let dom = "<div data-node-id=\"20240101000000-abcdefg\" data-type=\"NodeParagraph\" class=\"p\"><div contenteditable=\"true\" spellcheck=\"false\">foo <span data-type=\"strong em\">bar</span> <span data-type=\"a strong\" data-href=\"https://b3log.org\" data-title=\"B3log\">baz</span></div><div class=\"protyle-attr\" contenteditable=\"false\">\u{200b}</div></div>";
    assert_eq!(engine.blockdom2richhtml(dom), "<p id=\"20240101000000-abcdefg\">foo <strong><em>bar</em></strong> <a href=\"https://b3log.org\" title=\"B3log\"><strong>baz</strong></a></p>\n");
}
