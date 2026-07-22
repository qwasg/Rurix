# Rurix 黑洞渲染 Demo · 工程上下文交接文档

> **用途**:本文档为外部 AI(如 Claude/Kimi)提供 Rurix 工程的完整上下文,用于撰写"在 Rurix 上实现实时黑洞渲染 demo"的严谨提示词。
> **撰写时点**:2026-07-19
> **Rurix 版本**:v1.0.1-dist.2(stable channel latest,语言版号 v1.0.0,2026-07-14 发行)

---

## 0. 一句话任务定义

**在 Rurix v1.0 上实现一个科研级实时黑洞渲染 demo,通过 D3D12 桌面窗口呈现 1280×720 动态画面,要求严谨的重工美学与可追溯的物理推导。**

---

## 1. Rurix 是什么

**Rurix 是一门独立的、静态编译的 GPU 系统编程语言**——"GPU 系统编程的 Rust"。

- **形态**:单源双层模型,host 层与 kernel 子语言共享同一套类型系统、泛型、模块、const eval,通过函数着色(`fn`/`kernel fn`/`device fn`/`const fn`)+ 能力检查区分。
- **实现**:Rust 编写的 `rurixc`(约 4.2 万行),四层 IR(AST → HIR → TBIR → MIR),LLVM 22.1.x vendored 后端。
- **三后端**:NVPTX(主,`compute_89`)→ PTX;DXIL(G2 图形);SPIR-V(MB1 跨端)。
- **目标平台**:Windows 11 x64 + NVIDIA GPU(SM ≥ 8.9);D3D12 + Vulkan。
- **对标**:CUDA C++ / Mojo / Slang / Descend(不是 UE5 竞品,是 GPU 安全编程语言)。

---

## 2. 当前机器环境(已就绪)

| 项 | 状态 |
|---|---|
| GPU | **NVIDIA GeForce RTX 4070 Ti**(12GB,WDDM,基准硬件) |
| 驱动 | 620.02 / CUDA 13.2 |
| OS | Windows 11 x64 |
| Windows SDK | 10.0.26100.0 + 10.0.19041.0 |
| rurixc.exe | ✓ 已编译(debug + release) |
| rx.exe | ✓ 已编译(debug + release) |
| 工作目录 | `H:\rurix` |
| 编译器路径 | `H:\rurix\target\release\rx.exe` |
| D3D12 shim 源码 | `H:\rurix\src\rurix-d3d12\shim\rx_d3d12_shim.cpp` |

---

## 3. Rurix 设计哲学与硬限制(必须遵守)

### 3.1 14 条编号设计原则(摘要)

- **P-01 strict-only**:不存在静默降级,失败必为结构化错误。
- **P-02 GPU-first**:地址空间/执行层级/资源归属是类型一等公民。
- **P-05 显式优于隐式**:拷贝/同步/调度显式。
- **P-06 静态编译**:AOT,无 JIT 作为语言语义。
- **P-09 证据先于里程碑**:无 measured_local 证据不得宣布性能达标。
- **P-14 Windows/WDDM 一等环境**。

### 3.2 语言能力硬限制(设计上永不支持,无法绕过)

| 硬限制 | 对黑洞 demo 的影响 |
|---|---|
| **无 `dyn Trait` / 动态分发** | 用 enum dispatch 或静态单态化;黑洞 demo 程序化几何,不依赖多态,影响小 |
| **无 `async`/`await`** | host 编排走顺序 `stream.launch`;黑洞 demo 单 stream 即可 |
| **无 proc macro / 反射 / RTTI** | 无代码生成;黑洞 demo 不需要 |
| **无 `build.rs` / 编译期代码生成** | 无构建脚本;参数硬编码或 const fn |
| **`panic=abort`** | 崩溃即退出;需防御性编程 |

### 3.3 host 侧 stdlib 当前状态(重要)

> v1.0 stable 冻结的 stdlib spec 仅 **RXS-0104~0113 = core 数学库**。

| 类型 | 是否可用 |
|---|---|
| `Vec2`/`Vec3`/`Vec4`/`Mat2`/`Mat3`/`Mat4`(数学向量矩阵) | ✓ |
| `Point3`/`Vector3`/`Normal3`/`Aabb`/`Ray`(几何原语) | ✓ |
| `f32`/`f64`/`i32`/`u32`/`usize`/`bool` | ✓ |
| 固定大小数组 `[T; N]` | ✓ |
| `Result<T, E>` + `?` | ✓ |
| **动态数组 / `String` / `HashMap` / `BTreeMap`** | ✗ **未实现** |
| **文件 IO / 网络 / 线程 / 时间** | ✗ **未实现** |

