#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""物理库底座冒烟(步骤 88;G6.2;RFC-0017 §4.A/§4.C;验收门 G-G6-3)。

host 段(**恒跑**,纯 host 门,check_* 风格;Jolt 为 CPU 库,无 GPU 依赖):
  1. cargo 三档单测 exit 0:
       - `cargo test -p rurix-physics-sys`
       - `cargo test -p rurix-physics --no-default-features`
       - `cargo test -p rurix-physics`(default=jolt)
     并解析合并测试名单,按测试名关键字断言 RFC-0017 §4.A7 单测清单在位
     (固定步确定性/堆叠沉降/睡眠唤醒/批插体不锁死主步/query 与 step 并发/
     ContactEvent 有界 drain/SyncBudget 重置饱和)——缺失即红。
  2. §4.C4 grep 审计门(v1.2 + v1.4 收窄后判据,RFC-0017 修订记录 v1.2/v1.4
     + §9.1 末段留痕):
       - src/rurix-render/ 零 `rurix_physics_sys` 引用、零原生 Jolt/Rapier 类型名
         (JoltPhysics|JPC_|JPH::|rapier3d)——断言不变(0-byte 维持);
       - src/rurix-physics/(非 sys)公共 API 不透出 sys/原生类型(代码审计面);
         crate 内部 sys 消费收敛于 src/world.rs 单一模块——grep 判据 = 除
         src/rurix-physics/src/world.rs 外零 `rurix_physics_sys` 引用;
         原生 Jolt 类型名(JoltPhysics|JPC_|JPH::)两 crate 全禁维持
         (src/world.rs / src/rapier.rs 亦不例外);`rapier3d` 原生类型名
         (v1.4 收窄,镜像 v1.2 sys→world.rs 先例)收敛于 src/rapier.rs 单一
         sanctioned 消费模块,crate 其余文件零命中;
       - 全仓 src/ 除既有豁免白名单(rurix-rt 等 7 crate,unsafe-audit 已登记)
         与本波 rurix-physics-sys 外零新增 `unsafe_code = "allow"`;
       - src/rurix-physics-sys 内每个 unsafe 块携 `// SAFETY:` 注释(grep 级)。
  3. 写 evidence/physics_core_smoke_<ts>.json(过 ci/check_schemas.py;
     性能数字入 checks 不进硬门,P-09 / RFC-0017 §4.0-5)。

device 段:**无**(纯 host 门;G6 CI_GATES §2.88 行,CI_GATES §3 schema 同 PR 落)。

任一判据红 → 逐项打印定位后 exit 1(evidence 仍如实落盘,红不充绿)。

用法: py -3 ci/physics_core_smoke.py [--selftest]
  --selftest: 反 YAML-only 红绿自检(合成数据喂纯判定层),不跑 cargo、不写 evidence。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

# §4.A7 host 单测清单(check_key, 中文名, 测试名关键字[任一命中即在位])。
# 关键字对 cargo test 输出的测试名行(`test <path> ... ok`)合并集做大小写不敏感
# 子串匹配;七项任一缺位即红。
A7_TOPICS = [
    ("a7_fixed_step_determinism", "固定步确定性(N=100 重放全量逐位相等)", ("determin",)),
    ("a7_stack_settling", "堆叠沉降(箱塔静置收敛)", ("stack", "settl")),
    ("a7_sleep_wake", "睡眠唤醒(静置入睡 + 冲量唤醒)", ("sleep", "wake")),
    ("a7_batch_insert_no_stall", "批插体不锁死主步(prepare 交替期/finalize 单点提交)", ("batch",)),
    ("a7_query_step_concurrency", "query 与 step 并发烟测(step 外 ≥2 线程并发 cast)", ("concurren", "parallel")),
    ("a7_contact_bounded_drain", "ContactEvent 有界 drain(归一化排序 + 溢出计数)", ("contact", "drain")),
    ("a7_sync_budget_reset_saturation", "SyncBudget 每帧重置与饱和截断", ("budget", "saturat")),
]

