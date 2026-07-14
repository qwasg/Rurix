# RFC-0009 — single-source 宿主 GPU 编排 stdlib（std::gpu）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0009（4 位制,编号永不复用,10 §9.5） |
| 标题 | host `.rx` 的 single-source 宿主 GPU 编排:std::gpu 首期收敛子集(Context/Stream/Buffer/launch)+ rurix-rt C ABI 绑定 + 同源 PTX 嵌入,附 present typestate 面与宿主图像落盘桥 |
| 档位 | **Full RFC**(10 §3:**运行时语义**(launch/传输/销毁的执行语义进语言面)+ **FFI ABI**(宿主 codegen ↔ 运行时 C ABI 边界);AGENTS 硬规则 5) |
| 状态 | Agent Approved(2026-07-14)。agent 自主批准后可推进下游实现 PR |
| 承接里程碑 | MS1.2 / MS1.2b(milestones/ms1/MS1_CONTRACT.md,验收门 G-MS1-1 / G-MS1-2) |
| 关联条款 | 拟落 spec **RXS-0189~0199**;新建 `spec/host_orchestration.md`(见 §5) |
| 依据决策 | D-406 v2.0(agent 完全自主)· D-230~D-234(Driver API 薄层运行时/销毁纪律/poisoned)· D-130(present shim 边界,0-byte 维持)· D-113(FFI 战略;本 RFC 为**导入**方向,不触 `#[export(c)]`/RD-009)· D-205(LLVM pin)· D-209(link.exe)· 01 §4 §6 / 05 §1 / 08 §1 §2 / 10 §3 |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 自主决策,批准后推进下游实现 |
| Agent 批准 | Approved — 2026-07-14;批准范围含 🔒 §4.3(C ABI 边界)/ §4.4(launch marshalling);记录于本文件与 MS1_CONTRACT §7 |

---

## 1. 摘要

让 host `.rx` 代码直接编排 GPU:同一个 `.rx` 编译单元里写 `kernel fn` 与 host `fn main`,host 侧经 `std::gpu` 首期收敛子集(`Context`/`Stream`/`Buffer`/`PinnedBuffer` + `launch`/同步/传输)创建上下文、搬运数据、发射**同源 kernel**;`rx build` 一步产出单 EXE——device 段照既有 device 路编为 PTX(+可选 sm_89 cubin),以 G1.5 `DeviceArtifactSet` 形态嵌入 host 产物,运行期经既有装载协商(RXS-0150/0151/0076)加载。宿主调用经新薄 C ABI 层 `src/rurix-rt-cabi`(staticlib,`rxrt_*`/`rxp_*`/`rxio_*` 符号)绑定 rurix-rt。附带三件配套:present 宿主 typestate 面(`.rx` 侧 affine 消费式帧状态机,窗/泵/交换链维持 C++ shim,D-130 0-byte)、宿主图像落盘桥(`write_ppm`,RXS-0114~0117 语义)、前端机械面(extern "C" 符号保名 + `#[link]` 接线 + `mod name;` out-of-line 模块)。

```
single .rx ──rurixc──┬─ host 路:MIR→LLVM→link.exe ── EXE(嵌 @__rx_gpu_artifacts)
                     └─ device 路:kernel→NVPTX→PTX(+cubin)──┘
EXE ──rxrt_*(rurix-rt-cabi staticlib)── rurix-rt(pipeline/fatbin/interop)── nvcuda / D3D12 shim
```

## 2. 动机

使命判据(01 §6 第三层 / 11 §6)要求「生产级渲染器/仿真系统以 Rurix 为主语言(不是 bolt-on)」;01 §4 五年图景第 1 条进一步要求「kernel 与 host 调度代码在同一语言里……没有一行不可诊断的胶水代码」。用户 2026-07-14 裁定按**最严口径**操作化:宿主编排也须 `.rx`。

现状实证:kernel 已全 `.rx`(`src/rurix-rt/kernels/*.rx` → PTX),但宿主编排一律 Rust——`Stream::launch` 仅有**类型面契约**(RXS-0074/0075,`launch_check.rs` 四契约 + `tests/ui/launch/*` 语料),`mir_build` 遇 launch 报 unsupported,无执行语义、无运行时绑定;host `.rx` 唯一 builtin 是 `println`(hir.rs `Builtin::Println` → CRT `puts`)。所有既有集成(uc03/uc04/rurix-engine/GRX)都是宿主承载 + Rurix 当 compute pass = bolt-on。本 RFC 把 launch 类型面**兑现为执行语义**,是 MS1 关键路径与 UC-07 应用(RFC-0010)的前置。

