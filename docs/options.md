# 可配置选项清单

Lute 的配置全部落在两个结构体上：`ParseOptions`（解析，52 个字段）与
`RenderOptions`（渲染，42 个字段），去重后 **85 个字段**。JS 侧通过 76 个
`SetXxx` 方法暴露，另有 6 个数据注入方法（`PutEmojis` / `PutTerms` /
`AddAutoLinkDomainSuffix` 等）。剩下 12 个字段没有 JS 入口，属于引擎内部状态。

默认值与原 Go 版 `parse.NewOptions()` / `render.NewOptions()` 逐字段一致；
下表「默认」列是 Go 的默认值，不是建议值。

> 哪些选项可以编译期裁掉、怎么裁、实测省多少，见 `docs/feature-trim.md`。

## 1. GFM 基础语法

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `gfm_table` | `SetGFMTable` | on | 表格 |
| `gfm_task_list_item` | `SetGFMTaskListItem` | on | 任务列表 `- [ ]` |
| `gfm_strikethrough` | `SetGFMStrikethrough` | on | `~~删除线~~` |
| `gfm_strikethrough1` | `SetGFMStrikethrough1` | on | 单波浪线 `~删除线~` |
| `full_width_strikethrough` | `SetFullWidthStrikethrough` | off | 全角 `～～删除线～～` |
| `gfm_auto_link` | `SetGFMAutoLink` | on | 裸域名自动链接（依赖 TLD 表） |

## 2. 块级扩展

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `footnotes` | `SetFootnotes` | on | 脚注 |
| `toc` | `SetToC` | off | `[TOC]` 目录 |
| `heading_id` | `SetHeadingID` | parse on / render off | 标题自动生成 id |
| `yaml_front_matter` | `SetYamlFrontMatter` | on | `---` 头信息 |
| `setext` | `SetSetext` | on | `===` / `---` 标题 |
| `super_block` | `SetSuperBlock` | off | 超级块 |
| `custom_block` | `SetCustomBlock` | off | 自定义块 |
| `callout` | `SetCallout` | off | `::: tip` 引述块 |
| `tabs` | `SetTabs` | off | `::: tabs` 页签 |
| `block_ref` | `SetBlockRef` | off | 块引用 `((id "文本"))`（Protyle/Vditor 模式） |
| `file_annotation_ref` | `SetFileAnnotationRef` | off | 文件注解引用（来自 HTML，Protyle 用） |
| `tag` | `SetTag` | off | `#标签#`（Protyle/Vditor 模式） |
| `indent_code_block` | `SetIndentCodeBlock` | on | 四空格缩进代码块 |

## 3. 行内扩展

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `inline_math` | `SetInlineMath` | on | `$公式$` |
| `inline_math_allow_digit_after_open_marker` | `SetInlineMathAllowDigitAfterOpenMarker` | off | `$1$` 这种数字开头也认 |
| `mark` | `SetMark` | off | `==高亮==` |
| `sup` | `SetSup` | off | `^上标^` |
| `sub` | `SetSub` | off | `~下标~` |
| `inline_asterisk` | `SetInlineAsterisk` | on | `*` 作强调定界符 |
| `inline_underscore` | `SetInlineUnderscore` | on | `_` 作强调定界符 |
| `emoji` | `SetEmoji` | on | `:alias:` 转 emoji（依赖内置别名表） |
| `link_ref` | `SetLinkRef` | on | 引用式链接 `[a][b]` |
| `git_conflict` | `SetGitConflict` | off | `<<<<<<<` 冲突标记 |
| `paragraph_beginning_space` | `SetParagraphBeginningSpace` | off | 段首空格转缩进 |
| `data_image` | `SetDataImage` | on | 允许 `data:image/...` |
| `img_path_allow_space` | `SetImgPathAllowSpace` | off | 图片路径允许空格 |
| `html_tag2_text_mark` | `SetHTMLTag2TextMark` | off | HTML 标签转文本标记 |
| `text_mark` | `SetTextMark` | off | Protyle 文本标记节点 |

