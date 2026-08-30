# Phase B 纹理全覆盖 + mip 链设计侦察（day_0828 只读侦察产物）

日期：2026-08-28 · GPU：RTX 4070 Ti 12 GB（evidence/g35_render_gate_20260827T122240Z.json "gpu" 行）
范围：只读侦察结论 + 可执行设计提案。未改任何源码、未跑 GPU。

---

## 1. 现状盘点（file:line 全部核实）

### 1.1 装配面（`src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`）

| 事实 | 位置 |
|---|---|
| `G31_TEX_N_MAPPED = 12`、`G31_TEX_TILE = 2048`、`G31_TEX_GRID_COLS = 4` | L5289 / L5293 / L5296 |
| top-N 律法：逐材质三角数降序、并列 material_index 升序 | L5759-5773 |
| 图集尺寸 = cols×TILE × ceil(N/cols)×TILE → 12 槽 = 4×3 瓦 = **8192×6144** | L5793-5797 |
| 瓦片烘焙：槽 k → origin=(k%4, k/4)×2048，小图只占瓦左上 w×h | L5821-5834 |
| 图集存储 = `Vec<u32>` 打包 RGBA8（R\|G<<8\|B<<16\|A<<24），**4 B/texel SSBO**（非 f32 RGB） | L5341-5343, L5828-5832 |
| DDS 解码 `g31_dds_decode_rgba8`：BC1/BC3（+DX10 头），**仅 mip0**（`need = bw*bh*block_bytes` 只取首层） | L5365-5514（mip0 截取 L5409-5414） |
| pow2 且 ≤2048 fail-closed（wrap 精确域前提） | L5812-5816 |
| texmeta 布局：头 8 f32 `[atlas_w, atlas_h, slot_count, 0×5]` + 逐槽 8 f32 `[origin_x, origin_y, w, h, mod_r, mod_g, mod_b, 0]` | L5861-5882 |
| `mod = baseColorFactor × (1−metallic)`（槽 [sb+4..6]；slab 臂再预乘 R_slot） | L5856, L5240-5260 |
| tritex：1 f32/tri（槽号 ≥0 / −1 = 常量面回退） | L5890-5898 |
| texuv：6 f32/tri TEXCOORD_0（装配循环与 tris 同序烘焙） | L5923；源 L1786-1796, L1820-1825 |
| linlut：256 项 sRGB→linear f32 LUT（零 pow 位级锚） | L5516-5525, L5902 |
| host 参考采样 `g31_tex_host_sample`（与 kernel 逐 op 镜像） | L5555-5618 |

当前图集 SSBO 体量：8192×6144×4 B = **192 MiB**。

### 1.2 kernel 采样面（`src/rurix-render/kernels/g31_texture_gi.rx`）

| 事实 | 位置 |
|---|---|
| 绑定 = **13 路固定命名参数**：tlas, tris, mats, quads, points, params, texuv, texmeta, tritex, `atlas: View<global,u32>`, linlut, out_color, out_depth | L44-59 |
| UV 来源 = 主命中重心坐标 `committed_barycentric()` 插值逐三角 texuv 侧表 | L112-127, L173-176 |
| 寻址：REPEAT wrap（fract）→ G26 双线性坐标（floor 未钳 frac）→ 图集线性地址 `(oy+y)*aw + (ox+x)`（usize 精确域，图集 >2^24 texel 故 f32 禁用） | L178-179, L181-193, L194-204 |
| 纹素 unpack + 256 项 LUT + ×mod + tex_gate 混常量面 | L200-230 |
| **命中距离 `th = rq.committed_t()` 已在寄存器中可用**（mip 选择信号现成） | L115 |

### 1.3 窗口车道资源下标（`src/rurix-render/src/bin/g31_window_present.rs`）

| 段 | 下标 | 位置 |
|---|---|---|
| Mega 车道既有 | 0..=21 | L242-243 注释 |
| 窗口 encode | 22（params）/ 23（out） | L244-245 |
| `--textures` 追加 | **24=texuv, 25=texmeta, 26=tritex, 27=atlas, 28=linlut**（count 29） | L266-272 |
| `--svt` 追加（须随 textures） | 29=页表, 30=瓦片池, 31=miss 请求, 32=svtmeta, 33=fallback（count 34） | L283-289 |
| `--smooth-normals` 单臂 | **trinrm=24, tri_mr=25**（count 26）；×bloom 组合臂 = 32/33 | L1479-1492 |