**含义**:host 侧只能用 `[T; N]` 固定数组 + `extern "C"` FFI 调用 rurix-rt-cabi 的 C runtime。**黑洞 demo 所有参数必须 const 或硬编码,不能动态读取文件**。

### 3.4 数值纪律(科研严谨性)

> 摘自 `render_core.rx` 注释:"仅 + - * / 、比较、位运算 + `dmath::rx_sqrt`;pow/exp 一律不用;乘加显式分步(`let` 拆分)压低 FMA 收缩分歧面;f32→整数 cast 前一律 `rx_clamp`(NaN 安全)"。

- **无 `pow`/`exp`/`log`/`sin`/`cos`** —— 如需三角函数,用泰勒展开或查表(在 device fn 内手写)。
- **`rx_sqrt`**:Newton-Raphson 迭代,纯 f32 算术,不依赖 libdevice。
- **显式分步累加**:`let s = a + b; let t = s + c;` 而非 `a + b + c`,压低 FMA 分歧。

---

## 4. Rurix 语法速查(母本提炼)

### 4.1 函数着色(五种 fn)

```rust
fn main() { ... }              // host 入口
pub fn helper(x: f32) -> f32 { ... }   // host 普通函数
pub kernel fn rt_primary(t: ThreadCtx<2>, ...) { ... }  // GPU 入口
pub device fn dot3(ax: f32, ...) -> f32 { ... }   // GPU 标量 helper(kernel/plain fn 均可调)
pub const fn square(x: f32) -> f32 { x * x }  // 编译期求值(MIR 解释器)
```

### 4.2 GPU 类型一等公民

```rust
// 地址空间泛型(global/shared/constant/local/host)
View<global, f32>           // 只读 GPU buffer
ViewMut<global, f32>        // 独占可写 GPU buffer(backbuffer 用这个)
AtomicView<global, u32, (16,)>  // 原子视图,(16,) = bucket 数

// 资源生命周期(context-brand)
Context                     // GPU 上下文(创建设备/queue)
Stream<'ctx>                // FIFO 命令流
Buffer<C, T>                // 设备缓冲(C = context brand)
PinnedBuffer<C, T>          // 锁页缓冲(host↔device 传输)

// 线程上下文
ThreadCtx<1>                // 1D 网格,有 global_id() / thread_index() / block_dim()
ThreadCtx<2>                // 2D 网格,有 global_id_x() / global_id_y()
```

### 4.3 kernel 内 GPU 协作

```rust
pub kernel fn scan_block_u32(
    t: ThreadCtx<1>,
    flag: View<global, u32>,
    bofs: ViewMut<global, u32>,
    n: usize,
) {
    shared let buf: [u32; 256];        // shared memory(块内共享)
    let tid = t.thread_index();
    let i = t.global_id();
    let e = if i < n { flag[i] } else { 0u32 };
    buf[tid] = e;
    block.sync();                       // barrier(须 uniform control flow)
    // ... Blelloch scan
}
```

### 4.4 host↔kernel 编排

```rust
fn main() {
    let ctx = Context::create(...);
    let stream = Stream::new(&ctx);
    let buf: Buffer<C, f32> = Buffer::new(&ctx, n);
    
    // launch kernel:GridDim / BlockDim / args 元组
    stream.launch(my_kernel, GridDim(nblocks), BlockDim(256),
        (buf.view(), n, grid_dim));
    stream.sync();
}
```

### 4.5 Present typestate 帧循环(D3D12 桌面窗口)

```rust
// 创建窗口 + swapchain(render_w, render_h, window_w, window_h)
let sess = Present::create(&ctx, rw32, rh32, rw32, rh32);
let mut ready = sess.ready();
let mut running: i32 = 1;
let mut frames_done: usize = 0;

while running == 1 {
    // (1) 仿真或动画更新(可选)
    //     stream.launch(sim_kernel, ...);
    
    // (2) 获取 backbuffer 借用句柄
    let acq = ready.wait();
    let bb = acq.backbuffer();  // ViewMut<global, f32>,行主序紧密 f32 RGB,分量 0…255
    
    // (3) 渲染 kernel 直写 backbuffer
    stream.launch(render_kernel, GridDim(gx, gy), BlockDim(16, 16),
        (bb, rw, rh, /* 相机参数 */));
    
    // (4) 呈现
    let pres_ = acq.signal();
    let close = pres_.pump();      // 处理窗口消息(关窗请求等)
    ready = pres_.present();       // flip swapchain
    
    frames_done = frames_done + 1;
    if close { running = 0; }
    if frames_done >= MAX_FRAMES { running = 0; }
}
stream.sync();
```

