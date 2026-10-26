// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 一致性测试驱动器：读取用例 JSON，输出 API 结果 JSON。
//!
//! 用法：cargo run --example md2html -- cases.json

use std::collections::HashMap;
use std::io::Read;

use lute_core::lute::Lute;
use serde_json::{json, Value};

fn string_map(v: &Value) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Some(o) = v.as_object() {
        for (k, val) in o {
            m.insert(k.clone(), val.as_str().unwrap_or_default().to_string());
        }
    }
    m
}

fn set(lute: &mut Lute, name: &str, v: &Value) {
    let b = v.as_bool().unwrap_or(false);
    let s = v.as_str().unwrap_or_default().to_string();
    match name {
        "SetGFMTable" => lute.parse_options.gfm_table = b,
        "SetGFMTaskListItem" => lute.parse_options.gfm_task_list_item = b,
        "SetArbitraryTaskListItemMarker" => lute.parse_options.arbitrary_task_list_item_marker = b,
        "SetGFMTaskListItemClass" => lute.render_options.gfm_task_list_item_class = s,
        "SetDataTask" => lute.render_options.data_task = b,
        "SetExportNormalizeTaskListMarker" => {
            lute.render_options.export_normalize_task_list_marker = b
        }
        "SetGFMStrikethrough" => lute.parse_options.gfm_strikethrough = b,
        "SetGFMStrikethrough1" => lute.parse_options.gfm_strikethrough1 = b,
        "SetFullWidthStrikethrough" => lute.parse_options.full_width_strikethrough = b,
        "SetGFMAutoLink" => lute.parse_options.gfm_auto_link = b,
        "SetSoftBreak2HardBreak" => lute.render_options.soft_break2_hard_break = b,
        "SetCodeSyntaxHighlight" => lute.render_options.code_syntax_highlight = b,
        "SetCodeSyntaxHighlightDetectLang" => {
            lute.render_options.code_syntax_highlight_detect_lang = b
        }
        "SetCodeSyntaxHighlightInlineStyle" => {
            lute.render_options.code_syntax_highlight_inline_style = b
        }
        "SetCodeSyntaxHighlightLineNum" => lute.render_options.code_syntax_highlight_line_num = b,
        "SetCodeSyntaxHighlightStyleName" => {
            lute.render_options.code_syntax_highlight_style_name = s
        }
        "SetFootnotes" => lute.parse_options.footnotes = b,
        "SetToC" => {
            lute.parse_options.toc = b;
            lute.render_options.toc = b;
        }
        "SetHeadingID" => {
            lute.parse_options.heading_id = b;
            lute.render_options.heading_id = b;
        }
        "SetAutoSpace" => lute.render_options.auto_space = b,
        "SetFixTermTypo" => lute.render_options.fix_term_typo = b,
        "SetEmoji" => lute.parse_options.emoji = b,
        "SetEmojis" => lute.set_emojis(string_map(v)),
        "PutEmojis" => lute.put_emojis(string_map(v)),
        "SetEmojiSite" => lute.parse_options.emoji_site = s,
        "SetHeadingAnchor" => lute.render_options.heading_anchor = b,
        "SetTerms" => lute.set_terms(string_map(v)),
        "PutTerms" => lute.put_terms(string_map(v)),
        "SetVditorWYSIWYG" => {
            lute.parse_options.vditor_wysiwyg = b;
            lute.render_options.vditor_wysiwyg = b;
        }
        "SetProtyleWYSIWYG" => {
            lute.parse_options.protyle_wysiwyg = b;
            lute.render_options.protyle_wysiwyg = b;
        }
        "SetVditorIR" => {
            lute.parse_options.vditor_ir = b;
            lute.render_options.vditor_ir = b;
        }
        "SetVditorSV" => {
            lute.parse_options.vditor_sv = b;
            lute.render_options.vditor_sv = b;
        }
        "SetInlineMath" => lute.parse_options.inline_math = b,
        "SetInlineMathAllowDigitAfterOpenMarker" => {
            lute.parse_options.inline_math_allow_digit_after_open_marker = b
        }
        "SetLinkPrefix" => lute.render_options.link_prefix = s,
        "SetLinkBase" => lute.render_options.link_base = s,
        "SetVditorCodeBlockPreview" => lute.render_options.vditor_code_block_preview = b,
        "SetVditorMathBlockPreview" => lute.render_options.vditor_math_block_preview = b,
        "SetVditorHTMLBlockPreview" => lute.render_options.vditor_html_block_preview = b,
        "SetRenderListStyle" => lute.render_options.render_list_style = b,
        "SetSanitize" => lute.render_options.sanitize = b,
        "SetImageLazyLoading" => lute.render_options.image_lazy_loading = s,
        "SetChineseParagraphBeginningSpace" => {
            lute.render_options.chinese_paragraph_beginning_space = b
        }
        "SetYamlFrontMatter" => lute.parse_options.yaml_front_matter = b,
        "SetSetext" => lute.parse_options.setext = b,
        "SetBlockRef" => lute.parse_options.block_ref = b,
        "SetFileAnnotationRef" => lute.parse_options.file_annotation_ref = b,
        "SetMark" => lute.parse_options.mark = b,
        "SetKramdownIAL" => {
            lute.parse_options.kramdown_block_ial = b;
            lute.parse_options.kramdown_span_ial = b;
            lute.render_options.kramdown_block_ial = b;
            lute.render_options.kramdown_span_ial = b;
        }
        "SetKramdownBlockIAL" => {
            lute.parse_options.kramdown_block_ial = b;
            lute.render_options.kramdown_block_ial = b;
        }
        "SetKramdownSpanIAL" => {
            lute.parse_options.kramdown_span_ial = b;
            lute.render_options.kramdown_span_ial = b;
        }
        "SetKramdownIALIDRenderName" => lute.render_options.kramdown_ial_id_render_name = s,
        "SetTag" => lute.parse_options.tag = b,
        "SetImgPathAllowSpace" => lute.parse_options.img_path_allow_space = b,
        "SetSuperBlock" => {
            lute.parse_options.super_block = b;
            lute.render_options.super_block = b;
        }
        "SetCustomBlock" => lute.parse_options.custom_block = b,
        "SetSup" => lute.parse_options.sup = b,
        "SetSub" => lute.parse_options.sub = b,
        "SetInlineAsterisk" => lute.parse_options.inline_asterisk = b,
        "SetInlineUnderscore" => lute.parse_options.inline_underscore = b,
        "SetGitConflict" => lute.parse_options.git_conflict = b,
        "SetLinkRef" => lute.parse_options.link_ref = b,
        "SetIndentCodeBlock" => lute.parse_options.indent_code_block = b,
        "SetDataImage" => lute.parse_options.data_image = b,
        "SetTextMark" => lute.parse_options.text_mark = b,
        "SetSpin" => lute.parse_options.spin = b,
        "SetHTML2MarkdownAttrs" => {
            lute.parse_options.html2_markdown_attrs = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }
        "SetHTMLTag2TextMark" => lute.parse_options.html_tag2_text_mark = b,
        "SetParagraphBeginningSpace" => lute.parse_options.paragraph_beginning_space = b,
        "SetProtyleMarkNetImg" => lute.render_options.protyle_mark_net_img = b,
        "SetSpellcheck" => lute.render_options.spellcheck = b,
        "SetUnorderedListMarker" => lute.render_options.unordered_list_marker = s,
        "SetImgTag" => lute.render_options.img_tag = b,
        "SetPreventEncodeLinkSpace" => lute.render_options.prevent_encode_link_space = b,
        "SetCallout" => lute.parse_options.callout = b,
        "SetTabs" => lute.parse_options.tabs = b,
        "SetEnsureListItemParagraph" => lute.parse_options.ensure_list_item_paragraph = b,
        other => eprintln!("unhandled setup: {other}"),
    }
}

