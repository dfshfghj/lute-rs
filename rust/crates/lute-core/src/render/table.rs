// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

// Shared table structure from Go BaseRenderer.renderTableByHTML. Inline contents
// remain under the caller's renderer, including its escaping and Markdown markers.
use super::BaseRenderer;
use crate::ast::{NodeType, RawNode as Node, WalkStatus};

pub(crate) trait HtmlTableRenderer {
    fn table_base(&mut self) -> &mut BaseRenderer;
    fn render_table_node(&mut self, node: Node, entering: bool) -> WalkStatus;

    fn render_table_by_html(&mut self, table: Node) {
        let mut ials: Vec<(String, String)> = table
            .ial_raw()
            .into_iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "id" | "caption" | "updated" | "colgroup" | "custom-sy-table-rich"
                )
            })
            .collect();
        ials.sort_by(|a, b| a.0.cmp(&b.0));
        self.table_base().tag("table", &ials, false);
        self.table_base().newline();
        let caption = table.ial_attr("caption");
        if !caption.is_empty() {
            let caption =
                crate::html::unescape_html_str(&caption).replace("contenteditable=\"false\" ", "");
            self.table_base().write_str(&caption);
        }
        self.table_base().newline();

        self.table_base().tag("colgroup", &[], false);
        let colgroup = table.ial_attr("colgroup");
        if colgroup.is_empty() {
            if let Some(head) = table.child_by_type(NodeType::NodeTableHead) {
                if let Some(row) = head.first_child() {
                    let mut th = row.first_child();
                    while let Some(cell) = th {
                        if NodeType::NodeTableCell == cell.typ() {
                            let style = cell.ial_attr("style");
                            if !style.is_empty() {
                                self.table_base()
                                    .tag("col", &[("style".to_string(), style)], true);
                            } else {
                                self.table_base().tag("col", &[], true);
                            }
                        }
                        th = cell.next();
                    }
                }
            }
        } else {
            for style in colgroup.split('|') {
                if !style.is_empty() {
                    self.table_base()
                        .tag("col", &[("style".to_string(), style.to_string())], true);
                } else {
                    self.table_base().tag("col", &[], true);
                }
            }
        }
        self.table_base().tag("/colgroup", &[], false);

        let mut has_thead = false;
        let mut has_tbody = false;
        let mut child = table.first_child();
        while let Some(c) = child {
            if NodeType::NodeTableHead == c.typ() {
                self.table_base().tag("thead", &[], false);
                self.render_table_rows(c);
                self.table_base().tag("/thead", &[], false);
                has_thead = true;
            } else if NodeType::NodeTableRow == c.typ() {
                if !has_tbody {
                    self.table_base().tag("tbody", &[], false);
                    self.table_base().newline();
                    has_tbody = true;
                }
                self.render_table_row_html(c);
            }
            child = c.next();
        }
        if has_thead && !has_tbody {
            self.table_base().tag("tbody", &[], false);
            self.table_base().newline();
        }
        if has_tbody {
            self.table_base().tag("/tbody", &[], false);
            self.table_base().newline();
        }
        self.table_base().tag("/table", &[], false);
        self.table_base().newline();
    }

    fn render_table_rows(&mut self, thead: Node) {
        let mut row = thead.first_child();
        while let Some(r) = row {
            if NodeType::NodeTableRow == r.typ() {
                self.render_table_row_html(r);
            }
            row = r.next();
        }
    }

    fn render_table_row_html(&mut self, row: Node) {
        self.table_base().tag("tr", &[], false);
        self.table_base().newline();
        let mut cell = row.first_child();
        while let Some(c) = cell {
            if NodeType::NodeTableCell != c.typ() || c.ial_attr("class").contains("fn__none") {
                cell = c.next();
                continue;
            }
            let tag = if c
                .parent()
                .and_then(|p| p.parent())
                .map(|p| NodeType::NodeTableHead == p.typ())
                .unwrap_or(false)
            {
                "th"
            } else {
                "td"
            };
            let mut attrs: Vec<(String, String)> = c
                .ial_raw()
                .into_iter()
                .filter(|(k, _)| matches!(k.as_str(), "colspan" | "rowspan" | "style"))
                .collect();
            append_table_cell_align_style(c, &mut attrs);
            self.table_base().tag(tag, &attrs, false);
            // 富文本单元格直接输出片段，不展开投影后的行内节点。
            if c.data().table_cell_rich.is_some() {
                let inline = self.table_base().table_cell_rich_inline;
                let options = self.table_base().options.clone();
                let content = if inline {
                    crate::render::table_rich::table_cell_rich_inline_html(c, &options)
                } else {
                    crate::render::table_rich::table_cell_rich_html(
                        c.data().table_cell_rich.as_ref().unwrap(),
                        &options,
                    )
                };
                if let Some(content) = content {
                    self.table_base().write(&content);
                }
                self.table_base().tag(&format!("/{tag}"), &[], false);
                self.table_base().newline();
                cell = c.next();
                continue;
            }
            let mut child = c.first_child();
            while let Some(ch) = child {
                crate::ast::walk_raw(ch, &mut |n, entering| self.render_table_node(n, entering));
                child = ch.next();
            }
            self.table_base().tag(&format!("/{tag}"), &[], false);
            self.table_base().newline();
            cell = c.next();
        }
        self.table_base().tag("/tr", &[], false);
        self.table_base().newline();
    }
}

pub(crate) fn append_table_cell_align_style(node: Node, attrs: &mut Vec<(String, String)>) {
    let align = match node.data().table_cell_align {
        1 => "left",
        2 => "center",
        3 => "right",
        _ => return,
    };
    for (k, v) in attrs.iter_mut() {
        if k != "style" {
            continue;
        }
        for declaration in v.split(';') {
            if let Some((property, _)) = declaration.split_once(':') {
                if property.trim().eq_ignore_ascii_case("text-align") {
                    return;
                }
            }
        }
        let mut style = v.trim().to_string();
        if !style.is_empty() && !style.ends_with(';') {
            style.push(';');
        }
        if !style.is_empty() {
            style.push(' ');
        }
        *v = format!("{style}text-align: {align};");
        return;
    }
    attrs.push(("style".to_string(), format!("text-align: {align};")));
}

pub(crate) fn need_use_html_table(table: Node) -> bool {
    let mut ret = false;
    crate::ast::walk_raw(table, &mut |n, entering| {
        if !entering {
            return WalkStatus::WalkContinue;
        }
        if NodeType::NodeTable == n.typ() {
            if !n.ial_attr("caption").is_empty() {
                ret = true;
                return WalkStatus::WalkStop;
            }
        } else if NodeType::NodeTableCell == n.typ() {
            for (k, v) in n.ial_raw() {
                if ("colspan" == k || "rowspan" == k) && "1" != v {
                    ret = true;
                    return WalkStatus::WalkStop;
                }
            }
        }
        WalkStatus::WalkContinue
    });
    ret
}