**为何需要 Full RFC(而非 Direct/Mini)**:① 运行时语义——launch 表达式、H2D/D2H 传输、affine 资源 drop 的**执行语义**首次进入语言面;② FFI ABI——宿主 codegen 与运行时之间的 C ABI 边界(符号集/句柄表/marshalling 布局)是新 FFI ABI 面(硬规则 5)。判档争议向上取严(硬规则 8)。

## 3. 指导级解释(用户视角)

一个文件,从数据到 GPU 再回来:

```rx
kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>,
                a: f32, n: usize, t: ThreadCtx<1>) {
    let i = t.global_id();
    if i < n { out[i] = a * x[i] + out[i]; }
}

fn main() -> i32 {
    let ctx = Context::create();               // affine 根;失败 = 确定性诊断 + 终止
    let stream = ctx.stream();
    let n: usize = 1048576;

    let mut host = ctx.alloc_pinned(n);        // PinnedBuffer<C, f32>(元素类型经使用点推断)
    let mut i: usize = 0;
    while i < n { host.set(i, 1.0); i = i + 1; }

    let mut x = ctx.alloc(n);                  // Buffer<C, f32>
    let mut out = ctx.alloc(n);
    x.upload(&host);
    out.upload(&host);

    stream.launch(saxpy, GridDim(n / 256), BlockDim(256), (out, x, 2.0, n));
    stream.sync();

    out.download(&mut host);
    if host.get(0) != 3.0 { return 1; }
    0                                          // drop 序:buffer → stream → ctx(RXS-0193)
}
```

`rx build app.rx` → 单 EXE。错误照旧是编译期的:move 后再用 Buffer(RX4xxx)、launch 实参失配(RX2001)、host 直调 kernel(RX3001)、gpu 宿主 API 写进 kernel(新 RX3015)——全部编译期拦截;运行期 CUDA 失败产确定性诊断后终止,无静默降级(P-01)。present 帧循环与 PPM 落盘同样是 `.rx` 面(§4.6/§4.7),ruridrop(RFC-0010)整个应用层零 `.rs`。

## 4. 参考级设计

### 4.1 std::gpu 类型面(首期收敛子集,RXS-0189/0190)

- 类型即既有 lang items(resolve.rs `LangItems`:`Context`/`Stream`/`Buffer`/`GridDim`/`BlockDim`,可被用户遮蔽的语义不变),新增 lang item `PinnedBuffer`。全部为**非 Copy affine** 句柄结构(编译器合成布局:`handle: u64` + brand 幽灵参数);move/borrow 违例复用既有 RX4xxx 规则,**零新借用码**。
- 方法集(typeck 编译器已知签名分支,先例 `Stream::launch`/ThreadCtx/Atomic 分支):`Context::create() -> Context`、`ctx.stream() -> Stream<C>`、`ctx.alloc(n: usize) -> Buffer<C,T>`、`ctx.alloc_pinned(n) -> PinnedBuffer<C,T>`、`ctx.sync()`、`buf.upload(&PinnedBuffer<C,T>)`/`buf.download(&mut PinnedBuffer<C,T>)`/`buf.len()`、`pinned.get(i)->T`/`pinned.set(i,v)`/`pinned.len()`、`stream.launch(kernel, GridDim(..), BlockDim(..), (args..))`(既有类型契约 RXS-0074/0075 原样生效)、`stream.sync()`。元素类型 `T` 经推断合一;不可推断 → **RX2010**。首期 `T` ∈ {f32, i32, u32}。
- **着色合法性**:gpu 宿主 API 仅 host 着色上下文合法;出现在 `kernel`/`device fn` → **RX3015**(coloring 层,与 RX3001 同点位)。
- **brand**:`Context::create()` 每个调用点合成新鲜 opaque brand 类型;`Stream<C>`/`Buffer<C,T>` 泛型签名契约(RX3006)原样生效。若调用点合成在推断上不可行,降级方案 = 单 brand + cabi 运行期 context-id 校验(§9 Q-Brand)。
- **错误面**:首期方法不返回 `Result`(enum 变体构造 codegen 未通,mir_build 既有限制);运行期失败语义见 §4.5。

