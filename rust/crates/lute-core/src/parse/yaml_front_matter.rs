// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! YAML Front Matter。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

/// 判断 YAML Front Matter（---）是否开始。
pub fn yaml_front_matter_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.yaml_front_matter || p.ctx.indented || p.tree.root_raw().first_child().is_some() {
        return BLOCK_START_NONE;
    }

    if !parse_yaml_front_matter(p) {
        return BLOCK_START_NONE;
    }

    let node = p.tree.new_node_raw(NodeType::NodeYamlFrontMatter);
    p.tree.root_raw().append_child(node);
    p.ctx.tip = Some(node);
    BLOCK_START_LEAF
}

fn parse_yaml_front_matter(p: &Parser) -> bool {
    if lex::ITEM_HYPHEN != p.ctx.current_line[0] {
        return false;
    }

    let mut hyphen_length = 0;
    let mut i = 0;
    while i < p.ctx.current_line_len && lex::ITEM_HYPHEN == p.ctx.current_line[i] {
        hyphen_length += 1;
        i += 1;
    }
    3 == hyphen_length
}

impl<'a> Parser<'a> {
    /// 对应 Go 版 YamlFrontMatterContinue。
    pub(super) fn yaml_front_matter_continue(&mut self, node: Node) -> i32 {
        if is_yaml_front_matter_close(&self.ctx) {
            self.finalize(node);
            return 2;
        }
        0
    }

    pub(super) fn yaml_front_matter_finalize(&mut self, node: Node) {
        let tokens = node.tokens_vec();
        let mut tokens = if 3 <= tokens.len() {
            tokens[3..].to_vec()
        } else {
            Vec::new()
        };
        tokens = lex::trim_whitespace(&tokens).to_vec();
        if tokens.ends_with(b"---") {
            tokens.truncate(tokens.len() - 3);
        }
        node.set_tokens(tokens.clone());

        let open = self
            .tree
            .new_node_raw(NodeType::NodeYamlFrontMatterOpenMarker);
        node.append_child(open);
        let content = self.tree.new_node_raw(NodeType::NodeYamlFrontMatterContent);
        content.set_tokens(tokens);
        node.append_child(content);
        let close = self
            .tree
            .new_node_raw(NodeType::NodeYamlFrontMatterCloseMarker);
        node.append_child(close);
    }
}

fn is_yaml_front_matter_close(ctx: &crate::parse::Context) -> bool {
    if lex::ITEM_HYPHEN != ctx.current_line[0] {
        return false;
    }

    let mut hyphen_length = 0;
    let mut i = 0;
    while i < ctx.current_line_len && lex::ITEM_HYPHEN == ctx.current_line[i] {
        hyphen_length += 1;
        i += 1;
    }
    3 == hyphen_length
}
