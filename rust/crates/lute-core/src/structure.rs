// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Protyle Block DOM 结构变换，对齐 Go 版 protyle.go 的 OL2UL 等函数。

use crate::ast::{NodeType, RawNode as Node, Tree};
use crate::blockdom::{blockdom2tree_with_render_options, tree2blockdom};
use crate::editor;
use crate::options::{ParseOptions, RenderOptions};

fn render(tree: &Tree, render: &RenderOptions, parse: &ParseOptions) -> String {
    tree2blockdom(tree, render.clone(), parse.clone())
}

fn first_list(tree: &Tree) -> Option<Node> {
    let n = tree.root_raw().first_child()?;
    if NodeType::NodeList == n.typ() {
        Some(n)
    } else {
        None
    }
}

fn set_list_typ(list: Node, typ: i32) {
    if let Some(ld) = list.data_mut().list_data.as_mut() {
        ld.typ = typ;
    }
}

fn set_li_typ(li: Node, typ: i32) {
    if li.data().list_data.is_none() {
        li.data_mut().list_data = Some(crate::ast::ListData::default());
    }
    if let Some(ld) = li.data_mut().list_data.as_mut() {
        ld.typ = typ;
    }
}

pub fn ol2ul(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    set_list_typ(list, 0);
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeKramdownBlockIAL != cur.typ() {
            set_li_typ(cur, 0);
        }
        li = cur.next();
    }
    render(&tree, r, p)
}

pub fn ul2ol(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    set_list_typ(list, 1);
    let mut num = 1;
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeKramdownBlockIAL != cur.typ() {
            set_li_typ(cur, 1);
            if let Some(ld) = cur.data_mut().list_data.as_mut() {
                ld.num = num;
            }
            num += 1;
        }
        li = cur.next();
    }
    render(&tree, r, p)
}

pub fn to_task_list(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let mut tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    set_list_typ(list, 3);
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeListItem == cur.typ() {
            set_li_typ(cur, 3);
            let marker = tree.new_node_raw(NodeType::NodeTaskListItemMarker);
            cur.prepend_child(marker);
        }
        li = cur.next();
    }
    render(&tree, r, p)
}

pub fn tl2ol(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    if list.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0) != 3 {
        return html.to_string();
    }
    set_list_typ(list, 1);
    let mut num = 1;
    let mut unlinks = Vec::new();
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeKramdownBlockIAL != cur.typ() {
            if let Some(first) = cur.first_child() {
                unlinks.push(first);
            }
            set_li_typ(cur, 1);
            if let Some(ld) = cur.data_mut().list_data.as_mut() {
                ld.num = num;
            }
            num += 1;
        }
        li = cur.next();
    }
    for n in unlinks {
        n.unlink();
    }
    render(&tree, r, p)
}

pub fn tl2ul(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    if list.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0) != 3 {
        return html.to_string();
    }
    set_list_typ(list, 0);
    let mut unlinks = Vec::new();
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeKramdownBlockIAL != cur.typ() {
            if let Some(first) = cur.first_child() {
                unlinks.push(first);
            }
            set_li_typ(cur, 0);
        }
        li = cur.next();
    }
    for n in unlinks {
        n.unlink();
    }
    render(&tree, r, p)
}

pub fn convert_list_type(html: &str, target: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let target_typ = match target {
        "u" => 0,
        "o" => 1,
        "t" => 3,
        _ => return html.to_string(),
    };
    let mut tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = first_list(&tree) else {
        return html.to_string();
    };
    let mut lists = Vec::new();
    crate::ast::walk_raw(list, &mut |n, entering| {
        if entering
            && NodeType::NodeList == n.typ()
            && n.data().list_data.as_ref().map(|d| d.typ).unwrap_or(-1) != target_typ
        {
            lists.push(n);
        }
        crate::ast::WalkStatus::WalkContinue
    });
    if lists.is_empty() {
        return html.to_string();
    }
    for list in lists {
        set_list_typ(list, target_typ);
        let mut num = 1;
        let mut li = list.first_child();
        while let Some(cur) = li {
            if NodeType::NodeListItem != cur.typ() {
                li = cur.next();
                continue;
            }
            set_li_typ(cur, target_typ);
            if 1 == target_typ {
                if let Some(ld) = cur.data_mut().list_data.as_mut() {
                    ld.num = num;
                }
                num += 1;
            }
            let task = cur.child_by_type(NodeType::NodeTaskListItemMarker);
            if 3 == target_typ {
                if task.is_none() {
                    let marker = tree.new_node_raw(NodeType::NodeTaskListItemMarker);
                    cur.prepend_child(marker);
                }
            } else if let Some(task) = task {
                task.unlink();
            }
            li = cur.next();
        }
    }
    render(&tree, r, p)
}