### 4.2 lowering 与链接(RXS-0191/0195)

- **mir_build 直降字面符号**:gpu 方法/launch 降级为 `CallTarget::Fn { symbol: "rxrt_*" }`(不走 `mangle()`);host codegen 既有 extern declare 机制零改动发射 `declare`。**不走 prelude 源注入**(遮蔽破坏 lang item 判定 + 破 span,§7 否决)。
- **extern "C" 符号保名 + `#[link]` 接线**(RXS-0195,独立前端机械价值):extern 块内无 body fn 以**字面名**参与 codegen/链接(修缮 mir_build 对 extern fn 的 mangle);`#[link(name = "x")]` 在 driver link 段追加 `x.lib`,定位失败 → **RX7022**。
- **链接段**(driver.rs 既有 link.exe 命令,D-209):当编译单元实际使用 gpu/present/imageio 宿主 API 时,追加 `rurix_rt_cabi.lib` + Rust staticlib 所需系统库固定集(以 `--print native-static-libs` 实测 pin);.lib 定位序 = `RURIX_RT_CABI_LIB` env → rx.exe 旁 `lib/` → workspace `target/release/`(缺库时 `rx build` 编排 `cargo build -p rurix-rt-cabi --release`,先例 `build_pyd()`);定位/构建失败 → **RX7021**。host-only 程序链接线零漂移。
- **`mod name;` out-of-line 模块**(RXS-0196):parser 接受无花括号形态;driver 在 parse 后按「同目录 `name.rx`」装配为内联模块等价形态(SourceMap 多文件已支持);缺失/循环 → **RX1005**。resolve/typeck 零改动。跨包 `.rx` 依赖编译不在本期(§8)。

### 4.3 🔒 rxrt C ABI 运行时边界(RXS-0194;FFI ABI,agent 亲笔)

新 crate `src/rurix-rt-cabi`(`crate-type = ["staticlib"]`,先例 rurix-interop PYD 链路):`#[unsafe(no_mangle)] extern "C"` 符号面,u64 句柄表 + 状态断言,内部包 rurix-rt `pipeline.rs` ownership 系(`SharedContext`/`DeviceBox`/`SharedStream`)+ `fatbin.rs` 装载协商 + `interop.rs` 帧机。

```
rxrt_ctx_create(artifacts: *const u8) -> u64      // 0 = 失败;artifacts → §4.4 嵌入描述表
rxrt_ctx_destroy(ctx: u64)                        // 先 sync 再销毁(D-231 镜像)
rxrt_ctx_sync(ctx: u64) -> i32
rxrt_stream_create(ctx: u64) -> u64 / rxrt_stream_destroy(s) / rxrt_stream_sync(s) -> i32
rxrt_buf_alloc(ctx: u64, bytes: u64) -> u64 / rxrt_buf_free(b)   // free 前 ctx sync(防 in-flight UAF)
rxrt_buf_upload(b: u64, src: *const u8, bytes: u64) -> i32
rxrt_buf_download(b: u64, dst: *mut u8, bytes: u64) -> i32
rxrt_pinned_alloc(ctx: u64, bytes: u64) -> u64 / rxrt_pinned_ptr(p) -> *mut u8 / rxrt_pinned_free(p)
rxrt_launch(s: u64, entry: *const u8, gx,gy,gz,bx,by,bz: u32,
            slots: *const u64, kinds: *const u8, n_args: u64) -> i32
rxp_create(ctx,rw,rh,ww,wh) -> u64 / rxp_wait / rxp_backbuffer / rxp_signal / rxp_pump / rxp_present / rxp_destroy
rxio_write_ppm(path: *const u8, w: u32, h: u32, data: *const f32, n: u64) -> i32
```

- 符号集**含义冻结、布局不冻结为语言 ABI**(RXS-0180 L3 口径,对齐 RXS-0162/0165 先例):`rxrt_*` 面是工具链内部实现要求(RXS-0194 Implementation Requirements),非用户 stable ABI;用户面是 §4.1 的类型/方法语义。
- unsafe 全部集中于本 crate:逐处 `// SAFETY:` + unsafe-audit **U25** 登记;全仓其余 crate `unsafe_code=deny` 维持。
- 销毁纪律:`rxrt_buf_free` 先 `cuCtxSynchronize` 再 `cuMemFree`(镜像 `Context::drop` D-231);upload/download 走同 stream 序或 ctx sync——affine 检查覆盖不了的异步窗口由 cabi 纪律封口(§4.5 无 UB 承诺的实现支撑)。

