// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 链接引用定义与脚注定义的惰性索引。

use crate::ast::{NodeType, RawNode as Node, WalkStatus};

#[cfg(not(any(target_arch = "wasm32", feature = "javascript")))]
use super::case_fold::FOLD;
use super::case_fold::SIMPLE_FOLD_MIN;
use super::InlineParser;

/// 保存定义的文档顺序，用于跨大小写索引选择首个匹配。
#[derive(Clone)]
pub(super) struct LinkRefDef {
    pub order: usize,
    pub link: Node,
}

/// 用简单大小写折叠环中最小的字符建立与 `bytes.EqualFold` 一致的键。
pub(super) fn link_ref_fold_key(tokens: &[u8]) -> String {
    let s = String::from_utf8_lossy(tokens);
    let mut key = String::with_capacity(s.len());
    for r in s.chars() {
        if r.is_ascii_lowercase() {
            key.push((r as u8 - b'a' + b'A') as char);
        } else if '' < r {
            key.push(simple_fold_min(r));
        } else {
            key.push(r);
        }
    }
    key
}

fn simple_fold_min(c: char) -> char {
    let cp = c as u32;
    match SIMPLE_FOLD_MIN.binary_search_by_key(&cp, |&(k, _)| k) {
        Ok(i) => char::from_u32(SIMPLE_FOLD_MIN[i].1).unwrap_or(c),
        Err(_) => c,
    }
}

/// 对应 Go 版 `cases.Fold` 的完整大小写折叠。
#[cfg(not(any(target_arch = "wasm32", feature = "javascript")))]
pub(super) fn fold_bytes(tokens: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(tokens);
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        match FOLD.binary_search_by_key(&cp, |&(k, _)| k) {
            Ok(i) => out.push_str(FOLD[i].1),
            Err(_) => out.push(c),
        }
    }
    out.into_bytes()
}

/// Go's javascript build returns the original bytes before `bytes.EqualFold`.
#[cfg(any(target_arch = "wasm32", feature = "javascript"))]
pub(super) fn fold_bytes(tokens: &[u8]) -> Vec<u8> {
    tokens.to_vec()
}

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.indexLinkRefDefs。
    fn index_link_ref_defs(&mut self) {
        if self.link_ref_defs_indexed {
            return;
        }
        self.link_ref_defs_indexed = true;

        let root = self.tree.root_raw();
        let mut defs = Vec::new();
        crate::ast::walk_raw(root, &mut |n, entering| {
            if entering && NodeType::NodeLinkRefDef == n.typ() {
                defs.push(n);
            }
            WalkStatus::WalkContinue
        });

        let mut order = 0;
        for n in defs {
            let Some(link) = n.first_child() else {
                continue;
            };
            let def = LinkRefDef { order, link };
            let key = link_ref_fold_key(n.tokens());
            self.link_ref_defs.entry(key).or_insert_with(|| def.clone());
            let key = link_ref_fold_key(&fold_bytes(n.tokens()));
            self.link_ref_defs_folded
                .entry(key)
                .or_insert_with(|| def.clone());
            order += 1;
        }
    }

    /// 对应 Go 版 Tree.indexFootnotesDefs。
    fn index_footnotes_defs(&mut self) {
        if self.footnotes_defs_indexed {
            return;
        }
        self.footnotes_defs_indexed = true;

        let root = self.tree.root_raw();
        let mut defs = Vec::new();
        crate::ast::walk_raw(root, &mut |n, entering| {
            if entering && NodeType::NodeFootnotesDef == n.typ() {
                defs.push(n);
            }
            WalkStatus::WalkContinue
        });
        self.footnotes_defs = defs;
    }

    /// 对应 Go 版 Tree.FindLinkRefDefLink。
    pub(super) fn find_link_ref_def_link(&mut self, label: &[u8]) -> Option<Node> {
        if !self.options.link_ref {
            return None;
        }

        let label = if self.editor_mode() {
            replace_bytes(label, crate::editor::CARET.as_bytes(), b"")
        } else {
            label.to_vec()
        };

        self.index_link_ref_defs();
        if self.link_ref_defs.is_empty() {
            return None;
        }

        let key = link_ref_fold_key(&label);
        let mut first = self.link_ref_defs.get(&key).cloned();
        // 分别查询原文、仅标签全折叠、仅定义全折叠，保持三种匹配及文档顺序。
        if let Some(def) = self
            .link_ref_defs
            .get(&link_ref_fold_key(&fold_bytes(&label)))
        {
            if first.as_ref().map(|f| def.order < f.order).unwrap_or(true) {
                first = Some(def.clone());
            }
        }
        if let Some(def) = self.link_ref_defs_folded.get(&key) {
            if first.as_ref().map(|f| def.order < f.order).unwrap_or(true) {
                first = Some(def.clone());
            }
        }
        first.map(|def| def.link)
    }

    /// 对应 Go 版 Tree.FindFootnotesDef。
    pub(super) fn find_footnotes_def(&mut self, label: &[u8]) -> Option<(usize, Node)> {
        let label = if self.editor_mode() {
            replace_bytes(label, crate::editor::CARET.as_bytes(), b"")
        } else {
            label.to_vec()
        };

        self.index_footnotes_defs();
        for (i, n) in self.footnotes_defs.iter().enumerate() {
            if eq_fold(n.tokens(), &label) {
                return Some((i + 1, *n));
            }
        }
        None
    }
}

