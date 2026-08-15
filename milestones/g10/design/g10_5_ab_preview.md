# G10.5a 双端出图 A/B 首跑预演（cornell-box + bistro-interior）

> 波次定位：G10.5 首轮 A/B 对比波的 A 段（双端出图链路收官 + M130 双端核验腿 +
> 度量预演）。判据事实源：G10_ACCEPTANCE_MAP §1 M130 行 + §3.3；RFC-0026 §4.6 +
> v1.1 章 E errata；spec RXS-0384（+修订记录 v1.1）/ RXS-0385~0389 / RXS-0390。
> **G10 零通过线维持**：本文件全部数字为 measured_local 登记，不构成任何画质/
> 帧率通过判定；差距一律只登记不修复（G10 零修复纪律，M140 差距清单承接锚）。
>
> Assisted-by: Kimi-K3（G10.5a 波续）

## 1. 链路面（双端同源契约消费）

```
milestones/g10/corpus/{camera,lighting}_<scene>.json（M133 冻结登记面，G10.5a 取景校准）
  → g10_5_gen_contract_params.py → contract_params_<scene>.json（四节闭集，RXS-0384）
  ├─ Rurix 端：g10_5_scene_render（Rust 第三实现解析 + GI 管线真渲染）→ HDR EXR
  └─ UE 端：g10_param_contract.py（内嵌 CPython）→ g10_5_build_scenes.py 关卡建设
      → MRQ Phase B（g10_5_ue_render.py，tone curve 关闭 = HDR 臂捕获点）→ HDR EXR
双端 HDR → 同一 host 侧派生链（×2^(−EV100) 曝光尺度〔Rurix 臂；UE 臂 pipe 内手动
  曝光已施 ×1.0〕→ aces13 view transform → IEC 61966-2-1 sRGB 编码）→ LDR EXR
  → FLIP / SSIM / PSNR（ci/g10_flip_lib.py / g10_ssim_psnr_lib.py 单一事实源）
  + 逐像素 diff 报告（g10_m137_diff_report）
```

**契约 digest（M130 g10.5 门实测，evidence
`g10_m130_dual_determinism_contract_20260815T233315Z.json`）**：

| 场景 | param_digest（三方一致：host 参考 × Rust × UE 进程内） |
|---|---|
| cornell-box | `sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118` |
| bistro-interior | `sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514` |
| 联合登记值 | `sha256:64fd54df6e9be522d6dbb3bec8fac1eb30a0a421c7a5a8185a3452c381178aa4`（双场景 digest 字典序拼接 sha256） |

三重绑定：`base_commit=19ff2c66…`（门跑时 HEAD）· `session_run_id=g10ab-20260815T233315Z`（M139 机器前置消费面）。

**应用层探针（RXS-0390，逐点 pixel_delta ≤ 1e-3 px 实测）**：cornell-box 五点
max = **1.586e-05 px**；bistro-interior 五点 max = **2.565e-04 px**（UE 端 =
as-built 相机读回投影，Rurix 端 = f32 view_proj 探针，host f64 参考对账三面
一致）。

## 2. 本波缺陷修复登记（RED→GREEN 证据链）

| # | 缺陷 | 实证 | 处置 |
|---|---|---|---|
| D1 | **契约四元数共轭公式**（RFC-0026 §4.6 / RXS-0384 L2 冻结公式面）：det(M)=−1 反射矩阵共轭「转角保持」不成立，harness `quat_contract_to_ue` 实现 (w,−z,x,y)=R(M·axis,+θ) 镜像朝向 | 共轭恒等式随机对拍：缺陷式最大偏差 **6.35e0**（2000 组）/ pytest 首例 1.39e0；修订式 (w,z,−x,−y) 偏差 **0.0**；cornell 180° 取景为不变量特例、bistro 一般旋转全暴露 | RED 先行 commit `f7425dc8`（pytest 2 failed）→ spec-first commit `72c9511b`（visual_comparison.md v1.1 errata 行 + RFC-0026 v1.1 章 E + RXS-0390 条款，既有字面 0-byte）→ 修复 commit `19ff2c66`（pytest 4/4 GREEN） |
| D2 | **UE 关卡建设 Transform 组合序**（harness 面，非冻结公式）：`compose_transforms(parent, local)` 参序与 UE 5.8.1 语义（先 A 后 B）相反；cornell 全 identity local 不暴露 | bistro 网格 actor 落位实测 (894,−142,776) vs 正确 (894,776,142)（2326 号节点 bounds 对账） | harness 修复（本批 build_scenes.py），修复后 bounds 逐位吻合 |
| D3 | **Interchange 节点变换烘焙**：import_asset 把 glTF 节点世界变换烘进网格顶点（bistro 全 1186 节点 −90°X 旋转实测双重施加 → 场景倾倒全黑帧，nonzero=0/2073600） | 网格局部顶点 = C·R_node·v·100 实测反推（2326 号节点） | harness 修复：actor 只挂 R_fix（yaw+90° = M∘C⁻¹），节点 TRS 不重复施加；扁平单引用前提逐节点核验，多引用即报错不静默 |
| D4 | **MRQ tone curve 默认启用** → EXR 压缩进 ~[0,1]（非 HDR 臂捕获点） | 修复前 UE bistro 帧 max=1.033 | harness 修复：`MoviePipelineColorSetting.disable_tone_curve=True`（5.8 源树 MoviePipelineEXROutput.cpp SCS_FinalColorHDR 分支锚定），修复后 bistro max=77.82 scene-linear |
| D5 | 门脚本嵌套持 gpu_device_lock → 子进程自持锁互斥死锁 | g10.5 门首跑挂起 40 min 实测 | 门内不嵌套持锁（子进程自持），留痕 |

