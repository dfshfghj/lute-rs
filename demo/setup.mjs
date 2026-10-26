#!/usr/bin/env node
// 把 npm 装好的 vditor 与刚构建的 lute.min.js 复制到 demo/vendor。
// 复制而不是引用，是为了让 demo 目录可以整目录丢给任意静态服务器。

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const vendor = path.join(here, 'vendor');

function copyDir(from, to) {
  fs.rmSync(to, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.cpSync(from, to, { recursive: true });
}

const vditorPkg = path.join(here, 'node_modules', 'vditor');
if (!fs.existsSync(path.join(vditorPkg, 'dist'))) {
  console.error('缺少 vditor：先执行 npm install');
  process.exit(1);
}
copyDir(path.join(vditorPkg, 'dist'), path.join(vendor, 'vditor', 'dist'));
console.log('vendor/vditor/dist  <- vditor@' + JSON.parse(fs.readFileSync(path.join(vditorPkg, 'package.json'), 'utf8')).version);

const lute = path.join(here, '..', 'javascript', 'lute.min.js');
if (!fs.existsSync(lute)) {
  console.error('缺少 javascript/lute.min.js：先执行 npm run build:lute');
  process.exit(1);
}
fs.mkdirSync(vendor, { recursive: true });
fs.copyFileSync(lute, path.join(vendor, 'lute.min.js'));
console.log('vendor/lute.min.js  <- javascript/lute.min.js (' + fs.statSync(lute).size + ' 字节)');