# §4.C4 禁出面(v1.2 + v1.4 收窄,RFC-0017 修订记录 v1.2/v1.4 + §9.1 末段):
# - rurix-render 的 .rs 内零 `rurix_physics_sys` 引用(不变);
# - rurix-physics 的 .rs 内除 src/world.rs 单一消费模块外零 `rurix_physics_sys`
#   引用(safe crate 消费 sys crate = §4.0-1 Approved 架构本身,jolt 后端经
#   dep:rurix-physics-sys 实现;公共 API 不透出 sys 类型由代码审计面兜);
# - 原生 Jolt 类型名(JoltPhysics|JPC_|JPH::)两 crate 全禁维持
#   (src/world.rs 与 src/rapier.rs 亦不例外);
# - `rapier3d` 原生类型名(v1.4 收窄,镜像 v1.2 sys 消费收敛 src/world.rs
#   先例):收敛于 src/rapier.rs 单一 sanctioned 消费模块(G6.4 Rapier 快路径
#   §4.D2 Approved 架构本身),rurix-physics 其余文件与 rurix-render 全 crate
#   零命中(0-byte 维持)。
SYS_REF_RE = re.compile(r"rurix_physics_sys")
NATIVE_NAME_RE = re.compile(r"JoltPhysics|JPC_|JPH::|rapier3d")
# Jolt 原生名子集:两 crate 全禁,v1.4 收窄不豁免(src/rapier.rs 内同判红)。
NATIVE_JOLT_RE = re.compile(r"JoltPhysics|JPC_|JPH::")

# §4.C4(v1.2):rurix-physics crate 内部 sys 消费唯一 sanctioned 模块。
PHYSICS_WORLD_RS = "src/rurix-physics/src/world.rs"
# §4.C4(v1.4):`rapier3d` 原生类型名唯一 sanctioned 消费模块。
PHYSICS_RAPIER_RS = "src/rurix-physics/src/rapier.rs"

# `unsafe_code = "allow"` 既有豁免白名单(2026-07-31 全仓 src/ 普查基线;
# 各 crate unsafe-audit 注册见 unsafe-audit/*.md,AGENTS 硬规则 9 / 10 §7.6)。
UNSAFE_ALLOW_BASELINE = {
    "src/rurix-rt/Cargo.toml",               # M4 运行时 FFI 边界
    "src/rurix-rt-cabi/Cargo.toml",          # MS1.2 宿主编排 C ABI(U25)
    "src/rurix-interop/Cargo.toml",          # M8.1 CUDA/D3D12 互操作
    "src/rurix-cublas/Cargo.toml",           # M8.2 cuBLAS 绑定
    "src/rurix-engine/Cargo.toml",           # G1.3 引擎集成 cdylib
    "src/rurix-d3d12/Cargo.toml",            # G2 D3D12 shim
    "src/rurix-android-present/Cargo.toml",  # MB1 Android present(U28)
}
# 本波唯一新增豁免(RFC-0017 §4.C2,unsafe-audit/rurix-physics-sys.md U33 起)。
PHYSICS_SYS_CARGO = "src/rurix-physics-sys/Cargo.toml"
# G31+ 波 C Task C1 新增豁免(渲染器 SDK C ABI 实现层 cdylib,
# unsafe-audit/rurix-renderer-sdk.md U-59 登记;number_ledger v1.189)。
RENDERER_SDK_CARGO = "src/rurix-renderer-sdk/Cargo.toml"

CARGO_LEGS = [
    ("sys", ["cargo", "test", "-p", "rurix-physics-sys"]),
    ("physics_nodefault", ["cargo", "test", "-p", "rurix-physics", "--no-default-features"]),
    ("physics_default", ["cargo", "test", "-p", "rurix-physics"]),
]

# cargo test 输出的测试名行与通过计数行。
TEST_NAME_RE = re.compile(r"^test (\S+) \.\.\.", re.M)
TEST_OK_RE = re.compile(r"test result: ok\. (\d+) passed; 0 failed")

