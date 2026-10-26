//! Original Go table inputs, configuration and expected results, without Rust re-recording.
//! Regenerate with: go run ./conformance/cmd/exportgotests
#[path = "support/go_options.rs"]
mod go_options;

use lute_core::{ast::TESTING, lute::Lute};
use serde_json::{json, Value};
use std::collections::HashMap;

fn arg(expression: &str, case: &Value) -> Value {
    match expression {
        "test.from" | "test.original" => case["from"].clone(),
        "test.name" => case["name"].clone(),
        value => serde_json::from_str(value)
            .unwrap_or_else(|_| panic!("unsupported Go argument {value}")),
    }
}

fn invoke(engine: &Lute, api: &str, args: &[Value]) -> Result<String, String> {
    let text = |i: usize| args[i].as_str().expect("string API argument");
    Ok(match api {
        "MarkdownStr" => engine.markdown(text(0), text(1)),
        "FormatStr" => engine.format(text(0), text(1)),
        "Md2BlockDOM" => return engine.try_md2blockdom(text(0), args[1].as_bool().unwrap()),
        "Md2BlockDOMWithAutoLink" => {
            return engine.try_md2blockdom_with_auto_link(text(0), args[1].as_bool().unwrap())
        }
        "HTML2Md" => engine.html2md(text(0)),
        "HTML2BlockDOM" => engine.html2blockdom(text(0)),
        "HTML2VditorDOM" => engine.html2vditor_dom(text(0)),
        "Md2VditorDOM" => engine.md2vditor_dom(text(0)),
        "Md2VditorIRDOM" => engine.md2vditor_ir_dom(text(0)),
        "SpinBlockDOM" => engine.spin_blockdom(text(0)),
        "SpinVditorDOM" => engine.spin_vditor_dom(text(0)),
        "SpinVditorIRDOM" => engine.spin_vditor_ir_dom(text(0)),
        "SpinVditorSVDOM" => engine.spin_vditor_sv_dom(text(0)),
        "VditorDOM2Md" => engine.vditor_dom2md(text(0)),
        "VditorIRDOM2Md" => engine.vditor_ir_dom2md(text(0)),
        "BlockDOM2HTML" => engine.blockdom2html(text(0)),
        "BlockDOM2Md" => engine.blockdom2md(text(0)),
        "BlockDOM2StdMd" => engine.blockdom2stdmd(text(0)),
        "BlockDOM2Content" => engine.blockdom2content(text(0)),
        "BlockDOM2EscapeMarkerContent" => engine.blockdom2_escape_marker_content(text(0)),
        "InlineMd2BlockDOM" => engine.inline_md2blockdom(text(0)),
        "BlockDOM2InlineBlockDOM" => engine.blockdom2inline(text(0)),
        "RenderJSON" => engine.render_json(text(0)),
        "RenderEChartsJSON" => engine.render_echarts_json(text(0)),
        "RenderKityMinderJSON" => engine.render_kityminder_json(text(0)),
        "Callout2Blockquote" => engine.callout2blockquote(text(0)),
        "Blockquote2Callout" => engine.blockquote2callout(text(0)),
        "CancelSuperBlock" => engine.cancel_superblock(text(0)),
        "CancelBlockquote" => engine.cancel_blockquote(text(0)),
        "CancelCallout" => engine.cancel_callout(text(0)),
        "CancelList" => engine.cancel_list(text(0)),
        "Blocks2Ps" => engine.blocks2ps(text(0)),
        "Blocks2Hs" => engine.blocks2hs(text(0), text(1)),
        "UL2TL" => engine.ul2tl(text(0)),
        "TL2OL" => engine.tl2ol(text(0)),
        "UL2OL" => engine.ul2ol(text(0)),
        "OL2UL" => engine.ol2ul(text(0)),
        api => return Err(format!("missing original Go API adapter {api}")),
    })
}

