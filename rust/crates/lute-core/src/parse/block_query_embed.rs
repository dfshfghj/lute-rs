//! Block query embeds, aligned with Go parse/block_query_embed.go.

use crate::ast::{NodeType, RawNode as Node};
use crate::editor::CARET;
use crate::lex;
use crate::parse::{Parser, BLOCK_START_LEAF, BLOCK_START_NONE};

pub(super) fn block_query_embed_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.block_ref || p.ctx.indented {
        return BLOCK_START_NONE;
    }
    let Some(script) = parse_script(
        &p.ctx.current_line[p.ctx.next_nonspace..],
        p.options.protyle_wysiwyg,
    ) else {
        return BLOCK_START_NONE;
    };
    p.close_unmatched_blocks();
    while !p
        .ctx
        .tip
        .expect("tip")
        .can_contain(NodeType::NodeBlockQueryEmbed)
    {
        p.finalize(p.ctx.tip.expect("tip"));
    }
    let node = p.tree.new_node_raw(NodeType::NodeBlockQueryEmbed);
    for typ in [
        NodeType::NodeOpenBrace,
        NodeType::NodeOpenBrace,
        NodeType::NodeBlockQueryEmbedScript,
        NodeType::NodeCloseBrace,
        NodeType::NodeCloseBrace,
    ] {
        let child = p.tree.new_node_raw(typ);
        if typ == NodeType::NodeBlockQueryEmbedScript {
            child.set_tokens(script.clone());
        }
        node.append_child(child);
    }
    p.ctx.tip.expect("tip").append_child(node);
    p.ctx.tip = Some(node);
    BLOCK_START_LEAF
}

fn parse_script(tokens: &[u8], protyle: bool) -> Option<Vec<u8>> {
    let tokens = tokens.strip_prefix(CARET.as_bytes()).unwrap_or(tokens);
    let tokens = tokens.strip_prefix(b"{{")?;
    let mut tokens = lex::trim_whitespace(tokens).to_vec();
    if protyle {
        tokens = String::from_utf8_lossy(&tokens)
            .replace(CARET, "")
            .into_bytes();
    }
    let tokens = tokens.strip_suffix(b"}}")?;
    Some(lex::trim_whitespace(tokens).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_script_delimiters_and_editor_caret() {
        assert_eq!(
            parse_script(b"{{  SELECT *  }}\n", false),
            Some(b"SELECT *".to_vec())
        );
        assert_eq!(
            parse_script("‸{{ a‸ }}\n".as_bytes(), true),
            Some(b"a".to_vec())
        );
        assert_eq!(
            parse_script("{{ a‸ }}\n".as_bytes(), false),
            Some("a‸".as_bytes().to_vec())
        );
        for invalid in [b"{ x }}".as_slice(), b"{{ x }", b"{{ x }} suffix"] {
            assert_eq!(parse_script(invalid, false), None);
        }
    }
}