# evidence checks 键序(schema additionalProperties=false,须与 g6 schema 同步)。
CHECK_KEYS = (
    "sys_tests_pass", "sys_test_count",
    "physics_nodefault_tests_pass", "physics_nodefault_test_count",
    "physics_default_tests_pass", "physics_default_test_count",
    "a7_fixed_step_determinism", "a7_stack_settling", "a7_sleep_wake",
    "a7_batch_insert_no_stall", "a7_query_step_concurrency",
    "a7_contact_bounded_drain", "a7_sync_budget_reset_saturation",
    "audit_no_sys_ref", "audit_no_native_type_names",
    "audit_unsafe_allow_whitelist", "audit_safety_comments",
    "step_time_secs",
)


def _fail(msg: str) -> None:
    print(f"[physics_core_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd: Path = ROOT, timeout: int = 1800):
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout)
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


# ————————————————————— 纯判定层(selftest 直接喂合成数据)—————————————————————


def a7_presence(test_names: list[str]) -> dict[str, bool]:
    """§4.A7 清单:测试名合并集是否覆盖七主题。纯函数。"""
    blob = "\n".join(test_names).lower()
    return {key: any(kw in blob for kw in kws) for key, _label, kws in A7_TOPICS}


def audit_sys_refs(files: dict[str, str]) -> list[str]:
    """§4.C4(v1.2 收窄):rurix-render 零 `rurix_physics_sys` 引用;rurix-physics
    除 src/world.rs 唯一 sanctioned 消费模块外零引用。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        if path == PHYSICS_WORLD_RS:
            continue  # crate 内部 sys 消费收敛点(RFC-0017 修订记录 v1.2)
        for lineno, line in enumerate(text.splitlines(), 1):
            if SYS_REF_RE.search(line):
                problems.append(
                    f"{path}:{lineno}: 出现 rurix_physics_sys 引用"
                    "(§4.C4 v1.2:sys 消费收敛于 src/world.rs 单一模块,其余零引用)"
                )
    return problems


def audit_native_names(files: dict[str, str]) -> list[str]:
    """§4.C4(v1.4 收窄):零原生 Jolt/Rapier 类型名(JoltPhysics|JPC_|JPH::|rapier3d);
    唯一例外 = `rapier3d` 名收敛 src/rapier.rs 单一 sanctioned 模块(Jolt 名在该
    模块内仍全禁)。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        # v1.4:src/rapier.rs 内仅 Jolt 原生名判红,`rapier3d` 名 sanctioned
        rx = NATIVE_JOLT_RE if path == PHYSICS_RAPIER_RS else NATIVE_NAME_RE
        for lineno, line in enumerate(text.splitlines(), 1):
            m = rx.search(line)
            if m:
                problems.append(
                    f"{path}:{lineno}: 出现原生 Jolt/Rapier 类型名 {m.group(0)!r}(§4.C4 禁出)"
                )
    return problems


def audit_unsafe_allow(tomls: dict[str, str]) -> list[str]:
    """§4.C2/C4:白名单(既有 7 crate + 本波 rurix-physics-sys)外零新增
    `unsafe_code = "allow"`。纯函数。"""
    allowed = UNSAFE_ALLOW_BASELINE | {PHYSICS_SYS_CARGO, RENDERER_SDK_CARGO}
    problems: list[str] = []
    for path, text in sorted(tomls.items()):
        if re.search(r'unsafe_code\s*=\s*"allow"', text) and path not in allowed:
            problems.append(
                f'{path}: 新增 unsafe_code = "allow" 不在豁免白名单'
                "(§4.C2/C4;须先建 unsafe-audit 注册条目)"
            )
    return problems


