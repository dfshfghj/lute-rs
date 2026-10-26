use lute_core::dom::is_known_tag;

#[test]
fn known_names_match_go_atom_lookup_including_foreign_and_attribute_names() {
    for name in [
        "svg",
        "math",
        "foreignobject",
        "annotation-xml",
        "onclick",
        "p",
        "span",
    ] {
        assert!(is_known_tag(name), "Go html/atom recognizes {name}");
    }
    for name in ["use", "custom-element", "lute-unknown"] {
        assert!(
            !is_known_tag(name),
            "Go html/atom does not recognize {name}"
        );
    }
}

#[test]
fn dom_serialization_uses_go_html_escape_for_text_and_attributes() {
    let dom = lute_core::dom::parse("<div title=\"&quot;&#39;\">\"' &amp; &lt;</div>");
    assert_eq!(
        dom.html(dom.root),
        "<div title=\"&#34;&#39;\">&#34;&#39; &amp; &lt;</div>"
    );
}

#[test]
fn standard_html_tabs_use_inline_titles_and_preserve_boundary_spaces() {
    let mut lute = lute_core::lute::Lute::new();
    lute.parse_options.tabs = true;
    let dom = "<div class=\"tabs\" tabs-position=\"left\"><div class=\"tab-item\"><div class=\"tab-item-info\"><span class=\"tab-item-title\"> <strong>Title</strong> </span></div><div class=\"tab-item-content\"><p>Body</p></div></div></div>";
    assert_eq!(
        lute.html2md(dom),
        "::: tabs\n@tab &#32;**Title**&#32;\n\nBody\n\n:::\n"
    );
}

#[test]
fn html2tree_runs_shared_dom_normalization_before_building_ast() {
    let lute = lute_core::lute::Lute::new();
    let html = "<table>\n<thead>\n<tr><th>a</th><th>b</th></tr>\n</thead>\n<tbody>\n<tr><td>x</td><td>y</td></tr>\n</tbody>\n</table>";
    assert_eq!(lute.html2md(html), "| a | b |\n| --- | --- |\n| x | y |\n");
    assert_eq!(
        lute.html2md("<p><span class=\"language-math\">x</span></p>"),
        "$x$\n"
    );
}

#[test]
fn math_display_detection_searches_nested_spans_as_go_dom_helper_does() {
    let lute = lute_core::lute::Lute::new();
    let html = "<p>a<span class=\"language-math\" data-math=\"x\"><span><span class=\"katex-display\">rendered</span></span></span>b</p>";
    assert_eq!(lute.html2md(html), "a\n\n$$\nx\n$$\n\nb\n");
}
