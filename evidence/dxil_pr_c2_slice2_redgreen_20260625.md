# PR-C2 分片2 真实红绿证据 — RXS-0158 着色阶段着色 → DXIL 着色器类型降级

> 类型:实现 PR 红绿取证(G2.2 PR-C2 分片2,RXS-0158)。Provenance:`Assisted-by: kiro:claude-opus-4-8`(AI 代录机器可核对事实,非代决、非代签)。
> 工具链:patched llc 经 `RURIX_LLC=H:\llvm-clean-82c5bce5-build\bin\llc.exe`(RD-011 受控 dev 偏差,不改 D-205 pin)、dxc 签名 validator 经 `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`(1.9.2602.24)。
> 边界:本片只到「阶段 → DXIL 着色器类型 + shader profile」(类型/结构面);**不**定义阶段 I/O 签名 SV_*(RXS-0159)/ 内建变量·签名二进制 ABI(RFC-0003 §4.6)/ 纹理内存模型映射(06 §4.2)/ 绑定布局推导(G2.3)。

## 1. 落地阶段 `--target dxil` 端到端 validator accept(真实输出)

`rurixc --target dxil <accept>.rx`(feature `dxil-backend`),各阶段产对应 DXIL 着色器类型容器并经 dxc validator 接受:

```
=== vertex_noop (vertex → DXIL vertex shader, shadermodel6.0-vertex) ===
rurixc: --target dxil: DXIL container emitted + dxc validator accepted (build\probe\vertex_noop.dxc)
exit=0
=== fragment_noop (fragment → DXIL pixel shader, shadermodel6.0-pixel) ===
rurixc: --target dxil: DXIL container emitted + dxc validator accepted (build\probe\fragment_noop.dxc)
exit=0
=== compute_fn_noop (compute fn → DXIL compute shader, shadermodel6.0-compute) ===
rurixc: --target dxil: DXIL container emitted + dxc validator accepted (build\probe\compute_fn_noop.dxc)
exit=0
```

每阶段 shader type 正确:disasm golden(`tests/dxil/*.dxil-disasm`)证 `!dx.shaderModel = !{!"vs"/"ps"/"cs", 6, 0}`、PSVRuntimeInfo 标 Vertex/Pixel/Compute Shader。

## 2. deferred 阶段 reject → RX6008(真实输出)

mesh/task/RT 阶段合规降级越出阶段→着色器类型类型面(RD-012),请求降级 → RX6008(strict-only,不降级):

```
$ rurixc --target dxil conformance\dxil\reject\mesh_deferred.rx
error[RX6008]: DXIL shader-stage lowering not yet supported: mesh 着色阶段(→ DXIL mesh shader,SM6.5)的合规降级需线程组维度 + 输出拓扑声明,越出阶段→着色器类型类型面;映射已登记 RXS-0158 对应表,实现 deferred(RD-012)
 --> mesh_deferred.rx:6:9
exit=1
```

task → RX6008(amplification shader,需 DispatchMesh);raygen/closesthit/anyhit/miss → RX6008(library 多入口)。conformance/dxil/reject/{mesh,task,raygen}_deferred.rx 各 `//@ expect-error: RX6008` 全拦截。

## 3. 真实红绿:篡改阶段 codegen 输出 → golden 红 → 复原绿

篡改 `dxil_codegen::stage_target` 的 vertex 映射(`vertex`→错置为 `pixel`),重跑 DXIL golden + 单测:

### 红(篡改后)

```
test dxil_codegen::tests::vertex_stage_emits_dxil_vertex_shader ... FAILED
assertion failed: ir.contains("target triple = \"dxil-unknown-shadermodel6.0-vertex\"")

test dxil_ll_golden_matches ... FAILED
H:\rurix\...\tests/dxil\vs_noop.dxil-ll: DXIL IR golden 漂移
--- expected ---
target triple = "dxil-unknown-shadermodel6.0-vertex"
--- actual ---
target triple = "dxil-unknown-shadermodel6.0-pixel"

test dxil_disasm_golden_matches_when_toolchain_present ... FAILED
H:\rurix\...\tests/dxil\vs_noop.dxil-disasm: DXIL 反汇编 golden 漂移
test result: FAILED. 1 passed; 2 failed
```

IR 层 golden(always-on)+ disasm golden(validator 关卡)+ 单测三处同时捕获错置的 shader type。

### 绿(复原 `vertex`→`vertex` 后)

```
cargo test -p rurixc --features dxil-backend --test dxil_golden --lib dxil_codegen
→ 全 test result: ok(无 failed)
```

## 4. 验证命令汇总(全 PASS)

- `cargo build -p rurixc`(默认)+ `--features dxil-backend` 均过。
- `cargo test -p rurixc`(默认)+ `--features dxil-backend` 全过(dxil_codegen 8 单测含 vertex/fragment/compute accept + mesh/task/raygen RX6008;dxil_corpus accept 按 `//@ dxil-shader:` 裁定着色器类型 + reject RX6008;dxil_golden `.dxil-ll` always-on + `.dxil-disasm` validator 关卡)。
- `py -3 ci/trace_matrix.py --check` → PASS(158/158 全锚定,RXS-0158 带 conformance + tests + src 单测锚定)。
- `py -3 ci/check_schemas.py` → PASS(error_codes.json RX6008 + deferred.json RD-012)。
- `py -3 ci/bilingual_coverage.py` → PASS(zh/en 73/73 对齐,新增 codegen.dxil_stage_unsupported 双语覆盖)。
- `py -3 ci/check_guardrails.py HEAD`(正确 base = 分片1 tip)→ PASS(14 changed paths;evidence/ 既有文件 byte-unchanged,仅新增本报告)。
- `cargo clippy -p rurixc`(默认 + `--features dxil-backend --all-targets`)+ `cargo fmt -p rurixc -- --check` 干净。

## 5. LF 自核

新增/修改文件全 LF(无 CR、尾 0x0a),见 §6 自核输出。

## 6. LF 自核输出

对本片全部变更/新增文件(29 changed paths)逐字节核对 CR=0、尾字节 0x0a:

```
bad=0 files=29
```

（`.gitattributes` 为 `* -text`,禁一切换行转换、按原字节存取;故工作树即提交字节,全 LF。）
