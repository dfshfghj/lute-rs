# Demo

用来实测 Rust 实现

直接用 Vditor，改加载参数：

```js
new Vditor('vditor', {
  cdn: 'vendor/vditor',
  _lutePath: 'vendor/lute.min.js' // Lute 换成 WASM 构建
});
```

控制台里可以直接用 `__lute.Md2HTML('**x**')` 之类的调用折腾当前实例。