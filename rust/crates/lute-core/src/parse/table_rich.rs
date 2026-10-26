// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 表格单元格富文本源，对应 Go 版 parse/table_rich.go 与 markdown_table_rich.go。
//!
//! 富文本源以 kramdown 片段保存在单元格 IAL 或 BlockDOM 属性中；单元格自身只
//! 展示投影后的行内节点，块身份不进入文档块树。

use crate::ast::{
    self, decode_table_cell_rich, NodeType, RawNode as Node, TableCellRich, Tree,
    TABLE_CELL_RICH_ATTRIBUTE, TABLE_CELL_RICH_FORMAT, TABLE_CELL_RICH_IAL, TABLE_CELL_RICH_SPEC,
};
use crate::html as html_util;
use crate::options::{ParseOptions, RenderOptions};

/// 固定富文本存储语法，不受当前编辑器开关影响，对应 Go 版 parse.TableCellRichOptions。
pub fn table_cell_rich_options() -> ParseOptions {
    let mut o = ParseOptions::new();
    o.disable_table_cell_rich = true;
    o.protyle_wysiwyg = true;
    o.text_mark = true;
    o.html_tag2_text_mark = true;
    o.block_ref = true;
    o.file_annotation_ref = true;
    o.kramdown_block_ial = true;
    o.kramdown_span_ial = true;
    o.emoji = false;
    o.super_block = true;
    o.custom_block = true;
    o.git_conflict = true;
    o.callout = true;
    o.tabs = true;
    o.img_path_allow_space = true;
    o.sup = true;
    o.sub = true;
    o.tag = true;
    o.mark = true;
    o.gfm_strikethrough1 = false;
    o.full_width_strikethrough = true;
    o.inline_math_allow_digit_after_open_marker = true;
    o.footnotes = false;
    o.toc = false;
    o.heading_id = false;
    o.setext = false;
    o.yaml_front_matter = false;
    o.link_ref = false;
    o.indent_code_block = false;
    o.paragraph_beginning_space = true;
    o.arbitrary_task_list_item_marker = true;
    o.ensure_list_item_paragraph = true;
    o
}

/// 解析受支持的片段，对应 Go 版 parse.ParseTableCellRich。
pub fn parse_table_cell_rich(rich: &TableCellRich) -> Result<Tree, String> {
    rich.validate()?;
    let options = table_cell_rich_options();
    let mut tree = crate::parse::parse("", rich.content.as_bytes(), options.clone());
    let mut error = None;
    ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if !entering {
            return ast::WalkStatus::WalkContinue;
        }
        if n.is_block() {
            match n.typ() {
                NodeType::NodeDocument
                | NodeType::NodeParagraph
                | NodeType::NodeHeading
                | NodeType::NodeList
                | NodeType::NodeListItem
                | NodeType::NodeBlockquote
                | NodeType::NodeCodeBlock
                | NodeType::NodeMathBlock
                | NodeType::NodeKramdownBlockIAL => {}
                other => {
                    error = Some(format!("unsupported table cell rich text block [{other}]"));
                    return ast::WalkStatus::WalkStop;
                }
            }
        }
        if NodeType::NodeCodeBlock == n.typ() {
            let info = String::from_utf8_lossy(&n.data().code_block_info).to_lowercase();
            if let Some(language) = info.split_whitespace().next() {
                if matches!(
                    language,
                    "abc"
                        | "echarts"
                        | "flowchart"
                        | "graphviz"
                        | "infographic"
                        | "mermaid"
                        | "mindmap"
                        | "plantuml"
                ) {
                    error = Some(format!(
                        "unsupported table cell rich text code language [{language}]"
                    ));
                    return ast::WalkStatus::WalkStop;
                }
            }
        }
        ast::WalkStatus::WalkContinue
    });
    if let Some(error) = error {
        return Err(error);
    }
    crate::parse::text_marks2inlines(&mut tree, &options);
    crate::parse::nested_inlines2flatted_spans_hybrid(&mut tree, false);
    Ok(tree)
}

