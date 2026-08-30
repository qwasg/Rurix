# Rurix G37 Release Notes — sdk-1.1.0(候选)

> 状态:**bundle 候选**(2026-08-30,G37 W5 产)。W4 验收主链(GPU 整批重锚)在跑;
> 其结果若触发代码重建,本 bundle 按 `ci/g37_sdk_bundle_repack.py` 一键重打,W6 终验
> (`ci/g31_sdk_dist_smoke.py --gate g31.g37w5.dist` 全链:MSVC 离线可建 + GPU canonical
> 真跑)通过后转终版。候选产物:`dist/sdk_bundle/sdk-1.1.0/`(24 组件 + 发布七件,
> ≈2.26 MiB;`CANDIDATE_MANIFEST.json` 登记全部输入锚)。

## 1. 新能力(G37 战役新臂,窗口交互预览 `g31_window_present`)

- **透明材质(`--transparency`)**:主射线 ≤8 层穿透 + 点光阴影透射衰减;透明判定 =
  glTF alphaMode BLEND 或 baseColorFactor.a<1。GI2/AO/反射的次级射线仍视玻璃为
  不透明(如实留窗)。修复 bistro 玻璃隔断"雾状楔形"既有可见缺陷的主视角面。
- **RIS 选灯 + 灯片 CDF 面光 NEE(`--ris`/`--nee`)**:GI2 反弹 RIS 选灯(M=4~8 候选)
  与 44k 灯片 CDF 面光 NEE;能量口径不双计(NEE 开时灯片 emission 直取置零 + 代表灯
  反弹让位)。方差源头收缩路线(EVAL_RESTIR §9.3),非降噪器平台化。
- **LUT 色彩分级(`--lut off|neutral|warm|<path.cube>`)**:display encode 后段第 4 级
  补齐;LUT 表内嵌参数缓冲尾部,零新绑定。
- **PSO 变体账本**:管线全部会话构造期创建,运行期唯一重建点 = era 重建;账本守护
  (era0=precache 面,era≥1 miss 告警,`RURIX_G31_PSO_STRICT=1` 升 fail-closed,
  `--pso-report` sidecar)。
- **VisBuffer 生产证据臂(`--visbuffer`)**:窗口会话消费真轨迹相机 × 真簇 DAG 的
  device 证据链(cut→分箱→SW u64 原子软光栅→oracle 全等),sidecar evidence,
  presented 呈现面零改动。
- **帧生成组合(FG × full 预设)**:FG 与十九臂场景面正交组合;合法形态为两点式
  (all-off 基线 ∪ full 预设),散臂混搭 fail-closed 拒(防组合态爆炸);
  fg×hzb/slab/svt/lut 维持互斥留窗。
- **逐帧 cut→AS(`--cluster-per-frame-cut`)**:BLAS 顶点 refit 竞技场(全簇固定槽位
  拓扑 ≈72MB,cut 以顶点内容切换,TLAS 恒不动);帧 k 状态为帧号纯函数(确定性)。

## 2. 默认行为变更

- **`--quality` 缺省 off→full(十九臂交付默认)**;`off` 升为显式回退档。
  交互预览开箱即为全质量形态;CI 调用面已按 `--quality off` 显式补扫(A 类 18 调用点)。
- presented digest 锚随语义变更整批重收割(W4 在跑);锚为**二进制绑定锚**,
  跨重建以 W4 重锚结果为准。

## 3. 修复

- **rurixc「if 包 while」codegen**:structured_merge 前向可达遍历不裁剪循环回边
  导致 OpSelectionMerge 误指臂内块;修 = 交汇计算排除 latch→header 回边。
  98 生产 kernel 修复前后 90/90 位级全同(既有 SPV 零漂移)。
- **em+AE override 错位**(day_0828 遗留):`set_autoexp` 选择块补 `_EM` 两分支,
  emissive-tex 组合下自动曝光逐帧绑定错位消除。
- **法线烘焙件 v2**:slot14 源资产损坏件(整张常值非法法线)替换为平坦 (127,127)
  全 12 级 mip;其余 69 张与 v1 逐字节相等。检测律 = L1 范数常值域判据。
- **g34 三 kernel fx/fy 轴误用**:kernel 6 处 + host 镜像 2 处成对同步修,
  "同错互抵恒绿"假象消除;g34 三门 + g36 组合门 GPU 复跑全绿。
- **display encode v2 收编**:共享编码 SPV 切 v2(43b0c255→e7291c79),
  device-vs-host parity 硬门落地(exact=99.9891%,p100=1 LSB,防复发红臂含 ACES 转置)。
- **资源连号断言升级**:`g31_apply_autoexp` 连号断言 debug_assert→assert
  (release 下生效,车道创建期一次性常数代价)。

## 4. SDK bundle 变更(16→24 组件,sdk-1.0.0→sdk-1.1.0)

