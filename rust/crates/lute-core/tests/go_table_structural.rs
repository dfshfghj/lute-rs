//! Structural Go assertions. Fixtures contain verbatim Go literals, not Rust snapshots.
//! Missing native renderer/rich-cell APIs are listed in the migration coverage report.
use lute_core::{
    ast::{walk, WalkStatus, TESTING},
    lute::Lute,
    render::{protyle_preview::ProtylePreviewRenderer, Renderer},
};
use serde_json::Value;

fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/go_table_structural.json")).unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn engine() -> Lute {
    let mut e = Lute::new();
    e.parse_options.protyle_wysiwyg = true;
    e.render_options.protyle_wysiwyg = true;
    e.parse_options.kramdown_block_ial = true;
    e.parse_options.kramdown_span_ial = true;
    e.render_options.kramdown_block_ial = true;
    e.render_options.kramdown_span_ial = true;
    e
}
fn rich_engine() -> Lute {
    let mut e = engine();
    let p = &mut e.parse_options;
    p.text_mark = true;
    p.html_tag2_text_mark = true;
    p.block_ref = true;
    p.file_annotation_ref = true;
    p.emoji = false;
    p.super_block = true;
    p.custom_block = true;
    p.git_conflict = true;
    p.callout = true;
    p.tabs = true;
    p.img_path_allow_space = true;
    p.sup = true;
    p.sub = true;
    p.tag = true;
    p.mark = true;
    p.gfm_strikethrough1 = false;
    p.full_width_strikethrough = true;
    p.inline_math_allow_digit_after_open_marker = true;
    p.footnotes = false;
    p.toc = false;
    p.heading_id = false;
    p.setext = false;
    p.yaml_front_matter = false;
    p.link_ref = false;
    p.indent_code_block = false;
    p.paragraph_beginning_space = true;
    p.arbitrary_task_list_item_marker = true;
    p.ensure_list_item_paragraph = true;
    e
}
fn rich_json(e: &Lute, dom: &str) -> bool {
    let tree = e.blockdom2tree(dom);
    lute_core::render::json::render_json(&tree, &e.render_options).contains("\"TableCellRich\"")
}
#[derive(Default)]
struct Checks {
    failures: Vec<String>,
}
impl Checks {
    fn check(&mut self, ok: bool, case: impl std::fmt::Display, output: &str) {
        if !ok {
            self.failures
                .push(format!("Case: {case}\nActual: {output}"));
        }
    }
    fn finish(self, source: &str) {
        assert!(
            self.failures.is_empty(),
            "Source: {source}\n{}",
            self.failures.join("\n")
        );
    }
}

// Go: test/protyle_table_spans_test.go::TestBlockDOMTableSpans
#[test]
fn go_blockdom_table_spans() {
    let fixture = fixtures();
    let mut checks = Checks::default();
    for case in fixture["spans"].as_array().unwrap() {
        let e = engine();
        let name = text(&case["name"]);
        let expected = text(&case["expected"]);
        let mut source = format!("<div data-type=\"NodeTable\"><div><table><thead>{}</thead><tbody>{}</tbody></table></div></div>", text(&case["head"]),text(&case["body"]));
        for round in 0..3 {
            source = e.spin_blockdom(&source);
            checks.check(
                source.contains(expected),
                format!("{name}/spin-{round}"),
                &source,
            );
        }
        if name != "separate row groups" {
            let md = e.blockdom2stdmd(&source);
            checks.check(
                !md.contains("fn__none"),
                format!("{name}/export-placeholder"),
                &md,
            );
            let restored = e.html2blockdom(&md);
            let imported = case
                .get("imported")
                .map(text)
                .filter(|s| !s.is_empty())
                .unwrap_or(expected);
            checks.check(
                restored.contains(imported),
                format!("{name}/export-import"),
                &restored,
            );
        }
    }
    checks.finish("test/protyle_table_spans_test.go::TestBlockDOMTableSpans");
}

