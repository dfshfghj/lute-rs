//! Original Go API and structural regressions. Expectations remain those of Go.
use lute_core::{
    ast::{self, NodeType, WalkStatus},
    html,
    lute::Lute,
    parse,
    render::format::FormatRenderer,
};
fn protyle() -> Lute {
    let mut e = Lute::new();
    e.parse_options.protyle_wysiwyg = true;
    e.render_options.protyle_wysiwyg = true;
    e
}
fn ial(e: &mut Lute) {
    e.parse_options.kramdown_block_ial = true;
    e.parse_options.kramdown_span_ial = true;
    e.render_options.kramdown_block_ial = true;
    e.render_options.kramdown_span_ial = true;
}

// Go: test/api_gap_test.go::TestTextBundleAPI
#[test]
fn test_text_bundle_a_p_i() {
    let e = Lute::new();
    let (out, links) = e.text_bundle(
        "n",
        "![a](https://cdn.example.com/b.png)\n",
        vec!["https://cdn.example.com".into()],
    );
    assert_eq!("![a](assets/b.png)\n", out);
    assert_eq!(Some(vec!["https://cdn.example.com/b.png".into()]), links);
    let (out, links) = e.text_bundle("n", "![a](b.png)\n", vec![]);
    assert_eq!("![a](b.png)\n", out);
    assert_eq!(None, links);
}

// Go: test/api_gap_test.go::TestHTML2TextAPI
#[test]
fn test_h_t_m_l2_text_a_p_i() {
    let e = Lute::new();
    for (input, want) in [
        ("<p>x <b>y</b></p>", "x y"),
        (
            "<h1>标题</h1><p>段落一</p><p>段落二</p>",
            "标题段落一段落二",
        ),
        ("<ul><li>a</li><li>b</li></ul>", "ab"),
        ("", ""),
    ] {
        assert_eq!(want, e.html2text(input), "{input}");
    }
}

// Go: test/api_gap_test.go::TestSpaceAPI
#[test]
fn test_space_a_p_i() {
    for (input, want) in [
        ("中文abc中文123", "中文 abc 中文 123"),
        ("hello world", "hello world"),
        ("中文，abc。", "中文，abc。"),
    ] {
        assert_eq!(want, Lute::space(input));
    }
}

// Go: test/api_gap_test.go::TestTermsAPI
#[test]
fn test_terms_a_p_i() {
    let mut e = Lute::new();
    assert_eq!(Some(&"GitHub".into()), e.get_terms().get("github"));
    e.put_terms([("github".into(), "GitHub Inc.".into())].into());
    assert_eq!(Some(&"GitHub Inc.".into()), e.get_terms().get("github"));
    e.set_terms([("only".into(), "one".into())].into());
    assert_eq!(1, e.get_terms().len());
    assert_eq!(Some(&"one".into()), e.get_terms().get("only"));
}

// Go: test/api_gap_test.go::TestLinkBaseAPI
#[test]
fn test_link_base_a_p_i() {
    let mut e = Lute::new();
    assert_eq!("", e.get_link_base());
    e.render_options.link_base = "https://example.com/base".into();
    assert_eq!("https://example.com/base", e.get_link_base());
    assert!(e
        .markdown("", "![a](b.png)\n")
        .contains("https://example.com/base/b.png"));
}

