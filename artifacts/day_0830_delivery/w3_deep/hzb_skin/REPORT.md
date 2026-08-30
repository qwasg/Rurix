# G37 W3 hzb_skin——HZB×蒙皮同车道合并面 判档报告

- **门键字面**：`g37.wave3.hzb_skin`（harness evidence schema `rurix.g37.hzb_skin_unified_evidence.v1`,.tmp 工作区件不注册 check_schemas——G34-2/G34-3 同律）
- **留窗兑现**：G36 W4-W5 提交（bece24e7）登记字面「HZB×蒙皮同车道（新 kernel 合并面）归后续波」——本波兑现,`--hzb on --skin` 同开成立
- **判定**：**GO（已实装）**。合并所需结构 = 一个新 kernel + 一个 render_exec 加性镜像扩展 + 一个合并区段,未超一臂当量;唯一结构冲突（双 TLAS×蒙皮 BLAS refit,即任务书预判的「HZB 初剔的 BLAS 分解与逐帧蒙皮 AS rebuild 冲突」形）有界且已按 G34-2 双 TLAS 入口同律加性解决,不构成 no-go
- **纪律遵守**：未跑 GPU;未 `cargo build --release`;未碰 target-night;g31_window_present.rs / g14_3_lane_body.rs / g14_3_pipeline_perf.rs / milestones/ / registry/ / ci/ 全部 0-byte;既有 SPV 字节 0-byte（新 SPV 新文件）

---

## ① 蒙皮形变机制侦察结论

**蒙皮 = 预 pass 顶点形变，非 kernel 内蒙皮。**G34-3 车道结构（`g34_skin_section.rs`）：

1. `g31_skin` compute pass（pass 0,逐顶点 LBS）读绑定姿态/权重/palette 双表,**重写 tris SSBO 角色段**为当帧蒙皮世界空间顶点,同时写 `G34S_PREV`（上一帧 palette 蒙皮顶点表,MV 类 3 臂消费）。
2. **blas_refit 桥**（`FrameUpdate::blas_refit`,after_pass=0）：`vkCmdCopyBuffer` 把 tris 角色段拷进角色 BLAS 顶点缓冲 + 原地 UPDATE build（角色 BLAS 创建期 `updatable_blas` 打标）。
3. 场景 kernel `g34_unified_gi_skin` 消费 AS（3 实例单 TLAS）+ 输出 `out_hit` 4f32/px `[inst, prim, bu, bv]`（miss = inst −1 哨兵）;角色实例变换恒 identity——形变全在 BLAS 顶点内。
4. `g34_unified_mv` 三臂 MV（类 1 相机/类 2 刚性/类 3 蒙皮）消费 hit 通道 + PREV 表;**char_inst/dyn_inst 全经参数面下发（params[35]/[53]）,kernel 零硬编码实例号**——这是合并可行性的关键事实。

**HZB 车道几何输入与蒙皮的交点**（`g34_2_hzb.rs`）：静态场景逐 mesh 节点 BLAS 分解 + 动态尾槽;**双 TLAS**（表 0 = 初剔掩码面供 primary,表 1 = 全 0xFF 供 shade 阴影射线——被剔实例仍投阴影,RXS-0297 单 TLAS 签名纪律拆 pass）;`blas_refit: None`（G34-2 纯 TLAS 实例变换面）。primary 经 `inst_base` 前缀和表做 `pg = prim + inst_base[inst]` 全局分派——**角色段只需追加一个 inst_base 槽（char_tri_base）即被 shade 正确着色**（角色 mats 常量行/tritex −1 常量面,着色数学与 gi_skin 逐 op 同式）。

**结构冲突（侦察实证）**：render_exec 的 `FrameUpdate.blas_refit` 校验强制与 `tlas_update` **同槽**,而 `tlas_update_b`（表 1）校验强制与 blas_refit **异槽**（单帧单槽单写纪律）——即单帧只能 refit 一个 AS manager 的角色 BLAS 副本,但双 TLAS 双 manager 各持独立副本,主射线（表 0）与阴影射线（表 1）都需要当帧蒙皮 BLAS。旁路评估：cull mask 位域单 TLAS 方案被 rurixc 排除（`ray_query_initialize` cull mask 恒 0xFF,RXS-0298 冻结,动编译器远超本任务面）;双提交/隔帧交替 refit 方案有 2× GPU 税或语义降级,劣于加性扩展。

## ② 设计选择：混合最小路径（1 新 kernel + 1 执行器加性镜像 + 纯 host 接线）

