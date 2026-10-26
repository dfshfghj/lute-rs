// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 段落。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Context, Parser};

/// 对应 Go 版 ParagraphContinue。
pub fn paragraph_continue(ctx: &Context) -> i32 {
    if ctx.blank {
        return 1;
    }
    0
}

/// 对应 Go 版 Context.parseToC。
fn parse_toc(p: Node, options: &crate::options::ParseOptions) -> bool {
    let lines = lex::split(p.tokens(), lex::ITEM_NEWLINE);
    if 1 != lines.len() {
        return false;
    }

    let mut content = lex::trim_whitespace(lines[0]).to_vec();
    if options.vditor_wysiwyg || options.vditor_ir || options.vditor_sv {
        let caret = crate::editor::CARET.as_bytes();
        let mut stripped = Vec::with_capacity(content.len());
        let mut i = 0;
        while i < content.len() {
            if content[i..].starts_with(caret) {
                i += caret.len();
            } else {
                stripped.push(content[i]);
                i += 1;
            }
        }
        content = stripped;
    }
    content.eq_ignore_ascii_case(b"[toc]")
}

impl<'a> Parser<'a> {
    /// 对应 Go 版 paragraphFinalize。返回 true 表示末梢已被改写（表格场景）。
    pub(super) fn paragraph_finalize(&mut self, p: Node) -> bool {
        let tokens = if self.options.paragraph_beginning_space {
            lex::trim_right(p.tokens()).1.to_vec()
        } else {
            lex::trim_whitespace(p.tokens()).to_vec()
        };
        if p.data().tokens_initialized || !tokens.is_empty() {
            p.set_tokens(tokens);
        }

        // 解析链接引用定义
        let mut has_reference_defs = false;
        loop {
            let tokens = p.tokens_vec();
            if tokens.is_empty() || lex::ITEM_OPEN_BRACKET != tokens[0] {
                break;
            }
            match self.parse_link_ref_def(&tokens) {
                Some(remains) => {
                    p.set_tokens(remains);
                    has_reference_defs = true;
                }
                None => break,
            }
        }
        if has_reference_defs && lex::is_blank_line(p.tokens()) {
            p.unlink();
        }

        if (crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial)
            && p.parent().is_some_and(|parent| {
                parent.typ() == NodeType::NodeListItem && parent.first_child() == Some(p)
            })
        {
            let ial = super::ial::tokens2_ial(p.tokens());
            if !ial.is_empty() {
                // An IAL-only first list paragraph describes an empty paragraph,
                // rather than the list item itself (Go paragraphFinalize).
                let empty = self.tree.new_node_raw(NodeType::NodeParagraph);
                empty.data_mut().kramdown_ial = ial;
                empty.set_id(empty.ial_attr("id"));
                p.parent().unwrap().append_child(empty);
                let attrs = self.tree.new_node_raw(NodeType::NodeKramdownBlockIAL);
                attrs.set_tokens(super::ial::ial_to_tokens(&empty.ial_raw()));
                empty.insert_after(attrs);
                p.unlink();
                return false;
            }
        }

        if self.options.gfm_task_list_item {
            if let Some(list_item) = p.parent() {
                if NodeType::NodeListItem == list_item.typ()
                    && list_item.first_child() == Some(p)
                    && list_item
                        .data()
                        .list_data
                        .as_ref()
                        .is_some_and(|d| d.typ == 3)
                {
                    let editor_mode = self.options.vditor_wysiwyg
                        || self.options.vditor_ir
                        || self.options.vditor_sv
                        || (crate::PROTYLE && self.options.protyle_wysiwyg);
                    let is_task = if crate::PROTYLE && self.options.protyle_wysiwyg {
                        3 <= p.tokens().len()
                    } else {
                        3 < p.tokens().len()
                    };
                    if is_task {
                        let mut tokens = p.tokens_vec();
                        if crate::CAP_KRAMDOWN_BLOCK_IAL && self.options.kramdown_block_ial {
                            let ial = super::ial::parse_kramdown_ial_in_list_item(&tokens);
                            if !ial.is_empty() {
                                if let Some(end) = tokens.iter().position(|&b| b == b'}') {
                                    tokens = tokens[end + 1..].to_vec();
                                    p.data_mut().kramdown_ial = ial;
                                }
                            }
                        }
                        let caret = crate::editor::CARET.as_bytes();
                        let candidate = (tokens.len() == 3
                            && tokens[0] == b'['
                            && tokens[2] == b']'
                            && tokens[1] != b']')
                            || (tokens.len() > 3
                                && (lex::is_whitespace(tokens[3])
                                    || tokens[3] == caret[0]
                                    || tokens[2] == caret[0]));
                        if candidate {
                            let mut caret_in_marker = false;
                            if editor_mode {
                                if let Some(close) = tokens.iter().position(|&b| b == b']') {
                                    caret_in_marker = tokens.starts_with(caret)
                                        || tokens[close + 1..].starts_with(caret)
                                        || tokens[1..close]
                                            .windows(caret.len())
                                            .any(|w| w == caret);
                                    if caret_in_marker {
                                        tokens = String::from_utf8_lossy(&tokens)
                                            .replace(crate::editor::CARET, "")
                                            .into_bytes();
                                    }
                                }
                            }
                            if tokens.len() >= 3
                                && self.options.is_valid_task_list_item_marker(tokens[1])
                            {
                                let task_marker =
                                    self.tree.new_node_raw(NodeType::NodeTaskListItemMarker);
                                task_marker.set_tokens(tokens[..3].to_vec());
                                task_marker.revive_from_marker(tokens[1]);
                                if crate::PROTYLE && self.options.protyle_wysiwyg {
                                    p.insert_before(task_marker);
                                } else {
                                    p.prepend_child(task_marker);
                                }
                                let mut content = tokens[3..].to_vec();
                                if editor_mode {
                                    content = lex::trim_whitespace(&content).to_vec();
                                    if caret_in_marker {
                                        let mut prefix =
                                            format!(" {}", crate::editor::CARET).into_bytes();
                                        prefix.extend(content);
                                        content = prefix;
                                    } else if !(crate::PROTYLE && self.options.protyle_wysiwyg) {
                                        content.insert(0, b' ');
                                    }
                                }
                                p.set_tokens(content);
                                if !p.tokens().is_empty() {
                                    let subtree = super::parse_with_task_list_probe(
                                        "",
                                        p.tokens(),
                                        self.options.clone(),
                                        true,
                                    );
                                    if let Some(sub_block) = subtree.root_raw().first_child() {
                                        if sub_block.typ() != NodeType::NodeParagraph {
                                            if !(crate::PROTYLE && self.options.protyle_wysiwyg) {
                                                let space =
                                                    self.tree.new_node_raw(NodeType::NodeText);
                                                space.set_tokens(b" ".to_vec());
                                                list_item.prepend_child(space);
                                            }
                                            if let Some(first) = p.first_child() {
                                                list_item.prepend_child(first);
                                            }
                                            sub_block.set_id(p.id().to_owned());
                                            sub_block.data_mut().kramdown_ial =
                                                p.data().kramdown_ial.clone();
                                            let mut blocks = Vec::new();
                                            let mut current = Some(sub_block);
                                            while let Some(block) = current {
                                                if block.typ() == NodeType::NodeKramdownBlockIAL
                                                    && super::ial::is_doc_ial2(
                                                        &super::ial::tokens2_ial(block.tokens()),
                                                    )
                                                {
                                                    break;
                                                }
                                                blocks.push(block);
                                                current = block.next();
                                            }
                                            for block in blocks.into_iter().rev() {
                                                p.insert_after(block);
                                            }
                                            p.unlink();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.options.gfm_table {
            if let Some((ret_paragraph, table)) = self.parse_table(p) {
                if let Some(ret_paragraph) = ret_paragraph {
                    p.set_tokens(ret_paragraph.tokens_vec());
                    p.insert_after(table);
                    table.data_mut().close = true;
                    self.ctx.tip = Some(table);
                    return true;
                }
                // 将该段落节点转成表节点
                p.set_typ(NodeType::NodeTable);
                p.data_mut().table_aligns = table.data().table_aligns.clone();
                for tr in table.children() {
                    p.append_child(tr);
                }
            }
        }

        if self.options.toc && parse_toc(p, &self.options) {
            // 将该段落节点转换成目录节点
            p.set_typ(NodeType::NodeToC);
        }
        false
    }
}
