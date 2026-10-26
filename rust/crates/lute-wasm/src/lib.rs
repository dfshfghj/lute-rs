// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! wasm 绑定层：对外暴露与 GopherJS 产物完全一致的 Lute 对象。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lute_core::lute::Lute;
use wasm_bindgen::prelude::*;

mod api;
mod js_renderers;
mod node;
mod tree_api;
mod util;

pub use node::{JsNode, JsTree};
pub use tree_api::{get_heading_id, get_heading_id_raw};

/// 裁剪构建里被关掉的能力，对应的 setter 必须显式报错而不是静默失效。
///
/// 静默失效是裁剪最危险的失败模式：宿主调了 SetAutoSpace(true)，产物却按
/// false 渲染，输出与全量构建不同，而全量测试永远测不到（全量里能力是开的）。
fn require_capability(enabled: bool, name: &str) -> Result<(), JsValue> {
    if enabled {
        return Ok(());
    }
    Err(js_sys::Error::new(&format!(
        "lute: this build was compiled without the `{name}` capability"
    ))
    .into())
}

/// 对应 Go 版 lute.Lute 的 JS 包装。
#[wasm_bindgen]
pub struct LuteEngine {
    lute: Lute,
    // One error slot per active render supports synchronous JS reentry.
    renderer_errors: Rc<RefCell<Vec<Option<JsValue>>>>,
    js_renderer_funcs: HashMap<String, HashMap<lute_core::ast::NodeType, js_sys::Function>>,
}

impl LuteEngine {
    /// Callback exceptions cross the ABI only after renderers and borrows drop.
    /// Calling throw_val inside Rust rendering would bypass their destructors.
    fn with_renderer_errors<T>(
        &self,
        render: impl FnOnce() -> Result<T, JsValue>,
    ) -> Result<T, JsValue> {
        self.renderer_errors.borrow_mut().push(None);
        let result = render();
        match self.renderer_errors.borrow_mut().pop().flatten() {
            Some(error) => Err(error),
            None => result,
        }
    }
}

