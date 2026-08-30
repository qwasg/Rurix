#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W6 svt mutex_registered 登记态干跑（无 GPU/无 cargo 复核件）：
# 以真函数（ci/g31_svt_smoke.py 的 detect_svt_mutex / mutex_registered_exit）
# 走通「互斥字面命中 → 落盘登记件（自校验硬门）→ 退 0」正臂与
# 「host 腿红 → 退 1 不产件」红臂。哨兵时戳 19700101T*,产物经
# ci/check_schemas.py 新路由校验后由调用方删除（不留伪造 evidence）。
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location(
    "g31_svt_smoke", ROOT / "ci" / "g31_svt_smoke.py"
)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

# 与 harness fail-closed 全字面同形（W6_GATES.json svt 行实证同款）。
line = (
    f"[g14_3_pipeline_perf]: FAIL {m.MUTEX_LITERAL}（SVT 假设 = 2048 网格图集"
    "/texmeta origin/tritex 步幅 1,heap 化未适配——fail-closed 登记,"
    "SVT 深修归后续波）"
)
assert m.detect_svt_mutex("头\n" + line + "\n尾", 1) == line, "互斥字面捕获失败"

greps = {"assert_zero_svt_dependency": True, "SvtDependencyDetected": True}
ts = "19700101T000000Z"  # 哨兵时戳,防与真跑件混淆
rc = m.mutex_registered_exit(line, 1, True, True, greps, ts)
p = ROOT / "evidence" / f"g31_svt_mutex_registered_{ts}.json"
print(f"[dryrun] 正臂 rc={rc} evidence 在树={p.is_file()}")
assert rc == 0 and p.is_file(), "正臂应退 0 且产登记件"

# 红臂：host 金标准腿红 ⇒ 退 1 且不产件（登记态不得掩盖 host 面）。
ts_red = "19700101T000001Z"
rc_red = m.mutex_registered_exit(line, 1, False, True, greps, ts_red)
p_red = ROOT / "evidence" / f"g31_svt_mutex_registered_{ts_red}.json"
print(f"[dryrun] 红臂 rc={rc_red} 不产件={not p_red.is_file()}")
assert rc_red == 1 and not p_red.is_file(), "红臂应退 1 且不产件"
print("[dryrun] PASS（正臂产件待 check_schemas 路由校验后删除）")
sys.exit(0)
