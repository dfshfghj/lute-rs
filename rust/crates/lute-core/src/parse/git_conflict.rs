// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Git 冲突标记。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 判断 Git 冲突标记是否开始。
pub fn git_conflict_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.git_conflict || p.ctx.indented {
        return BLOCK_START_NONE;
    }

    if !p.ctx.current_line.starts_with(b"<<<<<<<") {
        return BLOCK_START_NONE;
    }

    p.close_unmatched_blocks();
    p.add_child(NodeType::NodeGitConflict);
    BLOCK_START_LEAF
}

impl<'a> Parser<'a> {
    pub(super) fn git_conflict_continue(&mut self, git_conflict: Node) -> i32 {
        if self.ctx.current_line.starts_with(b">>>>>>>") {
            self.finalize(git_conflict);
            return 2;
        }
        0
    }

    pub(super) fn git_conflict_finalize(&mut self, git_conflict: Node) {
        let tokens = git_conflict.tokens_vec();
        let mut parts = tokens.split(|&b| b == lex::ITEM_NEWLINE);
        let open_marker = parts.next().unwrap_or(&[]).to_vec();
        let content = parts.collect::<Vec<_>>().join(&lex::ITEM_NEWLINE);
        let content = content
            .iter()
            .copied()
            .skip_while(|b| b.is_ascii_whitespace())
            .collect::<Vec<u8>>();
        let content = trim_ascii_end(&content);
        let close_marker = trim_ascii_end(&self.ctx.current_line.clone());

        git_conflict.clear_tokens();
        let open = self.tree.new_node_raw(NodeType::NodeGitConflictOpenMarker);
        open.set_tokens(open_marker);
        git_conflict.append_child(open);
        let c = self.tree.new_node_raw(NodeType::NodeGitConflictContent);
        c.set_tokens(content);
        git_conflict.append_child(c);
        let close = self.tree.new_node_raw(NodeType::NodeGitConflictCloseMarker);
        close.set_tokens(close_marker);
        git_conflict.append_child(close);
    }
}

fn trim_ascii_end(tokens: &[u8]) -> Vec<u8> {
    let mut end = tokens.len();
    while 0 < end && tokens[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    tokens[..end].to_vec()
}
