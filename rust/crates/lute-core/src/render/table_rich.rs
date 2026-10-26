// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 表格单元格富文本源的渲染，对应 Go 版 render/table_rich.go。

use crate::ast::{NodeType, RawNode as Node, TableCellRich, WalkStatus};
use crate::options::RenderOptions;
use crate::render::html::HtmlRenderer;
use crate::render::Renderer;

/// 输出片段的静态 HTML，不把内部节点作为可编辑文档块暴露。
///
/// 对应 Go 版 render.TableCellRichHTML。源无法解析时返回 None，调用方按原行为
/// 决定是否降级；Go 在 Protyle 路径下同样不输出内容。
pub fn table_cell_rich_html(rich: &TableCellRich, options: &RenderOptions) -> Option<Vec<u8>> {
    let tree = crate::parse::table_rich::parse_table_cell_rich(rich).ok()?;
    let mut remove = Vec::new();
    crate::ast::walk_raw(tree.root_raw(), &mut |n, entering| {
        if entering && n.is_block() {
            n.set_id("");
            n.remove_ial_attr("id");
            n.remove_ial_attr("updated");
            if NodeType::NodeKramdownBlockIAL == n.typ() {
                remove.push(n);
            }
        }
        WalkStatus::WalkContinue
    });
    for n in remove {
        n.unlink();
    }

    let mut opt = options.clone();
    opt.protyle_wysiwyg = false;
    opt.kramdown_block_ial = false;
    opt.kramdown_span_ial = false;
    opt.code_syntax_highlight = false;
    opt.sanitize = true;
    opt.heading_id = false;
    opt.data_task = true;
    let mut renderer = HtmlRenderer::with_parse_options(
        &tree,
        opt,
        crate::parse::table_rich::table_cell_rich_options(),
    );
    Some(renderer.render())
}

/// 将块结构写入单行 HTML，实体编码防止换行和管道符拆分表格。
///
/// 对应 Go 版 render.tableCellRichInlineHTML 的成功分支。
pub(crate) fn table_cell_rich_inline_html(cell: Node, options: &RenderOptions) -> Option<Vec<u8>> {
    let rich = cell.data().table_cell_rich.as_ref()?;
    let mut content = table_cell_rich_html(rich, options)?;
    content = crate::vditor::replace_all(&content, b"|", b"&#124;");
    content = crate::vditor::replace_all(&content, b"\r", b"&#13;");
    content = crate::vditor::replace_all(&content, b"\n", b"&#10;");
    Some(content)
}