**冲突确认**：`--smooth-normals` 的 trinrm/tri_mr（24/25）与 `--textures` 的 texuv/texmeta（24/25）正面相撞——这是两臂 fail-closed 互斥的下标层根因之一。

### 1.4 互斥 fail-closed 位置（`g31_window_present.rs`）

- **L4913-4921**：`--smooth-normals on` 与 `--fg/--hzb/--textures/--svt/--slab-table` 互斥（L4916 含 `textures`）；L4922-4924 与 cluster/wp-hlod 互斥。
- L4925-4930：smooth on 且默认 SPV 时换载 `DEFAULT_SPV_G18_SMOOTH_NRM`（合流臂需第三个合体 kernel + 换载规则）。
- 相邻互斥（本波不动）：bloom×textures L4883-4891；cluster×textures L4974；wp-hlod×textures L4947。

### 1.5 已存在的 SVT 车道（重要前置事实）

`kernels/g31_svt_gi.rx`（L1-48）：C13 波已落 **稀疏虚拟纹理**生产车道（`--svt on`，须随 `--textures on`）——页表 1024²项 × 页 128²texel（虚拟空间上限 131072²）、物理瓦片池 130²（含 1 texel border）、miss 请求 1 f32/px 反馈 + host 流送（`streaming/svt.rs`）、**miss 回退 = 逐槽均值 ×mod**（= 今日"均值 albedo"观感）。图集不再直绑（L38-44）。全驻留时与图集直采位级一致。

### 1.6 采样代码既有 bug（侦察顺带发现，Phase B 应顺手修）

双线性底行插值 G/B 通道误用 `fy` 做水平混合（R 通道正确用 `fx`）：

```219:227:src/rurix-render/kernels/g31_texture_gi.rx
        let t0r = p00_r * (1.0 - fx) + p10_r * fx;
        let b0r = p01_r * (1.0 - fx) + p11_r * fx;
        let samp_r = (t0r * (1.0 - fy) + b0r * fy) * mod_r;
        let t0g = p00_g * (1.0 - fx) + p10_g * fx;
        let b0g = p01_g * (1.0 - fy) + p11_g * fy;
        let samp_g = (t0g * (1.0 - fy) + b0g * fy) * mod_g;
        let t0b = p00_b * (1.0 - fx) + p10_b * fx;
        let b0b = p01_b * (1.0 - fy) + p11_b * fy;
        let samp_b = (t0b * (1.0 - fy) + b0b * fy) * mod_b;
```

host 镜像同 bug（g14_3_lane_body.rs L5612/L5615）⇒ SSBO 腿探针 p100=0.0 仍绿（两侧同错）；sampler 腿 host 参考是**正确**双线性（L5652-5654），容差 ≤1 LSB 在 8-bit 量化下未暴露。传播面（逐字 fork）共 8 处：`g31_texture_gi.rx` L223/226、`g31_texture_probe.rx` L88/91、`g31_svt_gi.rx` L253/256、`g31_svt_probe.rx` L119/122、`g34_unified_gi.rx` L288/291、`g34_unified_gi_skin.rx` L274/277、`g34_unified_shade.rx` L227/230、host `g14_3_lane_body.rs` L5612/5615。修法 = 全部 `fy→fx`（kernel/host 同步改，探针继续位级绿）。

---

## 2. 70 材质 DDS 源清单（census 实测，material_census.json）

- 70 材质 70 张**互不共享**的 BaseColor DDS（无共享 URI）。
- **53 张 2048×2048**（50 DXT1/BC1 + 3 DXT5/BC3，**全部自带 12 级完整 mip 链**）+ **17 张 16×16** DXT5（5 级 mip）。≥1024² = 53 张。
- mip0 texel 总量 = **222.3 MTexel**；含链 ×4/3 = 296.4 MTexel。
- 存储形态对比：
  - **u32 打包 RGBA8（现形态 4 B/texel）：mip0 848 MiB / 含链 1131 MiB**
  - f32 RGB（12 B/texel）：mip0 2544 MiB / 含链 3392 MiB —— **直接否决**（RTX 4070 Ti 12 GB 下占 28%，且带宽×3，无收益：LUT 后按需转 f32 即今日既有路径）。
- top-12 三角覆盖 66.7%；top-24 = 88.0%；top-32 = 94.6%；全 70 = 100%。

---

## 3. 方案对比

### 方案 A：单大图集 2048 瓦片网格直扩（现律法参数化）

