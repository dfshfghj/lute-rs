// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! 节点与语法树的 JS 包装，对应 GopherJS 下 *ast.Node / *parse.Tree 的包装对象。

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use lute_core::ast::{Node, NodeType, Tree};
use wasm_bindgen::prelude::*;

/// JS 语法树所有权；逃逸的 JsNode 由 Node 自身保活，不依赖 JsTree 的生命周期。
pub(crate) type SharedTree = Rc<RefCell<Tree>>;

#[wasm_bindgen]
pub struct JsTree {
    pub(crate) inner: SharedTree,
}

#[wasm_bindgen]
impl JsTree {
    #[wasm_bindgen(js_name = Root)]
    pub fn root(&self) -> JsNode {
        let node = self.inner.borrow().root();
        JsNode::retained(node)
    }

    #[wasm_bindgen(getter, js_name = Name)]
    pub fn name(&self) -> String {
        self.inner.borrow().name.clone()
    }

    #[wasm_bindgen(getter, js_name = ID)]
    pub fn id(&self) -> String {
        self.inner.borrow().id.clone()
    }
}

#[wasm_bindgen]
pub struct JsNode {
    // Node 自身保活整个关联 arena，包括父/兄弟节点和跨树插入的节点。
    pub(crate) node: Node,
}

impl JsNode {
    pub(crate) fn retained(node: Node) -> JsNode {
        JsNode { node }
    }
}

#[wasm_bindgen]
impl JsNode {
    #[wasm_bindgen(js_name = Text)]
    pub fn text(&self) -> String {
        self.node.text()
    }

    #[wasm_bindgen(js_name = TokensStr)]
    pub fn tokens_str(&self) -> String {
        self.node.tokens_str()
    }

    #[wasm_bindgen(js_name = TextLen)]
    pub fn text_len(&self) -> u32 {
        self.node.text_len() as u32
    }

    #[wasm_bindgen(js_name = Content)]
    pub fn content(&self) -> String {
        self.node.content()
    }

    #[wasm_bindgen(js_name = ChildByType)]
    pub fn child_by_type(&self, typ: &str) -> JsValue {
        match node_type(typ).and_then(|t| self.node.child_by_type(t)) {
            Some(n) => self.wrap_child(n).into(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen(js_name = ChildrenByType)]
    pub fn children_by_type(&self, typ: &str) -> Array {
        let arr = Array::new();
        if let Some(t) = node_type(typ) {
            for c in self.node.children_by_type(t) {
                arr.push(&self.wrap_child(c).into());
            }
        }
        arr
    }

    #[wasm_bindgen(js_name = FirstDeepestChild)]
    pub fn first_deepest_child(&self) -> JsNode {
        let mut n = self.node.clone();
        while let Some(c) = n.first_child() {
            n = c;
        }
        self.wrap_child(n)
    }

    #[wasm_bindgen(js_name = LastDeepestChild)]
    pub fn last_deepest_child(&self) -> JsNode {
        let mut n = self.node.clone();
        while let Some(c) = n.last_child() {
            n = c;
        }
        self.wrap_child(n)
    }

    #[wasm_bindgen(js_name = NextNodeText)]
    pub fn next_node_text(&self) -> String {
        self.node.next().map(|n| n.text()).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = PreviousNodeText)]
    pub fn previous_node_text(&self) -> String {
        let mut prev = self.node.previous();
        if let Some(p) = &prev {
            if NodeType::NodeKramdownSpanIAL == p.typ() {
                prev = p.previous();
            }
        }
        prev.map(|n| n.text()).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = AppendChild)]
    pub fn append_child(&self, child: &JsNode) {
        self.node.append_child(&child.node);
    }

    #[wasm_bindgen(js_name = InsertBefore)]
    pub fn insert_before(&self, sibling: &JsNode) {
        self.node.insert_before(&sibling.node);
    }

    #[wasm_bindgen(js_name = InsertAfter)]
    pub fn insert_after(&self, sibling: &JsNode) {
        self.node.insert_after(&sibling.node);
    }

    #[wasm_bindgen(js_name = Unlink)]
    pub fn unlink(&self) {
        self.node.unlink();
    }

    #[wasm_bindgen(js_name = SetIALAttr)]
    pub fn set_ial_attr(&self, name: &str, value: &str) {
        self.node.set_ial_attr(name, value);
    }

    #[wasm_bindgen(js_name = IALAttr)]
    pub fn ial_attr(&self, name: &str) -> String {
        self.node.ial_attr(name)
    }

    #[wasm_bindgen(js_name = IsBlock)]
    pub fn is_block(&self) -> bool {
        self.node.is_block()
    }

    #[wasm_bindgen(js_name = ParentIs)]
    pub fn parent_is(&self, typ: &str) -> bool {
        match node_type(typ) {
            Some(t) => self.node.parent_is(&[t]),
            None => false,
        }
    }

    /// 对应 Go 版包装对象的 __internal_object__：字段访问走这里。
    #[wasm_bindgen(getter, js_name = __internal_object__)]
    pub fn internal_object(&self) -> JsValue {
        let obj = Object::new();
        let n = &self.node;
        let typ = n.typ().as_str();
        set(&obj, "Type", &JsValue::from_str(typ));
        set(&obj, "ID", &JsValue::from_str(&n.id()));
        set(&obj, "Tokens", &JsValue::from_str(&n.tokens_str()));
        set(&obj, "Parent", &self.child_value(n.parent()));
        set(&obj, "FirstChild", &self.child_value(n.first_child()));
        set(&obj, "LastChild", &self.child_value(n.last_child()));
        set(&obj, "Previous", &self.child_value(n.previous()));
        set(&obj, "Next", &self.child_value(n.next()));

        let ial = Array::new();
        for (k, v) in n.ial_raw() {
            let kv = Array::new();
            kv.push(&JsValue::from_str(&k));
            kv.push(&JsValue::from_str(&v));
            ial.push(&kv);
        }
        set(&obj, "KramdownIAL", &ial.into());

        // 小写别名，便于不熟悉 Go 字段名的调用方。
        set(&obj, "type", &JsValue::from_str(typ));
        set(&obj, "typ", &JsValue::from_str(typ));
        set(&obj, "id", &JsValue::from_str(&n.id()));
        set(&obj, "tokens", &JsValue::from_str(&n.tokens_str()));
        set(&obj, "parent", &self.child_value(n.parent()));
        set(&obj, "firstChild", &self.child_value(n.first_child()));
        set(&obj, "lastChild", &self.child_value(n.last_child()));
        set(&obj, "previous", &self.child_value(n.previous()));
        set(&obj, "next", &self.child_value(n.next()));
        obj.into()
    }
}

impl JsNode {
    fn wrap_child(&self, node: Node) -> JsNode {
        JsNode::retained(node)
    }

    fn child_value(&self, child: Option<Node>) -> JsValue {
        match child {
            Some(c) => self.wrap_child(c).into(),
            None => JsValue::NULL,
        }
    }
}

fn set(obj: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(obj, &JsValue::from_str(key), value);
}

pub(crate) fn node_type(s: &str) -> Option<NodeType> {
    NodeType::from_str(s)
}
