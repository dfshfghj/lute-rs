//! Migrated from test/commonmark_spec_test.go::TestSpec, including its exact options.
use lute_core::lute::Lute;

#[test]
fn go_commonmark_spec_html() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let cases: Vec<serde_json::Value> = serde_json::from_slice(
        &std::fs::read(root.join("conformance/fixtures/commonmark-spec.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cases.len(), 652);
    let javascript: Option<Vec<String>> = cfg!(any(target_arch = "wasm32", feature = "javascript"))
        .then(|| {
            serde_json::from_slice(
                &std::fs::read(root.join("conformance/golden/html_commonmark_js.json")).unwrap(),
            )
            .unwrap()
        });
    if let Some(expected) = &javascript {
        assert_eq!(expected.len(), cases.len());
    }
    let mut engine = Lute::new();
    engine.parse_options.gfm_task_list_item = false;
    engine.parse_options.gfm_table = false;
    engine.parse_options.gfm_auto_link = false;
    engine.parse_options.gfm_strikethrough = false;
    engine.render_options.soft_break2_hard_break = false;
    engine.render_options.code_syntax_highlight = false;
    engine.parse_options.heading_id = false;
    engine.render_options.heading_id = false;
    engine.render_options.auto_space = false;
    engine.render_options.fix_term_typo = false;
    engine.parse_options.emoji = false;
    engine.parse_options.yaml_front_matter = false;
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let markdown = case["markdown"].as_str().unwrap();
        let expected = javascript
            .as_ref()
            .map(|rows| rows[index].as_str())
            .unwrap_or_else(|| case["html"].as_str().unwrap());
        let actual = engine.markdown("", markdown);
        if actual != expected {
            failures.push(format!("Go TestSpec example {} ({}), input={markdown:?}\nexpected={expected:?}\nactual={actual:?}",case["example"],case["section"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / 652 original Go HTML cases failed:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