## 3. 双端出图实测（帧统计，measured_local）

| 场景 | 端 | 帧 | 覆盖（nonzero 比） | 亮度中位 | p90 | max |
|---|---|---|---|---|---|---|
| cornell-box | Rurix | 512×512 HDR | **92.90%** | 0.137656 | 1.215951 | 1.4382 |
| cornell-box | UE 5.8.1 | 512×512 HDR | **18.39%** | 0.0 | 0.594237 | 0.5957 |
| bistro-interior | Rurix | 1920×1080 HDR | **95.68%** | 0.133359 | 0.302763 | 2.9937 |
| bistro-interior | UE 5.8.1 | 1920×1080 HDR | **100.00%** | 2.798139 | 5.000016 | 77.8217 |

帧内容 digest（G10EXRD-1 布局实测）：

| 帧 | content digest |
|---|---|
| cornell-box Rurix HDR | `sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727` |
| cornell-box UE HDR | `sha256:c7c6f2cf1644ba79512da1f4f3fceeb2001826f4723681a35ab7a8ca9dc853a2` |
| bistro-interior Rurix HDR | `sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935` |
| bistro-interior UE HDR | `sha256:5bfe1f4965e72e85d4c75f21879f8c89bf1f4e292348fa7e82cd9faf0245cc19` |

LDR 派生链元数据互证实测闭合：四张 LDR 帧 `rurix:source_frame_digest` 与上行
对应 HDR 帧 content digest 逐位相等（RXS-0386 L2 派生链互证面）。

## 4. A/B 首跑度量（LDR 臂，RXS-0386 派生链；参考端 = UE5）

| 场景 | FLIP（LDR，ppd=默认 67.0206） | SSIM（Wang 2004 口径） | PSNR（dB，联合 MSE） | diff err_mean / err_p95 / err_max | diff over_threshold 比（阈值 0.0） |
|---|---|---|---|---|---|
| cornell-box | **0.338645** | **0.348298** | **13.9829** | 0.152145 / 0.472540 / 0.551828 | 0.929005 |
| bistro-interior | **0.940317** | **0.167102** | **2.5845** | 0.720894 / 0.923163 / 1.0 | 1.0 |

