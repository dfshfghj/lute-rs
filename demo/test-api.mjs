#!/usr/bin/env node
// 不启动浏览器，按 Vditor 的调用顺序把 Lute 跑一遍。
// 覆盖：加载单文件 bundle → 选项 setter（Vditor 的 zt.X）→ IR/WYSIWYG/SV 三套渲染 →
// SetJSRenderers 回调桥接 → 全局函数。
//
//   node demo/test-api.mjs

import assert from 'node:assert/strict';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const Lute = require(path.join(here, 'vendor', 'lute.min.js'));

const results = [];
function step(name, fn) {
  try {
    fn();
    results.push(['ok', name]);
  } catch (e) {
    results.push(['FAIL', name + ' — ' + (e && e.message ? e.message : e)]);
  }
}

const MD = [
  '# 标题',
  '',
  '段落 **加粗** 与 `代码`，引用 [链接](https://b3log.org)。',
  '',
  '- [x] 任务',
  '- 项目',
  '',
  '| A | B |',
  '| --- | --- |',
  '| 1 | 2 |',
  '',
  '> 引用',
  '',
  '```rust',
  'fn main() {}',
  '```',
  ''
].join('\n');

// Vditor 4.0.1 的 options → Lute 适配函数（dist/index.min.js 模块 214）原样复刻。
function optionsToLute(e) {
  const t = Lute.New();
  t.PutEmojis(e.emojis);
  t.SetEmojiSite(e.emojiSite);
  t.SetHeadingAnchor(e.headingAnchor);
  t.SetInlineMathAllowDigitAfterOpenMarker(e.inlineMathDigit);
  t.SetAutoSpace(e.autoSpace);
  t.SetCallout(e.callout);
  t.SetInlineMath(e.inlineMath);
  t.SetToC(e.toc);
  t.SetFootnotes(e.footnotes);
  t.SetFixTermTypo(e.fixTermTypo);
  t.SetVditorCodeBlockPreview(e.codeBlockPreview);
  t.SetVditorMathBlockPreview(e.mathBlockPreview);
  t.SetSanitize(e.sanitize);
  t.SetChineseParagraphBeginningSpace(e.paragraphBeginningSpace);
  t.SetRenderListStyle(e.listStyle);
  t.SetLinkBase(e.linkBase);
  t.SetLinkPrefix(e.linkPrefix);
  t.SetMark(e.mark);
  t.SetGFMAutoLink(e.gfmAutoLink);
  if (e.lazyLoadImage) t.SetImageLazyLoading(e.lazyLoadImage);
  t.SetSup(e.sup);
  t.SetSub(e.sub);
  return t;
}

const vditorDefaults = {
  emojis: {},
  emojiSite: 'https://unpkg.com/vditor@4.0.1/dist/images/emoji',
  headingAnchor: false,
  inlineMathDigit: false,
  autoSpace: false,
  callout: false,
  inlineMath: true,
  toc: true,
  footnotes: true,
  fixTermTypo: false,
  codeBlockPreview: true,
  mathBlockPreview: true,
  sanitize: true,
  paragraphBeginningSpace: false,
  listStyle: false,
  linkBase: '',
  linkPrefix: '',
  mark: true,
  gfmAutoLink: true,
  lazyLoadImage: false,
  sup: false,
  sub: false
};

step('Lute.Version 非空', () => {
  assert.equal(typeof Lute.Version, 'string');
  assert.ok(Lute.Version.length > 0, 'version: ' + JSON.stringify(Lute.Version));
});

let lute;
step('Vditor 的选项 setter 全部存在且不抛错', () => {
  lute = optionsToLute(vditorDefaults);
});

step('IR 模式：Md2VditorIRDOM / VditorIRDOM2Md', () => {
  lute.SetVditorIR(true);
  lute.SetVditorWYSIWYG(false);
  lute.SetVditorSV(false);
  const dom = lute.Md2VditorIRDOM(MD);
  assert.ok(dom.includes('<h1'), dom.slice(0, 120));
  assert.ok(dom.includes('加粗'));
  const back = lute.VditorIRDOM2Md(dom);
  assert.ok(back.includes('# 标题'), back.slice(0, 120));
});

// 任务列表的 IR 往返由 Go 定死：大写 X、两个空格，且任务项与普通项分成两个列表。
step('IR 任务列表往返与 Go 逐字节一致', () => {
  const dom = lute.Md2VditorIRDOM('- [x] 任务\n- 项目\n');
  assert.equal(lute.VditorIRDOM2Md(dom), '- [X]  任务\n\n- 项目\n');
});

step('WYSIWYG 模式：Md2VditorDOM / VditorDOM2Md', () => {
  lute.SetVditorIR(false);
  lute.SetVditorWYSIWYG(true);
  const dom = lute.Md2VditorDOM(MD);
  assert.ok(dom.includes('加粗'));
  const back = lute.VditorDOM2Md(dom);
  assert.ok(back.includes('# 标题'));
});

step('SV 模式：Md2VditorSVDOM', () => {
  lute.SetVditorWYSIWYG(false);
  lute.SetVditorSV(true);
  const dom = lute.Md2VditorSVDOM(MD);
  assert.ok(dom.includes('加粗'));
  lute.SetVditorSV(false);
});

step('SetJSRenderers 覆盖 Md2HTML 的段落渲染', () => {
  const engine = Lute.New();
  engine.SetJSRenderers({
    renderers: {
      Md2HTML: {
        renderText: (node, entering) => (entering ? [node.Text() + ' via Lute', Lute.WalkContinue] : ['', Lute.WalkContinue])
      }
    }
  });
  // Go 的 Lute.Markdown 会把 Md2HTMLRendererFuncs 装进 HTML 渲染器。
  assert.equal(engine.Md2HTML('**x**'), '<p><strong>x via Lute</strong></p>\n');
  assert.equal(engine.MarkdownStr('', '**x**'), '<p><strong>x via Lute</strong></p>\n');
});

step('SpinVditorDOM 往返（Vditor 预览用）', () => {
  const engine = Lute.New();
  engine.SetVditorWYSIWYG(true);
  const spun = engine.SpinVditorDOM('<p>[ToC]</p>');
  assert.equal(typeof spun, 'string');
  assert.ok(spun.length > 0);
});

step('BlockDOM 链路', () => {
  const engine = Lute.New();
  engine.SetProtyleWYSIWYG(true);
  const block = engine.Md2BlockDOM(MD);
  const md = engine.BlockDOM2Md(block);
  assert.ok(md.includes('# 标题'), md.slice(0, 120));
});

step('全局函数 Sanitize / EscapeHTMLStr / NewNodeID', () => {
  assert.equal(Lute.EscapeHTMLStr('<a&b>'), '&lt;a&amp;b&gt;');
  assert.equal(Lute.Sanitize('<img src=x onerror=alert(1)>'), '<img src="x">');
  assert.equal(typeof Lute.NewNodeID(), 'string');
});

const width = Math.max(...results.map(([s]) => s.length));
for (const [status, name] of results) {
  console.log((status === 'ok' ? '✓' : '✗') + ' ' + status.padEnd(width) + '  ' + name);
}
const failed = results.filter(([s]) => s !== 'ok').length;
console.log('\n' + (results.length - failed) + '/' + results.length + ' 通过');
process.exit(failed ? 1 : 0);
