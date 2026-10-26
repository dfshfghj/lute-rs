# 编译期功能裁剪

Lute 的功能按 Cargo feature 在编译期整块裁掉。**默认全开**，此时行为与 Go 版逐字节对齐；
裁剪构建用 `--no-default-features` 加白名单。

仓库发布的 `javascript/pkg/` 与 `javascript/lute.min.js` 都是 `default` 预设（8 个能力全开）。
下面的 `vditor` / `minimal` 是宿主自己构建时的参考，不随仓库发布。

「能力」= 一个 Cargo feature，和「选项」（`ParseOptions` / `RenderOptions` 里的 85 个运行时字段，
见 [options.md](./options.md)）不是一回事：选项是运行时开关，能力是编译期整块去掉。

## 能力清单

| feature | 默认 | 类别 | 关掉之后失去什么 | 实测 raw |
| --- | --- | --- | --- | --- |
| `emoji` | **开** | output | 内置 emoji 别名表；`:smile:` 不再渲染（除非宿主 `PutEmojis`） | −78 KB |
| `autolink` | **开** | output | GFM 顶级域名表；裸域名不再自动链接 | −20 KB |
| `mindmap` | **开** | output | mindmap 代码块不再渲染成 ECharts 结构 | 未单独测 |
| `render-json` | **开** | api | `RenderJSON`（AST 调试输出） | −10 KB |
| `vditor-sv` | **开** | api | `Md2VditorSVDOM` / `SpinVditorSVDOM` / `HTML2VditorSVDOM` | −17 KB |
| `protyle` | **开** | api | 36 个 API（`BlockDOM*` / `Md2BlockDOM*` / `ProtylePreview` / 块与列表结构变换）+ Protyle 渲染器 | −144 KB |
| `kramdown-ial` | **开** | api | 块级 IAL 恒为 off；`SetKramdownBlockIAL` 抛错 | −8.6 KB |
| `auto-space` | **开** | api | 自动空格恒为 off；`SetAutoSpace` 抛错 | −4.5 KB |

类别只有两种含义：

- **api**：关掉后**所有输入的输出都不变**，只是对应 API 不存在、或对应选项恒为 `false`
  且 setter 抛错。这类能力关掉是安全的，全量构建上的测试通过即可。
- **output**：关掉后**某些输入的输出会变**。这三个（`emoji` / `autolink` / `mindmap`）
  不归任何选项管——`emoji` 的选项默认就是 `true`，没有 setter 参与，throw 拦不住。
  所以必须人工确认生产不依赖。

判据：关掉之后，是否有某个**默认值为 true** 的选项、或某条**默认路径**会改变行为？
会 → output 类；不会 → api 类。

## 怎么裁

    cd rust/crates/lute-wasm

    # 全量（默认，8 个能力全开）
    wasm-pack build --target web --out-dir ../../../javascript/pkg --out-name lute_wasm

    # 裁剪：关掉 protyle 与 kramdown-ial，其余保留
    wasm-pack build --target web --out-dir ../../../javascript/pkg --out-name lute_wasm \
      --no-default-features \
      --features emoji,autolink,mindmap,render-json,vditor-sv,auto-space

要同时更新浏览器单文件产物（`javascript/lute.min.js`），把 `--out-dir` 换成临时目录，
再跑 `python3 javascript/build-wasm-bundle.py <临时目录> javascript/lute.min.js`；
或者直接改 `javascript/build-wasm.sh` 里的 `--no-default-features --features` 一行。

## 三个预设

| 预设 | features | raw | gzip | 相对全量 |
| --- | --- | --- | --- | --- |
| `default` | 全部 8 个 | 1,014,217 | 399,624 | — |
| `vditor` | 去掉 `protyle`、`kramdown-ial` | 851,148 | 340,312 | −163,069（−16.1%） |
| `minimal` | 再去掉 `emoji`、`autolink` | 751,695 | 301,592 | −262,522（−25.9%） |

`vditor` 预设的渲染输出与全量**逐字节相同**（IR / WYSIWYG / SV 三种模式 + 两种回写），
差别只是 36 个 Protyle API 从 `function` 变成 `undefined`。
`minimal` 额外丢掉内置 emoji 与 TLD 表，宿主需要自己 `PutEmojis` / `AddAutoLinkDomainSuffix`。

## 裁剪后的约定

1. **能力被关掉时，对应 setter 抛错**，不静默失效：

       SetAutoSpace        → Error: lute: this build was compiled without the auto-space capability
       SetProtyleWYSIWYG   → Error: lute: this build was compiled without the protyle capability
       SetKramdownBlockIAL → Error: lute: this build was compiled without the kramdown-ial capability

   实现是 3 行的 `require_capability`（`lute-wasm/src/lib.rs`），返回
   `Err(js_sys::Error::new(..).into())`，所以 JS 侧拿到的是真正的 `Error`，`e.message` 可用。