- **新 SPV 四件**(既有 `spv/` 组织方式并入):`g31_realism_transp.spv`(35983d0f…)
  / `g31_realism_ris.spv`(622a1c33…)/ `g31_display_encode_lut.spv`(9087b743…)
  / `g34_unified_primary_skin.spv`(7d3ae216…)。realism 链为链式超集谱系
  (transp 链位工件锚定,ris 为现行最高链位)。
- **许可义务四件**(GAP-01~03 闭合,与 release.yml 编排同口径):`LICENSE-MIT` /
  `LICENSE-APACHE` / `THIRD_PARTY_NOTICES.md`(rowan + 传递闭包,上游 LICENSE 逐字
  随附)/ `third_party_embedded.cdx.json`(内嵌库级 CycloneDX 补充视图)。
- **C ABI 不变**:`rurix_renderer.h` 9 函数,ABI 版本 0x00010000;SDK 包装 g14_3
  生产车道,本版新臂均为窗口预览 CLI 面,不扩 SDK ABI(RD-036 维持 open,判档
  见 W5 报告)。
- evidence 冻结面版本化:门 schema v1(const=16)0-byte 冻结,新 schema v2
  (`milestones/g31/g31_sdk_dist_v2_evidence_schema.json`,const=24)+ 前缀路由
  `g31_sdk_dist_v2_` 纯追加注册。
- vendor 运行件对账维持三件:NGX/Streamline 与 FSR 动态装载不捆绑
  (vendor SDK 二进制不入分发),basis_universal 静态入 DLL(Apache-2.0)。

## 5. 口径与度量声明

- **生成帧不入真实渲染帧率口径**:帧生成(FG)臂输出的生成帧与真实渲染帧分开计量,
  任何帧率/帧时数字除非显式标注,均指真实渲染帧;含生成帧的呈现率单独标注。
- 帧时数字为 **measured_local**(RTX 4070 Ti 单卡,Windows),非跨硬件承诺;
  full 十六臂参考帧时 9.5-10.7ms(90fps 预算内,day_0829 soak 口径),十九臂 W4
  重锚后以新鲜 evidence 为准。
- 渲染确定性以 presented/末帧 digest 位级锚见证;bench Stage A 18 格锚
  (`c1d28ad7…`/`g14_3_stage_a_digest_anchor.json`)全程零漂移。
- SDK 离线可建/canonical 真跑判据(160+10 帧,末帧 digest == Stage A 锚)属 W6
  终验面,本候选未跑(候选阶段完成:打包确定性 ×2、digest 一比一闭环、SBOM 双视图
  覆盖、四级校验安装、幂等再装)。

## 6. 已知限制(如实留窗)

- 透明:次级射线(GI2/AO/反射 NEE)视玻璃不透明;alpha 排序/折射未做。
- 反射:单样本 GGX 有偏近似(无 pdf 归一,能量 clamp 控);命中点用材质均值
  albedo 非贴图采样。
- 软阴影:光度项仍取灯心方向(lr≪d 近似);面光/PCSS 真解留窗。
- AO 挂在小常量环境光上,绝对幅度有限。
- 法线贴图:mip 链 XY box 平均未重归一化(kernel 侧归一化兜底);BC5 8bit 量化近似。
- FG 组合:两点式合法形态(all-off ∪ full),散臂混搭 fail-closed;
  fg×hzb/slab/svt/lut 互斥留窗。
- 异步双队列(M59):判档维持 no-go(digest 等价硬前置全过,重叠率中位 48.54%<50%
  阈值);机制件入库为后续窗基建。
- VSM 页管线:判档 no-go/留窗(ray 车道无 shadow map 生成成本可摊销);
  重启锚 = 光栅阴影档立项。
- vendor DLSS 格 validation 计数 1(vendor 层既有噪声,digest 全 MATCH,非本版引入)。
- params[52] 帧旋转 f32 >100k 帧精度退化(soak 32f 迭代口径不触)。
- 法线烘焙件 v2 为资产侧工件(≈350MB heap 面),不随 SDK bundle 分发;
  重生成链 = `bake_normals.py` + `pack_normals_bin.py`(K: 源资产在位)。

## 7. 签名与完整性

- 分发完整性信任根 = `SHA256SUMS`(干名字典序确定性)+ rurixup 四级内容寻址
  fail-closed(级① channel 锚 digest → ② bundle digest → ③ 树 digest → ④ 逐文件
  sha256;任一失配零半装拒装)。
- `signing_manifest.json` 为 **self-signed-test 声明面**:生产 Authenticode
  (Azure Artifact Signing)经 CI secret + 人工门控,本机打包不可达——候选包
  **不携生产签名**,如实降级登记(`CANDIDATE_MANIFEST.json` 同条)。终版发布走
  release.yml 编排时按 spec/release.md §4 门控。
