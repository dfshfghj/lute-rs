// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 提示块（> [!Note]），对应 Go 版 parse/callout.go。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;
use crate::parse::inline::link::parse_inline_link_dest;
use crate::parse::{Parser, BLOCK_START_CONTAINER, BLOCK_START_NONE};

/// 判断提示块是否开始。
pub fn callout_start(p: &mut Parser, _container: Node) -> i32 {
    if !p.options.callout {
        return BLOCK_START_NONE;
    }
    if p.ctx.indented {
        return BLOCK_START_NONE;
    }

    let marker = lex::peek(&p.ctx.current_line, p.ctx.next_nonspace);
    if lex::ITEM_GREATER != marker {
        return BLOCK_START_NONE;
    }

    p.ctx.advance_next_nonspace();
    p.ctx.advance_offset(1, false);

    let content =
        String::from_utf8_lossy(lex::trim_whitespace(&p.ctx.current_line[p.ctx.offset..]))
            .to_string();
    if !content.starts_with("[!") {
        return BLOCK_START_NONE;
    }
    if !content.contains(']') {
        return BLOCK_START_NONE;
    }

    p.close_unmatched_blocks();
    p.add_child(NodeType::NodeCallout);
    BLOCK_START_CONTAINER
}

impl<'a> Parser<'a> {
    pub(super) fn callout_continue(&mut self, _callout: Node) -> i32 {
        let ln = self.ctx.current_line.clone();
        if !self.ctx.indented && lex::peek(&ln, self.ctx.next_nonspace) == lex::ITEM_GREATER {
            self.ctx.advance_next_nonspace();
            self.ctx.advance_offset(1, false);
            let token = lex::peek(&ln, self.ctx.offset);
            if lex::ITEM_SPACE == token || lex::ITEM_TAB == token {
                self.ctx.advance_offset(1, true);
            }
            return 0;
        }
        1
    }

    pub(super) fn callout_finalize(&mut self, callout: Node) {
        let first = match callout.first_child() {
            Some(first) => first,
            None => return,
        };
        if first.next().is_none() && !first.tokens().contains(&lex::ITEM_NEWLINE) {
            // 只有一个子块的话转换为引述
            callout.set_typ(NodeType::NodeBlockquote);
            let marker = self.tree.new_node_raw(NodeType::NodeBlockquoteMarker);
            callout.prepend_child(marker);
            return;
        }

        let p = first;
        let tokens = p.tokens_vec();
        let lines = lex::split(&tokens, lex::ITEM_NEWLINE);
        let content = lex::trim_whitespace(lines[0]).to_vec();
        let close_bracket = content.iter().position(|&b| b == b']').unwrap_or(0);
        let typ =
            String::from_utf8_lossy(lex::trim_whitespace(&content[2..close_bracket])).to_string();
        let mut title =
            String::from_utf8_lossy(lex::trim_whitespace(&content[close_bracket + 1..]))
                .to_string();
        callout.data_mut().callout_type = typ.clone();
        callout.data_mut().callout_title_explicit = !title.is_empty();

        if let Some((icon, remains)) = self.parse_callout_image_icon(&title) {
            callout.data_mut().callout_icon = icon;
            callout.data_mut().callout_icon_type = 1;
            title = remains;
        }

        let icon = title.split(' ').next().unwrap_or_default().to_string();
        if callout.data().callout_icon.is_empty() && !icon.is_empty() {
            if is_emoji(&icon, &self.options.alias_emoji) {
                callout.data_mut().callout_icon = icon.clone();
                title = title[icon.len()..].trim().to_string();
            } else {
                let alias = icon.replace(':', "");
                if let Some(emoji) = self.options.alias_emoji.get(&alias).cloned() {
                    callout.data_mut().callout_icon = emoji.clone();
                    title = title[icon.len()..].trim().to_string();
                    if emoji.starts_with("/emojis/") {
                        callout.data_mut().callout_icon_type = 1;
                    }
                }
            }
        }

        if callout.data().callout_icon.is_empty() && is_built_in_callout_type(&typ) {
            callout.data_mut().callout_icon = get_callout_icon(&typ).to_string();
        }

        callout.data_mut().callout_title = title;
        let rest = if lines.len() > 1 {
            lines[1..].join(&lex::ITEM_NEWLINE)
        } else {
            Vec::new()
        };
        if rest.is_empty() {
            p.clear_tokens();
        } else {
            p.set_tokens(rest);
        }
    }

    fn parse_callout_image_icon(&self, title: &str) -> Option<(String, String)> {
        let marker = format!("![{}]", CALL_OUT_ICON_IMAGE_ALT);
        if !title.starts_with(&marker) {
            return None;
        }

        let (passed, mut rest, destination) =
            parse_inline_link_dest(&title.as_bytes()[marker.len()..], &self.options)?;
        let _ = passed;
        let destination = String::from_utf8_lossy(&destination).to_string();
        if !is_valid_callout_image_src(&destination) {
            return None;
        }
        if !rest.is_empty() && lex::ITEM_CLOSE_PAREN == rest[0] {
            rest.remove(0);
        }
        if !rest.is_empty() && !lex::is_whitespace(rest[0]) {
            return None;
        }
        Some((
            destination,
            String::from_utf8_lossy(lex::trim_whitespace(&rest)).to_string(),
        ))
    }
}

/// 提示块图标图片的 alt 文本。
const CALL_OUT_ICON_IMAGE_ALT: &str = "callout-icon";

pub(crate) fn is_built_in_callout_type(typ: &str) -> bool {
    matches!(typ, "NOTE" | "TIP" | "IMPORTANT" | "WARNING" | "CAUTION")
}

/// 对应 Go 版 ast.GetCalloutIcon。
pub fn get_callout_icon(typ: &str) -> &'static str {
    match typ {
        "NOTE" => "✏️",
        "TIP" => "💡",
        "IMPORTANT" => "❗",
        "WARNING" => "⚠️",
        "CAUTION" => "🚨",
        _ => "",
    }
}

/// 对应 Go 版 ast.GetCalloutTitle。
pub fn get_callout_title(typ: &str) -> &'static str {
    match typ {
        "NOTE" => "Note",
        "TIP" => "Tip",
        "IMPORTANT" => "Important",
        "WARNING" => "Warning",
        "CAUTION" => "Caution",
        _ => "",
    }
}

/// 对应 Go 版 parse.IsEmoji。
fn is_emoji(s: &str, alias_emoji: &std::collections::HashMap<String, String>) -> bool {
    alias_emoji.values().any(|v| v.starts_with(s))
}

/// 对应 Go 版 ast.IsValidCalloutImageSrc。
pub(crate) fn is_valid_callout_image_src(src: &str) -> bool {
    if src.starts_with("/emojis/") || src.starts_with("api/icon/") {
        return true;
    }
    let lower = src.to_lowercase();
    let idx = match lower.find("://") {
        Some(idx) => idx,
        None => return false,
    };
    let scheme = &lower[..idx];
    if "http" != scheme && "https" != scheme {
        return false;
    }
    let host = &lower[idx + 3..];
    let host = host.split('/').next().unwrap_or_default();
    !host.is_empty()
}
