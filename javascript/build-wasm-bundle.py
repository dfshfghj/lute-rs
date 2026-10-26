#!/usr/bin/env python3
# Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
# Copyright (c) 2019-present, b3log.org
#
# 把 wasm-pack 的 no-modules 产物与 wasm 字节合成单文件经典脚本。

import base64
import pathlib
import sys

src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])

glue = (src / 'lute_wasm.js').read_text(encoding='utf-8')
wasm = (src / 'lute_wasm_bg.wasm').read_bytes()
b64 = base64.b64encode(wasm).decode('ascii')

shim = r'''
// ---------------------------------------------------------------------------
// 同步初始化：把内嵌的 wasm 直接编译成实例，保证 Lute.New() 仍然是同步调用。
(function (global) {
  var bin = global.atob('__LUTE_WASM_BASE64__');
  var bytes = new Uint8Array(bin.length);
  for (var i = 0; i < bin.length; i++) {
    bytes[i] = bin.charCodeAt(i);
  }
  wasm_bindgen.initSync({ module: bytes });

  var w = wasm_bindgen;
  var Lute = {
    Version: w.luteVersion(),
    ready: Promise.resolve(null),
    WalkStop: 0,
    WalkSkipChildren: 1,
    WalkContinue: 2,
    Caret: '\u2038',
    New: function (options) {
      var engine = new w.LuteEngine();
      if (options) {
        engine.SetJSRenderers(options);
      }
      return engine;
    },
    NewNodeID: w.NewNodeID,
    GetHeadingID: w.GetHeadingID,
    GetHeadingIDRaw: w.GetHeadingIDRaw,
    EscapeHTMLStr: w.EscapeHTMLStr,
    UnEscapeHTMLStr: w.UnEscapeHTMLStr,
    Sanitize: w.Sanitize,
    EChartsMindmapStr: w.EChartsMindmapStr,
    BlockDOM2Content: w.BlockDOM2ContentGlobal
  };
  Lute.ready = Promise.resolve(Lute);
  Lute.Constants = {
    Caret: Lute.Caret,
    WalkStop: Lute.WalkStop,
    WalkSkipChildren: Lute.WalkSkipChildren,
    WalkContinue: Lute.WalkContinue
  };

  global.Lute = Lute;
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = Lute;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this);
'''.replace('__LUTE_WASM_BASE64__', b64)

out.write_text(glue + '\n' + shim, encoding='utf-8')
print('wrote %s (%d bytes)' % (out, out.stat().st_size))