/// 将片段转换成行内节点，保留图片、引用、样式以及可读的列表标记。
///
/// 对应 Go 版 parse.ProjectTableCellRich。Go 会克隆行内节点以避免与片段树共享，
/// 这里直接移动节点：片段树随后即被丢弃，arena 合并保证节点存活。
pub(crate) fn project_table_cell_rich(tree: &mut Tree) -> Node {
    let ret = tree.new_node_raw(NodeType::NodeTableCell);
    let mut blocks = Vec::new();
    ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering
            && matches!(
                n.typ(),
                NodeType::NodeParagraph
                    | NodeType::NodeHeading
                    | NodeType::NodeCodeBlock
                    | NodeType::NodeMathBlock
            )
        {
            blocks.push(n);
        }
        ast::WalkStatus::WalkContinue
    });

    let append_text = |tree: &mut Tree, ret: Node, text: &str| {
        for (index, line) in text.split('\n').enumerate() {
            if 0 < index {
                let node = tree.new_node_raw(NodeType::NodeBr);
                node.set_tokens(b"<br />".to_vec());
                ret.append_child(node);
            }
            let node = tree.new_node_raw(NodeType::NodeText);
            node.set_tokens(line.as_bytes().to_vec());
            ret.append_child(node);
        }
    };

    for block in blocks {
        if ret.first_child().is_some() {
            let node = tree.new_node_raw(NodeType::NodeBr);
            node.set_tokens(b"<br />".to_vec());
            ret.append_child(node);
        }
        match block.typ() {
            NodeType::NodeParagraph => {
                if let Some(list_item) =
                    block.parent().filter(|p| p.typ() == NodeType::NodeListItem)
                {
                    let mut first = list_item.first_child();
                    if first.is_some_and(|n| n.typ() == NodeType::NodeTaskListItemMarker) {
                        first = first.and_then(|n| n.next());
                    }
                    if first == Some(block) {
                        let mut marker = "- ".to_string();
                        if list_item
                            .data()
                            .list_data
                            .as_ref()
                            .is_some_and(|data| 1 == data.typ)
                        {
                            let num = list_item
                                .data()
                                .list_data
                                .as_ref()
                                .map(|data| data.num)
                                .unwrap_or(0);
                            marker = format!("{num}. ");
                        } else if let Some(task) =
                            list_item.child_by_type(NodeType::NodeTaskListItemMarker)
                        {
                            marker = if task.data().task_list_item_checked {
                                "- [x] ".to_string()
                            } else {
                                "- [ ] ".to_string()
                            };
                        }
                        append_text(tree, ret, &marker);
                    }
                }
                let mut children = Vec::new();
                for child in block.children() {
                    if !matches!(
                        child.typ(),
                        NodeType::NodeHeadingC8hMarker | NodeType::NodeTaskListItemMarker
                    ) {
                        children.push(child);
                    }
                }
                for child in children {
                    ret.append_child(child);
                }
            }
            NodeType::NodeCodeBlock => {
                if let Some(content) = block.child_by_type(NodeType::NodeCodeBlockCode) {
                    let code = tree.new_node_raw(NodeType::NodeTextMark);
                    code.data_mut().text_mark_type = "code".to_string();
                    let text = content.tokens_str();
                    code.data_mut().text_mark_text_content =
                        html_util::escape_html_str(text.strip_suffix('\n').unwrap_or(&text));
                    ret.append_child(code);
                }
            }
            NodeType::NodeMathBlock => {
                if let Some(content) = block.child_by_type(NodeType::NodeMathBlockContent) {
                    let math = tree.new_node_raw(NodeType::NodeTextMark);
                    math.data_mut().text_mark_type = "inline-math".to_string();
                    math.data_mut().text_mark_inline_math_content =
                        html_util::escape_html_str(content.tokens_str().trim());
                    ret.append_child(math);
                }
            }
            _ => {
                let mut children = Vec::new();
                for child in block.children() {
                    if !matches!(
                        child.typ(),
                        NodeType::NodeHeadingC8hMarker | NodeType::NodeTaskListItemMarker
                    ) {
                        children.push(child);
                    }
                }
                for child in children {
                    ret.append_child(child);
                }
            }
        }
    }
    ret
}

/// 仅在源成功解析后刷新派生的行内内容，对应 Go 版 parse.ApplyTableCellRichProjection。
pub(crate) fn apply_table_cell_rich_projection(cell: Node) -> Result<(), String> {
    let rich = match cell.data().table_cell_rich.clone() {
        Some(rich) => rich,
        None => return Ok(()),
    };
    let mut fragment = parse_table_cell_rich(&rich)?;
    let projection = project_table_cell_rich(&mut fragment);
    while let Some(child) = cell.first_child() {
        child.unlink();
    }
    while let Some(child) = projection.first_child() {
        cell.append_child(child);
    }
    Ok(())
}