### 4.6 backbuffer 像素契约(RXS-0143)

- 行主序紧密 `f32` RGB,每像素 3 个 f32。
- 分量域 `0…255`(注意不是 0…1)。
- kernel 内 `tone_map(c)` 到 `[0,1]` 后 `× 255.0` 写入。
- 写入:`bb[pi * 3] = r; bb[pi * 3 + 1] = g; bb[pi * 3 + 2] = b;`

### 4.7 关键 device fn 模式(标量进标量出)

```rust
pub device fn dot3(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> f32 {
    let x = ax * bx;
    let y = ay * by;
    let z = az * bz;
    let s = x + y;
    s + z
}

pub device fn len3(x: f32, y: f32, z: f32) -> f32 {
    dmath::rx_sqrt(dot3(x, y, z, x, y, z))
}

// Reinhard + sqrt tonemap(母本实现)
pub device fn tone_map(c: f32) -> f32 {
    let cc = dmath::rx_max(c, 0.0);
    let m = cc / (1.0 + cc);
    dmath::rx_sqrt(m)
}
```

### 4.8 dmath 模块可用函数

- `rx_sqrt(x: f32) -> f32`(Newton-Raphson,纯 f32)
- `rx_min(a, b)` / `rx_max(a, b)`
- `rx_clamp(v, lo, hi)`

### 4.9 模块组织

```
apps/blackhole/
├── rurix.toml          # 包清单
└── src/
    ├── realtime.rx     # 主入口(host main + present 帧循环)
    ├── render.rx       # kernel fn(黑洞 geodesic 积分 + 着色)
    ├── render_core.rx  # device fn(标量数学/几何/颜色)
    ├── dmath.rx        # device fn(rx_sqrt 等)
    ├── params.rx       # const 参数(度规/相机/吸积盘)
    └── starfield.rx    # device fn(背景星空程序化生成)
```

### 4.10 rurix.toml 包配置

```toml
[package]
name = "blackhole"
version = "0.1.0"
build = "declarative"
```

---

## 5. 母本代码完整片段(可直接参考)

### 5.1 ruridrop rt_primary kernel 签名(`apps/ruridrop/src/render_rt.rx:15-53`)

```rust
pub kernel fn rt_primary(
    t: ThreadCtx<2>,
    // ... 场景 buffer(View/ViewMut)...
    bb: ViewMut<global, f32>,    // backbuffer 借用句柄
    w: usize,
    h: usize,
    // ... 标量相机/光源参数 ...
) {
    let x = t.global_id_x();
    let y = t.global_id_y();
    if x >= w || y >= h {
        return;
    }
    let pi = y * w + x;
    // ... 像素中心光线计算 + 场景求交 + 着色 ...
    
    // 末尾:tone map → [0,1] → ×255 写 backbuffer
    bb[pi * 3] = render_core::tone_map(cr) * 255.0;
    bb[pi * 3 + 1] = render_core::tone_map(cg) * 255.0;
    bb[pi * 3 + 2] = render_core::tone_map(cb) * 255.0;
}
```

### 5.2 ruridrop present 帧循环(`apps/ruridrop/src/realtime.rx:378-482`)

完整骨架见第 4.5 节。关键点:
- `Present::create(&ctx, rw32, rh32, rw32, rh32)` 创建 1280×720 窗口
- 每帧:`ready.wait()` → `acq.backbuffer()` → `stream.launch(rt_primary, ...)` → `acq.signal()` → `pres_.pump()` → `pres_.present()`
- `MAX_FRAMES = 600` 帧上限
- `stream.sync()` 末尾同步

### 5.3 realtime.rx 系统库 FFI 接线(`apps/ruridrop/src/realtime.rx:24-36`)

```rust
extern "C" {
    fn putchar(c: i32) -> i32;
}

#[link(name = "user32")]
#[link(name = "d3d12")]
#[link(name = "dxgi")]
#[link(name = "d3dcompiler")]
extern "C" {
}
```

### 5.4 conformance 测试组织(accept/reject 双态)