impl Default for LuteEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl LuteEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LuteEngine {
        LuteEngine {
            lute: Lute::new(),
            renderer_errors: Rc::new(RefCell::new(Vec::new())),
            js_renderer_funcs: HashMap::new(),
        }
    }

    // ---------------------------------------------------------------- 选项

    #[wasm_bindgen(js_name = SetGFMTable)]
    pub fn set_gfm_table(&mut self, b: bool) {
        self.lute.parse_options.gfm_table = b;
    }

    #[wasm_bindgen(js_name = SetGFMTaskListItem)]
    pub fn set_gfm_task_list_item(&mut self, b: bool) {
        self.lute.parse_options.gfm_task_list_item = b;
    }

    #[wasm_bindgen(js_name = SetArbitraryTaskListItemMarker)]
    pub fn set_arbitrary_task_list_item_marker(&mut self, b: bool) {
        self.lute.parse_options.arbitrary_task_list_item_marker = b;
    }

    #[wasm_bindgen(js_name = SetGFMTaskListItemClass)]
    pub fn set_gfm_task_list_item_class(&mut self, class: &str) {
        self.lute.render_options.gfm_task_list_item_class = class.to_string();
    }

    #[wasm_bindgen(js_name = SetDataTask)]
    pub fn set_data_task(&mut self, b: bool) {
        self.lute.render_options.data_task = b;
    }

    #[wasm_bindgen(js_name = SetExportNormalizeTaskListMarker)]
    pub fn set_export_normalize_task_list_marker(&mut self, b: bool) {
        self.lute.render_options.export_normalize_task_list_marker = b;
    }

    #[wasm_bindgen(js_name = SetGFMStrikethrough)]
    pub fn set_gfm_strikethrough(&mut self, b: bool) {
        self.lute.parse_options.gfm_strikethrough = b;
    }

    #[wasm_bindgen(js_name = SetGFMStrikethrough1)]
    pub fn set_gfm_strikethrough1(&mut self, b: bool) {
        self.lute.parse_options.gfm_strikethrough1 = b;
    }

    #[wasm_bindgen(js_name = SetFullWidthStrikethrough)]
    pub fn set_full_width_strikethrough(&mut self, b: bool) {
        self.lute.parse_options.full_width_strikethrough = b;
    }

    #[wasm_bindgen(js_name = SetGFMAutoLink)]
    pub fn set_gfm_auto_link(&mut self, b: bool) {
        self.lute.parse_options.gfm_auto_link = b;
    }

    #[wasm_bindgen(js_name = SetSoftBreak2HardBreak)]
    pub fn set_soft_break2_hard_break(&mut self, b: bool) {
        self.lute.render_options.soft_break2_hard_break = b;
    }

    #[wasm_bindgen(js_name = SetCodeSyntaxHighlight)]
    pub fn set_code_syntax_highlight(&mut self, b: bool) {
        self.lute.render_options.code_syntax_highlight = b;
    }

    #[wasm_bindgen(js_name = SetCodeSyntaxHighlightDetectLang)]
    pub fn set_code_syntax_highlight_detect_lang(&mut self, b: bool) {
        self.lute.render_options.code_syntax_highlight_detect_lang = b;
    }

    #[wasm_bindgen(js_name = SetCodeSyntaxHighlightInlineStyle)]
    pub fn set_code_syntax_highlight_inline_style(&mut self, b: bool) {
        self.lute.render_options.code_syntax_highlight_inline_style = b;
    }

    #[wasm_bindgen(js_name = SetCodeSyntaxHighlightLineNum)]
    pub fn set_code_syntax_highlight_line_num(&mut self, b: bool) {
        self.lute.render_options.code_syntax_highlight_line_num = b;
    }

    #[wasm_bindgen(js_name = SetCodeSyntaxHighlightStyleName)]
    pub fn set_code_syntax_highlight_style_name(&mut self, name: &str) {
        self.lute.render_options.code_syntax_highlight_style_name = name.to_string();
    }

    #[wasm_bindgen(js_name = SetFootnotes)]
    pub fn set_footnotes(&mut self, b: bool) {
        self.lute.parse_options.footnotes = b;
    }

    #[wasm_bindgen(js_name = SetToC)]
    pub fn set_toc(&mut self, b: bool) {
        self.lute.parse_options.toc = b;
        self.lute.render_options.toc = b;
    }

    #[wasm_bindgen(js_name = SetHeadingID)]
    pub fn set_heading_id(&mut self, b: bool) {
        self.lute.parse_options.heading_id = b;
        self.lute.render_options.heading_id = b;
    }

    #[wasm_bindgen(js_name = SetAutoSpace)]
    pub fn set_auto_space(&mut self, b: bool) -> Result<(), JsValue> {
        require_capability(lute_core::CAP_AUTO_SPACE, "auto-space")?;
        self.lute.render_options.auto_space = b;
        Ok(())
    }

    #[wasm_bindgen(js_name = SetFixTermTypo)]
    pub fn set_fix_term_typo(&mut self, b: bool) {
        self.lute.render_options.fix_term_typo = b;
    }

    #[wasm_bindgen(js_name = SetEmoji)]
    pub fn set_emoji(&mut self, b: bool) {
        self.lute.parse_options.emoji = b;
    }

    #[wasm_bindgen(js_name = SetEmojis)]
    pub fn set_emojis(&mut self, emojis: &js_sys::Object) {
        self.lute.set_emojis(object_to_map(emojis));
    }

    #[wasm_bindgen(js_name = SetEmojiSite)]
    pub fn set_emoji_site(&mut self, site: &str) {
        self.lute.parse_options.emoji_site = site.to_string();
    }

    #[wasm_bindgen(js_name = SetHeadingAnchor)]
    pub fn set_heading_anchor(&mut self, b: bool) {
        self.lute.render_options.heading_anchor = b;
    }

    #[wasm_bindgen(js_name = SetTerms)]
    pub fn set_terms(&mut self, terms: &js_sys::Object) {
        self.lute.set_terms(object_to_map(terms));
    }

    #[wasm_bindgen(js_name = SetVditorWYSIWYG)]
    pub fn set_vditor_wysiwyg(&mut self, b: bool) {
        self.lute.parse_options.vditor_wysiwyg = b;
        self.lute.render_options.vditor_wysiwyg = b;
    }

    #[wasm_bindgen(js_name = SetProtyleWYSIWYG)]
    pub fn set_protyle_wysiwyg(&mut self, b: bool) -> Result<(), JsValue> {
        require_capability(lute_core::PROTYLE, "protyle")?;
        self.lute.parse_options.protyle_wysiwyg = b;
        self.lute.render_options.protyle_wysiwyg = b;
        Ok(())
    }

    #[wasm_bindgen(js_name = SetVditorIR)]
    pub fn set_vditor_ir(&mut self, b: bool) {
        self.lute.parse_options.vditor_ir = b;
        self.lute.render_options.vditor_ir = b;
    }

    #[wasm_bindgen(js_name = SetVditorSV)]
    pub fn set_vditor_sv(&mut self, b: bool) {
        self.lute.parse_options.vditor_sv = b;
        self.lute.render_options.vditor_sv = b;
    }

    #[wasm_bindgen(js_name = SetInlineMath)]
    pub fn set_inline_math(&mut self, b: bool) {
        self.lute.parse_options.inline_math = b;
    }

    #[wasm_bindgen(js_name = SetInlineMathAllowDigitAfterOpenMarker)]
    pub fn set_inline_math_allow_digit_after_open_marker(&mut self, b: bool) {
        self.lute
            .parse_options
            .inline_math_allow_digit_after_open_marker = b;
    }

    #[wasm_bindgen(js_name = SetLinkPrefix)]
    pub fn set_link_prefix(&mut self, prefix: &str) {
        self.lute.render_options.link_prefix = prefix.to_string();
    }

    #[wasm_bindgen(js_name = SetLinkBase)]
    pub fn set_link_base(&mut self, base: &str) {
        self.lute.render_options.link_base = base.to_string();
    }

    #[wasm_bindgen(js_name = GetLinkBase)]
    pub fn get_link_base(&self) -> String {
        self.lute.get_link_base()
    }

    #[wasm_bindgen(js_name = SetVditorCodeBlockPreview)]
    pub fn set_vditor_code_block_preview(&mut self, b: bool) {
        self.lute.render_options.vditor_code_block_preview = b;
    }

    #[wasm_bindgen(js_name = SetVditorMathBlockPreview)]
    pub fn set_vditor_math_block_preview(&mut self, b: bool) {
        self.lute.render_options.vditor_math_block_preview = b;
    }

    #[wasm_bindgen(js_name = SetVditorHTMLBlockPreview)]
    pub fn set_vditor_html_block_preview(&mut self, b: bool) {
        self.lute.render_options.vditor_html_block_preview = b;
    }

    #[wasm_bindgen(js_name = SetRenderListStyle)]
    pub fn set_render_list_style(&mut self, b: bool) {
        self.lute.render_options.render_list_style = b;
    }

    #[wasm_bindgen(js_name = SetSanitize)]
    pub fn set_sanitize(&mut self, b: bool) {
        self.lute.render_options.sanitize = b;
    }

    #[wasm_bindgen(js_name = SetImageLazyLoading)]
    pub fn set_image_lazy_loading(&mut self, data_src: &str) {
        self.lute.render_options.image_lazy_loading = data_src.to_string();
    }

    #[wasm_bindgen(js_name = SetChineseParagraphBeginningSpace)]
    pub fn set_chinese_paragraph_beginning_space(&mut self, b: bool) {
        self.lute.render_options.chinese_paragraph_beginning_space = b;
    }

    #[wasm_bindgen(js_name = SetYamlFrontMatter)]
    pub fn set_yaml_front_matter(&mut self, b: bool) {
        self.lute.parse_options.yaml_front_matter = b;
    }

    #[wasm_bindgen(js_name = SetSetext)]
    pub fn set_setext(&mut self, b: bool) {
        self.lute.parse_options.setext = b;
    }

    #[wasm_bindgen(js_name = SetBlockRef)]
    pub fn set_block_ref(&mut self, b: bool) {
        self.lute.parse_options.block_ref = b;
    }

    #[wasm_bindgen(js_name = SetFileAnnotationRef)]
    pub fn set_file_annotation_ref(&mut self, b: bool) {
        self.lute.parse_options.file_annotation_ref = b;
    }

    #[wasm_bindgen(js_name = SetMark)]
    pub fn set_mark(&mut self, b: bool) {
        self.lute.parse_options.mark = b;
    }

    #[wasm_bindgen(js_name = SetKramdownIAL)]
    pub fn set_kramdown_ial(&mut self, b: bool) {
        self.lute.parse_options.kramdown_block_ial = b;
        self.lute.parse_options.kramdown_span_ial = b;
        self.lute.render_options.kramdown_block_ial = b;
        self.lute.render_options.kramdown_span_ial = b;
    }

    #[wasm_bindgen(js_name = SetKramdownBlockIAL)]
    pub fn set_kramdown_block_ial(&mut self, b: bool) -> Result<(), JsValue> {
        require_capability(lute_core::CAP_KRAMDOWN_BLOCK_IAL, "kramdown-ial")?;
        self.lute.parse_options.kramdown_block_ial = b;
        self.lute.render_options.kramdown_block_ial = b;
        Ok(())
    }

    #[wasm_bindgen(js_name = SetKramdownSpanIAL)]
    pub fn set_kramdown_span_ial(&mut self, b: bool) {
        self.lute.parse_options.kramdown_span_ial = b;
        self.lute.render_options.kramdown_span_ial = b;
    }

    #[wasm_bindgen(js_name = SetKramdownIALIDRenderName)]
    pub fn set_kramdown_ial_id_render_name(&mut self, name: &str) {
        self.lute.render_options.kramdown_ial_id_render_name = name.to_string();
    }

    #[wasm_bindgen(js_name = SetTag)]
    pub fn set_tag(&mut self, b: bool) {
        self.lute.parse_options.tag = b;
    }

    #[wasm_bindgen(js_name = SetImgPathAllowSpace)]
    pub fn set_img_path_allow_space(&mut self, b: bool) {
        self.lute.parse_options.img_path_allow_space = b;
    }

    #[wasm_bindgen(js_name = SetSuperBlock)]
    pub fn set_super_block(&mut self, b: bool) {
        self.lute.parse_options.super_block = b;
        self.lute.render_options.super_block = b;
    }

    #[wasm_bindgen(js_name = SetCustomBlock)]
    pub fn set_custom_block(&mut self, b: bool) {
        self.lute.parse_options.custom_block = b;
    }

    #[wasm_bindgen(js_name = SetSup)]
    pub fn set_sup(&mut self, b: bool) {
        self.lute.parse_options.sup = b;
    }

    #[wasm_bindgen(js_name = SetSub)]
    pub fn set_sub(&mut self, b: bool) {
        self.lute.parse_options.sub = b;
    }

    #[wasm_bindgen(js_name = SetInlineAsterisk)]
    pub fn set_inline_asterisk(&mut self, b: bool) {
        self.lute.parse_options.inline_asterisk = b;
    }

    #[wasm_bindgen(js_name = SetInlineUnderscore)]
    pub fn set_inline_underscore(&mut self, b: bool) {
        self.lute.parse_options.inline_underscore = b;
    }

    #[wasm_bindgen(js_name = SetGitConflict)]
    pub fn set_git_conflict(&mut self, b: bool) {
        self.lute.parse_options.git_conflict = b;
    }

    #[wasm_bindgen(js_name = SetLinkRef)]
    pub fn set_link_ref(&mut self, b: bool) {
        self.lute.parse_options.link_ref = b;
    }

    #[wasm_bindgen(js_name = SetIndentCodeBlock)]
    pub fn set_indent_code_block(&mut self, b: bool) {
        self.lute.parse_options.indent_code_block = b;
    }

    #[wasm_bindgen(js_name = SetDataImage)]
    pub fn set_data_image(&mut self, b: bool) {
        self.lute.parse_options.data_image = b;
    }

    #[wasm_bindgen(js_name = SetTextMark)]
    pub fn set_text_mark(&mut self, b: bool) {
        self.lute.parse_options.text_mark = b;
    }

    #[wasm_bindgen(js_name = SetSpin)]
    pub fn set_spin(&mut self, b: bool) {
        self.lute.parse_options.spin = b;
    }

    #[wasm_bindgen(js_name = SetHTML2MarkdownAttrs)]
    pub fn set_html2_markdown_attrs(&mut self, attrs: &js_sys::Array) {
        self.lute.parse_options.html2_markdown_attrs = array_to_vec(attrs);
    }

    #[wasm_bindgen(js_name = SetHTMLTag2TextMark)]
    pub fn set_html_tag2_text_mark(&mut self, b: bool) {
        self.lute.parse_options.html_tag2_text_mark = b;
    }

    #[wasm_bindgen(js_name = SetParagraphBeginningSpace)]
    pub fn set_paragraph_beginning_space(&mut self, b: bool) {
        self.lute.parse_options.paragraph_beginning_space = b;
        self.lute.render_options.keep_paragraph_beginning_space = b;
    }

    #[wasm_bindgen(js_name = SetProtyleMarkNetImg)]
    pub fn set_protyle_mark_net_img(&mut self, b: bool) {
        self.lute.render_options.protyle_mark_net_img = b;
    }

    #[wasm_bindgen(js_name = SetSpellcheck)]
    pub fn set_spellcheck(&mut self, b: bool) {
        self.lute.render_options.spellcheck = b;
    }

    #[wasm_bindgen(js_name = SetUnorderedListMarker)]
    pub fn set_unordered_list_marker(&mut self, marker: &str) {
        self.lute.render_options.unordered_list_marker = marker.to_string();
    }

    #[wasm_bindgen(js_name = SetImgTag)]
    pub fn set_img_tag(&mut self, b: bool) {
        self.lute.render_options.img_tag = b;
    }

    #[wasm_bindgen(js_name = SetPreventEncodeLinkSpace)]
    pub fn set_prevent_encode_link_space(&mut self, b: bool) {
        self.lute.render_options.prevent_encode_link_space = b;
    }

    #[wasm_bindgen(js_name = SetCallout)]
    pub fn set_callout(&mut self, b: bool) {
        self.lute.parse_options.callout = b;
    }

    #[wasm_bindgen(js_name = SetTabs)]
    pub fn set_tabs(&mut self, b: bool) {
        self.lute.parse_options.tabs = b;
    }

    #[wasm_bindgen(js_name = SetEnsureListItemParagraph)]
    pub fn set_ensure_list_item_paragraph(&mut self, b: bool) {
        self.lute.parse_options.ensure_list_item_paragraph = b;
    }
}

