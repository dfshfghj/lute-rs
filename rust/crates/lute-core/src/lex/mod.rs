// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 词法与字节工具。与 Go 版 lex 包一一对应。

pub mod lexer;

pub use lexer::Lexer;

pub const ITEM_BACKTICK: u8 = b'`';
pub const ITEM_TILDE: u8 = b'~';
pub const ITEM_BANG: u8 = b'!';
pub const ITEM_CROSSHATCH: u8 = b'#';
pub const ITEM_ASTERISK: u8 = b'*';
pub const ITEM_OPEN_PAREN: u8 = b'(';
pub const ITEM_CLOSE_PAREN: u8 = b')';
pub const ITEM_HYPHEN: u8 = b'-';
pub const ITEM_UNDERSCORE: u8 = b'_';
pub const ITEM_PLUS: u8 = b'+';
pub const ITEM_EQUAL: u8 = b'=';
pub const ITEM_TAB: u8 = b'\t';
pub const ITEM_OPEN_BRACKET: u8 = b'[';
pub const ITEM_CLOSE_BRACKET: u8 = b']';
pub const ITEM_DOUBLEQUOTE: u8 = b'"';
pub const ITEM_SINGLEQUOTE: u8 = b'\'';
pub const ITEM_LESS: u8 = b'<';
pub const ITEM_GREATER: u8 = b'>';
pub const ITEM_SPACE: u8 = b' ';
pub const ITEM_NEWLINE: u8 = b'\n';
pub const ITEM_CARRIAGE_RETURN: u8 = b'\r';
pub const ITEM_BACKSLASH: u8 = b'\\';
pub const ITEM_SLASH: u8 = b'/';
pub const ITEM_DOT: u8 = b'.';
pub const ITEM_COLON: u8 = b':';
pub const ITEM_QUESTION: u8 = b'?';
pub const ITEM_AMPERSAND: u8 = b'&';
pub const ITEM_SEMICOLON: u8 = b';';
pub const ITEM_PIPE: u8 = b'|';
pub const ITEM_DOLLAR: u8 = b'$';
pub const ITEM_CARET: u8 = b'^';
pub const ITEM_OPEN_BRACE: u8 = b'{';
pub const ITEM_CLOSE_BRACE: u8 = b'}';

/// 判断 token 是否是空白。
#[inline]
pub fn is_whitespace(token: u8) -> bool {
    matches!(
        token,
        ITEM_SPACE | ITEM_NEWLINE | ITEM_TAB | 0x0B | 0x0C | 0x0D
    )
}

/// 判断字符是否是 Unicode 空白。
#[inline]
pub fn is_unicode_whitespace(r: char) -> bool {
    r.is_whitespace()
}

#[inline]
pub fn is_digit(token: u8) -> bool {
    token.is_ascii_digit()
}

#[inline]
pub fn is_hex_digit(token: u8) -> bool {
    token.is_ascii_hexdigit()
}

#[inline]
pub fn token_to_upper(token: u8) -> u8 {
    token.to_ascii_uppercase()
}

#[inline]
pub fn is_ascii_punct(token: u8) -> bool {
    (0x21..=0x2F).contains(&token)
        || (0x3A..=0x40).contains(&token)
        || (0x5B..=0x60).contains(&token)
        || (0x7B..=0x7E).contains(&token)
}

#[inline]
pub fn is_ascii_letter(token: u8) -> bool {
    token.is_ascii_alphabetic()
}

#[inline]
pub fn is_ascii_letter_num(token: u8) -> bool {
    token.is_ascii_alphanumeric()
}

/// 判断 tokens 是否全部由 ASCII 字母或数字组成。
pub fn is_ascii_letter_nums(tokens: &[u8]) -> bool {
    tokens.iter().all(|&t| is_ascii_letter_num(t))
}

#[inline]
pub fn is_ascii_letter_num_hyphen(token: u8) -> bool {
    token.is_ascii_alphanumeric() || ITEM_HYPHEN == token
}

/// 对应 Go 的 unicode.IsControl(rune(token))。
#[inline]
pub fn is_control(token: u8) -> bool {
    token < 0x20 || 0x7F == token || (0x80..=0x9F).contains(&token)
}

/// 判断 tokens 是否全部为空格。
pub fn is_blank(tokens: &[u8]) -> bool {
    tokens.iter().all(|&t| ITEM_SPACE == t)
}

