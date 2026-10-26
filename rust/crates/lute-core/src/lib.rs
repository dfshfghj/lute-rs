// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

#![deny(private_interfaces, private_bounds)]

//! Lute 结构化 Markdown 引擎的 Rust 核心。
//!
//! 结构与 Go 版保持一一对应，目标是通过 conformance 用例实现逐字节等价。
//! 设计决策见 docs/adr-0001-node-model.md。

pub mod ast;
pub mod base64;
pub mod blockdom;
pub(crate) mod char_class;
pub mod dom;
pub mod editor;
pub mod emoji;
pub mod html;
pub mod html2md;
pub(crate) mod html_entity;
pub mod lex;
pub mod lute;
pub mod options;
pub mod parse;
pub mod render;
pub mod structure;
pub mod terms;
pub mod vditor;

/// 与 Go 版 lute.Version 保持一致。
pub const VERSION: &str = "1.7.6";

/// Protyle 前端能力的编译期开关。
///
/// 关掉后 rustc 在 MIR 常量传播阶段就把 `if PROTYLE { ... }` 整块删掉，
/// 那些分支体里唯一的调用者随之消失，链接器再把对应函数整块回收。
/// 与 `#[cfg]` 的区别：分支体仍然参与类型检查（不会腐烂），
/// 也不需要写 `#[cfg(not(..))] let _ = x;` 这类补丁。
pub const PROTYLE: bool = cfg!(feature = "protyle");

/// Kramdown 块级 IAL 能力开关。
pub const CAP_KRAMDOWN_BLOCK_IAL: bool = cfg!(feature = "kramdown-ial");

/// 中英文自动空格能力开关。
pub const CAP_AUTO_SPACE: bool = cfg!(feature = "auto-space");