```
conformance/
├── borrowck/
│   ├── accept/
│   └── reject/
│       └── use_after_move/
│           └── basic.rx     // 首行 //@ spec: RXS-0054 + //@ expect-error: RX4001
├── atomics/accept/scoped_atomics_ok.rx
└── ...
```

---

## 6. 运行与验收方式

### 6.1 编译运行命令

```bash
# 工作目录:H:\rurix
# 方式 A:直接 run(编译+运行)
H:\rurix\target\release\rx.exe run apps\blackhole\src\realtime.rx

# 方式 B:先 build 再跑
H:\rurix\target\release\rx.exe build apps\blackhole\src\realtime.rx
# 产物在 build/ 下,运行 .exe
```

### 6.2 验收标准(科研级 + 重工美学)

**功能验收**:
- `rx run` 在桌面打开 1280×720 D3D12 窗口
- 看到黑洞事件视界(黑色圆盘)+ 光子球光环 + 吸积盘 + 背景星光扭曲
- 相机缓慢轨道运动(动画)或固定视角
- 帧率 ≥ 30fps(1spp 实时档,RTX 4070 Ti 基准)
- 末帧采样核验打印 `REALTIME_OK frames=<n> sample_ok=true`

**科研严谨性验收**:
- 度规选择有出处(Kerr 或 Schwarzschild,引用论文名称)
- geodesic 积分方法明确(RK4 或 Verlet,步长策略可解释)
- 多普勒因子公式可追溯(δ = 1/[γ(1-β·n)])
- 引力红移公式可追溯(Schwarzschild: 1+z = 1/√(1-2M/r))
- 吸积盘参数有物理意义(内半径 = ISCO,温度分布符合 Shakura-Sunyaev 或简化)
- 代码注释引用具体论文/公式编号

**工程严谨性验收**:
- 数值纪律:仅 +-*/ + rx_sqrt,无 pow/exp(或自实现)
- 显式分步累加(压低 FMA 分歧)
- f32→整数 cast 前 clamp(NaN 安全)
- backbuffer 像素域 0…255,tone_map 后 ×255
- 边界处理:零向量归一化、光线步数上限、t 截断
- 模块组织:render_core device fn 标量化、kernel 内不写复杂逻辑

---

## 7. 黑洞渲染科研要求(供 Claude 联网搜索参考)

### 7.1 推荐搜索关键词(论文/资料)

- **"Kerr geodesic ray tracing"** — 克尔度规光线追踪
- **"Luminet 1979 black hole visualization"** — Luminet 1979 经典黑洞图像模拟
- **"Interstellar Gargantua Double Negative"** — 《星际穿越》Gargantua 视觉 Oliver James/DNEG 论文
- **"Raptor black hole renderer"** — Raptor RT 黑洞渲染器
- **"Doppler beaming accretion disk"** — 吸积盘多普勒束效应
- **"Shakura-Sunyaev disk model"** — 标准吸积盘模型
- **"photon sphere Schwarzschild Kerr"** — 光子球
- **"ISCO innermost stable circular orbit"** — 最内稳定圆轨道
- **"gravitational redshift formula"** — 引力红移公式
- **"Interstellar movie black hole physics paper"** — James et al. 2015b(Class. Quantum Grav.)

### 7.2 度规选择建议

**首选 Kerr 度规**(自旋黑洞,视觉上更像 Gargantua):
- 自旋参数 `a = J/M`(0 ≤ a ≤ M)
- 视界半径 `r+ = M + √(M² - a²)`
- 光子球半径依赖纬度(Kerr 光子轨道复杂)
- 能层(ergosphere)出现在旋转黑洞

**简化选项 Schwarzschild 度规**(无自旋,入门):
- 视界 `r_s = 2M`
- 光子球 `r_ph = 3M`(球对称,光线可在 1.5 倍视界处绕行)
- ISCO `r_isco = 6M`
- 引力红移 `1 + z = 1/√(1 - 2M/r)`

### 7.3 必须实现的物理效应清单

| 效应 | 描述 | 视觉表现 |
|---|---|---|
| **事件视界** | 光线无法逃逸的临界半径 | 中央黑色圆盘 |
| **光子球** | 光线绕黑洞圆周运行的半径 | 视界外的亮环 |
| **引力透镜** | 光线在引力场中弯曲 | 背景星光扭曲、吸积盘被"抬升"到黑洞上方 |
| **次像** | 光线绕黑洞多圈 | 吸积盘在黑洞上下方都可见 |
| **吸积盘** | 物质围绕黑洞旋转的盘 | 横向亮带 |
| **多普勒束效应** | 朝向观察者的一侧蓝移变亮 | 吸积盘一侧亮一侧暗 |
| **引力红移** | 从强引力区出来的光红移 | 吸积盘内圈偏红 |
| **相对论束流时间延迟** | 高速旋转的时间膨胀 | 旋转视觉变形 |
| **ISCO 截断** | 吸积盘内边界 = ISCO | 盘内圈清晰边界 |