70 槽 → 8×9 瓦 = **16384×18432，mip0-only 1152 MiB**，padding 浪费 26.4%（16² 小图独占整瓦）。
- 改动量：host 常量 + 网格行数推导（~10 行）；kernel **0 行**（texmeta 驱动）。
- 缺点：无 mip（远景 minification 闪烁/走样保留）、1.1 GiB 无质量分级、padding 纯浪费。
- 变体 A'：瓦片一律 1024（装配期 box 半采样）= 8192×9216 = **288 MiB** mip0-only；近景质量封顶 1024。

### 方案 B：多页图集（texmeta + page 字段，kernel 按页选 SSBO）

**判定：kernel 语言不支持动态多 SSBO 索引** —— 绑定面 = 固定命名参数闭集（g31_texture_gi.rx L44-59；所有 kernel 变体皆逐个枚举绑定，无 buffer 数组语法；SVT kernel 的规避方式正是"图集不直绑、经页表在**固定两块** SSBO 内间接寻址"，g31_svt_gi.rx L38-46）。多页只能落成 k 个固定绑定 + 逐页 selection-arm 门（每页 ~40 行门控读），且每页占 1 个资源下标——与 SVT 段 29..33 抢位，屏障计划、descs、evidence 全面翻倍。**否决**（仅当单 SSBO 超 `maxStorageBufferRange` 才被迫启用；4070 Ti 报 4 GiB，1.13 GiB 安全，装配期加 fail-closed 断言即可）。

### 方案 C（推荐）：线性 texel heap + DDS mip 链直搬（u32 打包不变）

关键洞察：**"图集"本就是手动寻址的一维 SSBO**（kernel 只用 `aw` 做行距，L194-199），2D 网格是自缚——改为**线性 texel 堆**后瓦片尺寸上限、网格 padding、16384 维度顾虑全部消失：

- 布局：`atlas` SSBO = **u32 偏移头表**（70 槽 × 13 mip 槽位 = 910 项，含 base_offset 与各级 w/h 可推导）+ 逐槽逐 mip 连续 texel 段。偏移量最大 296 M > 2^24，f32 texmeta 存不精确 ⇒ **头表必须 u32，放进 atlas buffer 自身头部 = 零新增绑定**。
- texmeta 逐槽 8 f32 不变（w/h = mip0 尺寸；[sb+7] 空位 = mip_count）。
- 尺寸三档：
  - **保守默认：≥1024² 一律降到 1024 cap + 全 mip 链 = 283 MiB**；
  - 自适应：top-12（按三角数）保 2048 原生、其余 1024 = ~400 MiB；
  - 原生全量：1131 MiB（12 GB 卡可承受，画质臂选项）。
- mip 来源：**DDS 源 mip 直接搬**（53 张 2048² 全带 12 级链）——`g31_dds_decode_rgba8` 加逐级 offset 步进（块数按级折半，~20 行），**不做装配期 box 降采样**（省一套重采样代码 + 保留美术原始 mip；G11.3 manifest 锚 = mip0 rgba8 digest 不动，新增逐级 digest 为新 evidence 字段）。cap-1024 档 = 直接从 mip1 起搬（零重采样）。
- **mip 选择信号（kernel）**：`lod = clamp(log2(th · k_pix · k_tri · tex_w), 0, mips−1)`
  - `th` = 主命中距离，**寄存器现成**（g31_texture_gi.rx L115）；
  - `k_pix` = 2·tan(fovy/2)/height，host 一个新 params 槽（≥[43]，variant-owned 扩参先例 = dyn [42]/smooth [43]/lamp [49]）；
  - `k_tri` = sqrt(uv_area/world_area)，装配期逐三角预算——**tritex 步幅 1→2 f32 `[slot, k_tri]`，零新增绑定**；
  - 全 f32 算术 + floor，确定性协议不破（无导数、无跨像素交互）。
- 过滤：**首落 nearest-mip + bilinear**（维持 4 fetch + 既有 G26 块，kernel 增量 ~60 行）；trilinear（8 fetch + 级间 lerp，+~60 行）留 flag 后补——夜景 EV 下 mip 间跳变可接受，先拿全覆盖收益。
- kernel 改动量：~60 行（lod 计算 + 头表寻址）；host ~150 行（heap 装配 + 逐级解码 + k_tri + 参考采样镜像）。

### 方案 D：借道既有 SVT 车道（登记备选，非 Phase B 主路）

