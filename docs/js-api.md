# Lute JavaScript 接口规范

> 适用版本：Lute 1.7.6
> 现状实现：Rust + WebAssembly（`javascript/lute.min.js`、`javascript/lute.js`）
> 对齐基准：上游 Go 实现（输出快照见 `conformance/golden/`）
>
> 本文是**重写前后必须保持一致**的对外契约。任何签名、返回值形状、默认值的变更都必须
> 走版本升级，并同步更新 `conformance/` 用例与快照。

---

## 1. 加载方式

### 1.1 单文件经典脚本（默认）

```js
require("./lute.min.js")   // Node.js：挂载到 globalThis.Lute，同时 module.exports
// 浏览器：<script src="lute.min.js"></script>，同样得到全局 Lute
```

`lute.min.js` 由 `javascript/build-wasm.sh` 生成：wasm-pack 的 no-modules 产物
加上内嵌的 wasm 字节，初始化通过 `initSync` 同步完成，因此 `Lute.New()` 与
GopherJS 一样是同步调用，浏览器和 Node 均可直接使用。

### 1.2 ESM / 异步初始化

```js
import { Lute } from "./lute.js";
await Lute.ready;              // 可选：模块顶层已 await，导入完成即就绪
const lute = Lute.New();
lute.MarkdownStr("", "**hi**");   // 同步返回
```

`lute.js` 加载 `pkg/lute_wasm.js`（wasm-pack `--target web`），Node 下直接读取
wasm 文件，浏览器下走 `fetch`。两种产物共享同一套导出：

1. 初始化后挂载 `globalThis.Lute`（兼容层），ESM 另外提供 `export { Lute }`；
2. `Lute.New()` 返回的对象在方法名、参数顺序、返回值形状上与 GopherJS 完全一致；
3. 新增 `Lute.ready` Promise，但**同步 API 本身不会被 Promise 化**。

### 1.3 对齐基准

上游 Go 实现的输出冻结在 `conformance/golden/` 与 `conformance/fixtures/` 里，
Rust 集成测试直接读这些快照做逐字节比对。GopherJS 产物本身不在本仓库。

---

## 2. 类型映射

| Go 类型 | GopherJS 现状 | WASM 现状（已实现） | 说明 |
| --- | --- | --- | --- |
| `string` | string | string | 一致 |
| `bool` | boolean | boolean | 一致 |
| `int` | number | number | 一致 |
| `[]string` | Array\<string\> | Array\<string\> | 一致 |
| `map[string]string` | 普通对象 | 普通对象 | 键值均为 string |
| `[]byte`（入参） | **Uint8Array**（传 string 抛 `Cannot assign to read only property`） | Uint8Array 或 string 均可 | wasm-bindgen 按类数组处理，string 是额外能力，不破坏兼容 |
| `[]byte`（返回值） | **Uint8Array** | Uint8Array | 两版实测一致 |
| `(T, error)` | `[T, null]` / 抛异常 | 同左 | nil error 为 `null` |
| `(T, []string)` | `[T, null]` 或 `[T, [...]]` | 同左 | nil slice 为 `null` |
| `*ast.Node` | 包装对象 | 包装对象 | 见 §6 |
| `*parse.Tree` | 包装对象 | 包装对象 | 不建议在 JS 侧持有 |

> **重要**：由于 `[]byte` 入参在 GopherJS 下不接受 string，`Markdown` / `Format` / `TextBundle`
> 这三个字节版 API 在 GopherJS 下实际上不可用，JS 侧应始终使用 `MarkdownStr` / `FormatStr` / `TextBundleStr`。
> WASM 版保留字节版（`Uint8Array`），并额外接受 string（wasm-bindgen 的类数组转换），
> 属于向后兼容的增强；新代码仍推荐 `*Str` 变体。

---