2. **测试只在全量构建上跑。** 在裁剪构建上跑会红 70 条（367 → 297），
   那 70 条是「这条测试要的能力被砍了」，不是回归。理由见下文「测试策略」。

3. **不要为每个能力组合维护测试矩阵。** 预设只有 2–3 个，不是 2^N；
   能力之间没有交互，`default` 全绿 + 每个预设跑一次冒烟就够。

## 进阶：折叠单个选项

这一节是给宿主自己改源码用的，仓库不提供选项级预设。

如果你确定某个**选项**在生产里永远不会被设置，而且对体积要求很高，可以直接把它折叠掉。
代价是那个能力在产物里彻底不存在（setter 抛错），并且要改源码、重新编译。

收益参考（实测）：`setext` 只有 1 个读点，折掉省 1,092 字节；
`auto_space` 27 个读点，折掉省 4,501 字节。判断标准不是读点数，
而是「这条分支背后有没有一整条独占的路」。

### 五步

**1. 两个 Cargo.toml 各加一个 feature**

    # rust/crates/lute-core/Cargo.toml
    default = ["emoji", ..., "setext"]
    setext = []

    # rust/crates/lute-wasm/Cargo.toml
    default = ["emoji", ..., "setext"]
    setext = ["lute-core/setext"]

**2. 加一个常量**

    // rust/crates/lute-core/src/lib.rs
    pub const CAP_SETEXT: bool = cfg!(feature = "setext");

**3. 保护读点**

    # 先列出所有出现位置
    grep -rn 'options\.setext\b' rust/crates/lute-core/src

