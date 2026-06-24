# DXIL PR-C2 分片1 — 真实红绿 + 端到端 run 日志(G2.2,RXS-0157)

> 新增 evidence(本 PR 首次落盘,非既有篡改)。Provenance:`Assisted-by: kiro:claude-opus-4-8`。
> 工具链(dev 偏差,RD-011):patched llc = `H:\llvm-clean-82c5bce5-build\bin\llc.exe`
> (经 `RURIX_LLC`);dxc 签名 validator = `H:\dxc-round7\extracted\bin\x64`
> (dxv.exe/dxc.exe 1.9.2602.24,经 `RURIX_DXC_DIR`)。G-G2-2 device 真跑/呈现对照
> 仍 open,本片只到工具链 codegen + golden(AI 不代签)。

## 1. 端到端最小 compute(`rx build --target dxil`)

输入 `cs_noop.rx`(空体 compute kernel `kernel fn cs_noop() {}`):

```
$ rx build --target dxil cs_noop.rx -o on.dxc
rurixc: --target dxil: DXIL container emitted + dxc validator accepted (...\on.dxc)
on-exit=0

$ dxv.exe on.dxc            # 独立签名 validator 复验
Validation succeeded.
dxv-exit=0
```

降级链路:MIR(kernel 根)→ DirectX 三元组 LLVM IR
(`dxil-unknown-shadermodel6.0-compute` + `hlsl.shader`=compute/`hlsl.numthreads`=1,1,1)
→ patched llc `-filetype=obj` → DXIL 容器 → dxc validator **accept**。

## 2. strict-only reject(RX6007,P-01 无 fallback)

```
# feature 未启用(L1 后端不可用):
$ rx build --target dxil cs_noop.rx          # 默认 cargo build -p rx(无 dxil-backend)
error[RX6007]: ... `--target dxil` 需启用 cargo feature `dxil-backend`(...)
off-exit=1

# 子集外构造(L2,非平凡体):
$ rx build --target dxil conformance/dxil/reject/nontrivial_body.rx
error[RX6007]: ... DXIL 最小 compute 子集暂不支持非平凡 compute 体(...)
rej-exit=1
```

## 3. golden 真实红绿(篡改 → 红 → 复原 → 绿)

`tests/dxil_golden.rs`(`.dxil-ll` rurixc 自有 IR + `.dxil-disasm` 经 validator 接受后的
`dxc -dumpbin` 反汇编)。

| 阶段 | 动作 | 结果 |
|---|---|---|
| GREEN 基线 | 无篡改 | `test result: ok. 3 passed` |
| RED | 篡改 `render_dxil_module` numthreads `1,1,1`→`8,1,1` | `2 failed`:`cs_noop.dxil-ll: DXIL IR golden 漂移` + `cs_noop.dxil-disasm: DXIL 反汇编 golden 漂移`(expected `1,1,1` / actual `8,1,1`) |
| GREEN 复原 | numthreads 复原 `1,1,1` | `test result: ok. 3 passed` |

红相 diff 摘录:
```
--- expected ---
attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }
--- actual ---
attributes #0 = { noinline nounwind "hlsl.numthreads"="8,1,1" "hlsl.shader"="compute" }
```

## 4. 测试 / CI 门(本机真实输出)

| 命令 | 结果 |
|---|---|
| `cargo build -p rurixc -p rx`(默认) | Finished |
| `cargo build -p rurixc -p rx --features dxil-backend` | Finished |
| `cargo test -p rurixc -p rx`(默认全量) | all `test result: ok`(含既有 316 lib + 各 corpus) |
| `cargo test -p rurixc --features dxil-backend`(全量) | all ok(lib 316 含 dxil_codegen 2 + dxil_corpus 3 + dxil_golden 3) |
| `py -3 ci/trace_matrix.py --check` | PASS(**157/157** 全锚定;新增 RXS-0157 有锚定、无悬空) |
| `py -3 ci/check_schemas.py` | PASS(RX6007 entry/message_key 交叉校验) |
| `py -3 ci/bilingual_coverage.py` | PASS(zh/en key 集对齐 72/72,RX6007 双语覆盖) |

## 5. 边界声明

- patched llc / patch 二进制不入库(RD-011 隔离仓库外);committed D-205 pin
  (`C:\Program Files\LLVM`)/ `toolchain.rs` locate_clang 未改;DXIL llc 仅经 `RURIX_LLC`
  dev env 解析(env 缺失回落 pin 候选,均不可用 → SKIP,非静默 fallback)。
- 本片不碰 🔒 纹理内存模型映射(06 §4.2)/ FFI ABI 二进制布局(RFC-0003 §4.6)/
  绑定布局推导(G2.3,P-11);最小子集仅空体 compute 入口,语句/形参降级随后续分片。