pub fn blockquote2callout(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let mut tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(bq) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeBlockquote != bq.typ() {
        return html.to_string();
    }
    let marker = bq.first_child();
    let first_child = marker.and_then(|m| m.next());
    if first_child.is_none() {
        let id = crate::ast::new_node_id();
        let new_p = tree.new_node_raw(NodeType::NodeParagraph);
        new_p.data_mut().id = id.clone();
        new_p.set_ial_attr("id", &id);
        new_p.set_ial_attr("updated", &id[..14.min(id.len())]);
        bq.append_child(new_p);
        let ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
        ial.set_tokens(format!("{{: id=\"{}\"}}", id).into_bytes());
        bq.append_child(ial);
    }
    let mut content = String::new();
    if let Some(first_child) = first_child {
        if NodeType::NodeParagraph == first_child.typ() {
            let text = first_child.first_child();
            if text.is_none() {
                let empty = tree.new_node_raw(NodeType::NodeText);
                first_child.append_child(empty);
            }
            content = first_child
                .first_child()
                .map(|t| t.text())
                .unwrap_or_default();
            content = content.trim().to_string();
            content = content.trim_start_matches(editor::CARET).to_string();
        }
    }
    let mut typ = "NOTE".to_string();
    let mut first_is_type = false;
    if content.starts_with("[!") {
        if let Some(idx) = content.find(']') {
            typ = content[2..idx].to_string();
            first_is_type = true;
        }
    }
    if let Some(marker) = bq.first_child() {
        marker.unlink();
    }
    bq.data_mut().typ = NodeType::NodeCallout;
    bq.data_mut().callout_type = typ.clone();
    bq.data_mut().callout_title = crate::parse::callout::get_callout_title(&typ).to_string();
    bq.data_mut().callout_icon = crate::parse::callout::get_callout_icon(&typ).to_string();
    if first_is_type {
        let need_empty = first_child
            .and_then(|f| f.next())
            .and_then(|f| f.next())
            .is_none();
        if need_empty {
            let id = crate::ast::new_node_id();
            let new_p = tree.new_node_raw(NodeType::NodeParagraph);
            new_p.data_mut().id = id.clone();
            new_p.set_ial_attr("id", &id);
            new_p.set_ial_attr("updated", &id[..14.min(id.len())]);
            bq.append_child(new_p);
            let ial = tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
            ial.set_tokens(format!("{{: id=\"{}\"}}", id).into_bytes());
            bq.append_child(ial);
        }
        let title = content
            .find(']')
            .map(|i| content[i + 1..].trim().to_string())
            .unwrap_or_default();
        bq.data_mut().callout_title = title;
        if let Some(first) = bq.first_child() {
            first.unlink();
        }
        if let Some(first) = bq.first_child() {
            first.unlink();
        }
    }
    render(&tree, r, p)
}

