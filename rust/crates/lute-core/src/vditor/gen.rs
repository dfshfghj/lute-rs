// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 根据 Vditor DOM 生成 Markdown AST，对齐 Go 版 genASTByVditorDOM。

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::dom::{Dom, DomType};
use crate::editor;
use crate::options::ParseOptions;

use super::{is_inline, replace_all};

pub(crate) struct Ctx<'a> {
    pub tree: &'a mut Tree,
    pub tip: Node,
    pub parse_options: ParseOptions,
    pub render_options: crate::options::RenderOptions,
}

impl<'a> Ctx<'a> {
    fn new_node_raw(&mut self, typ: NodeType) -> Node {
        self.tree.new_node_raw(typ)
    }

    fn append(&mut self, n: Node) {
        self.tip.append_child(n);
    }

    fn append_text(&mut self, tokens: &[u8]) {
        let n = self.new_node_raw(NodeType::NodeText);
        n.set_tokens(tokens.to_vec());
        self.append(n);
    }
}

fn dom_text(dom: &Dom, n: usize) -> String {
    dom.text(n)
}

fn attr(dom: &Dom, n: usize, key: &str) -> String {
    dom.attr(n, key)
}

fn is_tag(dom: &Dom, n: usize, tag: &str) -> bool {
    dom.is_element(n, tag)
}

fn starts_with_newline(dom: &Dom, n: usize) -> bool {
    let data = &dom.nodes[n].data;
    data.starts_with('\n') || data.starts_with(&format!("{}\n", editor::ZWSP))
}

fn is_empty_text(dom: &Dom, n: usize) -> bool {
    if let Some(&first) = dom.nodes[n].children.first() {
        if "block-ref" == attr(dom, first, "data-type") {
            return false;
        }
    }
    let text = dom_text(dom, n).trim().to_string();
    if text.is_empty() || editor::ZWSP == text {
        return true;
    }
    let a = format!("{}{}", editor::ZWSP, editor::CARET);
    let b = format!("{}{}", editor::CARET, editor::ZWSP);
    a == text || b == text
}

fn has_attr(dom: &Dom, n: usize, key: &str) -> bool {
    dom.has_attr(n, key)
}

fn first_child(dom: &Dom, n: usize) -> Option<usize> {
    dom.nodes[n].children.first().copied()
}

fn last_child(dom: &Dom, n: usize) -> Option<usize> {
    dom.nodes[n].children.last().copied()
}

fn prev_sibling(dom: &Dom, n: usize) -> Option<usize> {
    dom.prev_sibling(n)
}