## 4. 属性 / 导出约定

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `kramdown_block_ial` | `SetKramdownBlockIAL` / `SetKramdownIAL` | off | 块级 IAL `{: id="x"}` |
| `kramdown_span_ial` | `SetKramdownSpanIAL` / `SetKramdownIAL` | off | 行内 IAL |
| `kramdown_ial_id_render_name` | `SetKramdownIALIDRenderName` | `id` | IAL 里 id 用的属性名 |
| `omit_table_cell_ial` | 内部 | off | 表格单元格不输出 IAL |
| `html2_markdown_attrs` | `SetHTML2MarkdownAttrs` | 空 | HTML→Md 要保留的属性名 |
| `keep_escaped` | 内部 | off | 导出保留反斜杠转义 |
| `ensure_list_item_paragraph` | `SetEnsureListItemParagraph` | off | 列表项强制成段落 |
| `arbitrary_task_list_item_marker` | `SetArbitraryTaskListItemMarker` | off | 任意字符作任务标记 |

## 5. 前端模式

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `vditor_wysiwyg` | `SetVditorWYSIWYG` | off | Vditor 所见即所得 |
| `vditor_ir` | `SetVditorIR` | off | Vditor 即时渲染 |
| `vditor_sv` | `SetVditorSV` | off | Vditor 分屏预览 |
| `protyle_wysiwyg` | `SetProtyleWYSIWYG` | off | SiYuan Protyle |
| `protyle_wysiwyg_auto_link` | 内部 | off | Protyle 下的自动链接 |
| `spin` | `SetSpin` | off | Spin 模式（Vditor 预览） |
| `protyle_contenteditable` | 内部 | on | 输出 `contenteditable` |
| `protyle_mark_net_img` | `SetProtyleMarkNetImg` | on | 网络图片加标记 |
| `spellcheck` | `SetSpellcheck` | off | 输出 `spellcheck` |
| `data_task` | `SetDataTask` | off | 输出 `data-task` |
| `gfm_task_list_item_class` | `SetGFMTaskListItemClass` | `vditor-task` | 任务项 class |
| `vditor_code_block_preview` | `SetVditorCodeBlockPreview` | on | 代码块预览包裹 |
| `vditor_math_block_preview` | `SetVditorMathBlockPreview` | on | 公式块预览包裹 |
| `vditor_html_block_preview` | `SetVditorHTMLBlockPreview` | on | HTML 块预览包裹 |

## 6. 渲染输出

| 选项 | JS | 默认 | 作用 |
| --- | --- | --- | --- |
| `auto_space` | `SetAutoSpace` | off | 中英文之间自动加空格 |
| `chinese_paragraph_beginning_space` | `SetChineseParagraphBeginningSpace` | off | 中文段首缩进 |
| `keep_paragraph_beginning_space` | `SetParagraphBeginningSpace` | off | 导出保留段首空格 |
| `fix_term_typo` | `SetFixTermTypo` | off | 术语自动修正（配合 `terms`） |
| `render_list_style` | `SetRenderListStyle` | off | 列表输出 `style="list-style:…"` |
| `soft_break2_hard_break` | `SetSoftBreak2HardBreak` | on | 软换行输出 `<br>` |
| `sanitize` | `SetSanitize` | off | 输出 HTML 消毒 |
| `heading_anchor` | `SetHeadingAnchor` | off | 标题输出锚点 |
| `image_lazy_loading` | `SetImageLazyLoading` | 空 | 图片 `loading="lazy"` |
| `link_base` | `SetLinkBase` | 空 | 相对链接补前缀 |
| `link_prefix` | `SetLinkPrefix` | 空 | 链接路径前缀 |
| `unordered_list_marker` | `SetUnorderedListMarker` | 空 | 无序列表符号 |
| `img_tag` | `SetImgTag` | off | 导出用 `<img>` 而非 `![]()` |
| `prevent_encode_link_space` | `SetPreventEncodeLinkSpace` | off | 链接空格不编码 |
| `node_index_start` | 内部 | 1 | Protyle 节点序号起点 |
| `export_normalize_task_list_marker` | `SetExportNormalizeTaskListMarker` | on | 导出任务标记归一化 |
| `tabs_markdown` | 内部 | off | 剪贴板保留页签结构 |
| `code_syntax_highlight` | `SetCodeSyntaxHighlight` | on | Chroma 高亮（Rust 版暂未接入） |
| `code_syntax_highlight_detect_lang` | `SetCodeSyntaxHighlightDetectLang` | off | 语言自动探测 |
| `code_syntax_highlight_inline_style` | `SetCodeSyntaxHighlightInlineStyle` | off | 内联样式 |
| `code_syntax_highlight_line_num` | `SetCodeSyntaxHighlightLineNum` | off | 行号 |
| `code_syntax_highlight_style_name` | `SetCodeSyntaxHighlightStyleName` | `github` | 高亮主题 |

