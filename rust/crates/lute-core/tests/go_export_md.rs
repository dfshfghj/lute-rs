//! Original test/protyle_export_md_test.go::TestProtyleExportMd.
//! Preserve its parse -> nonhybrid flatten -> Protyle export renderer chain.
use lute_core::{ast::TESTING, lute::Lute, parse, render::format::FormatRenderer};
use std::sync::atomic::Ordering;

#[test]
fn go_protyle_export_md_original_cases() {
    let mut engine = Lute::new();
    // Exact effects of the original setters, including parse/render paired options.
    engine.parse_options.protyle_wysiwyg = true;
    engine.render_options.protyle_wysiwyg = true;
    engine.parse_options.mark = true;
    engine.parse_options.block_ref = true;
    engine.parse_options.kramdown_block_ial = true;
    engine.parse_options.kramdown_span_ial = true;
    engine.render_options.kramdown_block_ial = true;
    engine.render_options.kramdown_span_ial = true;
    engine.parse_options.super_block = true;
    engine.parse_options.tag = true;
    engine.parse_options.sub = true;
    engine.parse_options.sup = true;
    engine.parse_options.git_conflict = true;
    engine.parse_options.indent_code_block = false;
    engine.parse_options.emoji_site = "http://127.0.0.1:6806/stage/protyle/images/emoji".into();
    engine.render_options.auto_space = true;
    engine.parse_options.paragraph_beginning_space = true;
    engine.render_options.keep_paragraph_beginning_space = true;
    engine.parse_options.file_annotation_ref = true;
    engine
        .parse_options
        .inline_math_allow_digit_after_open_marker = true;
    engine.parse_options.text_mark = true;
    engine.parse_options.img_path_allow_space = true;
    engine.render_options.img_tag = true;
    TESTING.store(true, Ordering::SeqCst);

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/go_original.json")).unwrap();
    let table = fixture["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["table"] == "protyleExportMdTests")
        .expect("original Go Protyle export table");
    assert_eq!(table["source"], "test/protyle_export_md_test.go");
    let cases = table["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    let mut failures = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let input = case["from"].as_str().unwrap();
        let expected = case["to"].as_str().unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut tree = parse::parse("", input.as_bytes(), engine.parse_options.clone());
            parse::nested_inlines2flatted_spans(&mut tree, true);
            let mut renderer = FormatRenderer::new_protyle_export(
                &tree,
                engine.render_options.clone(),
                engine.parse_options.clone(),
            );
            String::from_utf8(renderer.render()).expect("valid UTF-8 export")
        }));
        match result {
            Ok(actual) if actual == expected => {}
            Ok(actual) => failures.push(format!(
                "Go TestProtyleExportMd case {name}, input={input:?}\nexpected={expected:?}\nactual={actual:?}"
            )),
            Err(_) => failures.push(format!(
                "Go TestProtyleExportMd case {name} panicked, input={input:?}"
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} / 20 original Go export cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