pub(crate) fn gen_ast(ctx: &mut Ctx, dom: &mut Dom, idx: usize) {
    let data_render = attr(dom, idx, "data-render");
    if "1" == data_render || "2" == data_render {
        // 1：浮动工具栏，2：preview 代码块、数学公式块
        return;
    }

    let data_type = attr(dom, idx, "data-type");
    let tag =
        if dom.nodes[idx].typ == DomType::Text || !crate::dom::is_known_tag(&dom.nodes[idx].data) {
            String::new()
        } else {
            dom.nodes[idx].data.clone()
        };

    if "div" == tag {
        if "code-block" == data_type
            || "html-block" == data_type
            || "math-block" == data_type
            || "yaml-front-matter" == data_type
        {
            gen_children(ctx, dom, idx, gen_ast);
        } else if "link-ref-defs-block" == data_type {
            let text = dom_text(dom, idx);
            ctx.append_text(text.as_bytes());
        } else if "footnotes-block" == data_type {
            let Some(ol) = first_child(dom, idx) else {
                return;
            };
            if !is_tag(dom, ol, "ol") {
                return;
            }
            let mut li = first_child(dom, ol);
            while let Some(cur) = li {
                if "\n" == dom.nodes[cur].data {
                    li = dom.next_sibling(cur);
                    continue;
                }
                let original_html = dom.html(cur);
                let md = super::dom2md_with_options(
                    &format!("<ol data-type=\"footnotes-defs-ol\">{}</ol>", original_html),
                    &ctx.parse_options,
                    &ctx.render_options,
                    false,
                );
                let label = attr(dom, cur, "data-marker");
                let md = if md.len() >= 3 { &md[3..] } else { "" };
                let mut lines = md.split('\n').collect::<Vec<_>>();
                if lines.last() == Some(&"") {
                    lines.pop();
                }
                let mut out = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if 0 < i {
                        out.push_str("    ");
                        out.push_str(line);
                    } else {
                        out.push_str(line);
                    }
                    out.push('\n');
                }
                let out = format!("[{}]: {}", label, out);
                ctx.append_text(out.as_bytes());
                li = dom.next_sibling(cur);
            }
        } else if "toc-block" == data_type {
            let node = ctx.new_node_raw(NodeType::NodeToC);
            ctx.append(node);
        }
        return;
    }

    let class = attr(dom, idx, "class");
    let content = dom.nodes[idx].data.replace(editor::ZWSP, "");
    let node = ctx.new_node_raw(NodeType::NodeText);
    node.set_tokens(content.as_bytes().to_vec());

    match tag.as_str() {
        "" => {
            if content.is_empty() {
                return;
            }

            let check_indent_code_block = content.replace(editor::CARET, "").replace('\t', "    ");
            if !is_inline(dom, prev_sibling(dom, idx))
                && check_indent_code_block.starts_with("    ")
            {
                node.set_typ(NodeType::NodeCodeBlock);
                node.data_mut().is_fenced_code_block = true;
                let open = ctx.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                open.set_tokens("\u{0060}\u{0060}\u{0060}".as_bytes().to_vec());
                open.data_mut().code_block_fence_len = 3;
                node.append_child(open);
                let info = ctx.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                node.append_child(info);
                let start_caret = content.starts_with(editor::CARET);
                let mut content = content.clone();
                if start_caret {
                    content = content.replace(editor::CARET, "");
                }
                content = content.trim().to_string();
                if start_caret {
                    content = format!("{}{}", editor::CARET, content);
                }
                let code = ctx.new_node_raw(NodeType::NodeCodeBlockCode);
                code.set_tokens(content.as_bytes().to_vec());
                node.append_child(code);
                let close = ctx.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                close.set_tokens("\u{0060}\u{0060}\u{0060}".as_bytes().to_vec());
                close.data_mut().code_block_fence_len = 3;
                node.append_child(close);
                ctx.append(node);
                return;
            }
            if let Some(parent) = dom.nodes[idx].parent {
                if is_tag(dom, parent, "a") {
                    node.set_typ(NodeType::NodeLinkText);
                }
            }
            ctx.append(node);
        }
        "p" | "div" => {
            node.set_typ(NodeType::NodeParagraph);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if dom_text(dom, idx).trim().is_empty() {
                return;
            }
            node.set_typ(NodeType::NodeHeading);
            let level = (tag.as_bytes()[1] - b'0') as i32;
            node.data_mut().heading_level = level;
            let marker = attr(dom, idx, "data-marker");
            let id = attr(dom, idx, "data-id");
            node.data_mut().heading_setext = "=" == marker || "-" == marker;
            if !node.data().heading_setext {
                let h = ctx.new_node_raw(NodeType::NodeHeadingC8hMarker);
                h.set_tokens("#".repeat(level as usize).into_bytes());
                node.append_child(h);
            }
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            if !id.is_empty() {
                ctx.append_text(format!(" {{{}}}", id).as_bytes());
            }
            ctx.tip = old;
            return;
        }
        "hr" => {
            node.set_typ(NodeType::NodeThematicBreak);
            ctx.append(node);
        }
        "blockquote" => {
            let content = dom_text(dom, idx).trim().to_string();
            if content.is_empty() || "&gt;" == content || editor::CARET == content {
                return;
            }
            node.set_typ(NodeType::NodeBlockquote);
            let marker = ctx.new_node_raw(NodeType::NodeBlockquoteMarker);
            marker.set_tokens(b">".to_vec());
            node.append_child(marker);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "ol" | "ul" => {
            if first_child(dom, idx).is_none() {
                return;
            }
            node.set_typ(NodeType::NodeList);
            let mut ld = crate::ast::ListData::default();
            if "ol" == tag {
                ld.typ = 1;
            }
            let tight = attr(dom, idx, "data-tight");
            if "true" == tight || tight.is_empty() {
                ld.tight = true;
            }
            node.data_mut().list_data = Some(ld);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "li" => {
            node.set_typ(NodeType::NodeListItem);
            let mut marker = attr(dom, idx, "data-marker");
            let mut bullet = 0u8;
            if marker.is_empty() {
                if let Some(parent) = dom.nodes[idx].parent {
                    if is_tag(dom, parent, "ol") {
                        let first_li_marker = first_child(dom, parent)
                            .map(|c| attr(dom, c, "data-marker"))
                            .unwrap_or_default();
                        let start_attr = attr(dom, parent, "start");
                        marker = if start_attr.is_empty() {
                            "1".to_string()
                        } else {
                            start_attr
                        };
                        if !first_li_marker.is_empty() {
                            marker.push(first_li_marker.chars().last().unwrap_or('.'));
                        } else {
                            marker.push('.');
                        }
                    } else {
                        marker = attr(dom, parent, "data-marker");
                        if marker.is_empty() {
                            marker = "*".to_string();
                        }
                        bullet = marker.as_bytes()[0];
                    }
                } else {
                    marker = attr(dom, idx, "data-marker");
                    if marker.is_empty() {
                        marker = "*".to_string();
                    }
                    bullet = marker.as_bytes()[0];
                }
            } else if let Some(parent) = dom.nodes[idx].parent {
                if is_tag(dom, parent, "ol") {
                    if "*" == marker || "-" == marker || "+" == marker {
                        marker = "1.".to_string();
                    }
                    if "1." != marker
                        && "1)" != marker
                        && dom
                            .prev_sibling(idx)
                            .map(|p| !is_tag(dom, p, "li"))
                            .unwrap_or(false)
                    {
                        if let Some(grandparent) = dom.nodes[parent].parent {
                            if is_tag(dom, grandparent, "ol") || is_tag(dom, grandparent, "ul") {
                                // 子有序列表第一项必须从 1 开始
                                marker = "1.".to_string();
                            }
                        }
                    }
                    if "1." != marker
                        && "1)" != marker
                        && is_tag(dom, parent, "ol")
                        && first_child(dom, parent) == Some(idx)
                        && attr(dom, parent, "start").is_empty()
                    {
                        marker = "1.".to_string();
                    }
                } else {
                    if "*" != marker && "-" != marker && "+" != marker {
                        marker = "*".to_string();
                    }
                    bullet = marker.as_bytes()[0];
                }
            } else {
                marker = attr(dom, idx, "data-marker");
                if marker.is_empty() {
                    marker = "*".to_string();
                }
                bullet = marker.as_bytes()[0];
            }
            let mut ld = crate::ast::ListData {
                marker: marker.as_bytes().to_vec(),
                bullet_char: bullet,
                ..Default::default()
            };
            if 0 == bullet {
                ld.num = marker
                    .chars()
                    .next()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(0) as i32;
                ld.delimiter = *marker.as_bytes().last().unwrap_or(&b'.');
            }
            node.data_mut().list_data = Some(ld);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "pre" => {
            let Some(code) = first_child(dom, idx) else {
                return;
            };
            if !is_tag(dom, code, "code") {
                return;
            }
            let parent = dom.nodes[idx].parent.unwrap_or(idx);
            let mut marker = attr(dom, parent, "data-marker");
            if marker.is_empty() {
                marker = "\u{0060}\u{0060}\u{0060}".to_string();
            }

            let mut code_tokens = Vec::new();
            if let Some(&text) = dom.nodes[code].children.first() {
                code_tokens = dom.nodes[text].data.as_bytes().to_vec();
            }

            let div_data_type = attr(dom, parent, "data-type");
            match div_data_type.as_str() {
                "math-block" => {
                    node.set_typ(NodeType::NodeMathBlock);
                    let open = ctx.new_node_raw(NodeType::NodeMathBlockOpenMarker);
                    node.append_child(open);
                    let content = ctx.new_node_raw(NodeType::NodeMathBlockContent);
                    content.set_tokens(code_tokens);
                    node.append_child(content);
                    let close = ctx.new_node_raw(NodeType::NodeMathBlockCloseMarker);
                    node.append_child(close);
                    ctx.append(node);
                }
                "yaml-front-matter" => {
                    node.set_typ(NodeType::NodeYamlFrontMatter);
                    let open = ctx.new_node_raw(NodeType::NodeYamlFrontMatterOpenMarker);
                    node.append_child(open);
                    let content = ctx.new_node_raw(NodeType::NodeYamlFrontMatterContent);
                    content.set_tokens(code_tokens);
                    node.append_child(content);
                    let close = ctx.new_node_raw(NodeType::NodeYamlFrontMatterCloseMarker);
                    node.append_child(close);
                    ctx.append(node);
                }
                "html-block" => {
                    node.set_typ(NodeType::NodeHTMLBlock);
                    node.set_tokens(code_tokens);
                    ctx.append(node);
                }
                _ => {
                    node.set_typ(NodeType::NodeCodeBlock);
                    node.data_mut().is_fenced_code_block = true;
                    let open = ctx.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                    open.set_tokens(marker.as_bytes().to_vec());
                    open.data_mut().code_block_fence_len = marker.len() as i32;
                    node.append_child(open);
                    let info = ctx.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                    node.append_child(info);
                    let class = attr(dom, code, "class");
                    if class.contains("language-") {
                        let language = class["language-".len()..].to_string();
                        if let Some(last) = node.last_child() {
                            last.data_mut().code_block_info = language.into_bytes();
                        }
                    }
                    let content = ctx.new_node_raw(NodeType::NodeCodeBlockCode);
                    content.set_tokens(code_tokens);
                    node.append_child(content);
                    let close = ctx.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                    close.set_tokens(marker.as_bytes().to_vec());
                    close.data_mut().code_block_fence_len = marker.len() as i32;
                    node.append_child(close);
                    ctx.append(node);
                }
            }
            return;
        }
        "em" | "i" => {
            gen_inline_emph(ctx, dom, idx, &class, node, false);
            return;
        }
        "strong" | "b" => {
            gen_inline_emph(ctx, dom, idx, &class, node, true);
            return;
        }
        "del" | "s" | "strike" => {
            gen_inline_del(ctx, dom, idx, node);
            return;
        }
        "mark" => {
            gen_inline_mark(ctx, dom, idx, node);
            return;
        }
        "code" => {
            let Some(first) = first_child(dom, idx) else {
                return;
            };
            let content_str = dom.nodes[first].data.replace(editor::ZWSP, "");
            if editor::CARET == content_str {
                node.set_tokens(editor::CARET.as_bytes().to_vec());
                ctx.append(node);
                return;
            }
            if content_str.is_empty() {
                return;
            }
            let mut code_tokens = content_str.into_bytes();
            if "html-inline" == data_type {
                // 所见即所得行级 HTML 解析
                node.set_typ(NodeType::NodeInlineHTML);
                node.set_tokens(code_tokens);
                ctx.append(node);
                return;
            }

            let mut marker = attr(dom, idx, "data-marker");
            if marker.is_empty() {
                marker = "\u{0060}".to_string();
            }
            if code_tokens.starts_with(&[0x60u8]) {
                let mut padded = Vec::new();
                padded.push(b' ');
                padded.extend_from_slice(&code_tokens);
                padded.push(b' ');
                code_tokens = padded;
            }
            node.set_typ(NodeType::NodeCodeSpan);
            node.data_mut().code_marker_len = marker.len() as i32;
            let open = ctx.new_node_raw(NodeType::NodeCodeSpanOpenMarker);
            node.append_child(open);
            let content = ctx.new_node_raw(NodeType::NodeCodeSpanContent);
            content.set_tokens(code_tokens);
            node.append_child(content);
            let close = ctx.new_node_raw(NodeType::NodeCodeSpanCloseMarker);
            node.append_child(close);
            ctx.append(node);
            return;
        }
        "br" => {
            if let Some(parent) = dom.nodes[idx].parent {
                if dom.parent_is(idx, &["td", "th"]) {
                    let prev_ok = prev_sibling(dom, idx)
                        .map(|p| editor::CARET == dom.nodes[p].data)
                        .unwrap_or(true);
                    let next_ok = dom
                        .next_sibling(idx)
                        .map(|p| editor::CARET == dom.nodes[p].data)
                        .unwrap_or(true);
                    if prev_ok && next_ok {
                        return;
                    }
                    if dom.next_sibling(idx).is_none() {
                        return; // 删掉表格中结尾的 br
                    }
                    node.set_typ(NodeType::NodeInlineHTML);
                    node.set_tokens(b"<br />".to_vec());
                    ctx.append(node);
                    return;
                }
                if is_tag(dom, parent, "p") {
                    if let Some(next) = dom.next_sibling(parent) {
                        if is_tag(dom, next, "ul")
                            || is_tag(dom, next, "ol")
                            || is_tag(dom, next, "blockquote")
                        {
                            ctx.append_text(editor::ZWSP.as_bytes());
                            return;
                        }
                    }
                    if let Some(grandparent) = dom.nodes[parent].parent {
                        if let Some(next) = dom.next_sibling(grandparent) {
                            if is_tag(dom, next, "li") {
                                ctx.append_text(editor::ZWSP.as_bytes());
                                return;
                            }
                        }
                    }
                }
            }
            node.set_typ(NodeType::NodeHardBreak);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "a" => {
            if first_child(dom, idx).is_none()
                || dom
                    .nodes
                    .get(first_child(dom, idx).unwrap())
                    .map(|c| c.typ == DomType::Text)
                    .unwrap_or(false)
            {
                let text = dom_text(dom, idx);
                if text.is_empty() || editor::ZWSP == text {
                    return;
                }
            }
            node.set_typ(NodeType::NodeLink);
            let open = ctx.new_node_raw(NodeType::NodeOpenBracket);
            node.append_child(open);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            let close = ctx.new_node_raw(NodeType::NodeCloseBracket);
            node.append_child(close);
            let open_paren = ctx.new_node_raw(NodeType::NodeOpenParen);
            node.append_child(open_paren);
            let href = attr(dom, idx, "href");
            let href = super::trim_link_path(&href, &ctx.render_options);
            let dest = ctx.new_node_raw(NodeType::NodeLinkDest);
            dest.set_tokens(href.as_bytes().to_vec());
            node.append_child(dest);
            let link_title = attr(dom, idx, "title");
            if !link_title.is_empty() {
                let space = ctx.new_node_raw(NodeType::NodeLinkSpace);
                node.append_child(space);
                let title = ctx.new_node_raw(NodeType::NodeLinkTitle);
                title.set_tokens(link_title.into_bytes());
                node.append_child(title);
            }
            let close_paren = ctx.new_node_raw(NodeType::NodeCloseParen);
            node.append_child(close_paren);
            ctx.tip = old;
            return;
        }
        "img" => {
            let img_class = class.clone();
            let img_alt = attr(dom, idx, "alt");
            if "emoji" == img_class {
                node.set_typ(NodeType::NodeEmoji);
                let src = attr(dom, idx, "src");
                let emoji_img = ctx.new_node_raw(NodeType::NodeEmojiImg);
                emoji_img.set_tokens(
                    format!(
                        "<img alt=\"{}\" class=\"emoji\" src=\"{}\" title=\"{}\" />",
                        img_alt, src, img_alt
                    )
                    .into_bytes(),
                );
                let alias = ctx.new_node_raw(NodeType::NodeEmojiAlias);
                alias.set_tokens(format!(":{}:", img_alt).into_bytes());
                emoji_img.append_child(alias);
                node.append_child(emoji_img);
            } else {
                if "link-ref" == data_type {
                    node.set_typ(NodeType::NodeText);
                    let content = format!(
                        "![{}][{}]",
                        attr(dom, idx, "alt"),
                        attr(dom, idx, "data-link-label")
                    );
                    node.set_tokens(content.into_bytes());
                    ctx.append(node);
                    return;
                }
                node.set_typ(NodeType::NodeImage);
                let bang = ctx.new_node_raw(NodeType::NodeBang);
                node.append_child(bang);
                let open = ctx.new_node_raw(NodeType::NodeOpenBracket);
                node.append_child(open);
                if !img_alt.is_empty() {
                    let text = ctx.new_node_raw(NodeType::NodeLinkText);
                    text.set_tokens(img_alt.into_bytes());
                    node.append_child(text);
                }
                let close = ctx.new_node_raw(NodeType::NodeCloseBracket);
                node.append_child(close);
                let open_paren = ctx.new_node_raw(NodeType::NodeOpenParen);
                node.append_child(open_paren);
                let src = attr(dom, idx, "src");
                let src = super::trim_link_path(&src, &ctx.render_options);
                let dest = ctx.new_node_raw(NodeType::NodeLinkDest);
                dest.set_tokens(src.as_bytes().to_vec());
                node.append_child(dest);
                let link_title = attr(dom, idx, "title");
                if !link_title.is_empty() {
                    let space = ctx.new_node_raw(NodeType::NodeLinkSpace);
                    node.append_child(space);
                    let title = ctx.new_node_raw(NodeType::NodeLinkTitle);
                    title.set_tokens(link_title.into_bytes());
                    node.append_child(title);
                }
                let close_paren = ctx.new_node_raw(NodeType::NodeCloseParen);
                node.append_child(close_paren);
            }
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "input" => {
            let Some(parent) = dom.nodes[idx].parent else {
                return;
            };
            if dom.nodes[parent].parent.is_none()
                || (!is_tag(dom, parent, "p") && !is_tag(dom, parent, "li"))
            {
                // 仅允许 input 出现在任务列表中
                return;
            }
            if let Some(next) = dom.next_sibling(idx) {
                if is_tag(dom, next, "span") {
                    // 在任务列表前退格
                    if let Some(&first) = dom.nodes[next].children.first() {
                        dom.nodes[first].data = dom.nodes[first].data.trim().to_string();
                    }
                    return;
                }
            }
            node.set_typ(NodeType::NodeTaskListItemMarker);
            let data_task = attr(dom, idx, "data-task");
            let checked = has_attr(dom, idx, "checked");
            let marker = if 1 == data_task.len() {
                data_task.as_bytes()[0]
            } else if checked {
                b'X'
            } else {
                b' '
            };
            node.revive_from_marker(marker);
            ctx.append(node);
            if let Some(p) = node.parent() {
                if let Some(gp) = p.parent() {
                    if let Some(ld) = gp.data_mut().list_data.as_mut() {
                        ld.typ = 3;
                    }
                }
                if let Some(gp) = p.parent() {
                    if let Some(ggp) = gp.parent() {
                        if let Some(ld) = ggp.data_mut().list_data.as_mut() {
                            ld.typ = 3;
                        }
                    }
                }
            }
        }
        "table" => {
            node.set_typ(NodeType::NodeTable);
            let Some(thead) = first_child(dom, idx) else {
                return;
            };
            let Some(row) = first_child(dom, thead) else {
                return;
            };
            let mut aligns = Vec::new();
            let mut th = first_child(dom, row);
            while let Some(cur) = th {
                aligns.push(match attr(dom, cur, "align").as_str() {
                    "left" => 1,
                    "center" => 2,
                    "right" => 3,
                    _ => 0,
                });
                th = dom.next_sibling(cur);
            }
            node.data_mut().table_aligns = aligns;
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "thead" => {
            node.set_typ(NodeType::NodeTableHead);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "tbody" => {
            gen_children(ctx, dom, idx, gen_ast);
            return;
        }
        "tr" => {
            node.set_typ(NodeType::NodeTableRow);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "th" | "td" => {
            node.set_typ(NodeType::NodeTableCell);
            node.data_mut().table_cell_align = match attr(dom, idx, "align").as_str() {
                "left" => 1,
                "center" => 2,
                "right" => 3,
                _ => 0,
            };
            node.set_tokens(Vec::new());
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast);
            ctx.tip = old;
            return;
        }
        "sup" => {
            if first_child(dom, idx).is_none() {
                return;
            }
            if "footnotes-ref" == data_type {
                node.set_typ(NodeType::NodeText);
                let label = attr(dom, idx, "data-footnotes-label");
                let mut tokens = format!("[{}]", label).into_bytes();
                if let Some(&first) = dom.nodes[idx].children.first() {
                    if dom.nodes[first].data.contains(editor::CARET) {
                        tokens.extend_from_slice(editor::CARET.as_bytes());
                    }
                }
                node.set_tokens(tokens);
                ctx.append(node);
                return;
            }
            gen_inline_sup(ctx, dom, idx, node);
            return;
        }
        "sub" => {
            gen_inline_sub(ctx, dom, idx, node);
            return;
        }
        "span" => {
            if first_child(dom, idx).is_none() {
                return;
            }

            if class.contains("vditor-comment") {
                node.set_typ(NodeType::NodeInlineHTML);
                let mut buf = String::from("<span ");
                for (i, (k, v)) in dom.nodes[idx].attrs.iter().enumerate() {
                    buf.push_str(k);
                    if !v.is_empty() {
                        buf.push_str("=\"");
                        buf.push_str(v);
                        buf.push('"');
                    }
                    if i + 1 < dom.nodes[idx].attrs.len() {
                        buf.push(' ');
                    }
                }
                buf.push('>');
                node.set_tokens(buf.into_bytes());
                ctx.append(node);
                // 继续处理子节点，结尾补 </span>
                let old = ctx.tip;
                ctx.tip = node;
                gen_children(ctx, dom, idx, gen_ast);
                ctx.tip = old;
                let close = ctx.new_node_raw(NodeType::NodeInlineHTML);
                close.set_tokens(b"</span>".to_vec());
                ctx.append(close);
                return;
            }

            if "link-ref" == data_type {
                node.set_typ(NodeType::NodeText);
                let first_data = first_child(dom, idx)
                    .map(|c| dom.nodes[c].data.clone())
                    .unwrap_or_default();
                let mut content =
                    format!("[{}][{}]", first_data, attr(dom, idx, "data-link-label"));
                if let Some(next) = dom.next_sibling(idx) {
                    if "2" == attr(dom, next, "data-render") {
                        // 图片引用风格 ![text][label]
                        content = format!("!{}", content);
                    }
                }
                node.set_tokens(content.into_bytes());
                ctx.append(node);
                return;
            }

            let mut code_tokens: Vec<u8> = Vec::new();
            if let Some(&first) = dom.nodes[idx].children.first() {
                if editor::ZWSP == dom.nodes[first].data && attr(dom, idx, "style").is_empty() {
                    if let Some(next) = dom.next_sibling(first) {
                        if let Some(&fc) = dom.nodes[next].children.first() {
                            code_tokens = dom.nodes[fc].data.as_bytes().to_vec();
                        }
                    }
                } else if is_tag(dom, first, "code") {
                    if let Some(&fc) = dom.nodes[first].children.first() {
                        code_tokens = dom.nodes[fc].data.as_bytes().to_vec();
                        if editor::ZWSP == String::from_utf8_lossy(&code_tokens) {
                            return;
                        }
                    }
                }
            }
            if "math-inline" == data_type {
                node.set_typ(NodeType::NodeInlineMath);
                let open = ctx.new_node_raw(NodeType::NodeInlineMathOpenMarker);
                node.append_child(open);
                let content = ctx.new_node_raw(NodeType::NodeInlineMathContent);
                content.set_tokens(code_tokens);
                node.append_child(content);
                let close = ctx.new_node_raw(NodeType::NodeInlineMathCloseMarker);
                node.append_child(close);
                ctx.append(node);
                return;
            } else if "html-inline" == data_type {
                node.set_typ(NodeType::NodeInlineHTML);
                node.set_tokens(code_tokens);
                ctx.append(node);
                return;
            } else if "code-inline" == data_type {
                node.set_tokens(code_tokens);
                ctx.append(node);
                return;
            } else if "html-entity" == data_type {
                node.set_typ(NodeType::NodeText);
                node.set_tokens(code_tokens);
                ctx.append(node);
                return;
            }
            // 其他 span：继续处理子节点
            gen_children(ctx, dom, idx, gen_ast);
            return;
        }
        "font" => {
            node.set_typ(NodeType::NodeText);
            node.set_tokens(dom_text(dom, idx).into_bytes());
            ctx.append(node);
            return;
        }
        "details" => {
            node.set_typ(NodeType::NodeHTMLBlock);
            let html = dom.html(idx);
            let tokens = match html.find("</summary>") {
                Some(i) => html.as_bytes()[..i + "</summary>".len()].to_vec(),
                None => html.as_bytes().to_vec(),
            };
            node.set_tokens(tokens);
            ctx.append(node);
            gen_children(ctx, dom, idx, gen_ast);
            let close = ctx.new_node_raw(NodeType::NodeHTMLBlock);
            close.set_tokens(b"</details>".to_vec());
            ctx.append(close);
            return;
        }
        "kbd" => {
            node.set_typ(NodeType::NodeInlineHTML);
            node.set_tokens(dom.html(idx).into_bytes());
            ctx.append(node);
            return;
        }
        "summary" => {
            return;
        }
        _ => {
            node.set_typ(NodeType::NodeHTMLBlock);
            node.set_tokens(dom.html(idx).into_bytes());
            ctx.append(node);
            return;
        }
    }

    gen_children(ctx, dom, idx, gen_ast);
}

fn gen_inline_emph(
    ctx: &mut Ctx,
    dom: &mut Dom,
    idx: usize,
    _class: &str,
    node: Node,
    strong: bool,
) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }

    node.set_typ(if strong {
        NodeType::NodeStrong
    } else {
        NodeType::NodeEmphasis
    });
    let mut marker = attr(dom, idx, "data-marker");
    if marker.is_empty() {
        marker = if strong { "**" } else { "*" }.to_string();
    }
    let open_typ = if strong {
        if "__" == marker {
            NodeType::NodeStrongU8eOpenMarker
        } else {
            NodeType::NodeStrongA6kOpenMarker
        }
    } else if "_" == marker {
        NodeType::NodeEmU8eOpenMarker
    } else {
        NodeType::NodeEmA6kOpenMarker
    };
    let open = ctx.new_node_raw(open_typ);
    open.set_tokens(marker.as_bytes().to_vec());
    node.append_child(open);
    ctx.append(node);

    let last = last_child(dom, idx);
    if editor::CARET == dom.nodes[first].data && last.map(|l| is_tag(dom, l, "br")).unwrap_or(false)
    {
        // 处理结尾换行
        let text_node = ctx.new_node_raw(NodeType::NodeText);
        text_node.set_tokens(editor::CARET.as_bytes().to_vec());
        node.append_child(text_node);
        let close_typ = if strong {
            if "__" == marker {
                NodeType::NodeStrongU8eCloseMarker
            } else {
                NodeType::NodeStrongA6kCloseMarker
            }
        } else if "_" == marker {
            NodeType::NodeEmU8eCloseMarker
        } else {
            NodeType::NodeEmA6kCloseMarker
        };
        let close = ctx.new_node_raw(close_typ);
        close.set_tokens(marker.as_bytes().to_vec());
        node.append_child(close);
        return;
    }

    normalize_inline_boundary(ctx, dom, idx, node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast);
    ctx.tip = old;

    let close_typ = if strong {
        if "__" == marker {
            NodeType::NodeStrongU8eCloseMarker
        } else {
            NodeType::NodeStrongA6kCloseMarker
        }
    } else if "_" == marker {
        NodeType::NodeEmU8eCloseMarker
    } else {
        NodeType::NodeEmA6kCloseMarker
    };
    let close = ctx.new_node_raw(close_typ);
    close.set_tokens(marker.as_bytes().to_vec());
    node.append_child(close);
}

fn gen_inline_del(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }
    node.set_typ(NodeType::NodeStrikethrough);
    let marker = attr(dom, idx, "data-marker");
    let open_typ = if "~" == marker {
        NodeType::NodeStrikethrough1OpenMarker
    } else {
        NodeType::NodeStrikethrough2OpenMarker
    };
    let open = ctx.new_node_raw(open_typ);
    open.set_tokens(marker.as_bytes().to_vec());
    node.append_child(open);
    ctx.append(node);

    let last = last_child(dom, idx);
    if editor::CARET == dom.nodes[first].data && last.map(|l| is_tag(dom, l, "br")).unwrap_or(false)
    {
        let text_node = ctx.new_node_raw(NodeType::NodeText);
        text_node.set_tokens(editor::CARET.as_bytes().to_vec());
        node.append_child(text_node);
        let close_typ = if "~" == marker {
            NodeType::NodeStrikethrough1CloseMarker
        } else {
            NodeType::NodeStrikethrough2CloseMarker
        };
        let close = ctx.new_node_raw(close_typ);
        close.set_tokens(marker.as_bytes().to_vec());
        node.append_child(close);
        return;
    }

    normalize_inline_boundary(ctx, dom, idx, node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast);
    ctx.tip = old;

    let close_typ = if "~" == marker {
        NodeType::NodeStrikethrough1CloseMarker
    } else {
        NodeType::NodeStrikethrough2CloseMarker
    };
    let close = ctx.new_node_raw(close_typ);
    close.set_tokens(marker.as_bytes().to_vec());
    node.append_child(close);
}

fn gen_inline_mark(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }
    node.set_typ(NodeType::NodeMark);
    let marker = attr(dom, idx, "data-marker");
    let open_typ = if "=" == marker {
        NodeType::NodeMark1OpenMarker
    } else {
        NodeType::NodeMark2OpenMarker
    };
    let open = ctx.new_node_raw(open_typ);
    open.set_tokens(marker.as_bytes().to_vec());
    node.append_child(open);
    ctx.append(node);

    let last = last_child(dom, idx);
    if editor::CARET == dom.nodes[first].data && last.map(|l| is_tag(dom, l, "br")).unwrap_or(false)
    {
        let text_node = ctx.new_node_raw(NodeType::NodeText);
        text_node.set_tokens(editor::CARET.as_bytes().to_vec());
        node.append_child(text_node);
        let close_typ = if "=" == marker {
            NodeType::NodeMark1CloseMarker
        } else {
            NodeType::NodeMark2CloseMarker
        };
        let close = ctx.new_node_raw(close_typ);
        close.set_tokens(marker.as_bytes().to_vec());
        node.append_child(close);
        return;
    }

    normalize_inline_boundary(ctx, dom, idx, node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast);
    ctx.tip = old;

    let close_typ = if "=" == marker {
        NodeType::NodeMark1CloseMarker
    } else {
        NodeType::NodeMark2CloseMarker
    };
    let close = ctx.new_node_raw(close_typ);
    close.set_tokens(marker.as_bytes().to_vec());
    node.append_child(close);
}

fn gen_inline_sup(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }
    node.set_typ(NodeType::NodeSup);
    let mut marker = attr(dom, idx, "data-marker");
    if marker.is_empty() {
        marker = "^".to_string();
    }
    let open = ctx.new_node_raw(NodeType::NodeSupOpenMarker);
    open.set_tokens(marker.as_bytes().to_vec());
    node.append_child(open);
    ctx.append(node);

    let last = last_child(dom, idx);
    if editor::CARET == dom.nodes[first].data && last.map(|l| is_tag(dom, l, "br")).unwrap_or(false)
    {
        let text_node = ctx.new_node_raw(NodeType::NodeText);
        text_node.set_tokens(editor::CARET.as_bytes().to_vec());
        node.append_child(text_node);
        let close = ctx.new_node_raw(NodeType::NodeSupCloseMarker);
        close.set_tokens(marker.as_bytes().to_vec());
        node.append_child(close);
        return;
    }

    normalize_inline_boundary(ctx, dom, idx, node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast);
    ctx.tip = old;

    let close = ctx.new_node_raw(NodeType::NodeSupCloseMarker);
    close.set_tokens(marker.as_bytes().to_vec());
    node.append_child(close);
}

fn gen_inline_sub(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }
    node.set_typ(NodeType::NodeSub);
    let mut marker = attr(dom, idx, "data-marker");
    if marker.is_empty() {
        marker = "~".to_string();
    }
    let open = ctx.new_node_raw(NodeType::NodeSubOpenMarker);
    open.set_tokens(marker.as_bytes().to_vec());
    node.append_child(open);
    ctx.append(node);

    let last = last_child(dom, idx);
    if editor::CARET == dom.nodes[first].data && last.map(|l| is_tag(dom, l, "br")).unwrap_or(false)
    {
        let text_node = ctx.new_node_raw(NodeType::NodeText);
        text_node.set_tokens(editor::CARET.as_bytes().to_vec());
        node.append_child(text_node);
        let close = ctx.new_node_raw(NodeType::NodeSubCloseMarker);
        close.set_tokens(marker.as_bytes().to_vec());
        node.append_child(close);
        return;
    }

    normalize_inline_boundary(ctx, dom, idx, node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast);
    ctx.tip = old;

    let close = ctx.new_node_raw(NodeType::NodeSubCloseMarker);
    close.set_tokens(marker.as_bytes().to_vec());
    node.append_child(close);
}

