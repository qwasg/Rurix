# G7 场景与能力快照冻结(G7.1 SubTask 2.2;门 G-G7-3)

本文冻结 G-G7-3 要求的「一个代表性 1080p 场景 + 固定相机 + TLAS 描述 + W1/W2/W3
capability snapshot」。全部数值来自真实命令输出或真实代码读取;无法实采项如实标注。

- 采集日期: 2026-08-01
- 采集机: NVIDIA GeForce RTX 4070 Ti,driver 620.02,VRAM 12282 MiB
  (`nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv` 实采)
- 采集命令(均在仓库根 `h:\rurix` 实跑,exit 0):
  1. `cargo run -q -p uc06-renderer --bin uc06-renderer -- --frames 2 --size 1920x1080 --json`
     (host 1080p 全管线核验)
  2. `RURIX_REQUIRE_REAL=1 cargo run -q -p uc06-renderer --bin uc06-renderer --features vulkan -- --device --frames 2 --size 64x64 --json`
     (DeviceCaps + W1/W2 device 对拍;能力面与分辨率无关,沿用 ci/uc06_renderer_smoke.py 步骤 2 口径)
  3. geom-build 簇规模一次性探针(复刻 scene.rs 三网格构造输入,连跑 3 次)

## 1. 代表性场景(冻结)

- 名称: uc06-renderer G5 合成场景(plane + sphere + cube 三网格;RFC-0016 §1 demo 场景)。
- 构造入口: `apps/uc06-renderer/src/scene.rs` `build_scene()`(L137–245)。全部几何为
  解析式生成器,同参数同输出,零外部资产、零随机(scene.rs L6–7 模块纪律)。
- 构造参数(scene.rs L144–148 硬编码):
  - `plane`: `TriMesh::plane_grid(4, 3.0)`,实例变换 `translation(0.0, 0.0, 0.0)`
  - `sphere`: `TriMesh::uv_sphere(0.42, 24, 16)`,实例变换 `translation(-0.65, 0.42, 0.1)`
  - `cube`: `TriMesh::cube(0.32)`,实例变换 `translation(0.75, 0.32, -0.35)`
- 几何规模(2026-08-01 探针实采,3 次运行):
  - 三角形: plane 32 / sphere 720 / cube 12,**合计 764**(3 跑不变,解析式守恒)。
  - 网格顶点: 25 / 362 / 8,合计 395。
  - 实例数 = 3(实例序 = 网格序,scene.rs L203–204 断言锚定);GpuScene 网格 = 3;材质 = 3
    (albedo 硬编码 scene.rs L171–187)。
  - 簇层级 DAG: plane 叶簇 1 / DAG records 6,cube 叶簇 1 / records 2(3 跑稳定);
    sphere 叶簇 12~13 / records 30~42(3 跑观测 42/34/30 波动)。原因: geom-build 簇化含
    HashMap 评分打平序依赖,同参数跨调用簇划分可变、三角形守恒不变(scene.rs L297–301
    测试注释钉死此性质)。
  - **冻结口径**: 三角形数/顶点数/实例数/实例变换/材质为不变锚;簇划分不冻结具体簇数,
    冻结「≤128 tri/簇 + 三角形守恒 + page_id = 网格 id」契约面(scene.rs L316 上限断言)。
- 场景 digest: 场景本体无 content-hash / seed 字段。GI/RT 效果种子 =
  `RenderConfig::seed = 0x5255_5258_5543_0006`(pipeline.rs L50,固定默认种子)。
  场景本体冻结引用 = 「以 `apps/uc06-renderer/src/scene.rs` 当前默认构造
  `build_scene()`(L137–245)为准」。
- 光照常量(GI/硬阴影/VSM 共用同一份,scene.rs L250–257):
  `SUN_DIR = [0.35, -0.85, 0.40]`、`SUN_COLOR = [6.0, 5.6, 5.0]`、
  `SKY_COLOR = [0.28, 0.34, 0.44]`、`VSM_LIGHT_DIR = SUN_DIR`。

## 2. 固定相机 + 1080p 分辨率(冻结)

- 相机常量 `CAMERA`(scene.rs L270–277,「确定不动」L259):
  - eye = `[0.0, 2.2, 3.4]`;center = `[0.0, 0.35, 0.0]`;up = `[0.0, 1.0, 0.0]`
  - fov_y = `FRAC_PI_3`(60°);z_near = 0.1;z_far = 60.0
- 矩阵装配 `camera_matrices(w, h)`(pipeline.rs L74–93): `look_at_rh` +
  `perspective_rh_zo(fov_y, aspect = w/h, z_near, z_far)`;剔除相机
  `CullCamera { error_threshold_px: 1.0 }`。
- 分辨率链: CLI `--size 1920x1080` → 输出 1920×1080;内部分辨率 = 输出/2 = **960×540**
  (`RenderConfig::internal_w/h`,pipeline.rs L55–62);TSR 2× 超分回 1920×1080。
  相机矩阵按**内部分辨率**装配(pipeline.rs L146),aspect = 16/9(内外一致)。
- 静态性: MV 恒零是静态收敛证据的一部分(scene.rs L259);帧间唯一亚像素扰动为 TAA/TSR
  jitter 序列(pipeline.rs L174),非相机运动。
- 1080p 实跑核验(2026-08-01,命令 1,host 全管线):
  `exit_ok = true`,9/9 asserts 全 true;frames=2,internal 960×540;
  graph pass_count=15 / barrier_count=32 / fence_count=2;
  final mean=0.217498 / std=0.084461;shadow_lit_ratio=0.9632;
  streaming loaded=3 / pop_in=0。

## 3. TLAS 描述(冻结)