### 4.4 🔒 launch 动态语义与实参 marshalling(RXS-0191/0192;FFI ABI,agent 亲笔)

- **kernel 引用编译期绑定**:`stream.launch(saxpy, ...)` 的 `saxpy` 须为同编译单元 `kernel fn`(launch_check 既有裁决);mir_build 以 device MIR 同源 `mangle(name, def, &[])` 符号作 NUL 字符串常量喂 `rxrt_launch`(单一事实源,与 device 入口名一致)。kernel 非 host 值,不引入函数指针型。
- **marshalling**:实参元组物化为栈上 `[u64; n]` slot 数组 + `kinds` 字节数组(0 = Buffer 句柄 → cabi 换设备指针;1 = 标量按位样式存 slot 低位,cuLaunchKernel 按形参尺寸读前 4/8 字节,little-endian);cabi 内部构造 `kernelParams` 指针数组指向各 slot(物化纪律复刻 `interop.rs AcquiredFrame::launch` 已验证代码)。**首期实参子集 = Buffer<C,T> + {i32, u32, f32, usize}**;子集外 → **RX6024**(编译期,镜像 RX6003/6005 子集纪律)。
- **借用语义**:launch 对 `ViewMut` 位 Buffer 实参取 `&mut` 调用期短借用、`View` 位取 `&`;借用于调用表达式期间存续,重叠冲突由既有 NLL 裁决。launch 异步返回后的 device 侧使用窗口不由借用面表达——由 §4.3 销毁纪律(free 前 sync)保证无 UAF,语义封口于 RXS-0193(无 UB)。
- **single-source 嵌入**(RXS-0192):host 编译若发现 `kernel fn`,先走既有 device 路(`device_codegen::build_and_emit` + `ir_to_ptx` + ptxas 干验证 RXS-0073/libdevice RXS-0082 纪律)产 PTX,`ptxas` 在位时再 `compile_cubin(sm_89)`;codegen 发射 `@__rx_gpu_ptx`/`@__rx_gpu_cubin_sm89`/`@__rx_gpu_artifacts` 私有常量描述表;`Context::create()` 降级为 `rxrt_ctx_create(ptr @__rx_gpu_artifacts)`——**注册即传参**,不玩链接段魔法,可复现构建(RXS-0097)不受影响。运行期 cabi 组 `DeviceArtifactSet`(PTX fallback 必存 + sm 键 cubin)按 RXS-0150/0151 协商装载,module/CUfunction 按 entry 惰性缓存。嵌入阶段失败 → **RX6025**;工具链缺失按既有 SKIP 纪律嵌哨兵,运行期 launch 走确定性失败(§4.5)。

### 4.5 运行期错误与 poisoned(RXS-0193)

任何 `rxrt_*` 运行期失败(CUDA 错误/装载协商拒/哨兵产物):cabi 落 stderr 确定性诊断(操作名 + CUresult/原因 + context 序号)后进程终止(abort);poisoned context 后全部 gpu 操作确定性失败(对齐 RXS-0077),**不产生 UB、无静默降级**(P-01 strict-only)。`Result` 错误面为后续加性扩展(依赖 enum codegen,§8)。

### 4.6 present 宿主 typestate 面(RXS-0197/0198;D-130 0-byte)

- rurix-rt `interop.rs` 重构出**非闭包持有形态** `OwnedPresentSession`(收拢 `scope()` 的建链逻辑;既有 `scope()`/brand API 与 uc03 **0-byte 零漂移**);fence 偶/奇协议(acquire 2n / cuda_done 2n+1 / d3d_done 2n+2)单一事实源留 interop.rs,cabi `rxp_*` 只做状态断言转发。
- `.rx` 面 affine 消费式(镜像 RXS-0142,错序 = 既有 RX4xxx move 违例,编译期拦截):

