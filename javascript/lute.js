// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

// WASM 版 Lute 的 JS 兼容层。
//
// 目标：
//   1. 初始化后挂载 globalThis.Lute（与 GopherJS 产物一致）；
//   2. 同时提供 ESM 导出；
//   3. Lute.New() 返回对象的成员与 GopherJS 完全一致；
//   4. 初始化异步，但 API 本身保持同步，用 await Lute.ready 等待就绪。

import init, * as wasm from './pkg/lute_wasm.js';

const isNode =
  typeof process !== 'undefined' && process.versions && process.versions.node;

async function initWasm() {
  if (isNode) {
    const { readFile } = await import('node:fs/promises');
    const { fileURLToPath } = await import('node:url');
    const bytes = await readFile(
      fileURLToPath(new URL('./pkg/lute_wasm_bg.wasm', import.meta.url))
    );
    return init({ module_or_path: bytes });
  }
  return init();
}

const Lute = {
  Version: '',
  // GopherJS 版没有 ready，这里新增，业务侧 await 后可安全调用同步 API。
  ready: null,
  WalkStop: 0,
  WalkSkipChildren: 1,
  WalkContinue: 2,
  Caret: '\u2038',
  New(options) {
    const engine = new wasm.LuteEngine();
    if (options) {
      engine.SetJSRenderers(options);
    }
    return engine;
  },
  NewNodeID: wasm.NewNodeID,
  GetHeadingID: wasm.GetHeadingID,
  GetHeadingIDRaw: wasm.GetHeadingIDRaw,
  EscapeHTMLStr: wasm.EscapeHTMLStr,
  UnEscapeHTMLStr: wasm.UnEscapeHTMLStr,
  Sanitize: wasm.Sanitize,
  EChartsMindmapStr: wasm.EChartsMindmapStr,
  BlockDOM2Content: wasm.BlockDOM2ContentGlobal,
};

Lute.Constants = {
  Caret: Lute.Caret,
  WalkStop: Lute.WalkStop,
  WalkSkipChildren: Lute.WalkSkipChildren,
  WalkContinue: Lute.WalkContinue,
};

globalThis.Lute = Lute;

await initWasm();
Lute.Version = wasm.luteVersion();
Lute.ready = Promise.resolve(Lute);

export { Lute };
export default Lute;