#[test]
fn original_go_table_contracts() {
    TESTING.store(true, std::sync::atomic::Ordering::SeqCst);
    let corpus: Value = serde_json::from_str(include_str!("fixtures/go_original.json")).unwrap();
    assert_eq!(corpus["options_format"], "delta_from_go_defaults");
    let references: Value =
        serde_json::from_str(include_str!("fixtures/build_profiles.json")).unwrap();
    assert_eq!(references["schema_version"], 1);
    assert_eq!(
        references["original_defaults_sha256"],
        corpus["defaults_sha256"]
    );
    let profile_name = if cfg!(any(target_arch = "wasm32", feature = "javascript")) {
        "javascript"
    } else {
        "native-no-chroma"
    };
    let profile = references["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["name"] == profile_name)
        .expect("source build profile is missing");
    let mut reference_rows = HashMap::new();
    for row in profile["rows"].as_array().unwrap() {
        let key = (
            row["table"].as_str().unwrap(),
            row["driver_index"].as_u64().unwrap() as usize,
            row["case_index"].as_u64().unwrap() as usize,
        );
        assert!(
            reference_rows.insert(key, row).is_none(),
            "duplicate source profile row {key:?}"
        );
    }
    let expected_rows: usize = corpus["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| {
            table["drivers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|driver| driver["status"] == "string_table")
                .count()
                * table["cases"].as_array().unwrap().len()
        })
        .sum();
    assert_eq!(
        reference_rows.len(),
        expected_rows,
        "incomplete source profile coverage"
    );
    let mut reference_differences = Vec::new();
    let mut checked = 0;
    let mut failures = Vec::new();
    let mut unsupported = Vec::new();
    let filter = std::env::var("LUTE_GO_TEST_FILTER").unwrap_or_default();
    for table in corpus["tables"].as_array().unwrap() {
        for (driver_index, driver) in table["drivers"].as_array().unwrap().iter().enumerate() {
            let scope = format!(
                "{}::{}::{}",
                table["source"].as_str().unwrap(),
                driver["test"].as_str().unwrap(),
                table["table"].as_str().unwrap()
            );
            if driver["status"] != "string_table" {
                // These original chains have dedicated faithful integration tests.
                if table["table"] != "inlineTests"
                    && table["table"] != "protyleExportMdTests"
                    && scope.contains(&filter)
                {
                    unsupported.push(json!({"source":table["source"],"table":table["table"],"test":driver["test"],"reason":driver["reason"],"case_count":table["cases"].as_array().unwrap().len()}));
                }
                continue;
            }
            let setup_supported = driver["global_setup"].as_array().is_none_or(|setup| {
                setup.iter().all(|step| {
                    if step.as_str() == Some("parse.AddAutoLinkDomainSuffix(\"baz\")") {
                        lute_core::parse::add_auto_link_domain_suffix("baz");
                        true
                    } else {
                        false
                    }
                })
            });
            if !setup_supported {
                // Applying the wrong setup would turn a migration error into a
                // misleading engine difference. The missing API is a hard gap.
                if scope.contains(&filter) {
                    unsupported.push(json!({"source":table["source"],"table":table["table"],"test":driver["test"],"reason":"no Rust API for original global setup","global_setup":driver["global_setup"],"case_count":table["cases"].as_array().unwrap().len()}));
                }
                continue;
            }
            let mut engine = Lute::new();
            go_options::apply_parse_options(&mut engine, &driver["parse_options"]);
            go_options::apply_render_options(&mut engine, &driver["render_options"]);
            for (case_index, case) in table["cases"].as_array().unwrap().iter().enumerate() {
                let id = format!("{scope}::{}", case["name"].as_str().unwrap());
                if !id.contains(&filter) {
                    continue;
                }
                let row = reference_rows
                    .get(&(table["table"].as_str().unwrap(), driver_index, case_index))
                    .expect("missing source profile expectation");
                assert_eq!(row["source"], table["source"]);
                assert_eq!(row["test"], driver["test"]);
                assert_eq!(row["case"], case["name"]);
                assert_eq!(
                    row["original_expected"], case["to"],
                    "source profile no longer matches the archived native fixture"
                );
                let expected = row["expected"]
                    .as_str()
                    .expect("source profile expected string");
                if row["expected"] != case["to"] {
                    reference_differences.push(json!({"source":table["source"],"table":table["table"],"test":driver["test"],"case":case["name"],"driver_index":driver_index,"case_index":case_index,"original_expected":case["to"],"profile_expected":expected}));
                }
                let args: Vec<Value> = driver["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| arg(v.as_str().unwrap(), case))
                    .collect();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    invoke(&engine, driver["api"].as_str().unwrap(), &args)
                }));
                let actual = match result {
                    Ok(Ok(value)) => json!({"value":value}),
                    Ok(Err(error)) => json!({"error":error}),
                    Err(_) => json!({"error":"Rust panic"}),
                };
                checked += 1;
                if actual.get("value").and_then(Value::as_str) != Some(expected) {
                    failures.push(json!({"source":table["source"],"table":table["table"],"test":driver["test"],"case":case["name"],"driver_index":driver_index,"case_index":case_index,"api":driver["api"],"input":case["from"],"expected":expected,"original_expected":case["to"],"actual":actual}));
                }
            }
        }
    }
    assert!(
        checked > 0 || !unsupported.is_empty(),
        "no original Go tests match {filter:?}"
    );
    let report = json!({"build_profile":profile_name,"omitted_capabilities":["chroma"],"reference_differences":reference_differences,"checked":checked,"failures":failures,"unsupported_drivers":unsupported,"filter":filter});
    if let Some(path) = std::env::var_os("LUTE_GO_TEST_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        failures.is_empty() && unsupported.is_empty(),
        "{} / {checked} original Go cases failed; {} original drivers cannot yet be reproduced (strict failure, not ignored):\n{}",
        failures.len(),
        unsupported.len(),
        serde_json::to_string_pretty(&failures.iter().take(3).collect::<Vec<_>>()).unwrap()
    );
}

