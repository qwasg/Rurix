# 00 · 安装与工具链

> 语言 1.0 已发行(v1.0.0):stable 面(含 `rx` 命令面)已冻结,同一 edition 内只增不破坏(RD-008 已关闭)。

## 环境前提

| 项 | 要求 |
|---|---|
| 操作系统 | Windows 11(原生 COFF/PE/PDB 工具链) |
| GPU | NVIDIA(开发对照机 RTX 4070 Ti) |
| 驱动/运行时 | CUDA Toolkit + CUDA Driver API |
| C++ 工具链 | MSVC 2022(仅 `rx build`/`rx run` 的链接期需要;`rx check` 零此前提) |
| 构建宿主 | Rust 工具链(**仅方式 B 源码构建需要**;方式 A 预编译安装零 Rust 前提,D-201) |

> 仅做 `rx check`(纯前端静态检查)不需要 GPU 也不需要上表系统级前提;`rx run`/`rx bench` 等执行 GPU 路径才需要真实设备。GPU/MSVC/CUDA 为**文档化系统级前提,不计入安装时长**(rustup 同类口径)。

## 方式 A:安装预编译工具链(rurixup,推荐)

1. 从 [GitHub Releases](https://github.com/qwasg/Rurix/releases) 下载 `rurixup.exe`(bootstrap 空窗诚实说明:此步保护 = TLS + 手动核对 `SHA256SUMS`,与 rustup-init 同构;Authenticode 为自签测试证书,是纵深**非**信任根)。
2. 安装(经 repo 内信任根锚,四级内容寻址校验,任一级失配拒装):

```sh
rurixup.exe install v1.0.1-dist.1 --channel-file https://raw.githubusercontent.com/qwasg/Rurix/main/channels/stable.json
```

   成功输出 `RURIXUP_INSTALL: ... digest_levels_verified=4`,工具链物化到 `%USERPROFILE%\.rurix\toolchains\<版号>\bin\`(含 `rx.exe` 与 `bin\lib\rurix_rt_cabi.lib`——无 Rust 环境亦可 `rx build`)。
3. PATH 接入:`rurixup setup` 打印接入指令(默认不改环境);`rurixup setup --add-path` 显式写入用户 PATH。切换/列出版本:`rurixup default <版号>` / `rurixup list`。
4. 验证:`rx check <file.rx>` 退出 0。

> 当前可安装版号 = `v1.0.1-dist.1`(EA1.2 首次发布演练产物,pre-release;信任根锚 `channels/stable.json` 只登记经 owner 人工门合入的版本)。安装时长与网络带宽相关;冷启动验收采用两段式口径(干净 VM 至 `rx check` / 干净账户至首 kernel,各 ≤10 分钟 measured,RFC-0012 §4.10),本文档不作无限定的时长承诺。

## 方式 B:从源码构建(贡献者路径)

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