/// 解析收尾阶段把单元格 IAL 中的富文本源转换为行内投影。
///
/// 对应 Go 版 Tree.finalParseTableCellRich。
pub(crate) fn final_parse_table_cell_rich(tree: &mut Tree, options: &ParseOptions) {
    if options.disable_table_cell_rich {
        return;
    }
    let mut cells = Vec::new();
    ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering && NodeType::NodeTableCell == n.typ() {
            cells.push(n);
        }
        ast::WalkStatus::WalkContinue
    });
    for cell in cells {
        let Some(encoded) = cell
            .ial_raw()
            .into_iter()
            .find(|(name, _)| name == TABLE_CELL_RICH_IAL)
            .map(|(_, value)| value)
        else {
            continue;
        };
        cell.data_mut().table_cell_rich = Some(decode_table_cell_rich(&encoded));
        cell.remove_ial_attr(TABLE_CELL_RICH_IAL);
        let mut span_ials = Vec::new();
        for child in cell.children() {
            if NodeType::NodeKramdownSpanIAL == child.typ() {
                span_ials.push(child);
            }
        }
        for span_ial in span_ials {
            span_ial.unlink();
        }
        let _ = apply_table_cell_rich_projection(cell);
    }
}

/// 仅用于外部 Markdown 导入，将明确的块结构还原为已有的单元格富文本源。
///
/// 对应 Go 版 NormalizeMarkdownTableCells：已有载荷不重新解释，不支持或未闭合
/// 的结构保留原始行内内容。
pub fn normalize_markdown_table_cells(tree: &mut Tree) {
    let mut cells = Vec::new();
    ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering && NodeType::NodeTableCell == n.typ() {
            cells.push(n);
        }
        ast::WalkStatus::WalkContinue
    });
    for cell in cells {
        if cell.data().table_cell_rich.is_some() {
            continue;
        }
        let source = cell.data().table_cell_source.trim().to_string();
        if source.is_empty() {
            continue;
        }
        let content = if is_markdown_table_cell_block_html(&source) {
            markdown_table_cell_html_content(&source)
        } else if is_markdown_table_cell_task(&source) {
            markdown_table_cell_breaks(&source)
        } else {
            continue;
        };
        let rich = TableCellRich {
            spec: TABLE_CELL_RICH_SPEC,
            format: TABLE_CELL_RICH_FORMAT.to_string(),
            content,
            ..Default::default()
        };
        if parse_table_cell_rich(&rich).is_err() {
            continue;
        }
        cell.data_mut().table_cell_rich = Some(rich);
        let _ = apply_table_cell_rich_projection(cell);
        let mut table = cell.parent().and_then(|p| p.parent());
        if table.is_some_and(|t| t.typ() == NodeType::NodeTableHead) {
            table = table.and_then(|t| t.parent());
        }
        if let Some(table) = table.filter(|t| t.typ() == NodeType::NodeTable) {
            table.set_ial_attr("custom-sy-table-rich", "1");
        }
    }
}

fn markdown_table_cell_html_content(source: &str) -> String {
    let options = table_cell_rich_options();
    let mut render_options = RenderOptions::new();
    render_options.auto_space = false;
    render_options.export_normalize_task_list_marker = false;
    let sanitized = crate::render::sanitizer::sanitize(source.as_bytes());
    let tree = crate::html2md::html2tree_with_render_options(
        &String::from_utf8_lossy(&sanitized),
        options.clone(),
        render_options.clone(),
    );
    let mut renderer = crate::render::format::FormatRenderer::new(&tree, render_options, options);
    String::from_utf8_lossy(&renderer.render()).to_string()
}

/// 在通用 HTML 预处理展开单元格内容前保存受支持的块结构。
///
/// 对应 Go 版 preserveHTMLTableCellStructure。
pub fn preserve_html_table_cell_structure(dom: &mut crate::dom::Dom) {
    let mut cells = Vec::new();
    collect_html_cells(dom, dom.root, &mut cells);
    for cell in cells {
        if dom.has_attr(cell, TABLE_CELL_RICH_ATTRIBUTE) {
            continue;
        }
        let mut fragment = String::new();
        for child in dom.nodes[cell].children.clone() {
            fragment.push_str(&dom.html(child));
        }
        let fragment = fragment.trim();
        if !is_markdown_table_cell_block_html(fragment) {
            continue;
        }
        let rich = TableCellRich {
            spec: TABLE_CELL_RICH_SPEC,
            format: TABLE_CELL_RICH_FORMAT.to_string(),
            content: markdown_table_cell_html_content(fragment),
            ..Default::default()
        };
        if parse_table_cell_rich(&rich).is_ok() {
            dom.set_attr(cell, TABLE_CELL_RICH_ATTRIBUTE, &rich.encode());
        }
    }
}