fn map_json(m: &HashMap<String, String>) -> Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in m {
        obj.insert(k.clone(), json!(v));
    }
    Value::Object(obj)
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: md2html <cases.json>");
    let mut input = String::new();
    std::fs::File::open(&path)
        .expect("open cases")
        .read_to_string(&mut input)
        .expect("read cases");
    let cases: Vec<Value> = serde_json::from_str(&input).expect("parse cases");

    let mut out = serde_json::Map::new();
    for case in &cases {
        let id = case["id"].as_str().unwrap_or_default();
        let api = case["api"].as_str().unwrap_or_default();
        let args = case["args"].as_array().cloned().unwrap_or_default();
        let mut lute = Lute::new();
        if let Some(setup) = case.get("setup").and_then(|s| s.as_array()) {
            for call in setup {
                let name = call[0].as_str().unwrap_or_default();
                set(&mut lute, name, &call[1]);
            }
        }
        let arg_str = |i: usize| args.get(i).and_then(|a| a.as_str()).unwrap_or_default();
        let value = match api {
            "MarkdownStr" => json!(lute.markdown(arg_str(0), arg_str(1))),
            "FormatStr" => json!(lute.format(arg_str(0), arg_str(1))),
            "RenderJSON" => json!(lute.render_json(arg_str(0))),
            "TextBundleStr" => {
                let prefixes = args
                    .get(2)
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let (text, links) = lute.text_bundle(arg_str(0), arg_str(1), prefixes);
                match links {
                    Some(links) => json!([text, links]),
                    None => json!([text, null]),
                }
            }
            "Md2BlockDOM" => json!(lute.md2blockdom(
                arg_str(0),
                args.get(1).and_then(|a| a.as_bool()).unwrap_or(false)
            )),
            "Md2BlockDOMWithAutoLink" => json!(lute.md2blockdom_with_auto_link(
                arg_str(0),
                args.get(1).and_then(|a| a.as_bool()).unwrap_or(false)
            )),
            "HTML2BlockDOM" => json!(lute.html2blockdom(arg_str(0))),
            "BlockDOM2Md" => json!(lute.blockdom2md(arg_str(0))),
            "BlockDOM2StdMd" => json!(lute.blockdom2stdmd(arg_str(0))),
            "BlockDOM2HTML" => json!(lute.blockdom2html(arg_str(0))),
            "BlockDOM2Text" => json!(lute.blockdom2text(arg_str(0))),
            "BlockDOM2TextLen" => json!(lute.blockdom2textlen(arg_str(0))),
            "BlockDOM2Content" => json!(lute.blockdom2content(arg_str(0))),
            "BlockDOM2EscapeMarkerContent" => {
                json!(lute.blockdom2_escape_marker_content(arg_str(0)))
            }
            "BlockDOM2RichHTML" => json!(lute.blockdom2richhtml(arg_str(0))),
            "BlockDOM2InlineBlockDOM" => json!(lute.blockdom2inline(arg_str(0))),
            "InlineMd2BlockDOM" => json!(lute.inline_md2blockdom(arg_str(0))),
            "SpinBlockDOM" => {
                if arg_str(0).is_empty() {
                    json!(null)
                } else {
                    json!(lute.spin_blockdom(arg_str(0)))
                }
            }
            "OL2UL" => json!(lute.ol2ul(arg_str(0))),
            "UL2OL" => json!(lute.ul2ol(arg_str(0))),
            "UL2TL" => json!(lute.ul2tl(arg_str(0))),
            "OL2TL" => json!(lute.ol2tl(arg_str(0))),
            "TL2UL" => json!(lute.tl2ul(arg_str(0))),
            "TL2OL" => json!(lute.tl2ol(arg_str(0))),
            "ConvertListType" => json!(lute.convert_list_type(arg_str(0), arg_str(1))),
            "Blockquote2Callout" => json!(lute.blockquote2callout(arg_str(0))),
            "Callout2Blockquote" => json!(lute.callout2blockquote(arg_str(0))),
            "CancelBlockquote" => json!(lute.cancel_blockquote(arg_str(0))),
            "CancelCallout" => json!(lute.cancel_callout(arg_str(0))),
            "CancelList" => json!(lute.cancel_list(arg_str(0))),
            "CancelListRecursively" => json!(lute.cancel_list_recursively(arg_str(0))),
            "CancelSuperBlock" => json!(lute.cancel_superblock(arg_str(0))),
            "Blocks2Ps" => json!(lute.blocks2ps(arg_str(0))),
            "Blocks2Hs" => json!(lute.blocks2hs(arg_str(0), arg_str(1))),
            "Md2VditorDOM" => json!(lute.md2vditor_dom(arg_str(0))),
            "HTML2VditorDOM" => json!(lute.html2vditor_dom(arg_str(0))),
            "VditorDOM2HTML" => json!(lute.vditor_dom2html(arg_str(0))),
            "VditorDOM2Md" => json!(lute.vditor_dom2md(arg_str(0))),
            "SpinVditorDOM" => json!(lute.spin_vditor_dom(arg_str(0))),
            "Md2VditorIRDOM" => json!(lute.md2vditor_ir_dom(arg_str(0))),
            "HTML2VditorIRDOM" => json!(lute.html2vditor_ir_dom(arg_str(0))),
            "VditorIRDOM2HTML" => json!(lute.vditor_ir_dom2html(arg_str(0))),
            "VditorIRDOM2Md" => json!(lute.vditor_ir_dom2md(arg_str(0))),
            "SpinVditorIRDOM" => json!(lute.spin_vditor_ir_dom(arg_str(0))),
            "Md2VditorSVDOM" => json!(lute.md2vditor_sv_dom(arg_str(0))),
            "HTML2VditorSVDOM" => json!(lute.html2vditor_sv_dom(arg_str(0))),
            "SpinVditorSVDOM" => json!(lute.spin_vditor_sv_dom(arg_str(0))),
            "Md2HTML" => json!(lute.markdown("", arg_str(0))),
            "ProtylePreviewStr" => json!(lute.protyle_preview_str(arg_str(0), arg_str(1))),
            "GetLinkBase" => json!(lute.get_link_base()),
            "Lute.BlockDOM2Content" => json!(Lute::blockdom2content_global(arg_str(0))),
            "Space" => json!(Lute::space(arg_str(0))),
            "IsValidLinkDest" => json!(lute.is_valid_link_dest(arg_str(0))),
            "GetLinkDest" => json!(lute.get_link_dest(arg_str(0))),
            "GetEmojis" => map_json(&lute.get_emojis()),
            "RemoveEmoji" => json!(lute.remove_emoji(arg_str(0))),
            "GetTerms" => map_json(&lute.get_terms()),
            "HTML2Md" => json!(lute.html2md(arg_str(0))),
            "HTML2Markdown" => {
                let (md, err) = lute.html2markdown(arg_str(0));
                json!([md, err])
            }
            "HTML2Text" => json!(lute.html2text(arg_str(0))),
            "RenderEChartsJSON" => json!(lute.render_echarts_json(arg_str(0))),
            "RenderKityMinderJSON" => json!(lute.render_kityminder_json(arg_str(0))),
            "Lute.EChartsMindmapStr" => json!(Lute::echarts_mindmap_str(arg_str(0))),
            "Lute.Sanitize" => json!(Lute::sanitize(arg_str(0))),
            "Lute.EscapeHTMLStr" => json!(lute_core::html::escape_html_str(arg_str(0))),
            "Lute.UnEscapeHTMLStr" => json!(lute_core::html::unescape_html_str(arg_str(0))),
            "Lute.Constants" => json!({
                "Caret": lute_core::editor::CARET,
                "WalkStop": 0,
                "WalkSkipChildren": 1,
                "WalkContinue": 2
            }),
            _ => json!({ "$unsupported": api }),
        };
        out.insert(id.to_string(), value);
    }
    println!("{}", serde_json::to_string(&Value::Object(out)).unwrap());
}