texmeta 扩 70 槽虚拟 origin（虚拟空间 131072² 远够）+ 池预算（如 2048 瓦 = 132 MiB 常驻）+ miss 流送已全部现成；**回退面 = 逐槽均值 = 今日观感**，退化优雅。缺 mip（SVT-mip = 页表 ×4/3，后续波）。适合作为全覆盖的**终态形态**；Phase B 用方案 C 打底（texmeta 布局与 SVT 兼容，后续可平移）。

---

## 4. 推荐：方案 C（cap-1024 档起步，自适应档跟进）

理由：
1. **283 MiB** 起步（现 192 MiB 仅 +47%），一次拿到 100% 三角覆盖 + 全 mip 链（远景走样一并治）；
2. 零新增绑定、零 2D 网格 padding、无 16384 上限问题；kernel 语言约束（无动态多 SSBO）天然规避；
3. u32 打包 + LUT 既有位级锚全保留，探针法（SSBO 腿 p100=0.0 + 双跑位级）继续成立；
4. 与 SVT 终态兼容（heap 头表 = 页表的退化形）。

## 5. 与 `--smooth-normals` 合流的资源下标计划

- 现冲突：trinrm/tri_mr（24/25，L1479/1486）撞 texuv/texmeta（24/25，L266-267）。
- **合流臂（tex+nrm，SVT/bloom 互斥维持）**：texture 五件**留 24..28 不动**（SVT 29..33 栈依赖其序），**trinrm=29、tri_mr=30，count=31**。ggx 合流同臂（tri_mr 真表已含）。
- 未来 tex+nrm+svt 三合臂再谈：trinrm/tri_mr 让位 34/35（本波不动）。
- buffer 体量（1,046,609 tri）：texuv 25.1 MB、tritex（步幅 2）8.4 MB、trinrm 37.7 MB、tri_mr 8.4 MB、texmeta ~2.3 KB、linlut 1 KB、atlas heap 283 MiB（cap 档）——合流臂 SSBO 总增量 ≈ 363 MB，4070 Ti 12 GB 预算内。

## 6. host/kernel 改动清单（Phase B 施工单）

1. **修双线性 fx/fy bug ×8 处**（§1.6 列表；kernel+host 同步，探针位级绿维持）。
2. `g14_3_lane_body.rs`：`G31_TEX_N_MAPPED` 12→70；`g31_tex_load` 网格→heap（偏移头表 u32 进 atlas 头部）；`g31_dds_decode_rgba8` 逐级解码；texmeta[sb+7]=mip_count；tritex 步幅 2 + k_tri 装配；host 参考采样加 mip 选择镜像。
3. 新 kernel `g31_texture_nrm_gi.rx`（g18_smooth_nrm 法线 gather + 纹理采样块 + mip 选择合体，~250 行 fork+merge）+ NoContraction 后处理同律；SPV 落 `.tmp/g31_gates/texture/`。
4. `g31_window_present.rs`：L4916 从 smooth 互斥集移除 `textures`；L4925-4930 换载规则加 (smooth&&textures) 分支；新 descs 构建器（既有 `g31_lane_descs_nrm` L1522+ 与 tex descs L1718-1721 合体）+ 合流屏障计划 + trinrm=29/tri_mr=30；evidence 扩字段（atlas_form=heap、mip_levels、per-level digest）。
5. 探针/CI：probe 律法扩 mip 维（每槽 24 UV × 抽 3 mip 级）；`ci/g31_texture_sampling_smoke.py` 判读器同步。
6. fail-closed 新增：装配期断言 heap 字节 ≤ `maxStorageBufferRange`；DDS 缺 mip 链（mips < log2(w)+1）显式拒绝或按可用级截断登记。

## 7. 验收门建议

- 双跑位级一致（digest_seq）+ tex_on≠tex_off 生效门（既有）；
- SSBO 腿探针 p100=0.0（heap 化后继续）；逐槽逐级 rgba8 digest 与 G11.3 manifest（mip0）互核 + 新增逐级 digest 登记；
- 帧时回归门：夜巡实测 tex_on real_render_frame_ms 4.79 vs off 4.04（tex_on_ev.json）——全覆盖 + mip 后要求 ≤ +30%（mip 命中局部性通常反而降带宽）；
- 视觉验收位:右墙 Paris_Paintings（rank 56）/红帘 curtainB1（rank 13）/红墙 Plaster_Red（rank 50）三处从均值色块变为真实纹理（对照 crop 工具已备:artifacts/day_0828/recon/probe_pixels.py）。
