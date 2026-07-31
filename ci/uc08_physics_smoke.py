#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-08 物理合流 demo 冒烟(步骤 91;G6.3;RFC-0017 §4.B;验收门 G-G6-7)。

host 段(**恒跑**,无 GPU;host 全跑 ~155s,subprocess timeout 1800s 给足):
  1. `cargo test -p uc08-physics` exit 0(11 项,记录 passed 数);
  2. `cargo run -p uc08-physics -- --frames 96 --size 128x72 --json` exit 0,
     解析单行 JSON(subject=="uc08_physics"):
       - 16 断言字段全部在位且全 true(physics_step_measured/transform_landed/
         mv_dynamic_present_early/mv_zero_after_sleep/streaming_insert_seen/
         streaming_remove_receipt_seen/release_after_receipt_only/
         tlas_rebuilds_ge1/blas_static_zero_refit/final_image_nontrivial/
         shading_has_contrast/temporal_converges/sleep_reached_when_long_run/
         pso_zero_warnings/graph_fences_nonempty/graph_alias_saves)——
         缺位即红(反 YAML-only),非 true 即红;
       - physics.total_step_ms 为**正值 measured 数字** + stages 含 physics
         阶段正耗时(物理步耗时 measured;P-09:数字写入 checks 留证,
         **不做阈值断言、不进硬门**);
       - streaming/mv/transform 数据字段存在性核验(streaming.insert_frame/
         remove_frame/receipt_bodies/releases,mv.early_max/post_sleep_max,
         physics.transform_landed_max_err;防 YAML-only 空壳绿)。

device 段(**gate real**:Vulkan 在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则
SKIP=dev-env-degrade 退 0 不充绿,镜像 uc06 双态先例):
  3. `RURIX_REQUIRE_REAL=1 cargo run -p uc08-physics --features vulkan --
     --device --frames 4 --size 64x64 --json`——uc08 device 腿真跑,exit 0 且
     JSON device 段 `device_pixels_nontrivial==true` 且
     `device_motion_pixels_changed==true`(物理驱动变换 → readback 两帧像素
     非平凡 + 运动像素变化;对拍类字段非 true 永远硬红,禁止降级)。

任一判据红 → 逐项打印定位后 exit 1(evidence 仍如实落盘,红不充绿)。

用法: py -3 ci/uc08_physics_smoke.py [--selftest]
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

# uc08 --json 单行 JSON 的 16 断言字段(冻结面,apps/uc08-physics/src/pipeline.rs
# summary_json;缺位/非 true 即红)。
EXPECTED_ASSERTS = (
    "physics_step_measured", "transform_landed", "mv_dynamic_present_early",
    "mv_zero_after_sleep", "streaming_insert_seen", "streaming_remove_receipt_seen",
    "release_after_receipt_only", "tlas_rebuilds_ge1", "blas_static_zero_refit",
    "final_image_nontrivial", "shading_has_contrast", "temporal_converges",
    "sleep_reached_when_long_run", "pso_zero_warnings", "graph_fences_nonempty",
    "graph_alias_saves",
)

# cargo test 输出的通过计数行。
TEST_OK_RE = re.compile(r"test result: ok\. (\d+) passed; 0 failed")

# evidence checks 键序(schema additionalProperties=false,须与 g6 schema 同步)。
CHECK_KEYS = (
    "uc08_tests_pass", "uc08_test_count",
    "host_run_exit_ok", "host_json_exit_ok", "asserts_all_true",
    "physics_step_measured", "transform_landed", "mv_dynamic_present_early",
    "mv_zero_after_sleep", "streaming_insert_seen", "streaming_remove_receipt_seen",
    "release_after_receipt_only", "tlas_rebuilds_ge1", "blas_static_zero_refit",
    "final_image_nontrivial", "shading_has_contrast", "temporal_converges",
    "sleep_reached_when_long_run", "pso_zero_warnings", "graph_fences_nonempty",
    "graph_alias_saves",
    "physics_step_ms", "physics_stage_present",
    "fields_streaming_present", "fields_mv_present", "fields_transform_present",
    "device_run_pass", "device_name", "device_pixels_a", "device_pixels_b",
    "device_changed_pixels", "device_pixels_nontrivial",
    "device_motion_pixels_changed",
)


def _fail(msg: str) -> None:
    print(f"[uc08_physics_smoke] FAIL {msg}", file=sys.stderr)
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


def _is_pos_number(v) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0