fn collect_html_cells(dom: &crate::dom::Dom, node: usize, cells: &mut Vec<usize>) {
    let tag = dom.nodes[node].data.to_lowercase();
    if "td" == tag || "th" == tag {
        cells.push(node);
        return;
    }
    for child in dom.nodes[node].children.clone() {
        collect_html_cells(dom, child, cells);
    }
}

/// 要求显式闭合和受支持的标签，避免 HTML 容错解析吞掉原文。
///
/// 对应 Go 版 isMarkdownTableCellBlockHTML。
pub fn is_markdown_table_cell_block_html(source: &str) -> bool {
    if !source.starts_with('<') {
        return false;
    }
    let bytes = source.as_bytes();
    let mut stack: Vec<String> = Vec::new();
    let mut has_block = false;
    let mut pos = 0;
    while pos < bytes.len() {
        if b'<' != bytes[pos] {
            match bytes[pos..].iter().position(|b| b'<' == *b) {
                Some(offset) => pos += offset,
                None => pos = bytes.len(),
            }
            continue;
        }
        let Some(tag) = scan_html_tag(bytes, pos) else {
            return false;
        };
        pos = tag.end;
        match tag.kind {
            TagKind::Start | TagKind::SelfClosing => {
                let name = tag.name.as_str();
                match name {
                    "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li"
                    | "blockquote" | "pre" => has_block = true,
                    "span" | "strong" | "b" | "em" | "i" | "s" | "del" | "u" | "mark" | "kbd"
                    | "sub" | "sup" | "code" | "a" | "br" | "img" => {}
                    "div" => {
                        let math = tag
                            .attrs
                            .iter()
                            .any(|(key, value)| "class" == key && "language-math" == value);
                        if !math {
                            return false;
                        }
                        has_block = true;
                    }
                    "input" => {
                        let checkbox = tag.attrs.iter().any(|(key, value)| {
                            "type" == key && "checkbox" == value.to_lowercase()
                        });
                        if !checkbox {
                            return false;
                        }
                    }
                    _ => return false,
                }
                for (key, _) in &tag.attrs {
                    if key.starts_with("on") || TABLE_CELL_RICH_ATTRIBUTE == key {
                        return false;
                    }
                }
                if !matches!(name, "br" | "img" | "input") {
                    if TagKind::SelfClosing == tag.kind {
                        return false;
                    }
                    stack.push(name.to_string());
                }
            }
            TagKind::End => {
                if stack.pop().as_deref() != Some(tag.name.as_str()) {
                    return false;
                }
            }
        }
    }
    stack.is_empty() && has_block
}

#[derive(PartialEq, Clone, Copy)]
enum TagKind {
    Start,
    SelfClosing,
    End,
}

struct ScannedTag {
    kind: TagKind,
    name: String,
    attrs: Vec<(String, String)>,
    end: usize,
}

/// 扫描一个完整标签；注释、DOCTYPE 和处理指令一律视为不支持。
fn scan_html_tag(bytes: &[u8], start: usize) -> Option<ScannedTag> {
    let mut pos = start + 1;
    let kind = if bytes.get(pos) == Some(&b'/') {
        pos += 1;
        TagKind::End
    } else if matches!(bytes.get(pos), Some(b'!') | Some(b'?')) {
        return None;
    } else {
        TagKind::Start
    };
    let name_start = pos;
    while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || b'-' == bytes[pos]) {
        pos += 1;
    }
    if name_start == pos {
        return None;
    }
    let name = String::from_utf8_lossy(&bytes[name_start..pos]).to_lowercase();
    let mut attrs = Vec::new();
    let mut kind = kind;
    loop {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        match bytes.get(pos) {
            None => return None,
            Some(b'>') => {
                pos += 1;
                break;
            }
            Some(b'/') => {
                if bytes.get(pos + 1) != Some(&b'>') {
                    return None;
                }
                pos += 2;
                kind = TagKind::SelfClosing;
                break;
            }
            Some(_) => {}
        }
        let attr_start = pos;
        while pos < bytes.len()
            && !bytes[pos].is_ascii_whitespace()
            && !matches!(bytes[pos], b'=' | b'>' | b'/')
        {
            pos += 1;
        }
        if attr_start == pos {
            return None;
        }
        let key = String::from_utf8_lossy(&bytes[attr_start..pos]).to_lowercase();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut value = String::new();
        if bytes.get(pos) == Some(&b'=') {
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            match bytes.get(pos) {
                Some(&quote) if b'"' == quote || b'\'' == quote => {
                    let value_start = pos + 1;
                    let mut cursor = value_start;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    if cursor == bytes.len() {
                        return None;
                    }
                    value = String::from_utf8_lossy(&bytes[value_start..cursor]).to_string();
                    pos = cursor + 1;
                }
                None => return None,
                Some(_) => {
                    let value_start = pos;
                    while pos < bytes.len()
                        && !bytes[pos].is_ascii_whitespace()
                        && !matches!(bytes[pos], b'>' | b'/')
                    {
                        pos += 1;
                    }
                    value = String::from_utf8_lossy(&bytes[value_start..pos]).to_string();
                }
            }
        }
        if !attrs.iter().any(|(existing, _)| *existing == key) {
            attrs.push((key, value));
        }
    }
    Some(ScannedTag {
        kind,
        name,
        attrs,
        end: pos,
    })
}