// Go: test/protyle_merged_table_backslash_test.go::TestBlockDOM2StdMdMergedTablePreservesBackslashContent
#[test]
fn go_merged_table_markdown_backslash() {
    let output = engine().blockdom2stdmd(text(&fixtures()["merged"]));
    assert!(output.contains(r"<td>\*raj-s</td>") && output.contains(r"<td>\*[r]aj</td>"),"Source: test/protyle_merged_table_backslash_test.go::TestBlockDOM2StdMdMergedTablePreservesBackslashContent\nCase: merged\n{output}");
}
// Go: test/protyle_merged_table_backslash_test.go::TestBlockDOM2RichHTMLMergedTablePreservesBackslashContent
#[test]
fn go_merged_table_html_backslash() {
    let output = engine().blockdom2richhtml(text(&fixtures()["merged"]));
    assert!(output.contains("<td>*raj-s</td>") && output.contains("<td>*[r]aj</td>"),"Source: test/protyle_merged_table_backslash_test.go::TestBlockDOM2RichHTMLMergedTablePreservesBackslashContent\nCase: merged\n{output}");
}

// Go: test/h2p_test.go::TestHTML2BlockDOMTableCaption
#[test]
fn go_html2blockdom_table_caption() {
    TESTING.store(true, std::sync::atomic::Ordering::SeqCst);
    let mut e = engine();
    e.parse_options.html2_markdown_attrs = vec!["custom-*".into()];
    let f = fixtures();
    let c = &f["caption"];
    assert_eq!(
        e.html2blockdom(text(&c["from"])),
        text(&c["expected"]),
        "Source: test/h2p_test.go::TestHTML2BlockDOMTableCaption\nCase: caption"
    );
}

// Go: test/table_whitespace_test.go::TestVditorTableInlineWhitespaceRoundTrip
#[test]
fn go_vditor_table_inline_whitespace_round_trip() {
    let e = Lute::new();
    let md="| a *em* b **strong** c `code` d [link](https://example.com) e ~~del~~ f |\n| --- |\n| x |\n";
    let expected = "| a *em* b **strong** c `code` d [link](https://example.com) e ~~del~~ f |";
    let mut checks = Checks::default();
    for (name, actual) in [
        ("wysiwyg", e.vditor_dom2md(&e.md2vditor_dom(md))),
        ("ir", e.vditor_ir_dom2md(&e.md2vditor_ir_dom(md))),
    ] {
        checks.check(actual.split('\n').next() == Some(expected), name, &actual);
    }
    checks.finish("test/table_whitespace_test.go::TestVditorTableInlineWhitespaceRoundTrip");
}
// Go: test/table_whitespace_test.go::TestHTML2MdTableInlineWhitespace
#[test]
fn go_html2md_table_inline_whitespace() {
    let e = Lute::new();
    let html="<table>\n<thead><tr><th>\n a <em>em</em> b <strong>strong</strong> c <code>code</code> d <a href=\"https://example.com\">link</a> e <del>del</del> f \n</th></tr></thead>\n<tbody><tr><td>x</td></tr></tbody>\n</table>";
    let actual = e.html2md(html);
    assert_eq!(actual.split('\n').next().unwrap(),"| a *em* b **strong** c `code` d [link](https://example.com) e ~~del~~ f |","Source: test/table_whitespace_test.go::TestHTML2MdTableInlineWhitespace\nCase: inline-whitespace");
}

