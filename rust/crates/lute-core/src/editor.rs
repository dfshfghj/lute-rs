// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 编辑器相关常量，与 Go 版 editor 包一致。

/// 插入符 U+2038。
pub const CARET: &str = "‸";
pub const CARET_NEWLINE: &str = "‸
";
pub const CARET_REPLACEMENT: &str = "caretreplacement";
pub const FRONT_END_CARET: &str = "<wbr>";
pub const FRONT_END_CARET_SELF_CLOSE: &str = "<wbr/>";
/// 属性值中的换行转义。
pub const IAL_VAL_ESC_NEWLINE: &str = "_esc_newline_";
/// 零宽空格。
pub const ZWSP: &str = "\u{200B}";
/// 单词连接符。
pub const WORD_JOINER: &str = "\u{2060}";
/// 零宽连字。
pub const ZWJ: &str = "‍";