fn is_markdown_table_cell_task(source: &str) -> bool {
    let bytes = source.as_bytes();
    if 5 > bytes.len() || !matches!(bytes[0], b'-' | b'*' | b'+') {
        return false;
    }
    let rest = &bytes[1..];
    if rest.is_empty() || !matches!(rest[0], b' ' | b'\t') {
        return false;
    }
    let rest = trim_left_space(rest);
    if 3 > rest.len() || b'[' != rest[0] || b']' != rest[2] {
        return false;
    }
    if !table_cell_rich_options().is_valid_task_list_item_marker(rest[1]) {
        return false;
    }
    3 == rest.len() || matches!(rest[3], b' ' | b'\t')
}

fn trim_left_space(bytes: &[u8]) -> &[u8] {
    let mut pos = 0;
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
        pos += 1;
    }
    &bytes[pos..]
}

/// 只展开代码之外的显式换行，保留反斜杠转义和代码中的 HTML。
///
/// 对应 Go 版 markdownTableCellBreaks。
fn markdown_table_cell_breaks(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut code = 0usize;
    let mut pos = 0;
    while pos < bytes.len() {
        if b'\\' == bytes[pos] && pos + 1 < bytes.len() && 0 == code {
            out.push_str(&source[pos..pos + 2]);
            pos += 2;
            continue;
        }
        if b'`' == bytes[pos] {
            let mut end = pos + 1;
            while end < bytes.len() && b'`' == bytes[end] {
                end += 1;
            }
            if 0 == code {
                code = end - pos;
            } else if code == end - pos {
                code = 0;
            }
            out.push_str(&source[pos..end]);
            pos = end;
            continue;
        }
        if 0 == code && b'<' == bytes[pos] {
            if let Some(tag) = scan_html_tag(bytes, pos) {
                if "br" == tag.name && TagKind::End != tag.kind {
                    out.push('\n');
                    pos = tag.end;
                    continue;
                }
            }
        }
        let ch = source[pos..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        pos += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod go_table_rich_tests {
    //! 迁移自 test/table_rich_test.go，断言与原文一致。

    use super::*;
    use crate::lute::Lute;

    fn rich_lute() -> Lute {
        let mut engine = Lute::new();
        engine.parse_options = table_cell_rich_options();
        engine.parse_options.disable_table_cell_rich = false;
        engine.parse_options.kramdown_block_ial = true;
        engine.parse_options.kramdown_span_ial = true;
        engine.parse_options.text_mark = true;
        engine.render_options.protyle_wysiwyg = true;
        engine.render_options.kramdown_block_ial = true;
        engine.render_options.kramdown_span_ial = true;
        engine
    }

    fn first_rich_cell(tree: &Tree) -> Option<Node> {
        let mut ret = None;
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering && n.data().table_cell_rich.is_some() {
                ret = Some(n);
                return ast::WalkStatus::WalkStop;
            }
            ast::WalkStatus::WalkContinue
        });
        ret
    }

    fn first_cell(tree: &Tree) -> Node {
        tree.root_raw()
            .first_child()
            .unwrap()
            .last_child()
            .unwrap()
            .first_child()
            .unwrap()
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichRoundTrip
    fn go_table_rich_round_trip() {
        let engine = rich_lute();
        let sources = [
            "- first\n- **second**",
            "# title\n\n> quote\n\n```go\nfmt.Println(\"a|b\")\n```\n\n$$\nx^2\n$$",
            "- [ ] task\n  - nested\n\n![image](assets/picture.png)\n\n((20260101000000-abcdefg 'reference'))",
            "literal \\*marker\\* and `a|b`\n\nnext paragraph",
            "<span data-type=\"text\">font</span>{: style=\"font-family: var(--b3-font-family-emoji-reset), 'A&#92;&#92;B &gt; C', var(--b3-font-family-editor), var(--b3-font-family);\"}",
            "<span data-type=\"code\">&lt;span data-type=\"text\"&gt;font&lt;/span&gt;{: style=\"color: var(--b3-font-color1);\"}</span>",
            "",
        ];
        for source in sources {
            let tree = crate::parse::parse(
                "",
                b"| title |\n| --- |\n| old |",
                engine.parse_options.clone(),
            );
            let cell = first_cell(&tree);
            cell.data_mut().table_cell_rich = Some(TableCellRich {
                spec: TABLE_CELL_RICH_SPEC,
                format: TABLE_CELL_RICH_FORMAT.to_string(),
                content: source.to_string(),
                ..Default::default()
            });
            cell.set_ial_attr("style", "color: red;");
            apply_table_cell_rich_projection(cell).expect("projection");
            let mut dom = crate::blockdom::tree2blockdom(
                &tree,
                engine.render_options.clone(),
                engine.parse_options.clone(),
            );
            for iteration in 0..3 {
                dom = engine.spin_blockdom(&dom);
                let round = crate::blockdom::blockdom2tree(&dom, engine.parse_options.clone());
                let read = first_rich_cell(&round).expect("round trip lost rich source");
                let rich = read.data().table_cell_rich.as_ref().unwrap();
                assert_eq!(
                    rich.content, source,
                    "iteration {iteration} lost source: {dom}"
                );
                assert_eq!(read.ial_attr("style"), "color: red;", "cell style lost");
                ast::walk_raw(read, &mut |n, entering| {
                    if entering && n.is_block() {
                        panic!("cell projection contains a block: {}", n.typ());
                    }
                    ast::WalkStatus::WalkContinue
                });
            }
            let md = engine.blockdom2stdmd(&dom);
            assert!(
                !md.contains(TABLE_CELL_RICH_IAL) && !md.contains(TABLE_CELL_RICH_ATTRIBUTE),
                "standard Markdown leaked internal metadata: {md}"
            );
            let json = crate::render::json::render_json(
                &crate::blockdom::blockdom2tree(&dom, engine.parse_options.clone()),
                &engine.render_options,
            );
            assert!(
                json.contains("\"TableCellRich\""),
                "JSON dropped rich source: {json}"
            );
        }
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichClipboardAndMergedExport
    fn go_table_rich_clipboard_and_merged_export() {
        let engine = rich_lute();
        let tree = crate::parse::parse(
            "",
            b"| first | second |\n| --- | --- |\n| old | hidden |",
            engine.parse_options.clone(),
        );
        let cell = first_cell(&tree);
        let content = "- first\n- second\n\n```text\na | b\nc\n```";
        cell.data_mut().table_cell_rich = Some(TableCellRich {
            spec: TABLE_CELL_RICH_SPEC,
            format: TABLE_CELL_RICH_FORMAT.to_string(),
            content: content.to_string(),
            ..Default::default()
        });
        cell.set_ial_attr("colspan", "2");
        cell.next().unwrap().set_ial_attr("class", "fn__none");
        apply_table_cell_rich_projection(cell).expect("projection");

        let dom = crate::blockdom::tree2blockdom(
            &tree,
            engine.render_options.clone(),
            engine.parse_options.clone(),
        );
        let clipboard = engine.html2blockdom(&dom);
        let restored_tree =
            crate::blockdom::blockdom2tree(&clipboard, engine.parse_options.clone());
        let restored =
            first_rich_cell(&restored_tree).expect("HTML clipboard lost merged rich cell");
        assert_eq!(
            restored.data().table_cell_rich.as_ref().unwrap().content,
            content
        );
        assert_eq!(restored.ial_attr("colspan"), "2");

        let markdown = engine.blockdom2stdmd(&dom);
        println!("MD={markdown}");
        assert!(
            markdown.contains("first")
                && markdown.contains("a &#124; b&#10;c&#10;</code>")
                && !markdown.contains("table-cell-rich")
                && markdown.contains("<ul>"),
            "merged Markdown export lost cell structure: {markdown}"
        );
        let html = engine.blockdom2richhtml(&dom);
        assert!(
            html.contains("<ul>")
                && html.contains("<code")
                && !html.contains(TABLE_CELL_RICH_ATTRIBUTE),
            "HTML export lost the fragment structure: {html}"
        );
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichPreservesInvalidEmptyMetadata
    fn go_table_rich_preserves_invalid_empty_metadata() {
        let engine = rich_lute();
        let mut dom = "<div data-type=\"NodeTable\"><div contenteditable=\"true\"><table><thead><tr><th data-sy-table-cell-rich=\"\">preserve</th></tr></thead></table></div></div>".to_string();
        for _ in 0..3 {
            dom = engine.spin_blockdom(&dom);
            let tree = crate::blockdom::blockdom2tree(&dom, engine.parse_options.clone());
            let cell = first_rich_cell(&tree).expect("corrupt metadata was silently discarded");
            assert!(
                cell.data()
                    .table_cell_rich
                    .as_ref()
                    .unwrap()
                    .validate()
                    .is_err(),
                "corrupt metadata was accepted: {dom}"
            );
        }
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichCodeProjectionEscapesMarkup
    fn go_table_rich_code_projection_escapes_markup() {
        let engine = rich_lute();
        let tree = crate::parse::parse(
            "",
            b"| title |\n| --- |\n| value |",
            engine.parse_options.clone(),
        );
        let cell = first_cell(&tree);
        cell.data_mut().table_cell_rich = Some(TableCellRich {
            spec: TABLE_CELL_RICH_SPEC,
            format: TABLE_CELL_RICH_FORMAT.to_string(),
            content: "```html\n<script>alert(1)</script>\na | b & c\n```".to_string(),
            ..Default::default()
        });
        apply_table_cell_rich_projection(cell).expect("projection");
        let dom = crate::blockdom::tree2blockdom(
            &tree,
            engine.render_options.clone(),
            engine.parse_options.clone(),
        );
        let inline = engine.blockdom2inline(&dom);
        let markdown = engine.blockdom2stdmd(&dom);
        for (name, output) in [
            ("preview", dom.as_str()),
            ("inline conversion", inline.as_str()),
            ("Markdown", markdown.as_str()),
        ] {
            assert!(
                !output.contains("<script>") && output.contains("&lt;script&gt;"),
                "{name} interpreted literal code as markup: {output}"
            );
        }
        assert!(
            markdown.contains("a &#124; b &amp; c&#10;</code>") && !markdown.contains("\na |"),
            "code content broke its Markdown table cell: {markdown}"
        );
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichDoesNotReinterpretLegacyCells
    fn go_table_rich_does_not_reinterpret_legacy_cells() {
        let engine = rich_lute();
        let dom = engine.md2blockdom("| title |\n| --- |\n| - literal<br />1. literal |", false);
        let spun = engine.spin_blockdom(&dom);
        let tree = crate::blockdom::blockdom2tree(&spun, engine.parse_options.clone());
        assert!(
            first_rich_cell(&tree).is_none(),
            "legacy cell was implicitly converted"
        );
        assert!(
            !dom.contains("<ul>") && !dom.contains("<ol>"),
            "legacy text became a list: {dom}"
        );
    }

    #[test]
    // Go: test/table_rich_test.go::TestTableRichRejectsUnsupportedPayloads
    fn go_table_rich_rejects_unsupported_payloads() {
        let payloads = [
            TableCellRich {
                spec: 2,
                format: "kramdown".to_string(),
                content: "preserve".to_string(),
                ..Default::default()
            },
            TableCellRich {
                spec: TABLE_CELL_RICH_SPEC,
                format: "future".to_string(),
                content: "preserve".to_string(),
                ..Default::default()
            },
            TableCellRich {
                spec: TABLE_CELL_RICH_SPEC,
                format: "kramdown".to_string(),
                content: "| nested |\n| --- |\n| table |".to_string(),
                ..Default::default()
            },
            TableCellRich {
                spec: TABLE_CELL_RICH_SPEC,
                format: "kramdown".to_string(),
                content: "```mermaid\ngraph LR\n```".to_string(),
                ..Default::default()
            },
            crate::ast::decode_table_cell_rich("invalid%payload"),
        ];
        // 损坏载荷不能靠深层嵌套耗尽调用栈；无效载荷保留原文，不做静默替换。
        let mut nested = String::from(r#"{"spec":1,"format":"kramdown","content":"x","deep":"#);
        nested.push_str(&"[".repeat(4096));
        nested.push_str(&"]".repeat(4096));
        nested.push('}');
        let encoded = crate::base64::encode_raw_url(nested.as_bytes());
        let rich = crate::ast::decode_table_cell_rich(&encoded);
        assert!(rich.invalid, "deeply nested payload must be rejected");
        assert_eq!(rich.raw, encoded, "invalid payload keeps its original text");

        for rich in payloads {
            let before = rich.encode();
            assert!(
                parse_table_cell_rich(&rich).is_err(),
                "accepted unsupported source: {rich:?}"
            );
            assert_eq!(
                rich.encode(),
                before,
                "validation changed the original payload"
            );
        }
    }
}

#[cfg(test)]
mod go_markdown_table_structure_tests {
    //! 迁移自 test/markdown_table_structure_test.go::TestMarkdownTableStructureRoundTrip。

    use super::*;
    use crate::lute::Lute;

    fn rich_lute() -> Lute {
        let mut engine = Lute::new();
        engine.parse_options = table_cell_rich_options();
        engine.parse_options.disable_table_cell_rich = false;
        engine.parse_options.kramdown_block_ial = true;
        engine.parse_options.kramdown_span_ial = true;
        engine.parse_options.text_mark = true;
        engine.render_options.protyle_wysiwyg = true;
        engine.render_options.kramdown_block_ial = true;
        engine.render_options.kramdown_span_ial = true;
        engine
    }

    fn first_rich_cell(tree: &Tree) -> Option<Node> {
        let mut ret = None;
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering && n.data().table_cell_rich.is_some() {
                ret = Some(n);
                return ast::WalkStatus::WalkStop;
            }
            ast::WalkStatus::WalkContinue
        });
        ret
    }

    /// 对应 Go 版 tableStructure：只记录块身份、任务标记和代码/公式内容。
    fn table_structure(tree: &Tree) -> Vec<String> {
        let mut ret = Vec::new();
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if matches!(
                n.typ(),
                NodeType::NodeKramdownBlockIAL | NodeType::NodeKramdownSpanIAL
            ) {
                return if entering {
                    ast::WalkStatus::WalkSkipChildren
                } else {
                    ast::WalkStatus::WalkContinue
                };
            }
            if n.is_block() {
                ret.push(if entering {
                    n.typ().to_string()
                } else {
                    format!("/{}", n.typ())
                });
            }
            if entering {
                match n.typ() {
                    NodeType::NodeTaskListItemMarker => {
                        ret.push(format!("task:{}", n.effective_task_list_item_marker()));
                    }
                    NodeType::NodeCodeBlockCode | NodeType::NodeMathBlockContent => {
                        ret.push(format!("{}:{}", n.typ(), n.tokens_str()));
                    }
                    _ => {}
                }
            }
            ast::WalkStatus::WalkContinue
        });
        ret
    }

    #[test]
    // Go: test/markdown_table_structure_test.go::TestMarkdownTableStructureRoundTrip
    fn go_markdown_table_structure_round_trip() {
        let engine = rich_lute();
        let sources = [
            "- [ ] first\n- [x] checked\n- [/] progress\n- [X] upper",
            "- outer\n  - nested\n    - [ ] task\n- tail",
            "first paragraph\n\nsecond **bold** paragraph\n\n> quote\n\n# heading",
            "`@BT@`go\na | b\n<span>literal</span> & &#10;\n`@BT@`",
            "$$\na | b\n$$",
            "7. seven\n8. eight",
            "- [ ] <span style=\"color:var(--b3-font-color6)\">colored</span>\n\n![image](assets/a.png)",
        ];
        for source in sources {
            let tree = crate::parse::parse(
                "",
                b"| head |\n| --- |\n| old |",
                engine.parse_options.clone(),
            );
            let cell = tree
                .root_raw()
                .first_child()
                .unwrap()
                .last_child()
                .unwrap()
                .first_child()
                .unwrap();
            let rich = TableCellRich {
                spec: TABLE_CELL_RICH_SPEC,
                format: TABLE_CELL_RICH_FORMAT.to_string(),
                content: source.to_string(),
                ..Default::default()
            };
            cell.data_mut().table_cell_rich = Some(rich.clone());
            apply_table_cell_rich_projection(cell).expect("projection");
            let original = parse_table_cell_rich(&rich).expect("original fragment");
            let mut dom = crate::blockdom::tree2blockdom(
                &tree,
                engine.render_options.clone(),
                engine.parse_options.clone(),
            );
            for round in 0..3 {
                let markdown = engine.blockdom2stdmd(&dom);
                assert!(
                    !markdown.contains("table-cell-rich")
                        && 3 == markdown.trim().split('\n').count(),
                    "nonstandard metadata or broken row: {markdown}"
                );
                dom = engine.md2blockdom(&markdown, false);
                let restored_tree =
                    crate::blockdom::blockdom2tree(&dom, engine.parse_options.clone());
                let restored =
                    first_rich_cell(&restored_tree).expect("rich cell missing after export");
                let fragment =
                    parse_table_cell_rich(restored.data().table_cell_rich.as_ref().unwrap())
                        .expect("restored fragment");
                assert_eq!(
                    table_structure(&fragment),
                    table_structure(&original),
                    "round {round} source: {source}"
                );
            }
        }
    }
}