def audit_safety_comments(files: dict[str, str]) -> list[str]:
    """sys crate 内每个 unsafe 块前/后 3 行内须有 `// SAFETY:` 注释(grep 级)。纯函数。

    grep 级已知边界:行注释先行剥离(`//` 后内容不参与 unsafe 匹配);块注释
    `/* */` 与字符串字面量内的 `//` 不特殊处理(实现 PR 若触发误报按实例校准)。

    校准(2026-07-31,实现期实例):① `unsafe extern "C" fn` 类型别名/声明/回调定义与
    `unsafe extern "C" {` 块行是 FFI **声明面**,非 unsafe 块(clippy
    undocumented_unsafe_blocks 亦不要求其携 SAFETY);其健全性义务登记于
    unsafe-audit U33~U42,声明体内的真正 `unsafe {}` 块仍受本门与 clippy 双覆盖。
    故含 `unsafe extern` 的代码行跳过。② SAFETY 注释可以是多行块(注释行数计入
    ±3 窗口会把合法注释误判):除 ±3 行窗口外,自 unsafe 行向上穿越连续注释/空行
    扫描 `// SAFETY:`(遇非注释非空代码行即止)。
    """
    problems: list[str] = []
    for path, text in sorted(files.items()):
        lines = text.splitlines()
        for i, raw in enumerate(lines):
            code = raw.split("//", 1)[0]
            if not re.search(r"\bunsafe\b", code):
                continue
            if re.search(r"\bunsafe\s+extern\b", code):
                continue
            window = lines[max(0, i - 3): i + 3]
            if any("// SAFETY:" in w for w in window):
                continue
            # 多行 SAFETY 注释:向上穿越连续注释/空行块扫描(校准 ②)
            j = i - 1
            found = False
            while j >= 0:
                stripped = lines[j].strip()
                if "// SAFETY:" in lines[j]:
                    found = True
                if stripped and not stripped.startswith("//"):
                    break
                j -= 1
            if found:
                continue
            problems.append(
                f"{path}:{i + 1}: unsafe 块缺 `// SAFETY:` 注释"
                "(§4.C2 undocumented_unsafe_blocks = deny)"
            )
    return problems


# ————————————————————— IO 采集层 —————————————————————


