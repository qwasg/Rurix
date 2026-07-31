#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""渲染合流桥冒烟(步骤 89;G6.3;RFC-0017 §4.B;验收门 G-G6-4)。

host 段(**恒跑**,check_* 风格;bridge 为纯 host 类型面):
  1. cargo 两档单测 exit 0:
       - `cargo test -p rurix-physics`(default=jolt,54 项含 bridge 七测试)
       - `cargo test -p rurix-physics --no-default-features`(41 项)
     并解析合并测试名单,按测试名关键字断言 §4.B bridge 七行为测试在位
     (单向同步只写 active 动态体/睡眠体零写零 MV/预算截断确定性/flush 脏区间
     ==脏实例/motion hint prev-cur 差分/流送驻留批插+卸载 receipt/卸载竞态
     注入无悬垂)——缺失即红(反 YAML-only)。
  2. §4.B 机器可核面 grep 审计门(全部纯函数判定):
       ① render_no_physics_backref:src/rurix-render/**.rs 零
         `rurix_physics`/`rurix-physics` 引用(§4.B1-1 单向事实源:渲染器
         只读消费 GpuScene,永不回写物理);
       ② render_no_native_physics:src/rurix-render/**.rs 零
         `JoltPhysics|JPC_|JPH::|rapier3d|rurix_physics_sys`(渲染侧不持
         原生物理指针/类型,§4.C4 面向 render 的延伸);
       ③ bridge_no_as_api:src/rurix-physics/src/bridge/**.rs 零
         `TlasBuilder|BlasCache|TriBvh|as_manager|DynamicPolicy|temporal`
         (§4.B5:物理不新建 AS 所有者、不私写时域——只供脏信号/MV 提示,
         交 G5 既有 refit 决策树与时域栈消费);
       ④ receipt_type_discipline:src/rurix-physics/src/bridge/streaming.rs
         中 `RemovalReceipt` 无 pub 构造器、derive 不含 Clone/Copy,且
         `remove_page` 是其唯一产出口(§4.B4 先卸 body 再放页,移动语义
         单次消耗;grep 启发式,口径见 audit_receipt_discipline 文档)。
  3. 写 evidence/physics_bridge_smoke_<ts>.json(过 ci/check_schemas.py;
     计数数字入 checks 不进硬门,P-09 / RFC-0017 §4.0-5)。

device 段(**gate real**:Vulkan 在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则
SKIP=dev-env-degrade 退 0 不充绿,镜像 uc06 双态先例):
  4. `RURIX_REQUIRE_REAL=1 cargo run -p uc08-physics --features vulkan --
     --device --frames 4 --size 64x64 --json`——uc08 device 腿真跑,exit 0 且
     JSON device 段 `device_pixels_nontrivial==true` 且
     `device_motion_pixels_changed==true`(物理驱动变换 → readback 两帧像素
     非平凡 + 运动像素变化;对拍类字段非 true 永远硬红,禁止降级)。

任一判据红 → 逐项打印定位后 exit 1(evidence 仍如实落盘,红不充绿)。

用法: py -3 ci/physics_bridge_smoke.py [--selftest]
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

# §4.B bridge 七行为测试清单(check_key, 中文名, 测试名关键字[任一命中即在位])。
# 关键字对 cargo test 输出的测试名行(`test <path> ... ok`)合并集做大小写不敏感
# 子串匹配;七项任一缺位即红。七测试全在 tests/bridge.rs(jolt 档)。
BRIDGE_TESTS = [
    ("bridge_test_one_way_sync", "单向同步只写 active 动态体(§4.B1-1/§4.B2)",
     ("one_way_sync_writes_active_dynamic_only",)),
    ("bridge_test_sleeping_zero_write", "睡眠体零写零 MV(§4.A3/§4.B2/B3)",
     ("sleeping_body_zero_write_zero_mv",)),
    ("bridge_test_budget_truncation", "SyncBudget 截断确定性(§4.A6)",
     ("budget_truncation_deterministic",)),
    ("bridge_test_flush_dirty_ranges", "flush 脏区间==脏实例(§4.B5 同帧同源)",
     ("flush_dirty_ranges_match_dirty_instances",)),
    ("bridge_test_motion_hint", "motion hint prev/cur 差分(§4.B3)",
     ("motion_hint_tracks_prev_cur",)),
    ("bridge_test_streaming_receipt", "流送驻留批插+卸载 receipt(§4.B4)",
     ("streaming_insert_on_residency_and_remove_receipt",)),
    ("bridge_test_unload_race", "卸载竞态注入无悬垂(§4.B4)",
     ("unload_race_injection_no_dangling",)),
]

# 审计禁出面(§4.B 机器可核面):
# - ① render 零 rurix_physics/rurix-physics 引用(渲染器不回写物理,§4.B1-1);
# - ② render 零原生 Jolt/Rapier 类型名 + 零 sys crate(不持原生指针,§4.C4 延伸);
# - ③ bridge 零 AS 所有者/时域私写 API(§4.B5:物理只供脏信号/MV 提示)。
RENDER_PHYSICS_BACKREF_RE = re.compile(r"rurix[_-]physics")
RENDER_NATIVE_PHYSICS_RE = re.compile(r"JoltPhysics|JPC_|JPH::|rapier3d|rurix_physics_sys")
BRIDGE_AS_API_RE = re.compile(r"TlasBuilder|BlasCache|TriBvh|as_manager|DynamicPolicy|temporal")

# ④ receipt 纪律的审计文件(§4.B4 唯一 sanctioned 产出口所在)。
STREAMING_RS = "src/rurix-physics/src/bridge/streaming.rs"

CARGO_LEGS = [
    ("physics_default", ["cargo", "test", "-p", "rurix-physics"]),
    ("physics_nodefault", ["cargo", "test", "-p", "rurix-physics", "--no-default-features"]),
]

# cargo test 输出的测试名行与通过计数行。
TEST_NAME_RE = re.compile(r"^test (\S+) \.\.\.", re.M)
TEST_OK_RE = re.compile(r"test result: ok\. (\d+) passed; 0 failed")

# evidence checks 键序(schema additionalProperties=false,须与 g6 schema 同步)。
CHECK_KEYS = (
    "physics_default_tests_pass", "physics_default_test_count",
    "physics_nodefault_tests_pass", "physics_nodefault_test_count",
    "bridge_test_one_way_sync", "bridge_test_sleeping_zero_write",
    "bridge_test_budget_truncation", "bridge_test_flush_dirty_ranges",
    "bridge_test_motion_hint", "bridge_test_streaming_receipt",
    "bridge_test_unload_race",
    "audit_render_no_physics_backref", "audit_render_no_native_physics",
    "audit_bridge_no_as_api", "audit_receipt_type_discipline",
    "device_run_pass", "device_name", "device_pixels_a", "device_pixels_b",
    "device_changed_pixels", "device_pixels_nontrivial",
    "device_motion_pixels_changed",
)


def _fail(msg: str) -> None:
    print(f"[physics_bridge_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd: Path = ROOT, timeout: int = 1800, env_extra: dict | None = None):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout, env=env)
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


def bridge_presence(test_names: list[str]) -> dict[str, bool]:
    """§4.B 清单:测试名合并集是否覆盖 bridge 七行为测试。纯函数。"""
    blob = "\n".join(test_names).lower()
    return {key: any(kw in blob for kw in kws) for key, _label, kws in BRIDGE_TESTS}


def audit_render_no_physics_backref(files: dict[str, str]) -> list[str]:
    """① §4.B1-1:src/rurix-render/**.rs 零 rurix_physics/rurix-physics 引用
    (渲染器不回写物理,依赖方向 = rurix-physics → rurix-render 单向)。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        for lineno, line in enumerate(text.splitlines(), 1):
            if RENDER_PHYSICS_BACKREF_RE.search(line):
                problems.append(
                    f"{path}:{lineno}: 渲染侧出现 rurix_physics/rurix-physics 引用"
                    "(§4.B1-1 单向事实源:render 永不反向依赖物理)"
                )
    return problems


def audit_render_no_native_physics(files: dict[str, str]) -> list[str]:
    """② §4.C4 延伸:src/rurix-render/**.rs 零原生 Jolt/Rapier 类型名与 sys crate
    (JoltPhysics|JPC_|JPH::|rapier3d|rurix_physics_sys;不持原生物理指针)。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        for lineno, line in enumerate(text.splitlines(), 1):
            m = RENDER_NATIVE_PHYSICS_RE.search(line)
            if m:
                problems.append(
                    f"{path}:{lineno}: 渲染侧出现原生物理类型名 {m.group(0)!r}"
                    "(§4.C4 禁出面向 render 延伸;不持原生指针)"
                )
    return problems


def audit_bridge_no_as_api(files: dict[str, str]) -> list[str]:
    """③ §4.B5:bridge/**.rs 零 AS 所有者/时域私写 API
    (TlasBuilder|BlasCache|TriBvh|as_manager|DynamicPolicy|temporal;
    物理只供脏信号/MV 提示,不新建加速结构所有者、不私写时域)。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        for lineno, line in enumerate(text.splitlines(), 1):
            m = BRIDGE_AS_API_RE.search(line)
            if m:
                problems.append(
                    f"{path}:{lineno}: bridge 出现 AS/时域 API 名 {m.group(0)!r}"
                    "(§4.B5:不新建 AS 所有者、不私写时域,信号交 G5 既有决策树消费)"
                )
    return problems


def audit_receipt_discipline(text: str) -> list[str]:
    """④ §4.B4 RemovalReceipt 类型纪律(grep 启发式;先卸 body 再放页)。纯函数。

    口径(启发式边界,注释即判据):
    - 只看**非测试前缀**(首个 `#[cfg(test)]` 截断):tests 模块内的 receipt
      字面量构造是「编译期不可伪造的测试侧镜像」(streaming.rs 注释自述),放行;
    - derive 检查:取 `pub struct RemovalReceipt` 行**紧邻上方**的连续 `#[...]`
      属性行集,其中不得含 `Clone`/`Copy`(移动语义单次消耗);
    - 无 pub 构造器:`impl RemovalReceipt { ... }` 块内零返回 `Self`/
      `RemovalReceipt` 的 `pub fn`(含 `pub fn new`;访问器返回引用/PageKey 放行);
    - 唯一产出口:非测试前缀内,返回类型提及 `RemovalReceipt` 的 `pub fn` 集合
      恰为 {remove_page};且字面量构造点 `RemovalReceipt {`(排除 struct 声明与
      impl 块头)恰 1 处、且位于 `fn remove_page` 之后(即其函数体内)。
    """
    problems: list[str] = []
    head = text.split("#[cfg(test)]", 1)[0]
    lines = head.splitlines()
    struct_idx = next(
        (i for i, line in enumerate(lines) if "pub struct RemovalReceipt" in line), None
    )
    if struct_idx is None:
        return ["streaming.rs: 未找到 pub struct RemovalReceipt 声明(文件结构漂移,门需校准)"]
    attrs: list[str] = []
    j = struct_idx - 1
    while j >= 0 and lines[j].strip().startswith("#["):
        attrs.append(lines[j])
        j -= 1
    if re.search(r"\b(Clone|Copy)\b", "\n".join(attrs)):
        problems.append(
            "RemovalReceipt derive 含 Clone/Copy(§4.B4 移动语义单次消耗纪律破坏)"
        )
    impl_m = re.search(r"impl RemovalReceipt \{(.*?)\n\}", head, re.S)
    if impl_m is None:
        problems.append("streaming.rs: 未找到 impl RemovalReceipt 块(文件结构漂移,门需校准)")
    else:
        for fm in re.finditer(r"pub fn (\w+)[^;{]*?->\s*([^;{]+?)\s*\{", impl_m.group(1)):
            ret = fm.group(2)
            if "Self" in ret or "RemovalReceipt" in ret:
                problems.append(
                    f"impl RemovalReceipt::pub fn {fm.group(1)} 返回 {ret.strip()}"
                    "(§4.B4:无 pub 构造器,receipt 编译期不可伪造)"
                )
    producers = [
        fm.group(1)
        for fm in re.finditer(r"pub fn (\w+)\s*\([^;{]*?->\s*([^;{]+?)\s*\{", head)
        if "RemovalReceipt" in fm.group(2)
    ]
    if producers != ["remove_page"]:
        problems.append(
            f"RemovalReceipt 产出口集合 {producers} != ['remove_page']"
            "(§4.B4:remove_page 是唯一产出口)"
        )
    ctor_pos = [
        m.start()
        for m in re.finditer(r"RemovalReceipt\s*\{", head)
        if "pub struct" not in head[max(0, m.start() - 40): m.start()]
        and not re.search(r"\bimpl\s*$", head[max(0, m.start() - 40): m.start()])
    ]
    rp = head.find("fn remove_page")
    if rp < 0 or len(ctor_pos) != 1 or ctor_pos[0] < rp:
        problems.append(
            f"RemovalReceipt 字面量构造点 {len(ctor_pos)} 处(期望恰 1 处且在 "
            "fn remove_page 体内;§4.B4 唯一产出口)"
        )
    return problems


def judge_uc08_device_doc(doc: dict | None) -> tuple[bool, list[str]]:
    """device 段 JSON 判定:device 段非空 + 两对拍布尔 true + device_name 非空。
    对拍类字段非 true 永远硬红(禁止降级)。纯函数。"""
    if not isinstance(doc, dict):
        return False, ["uc08 --device JSON 解析失败"]
    dev = doc.get("device")
    if not isinstance(dev, dict):
        return False, ["JSON device 字段缺席(device_requested=true 须真跑)"]
    problems: list[str] = []
    for key in ("device_pixels_nontrivial", "device_motion_pixels_changed"):
        if dev.get(key) is not True:
            problems.append(
                f"device.{key} != true(={dev.get(key)!r};物理驱动变换 → readback "
                "像素/运动非平凡断言,对拍类非 true 永远硬红)"
            )
    if not dev.get("device_name"):
        problems.append("device.device_name 空(device 真跑须实名留证)")
    return (not problems), problems


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


def parse_uc08_json(out: str) -> dict | None:
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}") and '"subject":"uc08_physics"' in line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                return None
    return None


# ————————————————————— red 自检(反 YAML-only)—————————————————————

_RECEIPT_OK = """\
#[derive(Debug)]
pub struct RemovalReceipt {
    page: PageKey,
    bodies: Vec<BodyId>,
}
impl RemovalReceipt {
    pub fn page(&self) -> PageKey {
        self.page
    }
    pub fn removed_bodies(&self) -> &[BodyId] {
        &self.bodies
    }
}
impl StreamingBridge {
    pub fn remove_page(
        &mut self,
        page: PageKey,
    ) -> Result<RemovalReceipt, PhysicsError> {
        let bodies = vec![];
        Ok(RemovalReceipt { page, bodies })
    }
}
#[cfg(test)]
mod tests {
    fn receipt_mirror() {
        let receipt = RemovalReceipt { page, bodies };
    }
}
"""


def red_self_test() -> None:
    """合成数据断言各纯判定层能区分红绿;门失效即 exit 1。"""
    full = [f"bridge::tests::{n}" for n in (
        "one_way_sync_writes_active_dynamic_only", "sleeping_body_zero_write_zero_mv",
        "budget_truncation_deterministic", "flush_dirty_ranges_match_dirty_instances",
        "motion_hint_tracks_prev_cur", "streaming_insert_on_residency_and_remove_receipt",
        "unload_race_injection_no_dangling",
    )]
    if not all(bridge_presence(full).values()):
        _fail("red 自检失败:全覆盖测试名单未判全绿(门失效)")
    if all(bridge_presence(full[:-1]).values()):
        _fail("red 自检失败:缺一项名单被误判全绿(门失效)")
    if all(bridge_presence(["tests::unrelated_thing"]).values()):
        _fail("red 自检失败:无关名单被误判全绿(门失效)")
    backref = {"src/rurix-render/src/x.rs": "use rurix_physics::PhysicsBridge;\n"}
    if not audit_render_no_physics_backref(backref):
        _fail("red 自检失败:render 反向依赖物理未判红(门失效)")
    backref_dash = {"src/rurix-render/src/x.rs": "// see rurix-physics crate\n"}
    if not audit_render_no_physics_backref(backref_dash):
        _fail("red 自检失败:render rurix-physics 连写未判红(门失效)")
    native = {"src/rurix-render/src/y.rs": "let s: JPH::State;\n"}
    if not audit_render_no_native_physics(native):
        _fail("red 自检失败:render 原生类型名未判红(门失效)")
    clean = {"src/rurix-render/src/z.rs": "// clean render code\n"}
    if audit_render_no_physics_backref(clean) or audit_render_no_native_physics(clean):
        _fail("red 自检失败:干净 render 文件被误判红(门过严)")
    as_api = {"src/rurix-physics/src/bridge/mod.rs": "let t = TlasBuilder::new();\n"}
    if not audit_bridge_no_as_api(as_api):
        _fail("red 自检失败:bridge AS 所有者 API 未判红(门失效)")
    as_temporal = {"src/rurix-physics/src/bridge/mod.rs": "// temporal reprojection\n"}
    if not audit_bridge_no_as_api(as_temporal):
        _fail("red 自检失败:bridge temporal 私写未判红(门失效)")
    clean_bridge = {"src/rurix-physics/src/bridge/mod.rs": "// clean bridge code\n"}
    if audit_bridge_no_as_api(clean_bridge):
        _fail("red 自检失败:干净 bridge 文件被误判红(门过严)")
    if audit_receipt_discipline(_RECEIPT_OK):
        _fail("red 自检失败:合法 receipt 纪律被误判红(门过严)")
    clone_derive = _RECEIPT_OK.replace("#[derive(Debug)]", "#[derive(Debug, Clone)]")
    if not audit_receipt_discipline(clone_derive):
        _fail("red 自检失败:receipt derive Clone 未判红(门失效)")
    pub_new = _RECEIPT_OK.replace(
        "pub fn page(&self) -> PageKey {",
        "pub fn new(page: PageKey) -> Self {\n        Self { page, bodies: vec![] }\n    }\n    pub fn page(&self) -> PageKey {",
    )
    if not audit_receipt_discipline(pub_new):
        _fail("red 自检失败:receipt pub 构造器未判红(门失效)")
    second_producer = _RECEIPT_OK.replace(
        "#[cfg(test)]",
        "impl StreamingBridge {\n    pub fn forge(&self, page: PageKey) -> RemovalReceipt {\n"
        "        RemovalReceipt { page, bodies: vec![] }\n    }\n}\n#[cfg(test)]",
    )
    if not audit_receipt_discipline(second_producer):
        _fail("red 自检失败:receipt 第二产出口未判红(门失效)")
    good_doc = {"device": {
        "device_name": "RTX", "pixels_a": 100, "pixels_b": 100,
        "changed_pixels": 50, "device_pixels_nontrivial": True,
        "device_motion_pixels_changed": True,
    }}
    ok, probs = judge_uc08_device_doc(good_doc)
    if not ok or probs:
        _fail("red 自检失败:合法 device JSON 被误判红(门过严)")
    bad_doc = {"device": {
        "device_name": "RTX", "device_pixels_nontrivial": True,
        "device_motion_pixels_changed": False,
    }}
    ok, _probs = judge_uc08_device_doc(bad_doc)
    if ok:
        _fail("red 自检失败:device 对拍 false 未判红(门失效)")
    ok, _probs = judge_uc08_device_doc({"device": None})
    if ok:
        _fail("red 自检失败:device 缺席未判红(门失效)")


# ————————————————————— 检查段 —————————————————————


def skip(msg: str, failures: list[str]) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        failures.append(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
        return 1
    print(f"[physics_bridge_smoke] SKIP {msg}(dev-env-degrade,退出 0 不充绿)")
    return 0


def cargo_section(results: dict, failures: list[str]) -> bool:
    """判据 (a):cargo 两档单测 exit 0 + §4.B bridge 七测试清单在位。"""
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
            print(f"[physics_bridge_smoke] cargo 段 {leg} 档输出尾部:", file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            results[f"{leg}_tests_pass"] = False
            failures.append(f"cargo 段: `{' '.join(cmd)}` exit {code}(构建/单测红)")
            ok = False
        else:
            results[f"{leg}_tests_pass"] = True
        print(
            f"[physics_bridge_smoke] cargo 段 {leg}: rc={code}, "
            f"全过计数={results[f'{leg}_test_count']}"
        )
    presence = bridge_presence(all_names)
    for key, label, _kws in BRIDGE_TESTS:
        results[key] = presence[key]
        if not presence[key]:
            failures.append(f"§4.B bridge 测试清单缺位: {label}(cargo 输出测试名未命中关键字)")
    return ok and all(presence.values())


def audit_section(results: dict, failures: list[str]) -> bool:
    """判据 (b):§4.B 机器可核面 grep 审计门(四项,纯函数判定)。"""
    render_files = collect_rs_files(["src/rurix-render"])
    bridge_files = collect_rs_files(["src/rurix-physics/src/bridge"])
    streaming_path = ROOT / STREAMING_RS
    streaming_text = (
        streaming_path.read_text(encoding="utf-8") if streaming_path.is_file() else ""
    )
    checks = [
        ("audit_render_no_physics_backref", audit_render_no_physics_backref(render_files)),
        ("audit_render_no_native_physics", audit_render_no_native_physics(render_files)),
        ("audit_bridge_no_as_api", audit_bridge_no_as_api(bridge_files)),
        ("audit_receipt_type_discipline", audit_receipt_discipline(streaming_text)),
    ]
    ok = True
    for key, problems in checks:
        results[key] = not problems
        for p in problems:
            failures.append(f"§4.B 审计门: {p}")
        if problems:
            ok = False
        print(
            f"[physics_bridge_smoke] 审计 {key}: "
            f"{'PASS' if not problems else f'RED({len(problems)} 处)'}"
        )
    return ok


def device_section(results: dict, failures: list[str]) -> int:
    """判据 (c):uc08 device 腿真跑(gate real)——物理驱动变换 → readback 像素
    非平凡 + 运动像素变化。SKIP=dev-env-degrade 退 0 不充绿,REQUIRE_REAL 翻硬红。"""
    try:
        code, out, err = run(
            ["cargo", "run", "-q", "-p", "uc08-physics", "--features", "vulkan",
             "--", "--device", "--frames", "4", "--size", "64x64", "--json"],
            env_extra={"RURIX_REQUIRE_REAL": "1"}, timeout=1800,
        )
    except FileNotFoundError:
        results["device_run_pass"] = False
        failures.append("device 段: cargo 不在 PATH(uc08 device 腿未能执行)")
        return 1
    doc = parse_uc08_json(out)
    if code != 0 or doc is None:
        blob = out + err
        if "no-vulkan" in blob.lower() or "vulkan loader" in blob.lower() or "SKIP" in blob:
            results["device_run_pass"] = "SKIP"
            results["toolchain_skip"] = "no-vulkan"
            return skip("device 段:无 Vulkan loader(device 真跑归 gate real;host 段已恒跑)", failures)
        print(f"[physics_bridge_smoke] device 段输出尾部:", file=sys.stderr)
        print(blob[-2400:], file=sys.stderr)
        results["device_run_pass"] = False
        failures.append(f"device 段: uc08-physics --device 未过(rc={code},JSON 解析={'ok' if doc else '失败'})")
        return 1
    dev = doc.get("device")
    if dev is None:
        blob = out + err
        if any(k in blob.lower() for k in ("no-vulkan", "vulkan loader", "degrade", "降级")):
            results["device_run_pass"] = "SKIP"
            results["toolchain_skip"] = "no-vulkan"
            return skip("device 段: uc08 device 腿降级(dev-env degrade,不充绿)", failures)
        results["device_run_pass"] = False
        failures.append("device 段: JSON device 字段缺席且非降级(device_requested=true 须真跑)")
        return 1
    ok, problems = judge_uc08_device_doc(doc)
    results["device_run_pass"] = ok
    results["device_name"] = dev.get("device_name")
    results["device_pixels_a"] = dev.get("pixels_a")
    results["device_pixels_b"] = dev.get("pixels_b")
    results["device_changed_pixels"] = dev.get("changed_pixels")
    results["device_pixels_nontrivial"] = dev.get("device_pixels_nontrivial")
    results["device_motion_pixels_changed"] = dev.get("device_motion_pixels_changed")
    for p in problems:
        failures.append(f"device 段: {p}")
    if ok:
        print(
            f"[physics_bridge_smoke] device 段 PASS: {dev.get('device_name')} "
            f"pixels_a={dev.get('pixels_a')} pixels_b={dev.get('pixels_b')} "
            f"changed={dev.get('changed_pixels')}"
        )
    return 0 if ok else 1


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("device_run_pass") == "SKIP" or results.get("toolchain_skip") is not None
    # mock/SKIP 不充绿:_ok 要求 host 全绿且 device 段真跑判绿。
    subject_ok = host_ok and results.get("device_run_pass") is True
    doc = {
        "schema_version": 1,
        "subject": "physics_bridge_smoke",
        "milestone": "G6.3 / G-G6-4 (RFC-0017 §4.B)",
        "step": 89,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {k: results.get(k) for k in CHECK_KEYS if results.get(k) is not None},
        "physics_bridge_smoke_ok": subject_ok,
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"physics_bridge_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[physics_bridge_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    if "--selftest" in sys.argv:
        red_self_test()
        print("[physics_bridge_smoke] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")
        return 0
    results: dict = {}
    failures: list[str] = []
    cargo_ok = cargo_section(results, failures)
    audits_ok = audit_section(results, failures)
    host_ok = cargo_ok and audits_ok
    device_rc = device_section(results, failures) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if failures:
        print("[physics_bridge_smoke] FAIL 判据红清单:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    if device_rc != 0:
        return device_rc
    print("[physics_bridge_smoke] PASS(host 恒跑 + device gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