- 构建位置: `apps/uc06-renderer/src/scene.rs` L207–231。
- 构建来源: 3 网格的**逐实例世界空间三角形**(`build_mesh` 预变换,scene.rs L115–118;
  RT/VSM 同源面)。每网格一份 `TriBvh` BLAS(`TriBvh::build`,scene.rs L220);3 份
  `InstanceDesc`(transform = IDENTITY——三角形已世界空间;实例掩码: inst0 地面 0xFE
  允许阴影光线排除自遮挡,inst1/2 球/cube 0xFF 恒可见,scene.rs L221–229)。
- TLAS = `Tlas::build(&descs, &blases)`(scene.rs L231),instance_count = 3、BLAS 池 = 3
  (单测锚定 scene.rs L306–307);`Vec<TriBvh>` 实现 `BlasSet`。
- 消费方(同一份 TLAS/几何):
  - RTAO + 硬阴影: `EffectInputs::new(.., &scene.tlas, &scene.blases)` → `rtao_pass` /
    `hard_shadow_pass`(pipeline.rs L388–396);
  - 硬阴影探针: `tlas.any_hit_with_mask(&scene.blases, .., 0xFE, ..)`(pipeline.rs L1212–1219);
  - GI tracer 同源几何(`gi_scene_of`,pipeline.rs L205–220);VSM 深度光栅同源
    `world_tris`(scene.rs L53)。
- AS 生命周期策略单源: `src/rurix-render/src/rt/as_manager.rs`(BlasKey = 网格内容
  FNV-1a 哈希缓存;TlasBuilder 每帧全量重建;refit/rebuild 决策树)。device 侧 AS 构建
  执行器 = `src/rurix-rt/src/vk.rs` `run_ray_tracing_offscreen`(G3.6,BLAS→barrier→TLAS
  两段构建)——G7.3 W3b「复用既有 BLAS/TLAS/AsManager 所有权」的复用对象即此二处,
  禁止第二套 BVH。

## 4. W1/W2/W3 capability snapshot(2026-08-01 实采)

采集路径: `render_exec::probe_device_caps()`(instance 级只读探测,不建 device;
`src/rurix-rt/src/render_exec.rs` L578)经命令 2 的 JSON `device` 字段输出,
`RURIX_REQUIRE_REAL=1`。

### 4.1 DeviceCaps 全字段(实采值)

| 字段 | 值 |
|---|---|
| device_name | NVIDIA GeForce RTX 4070 Ti |
| synchronization2 | true |
| shader_buffer_int64_atomics | true |
| shader_int64 | true |
| ray_query | true |
| acceleration_structure | true |
| buffer_device_address | true |
| descriptor_indexing | true |
| deferred_host_operations | true |
| max_push_constants_size | 256 |

### 4.2 require_wave 判定(声明序: render_exec.rs L470–482)

- **W1**(`["synchronization2"]`): **PASS**。实测 `wave_w1_pass = true`;四个 W1 内核
  device↔host 对拍全绿: cull 72/120 簇集合一致 / classify-resolve 9216 像素一致 /
  vsm_page_mark 4 页一致 / taa max_err = 1.2e-7 ≤ 1e-5。
- **W2**(`+ shader_buffer_int64_atomics`): **PASS**。实测 `wave_w2_pass = true`;
  visbuffer_sw_u64 9216 词 u64 逐位一致(容差 0)。
- **W3**(七项能力链 = W2 + ray_query / acceleration_structure / buffer_device_address /
  descriptor_indexing / deferred_host_operations): **七项实采全 true → 全绿,无缺项**。
  按 `require_wave(caps, KernelWave::W3)`(render_exec.rs L508)判定为 Ok,不发生
  `MissingCapabilities` fail-closed。注意边界: 本快照冻结的是**能力面事实**;W3 内核
  (gi_probe / rtao / hard_shadow)的 device 真跑属 G7.3/G7.4 在途工作,不在本次冻结范围。

### 4.3 同采其他 device 实测

triangle_pixels = 4096(64×64 全覆盖)、compute_write_ok = true、mixed_pass_ok = true;
`validation_clean = false`——本次采集未设 `RURIX_VK_VALIDATION=1`(该字段语义 =
validation 层是否开启,如实记录,非设备缺陷)。

## 5. 限制与诚实边界

1. 簇数不冻结: geom-build HashMap 打平序使 sphere 簇划分跨运行波动(§1),冻结口径为
   三角形守恒与契约上限,不是具体簇数。
2. 场景无 digest 字段: 冻结引用为 scene.rs 当前默认构造的精确代码位置;若 scene.rs
   构造参数变更,本文须同步复审。
3. capability snapshot 绑定本机(RTX 4070 Ti / driver 620.02);换机/换驱动须按 §4
   命令重采,不得外推。

## 6. 修订(G7.6 One True Device Frame;几何/相机/光向 0-byte)

**日期**:2026-08-04。**理由**:步骤 96 帧链消费内部/输出分辨率对与动态位姿口径;
§1~§4 几何规模、相机常量、光照常量、capability 快照**逐字节不动**。

- **分辨率对(已在 §2,本修订显式钉为帧链口径)**:内部分辨率 **960×540** → 输出
  **1920×1080**(TSR 2×2 超分)。帧链 VisBuffer/lighting 在内部分辨率运行;最终色在
  输出分辨率。
- **动态位姿口径**:冻结几何(plane/sphere/cube 三网格 + 764 三角形)不变;动态 =
  G6 物理位姿经 `PhysicsBridge::sync_frame` 写入实例 3×4(单向事实源)。相机 eye/center
  与光向常量维持 §2/§1 冻结值(帧链不做相机运动;MV 来自实例运动)。
