// Vditor 演示页：Lute 换成 Rust/WASM 构建。
//
// 上游 vditor 包保持原样，唯一的改动是 _lutePath 指向 vendor/lute.min.js
// （由 ../javascript/build-wasm.sh 产出的单文件经典脚本，wasm 已内嵌）。
// Vditor 用动态 <script> 加载它，onload 之后才调用 Lute.New()，
// 所以同步初始化的 bundle 不需要任何额外等待逻辑。

const LUTE_PATH = 'vendor/lute.min.js';

const SAMPLE = `# Lute × Vditor

**Rust/WASM** 版 Lute 已经接管了 Vditor 的解析与渲染。左侧编辑器里的每次输入，
都会经过 \`Md2VditorIRDOM\` / \`VditorIRDOM2Md\` 往返。

## 列表

- [x] 段落、强调、\`行内代码\`
- [ ] 表格与代码块
- 引用：

> 结构化的 Markdown 引擎，渲染与解析共用一棵 AST。

## 表格

| 特性 | Go 版 | Rust 版 |
| --- | --- | --- |
| 富文本单元格 | 支持 | 支持 |
| Chroma 代码高亮 | 支持 | 暂不接入 |

## 代码

\`\`\`rust
pub fn main() {
    println!("hello from wasm");
}
\`\`\`

## 数学

行内 $a^2 + b^2 = c^2$，块级：

$$
\\int_0^1 x^2 dx = \\frac{1}{3}
$$
`;

function fatal(message, detail) {
  const box = document.getElementById('fatal');
  box.style.display = 'block';
  box.innerHTML = '<strong>' + message + '</strong><p>' + detail + '</p>';
  document.getElementById('vditor').style.display = 'none';
}

async function preflight() {
  const res = await fetch(LUTE_PATH, { method: 'HEAD' });
  if (!res.ok) {
    throw new Error(LUTE_PATH + ' 不可访问（HTTP ' + res.status + '）');
  }
}

function engineLabel() {
  // lute.min.js（WASM 版）带 ready；GopherJS 版没有。
  return window.Lute && window.Lute.ready ? 'Rust / WASM' : 'GopherJS（不是本 demo 的目标）';
}

function setBadges() {
  const Lute = window.Lute;
  document.getElementById('engine').textContent = engineLabel();
  document.getElementById('version').textContent = 'Lute v' + (Lute ? Lute.Version : '?');
}

function renderSelfCheck(lines) {
  document.getElementById('selfcheck').innerHTML = lines
    .map((l) => (l.ok === undefined ? l.text : '<span class="' + (l.ok ? 'ok' : 'bad') + '">' + l.text + '</span>'))
    .join('\n');
}

function selfCheck(vditor) {
  const engine = vditor.vditor.lute;
  const md = vditor.getValue();
  const lines = [];
  const run = (label, fn) => {
    const t0 = performance.now();
    try {
      const out = fn();
      const ms = (performance.now() - t0).toFixed(2);
      lines.push({ text: '✓ ' + label + '  ' + ms + ' ms  ' + (typeof out === 'string' ? out.length + ' 字符' : ''), ok: true });
      return out;
    } catch (e) {
      lines.push({ text: '✗ ' + label + '  ' + (e && e.message ? e.message : e), ok: false });
      return null;
    }
  };

  lines.push({ text: 'Lute.Version = ' + window.Lute.Version });
  const ir = run('Md2VditorIRDOM', () => engine.Md2VditorIRDOM(md));
  const backIr = run('VditorIRDOM2Md', () => engine.VditorIRDOM2Md(ir || ''));
  if (backIr !== null) {
    const same = backIr.trim() === md.trim();
    lines.push({ text: (same ? '✓ ' : '! ') + 'IR 往返与原文一致' + (same ? '' : '（存在归一化差异，属预期）'), ok: same ? true : undefined });
  }
  const block = run('Md2BlockDOM', () => engine.Md2BlockDOM(md));
  run('BlockDOM2StdMd', () => engine.BlockDOM2StdMd(block || ''));
  const dom = run('Md2VditorDOM', () => engine.Md2VditorDOM(md));
  run('VditorDOM2Md', () => engine.VditorDOM2Md(dom || ''));
  run('Md2VditorSVDOM', () => engine.Md2VditorSVDOM(md));
  run('Md2HTML', () => engine.Md2HTML(md));
  renderSelfCheck(lines);
}

async function boot() {
  try {
    await preflight();
  } catch (e) {
    fatal('加载 Lute 失败', e.message + '<br>先执行 <code>npm install</code> 与 <code>npm run build:lute</code>。');
    return;
  }

  const vditor = new Vditor('vditor', {
    cdn: 'vendor/vditor',
    _lutePath: LUTE_PATH,
    mode: 'ir',
    height: 560,
    lang: 'zh_CN',
    value: SAMPLE,
    cache: { enable: false },
    toolbar: [
      'headings', 'bold', 'italic', 'strike', '|',
      'list', 'ordered-list', 'check', 'outdent', 'indent', '|',
      'quote', 'line', 'code', 'inline-code', '|',
      'link', 'table', '|',
      'undo', 'redo', '|',
      'edit-mode', 'both', 'preview', 'outline', 'export', 'devtools', 'info'
    ],
    preview: {
      markdown: {
        toc: true,
        mark: true,
        footnotes: true,
        codeBlockPreview: true,
        mathBlockPreview: true
      },
      hljs: { lineNumber: true },
      math: { engine: 'KaTeX' }
    },
    after: () => {
      setBadges();
      selfCheck(vditor);
    }
  });

  // 页面里方便直接在控制台折腾：__lute.Md2HTML('**x**')、__vditor.getCurrentMode()
  Object.defineProperty(window, '__lute', { get: () => vditor.vditor.lute });
  window.__vditor = vditor;
}

boot();
