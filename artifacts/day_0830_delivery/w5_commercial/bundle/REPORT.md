# G37 W5:SDK bundle 重打包 + SBOM/签名刷新 + release notes — 交付报告

> 2026-08-30,纪律:禁 GPU / 禁 cargo build(二进制已就绪),纯打包/脚本/文档面;
> py_compile/selftest 允许。milestones/ 冻结面按 schema 版本化纪律;
> registry/deferred.json 只读(RD-036 判档以本报告为载体)。

## 1. 侦察结论

- **打包链本体** = `ci/g31_sdk_dist_smoke.py`(G31+ 波 C C5 门,g31.waveC.dist):
  16 组件扁平 staging → `rurixup release --channel stable`(bundle.json /
  channel_manifest.json / signing_manifest.json / sbom.spdx.json / sbom.cdx.json /
  SHA256SUMS / gate_decision.json 七产物)→ `rurixup install --from-dir` 四级校验
  物化(布局映射 = install.rs `component_rel_path`:*.h→include/ *.spv→spv/
  *.json→manifests/ *.md→docs/ *.cpp→examples/ *.lib→bin/lib/ 其余→bin/)。
- **v1 冻结面** = `milestones/g31/g31_sdk_dist_evidence_schema.json`:
  `component_count const=16`、components 枚举闭集 16、`version const=sdk-1.0.0`;
  门 selftest 与 schema 互核钉死。evidence 既有两件(`g31_sdk_dist_2026082*`)。
- **持久 bundle 产物此前不存在**:门产物落 `.tmp/g31_gates/sdk_dist/`(易失工作区),
  `dist/` 此前仅许可两件(licenses/ + sbom/,W5 GAP 闭合 agent 产)。
- **签名机制**(src/rurixup/src/signing.rs):`--sign 'name|status|timestamped|backend'`
  为外部验签状态**回填声明**;生产后端 Azure Artifact Signing 经 CI secret +
  人工门控,本机不可达;门先例 = self-signed-test 声明。
- **现成二进制**:`target/debug/rurixup.exe`(8/26)、runtime 五件(rurix_renderer
  三件 = 8/27 门跑 emit 产物,rurix_renderer_sdk 两件 = target/release 8/27)——
  SDK 源(apps/g31-renderer-sdk)自 058f8e68 合流后 0-byte,8/27 件即终态;
  `target-night/release/`(07:11-07:13)为窗口 bin 群,不含 SDK 件。

## 2. 组件闭集扩面处置:16 → 24

新增 8 件(任务 N=8):

| 组件 | sha256(前 16) | 来源 | 说明 |
|---|---|---|---|
| g31_realism_transp.spv | 35983d0f405169ec | 工件谱系锚定 | 透明臂链位;源快照已被 RIS 演进覆盖(链式超集律),自 `inputs/` 快照取件 + 锚硬核对 |
| g31_realism_ris.spv | 622a1c33c18e645c | 锚定(源在树可现编) | RIS/NEE 臂,realism 现行最高链位(源 = kernels/g31_realism.rx) |
| g31_display_encode_lut.spv | 9087b743a6fc426e | 锚定(源在树可现编) | LUT 色彩分级臂 |
| g34_unified_primary_skin.spv | 7d3ae216762e7939 | 锚定(源在树可现编) | HZB×蒙皮同车道 primary kernel |
| LICENSE-MIT | 82e7f8d9616f9532 | 树内 | 许可义务件(license 字面 MIT) |
| LICENSE-APACHE | c71d239df91726fc | 树内 | 许可义务件(Apache-2.0) |
| THIRD_PARTY_NOTICES.md | b14bdf90a696671a | 树内(dist/licenses/) | GAP-01 随附义务闭合件 |
| third_party_embedded.cdx.json | b982a18ac921c6b7 | 树内(dist/sbom/) | GAP-03 内嵌库级 CycloneDX 补充视图 |

- 四件新 SPV sha256 与任务锚前缀**逐一核对一致**;按既有 SPV 组织方式并入
  (干名扁平 staging,install 落 `spv/`)。
- 许可四件组件名/license 字面与 release.yml 编排段(GAP 闭合 agent 先例)**同口径**;
  第一方件 license 统一 `MIT OR Apache-2.0`(GAP-02 workspace 口径,v1 门全写
  Apache-2.0 的旧字面随 v2 修正)。
- 落位如实登记:NOTICES→docs/、cdx→manifests/(既有后缀律);LICENSE 双件无后缀
  按「其余」律落 bin/(install.rs 属禁改面〔禁 cargo〕,非理想落位,W6 后归主线酌处)。
- **法线烘焙件 v2 不入 bundle**:资产侧工件(≈350MB heap 面,SDK g14_3 车道不消费),
  release notes §6 登记重生成链。
- 输入快照 + 锚表:`artifacts/day_0830_delivery/w5_commercial/bundle/inputs/`
  (13 件锚定输入 + INPUT_ANCHORS.json;runtime 五件与旧 SPV 四件对上次 PASS 门
  bundle.json digest 一比一核对全 OK)。

## 3. schema 版本化(冻结面纪律)