```rx
let sess = Present::create(&ctx, 1280, 720, 1280, 720);
let mut ready = sess.ready();
loop {
    let mut acq = ready.wait();               // Ready → Acquired(消费 self)
    let bb = acq.backbuffer();                // 借用 Buffer<C, f32>(Drop 不释放,RXS-0198)
    stream.launch(blit_rgb, GridDim(n/256), BlockDim(256), (bb, fb, n));
    let pres = acq.signal();                  // Acquired → Presentable
    let close = pres.pump();
    ready = pres.present();                   // → Ready(帧 +1)
    if close { break; }
}
```

- `backbuffer()` 仅 Acquired 态可得,产**借用句柄** Buffer(owned=false,free no-op);内容布局对齐 present pass 读取约定(共享 f32 RGB,RXS-0143/RXS-0121 语义)。窗/泵/交换链/固定 present pass 全在 `rurix-d3d12/shim` **不动**(D-130)。

### 4.7 宿主图像落盘桥(RXS-0199)

`Image::write_ppm(path: &str, w: u32, h: u32, data: &PinnedBuffer<C, f32>)`(或等价自由函数面,实现 PR 定形)→ `rxio_write_ppm` → 桥 `image-io` crate 既有确定性 PPM 序列化(RXS-0114~0117 语义 0-byte 复用);量化口径对齐 `sr_quantize`(RXS-0116)。失败走 §4.5 确定性诊断。这补齐离线渲染「出图落盘」的最后一块 `.rx` 面(G5 缺口),使 UC-07 离线入口零 `.rs`。

## 5. 下游 spec 条款映射(spec diff,10 §3 要件)

新建 `spec/host_orchestration.md`,自 **RXS-0189** 起续号(当前最高 RXS-0188;0181~0184 GRX 分支占用维持跳号)。MS1.2 落 0189~0196,MS1.2b 落 0197~0199;各与 ≥1 测试锚定同 PR(硬规则 7,trace_matrix 维持全锚定),stable 快照两次加性重 bless(184→192→195,RXS-0180 L2)。

| 条款(拟) | 标题 | 测试锚定计划(每条 ≥1) |
|---|---|---|
| RXS-0189 | std::gpu 宿主编排类型面与 affine 语义(含宿主 API 着色合法性 RX3015) | conformance/host_orch/accept 单源语料 + reject(move 后再用 / device 上下文误用)UI 语料 |
| RXS-0190 | 宿主 GPU 方法签名与元素类型推断(RX2010) | conformance/host_orch/reject/elem_infer + typeck 单测 |
| RXS-0191 | launch 宿主 lowering 与实参借用/marshalling(RX6024) | conformance/host_orch/accept/saxpy_single_source.rx + reject/arg_subset |
| RXS-0192 | single-source device 产物嵌入与装载协商(RX6025;复用 RXS-0150/0151/0076/0073/0082) | ci/host_orch_smoke.py 篡改 PTX 红绿 + driver 单测 |
| RXS-0193 | 宿主运行期错误与 poisoned 语义(无 UB,对齐 RXS-0077) | cabi 单测(错序/失败注入确定性诊断)+ smoke 见证行 |
| RXS-0194 | rxrt C ABI 运行时边界(🔒 FFI;含义冻结、非语言 ABI,RXS-0180 L3 口径) | src/rurix-rt-cabi 单测锚定 |
| RXS-0195 | extern "C" 符号保名与 `#[link]` 接线(RX7022) | conformance/host_orch/link 语料 + driver 单测 |
| RXS-0196 | out-of-line 模块文件加载(`mod name;`,RX1005) | conformance/host_orch/mod_file accept/reject |
| RXS-0197 | present 宿主 typestate 面(镜像 RXS-0142;D-130 边界) | conformance/host_orch/present 错序 reject(编译期 move 违例)+ accept 帧循环语料 |
| RXS-0198 | present backbuffer 借用缓冲与 blit 契约(对齐 RXS-0143) | 同上 accept 语料 + interop 单测 |
| RXS-0199 | 宿主图像落盘桥(RXS-0114~0117 语义复用) | conformance/host_orch/imageio 语料 + cabi 单测(与 image-io 输出逐字节一致) |

