//! Regression expectations obtained from the Go engine (protyle.go and render/tabs.go).
use lute_core::lute::Lute;

#[test]
fn text_mark_entities_follow_conversion_mode() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeParagraph"><div contenteditable="true"><span data-type="code">&lt;b&gt;</span></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "<span data-type=\"code\">&lt;b&gt;</span>\n"
    );
    assert_eq!(lute.blockdom2stdmd(dom), "`<b>`\n");
}

#[test]
fn standalone_tab_keeps_title_body_and_syntax() {
    let mut lute = Lute::new();
    lute.parse_options.tabs = true;
    let dom = r#"<div data-node-id="20260909000000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info"><span class="tab-item-title">Title</span></div><div class="tab-item-content"><div data-type="NodeParagraph" class="p"><div contenteditable="true">Body</div></div></div></div>"#;
    let md = lute.blockdom2md(dom);
    assert!(
        md.starts_with("::: tabs\n@tab Title\n{: id=\"20260909000000-item001\"}\n\nBody\n\n:::\n"),
        "{md}"
    );
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "::: tabs\n\n@tab Title\n\nBody\n\n\n:::\n"
    );
}

#[test]
fn table_width_uses_dom_ast_tokens() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeTable"><div><table><thead><tr><th>Head</th><th>Long header</th></tr></thead><tbody><tr><td>a</td><td>z</td></tr></tbody></table></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "|Head|Long header|\n| ------| -------------|\n|a|z|\n"
    );
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "| Head | Long header |\n| ------ | ------------- |\n| a    | z           |\n"
    );
}

#[test]
fn styled_text_follows_keep_escaped_without_double_boundary_space() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeParagraph"><div contenteditable="true">Before <span data-type="strong"> &lt;x&gt; </span> after <span data-type="code">a|b &amp; "q"</span></div></div>"#;
    assert_eq!(lute.blockdom2md(dom), "Before <span data-type=\"strong\">&lt;x&gt;</span>  after <span data-type=\"code\">a|b &amp; \"q\"</span>\n");
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "Before  **<x>**   after `a|b & \"q\"`\n"
    );
}

#[test]
fn original_tab_title_block_keeps_identity_and_exports_once() {
    let mut lute = Lute::new();
    lute.parse_options.tabs = true;
    let dom = r#"<div data-node-id="20260905120000-tabs001" data-type="NodeTabs" class="tabs" tabs-active-id="20260905120000-item001"><div data-node-id="20260905120000-item001" data-type="NodeTabItem" class="tab-item"><div class="tab-item-info"><div data-node-id="20260905120000-title01" data-type="NodeParagraph" tabs-title="true" custom-test="kept"><div class="tab-item-title" contenteditable="true"><span data-type="strong">Title</span></div></div></div><div class="tab-item-content"><div data-type="NodeParagraph"><div contenteditable="true">Body</div></div></div></div></div>"#;
    assert_eq!(lute.blockdom2md(dom), "::: tabs\n@tab:active\n{: id=\"20260905120000-item001\"}\n\n<span data-type=\"strong\">Title</span>\n{: id=\"20260905120000-title01\" tabs-title=\"true\" custom-test=\"kept\"}\n\nBody\n\n:::\n{: id=\"20260905120000-tabs001\" tabs-active-id=\"20260905120000-item001\"}\n");
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "::: tabs\n\n@tab:active **Title**\n\n\nBody\n\n\n:::\n"
    );
}

#[test]
fn html_conversion_uses_original_keep_escaped_setting() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeParagraph"><div contenteditable="true"><span data-type="strong">&lt;em&gt; &amp; &quot;x&quot;</span></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "<span data-type=\"strong\">&lt;em&gt; &amp; \"x\"</span>\n"
    );
    assert_eq!(lute.blockdom2stdmd(dom), " **<em> & \"x\"**\n");
    assert_eq!(
        lute.blockdom2html(dom),
        "<p><span data-type=\"strong\"><em> &amp; &quot;x&quot;</span></p>\n"
    );
}

#[test]
fn escaped_table_text_counts_its_dom_container() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeTable"><div><table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td><span data-type="backslash">\*</span></td><td>中</td></tr></tbody></table></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "|a|b|\n| --------| ----|\n|\\\\*|中|\n"
    );
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "| a      | b  |\n| -------- | ---- |\n| \\\\* | 中 |\n"
    );
}

#[test]
fn tab_title_is_always_inline_markdown() {
    let mut lute = Lute::new();
    lute.parse_options.tabs = true;
    for title in ["# Title", "- foo", "---"] {
        let dom = format!(
            r#"<div data-type="NodeTabs"><div data-type="NodeTabItem"><div class="tab-item-info"><span class="tab-item-title">{title}</span></div><div class="tab-item-content"><div data-type="NodeParagraph"><div contenteditable="true">Body</div></div></div></div></div>"#
        );
        assert_eq!(
            lute.blockdom2stdmd(&dom),
            format!("::: tabs\n\n@tab {title}\n\nBody\n\n:::\n")
        );
    }
}

#[test]
fn unknown_dom_elements_share_text_postprocessing_in_tables() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeTable"><div><table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td><protyle-html data-content="ignored"></protyle-html></td><td>x</td></tr></tbody></table></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "|a|b|\n| --------------| ---|\n|protyle-html|x|\n"
    );
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "| a            | b |\n| -------------- | --- |\n| protyle-html | x |\n"
    );
}

#[test]
fn image_container_tokens_participate_in_table_width() {
    let lute = Lute::new();
    let dom = r#"<div data-type="NodeTable"><div><table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td><span data-type="img"><span><img data-src="b" alt="a"></span></span></td><td>x</td></tr></tbody></table></div></div>"#;
    assert_eq!(
        lute.blockdom2md(dom),
        "|a|b|\n| --------| ---|\n|![a](b)|x|\n"
    );
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "| a      | b |\n| -------- | --- |\n| ![a](b) | x |\n"
    );
}

#[test]
fn export_uses_its_own_block_boundary_rules() {
    let lute = Lute::new();
    let dom = r#"<aside>inline</aside><div data-type="NodeHeading" data-subtype="h2"><div contenteditable="true">heading</div></div>"#;
    assert_eq!(lute.blockdom2md(dom), "<aside>inline</aside>\n## heading\n");
    assert_eq!(
        lute.blockdom2stdmd(dom),
        "<aside>inline</aside>## heading\n"
    );
}