## 3. 全局对象 `Lute`

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `Version` | string | 当前为 `"1.7.6"` |
| `New(options?)` | `() => Lute` | 创建引擎实例；`options` 仅用于自定义渲染器，见 §7 |
| `WalkStop` | 0 | 停止遍历 |
| `WalkSkipChildren` | 1 | 跳过子节点 |
| `WalkContinue` | 2 | 继续遍历 |
| `Caret` | `"‸"` | 编辑器插入符标记（U+2038） |
| `NewNodeID()` | string | `yyyyMMddHHmmss-xxxxxxx`，时间 + 随机，**不确定** |
| `GetHeadingID(node)` | string | 需传入节点包装对象，仅在渲染回调中有意义 |
| `GetHeadingIDRaw(node)` | string | 同上 |
| `EscapeHTMLStr(s)` | string | HTML 转义 |
| `UnEscapeHTMLStr(s)` | string | HTML 反转义（注意大小写：`UnEscape`） |
| `Sanitize(s)` | string | XSS 过滤 |
| `EChartsMindmapStr(s)` | string | 列表文本转 ECharts 思维导图 JSON |
| `BlockDOM2Content(dom)` | string | Protyle BlockDOM 转纯文本内容 |

`BlockDOM2Content` 内部固定开启的选项（重写时必须一致）：

```
ProtyleWYSIWYG, BlockRef, FileAnnotationRef, KramdownIAL, Tag, SuperBlock,
ImgPathAllowSpace, GitConflict, Mark, Sup, Sub, InlineMathAllowDigitAfterOpenMarker,
Sanitize = true
Footnotes, ToC, IndentCodeBlock, ParagraphBeginningSpace, AutoSpace, HeadingID,
Setext, YamlFrontMatter, LinkRef, CodeSyntaxHighlight = false
```

---

## 4. 实例方法

共 152 个导出方法，其中 76 个是 Setter。所有方法都是**同步**的。实例内部持有 `ParseOptions` 与 `RenderOptions`，Setter 修改后对
后续调用生效；不同实例互不影响（全局 Emoji 表除外，见 §8）。

### 4.1 Markdown 输入

| 方法 | 签名 | 前置选项 | 说明 |
| --- | --- | --- | --- |
| `MarkdownStr` | `(name, markdown) => html` | — | 最常用入口 |
| `Markdown` | `(name, Uint8Array) => Uint8Array` | — | 字节版，JS 不实用 |
| `Md2HTML` | `(markdown) => html` | — | 同 MarkdownStr，name 固定空 |
| `Md2VditorDOM` | `(markdown) => vhtml` | `SetVditorWYSIWYG(true)` | Vditor 所见即所得 DOM |
| `Md2VditorIRDOM` | `(markdown) => vhtml` | `SetVditorIR(true)` | Vditor 即时渲染 DOM |
| `Md2VditorSVDOM` | `(markdown) => vhtml` | `SetVditorSV(true)` | Vditor 分屏预览 DOM |
| `Md2BlockDOM` | `(markdown, reserveEmptyParagraph) => vhtml` | `SetProtyleWYSIWYG(true)` | Protyle 块 DOM |
| `Md2BlockDOMWithAutoLink` | `(markdown, reserveEmptyParagraph) => vhtml` | 同上 | 自动链接变体 |
| `Md2BlockDOMTree` | `(markdown, reserveEmptyParagraph) => [vhtml, tree]` | 同上 | 返回树对象 |
| `InlineMd2BlockDOM` | `(markdown) => vhtml` | 同上 | 行级片段转 BlockDOM |
| `FormatStr` | `(name, markdown) => markdown` | — | 格式化 Markdown |
| `TextBundleStr` | `(name, markdown, linkPrefixes) => [textbundle, originalLinks]` | — | 打包 TextBundle |
| `RenderJSON` | `(markdown) => json` | — | AST 的 JSON 表示 |
| `RenderEChartsJSON` | `(markdown) => json` | — | ECharts 思维导图数据 |
| `RenderKityMinderJSON` | `(markdown) => json` | — | KityMinder 数据 |
| `ProtylePreviewStr` | `(name, markdown) => html` | — | Protyle 导出预览 HTML |

### 4.2 HTML 输入

