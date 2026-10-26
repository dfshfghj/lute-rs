// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Vditor DOM 转 Markdown，对齐 Go 版 vditor_wysiwyg.go / vditor_ir.go。

mod gen;

use crate::ast::NodeType;
use crate::dom::{Dom, DomType};
use crate::editor;
use crate::options::{ParseOptions, RenderOptions};
use crate::render::format::FormatRenderer;

/// 对应 Go 版 Lute.vditorDOM2Md。
pub fn vditor_dom2md(html: &str, options: &ParseOptions) -> String {
    dom2md_with_options(html, options, &RenderOptions::new(), false)
}

/// 对应 Go 版 Lute.vditorIRDOM2Md。
pub fn vditor_ir_dom2md(html: &str, options: &ParseOptions) -> String {
    dom2md_with_options(html, options, &RenderOptions::new(), true)
}

pub(crate) fn trim_link_path<'a>(mut path: &'a str, options: &RenderOptions) -> &'a str {
    if !options.link_prefix.is_empty() {
        path = path.strip_prefix(&options.link_prefix).unwrap_or(path);
    }
    if !options.link_base.is_empty() {
        let mut base = options.link_base.clone();
        if !base.ends_with('/') {
            base.push('/');
        }
        path = path.strip_prefix(&base).unwrap_or(path);
    }
    path
}

