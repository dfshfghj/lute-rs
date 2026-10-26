// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 自动链接（尖括号、GFM 裸链接、邮箱）。

use crate::ast::{NodeType, RawNode as Node};
use crate::editor;
use crate::html;
use crate::lex;

use super::{InlineContext, InlineParser};
use crate::parse::tlds::is_domain_suffix;

const MARKERS: &[u8] = b".!#$%&'*+/=?^`{|}~";

impl<'a> InlineParser<'a> {
    /// 对应 Go 版 Tree.newLink。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new_link(
        &mut self,
        typ: NodeType,
        text: &[u8],
        dest: &[u8],
        title: Option<&[u8]>,
        link_type: i32,
    ) -> Node {
        let editor_mode = self.editor_mode();
        let encoded_caret_tokens: &[u8] = b"%E2%80%B8";
        let caret = editor::CARET.as_bytes();

        let mut text = text.to_vec();
        let mut dest = dest.to_vec();
        let append_caret =
            editor_mode && text.ends_with(caret) && dest.ends_with(encoded_caret_tokens);
        if append_caret {
            text.truncate(text.len() - caret.len());
            dest.truncate(dest.len() - encoded_caret_tokens.len());
        }
        let prepend_caret =
            editor_mode && text.starts_with(caret) && dest.starts_with(encoded_caret_tokens);
        if prepend_caret {
            text.drain(..caret.len());
            dest.drain(..encoded_caret_tokens.len());
        }
        let text = html::decode_destination(&text);

        let ret = self.new_node_raw(typ);
        ret.data_mut().link_type = link_type;
        if NodeType::NodeImage == typ {
            let bang = self.new_node_raw(NodeType::NodeBang);
            ret.append_child(bang);
        }
        let open = self.new_node_raw(NodeType::NodeOpenBracket);
        ret.append_child(open);
        let link_text = self.new_node_raw(NodeType::NodeLinkText);
        link_text.set_tokens(text.clone());
        ret.append_child(link_text);
        let close = self.new_node_raw(NodeType::NodeCloseBracket);
        ret.append_child(close);
        let open_paren = self.new_node_raw(NodeType::NodeOpenParen);
        ret.append_child(open_paren);
        let link_dest = self.new_node_raw(NodeType::NodeLinkDest);
        link_dest.set_tokens(dest);
        ret.append_child(link_dest);
        if let Some(title) = title {
            let link_title = self.new_node_raw(NodeType::NodeLinkTitle);
            link_title.set_tokens(title.to_vec());
            ret.append_child(link_title);
        }
        let close_paren = self.new_node_raw(NodeType::NodeCloseParen);
        ret.append_child(close_paren);
        if append_caret || prepend_caret {
            let caret_node = self.new_node_raw(NodeType::NodeText);
            caret_node.set_tokens(caret.to_vec());
            ret.insert_after(caret_node);
        }
        if 1 == link_type {
            ret.data_mut().link_ref_label = text;
        }
        ret
    }

    /// 对应 Go 版 Tree.parseAutolink。
    pub(super) fn parse_autolink(&mut self, ctx: &mut InlineContext) -> Option<Node> {
        let mut schemed = false;
        let mut scheme = String::new();
        let mut dest = Vec::new();
        let mut i = ctx.pos + 1;
        while i < ctx.tokens_len && b'>' != ctx.tokens[i] {
            let token = ctx.tokens[i];
            if b' ' == token {
                return None;
            }
            dest.push(token);
            if !schemed {
                if b':' != token {
                    scheme.push(token as char);
                } else {
                    schemed = true;
                }
            }
            i += 1;
        }
        if !schemed || 3 > scheme.len() || i == ctx.tokens_len {
            return None;
        }

        if b'>' != ctx.tokens[i] {
            return None;
        }
        ctx.pos = 1 + i;
        let encoded = html::encode_destination(&dest);
        Some(self.new_link(NodeType::NodeLink, &dest, &encoded, None, 2))
    }

    /// 对应 Go 版 Tree.parseAutoEmailLink。
    pub(super) fn parse_auto_email_link(&mut self, ctx: &mut InlineContext) -> Option<Node> {
        let tokens = &ctx.tokens[1..];
        let mut dest: Vec<u8> = Vec::new();
        let mut passed = 0usize;
        let mut i = 0usize;
        let mut at = false;
        let length = tokens.len();
        while i < length {
            let token = tokens[i];
            dest.push(token);
            passed += 1;
            if b'@' == token {
                at = true;
                break;
            }
            if !lex::is_ascii_letter_num_hyphen(token) && !MARKERS.contains(&token) {
                return None;
            }
            i += 1;
        }

        if 1 > i || !at {
            return None;
        }

        let domain_part = &tokens[i + 1..];
        let length = domain_part.len();
        i = 0;
        let mut closed = false;
        while i < length {
            let token = domain_part[i];
            passed += 1;
            if b'>' == token {
                closed = true;
                break;
            }
            dest.push(token);
            if !lex::is_ascii_letter_num_hyphen(token) && b'.' != token {
                return None;
            }
            if 63 < i {
                return None;
            }
            i += 1;
        }

        if 1 > i || !closed {
            return None;
        }

        ctx.pos += passed + 1;
        let mut dest_full = b"mailto:".to_vec();
        dest_full.extend_from_slice(&dest);
        Some(self.new_link(NodeType::NodeLink, &dest, &dest_full, None, 2))
    }

    /// 对应 Go 版 Tree.parseGFMAutoEmailLink。
    pub(super) fn parse_gfm_auto_email_link(&mut self, node: Node) {
        let mut child = node.first_child();
        while let Some(c) = child {
            let next = c.next();
            let is_link_parent = c.parent().map(|p| p.typ()) == Some(NodeType::NodeLink);
            if NodeType::NodeText == c.typ() && c.parent().is_some() && !is_link_parent {
                self.parse_gfm_auto_email_link0(c);
            } else {
                self.parse_gfm_auto_email_link(c);
            }
            child = next;
        }
    }

    /// 对应 Go 版 Tree.parseGFMAutoEmailLink0。
    fn parse_gfm_auto_email_link0(&mut self, node: Node) {
        if let (Some(prev), Some(next)) = (node.previous(), node.next()) {
            if NodeType::NodeInlineHTML == prev.typ()
                && NodeType::NodeInlineHTML == next.typ()
                && prev.tokens().starts_with(b"<a ")
                && b"</a>" == next.tokens()
            {
                return;
            }
        }

        let tokens = node.tokens_vec();
        if !tokens.contains(&b'@') {
            return;
        }

        let length = tokens.len();
        let mut i = 0usize;
        'part: while i < length {
            let mut group: Vec<u8> = Vec::new();
            let mut at_index = 0usize;
            let mut j = i;
            while j < length {
                let token = tokens[j];
                if !lex::is_whitespace(token) {
                    group.push(token);
                    if b'@' == token {
                        at_index = j - i;
                    }
                    j += 1;
                    continue;
                }
                break;
            }
            if i == j {
                // 第一个字符就是空白符
                self.add_previous_text(node, &[tokens[j]]);
                i += 1;
                continue;
            }

            i = j;

            if 0 == at_index {
                self.add_previous_text(node, &group);
                continue;
            }

            let mut k = 0;
            while k < at_index {
                let token = group[k];
                if !is_valid_email_segment1(token) {
                    self.add_previous_text(node, &group);
                    continue 'part;
                }
                k += 1;
            }

            k += 1; // 跳过 @
            let mut item = 0u8;
            while k < group.len() {
                item = group[k];
                if !is_valid_email_segment2(item) {
                    self.add_previous_text(node, &group);
                    continue 'part;
                }
                k += 1;
            }

            let last = *group.last().unwrap_or(&0);
            if b'.' == last {
                // 如果以 . 结尾则剔除该 .
                group.pop();
                let mut dest = b"mailto:".to_vec();
                dest.extend_from_slice(&group);
                let link = self.new_link(NodeType::NodeLink, &group, &dest, None, 2);
                node.insert_before(link);
                self.add_previous_text(node, &[item]);
            } else if b'-' == last || b'_' == last {
                self.add_previous_text(node, &group);
                continue 'part;
            } else {
                let link = self.new_node_raw(NodeType::NodeLink);
                link.data_mut().link_type = 2;
                let text = self.new_node_raw(NodeType::NodeLinkText);
                text.set_tokens(group.clone());
                link.append_child(text);
                let mut dest = b"mailto:".to_vec();
                dest.extend_from_slice(&group);
                let dest_node = self.new_node_raw(NodeType::NodeLinkDest);
                dest_node.set_tokens(dest);
                link.append_child(dest_node);
                node.insert_before(link);
            }
        }

        node.unlink();
    }

    /// 对应 Go 版 Tree.parseGFMAutoLink。
    pub(super) fn parse_gfm_auto_link(&mut self, node: Node) {
        let mut child = node.first_child();
        while let Some(c) = child {
            let next = c.next();
            if NodeType::NodeText == c.typ() {
                self.parse_gfm_auto_link0(c);
            } else {
                self.parse_gfm_auto_link(c);
            }
            child = next;
        }
    }

    /// 对应 Go 版 Tree.parseGFMAutoLink0。
    fn parse_gfm_auto_link0(&mut self, node: Node) {
        let tokens = node.tokens_vec();
        let length = tokens.len();
        let min_link_len = 5;
        if min_link_len > length {
            return;
        }

        let mut i = 0usize;
        let mut text_start = 0usize;
        let mut text_end = 0usize;
        let mut www = false;
        let mut need_unlink = false;

        let custom_end = rfind_sub(&tokens, b"://").map(|v| v as isize).unwrap_or(-1);
        let mut custom_start = custom_end;
        while 0 < custom_start && lex::is_ascii_letter_num(tokens[(custom_start - 1) as usize]) {
            custom_start -= 1;
        }
        let mut last_standard = rfind_sub(&tokens, b"http://")
            .map(|v| v as isize)
            .unwrap_or(-1);
        if let Some(index) = rfind_sub(&tokens, b"https://") {
            if last_standard < index as isize {
                last_standard = index as isize;
            }
        }
        if let Some(index) = rfind_sub(&tokens, b"ftp://") {
            if last_standard < index as isize {
                last_standard = index as isize;
            }
        }

        while i < length {
            let mut token = tokens[i];
            let protocol: Vec<u8>;
            let tmp_len = length - i;
            if 10 <= tmp_len && tokens[i..].starts_with(b"www.") {
                protocol = b"http://".to_vec();
                www = true;
            } else if 13 <= tmp_len && tokens[i..].starts_with(b"http://") {
                protocol = tokens[i..i + 7].to_vec();
                i += 7;
            } else if 14 <= tmp_len && tokens[i..].starts_with(b"https://") {
                protocol = tokens[i..i + 8].to_vec();
                i += 8;
            } else if 12 <= tmp_len && tokens[i..].starts_with(b"ftp://") {
                protocol = tokens[i..i + 6].to_vec();
                i += 6;
            } else if custom_start <= i as isize
                && (i as isize) < custom_end
                && last_standard < i as isize
                && custom_end + 3 < length as isize
            {
                // 自定义协议均认为是有效的
                protocol = tokens[i..(custom_end + 3) as usize].to_vec();
                i = (custom_end + 3) as usize;
            } else {
                text_end += 1;
                if length - i < min_link_len {
                    // 剩余字符不足，已经不可能形成链接了
                    if need_unlink {
                        if text_start < text_end {
                            self.add_previous_text(node, &tokens[text_start..]);
                        } else {
                            self.add_previous_text(node, &tokens[text_end..]);
                        }
                        node.unlink();
                    }
                    return;
                }
                i += 1;
                continue;
            }

            if text_start < text_end {
                self.add_previous_text(node, &tokens[text_start..text_end]);
                need_unlink = true;
                text_start = text_end;
            }

            let mut url: Vec<u8> = Vec::new();
            let mut colon_seen = false;
            let mut path_seen = false;
            let mut j = i;
            while j < length {
                token = tokens[j];
                if lex::is_whitespace(token) || b'<' == token {
                    break;
                }

                if colon_seen && !path_seen {
                    // 端口部分仅允许由数字组成
                    if !lex::is_digit(token) && b'/' != token && b'?' != token && b'#' != token {
                        break;
                    }
                }
                if !colon_seen && b':' == token && !tokens[j..].starts_with(b"://") {
                    colon_seen = true;
                }
                if b'/' == token || b'?' == token || b'#' == token {
                    path_seen = true;
                }

                url.push(token);
                j += 1;
            }
            if i == j {
                // 第一个字符就断开了
                if 0x80 <= token {
                    if !www {
                        url.extend_from_slice(&protocol);
                    }
                    while i < length {
                        token = tokens[i];
                        if 0x80 > token {
                            break;
                        }
                        url.push(token);
                        i += 1;
                    }
                } else {
                    url.push(token);
                    i += 1;
                }

                if let Some(prev) = node.previous() {
                    prev.append_tokens(&url);
                }

                text_start = i;
                text_end = i;
                continue;
            }

            i = j;

            let mut k = 0;
            while k < url.len() && b'/' != url[k] {
                k += 1;
            }
            let mut domain = url[..k].to_vec();
            let mut port: Vec<u8> = Vec::new();
            if let Some(idx) = domain.iter().position(|&b| b == b':') {
                if 0 < idx {
                    port = domain[idx..].to_vec();
                    domain.truncate(idx);
                }
            }

            if !self.is_valid_domain(&protocol, &domain) {
                self.add_previous_text(node, &tokens[text_start..i]);
                need_unlink = true;
                text_start = i;
                text_end = i;
                continue;
            }

            let mut open_parens = 0i32;
            let mut close_parens = 0i32;
            let mut path = url[k..].to_vec();
            let mut path_len = path.len();
            if 0 < path_len {
                let mut l;
                for l in 0..path_len {
                    token = path[l];
                    if b'(' == token {
                        open_parens += 1;
                    } else if b')' == token {
                        close_parens += 1;
                    }
                }

                let mut trimmed = false;
                let last_token = path[path_len - 1];
                if b')' == last_token {
                    // 以 ) 结尾的话需要计算圆括号匹配
                    let mut unmatches = close_parens - open_parens;
                    if 0 < unmatches {
                        l = path_len - 1;
                        while 0 < unmatches {
                            token = path[l];
                            if b')' != token {
                                break;
                            }
                            unmatches -= 1;
                            i -= 1;
                            if l == 0 {
                                break;
                            }
                            l -= 1;
                        }
                        path.truncate(l + 1);
                        path_len = path.len();
                        trimmed = true;
                    } else {
                        trimmed = true;
                    }
                } else if b';' == last_token {
                    // 检查 HTML 实体
                    let mut found_amp = false;
                    let mut amp_idx = 0usize;
                    for l in (0..path_len).rev() {
                        token = path[l];
                        if b'&' == token {
                            found_amp = true;
                            amp_idx = l;
                            break;
                        }
                    }
                    if found_amp {
                        let entity = &path[amp_idx..path_len];
                        if 3 <= entity.len() {
                            let mut is_entity = true;
                            for &e in &entity[1..entity.len() - 1] {
                                if !lex::is_ascii_letter_num(e) {
                                    is_entity = false;
                                    break;
                                }
                            }
                            if is_entity {
                                path.truncate(amp_idx);
                                trimmed = true;
                                i -= path_len - amp_idx;
                            }
                        }
                    }
                }

                // 结尾标点符号不计入链接
                if !trimmed
                    && matches!(
                        last_token,
                        b'?' | b'!' | b'.' | b',' | b':' | b'*' | b'_' | b'~'
                    )
                {
                    path.truncate(path_len - 1);
                    i -= 1;
                }
            } else {
                let domain_len = domain.len();
                let last_token = domain[domain_len - 1];
                if lex::is_ascii_punct(last_token) {
                    domain.truncate(domain_len - 1);
                    i -= 1;
                }
            }

            let mut dest = protocol.clone();
            dest.extend_from_slice(&domain);
            dest.extend_from_slice(&port);
            dest.extend_from_slice(&path);
            let mut addr: Vec<u8> = Vec::new();
            if !www {
                addr.extend_from_slice(&protocol);
            }
            addr.extend_from_slice(&domain);
            addr.extend_from_slice(&port);
            addr.extend_from_slice(&path);
            let mut link_text = addr;
            if link_text.starts_with(b"https://github.com/") {
                if link_text.windows(8).any(|w| w == b"/issues/") {
                    // 优化 GitHub Issues 自动链接文本
                    let repo_start = "https://github.com/".len();
                    let rest = &link_text[repo_start..];
                    if let Some(pos) = find_sub(rest, b"/issues/") {
                        let repo = rest[..pos].to_vec();
                        let num_part = &rest[pos + 8..];
                        let num_end = find_sub(num_part, b"?").unwrap_or(num_part.len());
                        let num = &num_part[..num_end];
                        if !num.is_empty() && num.iter().all(|&d| lex::is_digit(d)) {
                            let mut t = b"Issue #".to_vec();
                            t.extend_from_slice(num);
                            t.extend_from_slice(" · ".as_bytes());
                            t.extend_from_slice(&repo);
                            link_text = t;
                        }
                    }
                } else if link_text.windows(6).any(|w| w == b"/pull/") {
                    // 优化 GitHub Pull Requests 自动链接文本
                    let repo_start = "https://github.com/".len();
                    let rest = &link_text[repo_start..];
                    if let Some(pos) = find_sub(rest, b"/pull/") {
                        let repo = rest[..pos].to_vec();
                        let num_part = &rest[pos + 6..];
                        let num_end = find_sub(num_part, b"?").unwrap_or(num_part.len());
                        let num = &num_part[..num_end];
                        if !num.is_empty() && num.iter().all(|&d| lex::is_digit(d)) {
                            let mut t = b"Pull Request #".to_vec();
                            t.extend_from_slice(num);
                            t.extend_from_slice(" · ".as_bytes());
                            t.extend_from_slice(&repo);
                            link_text = t;
                        }
                    }
                }
            }

            let encoded = html::encode_destination(&dest);
            let link = self.new_link(NodeType::NodeLink, &link_text, &encoded, None, 2);
            node.insert_before(link);
            need_unlink = true;

            text_start = i;
            text_end = i;
        }

        if text_start < text_end {
            self.add_previous_text(node, &tokens[text_start..text_end]);
            need_unlink = true;
        }
        if need_unlink {
            node.unlink();
        }
    }

    /// 对应 Go 版 Tree.isValidDomain。
    fn is_valid_domain(&self, protocol: &[u8], domain: &[u8]) -> bool {
        if 0 < protocol.len()
            && !protocol.windows(7).any(|w| w == b"http://")
            && !protocol.windows(8).any(|w| w == b"https://")
            && !protocol.windows(6).any(|w| w == b"ftp://")
        {
            // 自定义协议均认为是有效的
            return true;
        }

        // 与 lex.Split(domain, '.') 的段数保持一致（末尾空段不计）
        let mut total = domain.iter().filter(|&&b| b == b'.').count() + 1;
        if 0 < domain.len() && b'.' == domain[domain.len() - 1] {
            total -= 1;
        }
        if 2 > total {
            return false;
        }

        let mut segments = 0usize;
        let length = domain.len();
        let mut i = 0usize;
        loop {
            let start = i;
            while i < length && b'.' != domain[i] {
                i += 1;
            }
            let segment = &domain[start..i];

            if 0 < segment.len() {
                for &token in segment {
                    if !lex::is_ascii_letter_num_hyphen(token) {
                        return false;
                    }
                    if 2 < segments && (segments == total - 2 || segments == total - 1) {
                        if b'_' == token {
                            return false;
                        }
                    }
                }

                if segments == total - 1 {
                    let suffix_is_digit = segment.iter().all(|&b| lex::is_digit(b));
                    let valid_suffix = if !suffix_is_digit {
                        is_domain_suffix(std::str::from_utf8(segment).unwrap_or(""))
                    } else {
                        true
                    };
                    if !valid_suffix {
                        return false;
                    }
                }
            }

            segments += 1;
            if i >= length {
                break;
            }
            i += 1; // 跳过 .
        }
        true
    }

    /// 对应 Go 版 Tree.addPreviousText。
    fn add_previous_text(&mut self, node: Node, tokens: &[u8]) {
        match node.previous() {
            Some(prev) if NodeType::NodeText == prev.typ() => prev.append_tokens(tokens),
            _ => {
                let text = self.new_node_raw(NodeType::NodeText);
                text.set_tokens(tokens.to_vec());
                node.insert_before(text);
            }
        }
    }
}

fn is_valid_email_segment1(token: u8) -> bool {
    lex::is_ascii_letter_num_hyphen(token) || b'.' == token || b'+' == token || b'_' == token
}

fn is_valid_email_segment2(token: u8) -> bool {
    lex::is_ascii_letter_num_hyphen(token) || b'.' == token || b'_' == token
}

fn find_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).position(|w| w == sub)
}

fn rfind_sub(tokens: &[u8], sub: &[u8]) -> Option<usize> {
    if sub.is_empty() || tokens.len() < sub.len() {
        return None;
    }
    tokens.windows(sub.len()).rposition(|w| w == sub)
}
