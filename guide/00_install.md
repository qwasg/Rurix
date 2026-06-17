# 00 · 安装与工具链

> API 收敛期(RD-008):命令与产物形态可能随版本变化。

## 环境前提

| 项 | 要求 |
|---|---|
| 操作系统 | Windows 11(原生 COFF/PE/PDB 工具链) |
| GPU | NVIDIA(开发对照机 RTX 4070 Ti) |
| 驱动/运行时 | CUDA Toolkit + CUDA Driver API |
| C++ 工具链 | MSVC 2022 |
| 构建宿主 | Rust 工具链(Rurix 自身用 Rust 构建,D-201) |

> 仅做 `rx check`(纯前端静态检查)不需要 GPU;`rx run`/`rx bench` 等执行 GPU 路径才需要真实设备。

## 构建工具链

在仓库根:

```sh
cargo build --workspace
```

产物里最常用的是 `rx`(工具链 CLI)。本教程示例用调试版 `rx`:

```sh
cargo run -p rx -- <子命令> ...
# 或直接用产物 target/debug/rx(Windows 为 rx.exe)
```

## `rx` 子命令速览

| 子命令 | 作用 | 退出码约定(RXS-0083) |
|---|---|---|
| `rx check <input.rx>` | 仅做全量前端静态检查(借用/资源/类型),不产 codegen(RXS-0086) | 0=通过,1=诊断错误 |
| `rx build <input.rx> [-o <out>] [--emit=<target>]` | 编译;默认产 host EXE,`--emit` 可产 `ptx`/`pyd`/`mir`/`llvm-ir`(RXS-0084) | 0=成功 |
| `rx run <input.rx> [-o <out>]` | build 后执行产物,**透传产物退出码**(RXS-0085) | 透传 |
| `rx fmt [--check-idempotent] <file>` | 格式化 / 幂等校验 | 0=幂等 |
| `rx test [<file>] [--gpu]` | 发现并跑 `#[test]` / `#[test(gpu)]` | 0=全过 |
| `rx bench <name> [--smoke]` | 协议化微基准 | 透传 |
| `rx doc --root . --out target/doc` | 从单一事实源生成参考文档站 | 0=成功 |

## 跑通你的第一条命令

教程第一个示例就在仓库里,先验证环境:

```sh
cargo run -p rx -- check conformance/tutorial/01_hello.rx   # 期望退出 0
cargo run -p rx -- run   conformance/tutorial/01_hello.rx -o build/hello.exe
```

看到 `你好,Rurix` 即环境就绪。下一步 → [01 第一个程序](01_first_program.md)。

---

深入参考:`spec/toolchain.md`(RXS-0083~,rx CLI 语义)。