pub(crate) fn dom2md_with_options(
    html: &str,
    options: &ParseOptions,
    render_options: &RenderOptions,
    ir: bool,
) -> String {
    // 删掉插入符
    let mut html = html.to_string();
    html = html.replace(editor::FRONT_END_CARET, "");

    // 替换结尾空白，否则 HTML 解析会产生冗余节点导致生成空的代码块
    html = html.replace("\t\n", "\n");
    html = html.replace("    \n", "  \n");

    if html.trim().is_empty() {
        return String::new();
    }

    let mut dom = crate::dom::parse(&html);
    adjust_vditor_dom(&mut dom, options);

    let mut tree = crate::ast::Tree::new_with_name("");
    let root = tree.root_raw();
    {
        let mut ctx = gen::Ctx {
            tree: &mut tree,
            tip: root,
            parse_options: options.clone(),
            render_options: render_options.clone(),
        };
        let mut child = dom.nodes[dom.root].children.first().copied();
        while let Some(c) = child {
            if ir {
                gen::gen_ast_ir(&mut ctx, &mut dom, c);
            } else {
                gen::gen_ast(&mut ctx, &mut dom, c);
            }
            child = dom.next_sibling(c);
        }
    }

    // 调整树结构
    let root = tree.root_raw();
    crate::ast::walk_raw(root, &mut |n, entering| {
        if entering {
            match n.typ() {
                NodeType::NodeInlineHTML
                | NodeType::NodeCodeSpan
                | NodeType::NodeInlineMath
                | NodeType::NodeHTMLBlock
                | NodeType::NodeCodeBlockCode
                | NodeType::NodeMathBlockContent => {
                    let tokens = crate::html::unescape_bytes(&n.tokens_vec());
                    n.set_tokens(tokens);
                    if let Some(next) = n.next() {
                        if NodeType::NodeCodeSpan == next.typ()
                            && n.data().code_marker_len == next.data().code_marker_len
                        {
                            // 合并代码节点 https://github.com/Vanessa219/vditor/issues/167
                            if let Some(c1) = n.first_child().and_then(|c| c.next()) {
                                if let Some(c2) = next.first_child().and_then(|c| c.next()) {
                                    let mut tokens = c1.tokens_vec();
                                    tokens.extend_from_slice(&c2.tokens_vec());
                                    c1.set_tokens(tokens);
                                }
                            }
                            next.unlink();
                        }
                    }
                }
                NodeType::NodeList => {
                    // 浏览器生成的子列表是 ul.ul 形式，需要将其调整为 ul.li.ul
                    if let Some(parent) = n.parent() {
                        if NodeType::NodeList == parent.typ() {
                            if let Some(previous_li) = n.previous() {
                                previous_li.append_child(n);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        crate::ast::WalkStatus::WalkContinue
    });

    // 将 AST 进行 Markdown 格式化渲染
    let mut render_options = crate::options::RenderOptions::new();
    render_options.auto_space = false;
    render_options.fix_term_typo = false;
    let mut r = FormatRenderer::new(&tree, render_options, options.clone());
    String::from_utf8_lossy(&r.render()).to_string()
}

/// 对应 Go 版 Lute.adjustVditorDOM。
pub(crate) fn adjust_vditor_dom(dom: &mut Dom, options: &ParseOptions) {
    remove_empty_nodes(dom);
    remove_highlight_js_spans(dom);

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        remove_wbr(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        merge_vditor_dom_list0(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_vditor_dom_list_tight0(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_vditor_dom_list_list(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_vditor_dom_list_item_in_p(dom, c);
    }

    let mut children = dom.nodes[dom.root].children.clone();
    for c in children.drain(..) {
        remove_code_code(dom, c);
    }

    let mut children = dom.nodes[dom.root].children.clone();
    for c in children.drain(..) {
        adjust_vditor_dom_code_a(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_tag(dom, c, options);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        merge_same_strong(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_table_code(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_math(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_noscript_img(dom, c);
    }

    let children = dom.nodes[dom.root].children.clone();
    for c in children {
        adjust_block_in_table(dom, c, options);
    }
}

/// 对应 Go 版 normalizeProtyleInlineCodeCaret。
///
/// 行级元素末尾的反引号移到元素外，标题容器中的输入则保留在标题内。
pub(crate) fn normalize_protyle_inline_code_caret(dom: &mut Dom, n: usize) {
    let children = dom.nodes[n].children.clone();
    for c in children {
        normalize_protyle_inline_code_caret(dom, c);
    }
    if !dom.is_element(n, "span")
        || dom
            .attr(n, "class")
            .split_whitespace()
            .any(|c| "callout-title" == c)
    {
        return;
    }
    let Some(&last) = dom.nodes[n].children.last() else {
        return;
    };
    if DomType::Text != dom.nodes[last].typ {
        return;
    }
    let marker = format!("`{}", editor::CARET);
    let Some(rest) = dom.nodes[last]
        .data
        .strip_suffix(&marker)
        .map(str::to_string)
    else {
        return;
    };
    dom.nodes[last].data = rest;
    let caret = dom.create(DomType::Text, marker);
    dom.insert_after(last, caret);
}

/// 对应 Go 版 normalizeProtyleInlineCaretPlaceholders。
///
/// 行内代码/键盘/标签容器开头的占位符（ZWSP、BOM、WordJoiner）需要去掉，
/// 否则会进入 TextMark 内容参与后续渲染与比较。
pub(crate) fn normalize_protyle_inline_caret_placeholders(dom: &mut Dom, n: usize) {
    if is_protyle_inline_caret_container(dom, n) {
        trim_protyle_inline_caret_placeholder(dom, n);
    }
    let children = dom.nodes[n].children.clone();
    for c in children {
        normalize_protyle_inline_caret_placeholders(dom, c);
    }
}

fn is_protyle_inline_caret_container(dom: &Dom, n: usize) -> bool {
    if dom.is_element(n, "code") || dom.is_element(n, "kbd") {
        return true;
    }
    if !dom.is_element(n, "span") {
        return false;
    }
    dom.attr(n, "data-type")
        .split_whitespace()
        .any(|t| "code" == t || "kbd" == t || "tag" == t)
}

fn trim_protyle_inline_caret_placeholder(dom: &mut Dom, n: usize) {
    let Some(&first) = dom.nodes[n].children.first() else {
        return;
    };
    if DomType::Text != dom.nodes[first].typ {
        return;
    }
    for placeholder in [editor::ZWSP, "﻿", editor::WORD_JOINER] {
        if let Some(rest) = dom.nodes[first].data.strip_prefix(placeholder) {
            dom.nodes[first].data = rest.to_string();
            break;
        }
    }
}

fn adjust_block_in_table(dom: &mut Dom, n: usize, options: &ParseOptions) {
    if dom.parent_is(n, &["td", "th"]) && (dom.is_element(n, "ul") || dom.is_element(n, "ol")) {
        let div = dom.create(DomType::Element, "div");
        let text = dom.create(DomType::Text, "");
        dom.insert_before(n, text);
        dom.unlink(n);
        dom.append_child(div, n);
        let html = dom.html(n);
        let md = crate::html2md::html2markdown(
            &html,
            options.clone(),
            crate::options::RenderOptions::new(),
        );
        dom.nodes[text].data = md.trim().replace('\n', "<br />");
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_block_in_table(dom, c, options);
    }
}

fn adjust_tag(dom: &mut Dom, n: usize, options: &ParseOptions) {
    if dom.is_element(n, "div") && "NodeCodeBlock" == dom.attr(n, "data-type")
        || dom.is_element(n, "span") && "code" == dom.attr(n, "data-type")
    {
        return;
    }

    if dom.nodes[n].typ == DomType::Element {
        match dom.nodes[n].data.as_str() {
            "ucapcontent" => dom.nodes[n].data = "div".into(),
            "ucaptitle" => dom.nodes[n].data = "h2".into(),
            "markerow8" => dom.nodes[n].data = "span".into(),
            "app-document-text" => dom.nodes[n].data = "div".into(),
            "ms-cmark-node" => dom.nodes[n].data = "span".into(),
            _ => {}
        }
    } else if dom.nodes[n].typ == DomType::Text
        && !dom.parent_is(n, &["table"])
        && options.protyle_wysiwyg
    {
        // 将非表格内的文本节点中的 <br> 转换为 \n
        let data = dom.nodes[n]
            .data
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");
        dom.nodes[n].data = data;
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_tag(dom, c, options);
    }
}

fn adjust_noscript_img(dom: &mut Dom, n: usize) {
    if let Some(parent) = dom.nodes[n].parent {
        if dom.is_element(parent, "figure")
            && dom.is_element(n, "noscript")
            && dom.nodes[n]
                .children
                .first()
                .map(|&c| dom.nodes[c].data.starts_with("<img "))
                .unwrap_or(false)
        {
            let img = dom.nodes[n].children[0];
            dom.unlink(img);
            let html = dom.nodes[img].data.clone();
            let fragment = crate::dom::parse(&html);
            if let Some(&first) = fragment.nodes[fragment.root].children.first() {
                // 将 fragment 中的节点复制到当前 DOM。
                let new_img = copy_dom_subtree(dom, &fragment, first);
                dom.insert_before(n, new_img);
            }
            let mut unlinks = Vec::new();
            let mut c = Some(n);
            while let Some(cur) = c {
                if !dom.is_element(cur, "figcaption") {
                    unlinks.push(cur);
                }
                c = dom.next_sibling(cur);
            }
            for u in unlinks {
                dom.unlink(u);
            }
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_noscript_img(dom, c);
    }
}

fn copy_dom_subtree(dst: &mut Dom, src: &Dom, idx: usize) -> usize {
    let typ = src.nodes[idx].typ;
    let new_idx = dst.create(typ, src.nodes[idx].data.clone());
    for (k, v) in &src.nodes[idx].attrs {
        dst.set_attr(new_idx, k, v);
    }
    for &c in &src.nodes[idx].children {
        let child = copy_dom_subtree(dst, src, c);
        dst.append_child(new_idx, child);
    }
    new_idx
}

fn adjust_math(dom: &mut Dom, n: usize) {
    let class = dom.attr(n, "class");
    if class.contains("katex--display") || class.contains("katex--inline") {
        if let Some(span) = dom.find_descendant_by_tag_and_class(n, "span", "katex-mathml") {
            if let Some(&first) = dom.nodes[span].children.first() {
                let mut tex = dom.text(first);
                if !tex.is_empty() {
                    tex = tex.trim().to_string();
                    while tex.contains("\n ") {
                        tex = tex.replace("\n ", "\n");
                    }
                    if let Some(idx) = tex.rfind("\n\n\n\n") {
                        if idx > 0 {
                            // 根据最后 4 个换行符分隔公式内容，适配 CSDN 公式剪藏
                            tex = tex[idx + 4..].to_string();
                        }
                    }
                    tex = tex.trim().to_string();
                    dom.set_attr(n, "data-tex", &tex);
                }
            }
        }
    }

    if ((dom.is_element(n, "span") || dom.is_element(n, "div"))
        && class.contains("mwe-math-element"))
        || (class.contains("tex") && !class.contains("text"))
    {
        let annos = dom.children_by_tag(n, "annotation");
        if let Some(&anno) = annos.first() {
            if "application/x-tex" == dom.attr(anno, "encoding") {
                let tex = dom.text(anno);
                dom.set_attr(n, "data-tex", &tex);
                return;
            }
        }

        let imgs = dom.children_by_tag(n, "img");
        if let Some(&img) = imgs.first() {
            let math_content = dom.attr(img, "alt");
            if !math_content.is_empty() {
                dom.set_attr(n, "data-tex", &math_content);
                return;
            }
        }
    }

    if dom.is_element(n, "img") && class.contains("ma-tex-img") {
        let math_content = dom.attr(n, "alt");
        if !math_content.is_empty() {
            dom.nodes[n].data = "span".into();
            dom.set_attr(n, "data-tex", &math_content);
        }
        return;
    }

    let formula = dom.attr(n, "data-formula");
    if !formula.is_empty() {
        if let Some(next) = dom.next_sibling(n) {
            if dom.is_element(next, "script") && dom.attr(next, "type").contains("math/tex") {
                dom.unlink(next);
            }
        }
        if dom.nodes[n].typ == DomType::Element && "mjx-container" == dom.nodes[n].data {
            dom.nodes[n].data = "span".into();
        }
        dom.set_attr(n, "data-tex", &formula);
        return;
    }

    if class.contains("texhtml") {
        let math_content = dom_texhtml(dom, n);
        if !math_content.is_empty() {
            dom.set_attr(n, "data-tex", &math_content);
            return;
        }
    }

    if class.contains("math") {
        let scripts = dom.children_by_tag(n, "script");
        if let Some(&script) = scripts.first() {
            if "math/tex" == dom.attr(script, "type") {
                let math_content = dom.text(script);
                if !math_content.is_empty() {
                    dom.set_attr(n, "data-tex", &math_content);
                    return;
                }
            }
        }

        let math_content = dom.attr(n, "data-custom-copy-text");
        if !math_content.is_empty() {
            dom.set_attr(n, "data-tex", &math_content);
            return;
        }

        let math_content = dom.attr(n, "data-math");
        if !math_content.is_empty() {
            dom.set_attr(n, "data-tex", &math_content);
            return;
        }
    }

    if class.split_whitespace().any(|c| "language-math" == c) {
        let math_content = dom.attr(n, "data-math");
        if !math_content.is_empty() {
            dom.set_attr(n, "data-tex", &math_content);
            return;
        }
        let math_content = dom.text(n);
        if !math_content.is_empty() {
            dom.set_attr(n, "data-tex", &math_content);
            return;
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_math(dom, c);
    }
}

fn dom_texhtml(dom: &Dom, n: usize) -> String {
    if dom.nodes[n].typ == DomType::Text {
        return dom.nodes[n].data.clone();
    }
    let mut buf = String::new();
    for &c in &dom.nodes[n].children {
        dom_texhtml0(dom, c, &mut buf);
    }
    buf
}

fn dom_texhtml0(dom: &Dom, n: usize, buf: &mut String) {
    match dom.nodes[n].typ {
        DomType::Text => buf.push_str(&escape_math_symbol(&dom.nodes[n].data)),
        DomType::Element => match dom.nodes[n].data.as_str() {
            "sup" => buf.push_str("^{"),
            "sub" => buf.push_str("_{"),
            _ => {}
        },
        _ => {}
    }
    for &c in &dom.nodes[n].children {
        dom_texhtml0(dom, c, buf);
    }
    if dom.is_element(n, "sup") || dom.is_element(n, "sub") {
        buf.push('}');
    }
}

fn escape_math_symbol(s: &str) -> String {
    s.replace('_', "\\_")
        .replace('^', "\\^")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

fn adjust_table_code(dom: &mut Dom, n: usize) {
    if dom.is_element(n, "table") {
        let tds = dom.children_by_tag(n, "td");
        let mut unlinks = Vec::new();
        for td in tds {
            let td_class = dom.attr(td, "class");
            if td_class.contains("gutter") {
                unlinks.push(td);
                continue;
            }

            // 移除 <span class="lnt"> 格式的行号
            let spans = dom.children_by_tag(td, "span");
            let removed = spans.iter().any(|&s| "lnt" == dom.attr(s, "class"));
            if removed {
                unlinks.push(td);
                continue;
            }

            if td_class.contains("code") {
                if let Some(&c) = dom.nodes[td].children.first() {
                    if dom.is_element(c, "div") {
                        dom.nodes[c].data = "pre".into();
                        let mut lang = dom.attr(n, "class");
                        lang = lang.replace("syntaxhighlighter", "");
                        lang = lang.trim().to_string();
                        if !lang.is_empty() {
                            dom.set_attr(c, "class", &lang);
                        }
                        let divs = dom.nodes[c].children.clone();
                        for div in divs {
                            dom.nodes[div].data = "code".into();
                        }
                    }
                }
            }
        }

        for u in unlinks {
            dom.unlink(u);
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_table_code(dom, c);
    }
}

fn merge_same_strong(dom: &mut Dom, n: usize) {
    let mut c = dom.nodes[n].children.first().copied();
    while let Some(cur) = c {
        let next = dom.next_sibling(cur);
        if let Some(next) = next {
            if dom.is_element(cur, "strong") && dom.is_element(next, "strong") {
                let children = dom.nodes[next].children.clone();
                for cc in children {
                    dom.unlink(cc);
                    dom.append_child(cur, cc);
                }
                dom.unlink(next);
                c = dom.next_sibling(cur);
                continue;
            }
        }
        c = next;
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        merge_same_strong(dom, c);
    }
}

fn adjust_vditor_dom_list_list(dom: &mut Dom, n: usize) {
    if !dom.is_element(n, "ul") && !dom.is_element(n, "ol") && !dom.is_element(n, "li") {
        return;
    }

    if dom.is_element(n, "li") {
        if let Some(&first) = dom.nodes[n].children.first() {
            if dom.is_element(first, "br") {
                // 规范化换行时 li 的结构，对调 ZWSP 和 <br> 的位置
                dom.nodes[first].typ = DomType::Text;
                dom.nodes[first].data = editor::ZWSP.to_string();
                if let Some(next_li) = dom.next_sibling(n) {
                    if dom.is_element(next_li, "li") {
                        if let Some(&caret) = dom.nodes[next_li].children.first() {
                            let caret_zwsp = format!("{}{}", editor::CARET, editor::ZWSP);
                            if caret_zwsp == dom.nodes[caret].data {
                                dom.nodes[caret].data = format!("{}\n", editor::CARET);
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(parent) = dom.nodes[n].parent {
        if dom.is_element(parent, "ul") || dom.is_element(parent, "ol") {
            if let Some(prev_li) = dom.prev_sibling(n) {
                dom.unlink(n);
                dom.append_child(prev_li, n);
            }
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_vditor_dom_list_list(dom, c);
    }
}

fn remove_highlight_js_spans(dom: &mut Dom) {
    let mut spans = Vec::new();
    let mut c = Some(dom.root);
    while let Some(cur) = c {
        hljs_spans(dom, cur, &mut spans);
        c = dom.next_sibling(cur);
    }
    for span in spans {
        let text = dom.text(span);
        let text_node = dom.create(DomType::Text, text);
        dom.insert_before(span, text_node);
        dom.unlink(span);
    }
}

fn hljs_spans(dom: &Dom, n: usize, spans: &mut Vec<usize>) {
    if dom.is_element(n, "span") && dom.attr(n, "class").starts_with("hljs-") {
        spans.push(n);
        let text = dom.text(n);
        let _ = text;
    }
    for &c in &dom.nodes[n].children {
        hljs_spans(dom, c, spans);
    }
}

fn remove_empty_nodes(dom: &mut Dom) {
    let mut empty = Vec::new();
    let mut c = Some(dom.root);
    while let Some(cur) = c {
        search_empty_nodes(dom, cur, &mut empty);
        c = dom.next_sibling(cur);
    }
    for e in empty {
        dom.unlink(e);
    }
}

pub(super) fn is_inline(dom: &Dom, n: Option<usize>) -> bool {
    let Some(n) = n else {
        return false;
    };
    if dom.nodes[n].typ == DomType::Text {
        return true;
    }
    if dom.nodes[n].typ != DomType::Element {
        return false;
    }
    matches!(
        dom.nodes[n].data.as_str(),
        "code"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "mark"
            | "sup"
            | "sub"
            | "del"
            | "s"
            | "strike"
            | "a"
            | "img"
            | "u"
            | "kbd"
            | "span"
    )
}

fn search_empty_nodes(dom: &mut Dom, n: usize, empty: &mut Vec<usize>) {
    let typ = dom.nodes[n].typ;
    let is_text = typ == DomType::Text;
    // 对应 Go 版 switch n.DataAtom 的 case 0：文本、注释、DOCTYPE 等无标签节点。
    let is_zero_atom = matches!(typ, DomType::Text | DomType::Comment | DomType::Doctype);
    let tag = dom.nodes[n].data.clone();

    if is_zero_atom {
        let prev_inline = is_inline(dom, dom.prev_sibling(n));
        let next_inline = is_inline(dom, dom.next_sibling(n));
        let parent_inline = is_inline(dom, dom.nodes[n].parent);
        if !is_text {
            // 注释/DOCTYPE 只在非行级上下文中被移除，与 Go 版 case 0 一致。
            if !(prev_inline || next_inline || parent_inline) {
                if dom.nodes[n].data.is_empty() {
                    empty.push(n);
                }
                if DomType::Comment == typ {
                    empty.push(n);
                }
            }
        } else if prev_inline || next_inline || parent_inline {
            // 前节点或者后节点是行级节点的话保留该空白
            if let Some(prev) = dom.prev_sibling(n) {
                if dom.is_element(prev, "strong") {
                    // 前节点是加粗节点的话去掉 ZWSP
                    let mut data = dom.nodes[n].data.clone();
                    if let Some(stripped) = data.strip_prefix(editor::ZWSP) {
                        data = stripped.to_string();
                    }
                    if let Some(stripped) = data.strip_suffix(editor::ZWSP) {
                        data = stripped.to_string();
                    }
                    dom.nodes[n].data = data;
                }
            }
        } else {
            let mut data = dom.nodes[n].data.trim_matches(' ').to_string();
            while data.contains("\n\n") {
                // 浏览器剪藏扩展列表下方段落缩进成为子块
                data = data.replace("\n\n", "");
            }
            if data.is_empty() {
                empty.push(n);
                return;
            }

            if let Some(parent) = dom.nodes[n].parent {
                if dom.is_element(parent, "ol")
                    || dom.is_element(parent, "ul")
                    || dom.is_element(parent, "li")
                {
                    let next_is_none = dom.next_sibling(n).is_none();
                    let next_is_text = dom
                        .next_sibling(n)
                        .map(|x| dom.nodes[x].typ == DomType::Text)
                        .unwrap_or(false);
                    let next_is_ul = dom
                        .next_sibling(n)
                        .map(|x| dom.is_element(x, "ul"))
                        .unwrap_or(false);
                    if next_is_none
                        || next_is_text
                        || next_is_ul
                        || dom.nodes[n].data.trim().is_empty()
                    {
                        let trimmed = dom.nodes[n]
                            .data
                            .trim_end_matches(['\n', '\t', ' '])
                            .to_string();
                        dom.nodes[n].data = trimmed;
                    }
                }
                if dom.is_element(parent, "table")
                    || dom.is_element(parent, "thead")
                    || dom.is_element(parent, "tbody")
                    || dom.is_element(parent, "tr")
                {
                    dom.nodes[n].data = dom.nodes[n].data.trim().to_string();
                }
            }

            if dom.nodes[n].data.is_empty() {
                empty.push(n);
            }
        }
        return;
    }

    if dom.is_element(n, "span") {
        if let Some(&last) = dom.nodes[n].children.last() {
            if dom.is_element(last, "br") {
                // 如果行级标记节点最后一个子节点是 <br>，则将该 <br> 移动到该行级标记节点的后面
                dom.insert_after(n, last);
            }
        }

        let data_type = dom.attr(n, "data-type");
        if "search-mark" == data_type || "virtual-block-ref" == data_type {
            // 将嵌套在临时标记中的节点提升到临时标记节点之前
            empty.push(n);
            let children = dom.nodes[n].children.clone();
            for c in children {
                dom.insert_before(n, c);
            }
            return;
        }
    } else if matches!(
        tag.as_str(),
        "strong" | "b" | "em" | "i" | "del" | "s" | "strike" | "mark"
    ) {
        if let Some(&first) = dom.nodes[n].children.first() {
            if dom.is_element(first, "br") {
                empty.push(first);
                let br = dom.create(DomType::Element, "br");
                dom.insert_before(n, br);
            }
            if dom.nodes[first].typ == DomType::Text {
                let text = dom.nodes[first].data.clone();
                let spaces = prefix_spaces(&text);
                if !spaces.is_empty() {
                    dom.nodes[first].data = format!("{}{}", editor::ZWSP, text);
                }
            }
        }
        if let Some(&last) = dom.nodes[n].children.last() {
            if dom.is_element(last, "br") {
                empty.push(last);
                let br = dom.create(DomType::Element, "br");
                dom.insert_after(n, br);
            }
            if dom.nodes[last].typ == DomType::Text {
                let text = dom.nodes[last].data.clone();
                let spaces = suffix_spaces(&text);
                if !spaces.is_empty() {
                    if let Some(&first) = dom.nodes[n].children.first() {
                        let data = dom.nodes[first].data.clone();
                        dom.nodes[first].data = format!("{}{}", data, editor::ZWSP);
                    }
                }
            }
        }
    } else if "katex" == dom.attr(n, "class") {
        empty.push(n);
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        search_empty_nodes(dom, c, empty);
    }

    if dom.is_element(n, "ol") || dom.is_element(n, "ul") {
        if "footnotes-defs-ol" == dom.attr(n, "data-type") {
            return;
        }
        if let Some(&first) = dom.nodes[n].children.first() {
            if let Some(&ff) = dom.nodes[first].children.first() {
                if !dom.is_element(ff, "input") {
                    return;
                }
            }
        }

        let text = dom.text(n);
        if text.is_empty() {
            empty.push(n);
        }
    }
}

fn remove_wbr(dom: &mut Dom, n: usize) {
    let mut wbr = Vec::new();
    let mut c = Some(n);
    while let Some(cur) = c {
        search_wbr(dom, cur, &mut wbr);
        c = dom.next_sibling(cur);
    }
    for w in wbr {
        dom.unlink(w);
    }
}

fn search_wbr(dom: &Dom, n: usize, wbr: &mut Vec<usize>) {
    if dom.is_element(n, "wbr") && !dom.attr_values_with_prefix(n, "data-v-").is_empty() {
        wbr.push(n);
    }
    for &c in &dom.nodes[n].children {
        search_wbr(dom, c, wbr);
    }
}

fn merge_vditor_dom_list0(dom: &mut Dom, n: usize) {
    if dom.is_element(n, "ul") || dom.is_element(n, "ol") {
        if let Some(next) = dom.next_sibling(n) {
            if dom.nodes[n].data == dom.nodes[next].data && 1 == dom.nodes[next].attrs.len() {
                let children = dom.nodes[next].children.clone();
                for c in children {
                    dom.unlink(c);
                    dom.append_child(n, c);
                }
                dom.unlink(next);
            }
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        merge_vditor_dom_list0(dom, c);
    }
}

fn adjust_vditor_dom_list_tight0(dom: &mut Dom, n: usize) {
    if (dom.is_element(n, "ul") || dom.is_element(n, "ol")) && !dom.parent_is(n, &["pre"]) {
        let tight = is_tight_list(dom, n);
        dom.set_attr(n, "data-tight", &tight);
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_vditor_dom_list_tight0(dom, c);
    }
}

fn is_tight_list(dom: &Dom, list: usize) -> String {
    for &li in &dom.nodes[list].children {
        let mut sub_lists = 0;
        let mut sub_divs = 0;
        let mut sub_blockquotes = 0;
        let mut sub_paragraphs = 0;
        for &c in &dom.nodes[li].children {
            if dom.is_element(c, "ul") || dom.is_element(c, "ol") {
                sub_lists += 1;
            } else if dom.is_element(c, "div") {
                sub_divs += 1;
            } else if dom.is_element(c, "blockquote") {
                sub_blockquotes += 1;
            } else if dom.is_element(c, "p") {
                sub_paragraphs += 1;
            }
        }
        if 1 < sub_paragraphs || 1 < sub_blockquotes || 1 < sub_divs || 1 < sub_lists {
            return "false".to_string();
        }
        if 1 < sub_paragraphs + sub_divs
            || 1 < sub_paragraphs + sub_blockquotes
            || 1 < sub_paragraphs + sub_lists
        {
            return "false".to_string();
        }
    }
    "true".to_string()
}

fn adjust_vditor_dom_list_item_in_p(dom: &mut Dom, n: usize) {
    if dom.is_element(n, "li") {
        // li 换行时 id 重复需要重新生成
        if let Some(prev) = dom.prev_sibling(n) {
            if dom.attr(prev, "data-node-id") == dom.attr(n, "data-node-id") {
                dom.set_attr(n, "data-node-id", &crate::ast::new_node_id());
            }
        }
        // 松散 li 换行时和上一个 li.last id 重复
        if let Some(prev) = dom.prev_sibling(n) {
            if let Some(&first) = dom.nodes[n].children.first() {
                let id = dom.attr(first, "data-node-id");
                if !id.is_empty() {
                    if let Some(&prev_last) = dom.nodes[prev].children.last() {
                        if dom.attr(prev_last, "data-node-id") == id {
                            dom.set_attr(first, "data-node-id", &crate::ast::new_node_id());
                        }
                    }
                }
            }
        }

        // 在 li 下的每个非容器块节点用 p 包裹
        let mut c = dom.nodes[n].children.first().copied();
        while let Some(cur) = c {
            if list_item_enter(dom, n) {
                let p = dom.create(DomType::Element, "p");
                let caret = dom.create(DomType::Text, editor::CARET);
                let br = dom.create(DomType::Element, "br");
                dom.append_child(p, caret);
                dom.append_child(p, br);
                if let Some(&first) = dom.nodes[n].children.first() {
                    dom.unlink(first);
                }
                if let Some(&first) = dom.nodes[n].children.first() {
                    dom.unlink(first);
                }
                dom.append_child(n, p);
                c = Some(p);
                continue;
            }

            let is_block = dom.is_element(cur, "p")
                || dom.is_element(cur, "blockquote")
                || dom.is_element(cur, "ul")
                || dom.is_element(cur, "ol")
                || dom.is_element(cur, "div")
                || dom.is_element(cur, "table");
            if !is_block {
                let (spans, next_block) = forward_next_block(dom, cur);
                let p = dom.create(DomType::Element, "p");
                dom.insert_before(cur, p);
                for span in spans {
                    dom.unlink(span);
                    dom.append_child(p, span);
                }
                if let Some(nb) = next_block {
                    c = Some(nb);
                } else {
                    break;
                }
                continue;
            }
            c = dom.next_sibling(cur);
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_vditor_dom_list_item_in_p(dom, c);
    }
}

fn forward_next_block(dom: &Dom, span_node: usize) -> (Vec<usize>, Option<usize>) {
    let mut spans = Vec::new();
    let mut next = Some(span_node);
    while let Some(n) = next {
        if dom.is_element(n, "ol")
            || dom.is_element(n, "ul")
            || dom.is_element(n, "div")
            || dom.is_element(n, "blockquote")
            || dom.is_element(n, "table")
        {
            return (spans, None);
        }
        spans.push(n);
        next = dom.next_sibling(n);
    }
    (spans, None)
}

fn list_item_enter(dom: &Dom, li: usize) -> bool {
    let Some(&first) = dom.nodes[li].children.first() else {
        return false;
    };
    let Some(&last) = dom.nodes[li].children.last() else {
        return false;
    };
    editor::CARET == dom.nodes[first].data && "br" == dom.nodes[last].data
}

fn remove_code_code(dom: &mut Dom, n: usize) {
    if dom.is_element(n, "code") {
        if let Some(&first) = dom.nodes[n].children.first() {
            if dom.is_element(first, "code") {
                // code.code 重复嵌套，则不处理外层 code
                let children = dom.nodes[n].children.clone();
                for c in children {
                    dom.unlink(c);
                    dom.insert_before(n, c);
                }
                dom.unlink(n);
                return;
            }
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        remove_code_code(dom, c);
    }
}

fn adjust_vditor_dom_code_a(dom: &mut Dom, n: usize) {
    if dom.is_element(n, "code") {
        if let Some(&first) = dom.nodes[n].children.first() {
            let only = dom.nodes[n].children.len() == 1;
            if dom.is_element(first, "a") && only {
                // code.a 的情况将 a 移到 code 外层，即 a.code
                let prev = dom.prev_sibling(n);
                let next = dom.next_sibling(n);
                let parent = dom.nodes[n].parent;
                let a = first;
                dom.unlink(a);
                dom.unlink(n);

                let anchor_texts = dom.nodes[a].children.clone();
                for c in anchor_texts {
                    dom.unlink(c);
                    dom.append_child(n, c);
                }
                dom.append_child(a, n);

                if let Some(prev) = prev {
                    dom.insert_after(prev, a);
                } else if let Some(next) = next {
                    dom.insert_before(next, a);
                } else if let Some(parent) = parent {
                    dom.append_child(parent, a);
                }
                return;
            }
        }
    }

    let children = dom.nodes[n].children.clone();
    for c in children {
        adjust_vditor_dom_code_a(dom, c);
    }
}

fn prefix_spaces(text: &str) -> String {
    let mut ret = String::new();
    for c in text.chars() {
        if ' ' == c || '\u{00a0}' == c {
            ret.push(' ');
        } else {
            return ret;
        }
    }
    ret
}

fn suffix_spaces(text: &str) -> String {
    let mut ret = String::new();
    for c in text.chars().rev() {
        if ' ' == c || '\u{00a0}' == c {
            ret.push(' ');
        } else {
            return ret;
        }
    }
    ret
}

pub(crate) fn replace_all(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    crate::render::replace_bytes(tokens, old, new)
}

#[cfg(test)]
mod source_alignment_tests {
    use super::*;

    #[test]
    fn trim_link_path_removes_one_prefix_then_one_normalized_base() {
        let mut options = RenderOptions::new();
        options.link_prefix = "prefix:".into();
        options.link_base = "/configured/base".into();
        for (path, expected) in [
            ("prefix:/configured/base/dir/foo.txt", "dir/foo.txt"),
            ("prefix:/configured/base/../foo.png", "../foo.png"),
            ("prefix:/configured/base/./foo.png", "./foo.png"),
            (
                "https://example.com/configured/base/foo.txt",
                "https://example.com/configured/base/foo.txt",
            ),
            (
                "prefix:prefix:/configured/base/foo",
                "prefix:/configured/base/foo",
            ),
            (
                "/configured/base-sibling/foo",
                "/configured/base-sibling/foo",
            ),
        ] {
            assert_eq!(trim_link_path(path, &options), expected);
        }
        options.link_base.push('/');
        assert_eq!(
            trim_link_path("prefix:/configured/base/foo", &options),
            "foo"
        );
    }

    #[test]
    fn texhtml_balances_nested_script_braces_and_escapes_text() {
        // util.DomTexhtml / domTexhtml0: opening and closing script braces are
        // structural, while braces and underscores in text are escaped.
        let dom = crate::dom::parse("<span class=\"texhtml\">{<i>f</i><sub><i>k</i></sub>}<sup>n<sub>i_j</sub></sup></span>");
        let formula = dom.nodes[dom.root].children[0];
        assert_eq!(dom_texhtml(&dom, formula), "\\{f_{k}\\}^{n_{i\\_j}}");
    }
}