def judge_host_doc(doc: dict | None) -> tuple[bool, list[str], dict]:
    """host 段 JSON 判定:subject + 16 断言全在位全 true + exit_ok +
    physics.total_step_ms 正值 measured(P-09:留证不进硬门)+ stages 含
    physics 正耗时 + streaming/mv/transform 数据字段存在性。纯函数。

    返回 (ok, problems, extras);extras 携带逐断言值与留证数字。"""
    extras: dict = {"assert_values": {}}
    if not isinstance(doc, dict):
        return False, ["uc08 host --json 解析失败"], extras
    problems: list[str] = []
    if doc.get("subject") != "uc08_physics":
        problems.append(f"subject != uc08_physics(={doc.get('subject')!r})")
    if doc.get("exit_ok") is not True:
        problems.append("exit_ok != true(demo 内断言未全过)")
    asserts = doc.get("asserts")
    if not isinstance(asserts, dict):
        problems.append("asserts 字段缺席/非对象(反 YAML-only)")
    else:
        for name in EXPECTED_ASSERTS:
            v = asserts.get(name)
            extras["assert_values"][name] = v
            if v is None:
                problems.append(f"断言字段缺席: {name}(反 YAML-only 空壳绿封死)")
            elif v is not True:
                problems.append(f"断言 {name} != true(={v!r})")
    physics = doc.get("physics")
    physics = physics if isinstance(physics, dict) else {}
    step_ms = physics.get("total_step_ms")
    extras["physics_step_ms"] = step_ms
    if not _is_pos_number(step_ms):
        problems.append(
            f"physics.total_step_ms 非正值 measured 数字(={step_ms!r};"
            "P-09 数字入 checks 不进硬门,但必须是实测正值)"
        )
    stages = doc.get("stages")
    stage_ok = isinstance(stages, list) and any(
        isinstance(s, dict) and s.get("name") == "physics" and _is_pos_number(s.get("cpu_ms"))
        for s in stages
    )
    extras["physics_stage_present"] = stage_ok
    if not stage_ok:
        problems.append("stages 缺 physics 阶段正耗时记录(物理步 measured 留证面)")
    streaming = doc.get("streaming")
    streaming_ok = isinstance(streaming, dict) and all(
        k in streaming for k in ("insert_frame", "remove_frame", "receipt_bodies", "releases")
    )
    extras["fields_streaming_present"] = streaming_ok
    if not streaming_ok:
        problems.append(
            "streaming 数据字段缺席(须含 insert_frame/remove_frame/receipt_bodies/releases)"
        )
    mv = doc.get("mv")
    mv_ok = isinstance(mv, dict) and all(k in mv for k in ("early_max", "post_sleep_max"))
    extras["fields_mv_present"] = mv_ok
    if not mv_ok:
        problems.append("mv 数据字段缺席(须含 early_max/post_sleep_max)")
    transform_ok = "transform_landed_max_err" in physics
    extras["fields_transform_present"] = transform_ok
    if not transform_ok:
        problems.append("physics.transform_landed_max_err 字段缺席")
    return (not problems), problems, extras


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


def _good_host_doc() -> dict:
    return {
        "subject": "uc08_physics",
        "exit_ok": True,
        "asserts": {n: True for n in EXPECTED_ASSERTS},
        "physics": {"total_step_ms": 12.34, "transform_landed_max_err": 0.000001},
        "stages": [{"name": "physics", "cpu_ms": 12.34}, {"name": "taa", "cpu_ms": 0.5}],
        "streaming": {"insert_frame": 10, "remove_frame": 40, "receipt_bodies": 2, "releases": 2},
        "mv": {"early_max": 0.01, "post_sleep_max": 0.0},
    }


def red_self_test() -> None:
    """合成数据断言各纯判定层能区分红绿;门失效即 exit 1。"""
    ok, probs, _extras = judge_host_doc(_good_host_doc())
    if not ok or probs:
        _fail(f"red 自检失败:合法 host JSON 被误判红(门过严): {probs}")
    doc = _good_host_doc()
    doc["subject"] = "something_else"
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:subject 错未判红(门失效)")
    doc = _good_host_doc()
    doc["exit_ok"] = False
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:exit_ok false 未判红(门失效)")
    doc = _good_host_doc()
    del doc["asserts"]["transform_landed"]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:断言字段缺席未判红(反 YAML-only 失效)")
    doc = _good_host_doc()
    doc["asserts"]["mv_zero_after_sleep"] = False
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:断言 false 未判红(门失效)")
    doc = _good_host_doc()
    doc["physics"]["total_step_ms"] = 0.0
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:physics_step_ms 零值未判红(门失效)")
    doc = _good_host_doc()
    doc["physics"]["total_step_ms"] = True
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:physics_step_ms 布尔冒充数字未判红(门失效)")
    doc = _good_host_doc()
    doc["stages"] = [{"name": "taa", "cpu_ms": 0.5}]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:stages 缺 physics 未判红(门失效)")
    doc = _good_host_doc()
    del doc["streaming"]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:streaming 字段缺席未判红(门失效)")
    doc = _good_host_doc()
    del doc["mv"]["early_max"]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:mv 子字段缺席未判红(门失效)")
    doc = _good_host_doc()
    del doc["physics"]["transform_landed_max_err"]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:transform 字段缺席未判红(门失效)")
    good_dev = {"device": {
        "device_name": "RTX", "pixels_a": 100, "pixels_b": 100,
        "changed_pixels": 50, "device_pixels_nontrivial": True,
        "device_motion_pixels_changed": True,
    }}
    ok, probs = judge_uc08_device_doc(good_dev)
    if not ok or probs:
        _fail("red 自检失败:合法 device JSON 被误判红(门过严)")
    bad_dev = {"device": {
        "device_name": "RTX", "device_pixels_nontrivial": False,
        "device_motion_pixels_changed": True,
    }}
    if judge_uc08_device_doc(bad_dev)[0]:
        _fail("red 自检失败:device 对拍 false 未判红(门失效)")
    if judge_uc08_device_doc({"device": None})[0]:
        _fail("red 自检失败:device 缺席未判红(门失效)")


