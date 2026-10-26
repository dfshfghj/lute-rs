// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! 解析与渲染选项。字段与默认值必须与 Go 版 parse.Options / render.Options 完全一致。

use std::collections::HashMap;

#[cfg(feature = "emoji")]
use crate::emoji::EMOJI_ALIAS_UNICODE;
use crate::terms::TERMS;

/// Go 版 parse.Options。
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ParseOptions {
    /// 仅在外部 Markdown 导入时保留单元格原文供结构恢复。
    pub preserve_table_cell_source: bool,
    /// 代码内容的二次解析保留普通 HTML 标签，不把字面样式转换为文本标记。
    pub inline_code_html: bool,
    /// 禁止在富文本片段内部递归展开单元格富文本。
    pub disable_table_cell_rich: bool,

    pub gfm_table: bool,
    pub gfm_task_list_item: bool,
    pub gfm_strikethrough: bool,
    pub gfm_strikethrough1: bool,
    pub full_width_strikethrough: bool,
    pub gfm_auto_link: bool,
    pub footnotes: bool,
    pub heading_id: bool,
    pub toc: bool,
    pub emoji: bool,
    pub alias_emoji: HashMap<String, String>,
    pub emoji_alias: HashMap<String, String>,
    pub emoji_site: String,
    pub vditor_wysiwyg: bool,
    pub vditor_ir: bool,
    pub vditor_sv: bool,
    pub protyle_wysiwyg: bool,
    pub protyle_wysiwyg_auto_link: bool,
    pub inline_math: bool,
    pub inline_math_allow_digit_after_open_marker: bool,
    pub setext: bool,
    pub yaml_front_matter: bool,
    pub block_ref: bool,
    pub file_annotation_ref: bool,
    pub mark: bool,
    pub kramdown_block_ial: bool,
    pub kramdown_span_ial: bool,
    pub tag: bool,
    pub img_path_allow_space: bool,
    pub super_block: bool,
    pub custom_block: bool,
    pub sup: bool,
    pub sub: bool,
    pub inline_asterisk: bool,
    pub inline_underscore: bool,
    pub git_conflict: bool,
    pub link_ref: bool,
    pub indent_code_block: bool,
    pub paragraph_beginning_space: bool,
    pub data_image: bool,
    pub text_mark: bool,
    pub html_tag2_text_mark: bool,
    pub spin: bool,
    pub html2_markdown_attrs: Vec<String>,
    pub callout: bool,
    pub tabs: bool,
    pub keep_escaped: bool,
    pub arbitrary_task_list_item_marker: bool,
    pub ensure_list_item_paragraph: bool,
}

#[cfg(feature = "emoji")]
fn emoji_maps() -> (HashMap<String, String>, HashMap<String, String>) {
    let mut alias_emoji = HashMap::with_capacity(EMOJI_ALIAS_UNICODE.len());
    let mut emoji_alias = HashMap::with_capacity(EMOJI_ALIAS_UNICODE.len());
    for (alias, unicode) in EMOJI_ALIAS_UNICODE {
        alias_emoji.insert((*alias).to_string(), (*unicode).to_string());
        emoji_alias.insert((*unicode).to_string(), (*alias).to_string());
    }
    (alias_emoji, emoji_alias)
}