读法（诚实口径）：首跑数字被已登记场景面缺口主导（见 §5）——cornell 的
UE 侧壳体零辐射（81.6% 帧面黑）与 bistro 的双端亮度/材质口径差；数字本身
证明度量链路可跑、可分辨、可登记，不证明任何收敛宣称。diff 产物：
`K:\rurix-ext\g10-frames\g10_5\diff\<scene>\`（误差 EXR + 灰度热区图 PPM +
diff_report.json 区域统计 16×16）。

## 5. 差距清单候选（M140 承接锚；只登记不修复）

### 5.1 Rurix 侧渲染缺口（harness 头注诚实边界实测复核）

| # | 缺口 | measured 证据 |
|---|---|---|
| R1 | 材质子集 = 逐图元 baseColorFactor（Lambert）；baseColorTexture/法线/metallic-roughness 不采样 | bistro Rurix 帧近灰白（纹理所载色彩全缺）；G10.3 已登记 DDS 解码归后续波次 |
| R2 | 几何法线（winding 朝向 + 双面翻转），平滑法线不消费 | cornell 壳体单面片外向绕向被双面口径吞没（UE 侧同内容被剔除——口径差见 U1） |
| R3 | 灯种子集 = 契约 sun + sky 常量天光；点/面光源与 glTF emissive 不表达 | bistro 包内 pointLight1~N（glTF 节点实测 4+ 盏）与 emissive surfaces 不表达；cornell 语料点光源按契约降为 sun+sky（生成器注释登记） |
| R4 | GI = 屏幕探针单反弹（host 参考管线），非 Lumen 等效宣称 | bistro Rurix 帧高噪声（单反弹 + 有限样本）；HDR 中位 0.133 vs UE 2.798（≈21×，GI/天光口径差主因之一，见 §6 C1） |
| R5 | JSON 整数解析经 i64（u64 顶格 seed 被 fail-closed 拒绝） | 本波契约 seed=42 不触面，harness 头注登记 |

### 5.2 UE 侧场景面缺口

| # | 缺口 | measured 证据 |
|---|---|---|
| U1 | **cornell 壳体（墙/顶/地板）零辐射**：语料单面片外向绕向（生成器 `ci/_gen_g10_cornell_box.py` 墙/顶/地为单 quad、外向 CCW、法线属性内向）× UE 背面剔除口径 | UE 帧覆盖 48204/262144 = **18.39%**（仅双块可见）；Rurix 同内容 92.90%（双面着色口径）——内容/口径交互差，G10 零修复不改语料不改渲染器 |
| U2 | **bistro 纹理全缺**：包内 .dds 纹理 Interchange 不支持（导入错误逐条日志），材质实例 texture_parameter_values 空 | UE bistro 帧近纯白洗涤态（albedo 全 ≈ 白）；导入错误日志 `LogInterchangeEngine` 逐条在案 |
| U3 | Bistro 动画 Take 001 / glTF 相机节点不引用（动画剥离） | build_scenes 头注登记；相机采用 node 1186 静态位姿（corpus 校准登记） |

### 5.3 harness 面已知标注（非渲染差距）

| # | 事项 | 登记 |
|---|---|---|
| H1 | M137 diff 报告器 `domain` 字段硬编码 `scene-linear-hdr`；本预演消费 LDR 帧对，报告 domain 标签与输入域不符（误差数值本身按 LDR 对计算无误） | diff_report.json 实测；归 M139 波 harness 修订候选 |
| H2 | UE MRQ 出帧 4 帧（0~3），A/B 消费第 0 帧；warmup=64 引擎预热计数经 MRQ AntiAliasing 设置 | build_scenes MRQ 配置面 |
| H3 | 门首跑嵌套锁死锁（D5）已修复；UE cmd 模式 unreal.log 不进 stdout（探针输出走文件面 G10_5_PROBE_OUT） | g10_5_ue_run.py / probe_landmarks.py 头注 |

## 6. 口径差登记（caliber_diff 候选，M139 口径面消费）

| # | 口径差 | measured 数字 |
|---|---|---|
| C1 | **室内亮度主差**：bistro HDR 中位 UE 2.798139 vs Rurix 0.133359（≈21.0×）；cornell 块区 p90 UE 0.594237 vs Rurix ×2^(−EV100) 后 0.303988（≈1.95×） | §3 帧统计；主因 = GI/天光遮蔽口径（UE SkyLight 指定 cubemap 全向 IBL vs Rurix 屏幕探针单反弹）+ 太阳 lux→辐射度链差，不拟合、只登记 |
| C2 | 曝光链：双端 EV100 同字面；Rurix 臂派生尺度 = 2^(−EV100)（cornell 0.25 / bistro 0.5），UE 臂 pipe 内手动曝光已施（FixedExposure=2^(−EV100) 源码实证，build_scenes 头注）派生尺度 ×1.0 | 派生链参数登记 |
| C3 | UE EXR 位深 fp16 → f32 提升（RXS-0385 strip-and-log）；Rurix 原生 f32 | M134 既定口径沿用 |

## 7. 承接锚

- M139 A/B 对比门（`g10.p0.m139.ab_comparison`）：三重绑定消费面已备（M130
  g10.5 evidence `param_digest` + `base_commit` + `session_run_id` 三字段 +
  `application_probes[]`）；本预演 = 其度量执行面的首跑演练。
- M140 差距清单门：§5 候选行入 schema 化登记（UE5 模块归属枚举 +
  measured_delta 溯源 + 承接锚），承接 G11 修复范围法定输入。
- G10.5b/后续：wave5 聚合门、M139/M140 materialize 按 actual next_free 顺位。

## 8. 复跑面

```text
py -3 milestones/g10/harness/g10_5_gen_contract_params.py          # 契约参数（确定性逐字节）
cargo build -p rurix-asset --bin g10_5_scene_render                # Rurix harness
py -3 milestones/g10/harness/g10_5_ue_run.py <build_scenes.py>     # UE 关卡（env 三面见脚本头注）
py -3 milestones/g10/harness/g10_5_ue_render.py <scene>            # UE MRQ Phase B
py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.5
py -3 milestones/g10/harness/g10_5_ab_metrics.py                   # 本文件 §3/§4 数字面
```