- **错误码策略**:编译期新码按真实可达类别分配——RX1005(1xxx 模块装配)/ RX2010(2xxx 元素推断)/ RX3015(3xxx 着色)/ RX6024/RX6025(6xxx codegen)/ RX7021/RX7022(7xxx 工具链);运行期失败走 cabi 确定性诊断 + 终止,**不占 RX 码**(工具层口径,对齐 spec/release.md §3)。registry/error_codes.json 只追加 + en/zh message-key 成对(bilingual 88→95;以实现实际可达为准,不预留)。

## 6. feature gate / tracking / 实现序(10 §3 要件)

- **gate 形态 = 无独立 feature gate(加性 stdlib 面)**:与 RXS-0153~0156(着色阶段类型面)/RXS-0185~0188(工具面)同口径——新面为 edition 2026 内**加性演进**(RXS-0180 L2 只增不破坏),不 gate 即不 bifurcate 编译器;误用全部 strict-only 编译期拒(RX3015/RX2010/RX6024 等),不完整路径无静默出口。稳定化即随快照重 bless 进入 stable 面(§10)。
- **失败测试先行**:本 RFC 提案时点,`ci/host_orch_smoke.py`、`spec/host_orchestration.md`、`src/rurix-rt-cabi`、std::gpu lowering 在 `main` 上**均不存在** = RED;实现 PR 落地后转绿。
- **栈式实现序**(均门控于本 RFC 合入后;条款 commit 先于实现 commit,硬规则 7):
  1. **MS1.2 单 PR**(条款/实现/重 bless/步骤 52 不可分,步骤 49 硬红):spec RXS-0189~0196 → 前端机械(extern 保名 + `#[link]` + `mod name;`)→ `src/rurix-rt-cabi`(rxrt_*,U25)→ rurixc lowering(typeck 分支/mir_build 降级/driver 嵌入+链接)→ conformance/host_orch → 错误码 en/zh → 快照重 bless 184→192 + RD-026 登记 → CI 步骤 52 + counter + schema + trace 再生。
  2. **MS1.2b 单 PR**:spec RXS-0197~0199 → `OwnedPresentSession` 重构 + `rxp_*`/`rxio_*` + `.rx` typestate 面 → 锚定 → 快照重 bless 192→195 → uc03 回归网复核。
- **真实红绿**(反 YAML-only):步骤 52 内建——篡改嵌入 PTX 字节 → 装载协商拒(RXS-0192)红;桩化 device 写回 → 数值对照红;复原 → 绿;run URL 归档 MS1_CONTRACT §8。

## 7. 备选方案

- **维持 Rust 宿主(现状)**:被使命判据最严口径直接否决——宿主编排留在 Rust 即 bolt-on。
- **prelude 源注入脱糖**(把 gpu 方法拼为用户可见 extern 块注入源):否决——`Buffer`/`Stream` 用户同名定义会破坏 lang item 判定与 launch_check 契约;拼接破坏 span/诊断质量。
- **直接把 rurix-rt(rlib)挂链接**:否决——Rust 符号 mangled、ABI 不稳定,link.exe 无法消费;必须有 extern "C" 面,该面即 rurix-rt-cabi。
- **cdylib 动态绑定为主方案**:降为 Q-Link 降级预案——staticlib 单 EXE 分发面更干净(fatbin/单产物精神);若 CRT 静态合并翻车再切 cdylib+旁置 DLL。
- **`#[export(c)]` 路线**:方向相反(导出 vs 导入),且 RD-009 defer 维持;本 RFC 消费手写 extern "C"(RXS-0125 口径 0-byte)。
- **在 `.rx` 里重述 present fence 细粒度 wait/signal**:否决——把 external-semaphore unsafe 面二次实现在可见层,违反单一事实源并扩大 unsafe-audit 面;typestate 下沉 C ABI 后。

## 8. 不做(范围红线)

- **std::gpu 首期子集外**:AsyncBuffer 宿主面、Event、跨线程转移、多 stream 重叠编排、非 {f32,i32,u32} 元素、`Result` 错误面 → 登记 **RD-026**,硬需求出现按 10 §3 判档。
- **真 NVIDIA fatbinary 容器格式**:维持既有 defer;嵌入形态为 PTX 文本 + 裸 cubin 字节(DeviceArtifactSet 语义 0-byte)。
- **跨包 `.rx` 依赖编译 / `rx new` 脚手架**:`mod name;` 仅同包同目录;包间复用留后续判档。
- **窗口/输入/事件循环进语言**:D-130 维持;交互模式 → RD-027(RFC-0010 §8)。若出现「窗口/UI 框架进语言」或「通用异步宿主运行时」扩张诱惑 → 登记 SG-010 gating 而非提案。
- **多后端**(D-008/SG-003 红线 3)/ **DXIL 图形路**(RXS-0171 白名单不动)/ **`#[export(c)]`**(RD-009)均不触。