impl ParseOptions {
    /// 对应 Go 版 parse.NewOptions()。
    pub fn new() -> Self {
        #[cfg(feature = "emoji")]
        let (alias_emoji, emoji_alias) = emoji_maps();
        #[cfg(not(feature = "emoji"))]
        let (alias_emoji, emoji_alias) = (HashMap::new(), HashMap::new());

        Self {
            preserve_table_cell_source: false,
            inline_code_html: false,
            disable_table_cell_rich: false,
            gfm_table: true,
            gfm_task_list_item: true,
            gfm_strikethrough: true,
            gfm_strikethrough1: true,
            full_width_strikethrough: false,
            gfm_auto_link: true,
            footnotes: true,
            heading_id: true,
            toc: false,
            emoji: true,
            alias_emoji,
            emoji_alias,
            emoji_site: "https://cdn.jsdelivr.net/npm/vditor/dist/images/emoji".to_string(),
            vditor_wysiwyg: false,
            vditor_ir: false,
            vditor_sv: false,
            protyle_wysiwyg: false,
            protyle_wysiwyg_auto_link: false,
            inline_math: true,
            inline_math_allow_digit_after_open_marker: false,
            setext: true,
            yaml_front_matter: true,
            block_ref: false,
            file_annotation_ref: false,
            mark: false,
            kramdown_block_ial: false,
            kramdown_span_ial: false,
            tag: false,
            img_path_allow_space: false,
            super_block: false,
            custom_block: false,
            sup: false,
            sub: false,
            inline_asterisk: true,
            inline_underscore: true,
            git_conflict: false,
            link_ref: true,
            indent_code_block: true,
            paragraph_beginning_space: false,
            data_image: true,
            text_mark: false,
            html_tag2_text_mark: false,
            spin: false,
            html2_markdown_attrs: Vec::new(),
            callout: false,
            tabs: false,
            keep_escaped: false,
            arbitrary_task_list_item_marker: false,
            ensure_list_item_paragraph: false,
        }
    }

