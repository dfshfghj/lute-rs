//! test/parse_inline_test.go::TestInline: retain both original AST and text assertions.
use lute_core::{ast::NodeType, options::ParseOptions, parse::inline};

#[test]
fn go_test_inline_keeps_one_paragraph_and_original_text() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/go_original.json")).unwrap();
    let table = fixture["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["table"] == "inlineTests")
        .unwrap();
    let cases = table["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let tree = inline(
            "",
            case["from"].as_str().unwrap().as_bytes(),
            ParseOptions::new(),
        );
        assert_eq!(
            tree.root().first_child().unwrap().typ(),
            NodeType::NodeParagraph,
            "Go TestInline case {}",
            case["name"]
        );
        assert_eq!(
            tree.root().text(),
            case["to"].as_str().unwrap(),
            "Go TestInline case {}",
            case["name"]
        );
    }
}
