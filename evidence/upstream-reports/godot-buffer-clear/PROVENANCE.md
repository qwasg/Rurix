> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PROVENANCE — Godot `buffer_clear` 上游报告包

## 来源

- 摘取自分支 `codex/grx-godot-dxil-workspace`,路径 `spike/godot-rurix/upstream-repro/`。
- 该目录最后修改 commit:`b1490570b47d956ea8e063410e1b33ce3d90fb8f`(2026-07-13,"feat(grx): R1b 破墙 — misaligned buffer_clear 定罪修正,culling device-removal 消除,残留=cull 数学")。
- 重放方式:内容 0-byte 保真(逐文件 git blob hash 与源分支全等;源文件本已 LF + 尾换行,无需归一改写)。
- 目录结构:`ISSUE_DRAFT.md`(公开提报草稿)+ `mrp/`(最小复现工程,对应源分支 `rd-buffer-clear-misaligned-offset/`)。

## 真因(一句话)

`RenderingDevice.buffer_clear()` 以非 16 字节对齐的 offset(offset % 16 != 0)在主设备帧图内调用即致 D3D12 device removal(`DXGI_ERROR_DEVICE_REMOVED` / `0x887A0005`);根因为 `rendering_device_driver_d3d12.cpp` 的 `command_clear_buffer` 构建 RAW UAV 时 `FirstElement = p_offset / 4` 无 16 字节对齐守卫,违反 `D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT`。

## `<FILL>` 清零状态(4 处,EA1 支线实测回填后)

`ISSUE_DRAFT.md` 原有 4 处占位,2026-07-17 已在 owner 授权的官方 stock build 上实测清零(逐字命令+输出见 [repro_log_stock_20260717.md](repro_log_stock_20260717.md)):

1. **Tested versions — stock build 精确 commit hash**:**已回填 measured**——官方 **4.7.1-stable**(godot-builds release,2026-07-14)`--version` = `4.7.1.stable.official.a13da4feb`(full `a13da4feb8d8aefc283c3763d33a2f170a18d541`)。原诊断在自编 **4.7-dev**;现于最新官方 stable 100%-stock build 上**仍复现**(`0x887a0005` device removal,exit `0xC0000005`)——**影响面上修:非 dev-only,系已发行 stable 缺陷**,已在草稿 Tested versions 显要标注。
2. **Tested versions — Vulkan 对照 + 旧 stable**:**Vulkan 已回填 measured**——`--rendering-driver vulkan` 跑同工程 **300 帧干净退出(exit 0)**,证 removal 为 D3D12 专属。**旧 stable(4.3/4.4)本轮未测**(工具件仅下载 4.7.1-stable);`buffer_clear` 早于 4.0,大概率长期缺陷,留提报前建议项。
3. **System information**:**已回填(CLI 组合取证,诚实标注)**——`Windows 11 (build 28120) - Godot v4.7.1.stable.official [a13da4feb…] - Direct3D 12 (Forward+) - NVIDIA GeForce RTX 4070 Ti (nvidia; 32.0.16.2002 / NVIDIA ~620.02) - 13th Gen Intel Core i5-13600KF`。**非编辑器 Copy System Info 原样**(无 GUI,用 `--version` + PowerShell `Win32_*` CIM 组合;GPU 驱动为 Windows `DriverVersion` 字段);提报时 owner 宜以编辑器实串替换。取证机为 Win11 Insider Preview build 28120。
4. **MRP 附件**:提报时将本包 `mrp/` 目录整体打包为 `rd-buffer-clear-misaligned-offset-mrp.zip`(剔除 `.godot/` 缓存、保留 `project.godot`,<10 MB)。本轮实测即以该 `mrp/` 复制到 scratch(令 `.godot/` 缓存落在库外)后跑出 §4 复现。

另:草稿末尾整节 "Pre-filing checklist" 为 owner 自查用,粘贴到 GitHub 前须整节删除(item 1~3 已随本轮实测标注完成状态)。