// Go: test/markdown_table_html_style_test.go::TestMarkdownTableHTMLThemeStyles
#[test]
fn go_markdown_table_html_theme_styles() {
    let mut e = rich_engine();
    e.parse_options.html_tag2_text_mark = true;
    e.render_options.auto_space = false;
    let mut checks = Checks::default();
    let f = fixtures();
    for case in f["theme"].as_array().unwrap() {
        let md = format!("| Header |\n| --- |\n| {} |\n", text(&case["cell"]));
        for auto in [false, true] {
            let mut dom = if auto {
                e.md2blockdom_with_auto_link(&md, false)
            } else {
                e.md2blockdom(&md, false)
            };
            for round in 0..2 {
                let id = format!("{}/auto-{auto}/round-{round}", text(&case["name"]));
                checks.check(
                    !dom.contains("&lt;span")
                        && dom.matches("data-type=\"NodeTable\"").count() == 1,
                    &id,
                    &dom,
                );
                for want in case["want"].as_array().unwrap() {
                    checks.check(
                        dom.contains(text(want)),
                        format!("{id}/contains-{}", text(want)),
                        &dom,
                    );
                }
                dom = e.spin_blockdom(&dom);
            }
        }
    }
    checks.finish("test/markdown_table_html_style_test.go::TestMarkdownTableHTMLThemeStyles");
}
// Go: test/markdown_table_html_style_test.go::TestMarkdownTableHTMLThemeStyleBoundaries
#[test]
fn go_markdown_table_html_theme_style_boundaries() {
    let mut e = Lute::new();
    e.parse_options.protyle_wysiwyg = true;
    e.render_options.protyle_wysiwyg = true;
    e.parse_options.html_tag2_text_mark = true;
    e.parse_options.text_mark = true;
    let mut checks = Checks::default();
    let f = fixtures();
    for (i, input) in f["theme_boundaries"].as_array().unwrap().iter().enumerate() {
        let (dom, tree) = e.md2blockdomtree(text(input), false);
        checks.check(dom.contains("&lt;span"), format!("{i}/literal"), &dom);
        walk(&tree.root(), &mut |n, entering| {
            if entering {
                checks.check(
                    n.ial_attr("style").is_empty(),
                    format!("{i}/no-style"),
                    &n.dump(),
                );
            }
            WalkStatus::WalkContinue
        });
    }
    checks.finish(
        "test/markdown_table_html_style_test.go::TestMarkdownTableHTMLThemeStyleBoundaries",
    );
}

// Go: test/markdown_table_structure_test.go::TestMarkdownTableStructureBoundaries
#[test]
fn go_markdown_table_structure_boundaries() {
    let e = rich_engine();
    let mut checks = Checks::default();
    let f = fixtures();
    for (i, source) in f["structure_boundaries"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let dom = e.md2blockdom(&format!("| head |\n| --- |\n| {} |", text(source)), false);
        checks.check(!rich_json(&e, &dom), i, &dom);
    }
    let dom = r#"<div data-type="NodeTable"><div contenteditable="true"><table><thead><tr><th>head</th></tr></thead><tbody><tr><td>- [ ] existing text</td></tr></tbody></table></div></div>"#;
    let actual = e.spin_blockdom(dom);
    checks.check(!rich_json(&e, &actual), "existing ordinary cell", &actual);
    checks.finish("test/markdown_table_structure_test.go::TestMarkdownTableStructureBoundaries");
}

// Go: test/table_rich_test.go::TestTableRichDoesNotReinterpretLegacyCells
#[test]
fn go_table_rich_does_not_reinterpret_legacy_cells() {
    let e = rich_engine();
    let dom = e.md2blockdom("| title |\n| --- |\n| - literal<br />1. literal |", false);
    let mut c = Checks::default();
    c.check(
        !rich_json(&e, &e.spin_blockdom(&dom)),
        "no-rich-conversion",
        &dom,
    );
    c.check(
        !dom.contains("<ul>") && !dom.contains("<ol>"),
        "no-lists",
        &dom,
    );
    c.finish("test/table_rich_test.go::TestTableRichDoesNotReinterpretLegacyCells");
}

