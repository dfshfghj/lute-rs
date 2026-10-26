// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
//
// 自定义渲染器（SetJSRenderers）专项测试。
//
//   node javascript/conformance/renderers.mjs

import assert from 'node:assert/strict';
import { Lute } from '../lute.js';

await Lute.ready;

const GROUPS = [
  'HTML2Md',
  'HTML2VditorDOM',
  'HTML2VditorIRDOM',
  'HTML2BlockDOM',
  'HTML2VditorSVDOM',
  'Md2HTML',
  'Md2VditorDOM',
  'Md2VditorIRDOM',
  'Md2BlockDOM',
  'Md2VditorSVDOM',
];

// 与各分组配套的入口，用来验证回调确实被调用。
const ENTRIES = {
  HTML2Md: (l) => l.HTML2Md('<p>x</p>'),
  HTML2VditorDOM: (l) => l.HTML2VditorDOM('<p>x</p>'),
  HTML2VditorIRDOM: (l) => l.HTML2VditorIRDOM('<p>x</p>'),
  HTML2BlockDOM: (l) => l.HTML2BlockDOM('<p>x</p>'),
  HTML2VditorSVDOM: (l) => l.HTML2VditorSVDOM('<p>x</p>'),
  Md2HTML: (l) => l.MarkdownStr('', 'x'),
  Md2VditorDOM: (l) => l.Md2VditorDOM('x'),
  Md2VditorIRDOM: (l) => l.Md2VditorIRDOM('x'),
  Md2BlockDOM: (l) => l.Md2BlockDOM('x', false),
  Md2VditorSVDOM: (l) => l.Md2VditorSVDOM('x'),
};

// Go 版在这些分组里接了自定义渲染器（Md2HTML 由 Lute.Markdown 装载）。
const WIRED = new Set([
  'HTML2Md',
  'HTML2VditorDOM',
  'HTML2VditorIRDOM',
  'HTML2BlockDOM',
  'HTML2VditorSVDOM',
  'Md2HTML',
  'Md2VditorDOM',
  'Md2VditorIRDOM',
  'Md2BlockDOM',
  'Md2VditorSVDOM',
]);

let passed = 0;
function check(name, fn) {
  fn();
  passed++;
  console.log('ok   ' + name);
}

check('10 个分组都能注册', () => {
  for (const group of GROUPS) {
    const engine = Lute.New();
    engine.SetJSRenderers({
      renderers: { [group]: { renderText: () => ['', Lute.WalkContinue] } },
    });
  }
});

check('未知分组报错', () => {
  const engine = Lute.New();
  assert.throws(() =>
    engine.SetJSRenderers({ renderers: { Nope: { renderText: () => ['', 2] } } })
  );
});

for (const group of GROUPS) {
  check(group + ' 回调可读取节点', () => {
    let seen = null;
    const engine = Lute.New();
    engine.SetJSRenderers({
      renderers: {
        [group]: {
          renderText(node, entering) {
            if (entering && !seen) {
              seen = {
                text: node.Text(),
                tokens: node.TokensStr(),
                type: node.__internal_object__.Type,
                len: node.TextLen(),
              };
            }
            return ['', Lute.WalkContinue];
          },
        },
      },
    });
    const out = ENTRIES[group](engine);
    if (!WIRED.has(group)) {
      assert.ok(!seen, group + ' 不应触发回调');
      return;
    }
    assert.ok(seen, group + ' 未触发回调');
    assert.equal(seen.text, 'x');
    assert.equal(seen.tokens, 'x');
    assert.equal(seen.type, 'NodeText');
    assert.equal(seen.len, 1);
    assert.ok(!out.includes('>x<'));
  });
}

check('WalkContinue 输出自定义文本', () => {
  const engine = Lute.New();
  engine.SetJSRenderers({
    renderers: {
      Md2BlockDOM: {
        renderText: (node, entering) => (entering ? ['<CUSTOM>', Lute.WalkContinue] : ['', Lute.WalkContinue]),
      },
    },
  });
  const out = engine.Md2BlockDOM('x', false);
  assert.ok(out.includes('<CUSTOM>'));
  assert.ok(!out.includes('>x<'));
});

check('WalkSkipChildren 跳过子节点', () => {
  const engine = Lute.New();
  engine.SetJSRenderers({
    renderers: {
      Md2BlockDOM: {
        renderParagraph: (node, entering) =>
          entering ? ['<P>', Lute.WalkSkipChildren] : ['</P>', Lute.WalkContinue],
      },
    },
  });
  const out = engine.Md2BlockDOM('x', false);
  assert.ok(out.includes('<P>'));
  assert.ok(!out.includes('>x<'));
});