把**读点**改成 `if crate::CAP_SETEXT && options.setext` 的形式：

    - if p.ctx.indented || !p.options.setext {
    + if p.ctx.indented || !(crate::CAP_SETEXT && p.options.setext) {

**4. 让 setter 抛错**

    #[wasm_bindgen(js_name = SetSetext)]
    pub fn set_setext(&mut self, b: bool) -> Result<(), JsValue> {
        require_capability(lute_core::CAP_SETEXT, "setext")?;
        self.lute.parse_options.setext = b;
        Ok(())
    }

**5. 验证**

    cargo clippy --fix --allow-dirty --workspace --all-targets   # 清 unused_parens
    cargo fmt --all
    cargo test --workspace                    # 全量构建应仍是 367 通过
    wasm-pack build --target web --out-dir /tmp/x --out-name lute_wasm \
      --no-default-features --features emoji,autolink,mindmap,render-json,vditor-sv
    stat -c%s /tmp/x/lute_wasm_bg.wasm        # 与折叠前对比

### 五个坑

1. **只包读点，不包写点。** 引擎内部会改自己的选项（例如 `lute.parse_options.setext = false`
   在某个 API 里），包起来会编译不过。判据：出现 `options.x =` 或 `options.x:`（结构体初始化）就跳过。

2. **接收者要一起包。** 只替换 `options.setext` 会得到 `p.(CAP && options.setext)`。
   要匹配整条链：`p.options.setext` / `self.options.setext` / `self.base.options.setext` /
   `lute.parse_options.setext`。

3. **字段名前缀重叠。** `protyle_wysiwyg` 是 `protyle_wysiwyg_auto_link` 的前缀，
   替换必须带词边界，否则会把后者切坏（这个坑实际踩过，编译报
   expected one of `.` `;` `?` `else`, or an operator, found `_auto_link`）。

4. **clippy 会报 `unused_parens`。** `if (CAP && x)` 的外层括号是多余的，
   跑一次 `cargo clippy --fix` 就清掉了。

5. **数据表类不适用。** 这条只删分支。`emoji` / `autolink` 是删数据表，
   删了默认路径行为就变（`:smile:` 变字面量），不属于「折叠选项」。

折完之后记得把新能力加进上面的能力清单，别让它漂在清单外面。

---

# 设计说明

以下解释为什么这样设计，以及每一项的实测数据。

## 1. feature 定义

`lute-core` 与 `lute-wasm` 各有一组同名 feature，默认全开：

    default = ["emoji", "autolink", "mindmap", "render-json", "vditor-sv",
               "protyle", "kramdown-ial", "auto-space"]

## 2. 为什么用常量折叠，不用 #[cfg]

开关做成 `const`，读点写成 `if CAP_X && options.x`：

    // lute-core/src/lib.rs
    pub const PROTYLE: bool = cfg!(feature = "protyle");
    pub const CAP_AUTO_SPACE: bool = cfg!(feature = "auto-space");

    // 调用点
    if crate::PROTYLE {
        crate::blockdom::set_text_mark_node(..);
    }

rustc 在 MIR 阶段做常量传播后，`if false { .. }` 的整个分支体会被删掉（`SimplifyCfg`），
不依赖 LLVM 的乐观优化。分支体消失后，**分支体里唯一的调用者随之消失**，
链接器再把那些函数整块回收。

| | `#[cfg]` | `const` 折叠 |
| --- | --- | --- |
| 分支体是否参与类型检查 | 否，会腐烂 | **是** |
| 是否需要 not-cfg 的 let _ = x 补丁 | 需要 | 不需要 |
| 模块声明 / 结构体字段 / 方法是否需要逐个 gate | 需要 | **不需要** |
| 能否删掉共享函数里的选项分支 | 能，但要写两遍 | 能，一行 |

实测同一件事（关掉 Protyle）：`#[cfg]` 要改 46 处（41 方法 + 5 模块声明），
`const` 只改 2 处调用点，两者尺寸差 584 字节（0.07%）：

| 做法 | raw | 相对全量 |
| --- | --- | --- |
| `#[cfg]`：41 方法 + 5 模块 + 38 导出 | 869,864 | −144,163 |
| `const`：只改 2 处调用点 | 870,448 | −143,579 |

再加 45 处选项读点（`if crate::PROTYLE && self.options.protyle_wysiwyg`）后到 **859,100**，
比 `#[cfg]` 版本还多省 10,764 字节——因为 `#[cfg]` 版本根本没动那些分支。

全量构建尺寸与输出**完全不变**（`PROTYLE = true` 时 `true && x` 折叠回 `x`）。

## 3. 逐项实测

基线 1,014,027 字节（未加 throw 时的全量构建），release + wasm-opt。

| 能力 | 类别 | 读点 / 改动 | raw 减少 | gzip 减少 |
| --- | --- | --- | --- | --- |
| `emoji` | output | 表定义 + 表构造 2 处 | −78,196 | −28,845 |
| `autolink` | output | 表定义 + 取表 2 处 | −20,003 | −10,116 |
| `mindmap` | output | 模块 | 未单独测 | — |
| `render-json` | api | 1 方法 + 1 导出 | −9,971 | −5,312 |
| `vditor-sv` | api | 模块 + 3 方法 + 3 导出 | −17,470 | −5,700 |
| `protyle` | api | 5 模块 + 36 导出 + 47 读点 | −154,927 | −55,050 |
| `kramdown-ial` | api | 25 读点 | −8,582 | −3,488 |
| `auto-space` | api | 27 读点 | −4,501 | −1,355 |

### 3.1 数据表：emoji / autolink

这类是纯数据，表删掉即回收。关键取舍：**裁表，不裁分支**。

`PutEmojis` 是**合并**进内置表的（与 Go 版行为一致），所以表删掉后宿主自带的 emoji 照常工作；
如果把 `if self.options.emoji` 也一起裁掉，宿主传进来的 emoji 就永久失效——
第一版踩过，实测 `PutEmojis({smile:'😄'})` 之后 `:smile:` 仍是字面量。

三项数据类（emoji / autolink / RenderJSON）合计省 108 KB，其中数据段 99 KB、代码段 9.5 KB。

### 3.2 渲染器：SV / RenderJSON

有独立模块边界，关掉后没有别的调用者，链接器整块回收。
SV 渲染器 1498 行只值 17,470 字节，折算约 **12 字节/行**。

### 3.3 Protyle：整块摘掉

核对 `vditor@4.0.1/dist/index.min.js`：`Md2BlockDOM`、`BlockDOM2Md`、`BlockDOM2HTML`、
`BlockDOM2Text`、`ProtylePreview`、`SpinBlockDOM`、`HTML2BlockDOM`、`BlockDOM2StdMd`、
`BlockDOM2Content` **各出现 0 次**。Vditor 的 22 个 setter 里也没有一个碰 Protyle 选项。

关掉 `protyle` 后消失的 36 个 API（全量 155 → 裁剪 119）：

    BlockDOM2Content  BlockDOM2EscapeMarkerContent  BlockDOM2HTML  BlockDOM2InlineBlockDOM
    BlockDOM2Md  BlockDOM2RichHTML  BlockDOM2StdMd  BlockDOM2Text  BlockDOM2TextLen
    BlockDOM2Tree  Blockquote2Callout  Blocks2Hs  Blocks2Ps  Callout2Blockquote
    CancelBlockquote  CancelCallout  CancelList  CancelListRecursively  CancelSuperBlock
    ConvertListType  HTML2BlockDOM  InlineMd2BlockDOM  Md2BlockDOM  Md2BlockDOMTree
    Md2BlockDOMWithAutoLink  OL2TL  OL2UL  ProtylePreview  ProtylePreviewStr
    RenderNodeBlockDOM  SpinBlockDOM  TL2OL  TL2UL  Tree2BlockDOM  UL2OL  UL2TL

代码段 731,265 → 595,078（−136,187），数据段 273,525 → 263,949（−9,576）。
和 emoji/TLD 相反：**省下的是代码**，因为 blockdom / structure 本身就是几十个函数。

验证：IR / WYSIWYG / SV 三种渲染 + 两种回写，裁剪版与全量**逐字节相同**，
`demo/test-api.mjs` 跑出 9/10（唯一失败项是 `Md2BlockDOM is not a function`，正是被摘掉的 API）。

### 3.4 选项分支：kramdown-ial / auto-space

这两个是「选项」而不是「模块」，关掉后没有模块消失，只是 25 / 27 个读点被删：

| 选项 | 读点 | raw 减少 | 折算 |
| --- | --- | --- | --- |
| `kramdown_block_ial` | 25 | −8,582 | 343 字节/读点 |
| `auto_space` | 27 | −4,501 | 167 字节/读点 |

## 4. 折叠的边界

同一个「关掉不处理」，落到编译期有两种命运：

- **整块**：功能有模块边界，关掉后那些函数没有任何别的调用者，链接器整块回收。
- **一层**：选项检查散在共享函数里。折叠只删掉那几条指令，函数本身还在。

共享程度实测：

| 选项 | 检查点 | 分布函数 | 文件 |
| --- | --- | --- | --- |
| `protyle_wysiwyg` | 86 | 55 | 28 |
| `kramdown_block_ial` | 44 | 30 | 18 |
| `auto_space` | 34 | 24 | 10 |

**省不了「函数本身」**：如果某个函数同时服务 Vditor 和 Protyle，折叠只删掉 Protyle 那段指令，
函数还得留着。`render/vditor_ir` / `render/vditor` 合计约 120 KB 代码，是剩下最大的一块，
但 mode 判断散在 16 个函数里，裁它等于维护第二套解析路径。

顺带一个正面结论：**折叠在物理上不可能误删共享函数**。拆函数会，折叠不会。

## 5. 测试策略

测试只在全量构建上跑。理由分两类：

**api 类——全量通过即可，且可证明。** 全量 `CAP = true`，测试覆盖
`opt = false`（默认路径）与 `opt = true`（能力用例）。生产只用 `opt = false` 时：

    全量：  true  && false  = false
    裁剪：  false && false  = false   ← 求值相同

唯一前提是「生产不会把被关掉的选项设为 true」——throw 把这个前提变成机器保证。

**output 类——不够。** `emoji` / `gfm_auto_link` 默认就是 `true`，**没有 setter 参与**，
throw 拦不住。实测（一个 setter 都不调，全用默认值）：

| 输入 | 全量 | 裁剪（无 emoji 表） |
| --- | --- | --- |
| `:smile:` | `<p>😄</p>` | `<p>:smile:</p>` |
| `a :heart: b` | `<p>a ❤️ b</p>` | `<p>a :heart: b</p>` |

全量测试全绿，裁剪版静默改变默认行为。这不是「分支被折叠」，是「数据不存在」——
`if options.emoji` 仍然为真，只是查表查空。

## 6. 遗留

1. **`emoji` 表应当改成惰性构造。** `ParseOptions::new()` 每次都把约 1800 条别名建成
   两个 `HashMap`（约 3600 个 String）：全量 1–2 ms/实例（连续创建因 GC 可到 15 ms），
   没有表时 0.02–0.07 ms。即使不裁剪也该改，行为不变。
2. **`mindmap` 未单独测。** 它被 `render/html.rs` / `render/vditor.rs` /
   `render/vditor_ir.rs` 的 mindmap 代码块分支直接调用，关掉会改变输出，所以归 output 类。
3. **table-cell-rich + serde_json 约 −25 KB（未实测）。** 只有 SiYuan 用，
   但要从 8 处解析/渲染分支上摘。
4. **`lute.js` 的导出要按构建区分。** 关掉 `render-json` 后 `Lute.RenderJSON` 不存在。
5. **裁剪产物不再与 Go 默认行为逐字节一致**（内置 emoji / TLD 少了），
   原表 2075 条里相关用例会失败。对齐只在 `default` 构建上做。
