# G31+ 串行合流补丁：注册 g31_hlod_l4 / g31_svt(harness+gate) 三个 evidence schema
# 幂等：已注册则跳过；锚点 count==1 断言；写后复读驻留核验；py_compile。
import io, py_compile, sys

P = r"h:\rurix\ci\check_schemas.py"
s = io.open(P, encoding="utf-8", newline="").read()
CRLF = "\r\n" in s
if CRLF:
    s = s.replace("\r\n", "\n")

if "g31_hlod_l4_schema = load(" in s and "g31_hlod_l4_validator" in s and 'f.name.startswith("g31_hlod_l4_")' in s:
    print("[patch] already registered, skip")
    sys.exit(0)

# ── ① load 段 ──
a1 = '''    g31_rd027_poison_guard_schema = load(
        ROOT / "milestones/g31/g31_rd027_poison_guard_evidence_schema.json"
    )
'''
i1 = a1 + '''    # G31+ 波 C Task C12 HLOD L4 档门前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_hlod_l4_schema = load(
        ROOT / "milestones/g31/g31_hlod_l4_evidence_schema.json"
    )
    # G31+ 波 C Task C13 SVT 四行门/harness 双 schema 纯追加（重放幂等面）
    g31_svt_schema = load(
        ROOT / "milestones/g31/g31_svt_evidence_schema.json"
    )
    g31_svt_gate_schema = load(
        ROOT / "milestones/g31/g31_svt_gate_evidence_schema.json"
    )
'''
assert s.count(a1) == 1, "load anchor not unique"
s = s.replace(a1, i1)

# ── ② validator 段 ──
a2 = '''    g31_rd027_poison_guard_validator = (
        jsonschema.Draft7Validator(g31_rd027_poison_guard_schema)
        if g31_rd027_poison_guard_schema is not None
        else None
    )
'''
i2 = a2 + '''    g31_hlod_l4_validator = (
        jsonschema.Draft7Validator(g31_hlod_l4_schema)
        if g31_hlod_l4_schema is not None
        else None
    )
    g31_svt_validator = (
        jsonschema.Draft7Validator(g31_svt_schema)
        if g31_svt_schema is not None
        else None
    )
    g31_svt_gate_validator = (
        jsonschema.Draft7Validator(g31_svt_gate_schema)
        if g31_svt_gate_schema is not None
        else None
    )
'''
assert s.count(a2) == 1, "validator anchor not unique"
s = s.replace(a2, i2)

# ── ③ 前缀路由段（p4 路由之后、gpu fallthrough 之前；svt_gate 长前缀先于 harness）──
a3 = '''            validator = g31_p4_streaming_validator
        else:
            validator = gpu_validator
'''
i3 = '''            validator = g31_p4_streaming_validator
        elif (
            f.name.startswith("g31_hlod_l4_")
            and g31_hlod_l4_validator is not None
        ):
            # G31+ 波 C Task C12 HLOD L4 门裁决证据 →
            # milestones/g31/g31_hlod_l4_evidence_schema.json
            # （ci/g31_hlod_l4_smoke.py --gate g31.waveC.hlodl4 产）。
            validator = g31_hlod_l4_validator
        elif (
            f.name.startswith("g31_svt_gate_")
            and g31_svt_gate_validator is not None
        ):
            # G31+ 波 C Task C13 SVT 门裁决证据 →
            # milestones/g31/g31_svt_gate_evidence_schema.json
            # （ci/g31_svt_smoke.py --gate g31.waveC.svt 产）。
            validator = g31_svt_gate_validator
        elif (
            f.name.startswith("g31_svt_harness_")
            and g31_svt_validator is not None
        ):
            # G31+ 波 C Task C13 SVT harness 真跑证据 →
            # milestones/g31/g31_svt_evidence_schema.json。
            validator = g31_svt_validator
        else:
            validator = gpu_validator
'''
assert s.count(a3) == 1, "route anchor not unique"
s = s.replace(a3, i3)

if CRLF:
    s = s.replace("\n", "\r\n")
io.open(P, "w", encoding="utf-8", newline="").write(s)
# 驻留核验
s2 = io.open(P, encoding="utf-8", newline="").read().replace("\r\n", "\n")
for tok in ("g31_hlod_l4_schema = load(", "g31_hlod_l4_validator", 'f.name.startswith("g31_hlod_l4_")',
            "g31_svt_gate_schema = load(", 'f.name.startswith("g31_svt_gate_")',
            "g31_svt_schema = load(", 'f.name.startswith("g31_svt_harness_")'):
    assert tok in s2, f"persistence check failed: {tok}"
py_compile.compile(P, doraise=True)
print("[patch] registered 3 schemas (hlod_l4 / svt / svt_gate), persistence+compile OK")