### 7.4 数值方法

**geodesic 积分**(光线在弯曲时空中的传播):

```
对每像素:
  1. 从相机发射光线(初始 4-位置 x^μ + 4-动量 k^μ)
  2. 沿仿射参数 λ 积分 geodesic 方程:
     d²x^μ/dλ² + Γ^μ_αβ (dx^α/dλ)(dx^β/dλ) = 0
     (Christoffel 符号由度规 g_μν 计算)
  3. 用 RK4 或步长自适应 RK45 积分
  4. 终止条件:
     a. 光线越过视界(r < r+)→ 黑色
     b. 光线逃逸到远处(r > r_escape)→ 命中背景星空
     c. 光线穿过吸积盘平面(r_in < r < r_out 且 θ = π/2)→ 命中吸积盘
  5. 命中吸积盘 → 计算颜色(温度 + 多普勒 + 红移)
```

**多普勒因子**:
- `δ = 1 / [γ(1 - β·n)]`
- `γ = 1/√(1 - v²/c²)`(盘旋转速度)
- `β = v/c`(切向速度)
- `n` = 观察方向单位向量
- 观察亮度 `I_obs = δ³ · I_emit`(相对论束流)

**引力红移**(Schwarzschild):
- `1 + z = 1/√(1 - 2M/r_emit)`
- 观察频率 `ν_obs = ν_emit / (1 + z)`

### 7.5 吸积盘简化模型(Shakura-Sunyaev 启发)

- 内半径 `r_in = r_isco`(Schwarzschild: 6M;Kerr prograde: 1M~6M)
- 外半径 `r_out = 20M ~ 50M`
- 温度分布 `T(r) ∝ r^(-3/4)`(标准薄盘)
- 颜色映射:黑体辐射 → RGB(用简化的色温→RGB 表)
- 旋转:开普勒速度 `v_φ = √(M/r)`(Schwarzschild)

### 7.6 背景星空

- 程序化生成:固定 seed 的伪随机点光源分布
- 或简化:固定方向矢量数组 `[Ray; N]`(host 侧 const,无动态数组需求)
- 经引力透镜后位置扭曲(光线终点方向 → 星空查找)

### 7.7 相机

- 轨道相机:固定俯仰角,缓慢绕黑洞旋转(参数化 `angle = frame_count * delta`)
- 或静止相机:固定位置,吸积盘自转动画
- 视场角 FOV ~ 60-90 度
- 距离 `r_cam = 30M ~ 60M`(足够远看到全景)

### 7.8 视觉效果分层(供 Claude 撰写提示词时定目标)

**基础版**(1-2 小时,Schwarzschild):
- Schwarzschild 度规 + RK4 geodesic
- 简化吸积盘(纯黑体色温,无多普勒)
- 程序化星空背景
- Reinhard tonemap
- 1280×720 / 1spp 实时

**进阶版**(半天-1 天,Kerr):
- Kerr 度规 + RK4 自适应步长
- 多普勒束效应 + 引力红移
- Shakura-Sunyaev 温度分布
- 自旋参数可调
- ACES tonemap(自实现,无 pow/exp)

**科研版**(1-2 周):
- Kerr 度规精确数值积分
- 完整辐射转移(吸收 + 发射)
- 体积吸积盘(非薄盘)
- 光线多次散射
- 帧累积降噪(静态相机)

---

## 8. Rurix 工程纪律(可选,保留 strict-only 风格)

如果希望黑洞 demo 延续 Rurix 项目的工程严谨性:

- **失败测试先行**:先写 conformance reject 用例(如 `//@ expect-error: RX2001` 类型不匹配),再写实现
- **端到端 measured_local**:性能数据必须来自真实运行(`rx bench`),不能估计
- **不伪造归因**:bug 必须四层判别(rurixc IR / LLVM NVPTX / ptxas / 驱动 JIT),不甩锅
- **修订只追加**:close-out 后契约 0-byte 修改,新需求立 RD-### 编号
- **数值同义**:host refcpu 与 device kernel 数值逐位同义(可写 refcpu 验证档)
- **每 unsafe 块带 `// SAFETY:` 注释**(黑洞 demo 全 safe 即可,无 unsafe)

