# Rurix 入门教程

> **API 收敛期提示(RD-008)**:Rurix MVP 已完结,但公开面仍处收敛期,stable API 快照尚未冻结。本教程的语法与库名**可能随版本变化**;每个可独立编译的示例都挂在 CI 门(`ci/tutorial_smoke.py`)下,语言面一变即报警,故教程代码始终与当前工具链一致。

这是一份**从零写第一个 Rurix 程序**的渐进式教程。它面向已了解 Rust 与 GPU 编程基本概念、想上手 Rurix 的开发者。与 `rx doc` 生成的**参考文档站**(规范条款 / 错误码 / traceability 矩阵)不同,本教程是**叙述式学习路径**。

## 阅读前提

- 环境:Windows 11 + NVIDIA GPU、CUDA Toolkit、MSVC 2022(详见 [`00_install.md`](00_install.md))。
- 你能在仓库根跑通 `cargo build --workspace`。
- 想直接看真实可编译代码:见 [`../conformance/tutorial/`](../conformance/tutorial/)(本教程所有可独立编译的示例的唯一事实源)。

## 章节

| 章 | 主题 | 配套可编译示例 |
|---|---|---|
| [00 安装与工具链](00_install.md) | 构建 Rurix、`rx` 子命令速览 | — |
| [01 第一个程序](01_first_program.md) | host 程序、`rx check`/`run`、语言基础、函数着色 | `01_hello.rx` `02_host_basics.rx` `03_fn_colors.rx` |
| [02 第一个 kernel](02_first_kernel.md) | SAXPY kernel、Grid/View、launch 概念 | `04_first_kernel.rx` |
| [03 资源生命周期](03_resources.md) | affine Context/Stream/Event/Buffer、编译期拦截 | (参考 conformance + UC-02 demo) |

> 进阶章节(views/地址空间、shared memory/stdlib、函数着色细则、UC-02 三流水线、UC-03 SPH+软光栅走读)随后续 PR 补齐;深入参考见各章末尾链接到的 `spec/*.md`。

## 可编译示例 vs 参考片段

本教程的代码分两类,请注意区分:

- **可编译示例**(`conformance/tutorial/*.rx`):只用 `rx check` 能独立解析的语言面(host / `device fn` / `const fn` / kernel 定义),经 CI 门 `rx check` + `rx run` 端到端真跑。你照抄即可编译。
- **参考片段**:launch(`stream.launch`)、运行时资源(`Context`/`Stream`/`Buffer`)、stdlib 数学(`Vec3`/`Mat4`)等需要**包上下文与运行时**才能解析,教程以片段呈现并链接到既有 conformance 语料(已受解析门约束)与 `src/uc02-demo`、`src/uc03-demo` 真实演示。