- **旧 schema 0-byte**:`g31_sdk_dist_evidence_schema.json`(const=16)零改动,
  既有 v1 evidence 两件继续走 v1 路由校验。
- **新 schema**:`milestones/g31/g31_sdk_dist_v2_evidence_schema.json`
  (`rurix.g31.sdk_dist_evidence.v2`,subject `g31_sdk_dist_v2`,门键
  `g31.g37w5.dist`,wave `G37.W5`;`component_count const=24`、枚举闭集 24、
  `version const=sdk-1.1.0`、`from_dir.components const=24`;facts 闭集 9 /
  signed_dlls 2 / vendor_runtime 3 / offline_build 判据面与 v1 逐字同)。
- **patch 纯追加注册**:`ci/_patch_g31_sdk_dist_v2_schemas.py`(先例
  `_patch_g31_sdk_dist_schemas.py` 同型:锚唯一性机核 + 幂等 + py_compile);
  load/validator 插 v1 锚后,**route 插 v1 锚前**——v2 evidence 前缀
  `g31_sdk_dist_v2_` 被 v1 前缀包含,长前缀路由必须先匹配(先例 =
  `g31_texture_sampling_heap_` 系),脚本含路由序机核(v2 index < v1 index)。
  跑两遍验证幂等(第二遍只核验不重插)。
- **门脚本判读升级**(`ci/g31_sdk_dist_smoke.py` 本体,ci/ 非冻结面,W1
  texture 判读器同文件重写先例):组件闭集 16→24、SDK_VERSION sdk-1.1.0、
  schema/门键/subject/wave 随 v2、KERNELS 增现编三件、PREBUILT_SPV 锚定 transp
  (锚失配 FAIL fail-closed / 缺件 DEV_ENV_DEGRADE)、LICENSE_COMPONENTS 四件、
  license 字面分件查表、from_dir components=="24"、evidence 前缀
  `g31_sdk_dist_v2_`、selftest 断言全量随新 schema。
- **selftest 结果:PASS(67 臂全 ok)**——含 24 组件闭集正例/缺一多一必红、
  v2 schema 全 const 互核(component_count=24/version=sdk-1.1.0/枚举 24/
  from_dir=24)、新增映射断言(G37 SPV→spv/、NOTICES→docs/、cdx→manifests/、
  LICENSE→bin/)、license 查表、PREBUILT 锚形状。py_compile 三件绿
  (门脚本/patch 脚本/check_schemas.py)。

## 4. bundle 候选产物

- **一键幂等重打脚本**:`ci/g37_sdk_bundle_repack.py`(selftest 15 臂 PASS)。
  与全门分工:全门 = 从源重建全链(cargo + rurixc + MSVC + GPU,W6 终验);
  本脚本 = 已就绪工件纯打包(零 cargo/GPU/MSVC)。组件闭集/版号/license import
  自门脚本(单一事实源,两脚本不漂)。
- **产物路径**:`dist/sdk_bundle/sdk-1.1.0/` — 32 文件 **2,367,918 B ≈ 2.26 MiB**
  (24 组件平铺 + 发布七件 + CANDIDATE_MANIFEST.json 登记件)。
- **断言全绿**:release ×2 七产物逐字节一致(打包确定性)/ digest 一比一闭环
  (staging sha256 == bundle.json == SHA256SUMS,24 组件)/ SBOM SPDX+CycloneDX
  双视图覆盖 24 组件 + sdk-1.1.0 / 签名清单两 DLL 声明 + upload_permitted /
  `install --from-dir` 四级校验(components=24, digest_levels=4, 布局齐,
  逐字节==源, 幂等再装 registered=1)。**重跑幂等验证:产物字节级一致**。
- **SHA256SUMS 头 6 行**(干名字典序):

```
e28bda66d25274f15cfe0b0d860b7f2294a444d648b87feea5fb85a609c222ef  API_VERSIONING.md
c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4  LICENSE-APACHE
82e7f8d9616f953250aecd4b204c2e7b6ac43daad8e5d644b3bbf21ef854771d  LICENSE-MIT
b14bdf90a696671a90f414109764f95fb75a1c0f8a68c3f2f43d76218ec11a94  THIRD_PARTY_NOTICES.md
99d80e803505d8d0881da55fddf44d18f81eee8b39ad0958a3d4a865ed0b48d5  compatibility_matrix.md
0ef424422ea3ba87549f63416a5c072463d713b08d1abab35f41d843be4e6c99  feature_matrix.md
```

- 树内文档四件(integration_guide/feature_matrix/performance_tuning/
  API_VERSIONING)较 v1 bundle 已演进(树是事实源,如实入包);
  compatibility_matrix.md 与 v1 逐字节同。

## 5. 签名机制现状(如实登记)

**降级为「声明性 selftest 清单 + digest 信任根」**:生产 Authenticode(Azure
Artifact Signing)经 CI secret + 人工门控,本机打包不可达,候选包不携生产签名。
`signing_manifest.json` = 两 DLL self-signed-test 声明(status=Valid/timestamped/
verified 回填面,rurixup 门先例同型)+ 组件 content digest。分发完整性信任根 =
SHA256SUMS + rurixup 四级内容寻址 fail-closed。`CANDIDATE_MANIFEST.json`
`signing_degradation` 字段同条登记;终版发布走 release.yml 时按 spec/release.md §4 门控。

