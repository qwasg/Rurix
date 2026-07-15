# Rurix 语言规范 — single-source 宿主 GPU 编排语义面(std::gpu 宿主编排首期收敛子集 + rxrt C ABI 运行时边界 + 前端机械面;MS1.2 起)

> 条款:**RXS-0189 ~ RXS-0196**(MS1.2 波,本轮落带编号条款体)。**RXS-0197 ~ RXS-0199 为本文件预留区间**(MS1.2b:RXS-0197 present 宿主 typestate 面(镜像 RXS-0142,D-130 边界)/ RXS-0198 present backbuffer 借用缓冲与 blit 契约(对齐 RXS-0143)/ RXS-0199 宿主图像落盘桥(RXS-0114~0117 语义复用)——本轮**仅登记区间预留、不落其裸条款头**,条款体与测试锚定随 MS1.2b 实现 PR 同落)。体例见 [README.md](README.md)。
> 依据:**[RFC-0009](../rfcs/0009-host-gpu-orchestration.md)**(single-source 宿主 GPU 编排 stdlib std::gpu,**Agent Approved 2026-07-14**,§9 八项裁决全锁,§4 参考级设计为本文件条款内容事实源;🔒 §4.3 C ABI 边界 / §4.4 launch marshalling 为 agent 亲笔 FFI 面);01 §4/§6(使命判据最严口径:宿主编排也须 `.rx`);05 §1;08 §1/§2(rurix-rt 运行时对象,D-230~D-234);spec/device.md RXS-0066(函数着色)/ RXS-0073(ptxas 干验证)/ RXS-0074/0075(launch 类型契约与诊断)/ RXS-0076(PTX 装载协商)/ RXS-0077(poisoned context 状态机)/ RXS-0082(libdevice 链接纪律);spec/borrow.md RXS-0054(move 检查)/ RXS-0057~0061(借用主体);spec/pipeline.md RXS-0130~0134(affine 资源所有权语义);spec/interop.md RXS-0125(手写 extern "C" C ABI 口径);spec/release.md RXS-0150/0151(分发产物变体模型与 fatbin 装载协商序);spec/edition.md RXS-0180(stable 面加性演进 L2 / 快照非 ABI L3 口径)。授权:[../milestones/ms1/MS1_CONTRACT.md](../milestones/ms1/MS1_CONTRACT.md)(`in_scope: host_gpu_orchestration_stdlib`,D-MS1-2,G-MS1-1 / G-MS1-2)+ MS1_PLAN MS1.2 / MS1.2b。
> 档位:**Full RFC**(RFC-0009;10 §3 触发面:launch / 传输 / affine 资源 drop 的**运行时语义**首次进语言面 + 宿主 codegen ↔ 运行时 **FFI ABI** 边界,AGENTS 硬规则 5)。**agent 自主判档**,判档争议向上取严。任何触及 **std::gpu 首期子集外**(AsyncBuffer 宿主面 / Event / 跨线程转移 / 多 stream 重叠编排 / 非 {f32,i32,u32} 元素 / `Result` 错误面,RD-026)/ **真 NVIDIA fatbinary 容器格式** / **跨包 `.rx` 依赖编译** / **窗口·输入·事件循环进语言**(D-130 维持,RD-027;扩张诱惑登记 SG-010)/ **多后端**(D-008/SG-003)/ **DXIL 图形路**(RXS-0171 白名单不动)/ **`#[export(c)]`**(RD-009)的条款,必须停下标注「需升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):宿主编排误用以**编译期诊断**(RX3015 / RX2010 / RX6024 / RX6025 / RX7021 / RX7022 / RX1005,P-01 strict-only)定义,运行期失败以**确定性诊断 + 终止 + poisoned 传播**(RXS-0193)定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 commit 先于实现 commit**(MS1.2 单 PR 内,G-MS1-1);`ci/trace_matrix.py --check` 要求每条 `### RXS-####` ≥1 测试锚定(`//@ spec: RXS-####`)——本文件八条的锚定测试(conformance/host_orch/* 语料 + rurixc/cabi 单测 + ci/host_orch_smoke.py)随 MS1.2 实现 commit 同 PR 落,trace_matrix 至 184→192 全锚定;stable 快照因条款计数增长同 PR 重 bless(RXS-0180 L2 加性演进)。

---

## 1. 范围与编号区间

本文件承载 **single-source 宿主 GPU 编排语义面**的语义条款(MS1.2+,D-MS1-2):host `.rx` 与 `kernel fn` 同编译单元,host 侧经 `std::gpu` 首期收敛子集(`Context` / `Stream` / `Buffer` / `PinnedBuffer` + `launch` / 同步 / 传输)编排 GPU;`rx build` 一步产单 EXE——device 段照既有 device 路编为 PTX(+可选 sm cubin)以描述表形态嵌入 host 产物,运行期经既有装载协商加载;宿主调用经薄 C ABI 层 `src/rurix-rt-cabi`(staticlib,`rxrt_*` 符号)绑定 rurix-rt。覆盖语义面(RFC-0009 §4.1~§4.5 + 前端机械面):

- **std::gpu 宿主编排类型面与 affine 语义**(RXS-0189):lang item 句柄类型 + 方法集存在性 + 宿主 API 着色合法性(kernel/device 内使用 → RX3015);move/borrow 违例复用既有裁决,零新借用码。
- **方法签名与元素类型推断**(RXS-0190):编译器已知签名;元素类型经使用点推断合一,首期 T ∈ {f32, i32, u32};不可定型 → RX2010。
- **launch 宿主 lowering 与实参借用/marshalling**(RXS-0191):把 RXS-0074/0075 的类型契约**兑现为执行语义**;实参子集外 → RX6024;🔒 slot+kinds marshalling(agent 亲笔 FFI 面)。
- **single-source device 产物嵌入与装载协商**(RXS-0192):PTX 必存 + 可选 sm cubin 描述表嵌入;运行期复用 RXS-0150/0151 协商与 RXS-0076 版号梯子;嵌入失败 → RX6025。
- **宿主运行期错误与 poisoned 语义**(RXS-0193):确定性诊断 + 终止;poisoned 传播对齐 RXS-0077;**无 UB、无静默降级**(P-01)的正面语义条款。
- **rxrt C ABI 运行时边界**(RXS-0194,🔒 FFI):`rxrt_*` 符号面 / u64 句柄 / 销毁纪律;含义冻结、布局不冻结为语言 ABI(RXS-0180 L3 口径)。
- **extern "C" 符号保名与 `#[link]` 接线**(RXS-0195):前端机械面;`#[link]` 库定位失败 → RX7022,rurix_rt_cabi 定位/构建失败 → RX7021(工具链码按内聚并入本条,见 §4)。
- **out-of-line 模块文件加载**(RXS-0196):`mod name;` 同目录装配;缺失/循环 → RX1005。

不在本文件落语义本体的范围:present 宿主 typestate 面与宿主图像落盘桥(RXS-0197~0199 预留,随 MS1.2b 落体;窗/泵/交换链维持 C++ shim,D-130 0-byte);std::gpu 首期子集外(RD-026);真 NVIDIA fatbinary 容器;跨包 `.rx` 依赖 / `rx new` 脚手架;`#[export(c)]`(RD-009,本面为**导入**方向,消费手写 extern "C",RXS-0125 口径 0-byte)。

**编号区间**:本文件条款自 **RXS-0189** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0188 @ [release.md](release.md);RXS-0181~0184 已被 GRX showcase 分支 claim,跳号避撞维持)。MS1.2 落 **RXS-0189 ~ RXS-0196**(本轮条款体),MS1.2b 落 **RXS-0197 ~ RXS-0199**(本轮仅预留)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款(RXS-0189 ~ RXS-0196)

> 各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(10 §7.5)。条款内引用既有条款号即为语义复用,不重述其语义本体。编译期违例引 §3 错误码(spec 先行引用,registry 落码与 en/zh message-key 随实现 commit 同 PR 落);运行期失败不占 RX 段位(RXS-0193,工具层口径对齐 [release.md](release.md) §3 / RXS-0076/0077 先例)。gate 形态 = **无独立 feature gate**(加性 stdlib 面,RFC-0009 §6;误用全部 strict-only 编译期拒,不完整路径无静默出口)。

### RXS-0189 std::gpu 宿主编排类型面与 affine 语义

**Syntax**(宿主编排类型与方法集,lang items):

```
Context / Stream<C> / Buffer<C, T> / PinnedBuffer<C, T>       // 非 Copy affine 句柄结构
Context::create() -> Context                                   // affine 根;每调用点合成新鲜 brand C
ctx.stream() -> Stream<C>          ctx.sync()
ctx.alloc(n: usize) -> Buffer<C, T>          ctx.alloc_pinned(n: usize) -> PinnedBuffer<C, T>
buf.upload(&PinnedBuffer<C, T>)    buf.download(&mut PinnedBuffer<C, T>)    buf.len() -> usize
pinned.get(i: usize) -> T          pinned.set(i: usize, v: T)               pinned.len() -> usize
stream.launch(kernel, GridDim(..), BlockDim(..), (args..))     stream.sync()
```

**Legality**:

- 类型为编译器 lang items(`Context` / `Stream` / `Buffer` / `GridDim` / `BlockDim` 既有 + 新增 **`PinnedBuffer`**),用户同名定义优先遮蔽、语义不变(兜底纪律沿 RXS-0074)。全部句柄类型为**非 Copy affine**:move 后再用 / 重复 move / 借用冲突等违例**复用 RXS-0054 与 RXS-0057~0061 既有裁决**(零新借用码);affine 资源所有权语义对齐 RXS-0130~0134,本条不重述。
- **宿主 API 着色合法性**:std::gpu 宿主类型的构造与方法调用**仅 host 着色上下文合法**;出现在 `kernel` / `device fn` 体内 → **RX3015**(coloring 层,与 RX3001 同点位,扩展 RXS-0066 着色格)。
- **brand 契约**:`Context::create()` 每个调用点合成新鲜 opaque brand 类型 `C`;`Stream<C>` / `Buffer<C, T>` 泛型签名契约与跨 context 资源误用裁决(RX3006)复用 RXS-0074,原样生效。

**Dynamic Semantics**:

- affine 句柄 drop 按声明逆序发生(buffer → stream → ctx);drop 触发的运行时销毁遵守 RXS-0194 销毁纪律,失败语义封口于 RXS-0193。类型面自身无其余附加运行期语义(launch / 传输执行语义见 RXS-0191)。

**Implementation Requirements**:

- 句柄为编译器合成布局(`handle: u64` + brand 幽灵参数);方法集经 typeck 编译器已知签名分支表达(RXS-0190,先例 `Stream::launch` / ThreadCtx / Atomic 分支)。
- brand 调用点合成若在推断上不可行,降级方案 = 单 brand + cabi 运行期 context-id 校验(RX3006 保泛型签名面;RFC-0009 §9 Q-Brand,实现 PR 定案回填)。
- 首期方法不返回 `Result`(enum 变体构造 codegen 未通);错误面为后续加性扩展(RD-026),运行期失败语义见 RXS-0193。

> 测试锚定:conformance/host_orch/accept 单源语料(0 诊断)+ reject(move 后再用 / kernel 体内宿主 API → RX3015)UI 语料(随实现 commit 同 PR 落)。

### RXS-0190 宿主 GPU 方法签名与元素类型推断

**Legality**(编译器已知签名 + 元素类型定型,编译期 typeck 层):

- RXS-0189 方法集的签名为**编译器已知**(typeck 已知签名分支):接收者类型判定 + 实参/返回定型按已知签名合一;非法接收者 / 元数不符走既有类型诊断,不另立新码。
- 缓冲元素类型 `T`(`ctx.alloc(n)` / `ctx.alloc_pinned(n)` 产出的 `Buffer<C, T>` / `PinnedBuffer<C, T>`)经**使用点推断合一**(`set` / `get` / `upload` / `download` / launch 实参位等约束点);**首期 T ∈ {f32, i32, u32}**。
- **元素类型不可定型 → RX2010**:函数体内无任何使用点约束致 `T` 不可推断,或推断合一结果超出首期子集 {f32, i32, u32}(同一用户面问题——无法为缓冲定出首期合法元素类型;2xxx 类型段续接 RX2009)。

**Implementation Requirements**:

- 推断在 typeck 层实施,诊断 span 指向元素类型不可定型的 alloc 调用点;Err 容忍不级联(防一错多报,RXS-0075 同口径)。

> 测试锚定:conformance/host_orch/reject/elem_infer(RX2010)+ typeck 单测(随实现 commit 同 PR 落)。

### RXS-0191 launch 宿主 lowering 与实参借用/marshalling

**Syntax**:launch 调用形态复用 RXS-0074 `LaunchCall` 产生式,本条不改文法;本条为其**宿主执行语义与 lowering**。

**Legality**:

- **类型契约原样生效**:RXS-0074/0075 launch 类型契约(着色 RX3004 / 维度 RX3005 / brand RX3006 / 参数 RX2001)不变——本条兑现其执行语义,不改契约本体。
- **kernel 引用编译期绑定**:`KernelRef` 须解析到**同编译单元 `kernel fn`**(launch_check 既有裁决);kernel 非 host 值,不引入函数指针型。
- **首期实参子集** = `Buffer<C, T>` + 标量 {i32, u32, f32, usize};子集外实参 → **RX6024**(编译期,镜像 RX6003/6005 子集纪律;首期子集外扩张归 RD-026)。
- **借用语义**:launch 对 device 形参 `ViewMut` 位的 Buffer 实参取 `&mut` 调用期短借用、`View` 位取 `&`;借用于调用表达式期间存续,重叠冲突由既有 NLL 裁决(RXS-0057~0060)。

**Dynamic Semantics**:

- launch 表达式求值 = 以编译期绑定的 kernel 符号与物化实参调用运行时入口 `rxrt_launch`(RXS-0194):kernel 符号为 device MIR 同源 mangle 符号的 NUL 终止字符串常量(与 device 入口名**单一事实源**);grid/block 维度按 RXS-0074 维度契约展开为六个 u32。
- 🔒 **marshalling**(RFC-0009 §4.4,agent 亲笔 FFI 面):实参元组物化为栈上 `[u64; n]` slot 数组 + `kinds` 字节数组(0 = Buffer 句柄,运行时换设备指针;1 = 标量,按位样式存 slot 低位,little-endian,运行时按形参尺寸读前 4/8 字节);运行时内部构造 `kernelParams` 指针数组指向各 slot。该布局**含义冻结、不冻结为语言 ABI**(RXS-0180 L3 口径)。
- launch 异步返回;返回后 device 侧使用窗口不由借用面表达——无 use-after-free 由 RXS-0194 销毁纪律(free 前 sync)保证,运行期失败语义封口于 RXS-0193。

**Implementation Requirements**:

- mir_build 直降字面符号(`CallTarget::Fn { symbol: "rxrt_*" }`,不走 `mangle()`,不走 prelude 源注入,RFC-0009 §7 否决项);host codegen 既有 extern declare 机制零改动发射 declare。
- 物化纪律复刻 `interop.rs AcquiredFrame::launch` 已验证代码(单一先例源)。

> 测试锚定:conformance/host_orch/accept/saxpy_single_source.rx + reject/arg_subset(RX6024)(随实现 commit 同 PR 落)。

### RXS-0192 single-source device 产物嵌入与装载协商

**Syntax**(嵌入描述表,host codegen 私有常量,非用户文法面):

```
@__rx_gpu_ptx / @__rx_gpu_cubin_sm89 / @__rx_gpu_artifacts    // host 产物内私有常量描述表
```

**Legality**:

- host 编译单元含 `kernel fn` 时,device 路必走:既有 device codegen 产 PTX(编译期干验证复用 **RXS-0073** ptxas 纪律,libdevice 链接纪律复用 **RXS-0082**);**PTX 文本(NUL 终止)必存嵌入**;`ptxas` 在位时可选预编 sm cubin(首期 sm_89)一并入描述表。
- **嵌入阶段失败**(device 路产物无法产出 / 描述表无法构造)→ **RX6025**(编译期,6xxx codegen 段)。
- **工具链缺失**(ptxas 不在位等)按既有 SKIP 纪律嵌**哨兵产物**:编译不失败,运行期首次 gpu 操作走确定性失败(RXS-0193),**不静默降级**。

**Dynamic Semantics**:

- `Context::create()` 降级为 `rxrt_ctx_create(ptr @__rx_gpu_artifacts)`(**注册即传参**,不走链接段魔法);运行期组 `DeviceArtifactSet`(PTX fallback 必存 + sm 键 cubin),装载协商复用 **RXS-0150(变体模型)/ RXS-0151(协商序)**,PTX 版号梯子复用 **RXS-0076**;module / CUfunction 按 entry 惰性缓存。

**Implementation Requirements**:

- 嵌入形态 = PTX 文本 + 裸 cubin 字节(DeviceArtifactSet 语义 0-byte;真 NVIDIA fatbinary 容器维持 defer,§4);可复现构建(RXS-0097)不受影响。

> 测试锚定:ci/host_orch_smoke.py 篡改嵌入 PTX 红绿(装载协商拒 → 红,复原 → 绿)+ driver 嵌入单测(随实现 commit 同 PR 落)。

### RXS-0193 宿主运行期错误与 poisoned 语义

**Dynamic Semantics**(宿主编排失败面的正面闭合语义):

- 任何 `rxrt_*` 运行期失败(CUDA 错误 / 装载协商拒 / 哨兵产物):落 stderr **确定性诊断**(操作名 + CUresult/原因 + context 序号)后**进程终止**(abort);同输入同失败点,诊断内容确定。
- **poisoned 传播对齐 RXS-0077**:poisoned context 后,该 context 上全部后续 gpu 操作确定性失败(诊断 + 终止),不级联损坏。
- **无 UB、无静默降级**(P-01 strict-only):宿主编排全部失败路径闭合于 {编译期 RX 诊断(§3),运行期确定性诊断 + 终止},**不存在未定义行为出口**——affine 检查覆盖不了的异步窗口由 RXS-0194 销毁纪律封口,本条为该承诺的语义收口条款。

**Implementation Requirements**:

- 确定性诊断与终止由 cabi 层(RXS-0194)实施;运行期失败**不占编译期 RX 段位**(工具层口径,对齐 [release.md](release.md) §3 与 RXS-0076/0077 先例)。
- `Result` 错误面为后续加性扩展(依赖 enum codegen,RD-026);首期失败即终止,无部分恢复面。

> 测试锚定:cabi 单测(错序 / 失败注入 → 确定性诊断)+ ci/host_orch_smoke.py 失败路径见证行(随实现 commit 同 PR 落)。

### RXS-0194 rxrt C ABI 运行时边界(🔒 FFI)

**Syntax**(`rxrt_*` 符号面,`src/rurix-rt-cabi` staticlib;RFC-0009 §4.3,agent 亲笔):

```
rxrt_ctx_create(artifacts: *const u8) -> u64      // 0 = 失败;artifacts → RXS-0192 嵌入描述表
rxrt_ctx_destroy(ctx: u64)                        // 先 sync 再销毁(D-231 镜像)
rxrt_ctx_sync(ctx: u64) -> i32
rxrt_stream_create(ctx: u64) -> u64 / rxrt_stream_destroy(s) / rxrt_stream_sync(s) -> i32
rxrt_buf_alloc(ctx: u64, bytes: u64) -> u64 / rxrt_buf_free(b)   // free 前 ctx sync
rxrt_buf_upload(b: u64, src: *const u8, bytes: u64) -> i32
rxrt_buf_download(b: u64, dst: *mut u8, bytes: u64) -> i32
rxrt_pinned_alloc(ctx: u64, bytes: u64) -> u64 / rxrt_pinned_ptr(p) -> *mut u8 / rxrt_pinned_free(p)
rxrt_launch(s: u64, entry: *const u8, gx,gy,gz,bx,by,bz: u32,
            slots: *const u64, kinds: *const u8, n_args: u64) -> i32
```

**Legality**:

- 符号面**含义冻结**(每符号语义一经发布不改,只可追加新符号,10 §6 口径),**布局/签名细节不冻结为语言 ABI**(RXS-0180 L3 口径,对齐 RXS-0162/0165 先例):`rxrt_*` 面是工具链内部实现要求,**非用户 stable ABI**;用户面是 RXS-0189 的类型/方法语义。
- C ABI 形态复用 **RXS-0125** 手写 extern "C" 口径(`#[unsafe(no_mangle)] extern "C"`,staticlib 先例 rurix-interop),不触 `#[export(c)]`(RD-009)。

**Dynamic Semantics**:

- 句柄为 u64 不透明值(u64 句柄表 + 状态断言);无效句柄 / 状态违例 / 驱动失败 → 确定性失败(RXS-0193),不产生 UB。

**Implementation Requirements**(销毁纪律,🔒 agent 亲笔):

- `rxrt_ctx_destroy` 先 sync 再销毁(镜像 `Context::drop`,D-231);`rxrt_buf_free` 先 `cuCtxSynchronize` 再 `cuMemFree`(防 in-flight use-after-free);upload/download 走同 stream 序或 ctx sync——affine 检查覆盖不了的异步窗口由本纪律封口(RXS-0193 无 UB 承诺的实现支撑)。
- unsafe 全部集中于 `src/rurix-rt-cabi`:逐处 `// SAFETY:` + unsafe-audit **U25** 登记(硬规则 9);全仓其余 crate `unsafe_code=deny` 维持。
- cabi 内部包 rurix-rt `pipeline.rs` ownership 系 + `fatbin.rs` 装载协商 + `interop.rs` 帧机,**不复制第二份运行时逻辑**(单一事实源);`rxp_*` / `rxio_*` 延伸符号遵同一纪律,其语义随 RXS-0197~0199(MS1.2b)落体。

> 测试锚定:src/rurix-rt-cabi 单测(句柄表 / 状态断言 / 销毁纪律)(随实现 commit 同 PR 落)。

### RXS-0195 extern "C" 符号保名与 `#[link]` 接线

**Syntax**(前端机械面):

```
ExternBlock ::= Attr* "extern" "\"C\"" "{" ExternFnDecl* "}"   // 无 body fn 声明
LinkAttr    ::= "#[" "link" "(" "name" "=" StringLit ")" "]"   // 修饰 extern 块
```

**Legality**:

- extern "C" 块内无 body fn 以**字面名**参与 codegen declare 与链接(不 `mangle()`;修缮 mir_build 对 extern fn 的 mangle 路径)。
- `#[link(name = "x")]` 修饰 extern 块:driver link 段追加 `x.lib`;**库定位失败 → RX7022**(7xxx 工具链段,诊断带定位信息与可执行指引)。
- **隐式运行时库接线**(RX7021 按内聚并入本条,§4 留痕):当编译单元实际使用 gpu(及后续 present/imageio)宿主 API 时,driver link 段追加 `rurix_rt_cabi.lib` + Rust staticlib 所需系统库固定集(以 `--print native-static-libs` 实测 pin);`.lib` 定位序 = `RURIX_RT_CABI_LIB` env → rx.exe 旁 `lib/` → workspace `target/release/`(缺库时 `rx build` 编排 `cargo build -p rurix-rt-cabi --release`,先例 `build_pyd()`);**定位/构建失败 → RX7021**。host-only 程序(不使用宿主 API)链接线零漂移。

**Dynamic Semantics**:

- 本条为编译/链接期机械面,无运行期语言语义;链接产物中 extern 符号在装载期由链接器既有约定解析(D-209 link.exe)。

**Implementation Requirements**:

- driver.rs 既有 link.exe 命令追加库(D-209);诊断(RX7021/RX7022)含定位序与失败原因,措辞允许保守粗糙(07 §4)。

> 测试锚定:conformance/host_orch/link 语料(RX7022)+ driver 单测(定位序 / RX7021 路径)(随实现 commit 同 PR 落)。

### RXS-0196 out-of-line 模块文件加载

**Syntax**:

```
ModDecl ::= "mod" Ident ";"        // out-of-line 形态(无花括号)
```

**Legality**:

- `mod name;` 装配为「同目录 `name.rx` 文件内容作为该模块 body」的**内联等价形态**:装配后语义与 `mod name { <文件内容> }` 一致,resolve / typeck 语义零改动(RXS-0032~0038 名称语义原样适用)。
- **文件缺失 / 装配循环** → **RX1005**(1xxx 解析/模块装配段续接 RX1004;诊断 span 指向 `mod` 声明)。
- **跨包 `.rx` 依赖不在本条**(仅同包同目录;包间复用留后续判档,§4)。

**Implementation Requirements**:

- parser 接受无花括号形态;driver 在 parse 后按同目录 `name.rx` 装配(SourceMap 多文件既有支持);循环检测以装配栈判定。

> 测试锚定:conformance/host_orch/mod_file accept(装配等价)/ reject(缺失 / 循环 → RX1005)(随实现 commit 同 PR 落)。

## 3. 错误码引用汇总

> 编译期新码按**真实可达类别**分配(07 §5 语义分配段位;spec 先行引用,`registry/error_codes.json` 落码 + en/zh message-key 成对随 MS1.2 实现 commit 同 PR 落,bilingual 覆盖以实现实际可达为准、不预留不预造);**运行期失败不占 RX 码**(RXS-0193 确定性诊断 + 终止,工具层口径对齐 [release.md](release.md) §3)。含义冻结、只追加(10 §6)。

| 错误码 | 段位 | 含义 | 条款 |
|---|---|---|---|
| RX1005 | 1xxx 解析/模块装配 | `mod name;` 同目录文件缺失 / 装配循环 | RXS-0196 |
| RX2010 | 2xxx 类型 | 宿主 GPU 缓冲元素类型不可定型(无使用点不可推断 / 合一结果超出首期子集 {f32,i32,u32}) | RXS-0190 |
| RX3015 | 3xxx 着色 | std::gpu 宿主 API 出现在 `kernel` / `device fn` 体内(宿主 API 着色违例,与 RX3001 同点位) | RXS-0189 |
| RX6024 | 6xxx codegen | launch 实参超出首期 marshalling 子集(Buffer + {i32,u32,f32,usize} 之外) | RXS-0191 |
| RX6025 | 6xxx codegen | single-source device 产物嵌入失败(device 路产物 / 描述表构造失败) | RXS-0192 |
| RX7021 | 7xxx 工具链 | rurix_rt_cabi 运行时库定位 / 构建编排失败(定位序耗尽) | RXS-0195 |
| RX7022 | 7xxx 工具链 | `#[link(name)]` 指定库定位失败 | RXS-0195 |

launch 既有契约码(RX3004/RX3005/RX3006/RX2001)复用 RXS-0074/0075,本文件零重复分配;move/borrow 违例复用 RXS-0054/0057~0061 既有裁决(零新借用码)。

## 4. 升档 / 禁区留痕

- **本文件档位 = Full RFC(RFC-0009)**:触 launch/传输/销毁**运行时语义**进语言面 + 宿主 codegen ↔ 运行时 **FFI ABI** 边界(硬规则 5 / 10 §3);🔒 §4.3(C ABI 边界)/ §4.4(launch marshalling)为 agent 亲笔 FFI 面,批准记录于 RFC-0009 头部。agent 自主判档,判档争议向上取严。
- **RX7021 归属裁量(agent,本轮)**:RX7021(rurix_rt_cabi 库定位/构建编排失败)**并入 RXS-0195** 而非 RXS-0194——两码同属 driver link 段「.lib 接线失败」家族(与 RX7022 同实现点位、同 7xxx 工具链段),而 RXS-0194 为运行时 C ABI 语义边界(符号含义/句柄/销毁纪律),不掺构建系统关注点;工具链码收敛一条,内聚更高。
- **std::gpu 首期子集外(RD-026)**:AsyncBuffer 宿主面 / Event / 跨线程转移 / 多 stream 重叠编排 / 非 {f32,i32,u32} 元素 / `Result` 错误面 → 登记 RD-026,硬需求出现按 10 §3 判档;触及即停下标注「需升档」。
- **真 NVIDIA fatbinary 容器格式**:维持既有 defer;嵌入形态 = PTX 文本 + 裸 cubin 字节(RXS-0192,DeviceArtifactSet 语义 0-byte)。
- **跨包 `.rx` 依赖编译 / `rx new` 脚手架**:RXS-0196 仅同包同目录;包间复用留后续判档。
- **窗口 / 输入 / 事件循环进语言(D-130 / RD-027)**:present 面(RXS-0197~0199,MS1.2b)维持窗/泵/交换链在 C++ shim(D-130 0-byte);交互模式 → RD-027;「窗口/UI 框架进语言」或「通用异步宿主运行时」扩张诱惑 → 登记 SG-010 gating 而非提案。
- **多后端(D-008/SG-003)/ DXIL 图形路(RXS-0171 白名单)/ `#[export(c)]`(RD-009)**:均不触;本面为 FFI **导入**方向(消费手写 extern "C",RXS-0125 口径 0-byte)。触及即停下标注「需升档」。
- **UB 节禁区**:宿主编排失败面以**编译期诊断(§3)+ 运行期确定性诊断 + 终止 + poisoned 传播(RXS-0193)**定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5);RXS-0193 即「无 UB」的正面语义条款,其实现支撑为 RXS-0194 销毁纪律。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-14 | 新建 spec/host_orchestration.md(MS1.2 single-source 宿主 GPU 编排语义面起始文件,承 [RFC-0009](../rfcs/0009-host-gpu-orchestration.md) Agent Approved 2026-07-14,§9 八项裁决全锁):落带编号条款体 `### RXS-0189 ~ ### RXS-0196`(FLS 体例,按需分 Syntax/Legality/Dynamic Semantics/Implementation Requirements,严禁 UB 节)——RXS-0189 std::gpu 宿主编排类型面与 affine 语义(Context/Stream/Buffer/PinnedBuffer lang items 非 Copy affine,move/borrow 违例复用 RXS-0054/0057~0061,宿主 API 着色合法性 kernel/device 内使用 → RX3015)/ RXS-0190 宿主 GPU 方法签名与元素类型推断(编译器已知签名,T ∈ {f32,i32,u32},元素类型不可定型 → RX2010)/ RXS-0191 launch 宿主 lowering 与实参借用/marshalling(兑现 RXS-0074/0075 类型契约执行语义,kernel 引用编译期绑定同源 device 入口符号,ViewMut 位 &mut·View 位 & 调用期借用,实参子集外 → RX6024,🔒 slot+kinds marshalling 含义冻结非语言 ABI)/ RXS-0192 single-source device 产物嵌入与装载协商(PTX 必存 + 可选 sm cubin 描述表,复用 RXS-0150/0151/0076 协商与 RXS-0073/0082 编译期纪律,嵌入失败 → RX6025,工具链缺失嵌哨兵运行期确定性失败)/ RXS-0193 宿主运行期错误与 poisoned 语义(确定性诊断+终止,poisoned 对齐 RXS-0077,无 UB 无静默降级 P-01 正面语义条款)/ RXS-0194 rxrt C ABI 运行时边界(🔒 FFI:rxrt_* 符号面/u64 句柄/free 前 sync 销毁纪律,含义冻结·布局不冻结为语言 ABI RXS-0180 L3 口径,复用 RXS-0125 手写 extern "C" 口径,unsafe 集中 cabi + U25)/ RXS-0195 extern "C" 符号保名与 `#[link]` 接线(字面名链接 + .lib 追加,定位失败 RX7022;rurix_rt_cabi 定位/构建失败 RX7021 按内聚并入本条,§4 留痕)/ RXS-0196 out-of-line 模块文件加载(`mod name;` 同目录装配内联等价,缺失/循环 → RX1005)。**RXS-0197 ~ RXS-0199(present 宿主 typestate 面 / present backbuffer 借用缓冲与 blit 契约 / 宿主图像落盘桥)为本文件预留区间,仅登记说明不落裸条款头,条款体随 MS1.2b 实现 PR 同落**。§3 编译期新码 RX1005/RX2010/RX3015/RX6024/RX6025/RX7021/RX7022 spec 先行引用(registry 落码 + en/zh message-key 随 MS1.2 实现 commit 同 PR 落,真实可达不预造);运行期失败走确定性诊断+终止不占 RX 码。§4 升档/禁区留痕(RX7021 归属裁量 / RD-026 / fatbinary defer / 跨包 defer / D-130·RD-027·SG-010 / D-008·RXS-0171·RD-009 / UB 节禁区)。每条 ≥1 测试锚定(conformance/host_orch/* + rurixc/cabi 单测 + ci/host_orch_smoke.py)随 MS1.2 实现 commit 同 PR 落(条款 commit 先于实现 commit,硬规则 7;trace_matrix 184→192 全锚定;stable 快照同 PR 重 bless,RXS-0180 L2 加性演进) | **Full RFC**(RFC-0009) |
