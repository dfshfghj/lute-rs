// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! ATX / Setext 标题。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 判断 ATX 标题（#）是否开始。
pub fn atx_heading_start(p: &mut Parser, _container: Node) -> i32 {
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let parsed = p.parse_atx_heading();
    let (markers, content, level) = match parsed {
        Some(v) => v,
        None => return BLOCK_START_NONE,
    };

    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(content.len(), false);
    p.close_unmatched_blocks();
    let heading = p.add_child(NodeType::NodeHeading);
    heading.data_mut().heading_level = level;
    heading.set_tokens(content);
    p.add_child_marker(NodeType::NodeHeadingC8hMarker, &markers);
    p.ctx
        .advance_offset(p.ctx.current_line_len - p.ctx.offset, false);
    BLOCK_START_LEAF
}

/// 判断 Setext 标题（- =）是否开始。
pub fn setext_heading_start(p: &mut Parser, container: Node) -> i32 {
    if p.ctx.indented || NodeType::NodeParagraph != container.typ() || !p.options.setext {
        return BLOCK_START_NONE;
    }

    let level = p.parse_setext_heading();
    if 0 == level {
        return BLOCK_START_NONE;
    }

    if p.options.gfm_table {
        // 前两行可能解析出只有一个单元格的表，例如 "0\n-:\n-"
        if let Some(table) = p.parse_table0_for_setext(container.tokens_vec()) {
            container.set_typ(NodeType::NodeTable);
            container.data_mut().table_aligns = table.data().table_aligns.clone();
            for tr in table.children() {
                container.append_child(tr);
            }
            container.clear_tokens();
            return BLOCK_START_NONE;
        }
    }

    p.close_unmatched_blocks();

    // 解析链接引用定义
    loop {
        let tokens = container.tokens_vec();
        if tokens.is_empty() || lex::ITEM_OPEN_BRACKET != tokens[0] {
            break;
        }
        match p.parse_link_ref_def(&tokens) {
            Some(remains) => container.set_tokens(remains),
            None => break,
        }
    }

    if !container.tokens().is_empty() {
        let tokens = lex::trim_whitespace(container.tokens()).to_vec();
        let child = p.tree.new_node_raw(NodeType::NodeHeading);
        child.data_mut().heading_level = level;
        child.data_mut().heading_setext = true;
        child.set_tokens(tokens);
        container.insert_after(child);
        container.unlink();
        p.ctx.tip = Some(child);
        p.ctx
            .advance_offset(p.ctx.current_line_len - p.ctx.offset, false);
        return BLOCK_START_LEAF;
    }
    BLOCK_START_NONE
}

impl<'a> Parser<'a> {
    fn parse_atx_heading(&self) -> Option<(Vec<u8>, Vec<u8>, i32)> {
        let editor =
            self.options.vditor_wysiwyg || self.options.vditor_ir || self.options.vditor_sv;
        let mut tokens = self.ctx.current_line[self.ctx.next_nonspace..].to_vec();
        let start_caret = (editor || (crate::PROTYLE && self.options.protyle_wysiwyg))
            && tokens.starts_with(crate::editor::CARET.as_bytes());
        if start_caret {
            tokens = String::from_utf8_lossy(&tokens)
                .replace(crate::editor::CARET, "")
                .into_bytes();
        }
        if tokens.is_empty() || lex::ITEM_CROSSHATCH != tokens[0] {
            return None;
        }
        let in_caret = editor
            && tokens
                .windows("#‸#".len())
                .any(|part| part == "#‸#".as_bytes());
        if in_caret {
            tokens = String::from_utf8_lossy(&tokens)
                .replace(crate::editor::CARET, "")
                .into_bytes();
        }
        let level = lex::accept(&tokens, lex::ITEM_CROSSHATCH);
        if 6 < level {
            return None;
        }
        let end_caret = editor && tokens[level..].starts_with(" ‸".as_bytes());
        if end_caret {
            tokens = String::from_utf8_lossy(&tokens)
                .replace(crate::editor::CARET, "")
                .into_bytes();
        }

        if level < tokens.len() && !lex::is_whitespace(tokens[level]) {
            return None;
        }

        let markers = self.ctx.current_line
            [self.ctx.next_nonspace..self.ctx.next_nonspace + level + 1]
            .to_vec();

        let (_, tokens) = lex::trim_left(&tokens);
        let (_, tokens) = lex::trim_left(&tokens[level..]);

        let mut content = Vec::with_capacity(256);
        for &token in tokens {
            if lex::ITEM_NEWLINE == token {
                break;
            }
            content.push(token);
        }
        let content = lex::trim_whitespace(&content).to_vec();

        // 去掉结尾的闭合 # 序列
        let mut closing = content.len() as isize - 1;
        while 0 <= closing {
            let token = content[closing as usize];
            if lex::ITEM_CROSSHATCH == token {
                closing -= 1;
                continue;
            }
            if lex::ITEM_SPACE == token {
                break;
            }
            closing = content.len() as isize;
            break;
        }

        let mut content = if 0 >= closing {
            Vec::new()
        } else if 0 < closing {
            let mut c = content[..closing as usize].to_vec();
            c = lex::trim_whitespace(&c).to_vec();
            c
        } else {
            content
        };

        if start_caret || in_caret || end_caret {
            let mut with_caret = crate::editor::CARET.as_bytes().to_vec();
            with_caret.extend_from_slice(&content);
            content = with_caret;
        }

        let level = level as i32;
        Some((markers, content, level))
    }

    fn parse_setext_heading(&mut self) -> i32 {
        let editor = self.options.vditor_wysiwyg
            || self.options.vditor_ir
            || self.options.vditor_sv
            || (crate::PROTYLE && self.options.protyle_wysiwyg);
        let raw = lex::trim_whitespace(&self.ctx.current_line);
        let has_caret = editor
            && raw
                .windows(crate::editor::CARET.len())
                .any(|p| p == crate::editor::CARET.as_bytes());
        let owned;
        let ln = if has_caret {
            owned = String::from_utf8_lossy(raw)
                .replace(crate::editor::CARET, "")
                .into_bytes();
            owned.as_slice()
        } else {
            raw
        };
        if ln.is_empty() {
            return 0;
        }

        let marker = ln[0];
        if lex::ITEM_EQUAL != marker && lex::ITEM_HYPHEN != marker {
            return 0;
        }

        for &token in ln {
            if lex::ITEM_EQUAL != token && lex::ITEM_HYPHEN != token {
                return 0;
            }
            if marker != token {
                return 0;
            }
        }

        if has_caret {
            let tip = self.ctx.oldtip.expect("oldtip");
            let mut content = lex::trim_whitespace(tip.tokens()).to_vec();
            content.extend_from_slice(crate::editor::CARET.as_bytes());
            tip.set_tokens(content);
        }
        if lex::ITEM_HYPHEN == marker {
            2
        } else {
            1
        }
    }
}