| 面 | 处置 | 依据 |
|---|---|---|
| 主射线 kernel | **新 kernel** `g34_unified_primary_skin.rx` = G34-2 primary 全字面 + `out_hit [inst,prim,bu,bv]` 加性第 4 输出（gi_skin 扩面①同格式,`hit_f·(inst+1)−1` 哨兵位级门） | G36 留窗字面「新 kernel 合并面」;母版 `g34_unified_primary.rx` 0-byte |
| 执行器 | **render_exec 加性 `blas_refit_b`**：新入口 `next_provenance_with_update_dual_tlas_ex` / `execute_with_frame_update_dual_tlas_ex`（表 1 manager 的第二 BLAS refit,须与 tlas_update_b 同现同槽;`None` = 既有面 0-byte,既有双 TLAS 入口委托新入口） | 上述结构冲突的有界修复;G34-2「双 TLAS 更新加性入口」同律先例;并行任务 #90（render_exec_g37_fif_dyn.rs）已按本扩展三参形状预写调用点互锁 |
| shade/reduce/test/pack/g31_skin/g34_unified_mv | **六件 0-byte 消费**（角色分派走 inst_base 前缀和;MV 实例号参数化 char_inst=N+1/dyn_inst=N） | kernel 侧零硬编码实例号的侦察结论 |
| host | 新 include 区段 `g34_full_lane/g34_hzb_skin.rs`（G34HS 前缀）:G34-2 车道骨架（两阶段闭环/probe 三面对拍）+ G34-3 蒙皮件（g34skin_assets/核验三面）合并臂 | 两区段本体 0-byte,include! 同模块符号直消费 |

**合并语义要点**：实例分解 = [静态逐节点 BLAS 0..N-1 | 动态 N | 角色 N+1],动态/角色两尾槽恒可见不参剔（核验对象面,剔除计数 = 静态节点如实登记）;角色 BLAS 两副本创建期 updatable 打标,每拍双桥 refit（after_pass=0）;角色 TLAS 级 AABB 滞后一帧 = **G34-3 单表车道在案语义逐字继承**（TLAS 帧首 refit 读上一帧 BLAS 内容,质心 ≤4px/AABB ≤6px 容差吸收面）;闭环重拍幂等（palette 帧内不变 ⇒ g31_skin 重跑同输出 ⇒ 双桥重拷同字节）。

## ③ 修改清单 + 新 SPV sha

**新文件**（3 件）：
| 文件 | sha256 | 说明 |
|---|---|---|
| `src/rurix-render/kernels/g34_unified_primary_skin.rx` | `10ced5e6eacc86ed503eb81d2a3998a7eabf3c7091adad77e67c0131e09d009d` | 合并主射线 kernel（新 SPV 新文件） |
| `.tmp/g34_gates/hzb_skin/g34_unified_primary_skin.spv` | `7d3ae216762e793908b3394eca2173c82e9559ddf7a7ae235aba8c0a5d3b87f0` | rurixc 编译产物 + **spirv-val 全绿** |
| `src/rurix-render/src/bin/g34_full_lane/g34_hzb_skin.rs`（2528 行） | `c62432ea…f953f8` | 合并区段（descs/lane/main;门键 `g37.wave3.hzb_skin`） |

**修改文件**（3 件,全部标注「G37 W3 hzb_skin」注释）：
- `src/rurix-rt/src/render_exec.rs`：加性 `blas_refit_b`——`frame_update_state` 第三参 + `validate_blas_refit_b`（同现同槽/区段/对齐/after_pass 全 fail-closed）+ `PreparedFrameUpdate.blas_b` + `AsFrameOps.blas_refit_b` + `record_frame_body` 第二桥（表 1 manager,六步桥接与既有 refit 逐字同律）+ `*_dual_tlas_ex` 双新入口（既有入口委托 None = 0-byte;generation 记账 = tlas_b 同槽双写归并一代,blas_refit×tlas_update 同律）。FIF 路恒 None。
- `src/rurix-render/src/bin/g34_full_lane.rs`：撤除 `--hzb`×`--skin` 互斥字面 → 合并早分支（先于两单开分支;--full/--auto-move/--slab-table 必随,static-camera/headless 拒,geo×skin 留窗维持既有先行裁决）+ `--spv-hzbskin-primary` 旗标（闭集守卫:非同开面拒收）+ 尾部 include。
- `src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs`：`G34HzbBits::load` 加 `char_tri_base: Option<usize>` 参数（Some = inst_base 追加角色槽 + 实例计数 +1;既有调用传 None,产物字节不变）。

**0-byte 面**（不动清单）：`g34_unified_primary.rx` / `g34_unified_gi_skin.rx` / `g34_unified_mv.rx` / `g31_skin.rx` / `g27_hzb_reduce.rx` / `g27_hzb_test.rx` / `g31_hzb_pack.rx` / `g34_unified_shade.rx` / `g34_skin_section.rs` / `g14_3_lane_body.rs` / `g31_window_present.rs` / `g14_3_pipeline_perf.rs` / milestones / registry / ci。

## ④ cargo check 与 selftest 结果

