// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! JSON 渲染器，对应 Go 版 render/json_renderer.go。
//!
//! Go 版直接 `json.Marshal(ast.Node)`，这里按结构体字段声明顺序手写序列化，
//! 保证键顺序、omitempty 与 HTML 转义规则完全一致。

use std::collections::HashMap;

use crate::ast::{NodeType, RawNode as Node, Tree, WalkStatus};
use crate::options::RenderOptions;

/// Go encoding/json 的字符串转义（含 HTML 转义），不含两侧引号。
pub fn escape_json_string(s: &str) -> String {
    let mut ret = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => ret.push_str("\\\""),
            '\\' => ret.push_str("\\\\"),
            '\n' => ret.push_str("\\n"),
            '\r' => ret.push_str("\\r"),
            '\t' => ret.push_str("\\t"),
            '<' => ret.push_str("\\u003c"),
            '>' => ret.push_str("\\u003e"),
            '&' => ret.push_str("\\u0026"),
            '\u{2028}' => ret.push_str("\\u2028"),
            '\u{2029}' => ret.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                ret.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => ret.push(c),
        }
    }
    ret
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    out.push_str(&escape_json_string(s));
    out.push('"');
}

fn write_key(out: &mut String, key: &str, first: &mut bool) {
    if !*first {
        out.push(',');
    }
    *first = false;
    write_string(out, key);
    out.push(':');
}

fn write_int(out: &mut String, key: &str, v: i32, first: &mut bool) {
    if 0 == v {
        return;
    }
    write_key(out, key, first);
    out.push_str(&v.to_string());
}

fn write_bool(out: &mut String, key: &str, v: bool, first: &mut bool) {
    if !v {
        return;
    }
    write_key(out, key, first);
    out.push_str("true");
}

fn write_byte(out: &mut String, key: &str, v: u8, first: &mut bool) {
    if 0 == v {
        return;
    }
    write_key(out, key, first);
    out.push_str(&v.to_string());
}

fn write_str_field(out: &mut String, key: &str, v: &str, first: &mut bool) {
    if v.is_empty() {
        return;
    }
    write_key(out, key, first);
    write_string(out, v);
}

fn write_bytes(out: &mut String, key: &str, v: &[u8], first: &mut bool) {
    if v.is_empty() {
        return;
    }
    write_key(out, key, first);
    write_string(out, &crate::base64::encode_std(v));
}

fn write_int_list(out: &mut String, key: &str, v: &[i32], first: &mut bool) {
    if v.is_empty() {
        return;
    }
    write_key(out, key, first);
    out.push('[');
    for (i, x) in v.iter().enumerate() {
        if 0 < i {
            out.push(',');
        }
        out.push_str(&x.to_string());
    }
    out.push(']');
}

fn write_properties(out: &mut String, props: &HashMap<String, String>, first: &mut bool) {
    if props.is_empty() {
        return;
    }
    let mut keys: Vec<&String> = props.keys().collect();
    keys.sort();
    write_key(out, "Properties", first);
    out.push('{');
    for (i, k) in keys.iter().enumerate() {
        if 0 < i {
            out.push(',');
        }
        write_string(out, k);
        out.push(':');
        write_string(out, &props[*k]);
    }
    out.push('}');
}

fn write_list_data(out: &mut String, node: Node, first: &mut bool) {
    let ld = match node.data().list_data.as_ref() {
        Some(ld) => ld,
        None => return,
    };
    write_key(out, "ListData", first);
    out.push('{');
    let mut f = true;
    write_int(out, "Typ", ld.typ, &mut f);
    write_bool(out, "Tight", ld.tight, &mut f);
    write_byte(out, "BulletChar", ld.bullet_char, &mut f);
    write_int(out, "Start", ld.start, &mut f);
    write_byte(out, "Delimiter", ld.delimiter, &mut f);
    write_int(out, "Padding", ld.padding, &mut f);
    write_int(out, "MarkerOffset", ld.marker_offset, &mut f);
    write_bool(out, "Checked", ld.checked, &mut f);
    write_bytes(out, "Marker", &ld.marker, &mut f);
    write_int(out, "Num", ld.num, &mut f);
    out.push('}');
}

fn write_table_cell_rich(out: &mut String, node: Node, first: &mut bool) {
    let rich = match node.data().table_cell_rich.as_ref() {
        Some(r) => r,
        None => return,
    };
    write_key(out, "TableCellRich", first);
    out.push('{');
    out.push_str("\"spec\":");
    out.push_str(&rich.spec.to_string());
    out.push_str(",\"format\":");
    write_string(out, &rich.format);
    out.push_str(",\"content\":");
    write_string(out, &rich.content);
    out.push('}');
}