## 7. 数据注入（不是布尔开关）

| 方法 | 作用 |
| --- | --- |
| `PutEmojis(map)` / `GetEmojis()` / `RemoveEmoji(alias)` | emoji 别名表，**合并**进内置表 |
| `SetEmojis(map)` | 一次性设置别名表 |
| `SetEmojiSite(url)` | emoji 图片地址前缀 |
| `PutTerms(map)` / `GetTerms()` | 术语表，配合 `fix_term_typo` |
| `AddAutoLinkDomainSuffix(s)` | 追加自动链接域名后缀（进程级） |
| `SetLinkBase` / `GetLinkBase` | 链接前缀 |

## 8. 没有 JS 入口的内部字段

`preserve_table_cell_source`、`inline_code_html`、`disable_table_cell_rich`、
`alias_emoji`、`emoji_alias`、`terms`、`protyle_wysiwyg_auto_link`、`keep_escaped`、
`tabs_markdown`、`omit_table_cell_ial`、`node_index_start`、`protyle_contenteditable`。

这些由具体 API 在内部设置（例如 `TextBundle` 会打开 `tabs_markdown`，
`Md2BlockDOMWithAutoLink` 会打开 `protyle_wysiwyg_auto_link`），不要在外面改。

## Vditor 实际用到的子集

Vditor 4.0.1 的 options → Lute 适配函数只调这 22 个：

    PutEmojis SetEmojiSite SetHeadingAnchor SetInlineMathAllowDigitAfterOpenMarker
    SetAutoSpace SetCallout SetInlineMath SetToC SetFootnotes SetFixTermTypo
    SetVditorCodeBlockPreview SetVditorMathBlockPreview SetSanitize
    SetChineseParagraphBeginningSpace SetRenderListStyle SetLinkBase SetLinkPrefix
    SetMark SetGFMAutoLink SetImageLazyLoading SetSup SetSub

加上切换模式时的 `SetVditorIR` / `SetVditorWYSIWYG` / `SetVditorSV`，以及
`SetJSRenderers`。其余 50 多个选项在 Vditor 场景下始终是默认值——
这正是编译期裁剪能省下空间的前提：没被设置的选项，其数据表和分支都可以不进包。

## 与裁剪的关系

一个选项关掉 = 那一块不处理，但**编译期能不能省字节**取决于两件事：

1. 它的检查点背后有没有独立的函数边界；
2. 这些检查点是否被同一个编译期常量短路。

把开关做成 `const`（`pub const CAP_X: bool = cfg!(feature = "x")`），
读点写成 `if CAP_X && options.x`，rustc 在 MIR 阶段就把分支体删掉，
分支体里唯一的调用者随之消失，链接器再回收那些函数。实测约 **200–350 字节/读点**：

| 选项 | 读点数 | raw 减少 |
| --- | --- | --- |
| `protyle_wysiwyg` | 45 | −11.3 KB |
| `kramdown_block_ial` | 25 | −8.6 KB |
| `auto_space` | 27 | −4.5 KB |

所以「不能按选项裁」的说法只对 `#[cfg]` 成立（那种写法只删 if，不删分支体里
独有的调用者）。用常量折叠就都裁得掉，代价是那个能力在产物里彻底不存在了。

完整实测、构建预设与裁剪约定见 `docs/feature-trim.md`。