pub(crate) fn gen_ast_ir(ctx: &mut Ctx, dom: &mut Dom, idx: usize) {
    let data_render = attr(dom, idx, "data-render");
    if "1" == data_render || "2" == data_render {
        return;
    }

    let data_type = attr(dom, idx, "data-type");
    let tag = if dom.nodes[idx].typ == DomType::Text {
        String::new()
    } else {
        dom.nodes[idx].data.clone()
    };

    if "div" == tag {
        if "code-block" == data_type
            || "html-block" == data_type
            || "math-block" == data_type
            || "yaml-front-matter" == data_type
        {
            if ("code-block" == data_type || "math-block" == data_type)
                && !attr(dom, first_child(dom, idx).unwrap_or(idx), "data-type")
                    .contains("-block-open-marker")
            {
                // 处理在结尾 ``` 或者 $$ 后换行的情况
                let p = ctx.new_node_raw(NodeType::NodeParagraph);
                let text = ctx.new_node_raw(NodeType::NodeText);
                text.set_tokens(dom_text(dom, first_child(dom, idx).unwrap_or(idx)).into_bytes());
                p.append_child(text);
                ctx.append(p);
                ctx.tip = p;
                return;
            }
            gen_children(ctx, dom, idx, gen_ast_ir);
        } else if "link-ref-defs-block" == data_type {
            let text = dom_text(dom, idx);
            ctx.append_text(text.as_bytes());
        } else if "footnotes-def" == data_type {
            gen_children(ctx, dom, idx, gen_ast_ir);
        } else if "footnotes-block" == data_type {
            for &def in &dom.nodes[idx].children {
                let original_html = dom.html(def);
                let md = super::dom2md_with_options(
                    &original_html,
                    &ctx.parse_options,
                    &ctx.render_options,
                    true,
                );
                let mut lines: Vec<&str> = md.split('\n').collect();
                if lines.last() == Some(&"") {
                    lines.pop();
                }
                let mut out = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if 0 < i {
                        out.push_str("    ");
                        out.push_str(line);
                    } else {
                        out.push_str(line);
                    }
                    out.push('\n');
                }
                ctx.append_text(out.as_bytes());
            }
        } else if "toc-block" == data_type {
            let node = ctx.new_node_raw(NodeType::NodeToC);
            ctx.append(node);
        } else {
            let text = dom_text(dom, idx);
            if format!("{}\n", editor::CARET) == text {
                // 处理 FireFox 某些情况下产生的分段
                ctx.append_text(format!("{}\n", editor::CARET).as_bytes());
            }
        }
        return;
    }

    let class = attr(dom, idx, "class");
    let content = dom.nodes[idx].data.replace(editor::ZWSP, "");
    let node = ctx.new_node_raw(NodeType::NodeText);
    node.set_tokens(content.as_bytes().to_vec());

    match tag.as_str() {
        "" => {
            if content.is_empty() {
                return;
            }
            let check_indent_code_block = content.replace(editor::CARET, "").replace('\t', "    ");
            if !is_inline(dom, prev_sibling(dom, idx))
                && check_indent_code_block.starts_with("    ")
            {
                node.set_typ(NodeType::NodeCodeBlock);
                node.data_mut().is_fenced_code_block = true;
                let open = ctx.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                open.set_tokens("\u{0060}\u{0060}\u{0060}".as_bytes().to_vec());
                open.data_mut().code_block_fence_len = 3;
                node.append_child(open);
                let info = ctx.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                node.append_child(info);
                let start_caret = content.starts_with(editor::CARET);
                let mut content = content.clone();
                if start_caret {
                    content = content.replace(editor::CARET, "");
                }
                content = content.trim().to_string();
                if start_caret {
                    content = format!("{}{}", editor::CARET, content);
                }
                let code = ctx.new_node_raw(NodeType::NodeCodeBlockCode);
                code.set_tokens(content.as_bytes().to_vec());
                node.append_child(code);
                let close = ctx.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                close.set_tokens("\u{0060}\u{0060}\u{0060}".as_bytes().to_vec());
                close.data_mut().code_block_fence_len = 3;
                node.append_child(close);
                ctx.append(node);
                return;
            }
            if let Some(parent) = dom.nodes[idx].parent {
                if is_tag(dom, parent, "a") {
                    node.set_typ(NodeType::NodeLinkText);
                }
            }
            ctx.append(node);
        }
        "p" | "div" => {
            node.set_typ(NodeType::NodeParagraph);
            let text = dom_text(dom, idx);
            if "\n" == text
                && NodeType::NodeBlockquote == ctx.tip.typ()
                && ctx.tip.first_child().and_then(|c| c.next()).is_none()
            {
                // 不允许在 bq 第一个节点前换行
                return;
            }
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let text = dom_text(dom, idx);
            if text.trim().is_empty() {
                return;
            }
            node.set_typ(NodeType::NodeHeading);
            let marker = attr(dom, idx, "data-marker");
            node.data_mut().heading_setext = "=" == marker || "-" == marker;
            if !node.data().heading_setext {
                let first_text = first_child(dom, idx)
                    .map(|c| dom_text(dom, c))
                    .unwrap_or_default();
                node.data_mut().heading_level = first_text.matches('#').count() as i32;
            } else {
                let last_text = last_child(dom, idx)
                    .map(|c| dom_text(dom, c))
                    .unwrap_or_default();
                if last_text.replace(editor::CARET, "").trim().is_empty() {
                    node.set_typ(NodeType::NodeText);
                    node.set_tokens(text.as_bytes().to_vec());
                    ctx.append(node);
                    let old = ctx.tip;
                    ctx.tip = node;
                    gen_children(ctx, dom, idx, gen_ast_ir);
                    ctx.tip = old;
                    return;
                }
                node.data_mut().heading_level = if "=" == marker { 1 } else { 2 };
                if let Some(previous) =
                    last_child(dom, idx).and_then(|last| prev_sibling(dom, last))
                {
                    if let Some(text) = dom.nodes[previous].data.strip_suffix('\n') {
                        dom.nodes[previous].data = text.to_owned();
                    }
                }
            }
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "hr" => {
            node.set_typ(NodeType::NodeThematicBreak);
            ctx.append(node);
        }
        "blockquote" => {
            let content = dom_text(dom, idx).trim().to_string();
            if content.is_empty() || "&gt;" == content {
                return;
            }
            if editor::CARET == content {
                node.set_typ(NodeType::NodeText);
                node.set_tokens(content.as_bytes().to_vec());
                ctx.append(node);
            }
            node.set_typ(NodeType::NodeBlockquote);
            let marker = ctx.new_node_raw(NodeType::NodeBlockquoteMarker);
            marker.set_tokens(b">".to_vec());
            node.append_child(marker);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "ol" | "ul" => {
            if first_child(dom, idx).is_none() {
                return;
            }
            node.set_typ(NodeType::NodeList);
            let mut ld = crate::ast::ListData::default();
            if "ol" == tag {
                ld.typ = 1;
            }
            let tight = attr(dom, idx, "data-tight");
            if "true" == tight || tight.is_empty() {
                ld.tight = true;
            }
            node.data_mut().list_data = Some(ld);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "li" => {
            if let Some(p) = first_child(dom, idx) {
                if is_tag(dom, p, "p") {
                    if let Some(next) = dom.next_sibling(p) {
                        if is_tag(dom, next, "p") {
                            if let Some(ld) = ctx.tip.data_mut().list_data.as_mut() {
                                ld.tight = false;
                            }
                        }
                    }
                }
            }
            node.set_typ(NodeType::NodeListItem);
            let mut marker = attr(dom, idx, "data-marker");
            let mut bullet = 0u8;
            if marker.is_empty() {
                if let Some(parent) = dom.nodes[idx].parent {
                    if is_tag(dom, parent, "ol") {
                        let first_li_marker = first_child(dom, parent)
                            .map(|c| attr(dom, c, "data-marker"))
                            .unwrap_or_default();
                        let start_attr = attr(dom, parent, "start");
                        marker = if start_attr.is_empty() {
                            "1".to_string()
                        } else {
                            start_attr
                        };
                        if !first_li_marker.is_empty() {
                            marker.push(first_li_marker.chars().last().unwrap_or('.'));
                        } else {
                            marker.push('.');
                        }
                    } else {
                        marker = attr(dom, parent, "data-marker");
                        if marker.is_empty() {
                            marker = "*".to_string();
                        }
                        bullet = marker.as_bytes()[0];
                    }
                } else {
                    marker = attr(dom, idx, "data-marker");
                    if marker.is_empty() {
                        marker = "*".to_string();
                    }
                    bullet = marker.as_bytes()[0];
                }
            } else if let Some(parent) = dom.nodes[idx].parent {
                if is_tag(dom, parent, "ol") {
                    if "*" == marker || "-" == marker || "+" == marker {
                        marker = "1.".to_string();
                    }
                    if "1." != marker
                        && "1)" != marker
                        && dom
                            .prev_sibling(idx)
                            .map(|p| !is_tag(dom, p, "li"))
                            .unwrap_or(false)
                    {
                        if let Some(grandparent) = dom.nodes[parent].parent {
                            if is_tag(dom, grandparent, "ol") || is_tag(dom, grandparent, "ul") {
                                marker = "1.".to_string();
                            }
                        }
                    }
                    if "1." != marker
                        && "1)" != marker
                        && first_child(dom, parent) == Some(idx)
                        && attr(dom, parent, "start").is_empty()
                    {
                        marker = "1.".to_string();
                    }
                } else {
                    if "*" != marker && "-" != marker && "+" != marker {
                        marker = "*".to_string();
                    }
                    bullet = marker.as_bytes()[0];
                }
            } else {
                marker = attr(dom, idx, "data-marker");
                if marker.is_empty() {
                    marker = "*".to_string();
                }
                bullet = marker.as_bytes()[0];
            }
            let mut ld = crate::ast::ListData {
                marker: marker.as_bytes().to_vec(),
                bullet_char: bullet,
                ..Default::default()
            };
            if 0 == bullet {
                let num_part = &marker[..marker.len().saturating_sub(1)];
                ld.num = num_part.parse::<i32>().unwrap_or(0);
                ld.delimiter = *marker.as_bytes().last().unwrap_or(&b'.');
            }
            node.data_mut().list_data = Some(ld);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "pre" => {
            let Some(code) = first_child(dom, idx) else {
                return;
            };
            if !is_tag(dom, code, "code") {
                return;
            }
            let mut code_tokens = Vec::new();
            if let Some(&text) = dom.nodes[code].children.first() {
                code_tokens = dom.nodes[text].data.as_bytes().to_vec();
            }
            let parent = dom.nodes[idx].parent.unwrap_or(idx);
            match attr(dom, parent, "data-type").as_str() {
                "math-block" => {
                    node.set_typ(NodeType::NodeMathBlockContent);
                    node.set_tokens(code_tokens);
                    ctx.append(node);
                }
                "html-block" => {
                    node.set_typ(NodeType::NodeHTMLBlock);
                    node.set_tokens(code_tokens);
                    ctx.append(node);
                }
                "yaml-front-matter" => {
                    node.set_typ(NodeType::NodeYamlFrontMatter);
                    let content = ctx.new_node_raw(NodeType::NodeYamlFrontMatterContent);
                    content.set_tokens(code_tokens);
                    node.append_child(content);
                    ctx.append(node);
                }
                _ => {
                    node.set_typ(NodeType::NodeCodeBlockCode);
                    node.set_tokens(code_tokens);
                    ctx.append(node);
                }
            }
            return;
        }
        "em" | "i" => {
            gen_ir_emph(ctx, dom, idx, node, NodeType::NodeEmphasis);
            return;
        }
        "strong" | "b" => {
            gen_ir_emph(ctx, dom, idx, node, NodeType::NodeStrong);
            return;
        }
        "del" | "s" | "strike" => {
            gen_ir_emph(ctx, dom, idx, node, NodeType::NodeStrikethrough);
            return;
        }
        "mark" => {
            gen_ir_emph(ctx, dom, idx, node, NodeType::NodeMark);
            return;
        }
        "code" => {
            let Some(first) = first_child(dom, idx) else {
                return;
            };
            let content_str = dom.nodes[first].data.replace(editor::ZWSP, "");
            if editor::CARET == content_str {
                node.set_tokens(editor::CARET.as_bytes().to_vec());
                ctx.append(node);
                return;
            }
            if content_str.is_empty() {
                return;
            }
            let content = ctx.new_node_raw(NodeType::NodeCodeSpanContent);
            content.set_tokens(content_str.into_bytes());
            node.set_typ(NodeType::NodeCodeSpan);
            node.append_child(content);
            ctx.append(node);
            return;
        }
        "br" => {
            if let Some(parent) = dom.nodes[idx].parent {
                if dom.parent_is(idx, &["td", "th"]) {
                    let prev_ok = prev_sibling(dom, idx)
                        .map(|p| editor::CARET == dom.nodes[p].data)
                        .unwrap_or(true);
                    let next_ok = dom
                        .next_sibling(idx)
                        .map(|p| editor::CARET == dom.nodes[p].data)
                        .unwrap_or(true);
                    if prev_ok && next_ok {
                        return;
                    }
                    if dom.next_sibling(idx).is_none() {
                        return;
                    }
                    node.set_typ(NodeType::NodeInlineHTML);
                    node.set_tokens(b"<br />".to_vec());
                    ctx.append(node);
                    return;
                }
                if is_tag(dom, parent, "p") {
                    if let Some(next) = dom.next_sibling(parent) {
                        if is_tag(dom, next, "ul")
                            || is_tag(dom, next, "ol")
                            || is_tag(dom, next, "blockquote")
                        {
                            ctx.append_text(editor::ZWSP.as_bytes());
                            return;
                        }
                    }
                }
            }
            node.set_typ(NodeType::NodeHardBreak);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "a" => {
            node.set_typ(NodeType::NodeLink);
            let open = ctx.new_node_raw(NodeType::NodeOpenBracket);
            node.append_child(open);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            let close = ctx.new_node_raw(NodeType::NodeCloseBracket);
            node.append_child(close);
            let open_paren = ctx.new_node_raw(NodeType::NodeOpenParen);
            node.append_child(open_paren);
            let href = attr(dom, idx, "href");
            let href = super::trim_link_path(&href, &ctx.render_options);
            let dest = ctx.new_node_raw(NodeType::NodeLinkDest);
            dest.set_tokens(href.as_bytes().to_vec());
            node.append_child(dest);
            let link_title = attr(dom, idx, "title");
            if !link_title.is_empty() {
                let space = ctx.new_node_raw(NodeType::NodeLinkSpace);
                node.append_child(space);
                let title = ctx.new_node_raw(NodeType::NodeLinkTitle);
                title.set_tokens(link_title.into_bytes());
                node.append_child(title);
            }
            let close_paren = ctx.new_node_raw(NodeType::NodeCloseParen);
            node.append_child(close_paren);
            ctx.tip = old;
            return;
        }
        "img" => {
            let img_class = class.clone();
            let img_alt = attr(dom, idx, "alt");
            if "emoji" == img_class {
                node.set_typ(NodeType::NodeEmoji);
                let src = attr(dom, idx, "src");
                let emoji_img = ctx.new_node_raw(NodeType::NodeEmojiImg);
                emoji_img.set_tokens(
                    format!(
                        "<img alt=\"{}\" class=\"emoji\" src=\"{}\" title=\"{}\" />",
                        img_alt, src, img_alt
                    )
                    .into_bytes(),
                );
                let alias = ctx.new_node_raw(NodeType::NodeEmojiAlias);
                alias.set_tokens(format!(":{}:", img_alt).into_bytes());
                emoji_img.append_child(alias);
                node.append_child(emoji_img);
                ctx.append(node);
                let old = ctx.tip;
                ctx.tip = node;
                gen_children(ctx, dom, idx, gen_ast_ir);
                ctx.tip = old;
            }
            return;
        }
        "input" => {
            let Some(parent) = dom.nodes[idx].parent else {
                return;
            };
            if dom.nodes[parent].parent.is_none()
                || (!is_tag(dom, parent, "p") && !is_tag(dom, parent, "li"))
            {
                return;
            }
            if let Some(next) = dom.next_sibling(idx) {
                if is_tag(dom, next, "span") {
                    return;
                }
            }
            node.set_typ(NodeType::NodeTaskListItemMarker);
            let data_task = attr(dom, idx, "data-task");
            let checked = has_attr(dom, idx, "checked");
            let marker = if 1 == data_task.len() {
                data_task.as_bytes()[0]
            } else if checked {
                b'X'
            } else {
                b' '
            };
            node.revive_from_marker(marker);
            ctx.append(node);
            if let Some(p) = node.parent() {
                if let Some(gp) = p.parent() {
                    if let Some(ggp) = gp.parent() {
                        if let Some(ld) = ggp.data_mut().list_data.as_mut() {
                            ld.typ = 3;
                        }
                    }
                }
            }
        }
        "table" => {
            node.set_typ(NodeType::NodeTable);
            let Some(thead) = first_child(dom, idx) else {
                return;
            };
            let Some(row) = first_child(dom, thead) else {
                return;
            };
            let mut aligns = Vec::new();
            let mut th = first_child(dom, row);
            while let Some(cur) = th {
                aligns.push(match attr(dom, cur, "align").as_str() {
                    "left" => 1,
                    "center" => 2,
                    "right" => 3,
                    _ => 0,
                });
                th = dom.next_sibling(cur);
            }
            node.data_mut().table_aligns = aligns;
            node.set_tokens(Vec::new());
            // 表格开头输入会导致解析问题，所以插入一个空段落进行分隔
            let p = ctx.new_node_raw(NodeType::NodeParagraph);
            ctx.append(p);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "thead" => {
            node.set_typ(NodeType::NodeTableHead);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "tbody" => {
            gen_children(ctx, dom, idx, gen_ast_ir);
            return;
        }
        "tr" => {
            node.set_typ(NodeType::NodeTableRow);
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "th" | "td" => {
            node.set_typ(NodeType::NodeTableCell);
            node.data_mut().table_cell_align = match attr(dom, idx, "align").as_str() {
                "left" => 1,
                "center" => 2,
                "right" => 3,
                _ => 0,
            };
            node.set_tokens(Vec::new());
            ctx.append(node);
            let old = ctx.tip;
            ctx.tip = node;
            gen_children(ctx, dom, idx, gen_ast_ir);
            ctx.tip = old;
            return;
        }
        "sup" => {
            if first_child(dom, idx).is_none() {
                return;
            }
            if "footnotes-ref" == data_type {
                node.set_typ(NodeType::NodeText);
                node.set_tokens(dom_text(dom, idx).into_bytes());
                ctx.append(node);
            }
            return;
        }
        "span" => {
            match data_type.as_str() {
                "inline-node" | "em" | "strong" | "s" | "sup" | "sub" | "a" | "link-ref"
                | "img" | "code" | "heading-id" | "html-inline" | "inline-math" | "html-entity" => {
                    node.set_typ(NodeType::NodeText);
                    node.set_tokens(dom_text(dom, idx).into_bytes());
                    ctx.append(node);
                    return;
                }
                "math-block-close-marker" => {
                    let close = ctx.new_node_raw(NodeType::NodeMathBlockCloseMarker);
                    close.set_tokens(b"$$".to_vec());
                    ctx.append(close);
                    let parent = ctx.tip.parent();
                    if let Some(p) = parent {
                        ctx.tip = p;
                    }
                    return;
                }
                "math-block-open-marker" => {
                    node.set_typ(NodeType::NodeMathBlock);
                    let open = ctx.new_node_raw(NodeType::NodeMathBlockOpenMarker);
                    open.set_tokens(b"$$".to_vec());
                    node.append_child(open);
                    ctx.append(node);
                    ctx.tip = node;
                    return;
                }
                "yaml-front-matter-close-marker" => {
                    let close = ctx.new_node_raw(NodeType::NodeYamlFrontMatterCloseMarker);
                    close.set_tokens(b"---".to_vec());
                    ctx.append(close);
                    let parent = ctx.tip.parent();
                    if let Some(p) = parent {
                        ctx.tip = p;
                    }
                    return;
                }
                "yaml-front-matter-open-marker" => {
                    node.set_typ(NodeType::NodeYamlFrontMatter);
                    let open = ctx.new_node_raw(NodeType::NodeYamlFrontMatterOpenMarker);
                    open.set_tokens(b"---".to_vec());
                    node.append_child(open);
                    ctx.append(node);
                    ctx.tip = node;
                    return;
                }
                "code-block-open-marker" => {
                    let mut marker = dom_text(dom, idx).into_bytes();
                    let last_backtick = marker
                        .iter()
                        .rposition(|&b| b == 0x60)
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let mut trailing: Vec<u8> = Vec::new();
                    if 0 < last_backtick {
                        trailing = marker[last_backtick..].to_vec();
                        marker.truncate(last_backtick);
                    }
                    node.set_typ(NodeType::NodeCodeBlock);
                    node.data_mut().is_fenced_code_block = true;
                    let open = ctx.new_node_raw(NodeType::NodeCodeBlockFenceOpenMarker);
                    open.set_tokens(marker.clone());
                    open.data_mut().code_block_fence_len = marker.len() as i32;
                    node.append_child(open);
                    ctx.append(node);
                    ctx.tip = node;
                    if !trailing.is_empty() {
                        let info = ctx.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                        info.data_mut().code_block_info = trailing;
                        ctx.append(info);
                    }
                    return;
                }
                "code-block-info" => {
                    let mut info = dom_text(dom, idx).into_bytes();
                    info = replace_all(&info, editor::ZWSP.as_bytes(), b"");
                    let marker = ctx.new_node_raw(NodeType::NodeCodeBlockFenceInfoMarker);
                    marker.data_mut().code_block_info = info;
                    ctx.append(marker);
                    return;
                }
                "code-block-close-marker" => {
                    let mut marker = dom_text(dom, idx).into_bytes();
                    let last_backtick = marker
                        .iter()
                        .rposition(|&b| b == 0x60)
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    if 0 < last_backtick {
                        marker.truncate(last_backtick);
                    }
                    if marker.is_empty() {
                        marker = vec![0x60u8, 0x60, 0x60];
                    }
                    let close = ctx.new_node_raw(NodeType::NodeCodeBlockFenceCloseMarker);
                    close.set_tokens(marker.clone());
                    close.data_mut().code_block_fence_len = marker.len() as i32;
                    ctx.append(close);
                    let parent = ctx.tip.parent();
                    if let Some(p) = parent {
                        ctx.tip = p;
                    }
                    return;
                }
                "heading-marker" => {
                    let text = dom_text(dom, idx);
                    if text.contains(editor::CARET) {
                        let mut rest = text.replace('#', "").replace(editor::CARET, "");
                        rest = rest.trim().to_string();
                        let mut caret = String::new();
                        if !rest.is_empty() {
                            caret.push_str(&rest);
                        }
                        caret.push_str(editor::CARET);
                        ctx.append_text(caret.as_bytes());
                    }
                    return;
                }
                _ => {}
            }

            if first_child(dom, idx).is_none() {
                return;
            }

            let mut code_tokens: Vec<u8> = Vec::new();
            if let Some(&first) = dom.nodes[idx].children.first() {
                if editor::ZWSP == dom.nodes[first].data && attr(dom, idx, "style").is_empty() {
                    if let Some(next) = dom.next_sibling(first) {
                        if let Some(&fc) = dom.nodes[next].children.first() {
                            code_tokens = dom.nodes[fc].data.as_bytes().to_vec();
                        }
                    }
                } else if is_tag(dom, first, "code") {
                    if let Some(&fc) = dom.nodes[first].children.first() {
                        code_tokens = dom.nodes[fc].data.as_bytes().to_vec();
                        if editor::ZWSP == String::from_utf8_lossy(&code_tokens) {
                            return;
                        }
                    }
                } else {
                    gen_children(ctx, dom, idx, gen_ast_ir);
                    return;
                }
            }
            if "math-inline" == data_type {
                node.set_typ(NodeType::NodeInlineMath);
                let open = ctx.new_node_raw(NodeType::NodeInlineMathOpenMarker);
                node.append_child(open);
                let content = ctx.new_node_raw(NodeType::NodeInlineMathContent);
                content.set_tokens(code_tokens);
                node.append_child(content);
                let close = ctx.new_node_raw(NodeType::NodeInlineMathCloseMarker);
                node.append_child(close);
                ctx.append(node);
            } else if "html-inline" == data_type {
                node.set_typ(NodeType::NodeInlineHTML);
                node.set_tokens(code_tokens);
                ctx.append(node);
            } else if "code-inline" == data_type {
                node.set_tokens(code_tokens);
                ctx.append(node);
            } else if "html-entity" == data_type {
                node.set_typ(NodeType::NodeText);
                node.set_tokens(code_tokens);
                ctx.append(node);
            }
            return;
        }
        "font" => {
            node.set_typ(NodeType::NodeText);
            node.set_tokens(dom_text(dom, idx).into_bytes());
            ctx.append(node);
            return;
        }
        "details" => {
            node.set_typ(NodeType::NodeHTMLBlock);
            let html = dom.html(idx);
            let tokens = match html.find("</summary>") {
                Some(i) => html.as_bytes()[..i + "</summary>".len()].to_vec(),
                None => html.as_bytes().to_vec(),
            };
            node.set_tokens(tokens);
            ctx.append(node);
            gen_children(ctx, dom, idx, gen_ast_ir);
            let close = ctx.new_node_raw(NodeType::NodeHTMLBlock);
            close.set_tokens(b"</details>".to_vec());
            ctx.append(close);
            return;
        }
        "kbd" => {
            // kbd 标签由 code 标签构成节点
        }
        "summary" => {
            return;
        }
        _ => {
            node.set_typ(NodeType::NodeHTMLBlock);
            node.set_tokens(dom.html(idx).into_bytes());
            ctx.append(node);
            return;
        }
    }

    gen_children(ctx, dom, idx, gen_ast_ir);
}

fn gen_ir_emph(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node, typ: NodeType) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    if is_tag(dom, first, "br") {
        return;
    }
    if starts_with_newline(dom, first) {
        dom.nodes[first].data = dom.nodes[first]
            .data
            .trim_start_matches(['\u{200b}', '\n'])
            .to_string();
        ctx.append_text(format!("{}\n", editor::ZWSP).as_bytes());
    }
    let text = dom_text(dom, idx).trim().to_string();
    if is_empty_text(dom, idx) {
        return;
    }
    if editor::CARET == text {
        node.set_tokens(editor::CARET.as_bytes().to_vec());
        ctx.append(node);
        return;
    }
    node.set_typ(typ);
    ctx.append(node);
    let old = ctx.tip;
    ctx.tip = node;
    gen_children(ctx, dom, idx, gen_ast_ir);
    ctx.tip = old;
}

// Go DOM generation visits siblings after each recursive call: generation may insert
// whitespace nodes beside an inline element, so a pre-collected child list is stale.
fn gen_children(
    ctx: &mut Ctx,
    dom: &mut Dom,
    parent: usize,
    generate: fn(&mut Ctx, &mut Dom, usize),
) {
    let mut child = first_child(dom, parent);
    while let Some(current) = child {
        generate(ctx, dom, current);
        child = dom.next_sibling(current);
    }
}

fn normalize_inline_boundary(ctx: &mut Ctx, dom: &mut Dom, idx: usize, node: Node) {
    let Some(first) = first_child(dom, idx) else {
        return;
    };
    dom.nodes[first].data = dom.nodes[first].data.replace(editor::ZWSP, "");
    if dom.nodes[first].data.starts_with(' ') {
        dom.nodes[first].data = dom.nodes[first].data.trim_start_matches(' ').to_string();
        let space = ctx.new_node_raw(NodeType::NodeText);
        space.set_tokens(b" ".to_vec());
        node.insert_before(space);
    }
    if dom.next_sibling(first).is_none() {
        for whitespace in [' ', '\n'] {
            if dom.nodes[first].data.ends_with(whitespace) {
                dom.nodes[first].data = dom.nodes[first]
                    .data
                    .trim_end_matches(whitespace)
                    .to_string();
                let trailing = dom.create(DomType::Text, whitespace.to_string());
                dom.insert_after(idx, trailing);
            }
        }
    }
}

#[cfg(test)]
mod source_alignment_tests {
    use super::*;

    fn generate(html: &str, ir: bool) -> crate::ast::Tree {
        let mut dom = crate::dom::parse(html);
        let mut tree = crate::ast::Tree::new();
        let root = tree.root_raw();
        let mut ctx = Ctx {
            tree: &mut tree,
            tip: root,
            parse_options: crate::options::ParseOptions::new(),
            render_options: crate::options::RenderOptions::new(),
        };
        let document = dom.root;
        gen_children(
            &mut ctx,
            &mut dom,
            document,
            if ir { gen_ast_ir } else { gen_ast },
        );
        tree
    }

    #[test]
    fn inline_whitespace_moves_outside_markers_without_losing_siblings() {
        // Go genASTByVditorDOM moves both boundaries before visiting children;
        // the trailing text is inserted into the DOM during sibling traversal.
        for tag in ["em", "strong", "s", "mark", "sup", "sub"] {
            let tree = generate(&format!("<p>a<{tag}> b </{tag}>c</p>"), false);
            let paragraph = tree.root_raw().first_child().unwrap();
            let children: Vec<_> = paragraph.children().collect();
            assert_eq!(children.len(), 5, "{tag}");
            assert_eq!(children[0].tokens(), b"a");
            assert_eq!(children[1].tokens(), b" ");
            assert_eq!(children[2].text(), "b", "{tag}");
            assert_eq!(children[3].tokens(), b" ");
            assert_eq!(children[4].tokens(), b"c");
        }
    }

    #[test]
    fn nested_emphasis_newline_is_moved_once() {
        let tree = generate("<p><strong>foo<em>\u{200b}\nb</em></strong></p>", false);
        let strong = tree
            .root_raw()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap();
        let emphasis = strong.child_by_type(NodeType::NodeEmphasis).unwrap();
        assert_eq!(emphasis.text(), "b");
        assert_eq!(
            emphasis.previous().unwrap().tokens(),
            "\u{200b}\n".as_bytes()
        );
    }

    #[test]
    fn details_raw_delimiters_keep_block_content_as_siblings() {
        for ir in [false, true] {
            let tree = generate("<details><summary>title</summary><p>body</p></details>", ir);
            let children: Vec<_> = tree.root_raw().children().collect();
            assert_eq!(children.len(), 3);
            assert_eq!(children[0].typ(), NodeType::NodeHTMLBlock);
            assert_eq!(children[0].tokens(), b"<details><summary>title</summary>");
            assert_eq!(children[1].typ(), NodeType::NodeParagraph);
            assert_eq!(children[1].text(), "body");
            assert_eq!(children[2].tokens(), b"</details>");
        }
    }

    #[test]
    fn ir_generic_marker_spans_keep_their_text_children() {
        let tree = generate("<span class=\"vditor-ir__marker\">`</span><code>foo</code><span class=\"vditor-ir__marker\">`</span>", true);
        let children: Vec<_> = tree.root_raw().children().collect();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].tokens(), b"`");
        assert_eq!(children[1].typ(), NodeType::NodeCodeSpan);
        assert_eq!(children[1].first_child().unwrap().tokens(), b"foo");
        assert_eq!(children[2].tokens(), b"`");
    }
}