## DRAFT 标头口径(G-EA1-7「全部文件显式标头」的落地解释)

- 规范标头 `DRAFT — do NOT file` 已落于本包的**散文/源文件**:`ISSUE_DRAFT.md`(首行,归一自旧变体
  「DRAFT for owner review — do NOT file yet」)、`PROVENANCE.md`(本文件首行)、`mrp/README.md`(首行)、
  `mrp/main.gd` 与 `mrp/misaligned_clear_effect.gd`(首行注释 `# DRAFT — do NOT file — upstream MRP source`)。
- **不加标头的两文件**:`mrp/main.tscn`(Godot 场景序列化文件)与 `mrp/project.godot`(工程配置文件)。
  两者是 **Godot 引擎自动读写的结构化资源**,非人类散文;在其中插入自由文本注释头有**破坏工程可打开性 /
  被编辑器改写覆盖**的风险(尤其 `.tscn` 的 `[gd_scene]` 头必须是文件首行)。故按「全部**草稿散文/源**文件」
  口径豁免此二文件——它们本就是 MRP zip 的一部分、随 `ISSUE_DRAFT.md` 顶部的 do-NOT-file 标头与 README
  标头一并受本包的 DRAFT 定位覆盖。`project.godot` 顶部既有的 `;` 注释(说明 bug)保持不变。此口径 surface
  给 owner:若 close-out 人工核要求「字面全部文件」,可将 `.tscn`/`project.godot` 的 DRAFT 声明改由打包
  README/清单承载,而非注入结构化文件。

## `<FILL>` 清零实测(owner 2026-07-17 授权 stock build 下载 + 实测,已完成)

G-EA1-7 要求 Godot 包 `<FILL>` 清零须在**官方/干净 stock Godot build**(无自定义 module)上实测。
owner 白栀已于 **2026-07-17 会话明示授权**本批下载 stock build 与本机实测(工具件落 scratch,不入库)。
授权后下载官方 **4.7.1-stable**(godot-builds release;注意上游已由原诊断的 4.7-**dev** 前进到 4.7.1-stable,
2026-07-14),将本包 `mrp/` 复制到 scratch(令 `.godot/` 导入缓存落库外)后实测,四处产物**全部实测清零**:

| # | `ISSUE_DRAFT.md` 位置 | 实测结果(stock 4.7.1-stable 上) |
|---|---|---|
| 1 | Tested versions | `--version` = `4.7.1.stable.official.a13da4feb`(full `a13da4feb8d8aefc283c3763d33a2f170a18d541`);**仍复现** → 影响面上修为已发行 stable 缺陷 |
| 2 | Tested versions | Vulkan 对照 `--rendering-driver vulkan`:**clean**(300 帧 exit 0);旧 stable(4.3/4.4)本轮**未测**(仅下载 4.7.1-stable) |
| 3 | System information | CLI 组合串(`--version` + PowerShell `Win32_*`),**非编辑器 Copy System Info 原样**,已诚实标注 |
| 4 | Minimal reproduction project (MRP) | 提报时打包本包 `mrp/`(剔 `.godot/`、留 `project.godot`,<10 MB);本轮即以该 `mrp/` 跑出复现 |

判定退出码非 grep stdout(实测印证:崩溃时 stdout 的 `[repro]` 帧行未 flush,`0x887a0005` 走 stderr;exit
`0xC0000005`);连环 device-removal 会污染 GPU/TDR 态,故 crash 运行置最后、与 clean Vulkan 对照拉开间隔
(见 `mrp/README.md` §Notes)。逐字命令+输出:[repro_log_stock_20260717.md](repro_log_stock_20260717.md)。

## 提报纪律

- agent 只负责备包(本目录即备包产物);**上游提报由 owner 亲自执行,AI 不对外提交**。
- EA1 契约将 `upstream_filing` 列为 out_of_scope:本包在 EA1 期内仅作证据归档,不构成提报动作。
- 提报时点须重跑草稿 §Pre-filing checklist(stock build 确认、查重、填 `<FILL>`)。