pub fn callout2blockquote(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let mut tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(co) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeCallout != co.typ() {
        return html.to_string();
    }
    let mut title = co.data().callout_title.clone();
    if 0 == co.data().callout_icon_type {
        title = format!("{} {}", co.data().callout_icon, title);
    }
    co.data_mut().typ = NodeType::NodeBlockquote;
    let para = tree.new_node_raw(NodeType::NodeParagraph);
    if co.data().callout_icon_type == 1 {
        let icon = co.data().callout_icon.clone();
        let alt_start = icon
            .find("/emojis/")
            .map(|index| index + "/emojis/".len())
            .unwrap_or(0);
        let alt = &icon[alt_start..];
        let emoji = tree.new_node_raw(NodeType::NodeEmoji);
        let image = tree.new_node_raw(NodeType::NodeEmojiImg);
        image.set_tokens(
            format!("<img alt=\"{alt}\" class=\"emoji\" src=\"{icon}\" title=\"{alt}\" />")
                .into_bytes(),
        );
        let alias = tree.new_node_raw(NodeType::NodeEmojiAlias);
        alias.set_tokens(format!(":{alt}:").into_bytes());
        image.append_child(alias);
        emoji.append_child(image);
        para.append_child(emoji);
    }
    title = title.trim().to_string();
    if 1 == co.data().callout_icon_type {
        title = format!(" {}", title);
    }
    let text = tree.new_node_raw(NodeType::NodeText);
    text.set_tokens(title.into_bytes());
    para.append_child(text);
    co.prepend_child(para);
    co.prepend_child(tree.new_node_raw(NodeType::NodeBlockquoteMarker));
    render(&tree, r, p)
}

pub fn cancel_blockquote(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(bq) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeBlockquote != bq.typ() {
        return html.to_string();
    }
    let mut children = Vec::new();
    let mut c = bq.first_child();
    while let Some(cur) = c {
        children.push(cur);
        c = cur.next();
    }
    for cur in children {
        if NodeType::NodeBlockquoteMarker != cur.typ() {
            tree.root_raw().append_child(cur);
        } else {
            cur.unlink();
        }
    }
    bq.unlink();
    render(&tree, r, p)
}

pub fn cancel_callout(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let mut tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(co) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeCallout != co.typ() {
        return html.to_string();
    }
    let para = tree.new_node_raw(NodeType::NodeParagraph);
    for (k, v) in co.data().kramdown_ial.clone() {
        para.set_ial_attr(&k, &v);
    }
    let text = tree.new_node_raw(NodeType::NodeText);
    if !co.data().callout_icon.is_empty() {
        if 0 == co.data().callout_icon_type {
            text.set_tokens(
                format!("{} {}", co.data().callout_icon, co.data().callout_title).into_bytes(),
            );
        } else {
            let img = tree.new_node_raw(NodeType::NodeImage);
            img.append_child(tree.new_node_raw(NodeType::NodeBang));
            img.append_child(tree.new_node_raw(NodeType::NodeOpenBracket));
            img.append_child(tree.new_node_raw(NodeType::NodeCloseBracket));
            img.append_child(tree.new_node_raw(NodeType::NodeOpenParen));
            let dest = tree.new_node_raw(NodeType::NodeLinkDest);
            dest.set_tokens(co.data().callout_icon.clone().into_bytes());
            img.append_child(dest);
            img.append_child(tree.new_node_raw(NodeType::NodeCloseParen));
            para.append_child(img);
            text.set_tokens(co.data().callout_title.clone().into_bytes());
        }
    }
    para.append_child(text);
    tree.root_raw().append_child(para);
    let mut children = Vec::new();
    let mut c = co.first_child();
    while let Some(cur) = c {
        children.push(cur);
        c = cur.next();
    }
    for cur in children {
        tree.root_raw().append_child(cur);
    }
    co.unlink();
    render(&tree, r, p)
}

pub fn cancel_superblock(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(sb) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeSuperBlock != sb.typ() {
        return html.to_string();
    }
    let mut children = Vec::new();
    let mut c = sb.first_child();
    while let Some(cur) = c {
        children.push(cur);
        c = cur.next();
    }
    for cur in children {
        tree.root_raw().append_child(cur);
    }
    sb.unlink();
    render(&tree, r, p)
}

pub fn cancel_list(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeList != list.typ() {
        return html.to_string();
    }
    let mut appends = Vec::new();
    let mut unlinks = Vec::new();
    let mut li = list.first_child();
    while let Some(cur) = li {
        let mut c = cur.first_child();
        while let Some(child) = c {
            if NodeType::NodeTaskListItemMarker != child.typ() {
                appends.push(child);
            }
            c = child.next();
        }
        unlinks.push(cur);
        li = cur.next();
    }
    for c in appends {
        tree.root_raw().append_child(c);
    }
    for n in unlinks {
        n.unlink();
    }
    list.unlink();
    render(&tree, r, p)
}

