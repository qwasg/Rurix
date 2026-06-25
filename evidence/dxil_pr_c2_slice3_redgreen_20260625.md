# G2.2 PR-C2 分片3（RXS-0159）真实红绿证据

> 日期:2026-06-25。范围:RXS-0159 阶段 I/O → DXIL 签名/系统值语义降级(**类型面**)。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`。本文件为 evidence/ 新增(只增不删不改)。
> 工具链诚实声明:dev 环境无 patched llc(RURIX_LLC,RD-011)/ dxc validator
> (RURIX_DXC_DIR),故 `.dxil-disasm` + validator 真验证 + `rx build --target dxil` 真跑
> 关卡 **SKIP**(per-file / 整体),真实 disasm 红绿在带工具链环境;本片 always-on 验证
> 落在 `.dxil-ll`(rurixc 自有 DirectX 三元组 LLVM IR + 类型面签名元数据,确定性)+
> dxil_codegen 单测 + conformance accept/reject。

## 1. 改动摘要

- spec/dxil_backend.md:落 `### RXS-0159`(内建变量→SV_* 映射表 + 插值限定→DXIL 插值
  限定符映射表 + Syntax/Legality/Dynamic Semantics/Implementation Requirements),
  显式声明签名二进制 ABI 布局(寄存器/偏移/component mask)属 §9 Q-Builtin 🔒 FFI ABI
  禁区不在本条;trace 全锚定 159/159。
- src/rurixc/src/dxil_codegen.rs:从 AST 阶段签名(形参 I/O 结构体=输入、返回=输出)
  + 字段 `#[builtin]`/`#[interpolate]` 映射 SV 语义名 / 插值限定符,经类型面签名元数据
  `!rurix.dxil.sig.in`/`.out` emit;不可映射内建变量 / 整数非 flat 插值 → RX6009。
- registry/error_codes.json:新增 RX6009(只追加)+ en/zh message-key。
- registry/deferred.json:新增 RD-013(带 I/O 签名入口 body 数据流降级 deferred)。
- conformance/dxil accept(vertex_io/fragment_io)+ reject(builtin_unmappable/interp_integer);
  tests/dxil vertex/fragment I/O `.dxil-ll` golden + bless_log。

## 2. 验证命令与真实输出

```
$ cargo build -p rurixc                      → Finished `dev` profile
$ cargo build -p rurixc --features dxil-backend → Finished `dev` profile
$ cargo test  -p rurixc --features dxil-backend
  test result: ok. 327 passed; 0 failed; 0 ignored  (lib 单测,含 dxil_codegen RXS-0159 6 例)
  test dxil_corpus.rs:   ok. 3 passed; 0 failed  (accept/reject + spec 锚定)
  test dxil_golden.rs:   ok. 3 passed; 0 failed
    dxil_disasm_golden_matches_when_toolchain_present → SKIP(无 patched llc/dxc,RD-011)
$ cargo clippy -p rurixc --features dxil-backend --all-targets → 0 warning
$ cargo clippy -p rurixc --all-targets                         → 0 warning
$ cargo fmt -p rurixc -- --check                               → 干净
$ py -3 ci/trace_matrix.py --check   → PASS (159/159 clauses anchored, 438 files)
$ py -3 ci/check_schemas.py          → [check_schemas] PASS
$ py -3 ci/bilingual_coverage.py     → [bilingual] PASS (zh/en 74/74 对齐;提交快照冻结不改)
$ py -3 ci/check_guardrails.py origin/feat/g2.2-pr-c2-slice2-rxs0158 → PASS (12 changed)
$ py -3 ci/check_guardrails.py main  → PASS (29 changed)
```

## 3. 真实红绿（篡改 SV 语义映射 → golden 红 → 复原绿）

篡改:`dxil_codegen::sv_semantic` 中 `(Vertex, Out, "position") => Some("SV_Position")`
改为 `Some("SV_BROKEN_REDGREEN")`。

### 红(篡改后)

```
$ cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_ll_golden_matches
  test dxil_ll_golden_matches ... FAILED
  H:\rurix\...\tests\dxil\vs_io.dxil-ll: DXIL IR golden 漂移
    --- expected ---  !1 = !{!"pos", !"SV_Position"}
    --- actual   ---  !1 = !{!"pos", !"SV_BROKEN_REDGREEN"}
  test result: FAILED. 0 passed; 1 failed
$ cargo test -p rurixc --features dxil-backend  (lib)
  test dxil_codegen::tests::vertex_io_lowers_to_dxil_signature_semantics ... FAILED
  test result: FAILED. 326 passed; 1 failed
```

### 绿(复原 `SV_Position` 后)

```
$ cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_ll_golden_matches
  test result: ok. 1 passed; 0 failed
$ cargo test -p rurixc --features dxil-backend vertex_io_lowers
  test dxil_codegen::tests::vertex_io_lowers_to_dxil_signature_semantics ... ok
  test result: ok. 1 passed; 0 failed
```

## 4. 类型面边界(硬规则 5)

- 仅落 SV 语义名 + 插值限定符映射(`!rurix.dxil.sig.in`/`.out` 元数据 = `!{!"field",
  !"semantic"}`);**未 emit 任何寄存器/字节偏移/component mask**——签名二进制 ABI 布局由
  LLVM DirectX 后端 emit、经 dxc validator 验证,Rurix 不定义/不冻结(RFC-0003 §4.6 /
  §9 Q-Builtin 🔒 FFI ABI 禁区)。spec 条款显式声明该边界。
- 未碰 🔒 纹理内存模型映射(06 §4.2)/ 绑定布局推导(G2.3,P-11)/ 阶段间接口(RXS-0160);
  PTX 后端 / committed D-205 pin / toolchain.rs 未动;DXIL gate 维持 feature `dxil-backend`。
- 入口 body 数据流降级 deferred(RD-013),本片仅签名类型面 + void 入口 stub。