// ------------------------------------------------------------------ 全局函数

#[wasm_bindgen(js_name = luteVersion)]
pub fn lute_version() -> String {
    lute_core::VERSION.to_string()
}

#[wasm_bindgen(js_name = NewNodeID)]
pub fn new_node_id() -> String {
    lute_core::ast::new_node_id()
}

#[wasm_bindgen(js_name = EscapeHTMLStr)]
pub fn escape_html_str(s: &str) -> String {
    lute_core::html::escape_html_str(s)
}

#[wasm_bindgen(js_name = UnEscapeHTMLStr)]
pub fn un_escape_html_str(s: &str) -> String {
    lute_core::html::unescape_html_str(s)
}

#[wasm_bindgen(js_name = Sanitize)]
pub fn sanitize(s: &str) -> String {
    Lute::sanitize(s)
}

#[wasm_bindgen(js_name = EChartsMindmapStr)]
pub fn echarts_mindmap_str(s: &str) -> String {
    Lute::echarts_mindmap_str(s)
}

#[wasm_bindgen(js_name = BlockDOM2ContentGlobal)]
#[cfg(feature = "protyle")]
pub fn blockdom2content(dom: &str) -> String {
    Lute::blockdom2content_global(dom)
}

pub fn object_to_map(obj: &js_sys::Object) -> HashMap<String, String> {
    let mut ret = HashMap::new();
    for key in js_sys::Object::keys(obj).iter() {
        let k = key.as_string().unwrap_or_default();
        let v = js_sys::Reflect::get(obj, &key)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        ret.insert(k, v);
    }
    ret
}

pub fn array_to_vec(arr: &js_sys::Array) -> Vec<String> {
    arr.iter()
        .filter_map(|v| v.as_string())
        .collect::<Vec<String>>()
}

pub fn map_to_object(map: &HashMap<String, String>) -> js_sys::Object {
    let obj = js_sys::Object::new();
    for (k, v) in map {
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(k), &JsValue::from_str(v));
    }
    obj
}
