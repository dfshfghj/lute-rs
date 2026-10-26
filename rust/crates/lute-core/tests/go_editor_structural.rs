//! Original Go editor/structural regression assertions. Expected failures remain strict.
use lute_core::{
    ast::{NodeType, Tree},
    editor::{WORD_JOINER, ZWSP},
    lute::Lute,
    parse::parse,
};
fn engine() -> Lute {
    let mut l = Lute::new();
    l.parse_options.protyle_wysiwyg = true;
    l.render_options.protyle_wysiwyg = true;
    l.parse_options.kramdown_block_ial = true;
    l.parse_options.kramdown_span_ial = true;
    l.render_options.kramdown_block_ial = true;
    l.render_options.kramdown_span_ial = true;
    l
}
fn placeholder_engine() -> Lute {
    let mut l = engine();
    l.parse_options.tag = true;
    l.parse_options.text_mark = true;
    l.parse_options.html_tag2_text_mark = true;
    l.render_options.auto_space = false;
    l.parse_options.spin = true;
    l
}
fn placeholder_dom(s: &str) -> String {
    format!(
        r#"<div data-node-id="20260830120000-abcdefg" data-node-index="1" data-type="NodeParagraph" class="p" updated="20260830120000"><div contenteditable="true" spellcheck="false">{s}</div><div class="protyle-attr" contenteditable="false">{ZWSP}</div></div>"#
    )
}
fn exports(l: &Lute, dom: &str) -> Vec<(&'static str, String)> {
    vec![
        ("ast", l.blockdom2tree(dom).root().text()),
        ("markdown", l.blockdom2md(dom)),
        ("standard", l.blockdom2stdmd(dom)),
        ("html", l.blockdom2html(dom)),
        ("rich-html", l.blockdom2richhtml(dom)),
    ]
}
// Go: test/protyle_placeholder_test.go::TestProtyleInlinePlaceholderCodepoints
#[test]
fn go_placeholder_codepoints() {
    assert_eq!(ZWSP.chars().collect::<Vec<_>>(), ['\u{200b}']);
    assert_eq!(WORD_JOINER.chars().collect::<Vec<_>>(), ['\u{2060}']);
    assert_ne!(ZWSP, WORD_JOINER);
}
// Go: test/protyle_placeholder_test.go::TestProtyleInlinePlaceholderRendering
#[test]
fn go_placeholder_rendering() {
    let l = placeholder_engine();
    for (md, typ, content) in [
        ("a`code`b", "code", "code"),
        ("a**`code`**b", "strong code", "code"),
        ("a<kbd>key</kbd>b", "kbd", "key"),
        ("a#tag#b", "tag", "tag"),
    ] {
        let dom = l.md2blockdom(md, false);
        assert!(
            dom.contains(&format!(
                "a{ZWSP}<span data-type=\"{typ}\">{WORD_JOINER}{content}"
            )),
            "{dom}"
        );
        assert!(dom.contains(&format!("{content}</span>{ZWSP}b")), "{dom}");
    }
}
// Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderCompatibility
#[test]
fn go_placeholder_compatibility() {
    let mut l = placeholder_engine();
    for marker in [ZWSP, "\u{feff}", WORD_JOINER] {
        for (open, close) in [
            (r#"<span data-type="code">"#, "</span>"),
            (r#"<span data-type="strong code">"#, "</span>"),
            (r#"<span data-type="kbd">"#, "</span>"),
            ("<kbd>", "</kbd>"),
            (r#"<span data-type="tag">"#, "</span>"),
            ("<code>", "</code>"),
        ] {
            let dom = placeholder_dom(&format!(
                "outside{WORD_JOINER}{open}{marker}value{WORD_JOINER}tail{close}"
            ));
            assert_eq!(
                l.blockdom2tree(&dom).root().text(),
                format!("outside{WORD_JOINER}value{WORD_JOINER}tail")
            );
            l.parse_options.text_mark = false;
            let md = l.blockdom2md(&dom);
            assert!(md.contains(&format!("value{WORD_JOINER}tail")), "{md:?}");
            assert_eq!(md.matches(WORD_JOINER).count(), 2, "{md:?}");
            assert!(!md.contains('\u{feff}') && !md.contains(ZWSP));
            l.parse_options.text_mark = true;
        }
    }
}
// Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderNormalizationIsScoped
#[test]
fn go_placeholder_scoped() {
    let l = placeholder_engine();
    let inlines = format!(
        r#"outside{WORD_JOINER}<span data-type="strong">{WORD_JOINER}strong</span><kbd><span>foo</span>{WORD_JOINER}bar</kbd><kbd>{WORD_JOINER}outer<code>{WORD_JOINER}inner</code></kbd><span data-type="code">{WORD_JOINER}{WORD_JOINER}double</span><span data-type="code-block kbdish hashtag">{WORD_JOINER}similar</span><span data-type="code">&NoBreak;entity</span>"#
    );
    assert_eq!(l.blockdom2tree(&placeholder_dom(&inlines)).root().text(),format!("outside{WORD_JOINER}{WORD_JOINER}strongfoo{WORD_JOINER}barouterinner{WORD_JOINER}double{WORD_JOINER}similarentity"));
}
// Go: test/protyle_placeholder_test.go::TestSpinBlockDOMInlinePlaceholderCanonicalAndIdempotent
#[test]
fn go_placeholder_canonical() {
    struct RestoreTesting(bool);
    impl Drop for RestoreTesting {
        fn drop(&mut self) {
            lute_core::ast::TESTING.store(self.0, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let _testing =
        RestoreTesting(lute_core::ast::TESTING.swap(true, std::sync::atomic::Ordering::Relaxed));
    let l = placeholder_engine();
    let input=format!("a<span data-type=\"code\">\u{feff}code</span>bc<span data-type=\"strong code\">{ZWSP}both</span>de<span data-type=\"kbd\">{WORD_JOINER}key</span>fg<span data-type=\"tag\">{ZWSP}tag</span>h");
    let spun = l.spin_blockdom(&placeholder_dom(&input));
    for (a, typ, text, b) in [
        ("a", "code", "code", "b"),
        ("c", "strong code", "both", "d"),
        ("e", "kbd", "key", "f"),
        ("g", "tag", "tag", "h"),
    ] {
        assert!(
            spun.contains(&format!(
                "{a}{ZWSP}<span data-type=\"{typ}\">{WORD_JOINER}{text}</span>{ZWSP}{b}"
            )),
            "{spun}"
        );
    }
    assert!(!spun.contains('\u{feff}'));
    assert_eq!(l.spin_blockdom(&spun), spun);
}
// Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderDoesNotLeakToExports
#[test]
fn go_placeholder_no_export_leak() {
    let l = placeholder_engine();
    for marker in [ZWSP, "\u{feff}", WORD_JOINER] {
        let dom = placeholder_dom(&format!(
            "before <span data-type=\"code\">{marker}code</span> after"
        ));
        for (name, out) in exports(&l, &dom) {
            assert!(!out.contains(marker), "{name}: {out:?}");
        }
    }
}
// Go: test/protyle_placeholder_test.go::TestBlockDOMInlinePlaceholderPreservesRealWordJoiners
#[test]
fn go_placeholder_real_joiners() {
    let l = placeholder_engine();
    for marker in [ZWSP, "\u{feff}", WORD_JOINER] {
        let dom=placeholder_dom(&format!("before{WORD_JOINER}<span data-type=\"code\">{marker}{WORD_JOINER}co{WORD_JOINER}de</span>after"));
        for (name, out) in exports(&l, &dom) {
            assert_eq!(out.matches(WORD_JOINER).count(), 3, "{name}: {out:?}");
        }
    }
}
// Go: test/custom_block_test.go::TestCustomBlockOptionAndFence
#[test]
fn go_custom_block_fences() {
    let mut l = Lute::new();
    let md = ";;;plugin:test\ncontent\n;;;\nafter";
    assert_ne!(
        parse("", md.as_bytes(), l.parse_options.clone())
            .root()
            .first_child()
            .unwrap()
            .typ(),
        NodeType::NodeCustomBlock
    );
    l.parse_options.custom_block = true;
    let tree = parse("", md.as_bytes(), l.parse_options.clone());
    let n = tree.root().first_child().unwrap();
    assert_eq!(n.typ(), NodeType::NodeCustomBlock);
    assert_eq!(n.tokens_str(), "content\n");
    assert_eq!(n.next().unwrap().typ(), NodeType::NodeParagraph);
    for md in [
        ";;plugin:test\ncontent\n;;;",
        ";;;;plugin:test\ncontent\n;;;",
        ";;;plugin;test\ncontent\n;;;",
    ] {
        assert_ne!(
            parse("", md.as_bytes(), l.parse_options.clone())
                .root()
                .first_child()
                .unwrap()
                .typ(),
            NodeType::NodeCustomBlock
        );
    }
    let t = parse(
        "",
        b";;;plugin:test\ncontent\n;;;;\nafter",
        l.parse_options.clone(),
    );
    let n = t.root().first_child().unwrap();
    assert_eq!(n.typ(), NodeType::NodeCustomBlock);
    assert!(n.next().is_none());
    assert!(n.tokens_str().contains(";;;;\nafter"));
}
// Go: test/custom_block_test.go::TestCustomBlockSafeFallbackRenderers
#[test]
fn go_custom_block_safe_fallback() {
    let mut l = Lute::new();
    l.parse_options.custom_block = true;
    let info = "plugin:test\"\tonmouseover=\"alert(1)";
    let content = "<script>alert(\"x\")</script>&amp;\n";
    let md = format!(";;;{info}\n{content};;;");
    let outputs = [
        l.md2blockdom(&md, false),
        l.markdown("", &md),
        l.protyle_preview_str("", &md),
    ];
    for out in &outputs {
        assert!(!out.contains("onmouseover=\""), "{out}");
        assert!(
            out.contains(&format!(
                "data-info=\"{}\"",
                lute_core::html::escape_html_str(info)
            )),
            "{out}"
        );
        assert!(!out.contains("<script>"));
        assert!(
            out.contains(&format!(
                "<pre>{}</pre>",
                lute_core::html::escape_html_str(content)
            )),
            "{out}"
        );
    }
    let protyle = &outputs[0];
    let root = protyle.split('>').next().unwrap();
    assert_eq!(root.matches("data-type=\"NodeCustomBlock\"").count(), 1);
    assert!(root.contains("contenteditable=\"false\""));
    assert!(protyle.contains("class=\"custom-block__content\""));
}
fn assert_custom_md(l: &Lute, md: &str, expected: &str) {
    let t = parse("", md.as_bytes(), l.parse_options.clone());
    let n = t.root().first_child().unwrap();
    assert_eq!(n.typ(), NodeType::NodeCustomBlock);
    assert_eq!(n.tokens_str(), expected);
}
fn assert_custom_dom(l: &Lute, dom: &str, expected: &str) {
    let t = l.blockdom2tree(dom);
    let n = t.root().first_child().unwrap();
    assert_eq!(n.typ(), NodeType::NodeCustomBlock);
    assert_eq!(n.tokens_str(), expected);
}
// Go: test/custom_block_test.go::TestCustomBlockEntityRoundTripAndRenderedSubtree
#[test]
fn go_custom_block_entity_roundtrip() {
    let mut l = Lute::new();
    l.parse_options.custom_block = true;
    let content = "literal &amp; &quot; &#123; & <tag> \"quoted\"\n";
    let md = format!(";;;plugin:entity\n{content};;;");
    let mut dom = l.md2blockdom(&md, false);
    assert!(dom.contains("data-content=\"literal &amp;amp; &amp;quot; &amp;#123; &amp; &lt;tag&gt; &quot;quoted&quot;\n\""),"{dom}");
    assert_custom_md(&l, &l.blockdom2md(&dom), content);
    assert_custom_dom(&l, &l.spin_blockdom(&dom), content);
    let start = dom.find(r#"<div class="custom-block__content">"#).unwrap();
    let attr = dom.find(r#"<div class="protyle-attr""#).unwrap();
    assert!(start < attr);
    dom = format!(
        r#"{}<div class="custom-block__content"><div data-node-id="rendered-child" data-type="NodeParagraph" class="p"><div contenteditable="true">PLUGIN_RENDER_TREE</div></div></div>{}"#,
        &dom[..start],
        &dom[attr..]
    );
    let md = l.blockdom2md(&dom);
    assert!(!md.contains("PLUGIN_RENDER_TREE"));
    assert_custom_md(&l, &md, content);
    let spun = l.spin_blockdom(&dom);
    assert!(!spun.contains("PLUGIN_RENDER_TREE"));
    assert_custom_dom(&l, &spun, content);
}

// Go: test/callout_backtick_test.go::TestCalloutBacktickInput
#[test]
fn go_callout_backtick() {
    let mut l = engine();
    l.parse_options.text_mark = true;
    l.parse_options.callout = true;
    l.parse_options.spin = true;
    for (title, want) in [
        ("`<wbr>", "`<wbr>"),
        ("Note `<wbr>", "Note `<wbr>"),
        ("Note ``<wbr>", "Note ``<wbr>"),
        (
            r#"Note `code`<wbr>"#,
            r#"Note <span data-type="code">code</span><wbr>"#,
        ),
        (
            r#"Note <span data-type="code">code</span>`<wbr>"#,
            r#"Note <span data-type="code">code</span>`<wbr>"#,
        ),
    ] {
        let dom = format!(
            r#"<div data-type="NodeCallout" data-subtype="NOTE" class="callout"><div class="callout-info"><span class="callout-icon">✏️</span><span class="callout-title" contenteditable="true">{title}</span></div><div class="callout-content"><div data-type="NodeParagraph"><div contenteditable="true">body</div></div></div></div>"#
        );
        let got = l
            .spin_blockdom(&dom)
            .replace(ZWSP, "")
            .replace(WORD_JOINER, "");
        assert!(got.contains(want), "{title}: {got}");
    }
}
// Go: test/mindmap_block_test.go::TestMindmapBlockDOMRoundTrip
#[test]
fn go_mindmap_roundtrip() {
    let mut l = engine();
    l.parse_options.tabs = true;
    l.parse_options.block_ref = true;
    l.parse_options.super_block = true;
    l.render_options.super_block = true;
    l.parse_options.text_mark = true;
    let mut source=r#"<div data-node-id="20260923120000-map0001" data-type="NodeMindmap" data-subtype="u" class="mindmap"><div data-node-id="20260923120000-item001" data-type="NodeMindmapItem" data-subtype="u" data-marker="*" class="mindmap-item"><div class="protyle-action"></div><div data-node-id="20260923120000-para001" data-type="NodeParagraph" class="p"><div contenteditable="true">Root</div></div><div data-node-id="20260923120000-map0002" data-type="NodeMindmap" data-subtype="u" class="mindmap"><div data-node-id="20260923120000-item002" data-type="NodeMindmapItem" data-subtype="u" data-marker="*" class="mindmap-item"><div class="protyle-action"></div><div data-node-id="20260923120000-para002" data-type="NodeParagraph" class="p"><div contenteditable="true">Child</div></div></div></div></div></div>"#.to_owned();
    for _ in 0..3 {
        let t = l.blockdom2tree(&source);
        let n = t.root().first_child().unwrap();
        assert_eq!(n.typ(), NodeType::NodeMindmap);
        let item = n.first_child().unwrap();
        assert_eq!(item.typ(), NodeType::NodeMindmapItem);
        assert_eq!(
            item.child_by_type(NodeType::NodeMindmap)
                .unwrap()
                .first_child()
                .unwrap()
                .typ(),
            NodeType::NodeMindmapItem
        );
        let rendered = l.render_node_blockdom(&n);
        for s in [
            r#"data-type="NodeMindmap""#,
            r#"data-type="NodeMindmapItem""#,
            r#"class="mindmap""#,
        ] {
            assert!(rendered.contains(s), "{rendered}");
        }
        assert_eq!(n.id(), "20260923120000-map0001");
        assert_eq!(item.id(), "20260923120000-item001");
        source = rendered;
    }
    let spun = l.spin_blockdom(&source);
    assert_eq!(spun.matches(r#"data-type="NodeMindmap""#).count(), 2);
    assert_eq!(spun.matches(r#"data-type="NodeMindmapItem""#).count(), 2);
    for out in [l.blockdom2stdmd(&source), l.blockdom2html(&source)] {
        assert!(out.contains("Root") && out.contains("Child"), "{out}");
    }
}
// Go: test/mindmap_block_test.go::TestMindmapContainment
#[test]
fn go_mindmap_containment() {
    let mut t = Tree::new();
    let c = t.new_node(NodeType::NodeMindmap);
    let i = t.new_node(NodeType::NodeMindmapItem);
    assert!(c.can_contain(NodeType::NodeMindmapItem));
    assert!(!c.can_contain(NodeType::NodeListItem));
    assert!(i.can_contain(NodeType::NodeMindmap));
    assert!(i.can_contain(NodeType::NodeParagraph));
    for typ in [
        NodeType::NodeList,
        NodeType::NodeListItem,
        NodeType::NodeMindmapItem,
    ] {
        assert!(!i.can_contain(typ));
    }
}
// Go: test/tag_protyle_punctuation_test.go::TestProtyleTagPunctuationBoundary
#[test]
fn go_tag_punctuation() {
    let mut l = engine();
    l.parse_options.text_mark = true;
    l.parse_options.tag = true;
    for input in ["Per the exit condition, #83 shifts state. Escalated via the close (`20260819002715-enfbv43`, Routine Close), and whether this warrants breaking the standing \"#83 has no dedicated card\" convention.","#83 shifts state. See (#113)","#83 shifts state. See [#113]"]{let dom=l.md2blockdom(input,false);assert!(!dom.contains(r#"data-type="tag"#),"{dom}");assert_eq!(l.blockdom2stdmd(&dom).trim(),input);}
}
// Go: test/tag_protyle_punctuation_test.go::TestProtyleTagPunctuationBoundaryKeepsValidSyntax
#[test]
fn go_tag_valid_syntax() {
    let mut l = engine();
    l.parse_options.text_mark = true;
    l.parse_options.tag = true;
    for input in ["#foo#", "(#foo#)", r##""#foo#""##, "#foo bar#"] {
        let dom = l.md2blockdom(input, false);
        assert!(dom.contains(r#"data-type="tag""#), "{dom}");
    }
    assert_eq!(
        l.md2blockdom("#foo##bar#", false)
            .matches(r#"data-type="tag""#)
            .count(),
        2
    );
    assert!(l
        .md2blockdom("**foo.**1", false)
        .contains(r#"data-type="strong""#));
}
// Go: test/data_task_test.go::TestBlockDOM2StdMdTaskMarkerNormalization
#[test]
fn go_task_marker_normalization() {
    let mut l = Lute::new();
    l.parse_options.protyle_wysiwyg = true;
    l.render_options.protyle_wysiwyg = true;
    l.render_options.data_task = true;
    l.parse_options.arbitrary_task_list_item_marker = true;
    l.render_options.unordered_list_marker = "-".into();
    let md = "- [ ] todo\n- [/] progress\n\n  - [-] canceled\n- [X] done\n- [!] custom\n";
    let dom = l.md2blockdom(md, true);
    let normalized = "- [ ] todo\n- [X] progress\n\n  - [X] canceled\n- [X] done\n- [X] custom\n";
    assert_eq!(l.blockdom2stdmd(&dom), normalized);
    l.render_options.export_normalize_task_list_marker = false;
    assert_eq!(l.blockdom2stdmd(&dom), md);
    l.render_options.export_normalize_task_list_marker = true;
    assert_eq!(l.blockdom2stdmd(&dom), normalized);
}

// Go: test/protyle_media_controls_test.go::TestProtyleVideoControls
#[test]
fn go_video_controls() {
    let mut l = Lute::new();
    l.parse_options.protyle_wysiwyg = true;
    l.render_options.protyle_wysiwyg = true;
    l.render_options.sanitize = true;
    for (name, input, expected) in [
        (
            "missing",
            r###"<video src="assets/video.mp4"></video>"###,
            r###"<video src="assets/video.mp4" controls="controls"></video>"###,
        ),
        (
            "existing",
            r###"<video controls="controls" src="assets/video.mp4"></video>"###,
            r###"<video controls="controls" src="assets/video.mp4"></video>"###,
        ),
        (
            "boolean",
            r###"<video controls src="assets/video.mp4"></video>"###,
            r###"<video controls="" src="assets/video.mp4"></video>"###,
        ),
        (
            "attributes",
            r###"<video src="https://example.com/video.mp4?a=1&amp;b=2" poster="assets/poster.png" muted loop></video>"###,
            r###"<video src="https://example.com/video.mp4?a=1&amp;b=2" poster="assets/poster.png" muted="" loop="" controls="controls"></video>"###,
        ),
        (
            "source",
            r###"<video><source src="assets/video.mp4" type="video/mp4"></video>"###,
            r###"<video controls="controls"><source src="assets/video.mp4" type="video/mp4"/></video>"###,
        ),
    ] {
        let block = format!(
            r#"<div data-node-id="20260918120000-abcdefg" data-type="NodeVideo" class="iframe"><div class="iframe-content">{input}</div></div>"#
        );
        for (path, dom) in [
            ("markdown", l.md2blockdom(input, false)),
            ("html", l.html2blockdom(input)),
            ("existing-block", l.spin_blockdom(&block)),
        ] {
            let expected = if name == "source" && path == "html" {
                r#"<video src="assets/video.mp4" controls="controls"></video>"#
            } else {
                expected
            };
            assert!(dom.contains(expected), "{name}/{path}: {dom}");
            let spun = l.spin_blockdom(&dom);
            assert!(spun.contains(expected), "{name}/{path}: {spun}");
            assert_eq!(
                spun.matches(" controls=").count(),
                1,
                "{name}/{path}: {spun}"
            );
        }
    }
}

// Go: test/protyle_media_controls_test.go::TestProtyleAudioControls
#[test]
fn go_audio_controls() {
    let mut l = Lute::new();
    l.parse_options.protyle_wysiwyg = true;
    l.render_options.protyle_wysiwyg = true;
    l.render_options.sanitize = true;
    for (name, input, expected) in [
        (
            "missing",
            r###"<audio src="assets/audio.mp3"></audio>"###,
            r###"<audio src="assets/audio.mp3" controls="controls"></audio>"###,
        ),
        (
            "existing",
            r###"<audio controls="controls" src="assets/audio.mp3"></audio>"###,
            r###"<audio controls="controls" src="assets/audio.mp3"></audio>"###,
        ),
        (
            "boolean",
            r###"<audio controls src="assets/audio.mp3"></audio>"###,
            r###"<audio controls="" src="assets/audio.mp3"></audio>"###,
        ),
        (
            "attributes",
            r###"<audio src="https://example.com/audio.mp3?a=1&amp;b=2" preload="none" muted loop></audio>"###,
            r###"<audio src="https://example.com/audio.mp3?a=1&amp;b=2" preload="none" muted="" loop="" controls="controls"></audio>"###,
        ),
        (
            "source",
            r###"<audio><source src="assets/audio.mp3" type="audio/mpeg"></audio>"###,
            r###"<audio controls="controls"><source src="assets/audio.mp3" type="audio/mpeg"/></audio>"###,
        ),
    ] {
        let block = format!(
            r#"<div data-node-id="20260918120000-abcdefg" data-type="NodeAudio" class="iframe"><div class="iframe-content">{input}</div></div>"#
        );
        for (path, dom) in [
            ("markdown", l.md2blockdom(input, false)),
            ("html", l.html2blockdom(input)),
            ("existing-block", l.spin_blockdom(&block)),
        ] {
            assert!(dom.contains(expected), "{name}/{path}: {dom}");
            let spun = l.spin_blockdom(&dom);
            assert!(spun.contains(expected), "{name}/{path}: {spun}");
            assert_eq!(
                spun.matches(" controls=").count(),
                1,
                "{name}/{path}: {spun}"
            );
        }
    }
}
// Go: test/text_mark_delimiters_test.go::TestSpinCrossTextMarkDelimiters
#[test]
fn go_cross_text_mark_delimiters() {
    let mut l = engine();
    l.parse_options.text_mark = true;
    l.parse_options.spin = true;
    l.parse_options.mark = true;
    let wrap = |s: &str| {
        format!(
            r#"<div data-node-id="20260908150000-abcdefg" data-type="NodeParagraph" class="p"><div contenteditable="true">{s}</div><div class="protyle-attr" contenteditable="false"></div></div>"#
        )
    };
    for (name, input, want) in [
        (
            "close-inside",
            r###"*<span data-type="strong">11*<wbr></span>"###,
            r###"<span data-type="em strong">11</span>"###,
        ),
        (
            "open-inside",
            r###"<span data-type="strong">*11</span>*<wbr>"###,
            r###"<span data-type="em strong">11</span>"###,
        ),
        (
            "different-styles",
            r###"<span data-type="strong">*foo</span><span data-type="mark">bar*</span><wbr>"###,
            r###"<span data-type="em strong">foo</span><span data-type="em mark">bar</span>"###,
        ),
        (
            "mixed-text",
            r###"*foo<span data-type="strong">bar*</span><wbr>"###,
            r###"<span data-type="em">foo</span><span data-type="em strong">bar</span>"###,
        ),
        (
            "attributes",
            r###"*<span data-type="strong text" style="color: red;">11*</span><wbr>"###,
            r###"style="color: red;""###,
        ),
        (
            "unmatched",
            r###"*<span data-type="strong">11<wbr></span>"###,
            r###"*<span data-type="strong">11<wbr></span>"###,
        ),
        (
            "code-boundary",
            r###"*<span data-type="strong code">11*</span><wbr>"###,
            r###"11*</span>"###,
        ),
        (
            "escaped",
            r###"\*<span data-type="strong">11*</span><wbr>"###,
            r###"<span data-type="strong">11*</span>"###,
        ),
    ] {
        let got = l.spin_blockdom(&wrap(input));
        assert!(got.contains(want), "{name}: {got}");
        assert!(got.contains("<wbr>"));
        let again = l.spin_blockdom(&got);
        assert!(again.contains(want), "{name}: {again}");
        assert_eq!(l.spin_blockdom(&again), again);
    }
    l.parse_options.inline_asterisk = false;
    let got = l.spin_blockdom(&wrap(r#"*<span data-type="strong">11*</span><wbr>"#));
    assert!(!got.contains(r#"data-type="em"#), "{got}");
}

// Go: test/text_mark_escape_roundtrip_test.go::TestTextMarkEscapedKramdownRoundTrip
#[test]
fn go_text_mark_escape_roundtrip() {
    for mark in [
        "em", "strong", "s", "mark", "sup", "sub", "u", "kbd", "code", "text", "a em",
    ] {
        let mut l = engine();
        l.parse_options.text_mark = true;
        l.render_options.auto_space = false;
        l.parse_options.spin = true;
        let content = r#"<vitae> & &lt; "quote""#;
        let span = format!(
            r#"<span data-type="{mark}" data-href="https://example.com">{}</span>"#,
            lute_core::html::escape_html_str(content)
        );
        let mut dom = format!(
            r#"<div data-node-id="20260918120000-abcdefg" data-type="NodeParagraph"><div contenteditable="true">{span}</div></div>"#
        );
        let check = |got: &str| {
            let d = lute_core::dom::parse(got);
            let text = d.text(d.root).replace(ZWSP, "").replace(WORD_JOINER, "");
            assert_eq!(text, content, "{mark}: {got}");
        };
        for _ in 0..3 {
            check(&l.spin_blockdom(&dom));
            assert_eq!(l.blockdom2content(&dom).replace(ZWSP, ""), content);
            let md = l.blockdom2md(&dom);
            dom = l.md2blockdom(&md, false);
            check(&dom);
            check(&l.inline_md2blockdom(&span));
        }
    }
}
// Go: test/strikethrough_test.go::TestFullWidthStrikethrough
#[test]
fn go_fullwidth_strikethrough() {
    let mut l = Lute::new();
    l.parse_options.full_width_strikethrough = true;
    for (name, md, want) in [
        ("full-width", "～～foo～～", "<p><del>foo</del></p>\n"),
        ("suffix", "～～foo～～111", "<p><del>foo</del>111</p>\n"),
        ("ascii", "~~foo~~111", "<p><del>foo</del>111</p>\n"),
        ("mixed-open", "～～foo~~", "<p><del>foo</del></p>\n"),
        ("mixed-close", "~~foo～～", "<p><del>foo</del></p>\n"),
        ("single", "～foo～", "<p>～foo～</p>\n"),
        ("triple", "～～～foo～～～", "<p>～～～foo～～～</p>\n"),
        ("code", "`～～foo～～`", "<p><code>～～foo～～</code></p>\n"),
    ] {
        assert_eq!(l.markdown(name, md), want, "{name}");
    }
    assert_eq!(l.format("full-width-format", "～～foo～～"), "~~foo~~\n");
}
// Go: test/strikethrough_test.go::TestFullWidthStrikethroughDisabled
#[test]
fn go_fullwidth_strikethrough_disabled() {
    let mut l = Lute::new();
    l.parse_options.full_width_strikethrough = false;
    assert_eq!(
        l.markdown("full-width-disabled", "～～foo～～"),
        "<p>～～foo～～</p>\n"
    );
}
// Go: test/strikethrough_test.go::TestSpinBlockDOMFullWidthStrikethrough
#[test]
fn go_spin_fullwidth_strikethrough() {
    let mut l = engine();
    l.parse_options.text_mark = true;
    l.parse_options.spin = true;
    l.parse_options.full_width_strikethrough = true;
    let dom = concat!(
        r#"<div data-node-id="20260808120000-abcdefg" data-node-index="1" data-type="NodeParagraph" class="p"><div contenteditable="true" spellcheck="false">～～foo～～<wbr>111</div><div class="protyle-attr" contenteditable="false">"#,
        "\u{200b}",
        r#"</div></div>"#
    );
    assert!(l
        .spin_blockdom(dom)
        .contains(r#"<span data-type="s">foo</span><wbr>111"#));
    l.parse_options.full_width_strikethrough = false;
    assert!(l.spin_blockdom(dom).contains("～～foo～～<wbr>111"));
}