    /// 对应 Go 版 Options.IsValidTaskListItemMarker。
    pub fn is_valid_task_list_item_marker(&self, marker: u8) -> bool {
        (marker == b' ' || marker == b'x' || marker == b'X')
            || (self.arbitrary_task_list_item_marker && marker != b'[' && marker != b']')
    }
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Go 版 render.Options。
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RenderOptions {
    /// 剪贴板保留页签结构，文件导出默认平铺。
    pub tabs_markdown: bool,
    pub soft_break2_hard_break: bool,
    pub auto_space: bool,
    pub render_list_style: bool,
    pub code_syntax_highlight: bool,
    pub code_syntax_highlight_detect_lang: bool,
    pub code_syntax_highlight_inline_style: bool,
    pub code_syntax_highlight_line_num: bool,
    pub code_syntax_highlight_style_name: String,
    pub vditor_wysiwyg: bool,
    pub vditor_ir: bool,
    pub vditor_sv: bool,
    pub protyle_wysiwyg: bool,
    pub kramdown_block_ial: bool,
    pub kramdown_span_ial: bool,
    pub omit_table_cell_ial: bool,
    pub super_block: bool,
    pub image_lazy_loading: String,
    pub chinese_paragraph_beginning_space: bool,
    pub sanitize: bool,
    pub fix_term_typo: bool,
    pub terms: HashMap<String, String>,
    pub toc: bool,
    pub heading_id: bool,
    pub kramdown_ial_id_render_name: String,
    pub heading_anchor: bool,
    pub gfm_task_list_item_class: String,
    pub data_task: bool,
    pub vditor_code_block_preview: bool,
    pub vditor_math_block_preview: bool,
    pub vditor_html_block_preview: bool,
    pub link_base: String,
    pub link_prefix: String,
    pub node_index_start: i32,
    pub protyle_contenteditable: bool,
    pub keep_paragraph_beginning_space: bool,
    pub protyle_mark_net_img: bool,
    pub spellcheck: bool,
    pub unordered_list_marker: String,
    pub img_tag: bool,
    pub prevent_encode_link_space: bool,
    pub export_normalize_task_list_marker: bool,
}

impl RenderOptions {
    /// 对应 Go 版 render.NewOptions()。
    pub fn new() -> Self {
        let mut terms = HashMap::with_capacity(TERMS.len());
        for (k, v) in TERMS {
            terms.insert((*k).to_string(), (*v).to_string());
        }

        Self {
            tabs_markdown: false,
            soft_break2_hard_break: true,
            auto_space: false,
            render_list_style: false,
            code_syntax_highlight: true,
            code_syntax_highlight_detect_lang: false,
            code_syntax_highlight_inline_style: false,
            code_syntax_highlight_line_num: false,
            code_syntax_highlight_style_name: "github".to_string(),
            vditor_wysiwyg: false,
            vditor_ir: false,
            vditor_sv: false,
            protyle_wysiwyg: false,
            kramdown_block_ial: false,
            kramdown_span_ial: false,
            omit_table_cell_ial: false,
            super_block: false,
            image_lazy_loading: String::new(),
            chinese_paragraph_beginning_space: false,
            sanitize: false,
            fix_term_typo: false,
            terms,
            toc: false,
            heading_id: false,
            kramdown_ial_id_render_name: "id".to_string(),
            heading_anchor: false,
            gfm_task_list_item_class: "vditor-task".to_string(),
            data_task: false,
            vditor_code_block_preview: true,
            vditor_math_block_preview: true,
            vditor_html_block_preview: true,
            link_base: String::new(),
            link_prefix: String::new(),
            node_index_start: 1,
            protyle_contenteditable: true,
            keep_paragraph_beginning_space: false,
            protyle_mark_net_img: true,
            spellcheck: false,
            unordered_list_marker: String::new(),
            img_tag: false,
            prevent_encode_link_space: false,
            export_normalize_task_list_marker: true,
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_match_go() {
        let o = ParseOptions::new();
        assert!(o.gfm_table);
        assert!(o.gfm_task_list_item);
        assert!(o.gfm_strikethrough);
        assert!(o.gfm_strikethrough1);
        assert!(!o.full_width_strikethrough);
        assert!(o.gfm_auto_link);
        assert!(o.footnotes);
        assert!(o.heading_id);
        assert!(!o.toc);
        assert!(o.emoji);
        assert_eq!(2126, o.alias_emoji.len());
        // Unicode 有 37 组重复，Go 版 EmojiUnicodeAlias 同样是 2089
        assert_eq!(2089, o.emoji_alias.len());
        assert_eq!(
            "https://cdn.jsdelivr.net/npm/vditor/dist/images/emoji",
            o.emoji_site
        );
        assert!(o.inline_math);
        assert!(o.setext);
        assert!(o.yaml_front_matter);
        assert!(!o.block_ref);
        assert!(!o.file_annotation_ref);
        assert!(!o.mark);
        assert!(!o.kramdown_block_ial);
        assert!(!o.kramdown_span_ial);
        assert!(!o.tag);
        assert!(!o.img_path_allow_space);
        assert!(!o.super_block);
        assert!(!o.custom_block);
        assert!(!o.sup);
        assert!(!o.sub);
        assert!(o.inline_asterisk);
        assert!(o.inline_underscore);
        assert!(!o.git_conflict);
        assert!(o.link_ref);
        assert!(o.indent_code_block);
        assert!(!o.paragraph_beginning_space);
        assert!(o.data_image);
        assert!(!o.text_mark);
        assert!(!o.html_tag2_text_mark);
        assert!(!o.spin);
        assert!(o.html2_markdown_attrs.is_empty());
        assert!(!o.inline_code_html);
        assert!(!o.preserve_table_cell_source);
        assert!(!o.disable_table_cell_rich);
        assert!(!o.callout);
        assert!(!o.tabs);
        assert!(!o.keep_escaped);
        assert!(!o.arbitrary_task_list_item_marker);
        assert!(!o.ensure_list_item_paragraph);
    }

    #[test]
    fn render_defaults_match_go() {
        let o = RenderOptions::new();
        assert!(!o.tabs_markdown);
        assert!(o.soft_break2_hard_break);
        assert!(!o.auto_space);
        assert!(!o.render_list_style);
        assert!(o.code_syntax_highlight);
        assert!(!o.code_syntax_highlight_detect_lang);
        assert!(!o.code_syntax_highlight_inline_style);
        assert!(!o.code_syntax_highlight_line_num);
        assert_eq!("github", o.code_syntax_highlight_style_name);
        assert!(!o.vditor_wysiwyg);
        assert!(!o.vditor_ir);
        assert!(!o.vditor_sv);
        assert!(!o.protyle_wysiwyg);
        assert!(!o.kramdown_block_ial);
        assert!(!o.kramdown_span_ial);
        assert!(!o.omit_table_cell_ial);
        assert!(!o.super_block);
        assert_eq!("", o.image_lazy_loading);
        assert!(!o.chinese_paragraph_beginning_space);
        assert!(!o.sanitize);
        assert!(!o.fix_term_typo);
        assert_eq!(116, o.terms.len());
        assert_eq!(Some(&"GitHub".to_string()), o.terms.get("github"));
        assert!(!o.toc);
        assert!(!o.heading_id);
        assert_eq!("id", o.kramdown_ial_id_render_name);
        assert!(!o.heading_anchor);
        assert_eq!("vditor-task", o.gfm_task_list_item_class);
        assert!(!o.data_task);
        assert!(o.vditor_code_block_preview);
        assert!(o.vditor_math_block_preview);
        assert!(o.vditor_html_block_preview);
        assert_eq!("", o.link_base);
        assert_eq!("", o.link_prefix);
        assert_eq!(1, o.node_index_start);
        assert!(o.protyle_contenteditable);
        assert!(!o.keep_paragraph_beginning_space);
        assert!(o.protyle_mark_net_img);
        assert!(!o.spellcheck);
        assert_eq!("", o.unordered_list_marker);
        assert!(!o.img_tag);
        assert!(!o.prevent_encode_link_space);
        assert!(o.export_normalize_task_list_marker);
    }

    #[test]
    fn task_marker_validity() {
        let mut o = ParseOptions::new();
        assert!(o.is_valid_task_list_item_marker(b' '));
        assert!(o.is_valid_task_list_item_marker(b'x'));
        assert!(o.is_valid_task_list_item_marker(b'X'));
        assert!(!o.is_valid_task_list_item_marker(b'/'));
        o.arbitrary_task_list_item_marker = true;
        assert!(o.is_valid_task_list_item_marker(b'/'));
        assert!(!o.is_valid_task_list_item_marker(b'['));
    }

    /// 与 conformance/golden/options.json 比对
    /// 快照在 `conformance/golden/options.json`
    #[test]
    fn defaults_match_go_field_by_field() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../conformance/golden/options.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读取 {} 失败：{e}", path.display()));
        let golden: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let parse = serde_json::to_value(ParseOptions::new()).unwrap();
        let render = serde_json::to_value(RenderOptions::new()).unwrap();
        compare_section("parse", &parse, &golden["parse"]);
        compare_section("render", &render, &golden["render"]);
    }

    fn compare_section(section: &str, rust: &serde_json::Value, go: &serde_json::Value) {
        let rust = rust.as_object().expect("Rust 侧应为对象");
        let go = go.as_object().expect("Go 侧应为对象");

        // Go 版 inlineCodeHTML 是未导出字段，反射无法读取；其默认值在 parse_defaults_match_go 中断言。
        const RUST_ONLY: &[&str] = &["inline_code_html"];

        let mut keys: Vec<&String> = rust.keys().chain(go.keys()).collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            if RUST_ONLY.contains(&key.as_str()) {
                continue;
            }
            match (rust.get(key), go.get(key)) {
                (Some(r), Some(g)) if key == "emoji_alias" => {
                    // Go 版 EmojiUnicodeAlias 由 map 迭代构建，37 组重复 Unicode 的胜出别名
                    // 随进程变化，因此只比对键集合与长度。
                    let mut rk: Vec<&String> = r.as_object().unwrap().keys().collect();
                    let mut gk: Vec<&String> = g.as_object().unwrap().keys().collect();
                    rk.sort();
                    gk.sort();
                    assert_eq!(rk, gk, "{section}.{key} 键集合不一致");
                }
                (Some(r), Some(g)) => {
                    assert_eq!(r, g, "{section}.{key} 默认值不一致");
                }
                (Some(_), None) => panic!("{section}.{key} 只存在于 Rust 侧"),
                (None, Some(_)) => panic!("{section}.{key} 只存在于 Go 侧"),
                (None, None) => unreachable!(),
            }
        }
    }
}