- `cargo check -p rurix-render --features vendor-upscale --bin g34_full_lane`（dev/默认 target）：**exit 0 全绿**,本波新文件零告警（仅 g34_skin_section.rs 既有 3 条 unused 告警,先在）。
- `cargo check -p rurix-rt`：exit 0（执行器扩展 + 并行任务 #90 的 g37_fif_dyn 三参调用点互锁编译通过——该文件已按本扩展形状预写 `blas_b: None`/`blas_refit_b: None`）。
- `cargo check -p rurix-render --features vendor-upscale --bins`（全 bins 防回归面）：exit 0 零 error。
- `py -3 artifacts/day_0830_delivery/w3_deep/hzb_skin/accept.py --selftest`：**PASS**（42 项:skin 口径 9 臂 / hzb 口径 9 臂 / 合并腿公共判 9 臂 / digest 序列 4 臂 / 单开不降级 4 臂 / frame_ms 3 臂 + facts 闭集与调用契约互核;纯 CPU 零 GPU 零构建）。

## ⑤ 主 agent GPU 验收命令

```powershell
# 一键验收（构建 release + rurixc 现编十件 SPV + spirv-val + 五腿真跑 + 七 facts 判读;
# 五腿 = merged_a / merged_b(双跑) / allvis(RURIX_HZB_ALL_VISIBLE=1 像素中性臂)
#       / hzb_single / skin_single(单开不降级对照);gpu_device_lock 内串行）
py -3 artifacts/day_0830_delivery/w3_deep/hzb_skin/accept.py --run --frames 64 --warmup 10
```

判据（七 facts 闭集,accept.py 头注释全文）= **蒙皮动核验（skin 门口径:逐顶点位级/位置/MV 三类/窗级真动/类2激活）∧ HZB 金字塔/判定/剔除（hzb 门口径:parity 三面 + occluded_p1≥1）∧ 剔除像素中性（vs allvis 逐帧位级）∧ 双跑位级 ∧ 单开臂各自口径不降级 ∧ frame_ms 如实登记（G6 无硬门）**。产物落本目录 `accept_result_<ts>.json` + harness 真跑件留 `.tmp/g34_gates/hzb_skin/`。

手工单跑合并臂（调参/取证用）：

```powershell
# 前置:target\release\g34_full_lane.exe + WORK 目录 SPV（accept.py --run 会自动编译,
# 或手工: target\debug\rurixc.exe src\rurix-render\kernels\<k>.rx --target vulkan -o .tmp\g34_gates\hzb_skin\<k>.spv）
target\release\g34_full_lane.exe --hzb on --skin on --full `
  --slab-table milestones\g31\g31_slab_side_table_bistro_interior.json `
  --frames 64 --warmup 10 --auto-move orbit --tier 100 --hidden `
  --spv-hzbskin-primary .tmp\g34_gates\hzb_skin\g34_unified_primary_skin.spv `
  --spv-hzb-shade .tmp\g34_gates\hzb_skin\g34_unified_shade.spv `
  --spv-hzb-pack .tmp\g34_gates\hzb_skin\g31_hzb_pack.spv `
  --spv-hzb-reduce .tmp\g34_gates\hzb_skin\g27_hzb_reduce.spv `
  --spv-hzb-test .tmp\g34_gates\hzb_skin\g27_hzb_test.spv `
  --spv-skin .tmp\g34_gates\hzb_skin\g31_skin.spv `
  --spv-skin-mv .tmp\g34_gates\hzb_skin\g34_unified_mv.spv `
  --evidence .tmp\g34_gates\hzb_skin\leg_manual.json
# PASS 标记字面: "[g34_full_lane]: [hzb_skin] PASS"
```

## 提案登记（不直改面）

1. **ci/ 门脚本扩合并臂（提案,不直改）**：建议后续窗新增 `ci/g37_hzb_skin_smoke.py`（以 accept.py 七 facts 为蓝本蒸馏门裁决件 + PASS-only schema + check_schemas 前缀路由 `g37_hzb_skin_gate_` 纯追加）;或在 g34 两门脚本各加一条「合并臂在位时单开口径复跑」交叉腿。本波按纪律未动 ci/。
2. **g14_3_lane_body.rs**：零改动需求（全部消费为只读符号）,无提案。
3. **风险登记**：①角色 TLAS 级 AABB 滞后一帧为 G34-3 在案语义继承,若后续角色帧间位移增大需转 TLAS 后置 refit 结构（触 render_exec 录制序,另立窗）;②合并臂 evidence 为 .tmp 工作区件,数字待门裁决件蒸馏后方可登记 milestones。

## 交付物清单

- 实装:上述 3 新文件 + 3 修改文件（工作树未提交——并行会话在飞面多,按仓惯例由协调侧按文件名显式择取入批）
- 验收:`artifacts/day_0830_delivery/w3_deep/hzb_skin/accept.py`（--selftest PASS;--run 归主 agent GPU 面）
- 判档:本文件
