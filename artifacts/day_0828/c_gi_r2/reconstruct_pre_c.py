#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase C 恒等性排查工具：从当前 g31_texture_nrm_gi.rx 机械还原 pre-C 源
（逆向本相三处编辑），供「pre-C 源重编 == pre-C SPV 备份」字节对拍，
定位七臂锚漂移根因（源码-SPV divergence vs GI2 尾加恒等性破坏）。
"""
import re
import sys
from pathlib import Path

SRC = Path(r"h:\rurix\src\rurix-render\kernels\g31_texture_nrm_gi.rx")
OUT = Path(r"h:\rurix\.tmp\night_0828\spv\_pre_c_reconstruct.rx")

text = SRC.read_text(encoding="utf-8")

# 1) 还原头注释参数面（去 Phase C 四槽三行）。
text = text.replace(
    """//   （params 56 f32：[43] 平滑门 [44..48) 环境光 [48] GGX [49] 灯贡献阈
//   [50] k_pix；day_0828 Phase C GI2 加性臂 [51] 门 [52] frame_idx
//   [53] firefly clamp [54] gi_scale——[51]=0 关臂 while 零迭代 +0.0
//   恒等尾加）；""",
    """//   （params 56 f32：[43] 平滑门 [44..48) 环境光 [48] GGX [49] 灯贡献阈
//   [50] k_pix）；""",
)

# 2) 删除 GI2 段（从段首注释到 gi_i 递增收口，止于 ④ 深度注释前）。
m = re.search(
    r"        // ── Phase C GI2 加性臂.*?            gi_i = gi_i \+ 1;\r?\n        \}\r?\n",
    text,
    re.S,
)
if not m:
    print("FAIL: GI2 段未定位")
    sys.exit(1)
text = text[: m.start()] + text[m.end() :]

# 3) 还原输出三行 + 输出段注释。
text = text.replace(
    """        // ── 输出（lo = emission + albedo·inv_pi·direct + 环境光 + GGX + GI2
        //    + 天光；GI2 前逐字 g18_smooth_nrm——al_* 为采样/常量选择值；
        //    GI2 关臂 gi_* 恒 +0.0 ⇒ al_*·0.0 = +0.0 尾加位级恒等〔spec_* 非
        //    负 ⇒ x + +0.0 = x 逐位〕）──""",
    """        // ── 输出（lo = emission + albedo·inv_pi·direct + 环境光 + GGX + 天光；
        //    g18_smooth_nrm 逐字——al_* 为采样/常量选择值）──""",
)
text = text.replace(
    " + spec_r + al_r * gi_r) + sky_amb * 0.55;", " + spec_r) + sky_amb * 0.55;"
)
text = text.replace(
    " + spec_g + al_g * gi_g) + sky_amb * 0.65;", " + spec_g) + sky_amb * 0.65;"
)
text = text.replace(
    " + spec_b + al_b * gi_b) + sky_amb * 0.85;", " + spec_b) + sky_amb * 0.85;"
)

OUT.write_text(text, encoding="utf-8", newline="")
print(f"OK -> {OUT} ({len(text)} chars)")