---

## 9. 现有可参考的母本代码完整路径

| 文件 | 用途 |
|---|---|
| `H:\rurix\apps\ruridrop\rurix.toml` | 包配置模板 |
| `H:\rurix\apps\ruridrop\src\realtime.rx` | 完整 present 帧循环母本(612 行) |
| `H:\rurix\apps\ruridrop\src\render_rt.rx` | 实时 kernel 母本(252 行,DDA + Lambert) |
| `H:\rurix\apps\ruridrop\src\render_pt.rx` | 离线 PT kernel 母本(540 行,DDA + NEE) |
| `H:\rurix\apps\ruridrop\src\render_core.rx` | device fn 共享核心(标量数学/几何/颜色) |
| `H:\rurix\apps\ruridrop\src\dmath.rx` | rx_sqrt/rx_min/rx_max/rx_clamp |
| `H:\rurix\apps\ruridrop\src\rng.rx` | xorshift32 + wang_hash |
| `H:\rurix\apps\ruridrop\src\params.rx` | const 参数定义模式 |
| `H:\rurix\apps\ruridrop\src\sim.rx` | kernel + shared let + block.sync 母本 |
| `H:\rurix\conformance\atomics\accept\scoped_atomics_ok.rx` | AtomicView 用法 |
| `H:\rurix\conformance\borrowck\reject\use_after_move\basic.rx` | reject 测试标记模式 |

**建议**:让 Claude/Kimi 先精读 `realtime.rx` + `render_rt.rx` + `render_core.rx`,理解母本结构,再撰写黑洞 demo 提示词。

---

## 10. 给 Claude 的撰写提示词任务说明

**你的任务**:基于本文档提供的 Rurix 工程上下文,撰写一份**详细、严谨、可一步最大实现**的提示词,让 Kimi-K3 能在 Rurix v1.0 上实现实时黑洞渲染 demo。

**提示词必须包含**:

1. **任务定义**:实时黑洞渲染 demo,1280×720 D3D12 桌面窗口呈现,科研级严谨
2. **Rurix 语法关键点**:从本文档第 4 节提炼,重点强调硬限制(无 dyn/async/反射/动态数组;数值纪律无 pow/exp)
3. **母本代码引用**:让 Kimi 先精读 `realtime.rx` + `render_rt.rx` + `render_core.rx`(给出绝对路径)
4. **黑洞物理推导**:从本文档第 7 节提炼,要求引用具体论文(让 Kimi 联网搜索 Kerr geodesic / Luminet 1979 / Interstellar Gargantua)
5. **实现步骤**:从复制 ruridrop 到 apps/blackhole 开始,逐步替换 kernel
6. **验收标准**:从本文档第 6.2 节提炼,科研严谨性 + 工程严谨性双重要求
7. **联网搜索指令**:明确要求 Kimi 联网搜索黑洞模拟论文,引用具体公式与参数
8. **重工美学要求**:代码注释引用论文章节、参数有物理意义、模块组织清晰、数值纪律严格
9. **运行命令**:给出 `rx run` 与编译路径

**提示词风格要求**:
- 严谨、结构化、可执行
- 每个技术要求都附上 Rurix 母本依据或论文出处
- 明确区分"必须实现"与"可选进阶"
- 给出失败模式预案(如 kernel 编译错误如何 debug)

---

## 11. 关键风险与缓解

| 风险 | 缓解 |
|---|---|
| Kimi-K3 不熟悉 Rurix 语法 | 喂入本文档第 4 节 + 母本代码路径 |
| geodesic 数值积分爆炸 | 步长自适应 + t 截断 + 视界内终止 |
| 1spp 实时噪声重 | 静态相机可累积;动态相机接受噪声或降分辨率 |
| 无 pow/exp | 用泰勒展开或查表;或避开需要这些的公式 |
| backbuffer 域 0…255 | tone_map 后必须 ×255 |
| host 无动态数组 | 星空背景用固定大小数组或 device 内程序化生成 |
| 编译错误难 debug | 用 `rx check` 先做前端检查;参考 conformance accept 用例 |

---

**文档结束。转给 Claude 时,请附上"基于此上下文 + 联网搜索黑洞论文,撰写给 Kimi-K3 的严谨提示词"指令。**
