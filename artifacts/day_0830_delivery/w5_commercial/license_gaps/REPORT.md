<!-- Assisted-by: Cursor:Fable（G37 商业化收官 W5 license_gaps 子任务） -->
# G37 W5：vendor 许可矩阵 GAP-01~03 附带义务闭合 REPORT

> 战役：day_0830_delivery（G37 商业化收官）W5 商业化收尾。
> 任务语义：G33 波 C 遗留三缺口（登记语义 =「附带义务未闭前不以对应形态发布」）闭合，
> 为 SDK bundle 分发解除许可前置。
> 纪律执行：零 GPU、零 cargo；src/、kernels/、registry/deferred.json、milestones/ 既有行
> 字面零改写（closure 走追加）；ci/ 修改按专项授权。
> 日期：2026-08-29。

---

## 1. GAP 义务字面（事实源 = `milestones/g31/g31_vendor_license_matrix.json` gaps[]）

| 缺口 | 义务字面（一句话） |
|---|---|
| GAP-01 | 发布 bundle 资产（3 二进制 + 7 manifest 件）零 LICENSE/NOTICE 件——Rurix 本体 MIT OR Apache-2.0 双许可文本与 rx.exe 内嵌 rowan 的 MIT/Apache 声明保留义务须**随分发件闭合**（未来 Jolt/BasisU/FSR/NGX/Taichi 捆绑面同链）。 |
| GAP-02 | release.yml `--component` 三段许可单标 `Apache-2.0` 与 Cargo workspace `MIT OR Apache-2.0` 双许可**字面不一致**（保守单标不减损权利，但与 Cargo 元数据/SBOM licenseConcluded 字面不一致，consistency_note）。 |
| GAP-03 | SBOM 粒度 = 组件级，**未展开二进制内嵌第三方库**（如 rx.exe 内 rowan），granularity_note，归 C5 SBOM 扩展面。 |

## 2. 逐 GAP 闭合动作与产出

### GAP-01 —— 随附许可文本与第三方声明

**产出**：

1. **`dist/licenses/THIRD_PARTY_NOTICES.md`（新建）**——第三方声明与许可文本集合：
   - 现发行面内嵌闭包**逐项登记**（事实源 Cargo.lock）：`rx.exe` 内嵌 **rowan 0.15.18**
     （lock 解析；矩阵登记 0.15.1 为 `src/rurixc/Cargo.toml` 需求字面）+ 传递闭包
     **countme 3.0.1 / hashbrown 0.14.5 / memoffset 0.9.1 / rustc-hash 1.1.0 / text-size 1.1.1**
     ——比矩阵字面（仅 rowan）更完整的诚实闭包；`rurixup.exe`（依赖仅 rurix-pkg 自有）与
     `rurix_rt_cabi.lib`（rurixc 仅 build-dependencies，产物零第三方）零内嵌**如实登记**。
   - 六件许可文本 = 本机 cargo registry **源码包内 LICENSE 逐字复制**（零凭记忆重造；
     hashbrown/memoffset 版权行逐字保留，rowan/countme/rustc-hash/text-size 上游文件本身
     无版权行，如源登记），包 checksum 引 Cargo.lock 字面。双许可组件登记「本分发选择
     MIT 条款履行」，Apache-2.0 备选文本由随附 LICENSE-APACHE 承载。
   - SDK bundle 面（`rurix_renderer_sdk.dll` ← basis_universal 1.16.4 static-in-dll，
     Apache-2.0 文本+NOTICE 在树路径登记）与 NGX/FSR/Taichi/Jolt/rapier/CUDA 未来捆绑面
     义务**同链登记**（引用矩阵条目，owner 字面引用不复制）。
2. **`.github/workflows/release.yml` 打包清单接线**——发布编排追加 4 组件并入资产清单与
   回读自校验闭环：`LICENSE-MIT`、`LICENSE-APACHE`、`THIRD_PARTY_NOTICES.md`、
   `third_party_embedded.cdx.json`（每件 5 段 `--component` 进 bundle.json digest 闭环 +
   SHA256SUMS + SBOM 双视图 + gh release 资产；rurixup release 的「3 组件完备最小集」判定
   为下界判定，超集安全；签名面仍为 3 二进制 selftest 不变）。

### GAP-02 —— 许可字面一致化

**产出**：release.yml 三个二进制 `--component` 许可段 `Apache-2.0` → **`MIT OR Apache-2.0`**，
与 `Cargo.toml` workspace.package license 字面**逐字一致**；SBOM licenseConcluded /
CycloneDX license id 由同一 `--component` 字面生成（`src/rurixup/src/sbom.rs` 机制 0-byte），
自动同源同字面。rurixup 许可段为自由字面（`parse_component` 5 段解析，发布审计仅管
NVIDIA 分区白名单），无白名单阻断风险（侦察核实）。

### GAP-03 —— SBOM 内嵌库级展开

**产出**：**`dist/sbom/third_party_embedded.cdx.json`（新建，CycloneDX 1.5 补充视图）**：
- `rx.exe` → rowan 0.15.18 + 传递闭包 5 crate（`purl pkg:cargo/...` + Cargo.lock 包
  checksum + linkage=static-embedded + 引入链登记）；
- `rurixup.exe` / `rurix_rt_cabi.lib` → `third_party_embedded_count=0` 如实登记（附依赖闭包
  论证字面）；
