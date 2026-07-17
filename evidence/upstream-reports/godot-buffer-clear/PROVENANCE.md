> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PROVENANCE — Godot `buffer_clear` 上游报告包

## 来源

- 摘取自分支 `codex/grx-godot-dxil-workspace`,路径 `spike/godot-rurix/upstream-repro/`。
- 该目录最后修改 commit:`b1490570b47d956ea8e063410e1b33ce3d90fb8f`(2026-07-13,"feat(grx): R1b 破墙 — misaligned buffer_clear 定罪修正,culling device-removal 消除,残留=cull 数学")。
- 重放方式:内容 0-byte 保真(逐文件 git blob hash 与源分支全等;源文件本已 LF + 尾换行,无需归一改写)。
- 目录结构:`ISSUE_DRAFT.md`(公开提报草稿)+ `mrp/`(最小复现工程,对应源分支 `rd-buffer-clear-misaligned-offset/`)。

## 真因(一句话)

`RenderingDevice.buffer_clear()` 以非 16 字节对齐的 offset(offset % 16 != 0)在主设备帧图内调用即致 D3D12 device removal(`DXGI_ERROR_DEVICE_REMOVED` / `0x887A0005`);根因为 `rendering_device_driver_d3d12.cpp` 的 `command_clear_buffer` 构建 RAW UAV 时 `FirstElement = p_offset / 4` 无 16 字节对齐守卫,违反 `D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT`。

## `<FILL>` 残留清单(提报前待实测/待补)

`ISSUE_DRAFT.md` 中共 4 处占位:

1. **Tested versions — stock build 精确 commit hash**:复现最初在本机自编 4.7-dev 引擎上取得(唯一非标内容为一个本工程从不触及的休眠 module,代码路径 100% stock);提报前须在官方 4.7-dev snapshot 或干净自编 build 上重新确认并回填 hash。
2. **Tested versions — 旧 stable 是否同样复现**(如 4.3/4.4;`buffer_clear` 早于 4.0,大概率为长期缺陷而非 4.7 回归)。另 Vulkan 后端(`--rendering-driver vulkan`)对照结果亦待确认(预期不复现)。
3. **System information**:编辑器 *Help → Copy System Info* 完整串(GPU 驱动版本号 + CPU 型号)。已知部分:Windows 11 / Direct3D 12 (Forward+) / NVIDIA GeForce RTX 4070 Ti。
4. **MRP 附件**:`rd-buffer-clear-misaligned-offset-mrp.zip`——将 `mrp/` 打包(剔除 `.godot/` 缓存、保留 `project.godot`,<10 MB)后附上。

另:草稿末尾整节 "Pre-filing checklist" 为 owner 自查用,粘贴到 GitHub 前须整节删除。

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

## `<FILL>` 清零依赖(待 owner 授权下载 stock Godot build 后实测)

G-EA1-7 要求 Godot 包 `<FILL>` 清零,但清零须在**官方/干净 stock Godot 4.7-dev build**(无自定义
module)上实测。本机无 Godot,下载 stock dev snapshot 属 owner 逐件授权项(非本备包 agent 自决,禁网络
下载)。授权下载 stock build 后,在其编辑器打开 `mrp/` 工程实测,精确所需产物清单:

| # | `ISSUE_DRAFT.md` 位置 | 所需产物(stock build 上实测) |
|---|---|---|
| 1 | Tested versions | stock 4.7-dev build 的**精确 commit hash**(官方 dev snapshot 版本号或干净自编 build 的 `--version --verbose` 串) |
| 2 | Tested versions | Vulkan 后端对照(`--rendering-driver vulkan` 跑同工程,预期**不**复现)+ 旧 stable(如 4.3/4.4)是否同样复现 |
| 3 | System information | 编辑器 *Help → Copy System Info* 完整串(GPU 驱动版本号 + CPU 型号 + stock build 版本) |
| 4 | Minimal reproduction project (MRP) | `rd-buffer-clear-misaligned-offset-mrp.zip`(将 `mrp/` 剔除 `.godot/` 缓存打包,<10 MB) |

判定退出码非 grep stdout(崩溃时 buffered stdout 可能不 flush);连环 device-removal 会污染 GPU/TDR 态致
后续假崩,清洁 offset 先跑或拉开间隔(见 `mrp/README.md` §Notes)。**本包四处 `<FILL>` 全部保留**——清零
需 owner 授权 stock build,agent 不在本机自编 4.7-dev 上充数(否则 hash 无意义)。

## 提报纪律

- agent 只负责备包(本目录即备包产物);**上游提报由 owner 亲自执行,AI 不对外提交**。
- EA1 契约将 `upstream_filing` 列为 out_of_scope:本包在 EA1 期内仅作证据归档,不构成提报动作。
- 提报时点须重跑草稿 §Pre-filing checklist(stock build 确认、查重、填 `<FILL>`)。