# ————————————————————— 检查段 —————————————————————


def skip(msg: str, failures: list[str]) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        failures.append(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
        return 1
    print(f"[uc08_physics_smoke] SKIP {msg}(dev-env-degrade,退出 0 不充绿)")
    return 0


def host_section(results: dict, failures: list[str]) -> bool:
    """判据 (a)+(b):uc08 单测 exit 0 + host 全跑 96 帧 JSON 16 断言全 true。"""
    ok = True
    try:
        code, out, err = run(["cargo", "test", "-p", "uc08-physics"])
    except FileNotFoundError:
        results["uc08_tests_pass"] = False
        failures.append("host 段: cargo 不在 PATH(uc08 单测未能执行)")
        code = None
    if code is not None:
        blob = out + err
        results["uc08_test_count"] = sum(int(x) for x in TEST_OK_RE.findall(blob))
        results["uc08_tests_pass"] = code == 0
        if code != 0:
            print("[uc08_physics_smoke] host 段 cargo test 输出尾部:", file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            failures.append(f"host 段: `cargo test -p uc08-physics` exit {code}(单测红)")
            ok = False
        print(
            f"[uc08_physics_smoke] host 段 cargo test: rc={code}, "
            f"全过计数={results['uc08_test_count']}"
        )
    try:
        code, out, err = run(
            ["cargo", "run", "-q", "-p", "uc08-physics", "--",
             "--frames", "96", "--size", "128x72", "--json"],
            timeout=1800,  # host 全跑 ~155s,给足
        )
    except FileNotFoundError:
        results["host_run_exit_ok"] = False
        failures.append("host 段: cargo 不在 PATH(uc08 host 全跑未能执行)")
        return False
    doc = parse_uc08_json(out)
    if code != 0 or doc is None:
        print("[uc08_physics_smoke] host 段 uc08 run 输出尾部:", file=sys.stderr)
        print((out + err)[-2400:], file=sys.stderr)
        results["host_run_exit_ok"] = False
        failures.append(
            f"host 段: uc08 host 全跑未过(rc={code},JSON 解析={'ok' if doc else '失败'})"
        )
        return False
    jok, problems, extras = judge_host_doc(doc)
    results["host_run_exit_ok"] = True
    results["host_json_exit_ok"] = doc.get("exit_ok") is True
    for name in EXPECTED_ASSERTS:
        results[name] = extras["assert_values"].get(name)
    results["asserts_all_true"] = all(
        extras["assert_values"].get(n) is True for n in EXPECTED_ASSERTS
    )
    results["physics_step_ms"] = extras.get("physics_step_ms")
    results["physics_stage_present"] = extras["physics_stage_present"]
    results["fields_streaming_present"] = extras["fields_streaming_present"]
    results["fields_mv_present"] = extras["fields_mv_present"]
    results["fields_transform_present"] = extras["fields_transform_present"]
    for p in problems:
        failures.append(f"host 段: {p}")
    print(
        f"[uc08_physics_smoke] host 段 run: rc=0, asserts_all_true={results['asserts_all_true']}, "
        f"physics_step_ms={results['physics_step_ms']}(measured 留证,不进硬门)"
    )
    return ok and jok


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
        print("[uc08_physics_smoke] device 段输出尾部:", file=sys.stderr)
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
            f"[uc08_physics_smoke] device 段 PASS: {dev.get('device_name')} "
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
        "subject": "uc08_physics_smoke",
        "milestone": "G6.3 / G-G6-7 (RFC-0017 §4.B UC-08 合流 demo)",
        "step": 91,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {k: results.get(k) for k in CHECK_KEYS if results.get(k) is not None},
        "uc08_physics_smoke_ok": subject_ok,
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"uc08_physics_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[uc08_physics_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    if "--selftest" in sys.argv:
        red_self_test()
        print("[uc08_physics_smoke] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")
        return 0
    results: dict = {}
    failures: list[str] = []
    host_ok = host_section(results, failures)
    device_rc = device_section(results, failures) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if failures:
        print("[uc08_physics_smoke] FAIL 判据红清单:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    if device_rc != 0:
        return device_rc
    print("[uc08_physics_smoke] PASS(host 恒跑 + device gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