/// 使用 separator 切分。空片段不会被追加。
pub fn split(tokens: &[u8], separator: u8) -> Vec<&[u8]> {
    let mut ret = Vec::new();
    let mut start = 0;
    for (i, &token) in tokens.iter().enumerate() {
        if separator == token {
            ret.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    if start < tokens.len() {
        ret.push(&tokens[start..]);
    }
    ret
}

/// 使用 separator 切分，被反斜杠转义的字符不计入切分。
pub fn split_without_backslash_escape(tokens: &[u8], separator: u8) -> Vec<&[u8]> {
    let mut ret = Vec::new();
    let mut start = 0;
    for (i, &token) in tokens.iter().enumerate() {
        if separator != token || is_backslash_escape_punct(tokens, i) {
            continue;
        }
        ret.push(&tokens[start..i]);
        start = i + 1;
    }
    if start < tokens.len() {
        ret.push(&tokens[start..]);
    }
    ret
}

/// 原地替换全部 old 为 new。
pub fn replace_all(tokens: &mut [u8], old: u8, new: u8) {
    for token in tokens.iter_mut() {
        if *token == old {
            *token = new;
        }
    }
}

/// 将末尾的 "\n " 之类序列裁剪为 "\n"。
pub fn replace_newline_space(tokens: &mut Vec<u8>) {
    let mut i = tokens.len() as isize - 1;
    while 0 <= i {
        let token = tokens[i as usize];
        if ITEM_NEWLINE != token && ITEM_SPACE != token {
            break;
        }
        if 0 < i
            && ITEM_NEWLINE == tokens[(i - 1) as usize]
            && (ITEM_SPACE == token || ITEM_NEWLINE == token)
        {
            tokens.truncate(i as usize);
        }
        i -= 1;
    }
}

pub fn trim_whitespace(tokens: &[u8]) -> &[u8] {
    let (_, _, remains) = trim(tokens);
    remains
}

/// 去掉首尾空白，同时返回被去掉的部分。
pub fn trim(tokens: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let length = tokens.len();
    if 0 == length {
        return (&[], &[], tokens);
    }

    let mut start = 0usize;
    while start < length && is_whitespace(tokens[start]) {
        start += 1;
    }
    let left = &tokens[0..start];
    if start == length {
        start -= 1;
    }

    let mut end = length as isize - 1;
    while 0 <= end && is_whitespace(tokens[end as usize]) {
        end -= 1;
    }
    if end < start as isize {
        end = start as isize - 1;
    }
    let right = if 0 < end {
        &tokens[(end + 1) as usize..length]
    } else {
        &[]
    };
    let remains = &tokens[start..(end + 1) as usize];
    (left, right, remains)
}

/// 去掉右侧空白，返回的 whitespaces 为逆序。
pub fn trim_right(tokens: &[u8]) -> (Vec<u8>, &[u8]) {
    let length = tokens.len();
    if 1 > length {
        return (Vec::new(), tokens);
    }

    let mut whitespaces = Vec::new();
    let mut i = length as isize - 1;
    while 0 <= i {
        if !is_whitespace(tokens[i as usize]) {
            break;
        }
        whitespaces.push(tokens[i as usize]);
        i -= 1;
    }
    (whitespaces, &tokens[0..(i + 1) as usize])
}

/// 去掉左侧空白。
pub fn trim_left(tokens: &[u8]) -> (Vec<u8>, &[u8]) {
    let length = tokens.len();
    if 1 > length {
        return (Vec::new(), tokens);
    }

    let mut whitespaces = Vec::new();
    let mut i = 0usize;
    while i < length {
        if !is_whitespace(tokens[i]) {
            break;
        }
        whitespaces.push(tokens[i]);
        i += 1;
    }
    (whitespaces, &tokens[i..])
}

/// 连续接受 token，返回接受的长度。
pub fn accept(tokens: &[u8], token: u8) -> usize {
    tokens.iter().take_while(|&&t| t == token).count()
}

/// 依次尝试多个前缀，返回成功匹配的长度，全部失败返回 -1。
pub fn accept_tokenss(tokens: &[u8], some_tokenss: &[&[u8]]) -> isize {
    for i in 0..tokens.len() {
        let remains = &tokens[i..];
        for some_tokens in some_tokenss {
            let pos = accept_tokens(remains, some_tokens);
            if 0 <= pos {
                return pos;
            }
        }
    }
    -1
}

/// 匹配前缀，返回匹配长度；不匹配返回 -1。
pub fn accept_tokens(remains: &[u8], some_tokens: &[u8]) -> isize {
    if remains.len() < some_tokens.len() {
        return -1;
    }
    for (i, &expected) in some_tokens.iter().enumerate() {
        if expected != remains[i] {
            return -1;
        }
    }
    some_tokens.len() as isize
}

pub fn is_blank_line(tokens: &[u8]) -> bool {
    tokens
        .iter()
        .all(|&t| ITEM_SPACE == t || ITEM_TAB == t || ITEM_NEWLINE == t)
}

/// 按空白切分，首个元素可能为空串。
pub fn split_whitespace(tokens: &[u8]) -> Vec<&[u8]> {
    let mut ret: Vec<&[u8]> = vec![&[]];
    let mut i = 0usize;
    let mut last_is_whitespace = false;
    let mut start = 0usize;

    for (pos, &token) in tokens.iter().enumerate() {
        if is_whitespace(token) {
            if !last_is_whitespace {
                ret[i] = &tokens[start..pos];
                i += 1;
                ret.push(&[]);
                start = pos + 1;
            }
            last_is_whitespace = true;
        } else {
            if last_is_whitespace {
                start = pos;
            }
            last_is_whitespace = false;
        }
    }
    if start < tokens.len() {
        ret[i] = &tokens[start..];
    }
    ret
}

/// 判断 pos 处的标点是否被反斜杠转义。
pub fn is_backslash_escape_punct(tokens: &[u8], pos: usize) -> bool {
    if !is_ascii_punct(tokens[pos]) {
        return false;
    }

    let mut backslashes = 0usize;
    let mut i = pos as isize - 1;
    while 0 <= i {
        if ITEM_BACKSLASH != tokens[i as usize] {
            break;
        }
        backslashes += 1;
        i -= 1;
    }
    !backslashes.is_multiple_of(2)
}

/// 统计换行、空格、制表符数量。
pub fn stat_whitespace(tokens: &[u8]) -> (usize, usize, usize) {
    let mut newlines = 0;
    let mut spaces = 0;
    let mut tabs = 0;
    for &token in tokens {
        if ITEM_NEWLINE == token {
            newlines += 1;
        } else if ITEM_SPACE == token {
            spaces += 1;
        } else if ITEM_TAB == token {
            tabs += 1;
        }
    }
    (newlines, spaces, tabs)
}

/// 对应 Go 版 Spnl：允许最多一个换行的空白。
pub fn spnl(tokens: &[u8]) -> (bool, Vec<u8>, &[u8]) {
    let (passed, remains) = trim_left(tokens);
    let (newlines, _, _) = stat_whitespace(&passed);
    if 1 < newlines {
        return (false, Vec::new(), tokens);
    }
    (true, passed, remains)
}

pub fn peek(tokens: &[u8], pos: usize) -> u8 {
    tokens.get(pos).copied().unwrap_or(0)
}

/// 字节数组展示为 UTF-8 字符串时的显示长度。
pub fn bytes_show_length(bytes: &[u8]) -> usize {
    let mut length = 0usize;
    for &b in bytes {
        if (b & 0xc0) != 0x80 {
            if b < 0x7f {
                length += 1;
            } else {
                length += 2;
            }
        }
    }
    length
}

/// 在表格单元格中为 | 补齐反斜杠。
pub fn repeat_backslash_before_pipe(content: &str) -> String {
    let mut ret = String::with_capacity(content.len() + 8);
    let mut last = 0u8;
    let mut backslash_cnt = 0usize;
    for &b in content.as_bytes() {
        if ITEM_PIPE == b {
            if ITEM_BACKSLASH != last {
                ret.push(ITEM_BACKSLASH as char);
            }
            if 1 <= backslash_cnt {
                ret.push(ITEM_BACKSLASH as char);
            }
        }
        last = b;
        if ITEM_BACKSLASH == last {
            backslash_cnt += 1;
        } else {
            backslash_cnt = 0;
        }
        ret.push(b as char);
    }
    ret
}

/// 转义 CommonMark 行级标记符。
pub fn escape_common_markers(tokens: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len() + 8);
    for &token in tokens {
        if is_common_inline_marker(token) {
            ret.push(ITEM_BACKSLASH);
        }
        ret.push(token);
    }
    ret
}

/// 转义 Protyle 行级标记符。
pub fn escape_protyle_markers(tokens: &[u8]) -> Vec<u8> {
    let mut ret = Vec::with_capacity(tokens.len() + 8);
    for &token in tokens {
        if is_protyle_inline_marker(token) {
            ret.push(ITEM_BACKSLASH);
        }
        ret.push(token);
    }
    ret
}

pub fn is_common_inline_marker(token: u8) -> bool {
    matches!(
        token,
        ITEM_ASTERISK | ITEM_UNDERSCORE | ITEM_BACKSLASH | ITEM_BACKTICK | ITEM_TILDE | ITEM_DOLLAR
    )
}

pub fn is_protyle_inline_marker(token: u8) -> bool {
    matches!(
        token,
        ITEM_ASTERISK
            | ITEM_UNDERSCORE
            | ITEM_BACKSLASH
            | ITEM_BACKTICK
            | ITEM_TILDE
            | ITEM_DOLLAR
            | ITEM_EQUAL
            | ITEM_CARET
            | ITEM_LESS
            | ITEM_GREATER
    )
}

pub fn is_marker(token: u8) -> bool {
    matches!(
        token,
        ITEM_ASTERISK
            | ITEM_UNDERSCORE
            | ITEM_OPEN_BRACKET
            | ITEM_BANG
            | ITEM_NEWLINE
            | ITEM_BACKSLASH
            | ITEM_BACKTICK
            | ITEM_LESS
            | ITEM_CLOSE_BRACKET
            | ITEM_AMPERSAND
            | ITEM_TILDE
            | ITEM_DOLLAR
            | ITEM_OPEN_BRACE
            | ITEM_OPEN_PAREN
            | ITEM_EQUAL
            | ITEM_CROSSHATCH
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_classes() {
        assert!(is_whitespace(b' '));
        assert!(is_whitespace(b'\n'));
        assert!(!is_whitespace(b'a'));
        assert!(is_digit(b'9'));
        assert!(!is_digit(b'a'));
        assert!(is_hex_digit(b'F'));
        assert!(!is_hex_digit(b'g'));
        assert_eq!(b'A', token_to_upper(b'a'));
        assert_eq!(b'A', token_to_upper(b'A'));
        assert!(is_ascii_punct(b'!'));
        assert!(is_ascii_punct(b'~'));
        assert!(!is_ascii_punct(b'a'));
        assert!(is_ascii_letter_num(b'0'));
        assert!(is_ascii_letter_num_hyphen(b'-'));
        assert!(!is_ascii_letter_num_hyphen(b'_'));
        assert!(is_control(0x00));
        assert!(is_control(0x7f));
        assert!(!is_control(b'a'));
    }

    #[test]
    fn split_keeps_empty_pieces_out() {
        assert_eq!(vec![&b"a"[..], &b"b"[..]], split(b"a,b", b','));
        assert_eq!(vec![&b"a"[..], &b""[..], &b"b"[..]], split(b"a,,b", b','));
        assert!(split(b"", b',').is_empty());
    }

    #[test]
    fn split_respects_backslash_escape() {
        assert_eq!(2, split_without_backslash_escape(b"a\\|b|c", b'|').len());
        assert_eq!(3, split_without_backslash_escape(b"a|b|c", b'|').len());
    }

    #[test]
    fn trim_matches_go() {
        let (left, right, remains) = trim(b"  a  ");
        assert_eq!(b"  ", left);
        assert_eq!(b"  ", right);
        assert_eq!(b"a", remains);

        let (_, _, remains) = trim(b"   ");
        assert!(remains.is_empty());

        let (left, right, remains) = trim(b"");
        assert!(left.is_empty() && right.is_empty() && remains.is_empty());
    }

    #[test]
    fn trim_left_right() {
        let (ws, remains) = trim_left(b"  ab");
        assert_eq!(b"  ", &ws[..]);
        assert_eq!(b"ab", remains);

        let (ws, remains) = trim_right(b"ab  ");
        assert_eq!(b"  ", &ws[..]);
        assert_eq!(b"ab", remains);
    }

    #[test]
    fn accept_prefix() {
        assert_eq!(3, accept(b"aaab", b'a'));
        assert_eq!(0, accept(b"baa", b'a'));
        assert_eq!(3, accept_tokens(b"abc", b"abc"));
        assert_eq!(-1, accept_tokens(b"abd", b"abc"));
        assert_eq!(-1, accept_tokens(b"ab", b"abc"));
        assert_eq!(2, accept_tokenss(b"xxab", &[b"ab", b"xx"]));
        assert_eq!(-1, accept_tokenss(b"zz", &[b"ab"]));
    }

    #[test]
    fn whitespace_stats() {
        assert_eq!((1, 2, 1), stat_whitespace(b"\n  \t"));
        assert!(is_blank_line(b" \t\n"));
        assert!(!is_blank_line(b" a"));
        assert!(is_blank(b"   "));
        assert!(!is_blank(b" a"));
    }

    #[test]
    fn spnl_allows_one_newline() {
        let (ok, _, remains) = spnl(b" \nab");
        assert!(ok);
        assert_eq!(b"ab", remains);
        let (ok, _, remains) = spnl(b"\n\nab");
        assert!(!ok);
        assert_eq!(b"\n\nab", remains);
    }

    #[test]
    fn split_whitespace_keeps_leading_empty() {
        let parts = split_whitespace(b"a b");
        assert_eq!(vec![&b"a"[..], &b"b"[..]], parts);

        let parts = split_whitespace(b" a");
        assert_eq!(vec![&b""[..], &b"a"[..]], parts);
    }

    #[test]
    fn display_length() {
        assert_eq!(3, bytes_show_length("abc".as_bytes()));
        assert_eq!(2, bytes_show_length("中".as_bytes()));
        assert_eq!(4, bytes_show_length("中文".as_bytes()));
    }

    #[test]
    fn escape_markers() {
        assert_eq!(b"\\*a\\_".to_vec(), escape_common_markers(b"*a_"));
        assert_eq!(b"\\=a".to_vec(), escape_protyle_markers(b"=a"));
        assert_eq!(b"=a".to_vec(), escape_common_markers(b"=a"));
    }

    #[test]
    fn backslash_before_pipe() {
        assert_eq!("a\\|b", repeat_backslash_before_pipe("a|b"));
        assert_eq!("a\\\\|b", repeat_backslash_before_pipe("a\\|b"));
    }
}

#[cfg(test)]
mod go_function_boundaries {
    use super::*;

    /// lex/token.go: IsWhitespace/IsControl/IsASCIIPunct across every byte.
    #[test]
    fn byte_class_boundaries_match_go_ranges() {
        for byte in 0..=u8::MAX {
            assert_eq!(matches!(byte, 9..=13 | 32), is_whitespace(byte));
            assert_eq!(matches!(byte, 0..=31 | 127..=159), is_control(byte));
            assert_eq!(
                matches!(byte, 33..=47 | 58..=64 | 91..=96 | 123..=126),
                is_ascii_punct(byte)
            );
        }
    }

    /// lex/token.go: SplitWithoutBackslashEscape and IsBackslashEscapePunct.
    #[test]
    fn separator_escape_parity_and_empty_segments() {
        for count in 0..=8 {
            let mut input = vec![b'\\'; count];
            input.extend_from_slice(b"|x|");
            assert_eq!(count % 2 == 1, is_backslash_escape_punct(&input, count));
            let pieces = split_without_backslash_escape(&input, b'|');
            assert_eq!(if count % 2 == 1 { 1 } else { 2 }, pieces.len());
            assert!(!pieces.last().unwrap().is_empty());
        }
        assert_eq!(vec![&b""[..], &b""[..]], split(b"||", b'|'));
        assert!(split_without_backslash_escape(b"", b'|').is_empty());
    }

    /// lex/token.go: Spnl must return untouched input when a second newline occurs.
    #[test]
    fn spnl_preserves_failed_input_and_passed_whitespace() {
        assert_eq!((true, b" \t\n\r".to_vec(), &b"x"[..]), spnl(b" \t\n\rx"));
        assert_eq!((false, Vec::new(), &b" \n\t\nx"[..]), spnl(b" \n\t\nx"));
        assert_eq!((true, Vec::new(), &b""[..]), spnl(b""));
    }

    /// lex/token.go: Trim has a deliberate end > 0 guard; preserve Go's tuple.
    #[test]
    fn trim_empty_single_character_and_all_whitespace() {
        assert_eq!((&b""[..], &b""[..], &b""[..]), trim(b""));
        assert_eq!((&b" "[..], &b""[..], &b""[..]), trim(b" "));
        assert_eq!((&b"  "[..], &b""[..], &b""[..]), trim(b"  "));
        assert_eq!((&b""[..], &b""[..], &b"x"[..]), trim(b"x "));
        assert_eq!((b"\n\t ".to_vec(), &b"x"[..]), trim_right(b"x \t\n"));
    }
}
