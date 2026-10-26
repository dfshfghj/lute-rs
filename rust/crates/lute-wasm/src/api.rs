// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! 实例方法绑定：输入输出形状必须与 GopherJS 完全一致。

use js_sys::{Array, Object};
use lute_core::lute::Lute;
use wasm_bindgen::prelude::*;

use crate::{array_to_vec, map_to_object, object_to_map, LuteEngine};

#[cfg(feature = "protyle")]
const GO_NIL_PANIC: &str = "runtime error: invalid memory address or nil pointer dereference";

fn strings_to_array(v: Vec<String>) -> Array {
    let arr = Array::new();
    for s in v {
        arr.push(&JsValue::from_str(&s));
    }
    arr
}

fn opt_strings_to_array(v: Option<Vec<String>>) -> JsValue {
    match v {
        Some(v) => strings_to_array(v).into(),
        None => JsValue::NULL,
    }
}

fn bytes_to_vec(v: Vec<u8>) -> Vec<u8> {
    v
}

#[wasm_bindgen]
impl LuteEngine {
    // ---------------------------------------------------------- Markdown 输入

    // Md2HTML 分组能挂自定义渲染器，回调异常要在渲染器析构之后再抛回 JS。
    #[wasm_bindgen(js_name = Markdown)]
    pub fn markdown(&self, name: &str, markdown: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.with_renderer_errors(|| {
            Ok(bytes_to_vec(
                self.lute
                    .markdown(name, &String::from_utf8_lossy(markdown))
                    .into_bytes(),
            ))
        })
    }

