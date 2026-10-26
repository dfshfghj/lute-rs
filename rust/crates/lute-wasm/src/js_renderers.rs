// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! SetJSRenderers：把 JS 回调桥接成 Rust 渲染钩子。

use std::collections::HashMap;
use std::rc::Rc;

use js_sys::{Array, Function, Object, Reflect};
use lute_core::ast::{Node, NodeType, WalkStatus};
use wasm_bindgen::prelude::*;

use crate::node::JsNode;
use crate::LuteEngine;

/// 合法的渲染器分组名，与 Go 版 SetJSRenderers 一致。
const GROUPS: &[&str] = &[
    "HTML2Md",
    "HTML2VditorDOM",
    "HTML2VditorIRDOM",
    "HTML2BlockDOM",
    "HTML2VditorSVDOM",
    "Md2HTML",
    "Md2VditorDOM",
    "Md2VditorIRDOM",
    "Md2BlockDOM",
    "Md2VditorSVDOM",
];

#[wasm_bindgen]
impl LuteEngine {
    /// 对应 Go 版 Lute.SetJSRenderers。
    #[wasm_bindgen(js_name = SetJSRenderers)]
    pub fn set_js_renderers(&mut self, options: JsValue) -> Result<(), JsValue> {
        let renderers =
            Reflect::get(&options, &JsValue::from_str("renderers")).unwrap_or(JsValue::UNDEFINED);
        if !renderers.is_object() {
            return Ok(());
        }
        let renderers: Object = renderers.unchecked_into();

        for key in Object::keys(&renderers).iter() {
            let group = key.as_string().unwrap_or_default();
            if !GROUPS.contains(&group.as_str()) {
                return Err(JsValue::from_str(&format!(
                    "unknown ext renderer func [{}]",
                    group
                )));
            }
            let ext = Reflect::get(&renderers, &key).unwrap_or(JsValue::UNDEFINED);
            if !ext.is_object() {
                return Err(JsValue::from_str(&format!("invalid type [{}]", group)));
            }
            let ext: Object = ext.unchecked_into();

            let funcs = self.js_renderer_funcs.entry(group).or_default();
            for name in Object::keys(&ext).iter() {
                let func_name = name.as_string().unwrap_or_default();
                let Some(rest) = func_name.strip_prefix("render") else {
                    continue;
                };
                let Some(typ) = NodeType::from_str(&format!("Node{}", rest)) else {
                    continue;
                };
                let f = Reflect::get(&ext, &name).unwrap_or(JsValue::UNDEFINED);
                if let Ok(f) = f.dyn_into::<Function>() {
                    funcs.insert(typ, f);
                }
            }
        }

        // Go 为每组保存逐节点回调，重复调用只覆盖同组同节点的注册。
        let mut map: HashMap<String, lute_core::render::ExtRendererFunc> = HashMap::new();
        for (group, funcs) in &self.js_renderer_funcs {
            let funcs = Rc::new(funcs.clone());
            let errors = Rc::clone(&self.renderer_errors);
            let cb: lute_core::render::ExtRendererFunc = Rc::new(
                move |n: Node, entering: bool| -> Option<(String, WalkStatus)> {
                    if errors.borrow().last().is_some_and(Option::is_some) {
                        return Some((String::new(), WalkStatus::WalkStop));
                    }
                    let f = funcs.get(&n.typ())?;
                    let js_node: JsValue = JsNode::retained(n).into();
                    let ret = match f.call2(&JsValue::NULL, &js_node, &JsValue::from_bool(entering))
                    {
                        Ok(ret) => ret,
                        Err(e) => {
                            *errors.borrow_mut().last_mut().expect("render error scope") = Some(e);
                            return Some((String::new(), WalkStatus::WalkStop));
                        }
                    };
                    let arr: Array = match ret.dyn_into() {
                        Ok(arr) => arr,
                        Err(_) => return None,
                    };
                    let s = arr.get(0).as_string().unwrap_or_default();
                    let st = arr.get(1).as_f64().unwrap_or(2.0) as i32;
                    let status = match st {
                        0 => WalkStatus::WalkStop,
                        1 => WalkStatus::WalkSkipChildren,
                        _ => WalkStatus::WalkContinue,
                    };
                    Some((s, status))
                },
            );
            map.insert(group.clone(), cb);
        }

        self.lute.set_js_renderers(map);
        Ok(())
    }
}
