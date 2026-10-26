// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// 回归测试：锁定从 Go 版逐字节对齐时容易丢失的细节。

use lute_core::lute::Lute;

/// Go 版 KityMinder 渲染器里 headingChildren 从 heading 自身开始遍历，
/// 首轮即 break，因此标题的 children 恒为空，后续块保持为兄弟节点。
#[test]
fn kityminder_heading_has_no_children() {
    let lute = Lute::new();
    let got = lute.render_kityminder_json("# hi\n\npara\n\n- a\n");
    let want = concat!(
        r#"{"root":{"data":{"layout":"right","text":"","id":"","type":"NodeDocument","isContainer":true},"children":["#,
        r#"{"data":{"text":"hi","id":"","type":"NodeHeading","isContainer":false},"children":[]},"#,
        r#"{"data":{"text":"para","id":"","type":"NodeParagraph","isContainer":false},"children":[]},"#,
        r#"{"data":{"priority": "iconList","text":"","id":"","type":"NodeList","isContainer":true},"children":[{"data":{"text":"a","id":"","type":"NodeListItem","isContainer":true},"children":[]}]}]}}"#
    );
    assert_eq!(want, got);
}

/// Protyle 模式先把行内节点平铺为文本标记，转义发生在解析阶段：
/// 双引号不再二次转义为 `&quot;`，链接文本与代码共用同一套输出。
#[test]
fn protyle_flattens_text_marks() {
    let lute = Lute::new();
    let got = lute.md2blockdom("x `\"q\"` y", false);
    assert!(
        got.contains("<span data-type=\"code\">\u{2060}\"q\"</span>"),
        "{got}"
    );
    assert!(!got.contains("&quot;"), "{got}");

    let got = lute.md2blockdom("[t \"x\"](u)", false);
    assert!(
        got.contains("<span data-type=\"a\" data-href=\"u\">t \"x\"</span>"),
        "{got}"
    );
}

/// 脚注引用渲染为 sup/a，脚注定义首个子节点前插入锚点行内 HTML。
#[test]
fn protyle_renders_footnotes() {
    let lute = Lute::new();
    let got = lute.md2blockdom("脚注[^1]\n\n[^1]: 说明\n", false);
    assert!(
        got.contains(
            "<sup class=\"footnotes-ref\" id=\"footnotes-ref-1\"><a href=\"#footnotes-def-1\">1</a></sup>"
        ),
        "{got}"
    );
    assert!(
        got.contains("&lt;span id=&quot;footnotes-def-1&quot;&gt;&lt;/span&gt;"),
        "{got}"
    );
}

/// 链接引用定义会触发 Go 版的空指针崩溃，wasm 侧需要抛出同样的错误文本。
#[test]
fn protyle_link_ref_def_matches_go_nil_pointer() {
    let lute = Lute::new();
    let err = lute.try_md2blockdom("[a]: b\n", false).unwrap_err();
    assert_eq!(
        "runtime error: invalid memory address or nil pointer dereference",
        err
    );
}

/// 表格格式化按“显示宽度”对齐：ASCII 记 1 列，其余字符记 2 列（对应 Go 版
/// lex.BytesShowLength），而不是按 UTF-8 字节数或字符数。
#[test]
fn format_table_uses_display_width() {
    let lute = Lute::new();
    let md = "|n|value|描述|\n|--|--|-|\n|😃| 短| description|\n|x短|long long long long |short|";
    let want = concat!(
        "| n   | value               | 描述        |\n",
        "| --- | ------------------- | ----------- |\n",
        "| 😃  | 短                  | description |\n",
        "| x短 | long long long long | short       |\n"
    );
    assert_eq!(want, lute.format("", md));
}
