// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! GFM 表格。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::Parser;

impl<'a> Parser<'a> {
    /// 对应 Go 版 Context.parseTable。返回 (前置段落, 表格)。
    pub(super) fn parse_table(&mut self, paragraph: Node) -> Option<(Option<Node>, Node)> {
        let tokens = paragraph.tokens_vec();
        if crate::PROTYLE && self.options.protyle_wysiwyg {
            return self.parse_protyle_table(&tokens);
        }
        let length = tokens.len();
        let mut i = 0;
        while i < length {
            if lex::ITEM_NEWLINE == tokens[i] || 0 == i {
                let start = if 0 == i { i } else { i + 1 };
                if let Some(table) = self.parse_table0(&tokens[start..]) {
                    table.set_tokens(tokens[start..].to_vec());
                    let ret_paragraph = if 0 < i {
                        let p = self.tree.new_node_raw(NodeType::NodeParagraph);
                        p.set_tokens(tokens[..i].to_vec());
                        Some(p)
                    } else {
                        None
                    };
                    return Some((ret_paragraph, table));
                }
            }
            i += 1;
        }
        None
    }

    fn parse_protyle_table(&mut self, tokens: &[u8]) -> Option<(Option<Node>, Node)> {
        if !tokens.contains(&b'|') {
            return None;
        }
        let lines = lex::split(tokens, b'\n');
        let before: Vec<_> = lines
            .iter()
            .filter(|line| !line.contains(&b'|'))
            .copied()
            .collect();
        let lines = &lines[before.len()..];
        let delim = lines
            .iter()
            .position(|line| self.parse_table_delim_row(line).is_some())?;
        if delim < 1 {
            return None;
        }
        let aligns = self.parse_table_delim_row(lex::trim_whitespace(lines[delim]))?;
        let table = self.tree.new_node_raw(NodeType::NodeTable);
        table.data_mut().table_aligns = aligns.clone();
        let head = self.tree.new_node_raw(NodeType::NodeTableHead);
        table.append_child(head);
        for line in &lines[..delim] {
            let row = self.parse_table_row(lex::trim_whitespace(line), &aligns, true)?;
            self.extract_table_cell_ial(row, true);
            head.append_child(row);
        }
        for line in &lines[delim + 1..] {
            let row = self.parse_table_row(lex::trim_whitespace(line), &aligns, false)?;
            self.extract_table_cell_ial(row, true);
            table.append_child(row);
        }
        let paragraph = if before.is_empty() {
            None
        } else {
            let node = self.tree.new_node_raw(NodeType::NodeParagraph);
            node.set_tokens(before.join(&b'\n'));
            Some(node)
        };
        Some((paragraph, table))
    }

    /// Setext 标题起始时尝试解析表格。
    pub(super) fn parse_table0_for_setext(&mut self, tokens: Vec<u8>) -> Option<Node> {
        self.parse_table0(&tokens)
    }

    /// 对应 Go 版 Context.parseTable0。
    fn parse_table0(&mut self, tokens: &[u8]) -> Option<Node> {
        let head_end = tokens.iter().position(|&b| lex::ITEM_NEWLINE == b)?;
        let body = &tokens[head_end + 1..];
        let delim_end = body
            .iter()
            .position(|&b| lex::ITEM_NEWLINE == b)
            .unwrap_or(body.len());
        let delim_row = lex::trim_whitespace(&body[..delim_end]);
        let body = if delim_end < body.len() {
            &body[delim_end + 1..]
        } else {
            &[][..]
        };

        if 2 > delim_row.len() {
            // 换行+冒号会被识别为表格 https://github.com/88250/lute/issues/198
            return None;
        }

        let aligns = self.parse_table_delim_row(delim_row)?;

        if body.is_empty()
            && 1 == aligns.len()
            && 0 == aligns[0]
            && !tokens.contains(&lex::ITEM_PIPE)
        {
            // foo\n--- 与 Setext 标题规则冲突
            return None;
        }

        let head_row =
            self.parse_table_row(lex::trim_whitespace(&tokens[..head_end]), &aligns, true)?;
        self.extract_table_cell_ial(head_row, false);

        let table = self.tree.new_node_raw(NodeType::NodeTable);
        table.data_mut().table_aligns = aligns.clone();
        let head = self.new_table_head(head_row);
        table.append_child(head);

        for line in lex::split(body, lex::ITEM_NEWLINE) {
            let table_row = self.parse_table_row(lex::trim_whitespace(line), &aligns, false)?;
            self.extract_table_cell_ial(table_row, false);
            table.append_child(table_row);
        }

        Some(table)
    }