| 方法 | 签名 | 说明 |
| --- | --- | --- |
| `HTML2Md` | `(html) => markdown` | 标准 Markdown |
| `HTML2Markdown` | `(html) => [markdown, error]` | 失败时抛异常 |
| `HTML2Text` | `(dom) => text` | 去标签 |
| `HTML2VditorDOM` | `(html) => vhtml` | 需 `SetVditorWYSIWYG(true)` |
| `HTML2VditorIRDOM` | `(html) => vhtml` | 需 `SetVditorIR(true)` |
| `HTML2VditorSVDOM` | `(html) => vhtml` | 需 `SetVditorSV(true)` |
| `HTML2BlockDOM` | `(html) => vhtml` | 需 `SetProtyleWYSIWYG(true)` |
| `HTML2Tree` | `(html) => tree` | 返回语法树 |
| `HTMLNode2Tree` | `(node) => tree` | 传入 `html.Node` |

### 4.3 Vditor DOM 输入

| 方法 | 签名 | 前置选项 |
| --- | --- | --- |
| `VditorDOM2HTML` | `(vhtml) => html` | `SetVditorWYSIWYG(true)` |
| `VditorDOM2Md` | `(vhtml) => markdown` | `SetVditorWYSIWYG(true)` |
| `VditorIRDOM2HTML` | `(vhtml) => html` | `SetVditorIR(true)` |
| `VditorIRDOM2Md` | `(vhtml) => markdown` | `SetVditorIR(true)` |
| `SpinVditorDOM` | `(vhtml) => vhtml` | `SetVditorWYSIWYG(true)` |
| `SpinVditorIRDOM` | `(vhtml) => vhtml` | `SetVditorIR(true)` |
| `SpinVditorSVDOM` | `(markdown) => vhtml` | `SetVditorSV(true)` |

### 4.4 BlockDOM 输入（Protyle）

| 方法 | 签名 | 说明 |
| --- | --- | --- |
| `BlockDOM2Md` | `(blockdom) => markdown` | 保留 kramdown IAL |
| `BlockDOM2StdMd` | `(blockdom) => markdown` | 标准 Markdown，丢弃 IAL |
| `BlockDOM2HTML` | `(blockdom) => html` | |
| `BlockDOM2Text` | `(blockdom) => text` | 纯文本 |
| `BlockDOM2TextLen` | `(blockdom) => int` | 纯文本长度（字符数） |
| `BlockDOM2Content` | `(blockdom) => text` | 实例版：使用实例当前选项；全局 `Lute.BlockDOM2Content` 使用固定选项集（见第 3 节） |
| `BlockDOM2EscapeMarkerContent` | `(blockdom) => text` | 保留转义标记 |
| `BlockDOM2RichHTML` | `(blockdom) => html` | 富文本投影 |
| `BlockDOM2InlineBlockDOM` | `(blockdom) => vhtml` | 行级片段 |
| `BlockDOM2Tree` | `(blockdom) => tree` | 返回语法树 |
| `SpinBlockDOM` | `(blockdom) => blockdom` | 规范化：合并相邻同类型行级节点等 |
| `RenderNodeBlockDOM` | `(node) => blockdom` | 单节点渲染，需节点包装对象 |

### 4.5 结构变换（输入输出均为 BlockDOM）

| 方法 | 签名 | 说明 |
| --- | --- | --- |
| `OL2UL` / `UL2OL` | `(blockdom) => blockdom` | 有序 ↔ 无序 |
| `OL2TL` / `TL2OL` | `(blockdom) => blockdom` | 有序 ↔ 任务 |
| `UL2TL` / `TL2UL` | `(blockdom) => blockdom` | 无序 ↔ 任务（TL2UL 会移除任务标记） |
| `ConvertListType` | `(blockdom, target) => blockdom` | `target` ∈ `"o"` / `"u"` / `"t"` |
| `CancelList` | `(blockdom) => blockdom` | 拆掉一层列表 |
| `CancelListRecursively` | `(blockdom) => blockdom` | 递归拆列表 |
| `CancelBlockquote` | `(blockdom) => blockdom` | 去引用 |
| `CancelCallout` | `(blockdom) => blockdom` | 去提示块 |
| `CancelSuperBlock` | `(blockdom) => blockdom` | 拆超级块 |
| `Blockquote2Callout` | `(blockdom) => blockdom` | 引用转提示块 |
| `Callout2Blockquote` | `(blockdom) => blockdom` | 提示块转引用 |
| `Blocks2Hs` | `(blockdom, level) => blockdom` | 块转标题，`level` 为 `"1"`..`"6"` |
| `Blocks2Ps` | `(blockdom) => blockdom` | 块转段落 |