// Go: test/table_alignment_style_test.go::TestStandardMarkdownTableOmitsCellAttributes
#[test]
fn go_standard_markdown_table_omits_cell_attributes() {
    let e = engine();
    let dom = r#"<div data-type="NodeTable"><div contenteditable="true"><table><thead><tr><th style="text-align: left;">Header</th><th style="text-align: right;">Other</th></tr></thead><tbody><tr><td style="text-align: center; background-color: red;"><strong>value</strong></td><td style="text-align: right;">next</td></tr></tbody></table></div></div>"#;
    let md = e.blockdom2stdmd(dom);
    let mut c = Checks::default();
    for unwanted in ["{: ", "style=", "text-align", "background-color"] {
        c.check(!md.contains(unwanted), format!("omit-{unwanted}"), &md);
    }
    for expected in ["Header", "Other", "**value**", "next", "| :", ": |"] {
        c.check(md.contains(expected), format!("keep-{expected}"), &md);
    }
    let saved = e.blockdom2md(dom);
    c.check(
        saved.contains("background-color: red;"),
        "internal-style",
        &saved,
    );
    c.finish("test/table_alignment_style_test.go::TestStandardMarkdownTableOmitsCellAttributes");
}
// Go: test/table_alignment_style_test.go::TestHTMLTableCellAlignmentDifferencesSurviveMarkdown
#[test]
fn go_html_table_cell_alignment_differences_survive_markdown() {
    let e = Lute::new();
    let input = r#"<table><thead><tr><th style="text-align: center">Header</th><th>Other</th></tr></thead><tbody><tr><td style="text-align: right">different</td><td>same</td></tr></tbody></table>"#;
    let md = e.html2md(input);
    let mut c = Checks::default();
    c.check(
        md.contains("text-align: right;") && md.contains("| :-"),
        "markdown",
        &md,
    );
    let dom = e.md2blockdom(&md, false);
    c.check(
        dom.contains("text-align: right;") && dom.contains("text-align: center;"),
        "round-trip",
        &dom,
    );
    c.finish(
        "test/table_alignment_style_test.go::TestHTMLTableCellAlignmentDifferencesSurviveMarkdown",
    );
}

// Go: test/table_body_test.go::TestTableRenderersOmitEmptyBody
// Go: test/table_body_test.go::TestTableRenderersKeepNonemptyBody
// The export-renderer branch has no native Rust renderer and is recorded as unported.
#[test]
fn go_table_body_blockdom_and_preview() {
    let e = Lute::new();
    let mut c = Checks::default();
    for (name, md, body) in [
        (
            "TestTableRenderersOmitEmptyBody",
            "| foo |\n| --- |\n",
            false,
        ),
        (
            "TestTableRenderersKeepNonemptyBody",
            "| foo |\n| --- |\n| bar |\n",
            true,
        ),
    ] {
        let (dom, tree) = e.md2blockdomtree(md, false);
        let mut preview =
            ProtylePreviewRenderer::new(&tree, e.render_options.clone(), e.parse_options.clone());
        for (renderer, out) in [
            ("block DOM", dom),
            ("preview", String::from_utf8(preview.render()).unwrap()),
        ] {
            c.check(
                out.contains("<tbody") == body && out.contains("</tbody>") == body,
                format!("{name}/{renderer}"),
                &out,
            );
        }
    }
    c.finish("test/table_body_test.go");
}

// Go: test/table_alignment_style_test.go::TestTableCellAlignmentUsesInlineStyle
// The Word output branch has no Rust Docx renderer and is recorded as unported.
#[test]
fn go_table_cell_alignment_uses_inline_style() {
    let e = engine();
    let dom = e.md2blockdom(
        "| Header | Other |\n| :---: | ---: |\n| value | next |",
        false,
    );
    let mut c = Checks::default();
    for (case, output) in [
        ("legacy Markdown", dom.clone()),
        ("HTML", e.blockdom2richhtml(&dom)),
    ] {
        c.check(
            !output.contains(" align=\"")
                && output.contains("text-align: center")
                && output.contains("text-align: right"),
            case,
            &output,
        );
    }
    c.finish("test/table_alignment_style_test.go::TestTableCellAlignmentUsesInlineStyle");
}
