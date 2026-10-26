// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Emoji 别名替换。

use crate::ast::{NodeType, RawNode as Node};
use crate::lex;

use super::InlineParser;

const EMOJI_SITE_PLACEHOLDER: &[u8] = b"${emojiSite}";

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.emoji。
    pub(super) fn emoji(&mut self, node: Node) {
        let mut child = node.first_child();
        while let Some(c) = child {
            let next = c.next();
            if NodeType::NodeText == c.typ() || NodeType::NodeLinkText == c.typ() {
                self.emoji0(c);
            } else {
                self.emoji(c);
            }
            child = next;
        }
    }

    /// 对应 Go 版 Tree.emoji0。
    fn emoji0(&mut self, first: Node) {
        let tokens = first.tokens_vec();
        if 0 < tokens.len() && !tokens.contains(&b':') {
            return;
        }
        first.set_tokens(Vec::new());

        let length = tokens.len();
        let mut cur = first;
        let mut pos = 0usize;
        let mut i = 0usize;
        while i < length {
            let token = tokens[i];
            if i == length - 1 {
                cur.append_tokens(&tokens[pos..]);
                break;
            }

            if b':' != token {
                i += 1;
                continue;
            }

            cur.append_tokens(&tokens[pos..i]);

            let mut match_close_colon = false;
            pos = i + 1;
            while pos < length {
                let token = tokens[pos];
                if lex::is_whitespace(token) {
                    break;
                }
                if b':' == token {
                    match_close_colon = true;
                    break;
                }
                pos += 1;
            }
            if !match_close_colon {
                cur.append_tokens(&tokens[i..pos]);
                i += 1;
                continue;
            }

            let maybe_emoji = &tokens[i + 1..pos];
            if 1 > maybe_emoji.len() {
                cur.append_tokens(&tokens[pos..pos + 1]);
                i += 1;
                continue;
            }

            let alias = String::from_utf8_lossy(maybe_emoji).to_string();
            if let Some(emoji) = self.options.alias_emoji.get(&alias).cloned() {
                let emoji_node = self.new_node_raw(NodeType::NodeEmoji);
                let unicode_or_img = self.new_node_raw(NodeType::NodeEmojiUnicode);
                emoji_node.append_child(unicode_or_img);
                let emoji_tokens = emoji.as_bytes();
                if contains_sub(emoji_tokens, EMOJI_SITE_PLACEHOLDER) {
                    // 有的 Emoji 是图片链接，需要单独处理
                    let suffix = if "huaji" == alias {
                        ".gif"
                    } else if "siyuan" == alias {
                        ".svg"
                    } else {
                        ".png"
                    };
                    let src = format!("{}/{}{}", self.options.emoji_site, alias, suffix);
                    unicode_or_img.set_typ(NodeType::NodeEmojiImg);
                    unicode_or_img.set_tokens(emoji_img_tokens(&alias, &src));
                } else if emoji_tokens.contains(&b'.') {
                    // 自定义 Emoji 路径用 . 判断
                    unicode_or_img.set_typ(NodeType::NodeEmojiImg);
                    unicode_or_img.set_tokens(emoji_img_tokens(&alias, &emoji));
                } else {
                    unicode_or_img.set_tokens(emoji_tokens.to_vec());
                }

                let alias_node = self.new_node_raw(NodeType::NodeEmojiAlias);
                alias_node.set_tokens(tokens[i..pos + 1].to_vec());
                unicode_or_img.append_child(alias_node);
                cur.insert_after(emoji_node);

                if pos + 1 < length {
                    // 在 Emoji 节点后插入一个内容为空的文本节点，留作下次迭代
                    let text = self.new_node_raw(NodeType::NodeText);
                    emoji_node.insert_after(text);
                    cur = text;
                }
            } else {
                cur.append_tokens(&tokens[i..pos + 1]);
            }

            pos += 1;
            i = pos;
        }

        // 丢弃空的文本节点
        if 1 > first.tokens().len() {
            first.unlink();
        }
        if 1 > cur.tokens().len() {
            cur.unlink();
        }
    }
}

/// 对应 Go 版 Tree.EmojiImgTokens。
fn emoji_img_tokens(alias: &str, src: &str) -> Vec<u8> {
    format!(
        "<img alt=\"{}\" class=\"emoji\" src=\"{}\" title=\"{}\" />",
        alias, src, alias
    )
    .into_bytes()
}

fn contains_sub(tokens: &[u8], sub: &[u8]) -> bool {
    if sub.is_empty() || tokens.len() < sub.len() {
        return false;
    }
    tokens.windows(sub.len()).any(|w| w == sub)
}