/// 对应 Go 版 `bytes.EqualFold`。
fn eq_fold(a: &[u8], b: &[u8]) -> bool {
    let a = String::from_utf8_lossy(a);
    let b = String::from_utf8_lossy(b);
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) => {
                if simple_fold_min(x) != simple_fold_min(y) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

fn replace_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i..].starts_with(old) {
            ret.extend_from_slice(new);
            i += old.len();
        } else {
            ret.push(tokens[i]);
            i += 1;
        }
    }
    ret
}

#[cfg(test)]
mod go_reference_tests {
    use super::super::InlineParser;
    use crate::ast::NodeType;
    use crate::options::ParseOptions;

    /// Go parse/performance_regression_test.go: TestReferenceIndexCompatibility.
    /// Go's javascript build only uses simple folding; native also expands folds.
    #[test]
    fn go_reference_index_compatibility() {
        for (definitions, label, want) in [
            ("[Foo]: /first\n[foo]: /second\n", "FOO", "/first"),
            ("[K]: /first\n[K]: /second\n", "k", "/first"),
            ("[ſ]: /first\n[S]: /second\n", "s", "/first"),
            ("[ς]: /first\n[Σ]: /second\n", "σ", "/first"),
            (
                "[ẞ]: /first\n[SS]: /second\n",
                "ss",
                if cfg!(any(target_arch = "wasm32", feature = "javascript")) {
                    "/second"
                } else {
                    "/first"
                },
            ),
            (
                "[ss]: /first\n[ẞ]: /second\n",
                "ẞ",
                if cfg!(any(target_arch = "wasm32", feature = "javascript")) {
                    "/second"
                } else {
                    "/first"
                },
            ),
            ("[ßs]: /wrong\n[sß]: /right\n", "sß", "/right"),
            ("[ßs]: /wrong\n", "sß", ""),
            ("[foo]: /first\n", "missing", ""),
        ] {
            let mut tree = crate::parse::parse("", definitions.as_bytes(), ParseOptions::new());
            for protyle in [false, true] {
                let mut options = ParseOptions::new();
                options.protyle_wysiwyg = protyle;
                let label = if protyle {
                    format!("{label}{}", crate::editor::CARET)
                } else {
                    label.to_owned()
                };
                let mut parser = InlineParser::new(&mut tree, &options, false);
                let got = parser
                    .find_link_ref_def_link(label.as_bytes())
                    .and_then(|n| n.child_by_type(NodeType::NodeLinkDest))
                    .map(|n| String::from_utf8(n.tokens_vec()).unwrap())
                    .unwrap_or_default();
                assert_eq!(got, want, "{definitions:?}, {label:?}, protyle={protyle}");
            }
        }
    }
}