> 这些函数**不是**纯函数：输入不是预期结构时原样返回（例如 `OL2UL` 传入段落）。
> 它们会重新生成节点 ID 与 `updated` 时间戳。

### 4.6 工具

| 方法 | 签名 | 说明 |
| --- | --- | --- |
| `Space` | `(text) => text` | 中西文间插入空格 |
| `IsValidLinkDest` | `(s) => bool` | 是否为合法链接地址 |
| `GetLinkDest` | `(s) => s` | 提取链接地址，非法返回 `""` |
| `GetEmojis` | `() => {alias: unicode}` | 返回副本 |
| `PutEmojis` | `(map) => void` | 合并 |
| `SetEmojis` | `(map) => void` | **整体替换** |
| `RemoveEmoji` | `(s) => s` | 删除字符串中的 Emoji |
| `GetTerms` | `() => {term: replacement}` | 默认含 `github → GitHub` |
| `PutTerms` | `(map) => void` | 合并 |
| `SetTerms` | `(map) => void` | **整体替换** |
| `GetLinkBase` | `() => string` | |
| `Tree2HTML` | `(tree, options, parseOptions) => html` | 需传入选项对象，JS 侧基本不用 |
| `MergeSameSpan` | `(node) => void` | 仅渲染/自旋流程内部使用 |
| `MergeSameTextMark` | `(node) => void` | 同上 |

---

## 5. 选项 Setter

Setter 只写状态，不返回任何值。下表列出**默认值**，重写时默认值必须完全一致。

### 5.1 解析选项（ParseOptions）

| Setter | 默认 | 影响的语法 |
| --- | --- | --- |
| `SetGFMTable` | true | GFM 表格 |
| `SetGFMTaskListItem` | true | 任务列表 |
| `SetArbitraryTaskListItemMarker` | false | `[/]`、`[>]` 等任意任务标记 |
| `SetGFMStrikethrough` | true | `~~x~~` |
| `SetGFMStrikethrough1` | true | `~x~` 删除线 |
| `SetFullWidthStrikethrough` | false | `～～x～～` |
| `SetGFMAutoLink` | true | 裸 URL |
| `SetFootnotes` | true | 脚注 |
| `SetHeadingID` | true | 标题自定义 ID（同时写 render） |
| `SetToC` | false | `[toc]`（同时写 render） |
| `SetEmoji` | true | Emoji 别名替换 |
| `SetEmojis` | 内置 2126 条 | 替换别名表 |
| `SetEmojiSite` | `https://cdn.jsdelivr.net/npm/vditor/dist/images/emoji` | Emoji 图片前缀 |
| `SetInlineMath` | true | `$x$` |
| `SetInlineMathAllowDigitAfterOpenMarker` | false | `$1+1$` |
| `SetSetext` | true | Setext 标题 |
| `SetYamlFrontMatter` | true | YAML front matter |
| `SetBlockRef` | false | `((id "anchor"))` |
| `SetFileAnnotationRef` | false | `<<path/id "anchor">>` |
| `SetMark` | false | `==x==` |
| `SetKramdownIAL` | false | 块级 + 行级 IAL |
| `SetKramdownBlockIAL` | false | 块级 IAL |
| `SetKramdownSpanIAL` | false | 行级 IAL |
| `SetTag` | false | `#标签#` |
| `SetImgPathAllowSpace` | false | 图片路径含空格 |
| `SetSuperBlock` | false | `{{{row ... }}}`（同时写 render） |
| `SetCustomBlock` | false | `;;;plugin:xxx` |
| `SetSup` | false | `^x^` |
| `SetSub` | false | `~x~` 下标 |
| `SetInlineAsterisk` | true | `*x*` |
| `SetInlineUnderscore` | true | `_x_` |
| `SetGitConflict` | false | Git 冲突标记 |
| `SetLinkRef` | true | 链接引用定义 |
| `SetIndentCodeBlock` | true | 缩进代码块 |
| `SetParagraphBeginningSpace` | false | 段首空格（同时写 render.KeepParagraphBeginningSpace） |
| `SetDataImage` | true | `data:image/...` 图片 |
| `SetTextMark` | false | 通用行级节点 |
| `SetSpin` | false | 自旋内部标记，业务侧勿用 |
| `SetHTML2MarkdownAttrs` | `[]` | HTML→MD 保留的属性名 |
| `SetHTMLTag2TextMark` | false | `<u>` 等转文本标记 |
| `SetCallout` | false | 提示块 |
| `SetTabs` | false | 页签容器 |
| `SetEnsureListItemParagraph` | false | 空列表项补空段落 |

