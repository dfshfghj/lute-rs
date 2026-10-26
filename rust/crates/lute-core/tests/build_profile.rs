//! Build-tag contracts from parse/linkrefdef_case_fold{,js}.go.
use lute_core::lute::Lute;

const JAVASCRIPT: bool = cfg!(any(target_arch = "wasm32", feature = "javascript"));

fn assert_reference(definitions: &str, label: &str, native: &str, javascript: &str) {
    let engine = Lute::new();
    let input = format!("[{label}]\n\n{definitions}");
    let destination = if JAVASCRIPT { javascript } else { native };
    let expected = if destination.is_empty() {
        format!("<p>[{label}]</p>\n")
    } else {
        format!("<p><a href=\"{destination}\">{label}</a></p>\n")
    };
    assert_eq!(engine.markdown("", &input), expected, "{input:?}");
}

#[test]
fn only_native_reference_matching_expands_unicode_folds() {
    for (definition, label) in [("ß", "SS"), ("SS", "ẞ"), ("ﬃ", "FFI"), ("ffi", "ﬃ")] {
        assert_reference(&format!("[{definition}]: /url\n"), label, "/url", "");
    }
}

#[test]
fn both_profiles_preserve_go_simple_fold_matching() {
    for (definition, label) in [
        ("Foo", "FOO"),
        ("K", "k"),
        ("ſ", "s"),
        ("ς", "Σ"),
        ("ß", "ẞ"),
    ] {
        assert_reference(&format!("[{definition}]: /url\n"), label, "/url", "/url");
    }
}

#[test]
fn first_matching_definition_respects_the_active_fold_profile() {
    assert_reference(
        "[ẞ]: /expanded\n[SS]: /simple\n",
        "ss",
        "/expanded",
        "/simple",
    );
    assert_reference(
        "[ss]: /expanded\n[ẞ]: /simple\n",
        "ẞ",
        "/expanded",
        "/simple",
    );
    assert_reference("[K]: /first\n[K]: /second\n", "k", "/first", "/first");
    // Go folds either side separately; folding both sides would incorrectly match.
    assert_reference("[ßs]: /wrong\n", "sß", "", "");
    assert_reference("[ßs]: /wrong\n[sß]: /right\n", "sß", "/right", "/right");
}

#[test]
fn footnotes_keep_simple_folding_in_both_profiles() {
    let engine = Lute::new();
    let expanded = engine.markdown("", "[^SS]\n\n[^ß]: note\n");
    assert!(!expanded.contains("footnotes-ref"), "{expanded}");
    let simple = engine.markdown("", "[^k]\n\n[^K]: note\n");
    assert!(simple.contains("footnotes-ref"), "{simple}");
}