    #[wasm_bindgen(js_name = MarkdownStr)]
    pub fn markdown_str(&self, name: &str, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.markdown(name, markdown)))
    }

    #[wasm_bindgen(js_name = Md2HTML)]
    pub fn md2html(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.markdown("", markdown)))
    }

    #[wasm_bindgen(js_name = Format)]
    pub fn format(&self, name: &str, markdown: &[u8]) -> Vec<u8> {
        bytes_to_vec(
            self.lute
                .format(name, &String::from_utf8_lossy(markdown))
                .into_bytes(),
        )
    }

    #[wasm_bindgen(js_name = FormatStr)]
    pub fn format_str(&self, name: &str, markdown: &str) -> String {
        self.lute.format(name, markdown)
    }

    #[wasm_bindgen(js_name = TextBundle)]
    pub fn text_bundle(&self, name: &str, markdown: &[u8], link_prefixes: &Array) -> Array {
        let (text, links) = self.lute.text_bundle(
            name,
            &String::from_utf8_lossy(markdown),
            array_to_vec(link_prefixes),
        );
        let arr = Array::new();
        arr.push(&JsValue::from_str(&text));
        arr.push(&opt_strings_to_array(links));
        arr
    }

    #[wasm_bindgen(js_name = TextBundleStr)]
    pub fn text_bundle_str(&self, name: &str, markdown: &str, link_prefixes: &Array) -> Array {
        let (text, links) = self
            .lute
            .text_bundle(name, markdown, array_to_vec(link_prefixes));
        let arr = Array::new();
        arr.push(&JsValue::from_str(&text));
        arr.push(&opt_strings_to_array(links));
        arr
    }

    #[cfg(feature = "render-json")]
    #[wasm_bindgen(js_name = RenderJSON)]
    pub fn render_json(&self, markdown: &str) -> String {
        self.lute.render_json(markdown)
    }

    #[wasm_bindgen(js_name = RenderEChartsJSON)]
    pub fn render_echarts_json(&self, markdown: &str) -> String {
        self.lute.render_echarts_json(markdown)
    }

    #[wasm_bindgen(js_name = RenderKityMinderJSON)]
    pub fn render_kityminder_json(&self, markdown: &str) -> String {
        self.lute.render_kityminder_json(markdown)
    }

    #[wasm_bindgen(js_name = ProtylePreview)]
    #[cfg(feature = "protyle")]
    pub fn protyle_preview(&self, name: &str, markdown: &[u8]) -> Vec<u8> {
        self.lute
            .protyle_preview_str(name, &String::from_utf8_lossy(markdown))
            .into_bytes()
    }

    #[wasm_bindgen(js_name = ProtylePreviewStr)]
    #[cfg(feature = "protyle")]
    pub fn protyle_preview_str(&self, name: &str, markdown: &str) -> String {
        self.lute.protyle_preview_str(name, markdown)
    }

    // -------------------------------------------------------------- HTML 输入

    #[wasm_bindgen(js_name = HTML2Md)]
    pub fn html2md(&self, dom: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.html2md(dom)))
    }

    #[wasm_bindgen(js_name = HTML2Markdown)]
    pub fn html2markdown(&self, dom: &str) -> Result<Array, JsValue> {
        self.with_renderer_errors(|| {
            Ok({
                let (md, err) = self.lute.html2markdown(dom);
                let arr = Array::new();
                arr.push(&JsValue::from_str(&md));
                arr.push(&match err {
                    Some(e) => JsValue::from_str(&e),
                    None => JsValue::NULL,
                });
                arr
            })
        })
    }

    #[wasm_bindgen(js_name = HTML2Text)]
    pub fn html2text(&self, dom: &str) -> String {
        self.lute.html2text(dom)
    }

    #[wasm_bindgen(js_name = HTML2VditorDOM)]
    pub fn html2vditor_dom(&self, html: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.html2vditor_dom(html)))
    }

    #[wasm_bindgen(js_name = HTML2VditorIRDOM)]
    pub fn html2vditor_ir_dom(&self, html: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.html2vditor_ir_dom(html)))
    }

    #[cfg(feature = "vditor-sv")]
    #[wasm_bindgen(js_name = HTML2VditorSVDOM)]
    pub fn html2vditor_sv_dom(&self, html: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.html2vditor_sv_dom(html)))
    }

    #[wasm_bindgen(js_name = HTML2BlockDOM)]
    #[cfg(feature = "protyle")]
    pub fn html2blockdom(&self, html: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.html2blockdom(html)))
    }

    // ------------------------------------------------------------ Vditor 输入

    #[wasm_bindgen(js_name = VditorDOM2HTML)]
    pub fn vditor_dom2html(&self, vhtml: &str) -> String {
        self.lute.vditor_dom2html(vhtml)
    }

    #[wasm_bindgen(js_name = VditorDOM2Md)]
    pub fn vditor_dom2md(&self, vhtml: &str) -> String {
        self.lute.vditor_dom2md(vhtml)
    }

    #[wasm_bindgen(js_name = VditorIRDOM2HTML)]
    pub fn vditor_ir_dom2html(&self, vhtml: &str) -> String {
        self.lute.vditor_ir_dom2html(vhtml)
    }

    #[wasm_bindgen(js_name = VditorIRDOM2Md)]
    pub fn vditor_ir_dom2md(&self, vhtml: &str) -> String {
        self.lute.vditor_ir_dom2md(vhtml)
    }

    #[wasm_bindgen(js_name = SpinVditorDOM)]
    pub fn spin_vditor_dom(&self, vhtml: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.spin_vditor_dom(vhtml)))
    }

    #[wasm_bindgen(js_name = SpinVditorIRDOM)]
    pub fn spin_vditor_ir_dom(&self, vhtml: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.spin_vditor_ir_dom(vhtml)))
    }

    #[cfg(feature = "vditor-sv")]
    #[wasm_bindgen(js_name = SpinVditorSVDOM)]
    pub fn spin_vditor_sv_dom(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.spin_vditor_sv_dom(markdown)))
    }

    #[wasm_bindgen(js_name = Md2VditorDOM)]
    pub fn md2vditor_dom(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.md2vditor_dom(markdown)))
    }

    #[wasm_bindgen(js_name = Md2VditorIRDOM)]
    pub fn md2vditor_ir_dom(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.md2vditor_ir_dom(markdown)))
    }

    #[cfg(feature = "vditor-sv")]
    #[wasm_bindgen(js_name = Md2VditorSVDOM)]
    pub fn md2vditor_sv_dom(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.md2vditor_sv_dom(markdown)))
    }

    // ----------------------------------------------------------- BlockDOM 输入

    #[wasm_bindgen(js_name = Md2BlockDOM)]
    #[cfg(feature = "protyle")]
    pub fn md2blockdom(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<String, JsValue> {
        self.with_renderer_errors(|| {
            self.lute
                .try_md2blockdom(markdown, reserve_empty_paragraph)
                .map_err(|e| js_sys::Error::new(&e).into())
        })
    }

    #[wasm_bindgen(js_name = Md2BlockDOMWithAutoLink)]
    #[cfg(feature = "protyle")]
    pub fn md2blockdom_with_auto_link(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<String, JsValue> {
        self.with_renderer_errors(|| {
            self.lute
                .try_md2blockdom_with_auto_link(markdown, reserve_empty_paragraph)
                .map_err(|e| js_sys::Error::new(&e).into())
        })
    }

    #[wasm_bindgen(js_name = InlineMd2BlockDOM)]
    #[cfg(feature = "protyle")]
    pub fn inline_md2blockdom(&self, markdown: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.inline_md2blockdom(markdown)))
    }

    #[wasm_bindgen(js_name = BlockDOM2Md)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2md(&self, blockdom: &str) -> String {
        self.lute.blockdom2md(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2StdMd)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2stdmd(&self, blockdom: &str) -> String {
        self.lute.blockdom2stdmd(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2HTML)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2html(&self, blockdom: &str) -> String {
        self.lute.blockdom2html(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2Text)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2text(&self, blockdom: &str) -> String {
        self.lute.blockdom2text(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2TextLen)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2textlen(&self, blockdom: &str) -> u32 {
        self.lute.blockdom2textlen(blockdom) as u32
    }

    #[wasm_bindgen(js_name = BlockDOM2Content)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2content(&self, blockdom: &str) -> String {
        self.lute.blockdom2content(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2EscapeMarkerContent)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2_escape_marker_content(&self, blockdom: &str) -> String {
        self.lute.blockdom2_escape_marker_content(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2RichHTML)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2richhtml(&self, blockdom: &str) -> String {
        self.lute.blockdom2richhtml(blockdom)
    }

    #[wasm_bindgen(js_name = BlockDOM2InlineBlockDOM)]
    #[cfg(feature = "protyle")]
    pub fn blockdom2inline_blockdom(&self, blockdom: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.blockdom2inline(blockdom)))
    }

    #[wasm_bindgen(js_name = SpinBlockDOM)]
    #[cfg(feature = "protyle")]
    pub fn spin_blockdom(&self, blockdom: &str) -> Result<String, JsValue> {
        self.with_renderer_errors(|| {
            if blockdom.is_empty() {
                return Err(JsValue::from_str(GO_NIL_PANIC));
            }
            Ok(self.lute.spin_blockdom(blockdom))
        })
    }

    // ------------------------------------------------------------ 结构变换

    #[wasm_bindgen(js_name = OL2UL)]
    #[cfg(feature = "protyle")]
    pub fn ol2ul(&self, blockdom: &str) -> String {
        self.lute.ol2ul(blockdom)
    }

    #[wasm_bindgen(js_name = UL2OL)]
    #[cfg(feature = "protyle")]
    pub fn ul2ol(&self, blockdom: &str) -> String {
        self.lute.ul2ol(blockdom)
    }

    #[wasm_bindgen(js_name = UL2TL)]
    #[cfg(feature = "protyle")]
    pub fn ul2tl(&self, blockdom: &str) -> String {
        self.lute.ul2tl(blockdom)
    }

    #[wasm_bindgen(js_name = OL2TL)]
    #[cfg(feature = "protyle")]
    pub fn ol2tl(&self, blockdom: &str) -> String {
        self.lute.ol2tl(blockdom)
    }

    #[wasm_bindgen(js_name = TL2UL)]
    #[cfg(feature = "protyle")]
    pub fn tl2ul(&self, blockdom: &str) -> String {
        self.lute.tl2ul(blockdom)
    }

    #[wasm_bindgen(js_name = TL2OL)]
    #[cfg(feature = "protyle")]
    pub fn tl2ol(&self, blockdom: &str) -> String {
        self.lute.tl2ol(blockdom)
    }

    #[wasm_bindgen(js_name = ConvertListType)]
    #[cfg(feature = "protyle")]
    pub fn convert_list_type(&self, blockdom: &str, target: &str) -> String {
        self.lute.convert_list_type(blockdom, target)
    }

    #[wasm_bindgen(js_name = CancelList)]
    #[cfg(feature = "protyle")]
    pub fn cancel_list(&self, blockdom: &str) -> String {
        self.lute.cancel_list(blockdom)
    }

    #[wasm_bindgen(js_name = CancelListRecursively)]
    #[cfg(feature = "protyle")]
    pub fn cancel_list_recursively(&self, blockdom: &str) -> String {
        self.lute.cancel_list_recursively(blockdom)
    }

    #[wasm_bindgen(js_name = CancelBlockquote)]
    #[cfg(feature = "protyle")]
    pub fn cancel_blockquote(&self, blockdom: &str) -> String {
        self.lute.cancel_blockquote(blockdom)
    }

    #[wasm_bindgen(js_name = CancelCallout)]
    #[cfg(feature = "protyle")]
    pub fn cancel_callout(&self, blockdom: &str) -> String {
        self.lute.cancel_callout(blockdom)
    }

    #[wasm_bindgen(js_name = CancelSuperBlock)]
    #[cfg(feature = "protyle")]
    pub fn cancel_superblock(&self, blockdom: &str) -> String {
        self.lute.cancel_superblock(blockdom)
    }

    #[wasm_bindgen(js_name = Blockquote2Callout)]
    #[cfg(feature = "protyle")]
    pub fn blockquote2callout(&self, blockdom: &str) -> String {
        self.lute.blockquote2callout(blockdom)
    }

    #[wasm_bindgen(js_name = Callout2Blockquote)]
    #[cfg(feature = "protyle")]
    pub fn callout2blockquote(&self, blockdom: &str) -> String {
        self.lute.callout2blockquote(blockdom)
    }

    #[wasm_bindgen(js_name = Blocks2Hs)]
    #[cfg(feature = "protyle")]
    pub fn blocks2hs(&self, blockdom: &str, level: &str) -> String {
        self.lute.blocks2hs(blockdom, level)
    }

    #[wasm_bindgen(js_name = Blocks2Ps)]
    #[cfg(feature = "protyle")]
    pub fn blocks2ps(&self, blockdom: &str) -> String {
        self.lute.blocks2ps(blockdom)
    }

    // ---------------------------------------------------------------- 工具

    #[wasm_bindgen(js_name = Space)]
    pub fn space(&self, text: &str) -> String {
        Lute::space(text)
    }

    #[wasm_bindgen(js_name = IsValidLinkDest)]
    pub fn is_valid_link_dest(&self, s: &str) -> bool {
        self.lute.is_valid_link_dest(s)
    }

    #[wasm_bindgen(js_name = GetLinkDest)]
    pub fn get_link_dest(&self, s: &str) -> String {
        self.lute.get_link_dest(s)
    }

    #[wasm_bindgen(js_name = GetEmojis)]
    pub fn get_emojis(&self) -> Object {
        map_to_object(&self.lute.get_emojis())
    }

    #[wasm_bindgen(js_name = PutEmojis)]
    pub fn put_emojis(&mut self, emojis: &Object) {
        self.lute.put_emojis(object_to_map(emojis));
    }

    #[wasm_bindgen(js_name = RemoveEmoji)]
    pub fn remove_emoji(&self, s: &str) -> String {
        self.lute.remove_emoji(s)
    }

    #[wasm_bindgen(js_name = GetTerms)]
    pub fn get_terms(&self) -> Object {
        map_to_object(&self.lute.get_terms())
    }

    #[wasm_bindgen(js_name = PutTerms)]
    pub fn put_terms(&mut self, terms: &Object) {
        self.lute.put_terms(object_to_map(terms));
    }
}