- SDK bundle 面 `rurix_renderer_sdk.dll` → basis_universal 1.16.4（static-in-dll，
  `pkg:github/BinomialLLC/basis_universal@900e40fb…`）同批展开；
- 随 release.yml 资产分发（进 digest 闭环）。组件级生成机制（`sbom.rs`）**0-byte 不动**
  （src/ 禁改纪律）——生成器自动展开归后续 src 授权批次，登记为残余。

### 登记更新（append-only）

- **矩阵 JSON**（`milestones/g31/g31_vendor_license_matrix.json`）：三个 gaps[] 各追加
  `closure` 段（closed_date=2026-08-29 + actions + evidence 路径 + residual），
  `rust_rowan` 条目追加 `closure_note`（条件「GAP-01 闭合后转 cleared」义务面兑现留痕）。
  **既有行与 `status:"open"` / `redistribution_status:"conditional"` 字面零改写**——
  这同时是 evidence schema（`g31_vendor_license_evidence_schema.json` 把 gap status 钉为
  const `open`、summary 钉为 cleared 15/conditional 1）下唯一合规写法；正式改判归下一次
  矩阵版本化修订（需 schema 同批修订，milestones/ 既有行授权外）。
- **人读渲染面**（`docs/renderer/vendor_license_matrix.md`）：追加 §6「GAP closure 登记」
  （三行 closure 表 + rowan 条件兑现说明 + 发布口径说明），§1~§5 既有行零触碰。

### 门判读升级（ci/ 专项授权）

`ci/g31_vendor_license_smoke.py` leg `obligations_and_gaps_registered` 追加 **closure 机核**
（CHECK_KEYS/evidence schema 闭集 0-byte，不新增 check 键）：
- 三件各带 closure{closed_date 格式 + actions 非空 + evidence **逐路径在树**}；
- GAP-01 实物核：NOTICES 在树非空且覆盖 rowan（**与 Cargo.lock 锁定版本互核**）+ 传递闭包
  四件 + basis_universal；release.yml 4 许可组件在案且源文件在树；
- GAP-02 字面核：release.yml 三段许可 == `Cargo.toml` workspace 字面（双向读取，非写死）；
- GAP-03 互核：补充 SBOM bomFormat/rx.exe→rowan 版本+purl 与 Cargo.lock 一致 + 其余分发
  组件登记在场。
- selftest 追加红臂⑥（closure 段缺失必红）⑦（closure 证据路径断链必红）+ 真树绿臂
  （7 RED + 2 GREEN）。evidence gaps[] 追加 `closure_registered/closure_date` 字段
  （schema 无 additionalProperties:false，PASS-only 路由不受影响）。

## 3. 核验结果

| 核验 | 结果 |
|---|---|
| `py -3 ci/g31_vendor_license_smoke.py --selftest` | PASS（7 RED + 2 GREEN；含 closure 缺段/证据断链两红臂） |
| `py -3 ci/g31_vendor_license_smoke.py --gate g31.waveC.license` | PASS 7/7 checks（本 REPORT 落盘前 fail-closed 实测红一次：closure 证据路径不在树即门红——证据与实物脱节被真拦截后复绿） |
| `py -3 ci/check_schemas.py` | PASS（新 evidence 走 g31_vendor_license_ 前缀路由，gaps.status const=open 维持满足） |
| 矩阵/md 既有行 0-byte | git diff 仅 + 行（追加），零 - 行（见 §4 复核记录） |

## 4. 残余（无法本机闭合项，如实登记）

1. **历史已发布资产不可追溯补件**：v1.0.1-dist.1/.2 两个 pre-release 演练资产已上传（零
   LICENSE/NOTICE 为已发生事实）；closure 对**下一次 release run 起效**。历史件为自签测试
   证书 selftest 性质（非生产信任根）在案，无生产分发暴露面。
2. **rust_rowan 字面改判 conditional→cleared**：义务实质已闭（closure_note + 门机核），但
   status 字面改写需矩阵版本化修订 + evidence schema（summary const cleared=15/conditional=1、
   gaps.status const=open）同批修订——schema 在 milestones/ 既有行冻结面内，超出本批
   append-only 授权，归后续版本化修订批。
3. **sbom.rs 生成器自动展开**：内嵌库级展开当前以补充视图（静态登记 + 门与 Cargo.lock
   互核防漂移）承载；生成器本体（Cargo.lock 驱动、随每次 release 重生成）归后续 src
   授权批次。
4. **外部法务确认类义务：无**——本批三 GAP 均为工程随附/字面/粒度义务（OSI 许可面沿
   G13 FSR MIT 零障碍先例，零新 owner 动作；非 OSI 面 G13 owner 在案 + Attachment A 机制
   在案，本批未触碰）。

## 5. 发布口径变化

「附带义务未闭前不以对应形态发布」前置：GAP-01（随附）/GAP-02（字面）/GAP-03（展开）
三义务已在分发编排内闭合并有门机核——SDK bundle 分发链的**许可前置解除**（closure 段 +
`g31.waveC.license` closure 腿承载）。SDK bundle 自身重打包（16 组件闭集扩许可件涉
`g31_sdk_dist_evidence_schema.json` const=16 冻结面）归 W5 SDK 重打包任务按其授权处理；
在扩集落地前，SDK bundle 分发形态随附 `dist/licenses/THIRD_PARTY_NOTICES.md` +
basis NOTICE 即满足义务字面（NOTICES §3 登记）。