check('WalkStop 终止遍历', () => {
  const engine = Lute.New();
  let calls = 0;
  engine.SetJSRenderers({
    renderers: {
      Md2BlockDOM: {
        renderText: () => {
          calls++;
          return ['<STOP>', Lute.WalkStop];
        },
        renderParagraph: () => {
          calls++;
          return ['', Lute.WalkContinue];
        },
      },
    },
  });
  const out = engine.Md2BlockDOM('a\n\nb', false);
  assert.ok(out.includes('<STOP>'));
  assert.equal(calls, 2); // paragraph 进入、第一个 text；WalkStop 后不再遍历
});

check('回调里可以修改节点', () => {
  const engine = Lute.New();
  engine.SetJSRenderers({
    renderers: {
      Md2BlockDOM: {
        renderText(node, entering) {
          if (entering) {
            const child = node.__internal_object__.Parent;
            assert.ok(child);
            node.SetIALAttr('data-x', '1');
            assert.equal(node.IALAttr('data-x'), '1');
            assert.equal(node.IsBlock(), false);
            assert.equal(node.ParentIs('NodeParagraph'), true);
          }
          return [entering ? node.Text() : '', Lute.WalkContinue];
        },
      },
    },
  });
  const out = engine.Md2BlockDOM('x', false);
  assert.ok(out.includes('>x<'));
});

for (const group of WIRED) {
  check(group + ' 回调异常保留对象身份、节点及后续调用能力', () => {
    const engine = Lute.New();
    const failure = new Error('boom');
    let retained;
    engine.SetJSRenderers({ renderers: { [group]: {
      renderText: (node, entering) => {
        if (entering) {
          retained = node;
          throw failure;
        }
        return ['', Lute.WalkContinue];
      },
    } } });
    assert.throws(() => ENTRIES[group](engine), (error) => error === failure);
    assert.equal(retained.TokensStr(), 'x');
    assert.equal(retained.ParentIs('NodeParagraph'), true);

    // Go 的注册按节点覆盖；替换抛异常的函数后复用同一个引擎。
    engine.SetJSRenderers({ renderers: { [group]: {
      renderText: (node, entering) => [entering ? 'after-error' : '', Lute.WalkContinue],
    } } });
    assert.ok(ENTRIES[group](engine).includes('after-error'));
    assert.equal(retained.TokensStr(), 'x');
    retained.free();
  });
}

check('树、节点及 inline 入口的回调异常释放借用', () => {
  const engine = Lute.New();
  const tree = engine.HTML2Tree('<p>retained</p>');
  const root = tree.Root();
  const failure = { message: 'render failed' };
  engine.SetJSRenderers({ renderers: { Md2BlockDOM: {
    renderText: () => { throw failure; },
  } } });
  for (const render of [
    () => engine.Tree2BlockDOM(tree),
    () => engine.RenderNodeBlockDOM(root),
    () => engine.Md2BlockDOMTree('x', false),
    () => engine.InlineMd2BlockDOM('x'),
  ]) {
    assert.throws(render, (error) => error === failure);
    assert.equal(root.Text(), 'retained');
  }
  engine.SetJSRenderers({ renderers: { Md2BlockDOM: {
    renderText: (node, entering) => [entering ? node.TokensStr() : '', Lute.WalkContinue],
  } } });
  assert.match(engine.Tree2BlockDOM(tree), />retained</);
  root.free();
  tree.free();
});

check('嵌套渲染捕获回调异常后外层渲染继续', () => {
  const engine = Lute.New();
  const failure = { message: 'nested failure' };
  let nested = false;
  engine.SetJSRenderers({ renderers: { HTML2Md: {
    renderText: (_node, entering) => {
      if (!entering) return ['', Lute.WalkContinue];
      if (nested) throw failure;
      nested = true;
      try {
        assert.throws(() => engine.HTML2Md('<p>nested</p>'), (error) => error === failure);
      } finally {
        nested = false;
      }
      return ['outer', Lute.WalkContinue];
    },
  } } });
  assert.equal(engine.HTML2Md('<p>x</p>'), 'outer\n');
  assert.equal(engine.HTML2Md('<p>x</p>'), 'outer\n');
});

check('已知 Protyle 错误可捕获且不是 WASM trap', () => {
  const engine = Lute.New();
  for (const method of ['Md2BlockDOM', 'Md2BlockDOMWithAutoLink', 'Md2BlockDOMTree']) {
    assert.throws(() => engine[method]('[ref]: /target\n\n[ref]', false), (error) => {
      assert.ok(error instanceof Error);
      assert.ok(!(error instanceof WebAssembly.RuntimeError));
      assert.match(error.message, /nil pointer dereference/);
      return true;
    });
    assert.match(engine.Md2BlockDOM('next', false), />next</);
  }
});

check('空 renderers 不报错', () => {
  const engine = Lute.New();
  engine.SetJSRenderers({ renderers: {} });
  assert.equal(engine.MarkdownStr('', '# x'), '<h1>x</h1>\n');
});

console.log('\nPASS ' + passed + ' renderer cases');