/// 序列化单个节点（不含 Children 字段），对应 json.Marshal(node)。
fn write_node_fields(out: &mut String, node: Node, type_str: &str, data: &str, first: &mut bool) {
    let d = node.data();
    write_str_field(out, "ID", &d.id, first);
    write_str_field(out, "Spec", &d.spec, first);
    write_key(out, "Type", first);
    write_string(out, type_str);
    if !data.is_empty() {
        write_key(out, "Data", first);
        write_string(out, data);
    }
    write_int(out, "CodeMarkerLen", d.code_marker_len, first);
    write_bool(out, "IsFencedCodeBlock", d.is_fenced_code_block, first);
    write_byte(out, "CodeBlockFenceChar", d.code_block_fence_char, first);
    write_int(out, "CodeBlockFenceLen", d.code_block_fence_len, first);
    write_int(
        out,
        "CodeBlockFenceOffset",
        d.code_block_fence_offset,
        first,
    );
    write_bytes(out, "CodeBlockOpenFence", &d.code_block_open_fence, first);
    write_bytes(out, "CodeBlockInfo", &d.code_block_info, first);
    write_bytes(out, "CodeBlockCloseFence", &d.code_block_close_fence, first);
    write_int(out, "HtmlBlockType", d.html_block_type, first);
    write_list_data(out, node, first);
    write_bool(out, "TaskListItemChecked", d.task_list_item_checked, first);
    write_byte(out, "TaskListItemMarker", d.task_list_item_marker, first);
    write_int_list(out, "TableAligns", &d.table_aligns, first);
    write_int(out, "TableCellAlign", d.table_cell_align, first);
    write_int(
        out,
        "TableCellContentWidth",
        d.table_cell_content_width,
        first,
    );
    write_int(
        out,
        "TableCellContentMaxWidth",
        d.table_cell_content_max_width,
        first,
    );
    write_table_cell_rich(out, node, first);
    write_int(out, "LinkType", d.link_type, first);
    write_bytes(out, "LinkRefLabel", &d.link_ref_label, first);
    write_int(out, "HeadingLevel", d.heading_level, first);
    write_bool(out, "HeadingSetext", d.heading_setext, first);
    write_str_field(out, "HeadingNormalizedID", &d.heading_normalized_id, first);
    write_int(
        out,
        "MathBlockDollarOffset",
        d.math_block_dollar_offset,
        first,
    );
    write_bytes(out, "FootnotesRefLabel", &d.footnotes_ref_label, first);
    write_str_field(out, "FootnotesRefId", &d.footnotes_ref_id, first);
    if !d.footnotes_refs.is_empty() {
        write_key(out, "FootnotesRefs", first);
        out.push('[');
        for (i, r) in d.footnotes_refs.iter().enumerate() {
            if 0 < i {
                out.push(',');
            }
            out.push('{');
            let mut f = true;
            write_node_fields(out, *r, "", "", &mut f);
            out.push('}');
        }
        out.push(']');
    }
    write_bytes(out, "HtmlEntityTokens", &d.html_entity_tokens, first);
    write_properties(out, &d.properties, first);
    write_str_field(out, "TextMarkType", &d.text_mark_type, first);
    write_str_field(out, "TextMarkAHref", &d.text_mark_a_href, first);
    write_str_field(out, "TextMarkATitle", &d.text_mark_a_title, first);
    write_str_field(
        out,
        "TextMarkInlineMathContent",
        &d.text_mark_inline_math_content,
        first,
    );
    write_str_field(
        out,
        "TextMarkInlineMemoContent",
        &d.text_mark_inline_memo_content,
        first,
    );
    write_str_field(out, "TextMarkBlockRefID", &d.text_mark_block_ref_id, first);
    write_str_field(
        out,
        "TextMarkBlockRefSubtype",
        &d.text_mark_block_ref_subtype,
        first,
    );
    write_str_field(
        out,
        "TextMarkFileAnnotationRefID",
        &d.text_mark_file_annotation_ref_id,
        first,
    );
    write_str_field(
        out,
        "TextMarkFlashcardOcclusionID",
        &d.text_mark_flashcard_occlusion_id,
        first,
    );
    write_str_field(out, "TextMarkTextContent", &d.text_mark_text_content, first);
    write_str_field(out, "AttributeViewID", &d.attribute_view_id, first);
    write_str_field(out, "AttributeViewType", &d.attribute_view_type, first);
    write_int(
        out,
        "CustomBlockFenceOffset",
        d.custom_block_fence_offset,
        first,
    );
    write_str_field(out, "CustomBlockInfo", &d.custom_block_info, first);
    write_str_field(out, "CalloutType", &d.callout_type, first);
    write_str_field(out, "CalloutTitle", &d.callout_title, first);
    write_bool(out, "CalloutTitleExplicit", d.callout_title_explicit, first);
    write_str_field(out, "TabItemTitle", &d.tab_item_title, first);
    write_str_field(out, "CalloutIcon", &d.callout_icon, first);
    write_int(out, "CalloutIconType", d.callout_icon_type, first);
}

/// 对应 Go 版 render.NewJSONRenderer + Render。
pub fn render_json(tree: &Tree, _options: &RenderOptions) -> String {
    // 渲染器剔除语法树块级 IAL 节点
    let root = tree.root_raw();
    let mut ials = Vec::new();
    crate::ast::walk_raw(root, &mut |n, entering| {
        if entering && NodeType::NodeKramdownBlockIAL == n.typ() {
            ials.push(n);
        }
        WalkStatus::WalkContinue
    });
    for ial in ials {
        ial.unlink();
    }

    let mut out = String::new();
    crate::ast::walk_raw(root, &mut |n, entering| {
        if entering {
            if n.previous().is_some() {
                out.push(',');
            }
            let data = String::from_utf8_lossy(n.tokens()).to_string();
            let props = node_properties(n);
            let mut first = true;
            out.push('{');
            // Properties 需要在 KramdownIAL 基础上写入，先临时替换
            let saved = std::mem::replace(&mut n.data_mut().properties, props);
            write_node_fields(&mut out, n, n.typ().as_str(), &data, &mut first);
            n.data_mut().properties = saved;
            if n.first_child().is_some() {
                if !first {
                    out.push(',');
                }
                out.push_str("\"Children\":[");
            } else {
                out.push('}');
            }
        } else if n.first_child().is_some() {
            out.push(']');
            out.push('}');
        }
        WalkStatus::WalkContinue
    });
    out
}

fn node_properties(node: Node) -> HashMap<String, String> {
    let mut ret = HashMap::new();
    for (k, v) in node.ial_raw() {
        ret.insert(k, v);
    }
    ret.remove("refcount");
    ret.remove("av-names");
    ret
}