### 5.2 渲染选项（RenderOptions）

| Setter | 默认 | 影响 |
| --- | --- | --- |
| `SetSoftBreak2HardBreak` | true | 软换行转 `<br />` |
| `SetAutoSpace` | false | 中西文自动空格 |
| `SetFixTermTypo` | false | 术语修正 |
| `SetTerms` | 内置词典 | 替换术语表 |
| `SetRenderListStyle` | false | `data-style` 属性 |
| `SetCodeSyntaxHighlight` | true | 语法高亮（**JS 构建无效果**） |
| `SetCodeSyntaxHighlightDetectLang` | false | 自动识别语言 |
| `SetCodeSyntaxHighlightInlineStyle` | false | 内联样式 |
| `SetCodeSyntaxHighlightLineNum` | false | 行号 |
| `SetCodeSyntaxHighlightStyleName` | `"github"` | 主题名 |
| `SetHeadingAnchor` | false | 标题锚点 |
| `SetGFMTaskListItemClass` | `"vditor-task"` | 任务列表类名 |
| `SetDataTask` | false | `data-task` 属性 |
| `SetExportNormalizeTaskListMarker` | true | 非标准标记统一导出为 `[X]` |
| `SetLinkBase` | `""` | 相对链接前缀 |
| `SetLinkPrefix` | `""` | 强制链接前缀 |
| `SetImageLazyLoading` | `""` | 懒加载占位图 |
| `SetImgTag` | false | 用 `<img>` 渲染图片 |
| `SetPreventEncodeLinkSpace` | false | 链接空格不编码为 `%20` |
| `SetSanitize` | false | XSS 过滤（**不可作为安全边界**） |
| `SetChineseParagraphBeginningSpace` | false | 段首空两格（`class="indent--2"`） |
| `SetKramdownIALIDRenderName` | `"id"` | IAL 中 id 的渲染名 |
| `SetProtyleMarkNetImg` | true | 标记网络图片 |
| `SetSpellcheck` | false | `spellcheck` 属性 |
| `SetUnorderedListMarker` | `""` | 无序列表标记 |
| `SetVditorCodeBlockPreview` | true | 代码块预览区 |
| `SetVditorMathBlockPreview` | true | 公式块预览区 |
| `SetVditorHTMLBlockPreview` | true | HTML 块预览区 |

### 5.3 模式 Setter（同时写 parse 与 render）

| Setter | 默认 |
| --- | --- |
| `SetVditorWYSIWYG` | false |
| `SetVditorIR` | false |
| `SetVditorSV` | false |
| `SetProtyleWYSIWYG` | false |

> 模式之间**不是互斥的**：同时开启时行为未定义。业务侧必须只开一个。

---

## 6. 渲染回调中的节点对象

`SetJSRenderers` 注册的回调会收到 `*ast.Node` 的包装对象。

- **只能调用方法，不能直接读字段**：`node.Type` 是 `undefined`。
- 字段要通过 `node.__internal_object__` 访问，例如 `node.__internal_object__.Type`、
  `node.__internal_object__.parent`。
- 可用方法（节选）：`Text()`、`TokensStr()`、`TextLen()`、`Content()`、`ChildByType(t)`、
  `ChildrenByType(t)`、`FirstDeepestChild()`、`LastDeepestChild()`、`NextNodeText()`、
  `PreviousNodeText()`、`AppendChild(n)`、`InsertBefore(n)`、`InsertAfter(n)`、`Unlink()`、
  `SetIALAttr(k, v)`、`IALAttr(k)`、`IsBlock()`、`ParentIs(t)`。

WASM 版必须保留 `__internal_object__`（或提供等价的 `raw` 访问器），否则现有自定义渲染器会失效。