## 9. 未决问题 / 关键裁决(agent 自主签署)

| # | 裁决点 | 裁决(2026-07-14) |
|---|---|---|
| Q-Surface | std::gpu 首期子集边界 | Context/Stream/Buffer/PinnedBuffer/launch/sync/upload/download/get/set/len;元素 {f32,i32,u32};其余 → RD-026 |
| Q-Link | 宿主 EXE ↔ rurix-rt 绑定 | 新 crate rurix-rt-cabi(staticlib)+ link.exe 静态链 + 库集实测 pin;降级预案 cdylib+旁置 DLL(实现 PR 若触发,回本 RFC 修订留痕) |
| Q-Marshal | launch marshalling ABI | slot[u64]+kinds 字节数组;实参子集锁死(RX6024);物化纪律复刻 interop.rs;🔒 §4.4 |
| Q-Embed | device 产物嵌入形态 | PTX 文本(NUL 终止)+ 可选 sm_89 cubin + 描述表常量;注册即传参;fatbin 真容器不进首期 |
| Q-Affine | 宿主 affine 资源面 | 复用 RXS-0130~0134 语义 + 既有 RX4xxx 借用规则,零新借用码;drop 序 = 声明逆序,RXS-0193 |
| Q-Brand | brand 生成形态 | 首选调用点合成新鲜 brand;推断不可行 → 单 brand + cabi 运行期 context-id 校验(RX3006 保泛型签名面);实现 PR 定案回填本行 |
| Q-Err | 失败模式载体 | 运行期 = 确定性诊断 + 终止(无 Result,无新运行期 RX 码);编译期按 §5 策略落码 |
| Q-Present | present 面归属 | 进本 RFC(RXS-0197~0199)但**不进 std::gpu 核心**:独立 Present typestate lang items,fence 协议单一事实源留 interop.rs,D-130 0-byte |

## 10. 稳定化与 provenance

- **稳定化**(10 §5):无独立 gate(§6 加性面口径);条款随两次快照重 bless 进入 stable 面(RXS-0180 L2 同 edition 只增不破坏);MS1 close-out 即首个「里程碑无重大修订」观察点,后续按 10 §5 常规通道。FCP-lite:本 RFC 为 Full RFC 触发面,合入即在 FCP-lite #121 开放通道下公开(advisory,通告即推进,agent 自主裁决合入,10 §2.2)。
- **Provenance**:`Assisted-by: claude-code:claude-fable-5`;agent 自主批准并记录(D-406 v2.0)。

## 11. 规范与实现依据

- 仓内:spec/device.md RXS-0066~0082 / spec/pipeline.md RXS-0130~0134 / spec/interop.md RXS-0125 / spec/interop_d3d12.md RXS-0140~0143 / spec/imageio.md RXS-0114~0117 / spec/release.md RXS-0150~0152 / spec/edition.md RXS-0180;src/rurixc/src/{driver,mir_build,typeck,resolve,launch_check,device_codegen,codegen}.rs;src/rurix-rt/src/{pipeline,fatbin,interop}.rs;src/rurix-interop/src/ffi.rs(staticlib C ABI 先例);src/uc03-demo/src/present.rs(帧循环母本)。
- 外部:CUDA Driver API(cuLaunchKernel kernelParams 约定/cuModuleLoadData);MSVC link.exe 静态库链接约定;NVIDIA PTX ISA(版号梯子 RXS-0076)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-07-14 | AI 起草初版(MS1.1;设计承 Phase-2 三方案勘察:rurixc 编译/链接流水、launch 类型面现状、rurix-rt ownership 系与 interop 帧机实证) | Full RFC(Draft) |
| Agent approval | 2026-07-14 | agent 自主批准全文(含 🔒 §4.3/§4.4 FFI ABI 面与 §9 八项裁决)并记录;批准后推进 MS1.2/MS1.2b 实现 PR | Full RFC(Agent Approved) |