def collect_rs_files(dirs: list[str]) -> dict[str, str]:
    files: dict[str, str] = {}
    for d in dirs:
        base = ROOT / d
        if not base.is_dir():
            continue
        for p in sorted(base.rglob("*.rs")):
            files[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    return files


def collect_cargo_tomls() -> dict[str, str]:
    files: dict[str, str] = {}
    for p in sorted((ROOT / "src").glob("*/Cargo.toml")):
        files[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    return files


# ————————————————————— red 自检(反 YAML-only)—————————————————————


def red_self_test() -> None:
    """合成数据断言各纯判定层能区分红绿;门失效即 exit 1。"""
    full = [f"tests::{n}" for n in (
        "fixed_step_determinism_replay_100", "stack_settling_box_tower",
        "sleep_wake_impulse", "batch_insert_no_stall_main_step",
        "query_step_concurrent_cast", "contact_bounded_drain",
        "sync_budget_reset_saturation",
    )]
    if not all(a7_presence(full).values()):
        _fail("red 自检失败:全覆盖测试名单未判全绿(门失效)")
    if all(a7_presence(["tests::unrelated_thing"]).values()):
        _fail("red 自检失败:无关名单被误判全绿(门失效)")
    bad = {"src/rurix-physics/src/x.rs": "use rurix_physics_sys::SysWorld;\nlet s: JPH::State;\n"}
    if not audit_sys_refs(bad) or not audit_native_names(bad):
        _fail("red 自检失败:sys 引用/原生类型名未判红(门失效)")
    clean = {"src/rurix-render/src/y.rs": "// clean\n"}
    if audit_sys_refs(clean) or audit_native_names(clean):
        _fail("red 自检失败:干净文件被误判红(门过严)")
    # v1.2 收窄:src/world.rs 为 rurix-physics 内唯一 sanctioned sys 消费模块——
    # 其 sys 引用判绿(门过严检查),原生类型名仍判红(两 crate 全禁维持)。
    sanctioned = {PHYSICS_WORLD_RS: "use rurix_physics_sys::SysWorld;\n"}
    if audit_sys_refs(sanctioned):
        _fail("red 自检失败:src/world.rs sanctioned sys 消费被误判红(门过严,v1.2)")
    native_in_world = {PHYSICS_WORLD_RS: "use rurix_physics_sys::SysWorld;\nlet s: JPH::State;\n"}
    if not audit_native_names(native_in_world):
        _fail("red 自检失败:src/world.rs 原生类型名未判红(两 crate 全禁维持,门失效)")
    # v1.4 收窄:`rapier3d` 名收敛 src/rapier.rs 单一模块——其 rapier3d 引用判绿
    # (门过严检查);Jolt 名在 src/rapier.rs 内仍判红;rapier3d 名在 src/world.rs、
    # rurix-physics 其余文件与 rurix-render 仍判红(0-byte 维持,门失效检查)。
    sanctioned_rapier = {PHYSICS_RAPIER_RS: "use rapier3d::dynamics::RigidBodySet;\n"}
    if audit_native_names(sanctioned_rapier):
        _fail("red 自检失败:src/rapier.rs sanctioned rapier3d 消费被误判红(门过严,v1.4)")
    jolt_in_rapier = {PHYSICS_RAPIER_RS: "use rapier3d::prelude::*;\nlet s: JPH::State;\n"}
    if not audit_native_names(jolt_in_rapier):
        _fail("red 自检失败:src/rapier.rs 内 Jolt 原生名未判红(Jolt 名全禁维持,门失效)")
    rapier_in_world = {PHYSICS_WORLD_RS: "use rapier3d::dynamics::RigidBodySet;\n"}
    if not audit_native_names(rapier_in_world):
        _fail("red 自检失败:src/world.rs 内 rapier3d 名未判红(豁免仅 src/rapier.rs,门失效)")
    rapier_in_render = {"src/rurix-render/src/y.rs": "let _ = core::mem::size_of::<rapier3d::math::Vector>();\n"}
    if not audit_native_names(rapier_in_render):
        _fail("red 自检失败:rurix-render 内 rapier3d 名未判红(render 0-byte 维持,门失效)")
    if not audit_unsafe_allow({"src/rurix-foo/Cargo.toml": 'unsafe_code = "allow"\n'}):
        _fail("red 自检失败:白名单外 unsafe_code=allow 未判红(门失效)")
    if audit_unsafe_allow({
        "src/rurix-rt/Cargo.toml": 'unsafe_code = "allow"\n',
        PHYSICS_SYS_CARGO: 'unsafe_code = "allow"\n',
    }):
        _fail("red 自检失败:白名单内豁免被误判红(门过严)")
    no_safety = {"src/rurix-physics-sys/src/z.rs": "let p = unsafe { core::ptr::read(q) };\n"}
    if not audit_safety_comments(no_safety):
        _fail("red 自检失败:缺 SAFETY 注释的 unsafe 未判红(门失效)")
    with_safety = {"src/rurix-physics-sys/src/z.rs": (
        "// SAFETY: q 生命周期覆盖本块\n"
        "let p = unsafe { core::ptr::read(q) };\n"
        "// unsafe impl Send 占位注释行\n"
    )}
    if audit_safety_comments(with_safety):
        _fail("red 自检失败:有 SAFETY 注释被误判红 / 注释行 unsafe 被误捕(门过严)")
    extern_decl = {"src/rurix-physics-sys/src/z.rs": (
        "pub type Cb = unsafe extern \"C\" fn(*const c_void);\n"
        "unsafe extern \"C\" fn on_event(p: *const c_void) {\n"
        "    // SAFETY: p 指向调用帧栈上有效状态\n"
        "    let s = unsafe { &*p.cast::<u64>() };\n"
        "}\n"
    )}
    if audit_safety_comments(extern_decl):
        _fail("red 自检失败:unsafe extern 声明面被误判红(校准失效)")
    multiline_safety = {"src/rurix-physics-sys/src/z.rs": (
        "// SAFETY: 多行注释首行——句柄独占拥有、未提前释放;\n"
        "// 销毁序 = 摘除监听器 → 系统 → shape 引用 → 过滤器,\n"
        "// 逐行论证占四行,超出 ±3 窗口仍应判绿。\n"
        "// 末行。\n"
        "unsafe {\n"
        "    destroy(p);\n"
        "}\n"
    )}
    if audit_safety_comments(multiline_safety):
        _fail("red 自检失败:多行 SAFETY 注释块被误判红(校准 ② 失效)")
    far_safety = {"src/rurix-physics-sys/src/z.rs": (
        "// SAFETY: 隔了代码行的注释不算数\n"
        "let x = 1;\n"
        "let y = 2;\n"
        "let z = 3;\n"
        "let w = 4;\n"
        "unsafe {\n"
        "    destroy(p);\n"
        "}\n"
    )}
    if not audit_safety_comments(far_safety):
        _fail("red 自检失败:被代码行隔断的 SAFETY 注释未判红(校准 ② 过宽)")


# ————————————————————— 检查段 —————————————————————


def cargo_section(results: dict, failures: list[str]) -> bool:
    """判据 (a):cargo 三档单测 exit 0 + §4.A7 单测清单在位。"""
    all_names: list[str] = []
    ok = True
    for leg, cmd in CARGO_LEGS:
        try:
            code, out, err = run(cmd)
        except FileNotFoundError:
            results[f"{leg}_tests_pass"] = False
            failures.append(f"cargo 段: cargo 不在 PATH({leg} 档未能执行)")
            ok = False
            continue
        blob = out + err
        counts = TEST_OK_RE.findall(blob)
        results[f"{leg}_test_count"] = sum(int(x) for x in counts)
        all_names += TEST_NAME_RE.findall(blob)
        if code != 0:
            print(f"[physics_core_smoke] cargo 段 {leg} 档输出尾部:", file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            results[f"{leg}_tests_pass"] = False
            failures.append(f"cargo 段: `{' '.join(cmd)}` exit {code}(构建/单测红)")
            ok = False
        else:
            results[f"{leg}_tests_pass"] = True
        print(
            f"[physics_core_smoke] cargo 段 {leg}: rc={code}, "
            f"全过计数={results[f'{leg}_test_count']}"
        )
    presence = a7_presence(all_names)
    for key, label, _kws in A7_TOPICS:
        results[key] = presence[key]
        if not presence[key]:
            failures.append(f"§4.A7 单测清单缺位: {label}(三档 cargo 输出测试名未命中关键字)")
    return ok and all(presence.values())


def audit_section(results: dict, failures: list[str]) -> bool:
    """判据 (b):§4.C4 grep 审计门(v1.2 + v1.4 收窄后判据,RFC-0017 修订记录 v1.2/v1.4)。"""
    scoped = collect_rs_files(["src/rurix-render", "src/rurix-physics"])
    checks = [
        ("audit_no_sys_ref", audit_sys_refs(scoped)),
        ("audit_no_native_type_names", audit_native_names(scoped)),
        ("audit_unsafe_allow_whitelist", audit_unsafe_allow(collect_cargo_tomls())),
        ("audit_safety_comments", audit_safety_comments(collect_rs_files(["src/rurix-physics-sys"]))),
    ]
    ok = True
    for key, problems in checks:
        results[key] = not problems
        for p in problems:
            failures.append(f"§4.C4 审计门: {p}")
        if problems:
            ok = False
        print(
            f"[physics_core_smoke] 审计 {key}: "
            f"{'PASS' if not problems else f'RED({len(problems)} 处)'}"
        )
    return ok


def write_evidence(results: dict, host_ok: bool) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "physics_core_smoke",
        "milestone": "G6.2 / G-G6-3 (RFC-0017 §4.A/§4.C)",
        "step": 88,
        "host_section_pass": host_ok,
        # 纯 host 门,无 device 段(G6 CI_GATES §2.88)→ null。
        "device_section_rc": None,
        "checks": {k: results.get(k) for k in CHECK_KEYS if results.get(k) is not None},
        "physics_core_smoke_ok": host_ok,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"physics_core_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[physics_core_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    if "--selftest" in sys.argv:
        red_self_test()
        print("[physics_core_smoke] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")
        return 0
    results: dict = {}
    failures: list[str] = []
    cargo_ok = cargo_section(results, failures)
    audits_ok = audit_section(results, failures)
    host_ok = cargo_ok and audits_ok
    write_evidence(results, host_ok)
    if failures:
        print("[physics_core_smoke] FAIL 判据红清单:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("[physics_core_smoke] PASS(host 恒跑,纯 host 门)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