---

## 7. 自定义渲染器

```js
lute.SetJSRenderers({
  renderers: {
    Md2HTML: {
      renderText(node, entering) {
        return [node.Text(), Lute.WalkContinue];
      },
    },
  },
});
```

- 渲染器分组名只能是：
  `HTML2Md`、`HTML2VditorDOM`、`HTML2VditorIRDOM`、`HTML2BlockDOM`、`HTML2VditorSVDOM`、
  `Md2HTML`、`Md2VditorDOM`、`Md2VditorIRDOM`、`Md2BlockDOM`、`Md2VditorSVDOM`。
- 方法名规则：`render` + 节点类型去掉 `Node` 前缀，例如 `NodeText → renderText`、
  `NodeLinkDest → renderLinkDest`。
- 返回值必须是 `[string, number]`，第二个元素是 `WalkStop/WalkSkipChildren/WalkContinue`。
- **空对象会抛异常**：`{renderers: {}}` 在 GopherJS 下报 `Cannot read properties of undefined`。
  重写时应修复为无操作，但不得改变已有行为。
- 未注册的节点类型走默认渲染器。

---

## 8. 错误、panic 与线程模型

- Go 的 `error` 在多返回值中表现为 `null`；非 nil 时 GopherJS 会抛出 JS 异常。
- Go 的 `panic` 在 JS 中表现为普通 `Error`（`runtime error: ...`）。
  JS 构建的 `util.RecoverPanic` 是空实现，所以 panic 不会被吞掉。
- 单线程模型：不要在回调里递归调用同一实例的转换方法。
- 全局状态：Emoji 表由 `parse.EmojiLock` 保护；`FormatNodeSync` /
  `ProtyleExportMdNodeSync` 使用包级单例 + 互斥锁。WASM 单实例下无并发问题，
  但多 Worker 各自加载模块时互不共享。

---

## 9. 确定性与测试归一化

以下内容**不稳定**，不能直接用于快照比对：

1. `ast.NewNodeID()` 生成的 `yyyyMMddHHmmss-xxxxxxx`；
2. 由节点 ID 派生的 `updated="yyyyMMddHHmmss"` / `created="..."`；
3. `data-node-index`（同一输入下稳定，可保留）。

`conformance/` 的归一化规则（Go 与 JS 必须一致）：

```
\b[0-9]{14}-[0-9a-z]{7}\b          → 20060102150405-1a2b3c4
(updated|created)="[0-9]{14}"     → $1="20060102150405"
```

---

## 10. 已知的 JS 构建差异

GopherJS 产物与 Go 原生构建并不完全等价，重写时需要明确取舍：

| 差异 | Go 原生 | GopherJS | 重写建议 |
| --- | --- | --- | --- |
| 代码块语法高亮 | chroma 完整实现 | 无高亮 | 保持无高亮（与 `golden/js.json` 对齐），高亮留给前端 |
| 链接引用 Unicode 全折叠 | `x/text/cases` | 仅 `bytes.EqualFold` | **已实现**：native 用完整折叠表（`case_fold.rs`），wasm 保持 `EqualFold` 以与 JS 构建一致 |
| `RecoverPanic` | 捕获 panic 转 error | 空实现，panic 抛出 | 与 JS 对齐 |
| `[]byte` 入参 | `[]byte` | Uint8Array | 已实现：同时接受 string（增强，GopherJS 下会抛错） |

---

## 11. 验收清单

1. ✅ `cargo test --workspace` 全绿（367 个测试函数），原表 2075/2075，native 与 javascript 两个 profile 都过；
2. ✅ `node javascript/conformance/renderers.mjs` → 30/30（自定义渲染器与全局函数）；
3. ✅ `node demo/test-api.mjs` → 10/10（按 Vditor 的调用顺序复刻 options → Lute）；
4. ✅ `__internal_object__` 兼容层可用；
5. ✅ 全局 `Lute` 挂载与 ESM 导出同时可用；
6. ✅ 初始化后首个 `MarkdownStr` 调用为同步返回（不返回 Promise）；
7. ✅ `demo/` 用未修改的上游 Vditor 包即可运行。

证据与快照见 `conformance/`。