fn cancel_one_list(list: Node) {
    let mut children = Vec::new();
    let mut li = list.first_child();
    while let Some(cur) = li {
        if NodeType::NodeListItem != cur.typ() {
            li = cur.next();
            continue;
        }
        let mut c = cur.first_child();
        while let Some(child) = c {
            if NodeType::NodeTaskListItemMarker != child.typ() {
                children.push(child);
            }
            c = child.next();
        }
        li = cur.next();
    }
    for child in children {
        list.insert_before(child);
    }
    list.unlink();
}

pub fn cancel_list_recursively(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let Some(list) = tree.root_raw().first_child() else {
        return html.to_string();
    };
    if NodeType::NodeList != list.typ() || list.data().list_data.is_none() {
        return html.to_string();
    }
    let source_typ = list.data().list_data.as_ref().map(|d| d.typ).unwrap_or(0);
    let mut lists = Vec::new();
    crate::ast::walk_raw(list, &mut |n, entering| {
        if entering
            && NodeType::NodeList == n.typ()
            && n.data().list_data.as_ref().map(|d| d.typ).unwrap_or(-1) == source_typ
        {
            lists.push(n);
        }
        crate::ast::WalkStatus::WalkContinue
    });
    for item in lists.into_iter().rev() {
        cancel_one_list(item);
    }
    render(&tree, r, p)
}

pub fn blocks2ps(html: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let mut appends = Vec::new();
    let mut unlinks = Vec::new();
    let mut c = tree.root_raw().first_child();
    while let Some(n) = c {
        match n.typ() {
            NodeType::NodeHeading => {
                n.data_mut().typ = NodeType::NodeParagraph;
                appends.push(n);
                unlinks.push(n);
            }
            NodeType::NodeBlockquote => {
                let mut sub = n.first_child();
                while let Some(cur) = sub {
                    if NodeType::NodeBlockquoteMarker == cur.typ() {
                        unlinks.push(cur);
                    } else {
                        appends.push(cur);
                    }
                    sub = cur.next();
                }
                unlinks.push(n);
            }
            NodeType::NodeList => {
                let mut li = n.first_child();
                while let Some(cur) = li {
                    let mut sub = cur.first_child();
                    while let Some(child) = sub {
                        if NodeType::NodeTaskListItemMarker != child.typ() {
                            appends.push(child);
                        }
                        sub = child.next();
                    }
                    unlinks.push(cur);
                    li = cur.next();
                }
                unlinks.push(n);
            }
            _ => {
                appends.push(n);
                unlinks.push(n);
            }
        }
        c = n.next();
    }
    for n in unlinks {
        n.unlink();
    }
    for n in appends {
        tree.root_raw().append_child(n);
    }
    render(&tree, r, p)
}

pub fn blocks2hs(html: &str, level: &str, r: &RenderOptions, p: &ParseOptions) -> String {
    let tree = blockdom2tree_with_render_options(html, p.clone(), r);
    let level: i32 = level.parse().unwrap_or(1);
    let mut c = tree.root_raw().first_child();
    while let Some(n) = c {
        if NodeType::NodeParagraph == n.typ() || NodeType::NodeHeading == n.typ() {
            n.data_mut().typ = NodeType::NodeHeading;
            if let Some(first) = n.first_child() {
                let tokens = first.tokens_vec();
                let tokens: Vec<u8> = tokens.into_iter().filter(|&b| b != b'\n').collect();
                let start = tokens
                    .iter()
                    .position(|&b| !matches!(b, b' ' | b'\t' | b'\n'))
                    .unwrap_or(tokens.len());
                first.set_tokens(tokens[start..].to_vec());
            }
            n.data_mut().heading_level = level;
        }
        c = n.next();
    }
    render(&tree, r, p)
}