// Go: test/api_gap_test.go::TestVditorRoundTripAPI
#[test]
fn test_vditor_round_trip_a_p_i() {
    let mut e = Lute::new();
    e.parse_options.vditor_wysiwyg = true;
    e.render_options.vditor_wysiwyg = true;
    let dom = e.md2vditor_dom("# 标题\n\n正文 **粗体**\n");
    assert!(dom.contains("<h1"));
    assert_eq!("# 标题\n\n正文 **粗体**\n", e.vditor_dom2md(&dom));
    assert!(e.vditor_dom2html(&dom).contains("<h1>标题</h1>"));
    let mut ir = Lute::new();
    ir.parse_options.vditor_ir = true;
    ir.render_options.vditor_ir = true;
    let dom = ir.md2vditor_ir_dom("# 标题\n\n正文\n");
    assert!(dom.contains("vditor-ir__node"));
    assert_eq!("# 标题\n\n正文\n", ir.vditor_ir_dom2md(&dom));
    assert!(ir.vditor_ir_dom2html(&dom).contains("<h1>标题</h1>"));
    assert_eq!(
        r#"<p data-block="0">x</p>"#,
        ir.html2vditor_ir_dom("<p>x</p>")
    );
    let mut sv = Lute::new();
    sv.parse_options.vditor_sv = true;
    sv.render_options.vditor_sv = true;
    assert!(sv
        .html2vditor_sv_dom("<p>x</p>")
        .contains(r#"data-type="text""#));
}

// Go: test/api_gap_test.go::TestBlockDOMTextAPI
#[test]
fn test_block_d_o_m_text_a_p_i() {
    let e = protyle();
    let dom = e.md2blockdom("# 标题\n\n正文\n", false);
    assert_eq!("标题正文", e.blockdom2text(&dom));
    assert_eq!(4, e.blockdom2textlen(&dom));
    assert_eq!("", e.blockdom2text(""));
}

// Go: test/api_gap_test.go::TestTL2ULRemovesTaskMarker
#[test]
fn test_t_l2_u_l_removes_task_marker() {
    let e = Lute::new();
    let dom = e.tl2ul(&e.md2blockdom("- [ ] a\n- [x] b\n", false));
    assert!(!dom.contains("protyle-action--task") && !dom.contains("protyle-task--done"));
    assert!(dom.contains(r#"data-subtype="u""#));
}

// Go: test/api_gap_test.go::TestSpinMergesAdjacentSameSpan
#[test]
// Original Go fixture includes a zero-width space.
#[allow(clippy::invisible_characters)]
fn test_spin_merges_adjacent_same_span() {
    let e = protyle();
    let dom=e.spin_blockdom(r#"<div data-node-id="20210101120000-abcdefg" data-node-index="1" data-type="NodeParagraph" class="p"><div contenteditable="true" spellcheck="false"><span data-type="strong">a</span><span data-type="strong">b</span></div><div class="protyle-attr" contenteditable="false">​</div></div>"#);
    assert_eq!(1, dom.matches(r#"data-type="strong""#).count());
    assert!(dom.contains(r#"<span data-type="strong">ab</span>"#));
}

// Go: test/api_gap_test.go::TestBlockDOM2TextLenIgnoresMarkup
#[test]
fn test_block_d_o_m2_text_len_ignores_markup() {
    let e = protyle();
    assert_eq!(
        5,
        e.blockdom2textlen(&e.md2blockdom("**ab** *cd*\n", false))
    );
}

// Go: test/emoji_test.go::TestCleanEmoji
#[test]
fn test_clean_emoji() {
    let e = Lute::new();
    for (input, want) in [
        ("❤️ foo", "foo"),
        ("bar ❤️ foo", "bar  foo"),
        ("❤️ foo ❤️bar", "foo bar"),
    ] {
        assert_eq!(want, e.remove_emoji(input));
    }
}

// Go: test/emoji_unicode17_test.go::TestUnicode17Emoji
#[test]
fn test_unicode17_emoji() {
    let entries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../conformance/fixtures/emoji_unicode17.json"
    ))
    .unwrap();
    assert_eq!(163, entries.len());
    let mut e = Lute::new();
    e.parse_options.callout = true;
    for entry in entries {
        let alias = entry["alias"].as_str().unwrap();
        let emoji = entry["emoji"].as_str().unwrap();
        assert_eq!(Some(&emoji.to_string()), e.get_emojis().get(alias));
        assert_eq!(
            Some(&alias.to_string()),
            e.parse_options.emoji_alias.get(emoji),
            "reverse mapping: {alias}"
        );
        for input in [emoji.to_string(), format!(":{alias}:")] {
            assert_eq!(
                format!("<p>{emoji}</p>\n"),
                e.markdown("", &input),
                "{alias}"
            );
            let got = e.markdown("", &format!("> [!NOTE] {input} Title\n> Content\n"));
            assert!(got.contains(&format!(r#"<span class="callout-icon">{emoji}</span><span class="callout-title">Title</span>"#)),"{alias}: {got}");
        }
    }
}

// Go: test/headingid_test.go::TestHeadingIDRaw
#[test]
fn test_heading_i_d_raw() {
    let mut e = Lute::new();
    e.parse_options.heading_id = true;
    for (input, want) in [
        ("# Heading", ""),
        ("# Heading {#custom-id}", "#custom-id"),
        ("# Heading {#raw_custom-ID}", "#raw_custom-ID"),
        ("# Heading {# 自定义 ID}", "# 自定义 ID"),
    ] {
        let tree = parse::parse("", input.as_bytes(), e.parse_options.clone());
        assert_eq!(
            want,
            Lute::heading_id_raw(&tree.root().first_child().unwrap())
        );
    }
}

// Go: test/textbundle_test.go::TestTextBundle
#[test]
fn test_text_bundle() {
    let e = Lute::new();
    let (out,links)=e.text_bundle("0","[foo](https://img.hacpai.com/dir1/bar.zip)\n\n![foo](https://b3logfile.com/dir2/baz.png)\n",vec!["https://img.hacpai.com".into(),"https://b3logfile.com".into()]);
    assert_eq!(
        "[foo](assets/dir1/bar.zip)\n\n![foo](assets/dir2/baz.png)\n",
        out
    );
    assert_eq!(
        Some(vec![
            "https://img.hacpai.com/dir1/bar.zip".into(),
            "https://b3logfile.com/dir2/baz.png".into()
        ]),
        links
    );
}

// Go: test/format_test.go::TestFormatCases
#[test]
fn test_format_cases() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../conformance/fixtures/format");
    let mut count = 0;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("format-case") || name.contains("formatted") {
            continue;
        }
        let stem = name.strip_suffix(".md").unwrap();
        let input = std::fs::read_to_string(entry.path()).unwrap();
        let expected = std::fs::read_to_string(dir.join(format!("{stem}-formatted.md"))).unwrap();
        let mut e = Lute::new();
        e.render_options.auto_space = true;
        let actual = e.format(&name, &input);
        if expected != actual {
            failures.push(format!("{name}: expected={expected:?}, actual={actual:?}"));
        }
        count += 1;
    }
    assert!(count > 0);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Go: test/format_test.go::TestFormatTaskListHTMLBlock
#[test]
fn test_format_task_list_h_t_m_l_block() {
    let mut e = protyle();
    ial(&mut e);
    e.parse_options.ensure_list_item_paragraph = true;
    let mut tree = parse::parse(
        "",
        b"> - [ ] <div>\n>   Hello\n>   </div>\n",
        e.parse_options.clone(),
    );
    for i in 0..2 {
        if i == 1 {
            e.render_options.auto_space = true;
        }
        let formatted =
            FormatRenderer::new(&tree, e.render_options.clone(), e.parse_options.clone()).render();
        tree = parse::parse("", &formatted, e.parse_options.clone());
        let quote = tree.root().child_by_type(NodeType::NodeBlockquote).unwrap();
        let list = quote.child_by_type(NodeType::NodeList).unwrap();
        let item = list.child_by_type(NodeType::NodeListItem).unwrap();
        let marker = item.first_child().unwrap();
        assert_eq!(NodeType::NodeTaskListItemMarker, marker.typ());
        assert_eq!(NodeType::NodeHTMLBlock, marker.next().unwrap().typ());
    }
}

// Go: test/parse_block_test.go::TestBlock
#[test]
fn test_block() {
    let mut e = Lute::new();
    e.parse_options.vditor_ir = true;
    e.parse_options.mark = true;
    e.parse_options.block_ref = true;
    ial(&mut e);
    e.parse_options.super_block = true;
    e.render_options.auto_space = false;
    e.parse_options.sub = true;
    e.parse_options.sup = true;
    e.parse_options.git_conflict = true;
    e.parse_options.file_annotation_ref = true;
    let source = include_bytes!("../../../../conformance/fixtures/commonmark-spec-kramdown.md");
    let _blocks = parse::parse_block("", source, e.parse_options.clone());
    let _tree = parse::parse("", source, e.parse_options);
}

// Go: test/link_dest_test.go::TestGetLinkDest
#[test]
fn test_get_link_dest() {
    let e = Lute::new();
    assert_eq!(
        r###"http://www.bing.com/search?q=%22%E4%BD%A0%E5%A5%BD%22"###,
        e.get_link_dest(r###"www.bing.com/search?q="你好""###)
    );
    assert_eq!(
        r###"http://www.bing.com/search?q=@Hello"###,
        e.get_link_dest(r###"www.bing.com/search?q=@Hello"###)
    );
    assert_eq!(
        r###"http://www.bing.com/search?q=%22Hello%22"###,
        e.get_link_dest(r###"www.bing.com/search?q="Hello""###)
    );
    assert_eq!(
        r###"https://foo.com/bar:baz"###,
        e.get_link_dest(r###"https://foo.com/bar:baz"###)
    );
    assert_eq!(r###"bar.com"###, e.get_link_dest(r###"[foo](bar.com)"###));
    assert_eq!(
        r###"siyuan://blocks/20220817180757-c57m8qi"###,
        e.get_link_dest(r###"[foo](siyuan://blocks/20220817180757-c57m8qi)"###)
    );
    assert_eq!(
        r###"https://abc.to/"###,
        e.get_link_dest(r###"[foo](https://abc.to/)"###)
    );
    assert_eq!(
        r###"https://abc.pm/"###,
        e.get_link_dest(r###"[foo](https://abc.pm/)"###)
    );
    assert_eq!(
        r###"file://D:\Admin\Downloads\隔壁叔叔过年还在玩思源.jpg"###,
        e.get_link_dest(r###"file://D:\Admin\Downloads\隔壁叔叔过年还在玩思源.jpg"###)
    );
}

// Go: test/link_dest_test.go::TestIsValidLinkDest
#[test]
fn test_is_valid_link_dest() {
    let e = Lute::new();
    assert!(!e.is_valid_link_dest(r###"[foo](bar.com)"###));
    assert!(e.is_valid_link_dest(r###"siyuan://blocks/20220817180757-c57m8qi"###));
    assert!(e.is_valid_link_dest(r###"https://abc.to/"###));
    assert!(e.is_valid_link_dest(r###"https://abc.pm/"###));
    assert!(e.is_valid_link_dest(r###"https://abc.dev"###));
    assert!(
        e.is_valid_link_dest(r###"https://www.notion.so/AUR-9b010a17ca2d4996a898a801426b0585"###)
    );
    assert!(e.is_valid_link_dest(r###"http://127.0.0.1:6806"###));
    assert!(e.is_valid_link_dest(r###"https://ld246.com"###));
    assert!(!e.is_valid_link_dest(r###"https://ld246"###));
    assert!(!e.is_valid_link_dest(r###"ld246.com"###));
    assert!(e.is_valid_link_dest(r###"https://www.electronjs.org/docs/api/shell"###));
}

// Go: test/link_dest_encode_test.go::TestLinkDestEncode
#[test]
fn test_link_dest_encode() {
    let input = "http://foo.bar/测试".as_bytes();
    assert_eq!(
        input,
        html::decode_destination(&html::encode_destination(input))
    );
}

// Go: test/link_dest_encode_test.go::TestLinkDestDecode
#[test]
fn test_link_dest_decode() {
    assert_eq!(r"C:\Users\DL882\Documents\SiYuan\data\assets\测试-20210621160821-zxz1j35.pdf".as_bytes(),html::decode_destination(r"C:\Users\DL882\Documents\SiYuan\data\assets\%E6%B5%8B%E8%AF%95-20210621160821-zxz1j35.pdf".as_bytes()));
}

// Go: test/h2m_code_banner_test.go::TestHTMLCodeBlockBanner
#[test]
fn test_h_t_m_l_code_block_banner() {
    let e = Lute::new();
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre>echo hello</pre></div>"###;
        let want = r###"```bash
echo hello
```
"###;
        assert_eq!(want, e.html2md(input), r###"plain"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"plain"###
        );
    }
    {
        let input = r###"<div class="extra md-code-block"><div class="md-code-block-banner extra"><svg><title>Icon</title></svg><span> bash </span><button>复制代码</button><div role="button"><span>Copy</span></div></div><pre><span class="token">echo</span> &lt;a&gt; &amp; b
  next

last</pre></div>"###;
        let want = r###"```bash
echo <a> & b
  next

last
```
"###;
        assert_eq!(want, e.html2md(input), r###"highlight and controls"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"highlight and controls"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre><code class="language-python">pass</code></pre></div>"###;
        let want = r###"```python
pass
```
"###;
        assert_eq!(want, e.html2md(input), r###"existing code language"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"existing code language"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre class="language-python">pass</pre></div>"###;
        let want = r###"```python
pass
```
"###;
        assert_eq!(want, e.html2md(input), r###"existing pre language"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"existing pre language"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre class="python">pass</pre></div>"###;
        let want = r###"```python
pass
```
"###;
        assert_eq!(want, e.html2md(input), r###"existing bare language"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"existing bare language"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span><div class="ds-icon-button"><span>复制</span></div></div><pre>echo x</pre></div>"###;
        let want = r###"```bash
echo x
```
"###;
        assert_eq!(want, e.html2md(input), r###"custom button"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"custom button"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre data-language="python">pass</pre></div>"###;
        let want = r###"```python
pass
```
"###;
        assert_eq!(want, e.html2md(input), r###"existing data language"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"existing data language"###
        );
    }
    {
        let input = r###"<div class="md-code-block"><div class="md-code-block-banner"><span>C++</span></div><pre><code>int x;</code></pre></div>"###;
        let want = r###"```C++
int x;
```
"###;
        assert_eq!(want, e.html2md(input), r###"code without language"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"code without language"###
        );
    }
    {
        let input = r###"<p>before</p><div class="md-code-block"><div class="md-code-block-banner"><span>bash</span></div><pre>echo x</pre></div><div class="md-code-block"><div class="md-code-block-banner"><span>go</span></div><pre>package main</pre></div><p>after</p>"###;
        let want = r###"before

```bash
echo x
```


```go
package main
```


after
"###;
        assert_eq!(want, e.html2md(input), r###"multiple blocks"###);
        assert_eq!(
            want.replace("```\n\n\n", "```\n\n"),
            e.blockdom2stdmd(&e.html2blockdom(input)),
            r###"multiple blocks"###
        );
    }
}

// Go: test/h2m_code_banner_test.go::TestHTMLCodeBlockBannerPreservesUnrecognizedContent
#[test]
fn test_h_t_m_l_code_block_banner_preserves_unrecognized_content() {
    let e = Lute::new();
    {
        let body = r###"<div class="md-code-block-banner"><span>bash script</span></div><pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<div class="md-code-block-banner"><span>bash</span><span>python</span></div><pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body =
            r###"<div class="md-code-block-banner"><span>Copy</span></div><pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<div class="md-code-block-banner"><span>unknown_language</span></div><pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<div class="md-code-block-banner"><span>bash</span></div><p>echo x</p>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<div class="md-code-block-banner"><span>bash</span></div><pre>x</pre><pre>y</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<div class="md-code-block-banner"><span>bash</span></div><div class="md-code-block-banner">python</div><pre>x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<p>bash</p><pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    {
        let body = r###"<pre>echo x</pre>"###;
        assert_eq!(
            e.html2md(&format!("<div>{body}</div>")),
            e.html2md(&format!(r#"<div class="md-code-block">{body}</div>"#))
        );
    }
    let html = r#"<div class="md-code-block-other"><div class="md-code-block-banner"><span>bash</span></div><pre>echo x</pre></div>"#;
    assert!(e.html2md(html).starts_with("bash\n\n```\n"));
}

// Go: test/h2m_test.go::TestHTML2MdLanguageMath
#[test]
fn test_h_t_m_l2_md_language_math() {
    let e = Lute::new();
    assert_eq!(
        r###"$a^2 + b^2 = c^2$
"###,
        e.html2md(&e.markdown("", r###"$a^2 + b^2 = c^2$"###)),
        r###"7"###
    );
    assert_eq!(
        r###"# $x$
"###,
        e.html2md(&e.markdown("", r###"# $x$"###)),
        r###"6"###
    );
    assert_eq!(
        r###"*$x$*
"###,
        e.html2md(&e.markdown("", r###"*$x$*"###)),
        r###"5"###
    );
    assert_eq!(
        r###"[$x$](url)
"###,
        e.html2md(&e.markdown("", r###"[$x$](url)"###)),
        r###"4"###
    );
    assert_eq!(
        r###"* $x$
"###,
        e.html2md(&e.markdown("", r###"- $x$"###)),
        r###"3"###
    );
    assert_eq!(
        r###"| $x$ |
| ----- |
"###,
        e.html2md(&e.markdown(
            "",
            r###"| $x$ |
| --- |"###
        )),
        r###"2"###
    );
    assert_eq!(
        r###"$$
x
$$
"###,
        e.html2md(&e.markdown("", r###"$$x$$"###)),
        r###"1"###
    );
    assert_eq!(
        r###"$x$
"###,
        e.html2md(r###"<p><span class="foo language-math bar">x</span></p>"###),
        r###"4"###
    );
    assert_eq!(
        r###"x
"###,
        e.html2md(r###"<p><span class="not-language-math">x</span></p>"###),
        r###"3"###
    );
    assert_eq!(r###"$$
x
$$
"###,e.html2md(r###"<p><span class="language-math" data-math="x"><span class="katex-display"><span class="katex">x</span></span></span></p>"###),r###"2"###);
    assert_eq!(
        r###"```math
x
```
"###,
        e.html2md(r###"<pre><code class="language-math">x</code></pre>"###),
        r###"1"###
    );
}

// Go: test/api_gap_test.go::TestSetterSmoke
#[test]
fn test_setter_smoke() {
    for i in 0..10 {
        let mut e = Lute::new();
        let (input, want, block) = match i {
            0 => {
                e.render_options.chinese_paragraph_beginning_space = true;
                ("a\n", r#"class="indent--2""#, false)
            }
            1 => {
                e.render_options.heading_anchor = true;
                ("# a\n", "vditorAnchor-a", false)
            }
            2 => {
                e.render_options.render_list_style = true;
                ("- a\n", r#"data-style="-""#, false)
            }
            3 => {
                e.render_options.img_tag = true;
                ("![a](b.png)\n", r#"<img src="b.png" alt="a" />"#, false)
            }
            4 => {
                e = protyle();
                e.render_options.spellcheck = true;
                ("a\n", r#"spellcheck="true""#, true)
            }
            5 => {
                e = protyle();
                e.parse_options.html_tag2_text_mark = true;
                ("<u>x</u>\n", r#"data-type="u""#, true)
            }
            6 => {
                e = protyle();
                e.parse_options.text_mark = true;
                e.parse_options.tag = true;
                ("#a#\n", r#"data-type="tag""#, true)
            }
            7 => {
                e = protyle();
                e.parse_options.ensure_list_item_paragraph = true;
                ("- \n  - a\n", r#"data-subtype="u""#, true)
            }
            8 => {
                ial(&mut e);
                e.render_options.kramdown_ial_id_render_name = "custom-id".into();
                ("# a\n{: id=\"x\"}\n", "custom-id", false)
            }
            _ => {
                e.render_options.gfm_task_list_item_class = "my-task".into();
                ("- [ ] a\n", "my-task", false)
            }
        };
        let got = if block {
            e.md2blockdom(input, false)
        } else {
            e.markdown("", input)
        };
        assert!(got.contains(want), "setter {i}: {got:?}");
    }
}

// Go: test/file_annotation_ref_test.go::TestFileAnnotationRefPaths
#[test]
fn test_file_annotation_ref_paths() {
    let mut e = protyle();
    e.parse_options.file_annotation_ref = true;
    e.parse_options.text_mark = true;
    for reference in [
        "assets/a.pdf/20210911230820-lhiaysx",
        "assets/document.pdf/20210911230820-lhiaysx",
        "assets/文档 空格.PDF/20210911230820-lhiaysx",
        "assets/folder/a%20b.pdf/20210911230820-lhiaysx",
        "assets/document-20210911230735-pzlpdtf.pdf/20210911230820-lhiaysx",
        "assets/document.pdf/20210911230820-lhiaysx?box=20210911230735-pzlpdtf&dataPath=/docs/a.sy",
        "assets/document.pdf/20210911230820-lhiaysx?dataPath=%2Fdocs%2Fa%20b.sy#view",
        "assets/a&copy;.pdf/20210911230820-lhiaysx?dataPath=/a.sy&copy;=yes",
    ] {
        for anchor in [
            "",
            r#" """#,
            r#" "a""#,
            r#" "<anchor>""#,
            r#" "a \"quote\"""#,
        ] {
            let md = format!("<<{reference}{anchor}>>");
            let tree = parse::parse("", md.as_bytes(), e.parse_options.clone());
            let node = tree.root().first_child().unwrap().first_child().unwrap();
            assert_eq!(NodeType::NodeFileAnnotationRef, node.typ(), "{md}");
            assert_eq!(
                reference.as_bytes(),
                node.child_by_type(NodeType::NodeFileAnnotationRefID)
                    .unwrap()
                    .tokens()
            );
            if anchor == r#" "a""# {
                let dom = e.md2blockdom(&md, false);
                assert!(dom.contains(&format!(
                    r#"data-id="{}""#,
                    html::escape_html_str(reference)
                )));
                assert_eq!(md, e.blockdom2stdmd(&dom).trim());
            }
        }
    }
}

// Go: test/file_annotation_ref_test.go::TestFileAnnotationRefInvalid
#[test]
fn test_file_annotation_ref_invalid() {
    let mut e = Lute::new();
    e.parse_options.file_annotation_ref = true;
    for reference in [
        "a.pdf/20210911230820-lhiaysx",
        "https://example.com/a.pdf/20210911230820-lhiaysx",
        "assets/a.txt/20210911230820-lhiaysx",
        "assets/a.pdf/20210911230820-lhiays",
        "assets/a.pdf/20210911230820-lhiays_",
        "assets/a.pdf/20210911230820-LHIAYSX",
        "assets/a.pdf/20210911230820-lhiaysx/extra",
        "assets/../a.pdf/20210911230820-lhiaysx",
        "assets//a.pdf/20210911230820-lhiaysx",
        "assets/<a>.pdf/20210911230820-lhiaysx",
    ] {
        let md = format!(r#"<<{reference} "anchor">>"#);
        let tree = parse::parse("", md.as_bytes(), e.parse_options.clone());
        ast::walk(&tree.root(), &mut |n, entering| {
            assert!(
                !entering || n.typ() != NodeType::NodeFileAnnotationRef,
                "{md}"
            );
            WalkStatus::WalkContinue
        });
    }
    for md in [
        "<<",
        r#"<<assets/a.pdf/20210911230820-lhiaysx "unterminated"#,
        r#"<<assets/a.pdf/20210911230820-lhiaysx "a">"#,
    ] {
        assert!(e.markdown("", md).contains("&lt;&lt;"));
    }
}

// Go: test/span_ial_entity_test.go::TestSpanIALEntityBoundaries
#[test]
fn test_span_i_a_l_entity_boundaries() {
    let mut e = protyle();
    ial(&mut e);
    e.render_options.auto_space = false;
    {
        let input = r###"<span data-type="text">x</span>{: style="font-family: &quot;Segoe UI&quot;;"}tail &amp; end"###;
        let want = r###"font-family: "Segoe UI";"###;
        let dom = e.md2blockdom(input, false);
        assert!(
            !dom.contains("{:") && dom.contains("tail &amp; end"),
            "{dom}"
        );
        let tree = parse::parse("", input.as_bytes(), e.parse_options.clone());
        let mut found = false;
        ast::walk(&tree.root(), &mut |n, entering| {
            if entering
                && n.typ() == NodeType::NodeTextMark
                && (n.ial_attr("style") == want || n.ial_attr("title") == want)
            {
                found = true;
            }
            WalkStatus::WalkContinue
        });
        assert!(found, "{dom}");
    }
    {
        let input = r###"<span data-type="text">x</span>{: style="font-family: &#34;Segoe UI&#34;;"}tail &amp; end"###;
        let want = r###"font-family: "Segoe UI";"###;
        let dom = e.md2blockdom(input, false);
        assert!(
            !dom.contains("{:") && dom.contains("tail &amp; end"),
            "{dom}"
        );
        let tree = parse::parse("", input.as_bytes(), e.parse_options.clone());
        let mut found = false;
        ast::walk(&tree.root(), &mut |n, entering| {
            if entering
                && n.typ() == NodeType::NodeTextMark
                && (n.ial_attr("style") == want || n.ial_attr("title") == want)
            {
                found = true;
            }
            WalkStatus::WalkContinue
        });
        assert!(found, "{dom}");
    }
    {
        let input = r###"<span data-type="text" style="font-family: &quot;Segoe UI&quot;;">x</span>{: style="font-family: &quot;Segoe UI&quot;;"}tail &amp; end"###;
        let want = r###"font-family: "Segoe UI";"###;
        let dom = e.md2blockdom(input, false);
        assert!(
            !dom.contains("{:") && dom.contains("tail &amp; end"),
            "{dom}"
        );
        let tree = parse::parse("", input.as_bytes(), e.parse_options.clone());
        let mut found = false;
        ast::walk(&tree.root(), &mut |n, entering| {
            if entering
                && n.typ() == NodeType::NodeTextMark
                && (n.ial_attr("style") == want || n.ial_attr("title") == want)
            {
                found = true;
            }
            WalkStatus::WalkContinue
        });
        assert!(found, "{dom}");
    }
    {
        let input = r###"<span data-type="text">x</span>{: title="A &amp; B &#123;C&#125;"}tail &amp; end"###;
        let want = r###"A & B {C}"###;
        let dom = e.md2blockdom(input, false);
        assert!(
            !dom.contains("{:") && dom.contains("tail &amp; end"),
            "{dom}"
        );
        let tree = parse::parse("", input.as_bytes(), e.parse_options.clone());
        let mut found = false;
        ast::walk(&tree.root(), &mut |n, entering| {
            if entering
                && n.typ() == NodeType::NodeTextMark
                && (n.ial_attr("style") == want || n.ial_attr("title") == want)
            {
                found = true;
            }
            WalkStatus::WalkContinue
        });
        assert!(found, "{dom}");
    }
    {
        let dom = e.md2blockdom(
            r###"<span data-type="text">x</span>{: style="font-family: &quot;Segoe UI&quot;;""###,
            false,
        );
        assert!(
            dom.contains("{:") && !dom.contains(r#"style="font-family:"#),
            "{dom}"
        );
    }
    {
        let dom = e.md2blockdom(
            r###"<span data-type="text">x</span>{: style="font-family: &quot;Segoe UI&quot;;"

tail}"###,
            false,
        );
        assert!(
            dom.contains("{:") && !dom.contains(r#"style="font-family:"#),
            "{dom}"
        );
    }
    {
        let dom = e.md2blockdom(
            r###"<span data-type="text">x</span>{: title="&quot;a" **bold** }"###,
            false,
        );
        assert!(
            dom.contains("{:") && !dom.contains(r#"style="font-family:"#),
            "{dom}"
        );
    }
}

// Go: test/html_block_test.go::TestHTMLBlockDataContentRoundTrip
#[test]
// Original Go fixture includes a zero-width space.
#[allow(clippy::invisible_characters)]
fn test_h_t_m_l_block_data_content_round_trip() {
    let e = Lute::new();
    let content = r###"<div title="&quot;">&lt;test&gt; &amp; &#x3C;</div>"###;
    let mut dom=r###"<div data-node-id="20260724120000-abcdefg" data-type="NodeHTMLBlock" class="render-node" data-subtype="block"><div class="protyle-icons"></div><div><protyle-html data-content="&lt;div title=&#34;&amp;quot;&#34;&gt;&amp;lt;test&amp;gt; &amp;amp; &amp;#x3C;&lt;/div&gt;"></protyle-html><span style="position: absolute">​</span></div><div class="protyle-attr" contenteditable="false">​</div></div>"###.to_string();
    for _ in 0..3 {
        let tree = e.blockdom2tree(&dom);
        let node = tree.root().first_child().unwrap();
        assert_eq!(content.as_bytes(), node.tokens());
        dom = e.render_node_blockdom(&node);
    }
}

// Go: test/html_block_test.go::TestHTMLBlockOpenTagConsumesWholeLine
#[test]
fn test_h_t_m_l_block_open_tag_consumes_whole_line() {
    let mut e = Lute::new();
    e.render_options.soft_break2_hard_break = false;
    assert_eq!(
        r###"<p><a href="aaa" target="_blank"><strong>x</strong></a>
<strong>y</strong></p>
"###,
        e.markdown(
            r###"inline content after multiple attributes"###,
            r###"<a href="aaa" target="_blank">**x**</a>
**y**"###
        )
    );
    assert_eq!(
        r###"<p><a href='aaa' target='_blank'><strong>x</strong></a></p>
"###,
        e.markdown(
            r###"inline content after single-quoted attributes"###,
            r###"<a href='aaa' target='_blank'>**x**</a>"###
        )
    );
    assert_eq!(
        r###"<a href="aaa" target="_blank">
**x**
</a>
"###,
        e.markdown(
            r###"type 7 block with multiple attributes"###,
            r###"<a href="aaa" target="_blank">
**x**
</a>"###
        )
    );
}

// Go: test/image_title_data_attribute_test.go::TestImageTitleDataAttribute
#[test]
fn test_image_title_data_attribute() {
    for (name, attrs, title) in [
        (
            "legacy",
            r#"title="caption &amp; &lt;literal&gt;""#,
            "caption & <literal>",
        ),
        (
            "data",
            r#"data-title="caption &amp; &lt;literal&gt;""#,
            "caption & <literal>",
        ),
        (
            "preferred",
            r#"title="legacy" data-title="updated""#,
            "updated",
        ),
        ("cleared", r#"title="legacy" data-title="""#, ""),
    ] {
        let e = protyle();
        let dom = format!(
            r#"<div data-node-id="20261010130000-abcdefg" data-type="NodeParagraph"><div contenteditable="true"><span data-type="img" class="img"><span> </span><span><img src="assets/image.png" data-src="assets/image.png" alt="description" {attrs}><span class="protyle-action__title"><span>caption</span></span></span><span> </span></span></div></div>"#
        );
        for input in [
            dom.clone(),
            e.spin_blockdom(&dom),
            e.md2blockdom(&e.blockdom2md(&dom), false),
            e.md2blockdom(&e.blockdom2stdmd(&dom), false),
        ] {
            let tree = e.blockdom2tree(&input);
            let image = tree
                .root()
                .first_child()
                .unwrap()
                .child_by_type(NodeType::NodeImage)
                .unwrap();
            let actual = image
                .child_by_type(NodeType::NodeLinkTitle)
                .map(|n| n.tokens())
                .unwrap_or_default();
            assert_eq!(title.as_bytes(), actual, "{name}");
        }
        if !title.is_empty() {
            assert!(e.blockdom2html(&dom).contains("title=\""));
        }
    }
}

// Go: test/link_base_prefix_test.go::TestVditorLinkBaseDataImage
#[test]
fn test_vditor_link_base_data_image() {
    let mut e = Lute::new();
    e.parse_options.vditor_wysiwyg = true;
    e.render_options.vditor_wysiwyg = true;
    e.render_options.link_base = "http://domain.com/path/".into();
    for dom in [
        e.md2vditor_ir_dom("![foo](data:image/png;base64,AAAA)"),
        e.md2vditor_dom("![foo](data:image/png;base64,AAAA)"),
    ] {
        assert!(dom.contains(r#"src="data:image/png;base64,AAAA""#), "{dom}");
    }
}

// Go: test/link_base_prefix_test.go::TestVditorLinkBaseRoundTrip
#[test]
fn test_vditor_link_base_round_trip() {
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base"###.into();
        e.render_options.link_prefix = r###""###.into();
        let md = r###"![image.png](.\实现设计.assets\image.png)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"windows-relative-path"###
        );
    }
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base"###.into();
        e.render_options.link_prefix = r###""###.into();
        let md = r###"[foo](dir/foo.txt)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"relative-link"###
        );
    }
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base"###.into();
        e.render_options.link_prefix = r###""###.into();
        let md = r###"![foo](../images/foo.png)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"parent-relative-image"###
        );
    }
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base/"###.into();
        e.render_options.link_prefix = r###""###.into();
        let md = r###"![foo](./images/foo.png)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"trailing-slash"###
        );
    }
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base"###.into();
        e.render_options.link_prefix = r###"prefix:"###.into();
        let md = r###"[foo](dir/foo.txt)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"link-prefix"###
        );
    }
    {
        let mut e = Lute::new();
        e.parse_options.vditor_wysiwyg = true;
        e.render_options.vditor_wysiwyg = true;
        e.render_options.link_base = r###"/configured/base"###.into();
        e.render_options.link_prefix = r###""###.into();
        let md = r###"[foo](https://example.com/configured/base/foo.txt)"###;
        let ir = e.md2vditor_ir_dom(md);
        let markdown = e.vditor_ir_dom2md(&ir);
        assert_eq!(
            format!("{md}\n"),
            e.vditor_dom2md(&e.md2vditor_dom(&markdown)),
            r###"external-link-containing-base"###
        );
    }
}

// Go: test/link_base_prefix_test.go::TestProtyleLinkBaseRoundTrip
#[test]
fn test_protyle_link_base_round_trip() {
    let mut e = protyle();
    e.render_options.link_base = "/configured/base".into();
    e.render_options.link_prefix = "prefix:".into();
    let md = "[foo](dir/foo.txt)";
    assert_eq!(
        format!("{md}\n"),
        e.blockdom2stdmd(&e.md2blockdom(md, false))
    );
}

// Go: test/format_test.go::TestFormatNodeSync (FormatRenderer assertion; missing sync API recorded separately).
#[test]
fn test_format_node_sync_renderer() {
    let mut e = Lute::new();
    e.render_options.auto_space = true;
    let tree = parse::parse("", "foo中文bar".as_bytes(), e.parse_options.clone());
    let output = FormatRenderer::new(&tree, e.render_options, e.parse_options).render();
    assert_eq!("foo 中文 bar\n".as_bytes(), output);
}

// Go: lute.go::Markdown 把 Md2HTMLRendererFuncs 逐个装进 HTML 渲染器。
// Vditor 之外的调用方（含 lute.html / demo.js）都靠 Md2HTML 分组改写输出。
#[test]
fn test_markdown_wires_md2html_js_renderers() {
    let mut e = Lute::new();
    let funcs: lute_core::render::ExtRendererFunc = std::rc::Rc::new(|node, entering| {
        if entering && NodeType::NodeText == node.typ() {
            Some((node.tokens_str() + " via Lute", WalkStatus::WalkContinue))
        } else {
            None
        }
    });
    e.set_js_renderers(std::collections::HashMap::from([(
        "Md2HTML".to_string(),
        funcs,
    )]));
    assert_eq!(
        "<p><strong>x via Lute</strong></p>\n",
        e.markdown("", "**x**")
    );
}
