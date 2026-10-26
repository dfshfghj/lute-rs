// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! 语法树相关 API：返回 Tree / Node 包装对象的方法。

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "protyle")]
use js_sys::Array;
use lute_core::ast::Tree;
use wasm_bindgen::prelude::*;

use crate::node::{JsNode, JsTree};
use crate::LuteEngine;

fn wrap_tree(tree: Tree) -> JsTree {
    JsTree {
        inner: Rc::new(RefCell::new(tree)),
    }
}

#[wasm_bindgen]
impl LuteEngine {
    #[wasm_bindgen(js_name = HTML2Tree)]
    pub fn html2tree(&self, html: &str) -> JsTree {
        wrap_tree(self.lute.html2tree(html))
    }

    /// 对应 Go 版 Lute.HTMLNode2Tree。JS 侧没有 html.Node 包装，这里接受 HTML 字符串。
    #[wasm_bindgen(js_name = HTMLNode2Tree)]
    pub fn html_node2tree(&self, node: &str) -> JsTree {
        wrap_tree(self.lute.html2tree(node))
    }

    #[cfg(feature = "protyle")]
    #[wasm_bindgen(js_name = BlockDOM2Tree)]
    pub fn blockdom2tree(&self, blockdom: &str) -> JsTree {
        wrap_tree(self.lute.blockdom2tree(blockdom))
    }

    #[cfg(feature = "protyle")]
    #[wasm_bindgen(js_name = Md2BlockDOMTree)]
    pub fn md2blockdomtree(
        &self,
        markdown: &str,
        reserve_empty_paragraph: bool,
    ) -> Result<Array, JsValue> {
        self.with_renderer_errors(|| {
            let (html, tree) = self
                .lute
                .try_md2blockdomtree(markdown, reserve_empty_paragraph)
                .map_err(|e| JsValue::from(js_sys::Error::new(&e)))?;
            let arr = Array::new();
            arr.push(&JsValue::from_str(&html));
            arr.push(&wrap_tree(tree).into());
            Ok(arr)
        })
    }

    #[wasm_bindgen(js_name = Tree2HTML)]
    pub fn tree2html(&self, tree: &JsTree, _options: JsValue, _parse_options: JsValue) -> String {
        self.lute.tree2html(&tree.inner.borrow())
    }

    #[cfg(feature = "protyle")]
    #[wasm_bindgen(js_name = Tree2BlockDOM)]
    pub fn tree2blockdom(
        &self,
        tree: &JsTree,
        _options: JsValue,
        _parse_options: JsValue,
    ) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.tree2blockdom(&tree.inner.borrow())))
    }

    #[cfg(feature = "protyle")]
    #[wasm_bindgen(js_name = RenderNodeBlockDOM)]
    pub fn render_node_blockdom(&self, node: &JsNode) -> Result<String, JsValue> {
        self.with_renderer_errors(|| Ok(self.lute.render_node_blockdom(&node.node)))
    }

    #[wasm_bindgen(js_name = MergeSameSpan)]
    pub fn merge_same_span(&self, node: &JsNode) {
        self.lute.merge_same_span(&node.node);
    }

    #[wasm_bindgen(js_name = MergeSameTextMark)]
    pub fn merge_same_text_mark(&self, node: &JsNode) {
        self.lute.merge_same_text_mark(&node.node);
    }
}

#[wasm_bindgen(js_name = GetHeadingID)]
pub fn get_heading_id(node: &JsNode) -> String {
    lute_core::lute::Lute::heading_id(&node.node)
}

#[wasm_bindgen(js_name = GetHeadingIDRaw)]
pub fn get_heading_id_raw(node: &JsNode) -> String {
    lute_core::lute::Lute::heading_id_raw(&node.node)
}
