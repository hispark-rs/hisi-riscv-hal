# hisi-riscv-hal 架构

本仓库是 [ws63-rs](https://github.com/hispark-rs/ws63-rs) monorepo 的子模块。

`hisi-riscv-hal` 是 WS63 的手写安全驱动层（31 个外设），基于 `embedded-hal 1.0`，采用外设单例 + `'d` 生命周期、
sealed trait、`#![no_std]` 设计。

完整架构与评审（集中维护于主仓库）：
- 组件文档：<https://github.com/hispark-rs/ws63-rs/blob/main/docs/architecture/hisi-riscv-hal.md>
- 总体架构：<https://github.com/hispark-rs/ws63-rs/blob/main/docs/architecture/overview.md>
- 整改排期：<https://github.com/hispark-rs/ws63-rs/blob/main/ROADMAP.md>