// Go: test/m2p_test.go::TestMd2BlockDOMWithAutoLinkState
#[test]
fn go_auto_link_warmup_does_not_leak_into_plain_blockdom() {
    TESTING.store(true, std::sync::atomic::Ordering::SeqCst);
    let corpus: Value = serde_json::from_str(include_str!("fixtures/go_original.json")).unwrap();
    let table = corpus["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["table"] == "md2BlockDOMWithAutoLinkStateTests")
        .unwrap();
    let driver = &table["drivers"][0];
    let mut engine = Lute::new();
    go_options::apply_parse_options(&mut engine, &driver["parse_options"]);
    go_options::apply_render_options(&mut engine, &driver["render_options"]);
    let cases = table["cases"].as_array().unwrap();
    for case in cases {
        assert_eq!(
            engine
                .try_md2blockdom(case["from"].as_str().unwrap(), true)
                .unwrap(),
            case["to"].as_str().unwrap(),
            "baseline case {}",
            case["name"]
        );
    }
    let _warmup = engine
        .try_md2blockdom_with_auto_link(cases[0]["from"].as_str().unwrap(), true)
        .unwrap();
    for case in cases {
        assert_eq!(
            engine
                .try_md2blockdom(case["from"].as_str().unwrap(), true)
                .unwrap(),
            case["to"].as_str().unwrap(),
            "after auto-link case {}",
            case["name"]
        );
    }
}

// Go: test/spin_block_test.go::TestSpinBlockDOM (original additional case-288 assertion).
#[test]
fn go_spin_blockdom_case_288_is_idempotent() {
    TESTING.store(true, std::sync::atomic::Ordering::SeqCst);
    let corpus: Value = serde_json::from_str(include_str!("fixtures/go_original.json")).unwrap();
    let table = corpus["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["table"] == "spinBlockDOMTests")
        .unwrap();
    let driver = &table["drivers"][0];
    let mut engine = Lute::new();
    go_options::apply_parse_options(&mut engine, &driver["parse_options"]);
    go_options::apply_render_options(&mut engine, &driver["render_options"]);
    let case = table["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "288")
        .unwrap();
    let first = engine.spin_blockdom(case["from"].as_str().unwrap());
    assert_eq!(first, case["to"].as_str().unwrap(), "original case 288");
    assert_eq!(
        engine.spin_blockdom(&first),
        first,
        "original second-Spin assertion"
    );
}