    fn extract_table_cell_ial(&mut self, row: Node, leading: bool) {
        for cell in row.children() {
            let tokens = cell.tokens_vec();
            let start = if leading {
                tokens.windows(2).position(|part| part == b"{:")
            } else {
                tokens.windows(2).rposition(|part| part == b"{:")
            };
            let Some(start) = start else {
                continue;
            };
            if leading && start != 0 {
                continue;
            }
            let Some((end, ial)) = super::inline::parse_span_ial(&tokens[start..]) else {
                continue;
            };
            let structural = ial.iter().any(|(key, value)| {
                matches!(
                    key.as_str(),
                    "colspan" | "rowspan" | "style" | "table-cell-rich"
                ) || key == "class" && value.split_whitespace().any(|class| class == "fn__none")
            });
            if !structural {
                continue;
            }
            cell.data_mut().kramdown_ial = ial;
            for declaration in cell.ial_attr("style").split(';') {
                let Some((property, value)) = declaration.split_once(':') else {
                    continue;
                };
                if !property.trim().eq_ignore_ascii_case("text-align") {
                    continue;
                }
                cell.data_mut().table_cell_align = match value
                    .trim()
                    .trim_end_matches("!important")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "left" => 1,
                    "center" => 2,
                    "right" => 3,
                    _ => 0,
                };
            }
            let span = self.tree.new_node_raw(NodeType::NodeKramdownSpanIAL);
            span.set_tokens(tokens[start..start + end + 1].to_vec());
            if leading {
                cell.prepend_child(span);
            } else {
                cell.insert_after(span);
            }
            let mut content = tokens[..start].to_vec();
            content.extend_from_slice(&tokens[start + end + 1..]);
            cell.set_tokens(content);
        }
    }

    fn new_table_head(&mut self, head_row: Node) -> Node {
        let head = self.tree.new_node_raw(NodeType::NodeTableHead);
        let tr = self.tree.new_node_raw(NodeType::NodeTableRow);
        head.append_child(tr);
        for c in head_row.children() {
            tr.append_child(c);
        }
        head
    }

    /// 对应 Go 版 Context.parseTableRow。
    fn parse_table_row(&mut self, line: &[u8], aligns: &[i32], is_head: bool) -> Option<Node> {
        let ret = self.tree.new_node_raw(NodeType::NodeTableRow);
        ret.data_mut().table_aligns = aligns.to_vec();

        let mut line = line.to_vec();
        if let Some(idx) = line.windows(2).position(|w| w == b"\\|") {
            if 0 < idx
                && (in_inline(&line, idx, lex::ITEM_DOLLAR)
                    || in_inline(&line, idx, lex::ITEM_BACKTICK))
            {
                line = replace_all_bytes(&line, b"\\|", b"\\&#124;");
            }
        }

        let mut cols: Vec<Vec<u8>> = lex::split_without_backslash_escape(&line, lex::ITEM_PIPE)
            .into_iter()
            .map(|c| c.to_vec())
            .collect();
        if cols.is_empty() {
            return None;
        }
        if lex::is_blank(&cols[0]) {
            cols.remove(0);
        }
        if !cols.is_empty() && lex::is_blank(cols.last().unwrap()) {
            cols.pop();
        }

        let cols_len = cols.len();
        let aligns_len = aligns.len();
        if is_head && cols_len > aligns_len {
            // 分隔符行定义了表的列数，表头列数大于该列数时不满足表格式
            return None;
        }

        let mut i = 0;
        while i < cols_len && i < aligns_len {
            let mut col = lex::trim_whitespace(&cols[i]).to_vec();
            col = replace_all_bytes(&col, b"&#124;", b"|");
            let cell = self.tree.new_node_raw(NodeType::NodeTableCell);
            cell.data_mut().table_cell_align = aligns[i];
            cell.set_tokens(col);
            ret.append_child(cell);
            i += 1;
        }

        // 可能需要补全剩余的列
        while i < aligns_len {
            let cell = self.tree.new_node_raw(NodeType::NodeTableCell);
            cell.data_mut().table_cell_align = aligns[i];
            ret.append_child(cell);
            i += 1;
        }
        Some(ret)
    }

    fn parse_table_delim_row(&self, line: &[u8]) -> Option<Vec<i32>> {
        if line.is_empty() {
            return None;
        }

        for &token in line {
            if lex::ITEM_PIPE != token
                && lex::ITEM_HYPHEN != token
                && lex::ITEM_COLON != token
                && lex::ITEM_SPACE != token
            {
                return None;
            }
        }

        let mut line = line.to_vec();
        if let Some(idx) = line.windows(2).position(|w| w == b"\\|") {
            if 0 < idx
                && (in_inline(&line, idx, lex::ITEM_DOLLAR)
                    || in_inline(&line, idx, lex::ITEM_BACKTICK))
            {
                line = replace_all_bytes(&line, b"\\|", b"\\&#124;");
            }
        }

        let mut cols: Vec<Vec<u8>> = lex::split_without_backslash_escape(&line, lex::ITEM_PIPE)
            .into_iter()
            .map(|c| c.to_vec())
            .collect();
        if lex::is_blank(&cols[0]) {
            cols.remove(0);
        }
        if !cols.is_empty() && lex::is_blank(cols.last().unwrap()) {
            cols.pop();
        }

        let mut alignments = Vec::with_capacity(cols.len());
        for col in cols {
            let mut col = lex::trim_whitespace(&col).to_vec();
            col = replace_all_bytes(&col, b"&#124;", b"|");
            if col.is_empty() {
                return None;
            }
            let align = table_delim_align(&col);
            if -1 == align {
                return None;
            }
            alignments.push(align);
        }
        Some(alignments)
    }
}