## 6. release notes

`dist/RELEASE_NOTES_G37.md`:候选状态声明 / 七组新臂 / `--quality` 默认翻转
(off→full 十九臂)/ 修复六项(rurixc if-while / em+AE / slot14 v2 / g34 fx-fy /
encode v2+parity 门 / assert 升级)/ bundle 16→24 / **口径声明含「生成帧不入真实
渲染帧率口径」**、measured_local 单卡口径、W4 重锚依赖 / 已知限制十一条(全部
如实留窗字面)/ 签名降级。

## 7. RD-036 C ABI v2 判档(registry/deferred.json 只读,本报告为判档载体)

**结论:维持 open(maintain-open),本役零 C ABI v2 扩面需求;登记归主线。**

判档理由(backfill 两条件逐项字面核验):

1. **① upcall 硬需求——不成立**。G37 全部新臂(transparency/LUT/PSO 账本/
   VisBuffer/RIS/NEE/FG 组合/frame-cut)均为窗口 bin `g31_window_present.exe` 的
   CLI flag 面 + kernel/装配层内部机制,零「.rx 调起宿主回调」形态;窗口 demo
   flag 面不属 SDK ABI。
2. **② 外部固定 ABI——不成立**。本役无新嵌入面;SDK 包装的 g14_3 生产车道
   API 零变化(apps/g31-renderer-sdk 自 058f8e68 合流后 0-byte),
   `rurix_renderer.h` 维持 9 函数(abi_version/caps_probe/create/destroy/
   load_scene/set_camera/set_exposure_ev100/render_frame/present),签名全部
   标量 + 裸指针,在 C 兼容子集 v1 包络内;ABI 版本 0x00010000 不变。
3. bundle 组件扩面(SPV/许可件)为**分发数据面**,不触 ABI;sdk-1.0.0→sdk-1.1.0
   为 bundle 版号非 ABI 版号。
4. 谱系一致:与 2026-08-24(G24.3 M-d)、2026-08-25(G31+ C1)两次判档同向
   (backfill 字面无一成立,诚实维持不冒充 close)。主线收账时可按 G24.3 M-d
   先例向 RD-036 history 追加本行(disposition=maintain-open,
   reeval_anchor=超界 FFI 需求成立)。

## 8. 残余账与 W6 指引

- **W4 依赖**:候选基于 8/27 SDK 终态件 + 战役 SPV 锚。W4 验收若触发代码重建:
  重收割工件 → 刷新 `inputs/` 快照与 `INPUT_ANCHORS.json`(transp 若重造,同步
  更新门脚本 `PREBUILT_SPV` 锚)→ `py -3 ci/g37_sdk_bundle_repack.py --status final`
  一键重打;W6 全链终验 = `py -3 ci/g31_sdk_dist_smoke.py --gate g31.g37w5.dist`
  (cargo 构建 + SPV 现编 + MSVC 离线可建 + GPU canonical 160+10 digest 对拍,
  PASS 落 `evidence/g31_sdk_dist_v2_<ts>.json`)。
- **pre-existing 失配(非本任务引入,归 W1)**:`ci/check_schemas.py` 全跑现一红——
  `evidence/g31_encode_parity_20260829T171623Z.json` `harness.frames_completed=8`
  vs schema `const=10`(W1 encode parity 门自身 evidence/schema 失配;本任务改动
  仅 sdk_dist_v2 三处纯追加,与该路由无关,已复核)。
- check_schemas.py 工作区含 W1 其他 agent 未提交注册块(texture_heap/encode_parity),
  与本任务 v2 块共存无冲突(patch 锚唯一性机核通过)。
- v1 门键 `g31.waveC.dist` 随门脚本升级退役(evidence 谱系保留);如需复跑 v1 形态,
  git 历史可取。
- install.rs 无后缀件落 bin/ 的许可件落位归主线酌处(涉 rust 面,本役禁改)。

## 9. 交付物清单

| 类型 | 路径 |
|---|---|
| bundle 候选 | `dist/sdk_bundle/sdk-1.1.0/`(32 件,2,367,918 B) |
| 一键重打脚本 | `ci/g37_sdk_bundle_repack.py`(selftest 15 臂 PASS) |
| schema v2 | `milestones/g31/g31_sdk_dist_v2_evidence_schema.json` |
| patch 注册脚本 | `ci/_patch_g31_sdk_dist_v2_schemas.py`(幂等 ×2 验证) |
| 门脚本升级 | `ci/g31_sdk_dist_smoke.py`(selftest 67 臂 PASS) |
| release notes | `dist/RELEASE_NOTES_G37.md` |
| 输入快照+锚表 | `artifacts/day_0830_delivery/w5_commercial/bundle/inputs/`(13 件 + INPUT_ANCHORS.json) |
| 本报告 | `artifacts/day_0830_delivery/w5_commercial/bundle/REPORT.md` |
