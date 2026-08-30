# W1 修复:slot14 法线源件损坏(day_0829 HANDOVER §H 登记项)

日期 2026-08-29(G37 商业化收官 W1 子任务)。纪律遵守:零 GPU、零 cargo、纯 Python;
v1 目录 `baked_normals_bin/` 0-byte(时间戳 2026/8/29 16:11 未动);src/、kernels/、
milestones/、registry/ 未触。

## 1. 结论一览

| 项 | 值 |
| --- | --- |
| 损坏件 | `slot14.rgba8bin`(材质 `Paris_Table_cloth_01`,mat12,源 `Paris_Table_cloth_01_Normal.dds`) |
| v2 目录 | `artifacts/day_0829_realism/a4_normalmap/baked_normals_bin_v2/`(70 × rgba8bin + manifest_bin.json,不覆盖 v1) |
| 产出路线 | **完整重烘链**(K: 盘源资产在位,gltf + 全 70 张 DDS,源 sha 与 v1 manifest 一致)——非字节复制替换法 |
| 69 张字节相等 | **PASS**(逐张 sha256 双记账 + 逐字节比较,字节差异槽 = [14] 恰好) |
| slot14 v2 | 2048×2048,12 级全 mip,22,369,632 B,全链 RGBA 常值 (127,127,128,255)(法线=+Z) |
| slot14 v2 sha256 | `77bc6c00cd9c8b1db272d8f8c0a7f3586d191c04ce402e3d2d2d413f4e62bd7e`(v1: `e73d692d…41f8`) |
| manifest 差异 | 顶层 0 键差异;70 行中仅 slot14 行变化:`output_sha256`、`mip0_rgba8_sha256`、新增 `sanitized` 登记 |
| 校验 | `verify_v1_v2.py` 11/11 PASS(`verify_output.json` / `verify_log.txt`) |

## 2. 缺陷复核(修前实测)

v1 `slot14.rgba8bin` 全文件(12 级 mip 全部)常值 R=53、G=53、B=128、A=255。
kernel 解码律 x=y=(53−127)/127≈−0.582677:

- ‖xy‖₁ = |x|+|y| = **1.165 > 1**;‖xy‖₂ = 0.824 < 1(z=√(1−x²−y²)=0.567 仍可重建)。
- **判据澄清(如实登记)**:HANDOVER §H"‖xy‖>1 非法法线"对 (53,53) 唯有 L1 范数自洽,
  检测判据据此取 **L1**。判据域实测 v1 全 70 张:mip0 常值件 15 张,仅 slot14 非平坦,
  其余 14 张常值恰为 (127,127)(范数 0)——判据只命中 slot14,其余 69 张绝对安全。

## 3. 烘焙脚本修复(加性,γ/解码逻辑 0-byte)

`bake_normals.py` 新增(单一事实源):

- `FLAT_RG=(127,127)` + `detect_illegal_const_rg(rg)`:mip0 整张常值且 ‖xy‖₁>1
  ((byte−127)/127 解码)⇒ 返回登记 dict(const_rg/decoded_xy/norm_l1/norm_l2/law);
- `flat_like(rg)`:同形平坦替换件;
- `bake_one` 接线:命中 ⇒ 打印 WARN + 替换平坦 + entry 增 `sanitized` 登记(不命中路径字节不变);
- `--out` 参数(默认 `baked_normals/` 不变,供 v2 重烘不覆盖旧目录)。

`pack_normals_bin.py` 最小接线(import 检测函数 + mip0 判定命中即全链替换 + entry 登记 + `--out`)。
**修改域解释决定(如实登记)**:任务文本仅点名 bake_normals.py,但 rgba8bin 消费件由
pack_normals_bin.py 自行重解码 DDS 生产(不经 bake_one),只改前者则"未来重烘自动修复"
对运行时消费件不成立(HANDOVER §I 重烘命令 = 两脚本连跑)。故检测函数落 bake_normals.py
单一事实源,pack 侧加最小消费接线——两者均为 a4_normalmap/ 下脚本的加性修改。

## 4. 产出与验证(全 PASS)

1. **PNG 侧脚本修复验证**:`bake_normals.py --limit 15 --out rebake_png_limit15/`——
   slot00-13 与 v1 PNG 逐字节一致(sha256,`check_png_limit15.py` PASS;字节相同件已清理,
   保留 slot14 平坦 PNG + manifest 为证),slot14 触发 WARN 并平坦化(mean/min/max 全 127,
   manifest 带 `sanitized`)。日志 `bake_limit15_log.txt`。
2. **v2 完整重烘**:`pack_normals_bin.py --out baked_normals_bin_v2/` 全 70 槽,
   packed=70 anomalies=0,slot14 WARN 触发(日志 `pack_v2_log.txt`,68 s)。
3. **v1/v2 对比校验** `verify_v1_v2.py`(输出 `verify_output.json`,11/11 PASS):
   - 文件集:两目录各 71 件(70 bin + manifest)无多余;
   - 69 张(slot≠14)逐字节相等(重烘重现即确定性自证,等价于字节复制);
   - slot14:头 (2048,2048,12) 与 v1 一致、尺寸 22,369,632 B 一致,v2 全文件 ==
     **独立构造**的平坦参照(校验脚本自行拼头+12 级常值,不经 pack 链,双源对证);
   - manifest:顶层键 0 差异,69 行 dict 字面一致,slot14 行变化字段恰为
     {output_sha256, mip0_rgba8_sha256, +sanitized},且 manifest sha == 实文件 sha。

slot14 v2 指纹:`output_sha256 = sha256:77bc6c00cd9c8b1db272d8f8c0a7f3586d191c04ce402e3d2d2d413f4e62bd7e`,
`mip0_rgba8_sha256 = sha256:7c4040a1c92f79c18300fb65926c52b8bf5843c66db7c77cecde8c5026f07687`。

## 5. 交接注意(主 agent 域)

- 窗口 bin 的消费路径切换到 `baked_normals_bin_v2/` 由主线执行;HANDOVER §H 律:
  **任一修复接入消费即重锚**(现锚 5db2e7d7 基于 v1,v1 未动故现锚不受本任务影响)。
- 备选修法"trinm 该材质置 −1"未走(本任务钉烘焙侧替换法)。
- 证据清单(本目录):`bake_limit15_log.txt` / `check_png_limit15.py` /
  `rebake_png_limit15/`(slot14 平坦 PNG + manifest)/ `pack_v2_log.txt` /
  `verify_v1_v2.py` / `verify_log.txt` / `verify_output.json`。