/// 对应 Go 版 tableDelimAlign。
fn table_delim_align(col: &[u8]) -> i32 {
    let length = col.len();
    if 1 > length {
        return -1;
    }

    let left = lex::ITEM_COLON == col[0];
    let right = lex::ITEM_COLON == col[length - 1];

    let mut i = 1;
    while i + 1 < length {
        if lex::ITEM_HYPHEN != col[i] {
            return -1;
        }
        i += 1;
    }

    if left && right {
        return 2;
    }
    if left {
        return 1;
    }
    if right {
        return 3;
    }
    0
}

/// 对应 Go 版 inInline。
fn in_inline(tokens: &[u8], i: usize, math_or_code_marker: u8) -> bool {
    if i + 1 >= tokens.len() || i < 1 {
        return false;
    }

    let start = tokens[..i].iter().position(|&b| b == math_or_code_marker);
    let count = tokens[..i]
        .iter()
        .filter(|&&b| b == math_or_code_marker)
        .count();
    if 0 == count % 2 {
        return false;
    }
    let end = tokens[i + 1..]
        .iter()
        .position(|&b| b == math_or_code_marker);
    start.is_some() && end.is_some()
}

fn replace_all_bytes(tokens: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
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
mod go_table_tests {
    /// Go parse/performance_regression_test.go: TestTableCandidateCompatibility,
    /// direct parseTable0 assertion (independent of Setext block precedence).
    #[test]
    fn go_table_candidate_direct_single_column() {
        let mut tree = crate::ast::Tree::new();
        let mut parser =
            crate::parse::Parser::new(&mut tree, b"", crate::options::ParseOptions::new());
        assert!(parser.parse_table0(b"foo\n---\nbar").is_some());
    }
}

#[cfg(test)]
mod go_table_structural_tests {
    use crate::{
        ast::{self, TableCellRich, WalkStatus},
        lute::Lute,
        parse::parse,
        render::format::FormatRenderer,
    };

    fn engine() -> Lute {
        let mut e = Lute::new();
        e.parse_options.protyle_wysiwyg = true;
        e.render_options.protyle_wysiwyg = true;
        e.parse_options.kramdown_block_ial = true;
        e.parse_options.kramdown_span_ial = true;
        e.render_options.kramdown_block_ial = true;
        e.render_options.kramdown_span_ial = true;
        e
    }

    fn rich_engine() -> Lute {
        let mut e = engine();
        let p = &mut e.parse_options;
        p.text_mark = true;
        p.html_tag2_text_mark = true;
        p.block_ref = true;
        p.file_annotation_ref = true;
        p.emoji = false;
        p.super_block = true;
        p.custom_block = true;
        p.git_conflict = true;
        p.callout = true;
        p.tabs = true;
        p.img_path_allow_space = true;
        p.sup = true;
        p.sub = true;
        p.tag = true;
        p.mark = true;
        p.gfm_strikethrough1 = false;
        p.full_width_strikethrough = true;
        p.inline_math_allow_digit_after_open_marker = true;
        p.footnotes = false;
        p.toc = false;
        p.heading_id = false;
        p.setext = false;
        p.yaml_front_matter = false;
        p.link_ref = false;
        p.indent_code_block = false;
        p.paragraph_beginning_space = true;
        p.arbitrary_task_list_item_marker = true;
        p.ensure_list_item_paragraph = true;
        e
    }
    // Go: test/table_alignment_style_test.go::TestLegacyTableCellAlignmentSurvivesMarkdownExport
    #[test]
    fn go_legacy_table_cell_alignment_survives_markdown_export() {
        let e = engine();
        let tree = parse(
            "",
            b"| Header |\n| :---: |\n| value |\n",
            e.parse_options.clone(),
        );
        let table = tree.root_raw().first_child().unwrap();
        let cell = table
            .first_child()
            .unwrap()
            .next()
            .unwrap()
            .first_child()
            .unwrap();
        cell.data_mut().table_cell_align = 3;
        let mut renderer = FormatRenderer::new_protyle_export(
            &tree,
            e.render_options.clone(),
            e.parse_options.clone(),
        );
        let md = String::from_utf8(renderer.render()).unwrap();
        assert!(md.contains("text-align: right;"),"Source: test/table_alignment_style_test.go::TestLegacyTableCellAlignmentSurvivesMarkdownExport\nCase: export-style\n{md}");
        let round = parse("", md.as_bytes(), e.parse_options);
        let cell = round
            .root_raw()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .next()
            .unwrap()
            .first_child()
            .unwrap();
        assert_eq!(cell.data().table_cell_align,3,"Source: test/table_alignment_style_test.go::TestLegacyTableCellAlignmentSurvivesMarkdownExport\nCase: reparsed-alignment\n{md}");
    }

    // Go: test/protyle_table_spans_test.go::TestBlockDOMTableSpansPreserveRichSource
    #[test]
    fn go_table_spans_preserve_rich_source() {
        let e = Lute::new();
        let rich = TableCellRich {
            spec: 1,
            format: "kramdown".into(),
            content: "- first\n- second".into(),
            ..Default::default()
        };
        let source=format!("<div data-type=\"NodeTable\" custom-sy-table-rich=\"1\"><div><table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td rowspan=\"2\" data-sy-table-cell-rich=\"{}\"></td><td>b</td></tr><tr><td>c</td></tr></tbody></table></div></div>",rich.encode());
        let tree = e.blockdom2tree(&source);
        let mut found = false;
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering {
                if let Some(read) = &n.data().table_cell_rich {
                    found = true;
                    assert_eq!((read.spec,&read.format,&read.content,&read.raw,read.invalid),(rich.spec,&rich.format,&rich.content,&rich.raw,rich.invalid),"Source: test/protyle_table_spans_test.go::TestBlockDOMTableSpansPreserveRichSource\nCase: unchanged-rich-source");
                }
            }
            WalkStatus::WalkContinue
        });
        assert!(found,"Source: test/protyle_table_spans_test.go::TestBlockDOMTableSpansPreserveRichSource\nCase: source-present");
    }

    // Go: test/table_alignment_style_test.go::TestTableCellInlineStyleOverridesLegacyAlign
    // The Docx rendering assertions are recorded as unported: no corresponding renderer exists.
    #[test]
    fn go_inline_style_overrides_legacy_alignment() {
        let e = engine();
        let dom = r#"<div data-type="NodeTable"><div contenteditable="true"><table><thead><tr><th align="left" style="background-color: red; text-align: right;">Header</th></tr></thead><tbody><tr><td align="center" style="vertical-align: middle;">value</td></tr></tbody></table></div></div>"#;
        let tree = e.blockdom2tree(dom);
        let head = tree
            .root_raw()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap();
        assert_eq!(head.data().table_cell_align,3,"Source: test/table_alignment_style_test.go::TestTableCellInlineStyleOverridesLegacyAlign\nCase: inline-overrides-align");
        let md = e.blockdom2md(dom);
        let round = e.md2blockdom(&md, false);
        assert!(round.contains("text-align: right")&&round.contains("vertical-align: middle"),"Source: test/table_alignment_style_test.go::TestTableCellInlineStyleOverridesLegacyAlign\nCase: markdown-round-trip\n{md}");
    }
    // Go: test/markdown_table_structure_test.go::TestHTMLTableStructureImport
    #[test]
    fn go_html_table_structure_import() {
        let e = rich_engine();
        let dom=e.html2blockdom("<table><tr><th>head</th></tr><tr><td colspan=\"2\"><ul><li>outer<ul><li>nested</li></ul></li></ul><pre><code class=\"language-go\">a | b\nc\n</code></pre></td></tr></table>");
        let tree = e.blockdom2tree(&dom);
        let mut found = None;
        ast::walk_raw(tree.root_raw(), &mut |n, entering| {
            if entering && n.data().table_cell_rich.is_some() {
                found = Some(n);
                return WalkStatus::WalkStop;
            }
            WalkStatus::WalkContinue
        });
        let cell=found.expect("Source: test/markdown_table_structure_test.go::TestHTMLTableStructureImport; Case: rich-cell-present");
        assert_eq!(cell.ial_attr("colspan"),"2","Source: test/markdown_table_structure_test.go::TestHTMLTableStructureImport; Case: colspan");
        assert!(cell.data().table_cell_rich.as_ref().unwrap().content.contains("a | b\nc\n")&&dom.contains("<ul>"),"Source: test/markdown_table_structure_test.go::TestHTMLTableStructureImport; Case: block-structure\n{dom}");
    }
    // Go: test/markdown_table_structure_test.go::TestMarkdownTableLegacyTasks
    #[test]
    fn go_markdown_table_legacy_tasks() {
        let e = rich_engine();
        let mut failures = Vec::new();
        for source in [
            "- [ ] task",
            "- [x] done<br />- [ ] todo",
            "- [/] custom<br/>  - [ ] nested",
            "- [ ] `<br />` literal",
        ] {
            let dom = e.md2blockdom(&format!("| head |\n| --- |\n| {source} |"), false);
            let tree = e.blockdom2tree(&dom);
            let mut found = None;
            ast::walk_raw(tree.root_raw(), &mut |n, entering| {
                if entering && n.data().table_cell_rich.is_some() {
                    found = Some(n);
                    return WalkStatus::WalkStop;
                }
                WalkStatus::WalkContinue
            });
            if found.is_none() || !dom.contains("type=\"checkbox\"") {
                failures.push(format!("Case: {source}/task-restored\n{dom}"));
            }
            if source.contains("`<br />`")
                && !found.is_some_and(|n| {
                    n.data()
                        .table_cell_rich
                        .as_ref()
                        .unwrap()
                        .content
                        .contains("`<br />`")
                })
            {
                failures.push(format!("Case: {source}/code-break-preserved\n{dom}"));
            }
        }
        assert!(
            failures.is_empty(),
            "Source: test/markdown_table_structure_test.go::TestMarkdownTableLegacyTasks\n{}",
            failures.join("\n")
        );
    }
}
