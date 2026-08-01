#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G6.5 Taichi Vulkan AOT spike 冒烟(步骤 92;G-G6-6;RFC-0017 §4.E,成功臂)。

host 段(**恒跑**,无 GPU/DLL 也绿,check_* 风格):
  1. AOT 资产核验(spec「AOT 资产与生成脚本」):apps/uc09-taichi-spike/assets/
     particles.tcm 在位且非空 + 实测 sha256 == particles.tcm.sha256 登记(登记字段
     64 位小写 hex 形态核验)+ 生成脚本 gen_particles_aot.py 在树;任缺即红。
  2. feature 默认 off 核验(spec「TiRT FFI 模块」,cargo metadata 机验非 grep):
     rurix-rt 与 uc09-taichi-spike 的 features 均含 `taichi-tirt` 且 default
     feature 集均不含 `taichi-tirt`(默认构建零 taichi 依赖、零新 unsafe)。
  3. 三条禁止机器可核(§4.E4):rurix-physics 全树(Cargo.toml + src)零 taichi
     引用;rurix-render 全树零 taichi 引用且 lib.rs `#![forbid(unsafe_code)]`
     在位;tirt.rs 与 uc09 源码零 CUDA 主物理路径(grep 无 cuda 求解面引用——
     `\bcuda\b`/ti.cuda/TI_ARCH_CUDA/CUDA Driver API 前缀零命中;允许 tirt.rs/
     uc09 全树不出现「cuda」字样,nvcuda.dll 装载纪律镜像提及不算命中)。
  4. U43 登记核验:unsafe-audit/rurix-rt.md 含 `| U43 |` 条目(TiRT FFI 边界)。
  5. cargo 腿:cargo test -p uc09-taichi-spike(default features)exit 0(计数
     为 0 判红,反 vacuous-green)+ host 腿真跑 cargo run -p uc09-taichi-spike --
     --json exit 0,单行 JSON subject==uc09_taichi_spike/mode==host/exit_ok==true
     且 8 host 断言(asset_tcm_present/asset_sha256_registered/asset_sha256_match/
     asset_gen_script_present/graph_import_marked/graph_import_not_pooled/
     graph_transient_pooled/graph_copy_recorded)全部在位且全 true(缺位/非 true
     即红,反 YAML-only)+ 测试名/断言关键字在源在位(host.rs/main.rs/device.rs)。
  6. evidence 落盘 evidence/taichi_vulkan_spike_<UTC 紧凑时间戳>.json + 对
     milestones/g6/taichi_vulkan_spike_evidence_schema.json 自校验。

device 段(**gate real**;镜像 uc08 smoke SKIP=dev-env-degrade 体例):
  RURIX_REQUIRE_REAL=1 且 RURIX_TAICHI_C_API_DLL 设位(路径在盘)→
  cargo run -p uc09-taichi-spike --features taichi-tirt -- --json 真跑:exit 0 +
  device 五断言(device_launch_ok/device_buffer_exported/device_graph_copy_wired/
  device_readback_nonzero/device_first_values_exact)全 true + nonzero_count==64 +
  first_values 与 [i*1.5+1.0] 逐位相等(前 4 值 [1.0,2.5,4.0,5.5],f32 精确可表)。
  缺 provisioning(DLL 未设/不在盘)→ SKIP=dev-env degrade 退 0 不充绿(CI runner
  无 taichi DLL 属预期);provisioning 在位而真失败/断言红 → 永远硬红
  (RURIX_REQUIRE_REAL=1 下真失败翻硬红,mock/SKIP 不充绿)。
  measured 字段(device_name/particle_count/nonzero_count/exported_buffer_size/
  first_values)入 checks 留证不进硬门(P-09)。

任一判据红 → 逐项打印定位后 exit 1(evidence 仍如实落盘,红不充绿)。

用法: py -3 ci/taichi_vulkan_spike_smoke.py [--selftest]
  --selftest: 反 YAML-only 红绿自检(合成数据喂纯判定层),不跑 cargo、不写 evidence。
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g6/taichi_vulkan_spike_evidence_schema.json"

UC09 = "apps/uc09-taichi-spike"
ASSETS = f"{UC09}/assets"
TIRT_RS = "src/rurix-rt/src/tirt.rs"
RENDER_LIB_RS = "src/rurix-render/src/lib.rs"
UNSAFE_AUDIT_RT = "unsafe-audit/rurix-rt.md"

# uc09 host 腿 --json 的 8 断言字段(冻结面,apps/uc09-taichi-spike/src/host.rs
# run_host_leg;缺位/非 true 即红,反 YAML-only)。
EXPECTED_HOST_ASSERTS = (
    "asset_tcm_present", "asset_sha256_registered", "asset_sha256_match",
    "asset_gen_script_present", "graph_import_marked",
    "graph_import_not_pooled", "graph_transient_pooled", "graph_copy_recorded",
)

# uc09 device 腿 --json 的五断言字段(冻结面,apps/uc09-taichi-spike/src/device.rs
# run_device_leg,§4.E3 四段闭合 + 值域逐位契约;缺位/非 true 永远硬红)。
EXPECTED_DEVICE_ASSERTS = (
    "device_launch_ok", "device_buffer_exported", "device_graph_copy_wired",
    "device_readback_nonzero", "device_first_values_exact",
)

# AOT 资产契约:kernel fill_particles,f32 x 64,p[i] = i*1.5+1.0(全部 f32 精确
# 可表,逐位比较可行;device.rs expected_values 互锁)。
EXPECTED_PARTICLE_COUNT = 64
EXPECTED_PARTICLE_BYTES = EXPECTED_PARTICLE_COUNT * 4
EXPECTED_FIRST_VALUES = [i * 1.5 + 1.0 for i in range(4)]  # [1.0, 2.5, 4.0, 5.5]

# §4.E4 三条禁止审计面:taichi 名(大小写不敏感);CUDA 主物理路径求解面
# (`cuda` 词/TI_ARCH_CUDA/ti.cuda/CUDA Driver API 前缀;nvcuda.dll 装载纪律
# 镜像提及非词边界 cuda,不算命中)。
TAICHI_RE = re.compile(r"taichi", re.I)
CUDA_SOLVER_RE = re.compile(
    r"\bcuda\b|ti_arch_cuda|ti\.cuda"
    r"|\bcu(?:Init|Ctx[A-Za-z0-9_]*|Mem[A-Za-z0-9_]*|Launch[A-Za-z0-9_]*"
    r"|Module[A-Za-z0-9_]*|Stream[A-Za-z0-9_]*|Event[A-Za-z0-9_]*"
    r"|Device[A-Za-z0-9_]*|Driver[A-Za-z0-9_]*)",
    re.I,
)
FORBID_UNSAFE = "#![forbid(unsafe_code)]"
U43_RE = re.compile(r"\|\s*U43\s*\|")

# 反 YAML-only:测试名/断言关键字在源钉定(文件 -> 必需子串;physics_core_smoke
# §4.A7 清单关键字先例)。
ASSERT_KEYWORD_FILES = {
    f"{UC09}/src/host.rs": EXPECTED_HOST_ASSERTS + (
        "asset_hash_matches_registration", "graph_import_marking_and_copy_record",
        "host_leg_asserts_all_pass",
    ),
    f"{UC09}/src/main.rs": (
        "uc09_taichi_spike", "summary_json_is_single_line_and_frozen_shape",
        "cli_parse_rejects_unknown",
    ),
    f"{UC09}/src/device.rs": EXPECTED_DEVICE_ASSERTS + (
        "provisioning_classification", "expected_values_match_contract",
    ),
}

# cargo test 输出的通过计数行。
TEST_OK_RE = re.compile(r"test result: ok\. (\d+) passed; 0 failed")

# evidence checks 键序(schema additionalProperties=false,须与 g6 schema 同步)。
CHECK_KEYS = (
    "aot_asset_present", "aot_sha256_match", "aot_gen_script_present",
    "feature_off_rurix_rt", "feature_off_uc09",
    "audit_physics_zero_taichi", "audit_render_zero_taichi",
    "audit_render_forbid_unsafe", "audit_no_cuda_solver_path",
    "audit_u43_registered",
    "uc09_tests_pass", "uc09_test_count",
    "host_run_exit_ok", "host_json_exit_ok", "host_asserts_all_true",
    "host_assert_keywords_present",
) + EXPECTED_HOST_ASSERTS + (
    "tcm_sha256",
    "device_run_pass", "device_asserts_all_true",
) + EXPECTED_DEVICE_ASSERTS + (
    "device_nonzero_count_64", "device_first_values_bitwise",
    "device_name", "device_particle_count", "device_nonzero_count",
    "device_exported_buffer_size", "device_first_values",
    "step_time_secs",
)


def _fail(msg: str) -> None:
    print(f"[taichi_vulkan_spike_smoke] FAIL {msg}", file=sys.stderr)
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


def judge_asset(tcm_present: bool, measured: str | None, registered: str | None,
                gen_present: bool) -> list[str]:
    """判据 1:AOT 资产三件套(产物在位非空 + sha256 实测==登记 + 生成脚本在树)。
    纯函数。"""
    problems: list[str] = []
    if not tcm_present:
        problems.append(f"{ASSETS}/particles.tcm 缺席/为空(AOT 产物入仓,任缺即红)")
    if not (isinstance(registered, str) and len(registered) == 64
            and all(c in "0123456789abcdef" for c in registered)):
        problems.append(
            f"{ASSETS}/particles.tcm.sha256 登记缺席/形态非法(={registered!r};"
            "须 64 位小写 hex)"
        )
    elif measured is not None and measured != registered:
        problems.append(
            f"实测 sha256 与登记不一致:{measured!r} != {registered!r}"
            "(资产本体核验;再生成物非逐位可复现,核验对象为入仓产物本体)"
        )
    if not gen_present:
        problems.append(f"{ASSETS}/gen_particles_aot.py 不在树(生成脚本须入仓)")
    return problems


def judge_feature_off(pkg_label: str, features: dict | None) -> list[str]:
    """判据 2:feature `taichi-tirt` 存在但默认 off(cargo metadata 机验)。纯函数。"""
    problems: list[str] = []
    if not isinstance(features, dict):
        return [f"cargo metadata 缺 {pkg_label} 包"]
    if "taichi-tirt" not in features:
        problems.append(f"{pkg_label} features 缺 `taichi-tirt`(G6.5 spike feature 须在位)")
    if "taichi-tirt" in (features.get("default") or []):
        problems.append(
            f"{pkg_label} default feature 集含 `taichi-tirt`"
            "(feature 默认 off:默认构建零 taichi 依赖、零新 unsafe)"
        )
    return problems


def audit_no_taichi(files: dict[str, str]) -> list[str]:
    """判据 3a/3b:文件集零 taichi 引用(§4.E4 禁 Taichi 替代主刚体/渲染面)。
    纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        for lineno, line in enumerate(text.splitlines(), 1):
            if TAICHI_RE.search(line):
                problems.append(
                    f"{path}:{lineno}: 出现 taichi 引用(§4.E4:零 taichi 依赖审计面;"
                    f"命中行 {line.strip()!r})"
                )
    return problems


def audit_forbid_unsafe(lib_rs: str | None) -> list[str]:
    """判据 3b:rurix-render lib.rs `#![forbid(unsafe_code)]` 在位(G5 冻结面)。
    纯函数。"""
    if lib_rs is None:
        return [f"{RENDER_LIB_RS} 缺席(rurix-render 库面须在树)"]
    if FORBID_UNSAFE not in lib_rs:
        return [
            f"{RENDER_LIB_RS} 缺 `{FORBID_UNSAFE}`"
            "(§4.E4:rurix-render 全 safe 冻结面,unsafe 集中 rurix-rt tirt 模块 U43)"
        ]
    return []


def audit_no_cuda_solver(files: dict[str, str]) -> list[str]:
    """判据 3c:tirt.rs 与 uc09 源码零 CUDA 主物理路径(§4.E4:无 CUDA 后端新增
    「主物理」求解;允许全树零「cuda」字样)。纯函数。"""
    problems: list[str] = []
    for path, text in sorted(files.items()):
        for lineno, line in enumerate(text.splitlines(), 1):
            m = CUDA_SOLVER_RE.search(line)
            if m:
                problems.append(
                    f"{path}:{lineno}: 出现 CUDA 求解面引用 {m.group(0)!r}"
                    f"(§4.E4:禁 CUDA 后端主物理路径;命中行 {line.strip()!r})"
                )
    return problems


def audit_u43(text: str | None) -> list[str]:
    """判据 4:unsafe-audit/rurix-rt.md 含 `| U43 |` 条目(TiRT FFI 边界登记)。
    纯函数。"""
    if text is None:
        return [f"{UNSAFE_AUDIT_RT} 缺席(unsafe 登记面须在树)"]
    if not U43_RE.search(text):
        return [
            f"{UNSAFE_AUDIT_RT} 缺 `| U43 |` 条目"
            "(tirt 模块 unsafe 集中登记,U next_free 43 已消费)"
        ]
    return []


def audit_keywords(files: dict[str, str],
                   markers: dict[str, tuple[str, ...]] = ASSERT_KEYWORD_FILES) -> list[str]:
    """判据 5c:测试名/断言关键字在源钉定(反 YAML-only 空壳绿)。纯函数。"""
    problems: list[str] = []
    for path, needles in markers.items():
        text = files.get(path)
        if text is None:
            problems.append(f"{path} 缺席(测试/断言关键字审计面缺位)")
            continue
        for needle in needles:
            if needle not in text:
                problems.append(
                    f"{path} 缺关键字 {needle!r}(测试名/断言字段冻结面,反 YAML-only)"
                )
    return problems


def judge_host_doc(doc: dict | None) -> tuple[bool, list[str], dict]:
    """判据 5b:host 腿 --json 判定:subject/mode/exit_ok + 8 断言全在位全 true +
    device 字段恒 null 且 device_status==feature_off(host 模式冻结形态)。
    返回 (ok, problems, extras);extras 携带逐断言值。纯函数。"""
    extras: dict = {"assert_values": {}}
    if not isinstance(doc, dict):
        return False, ["uc09 host --json 解析失败"], extras
    problems: list[str] = []
    if doc.get("subject") != "uc09_taichi_spike":
        problems.append(f"subject != uc09_taichi_spike(={doc.get('subject')!r})")
    if doc.get("mode") != "host":
        problems.append(f"mode != host(={doc.get('mode')!r};default features 须 host 模式)")
    if doc.get("exit_ok") is not True:
        problems.append("exit_ok != true(demo 内断言未全过)")
    if doc.get("device") is not None:
        problems.append("device 字段非 null(host 模式 device 腿须 feature_off)")
    if doc.get("device_status") != "feature_off":
        problems.append(
            f"device_status != feature_off(={doc.get('device_status')!r};host 模式冻结形态)"
        )
    asserts = doc.get("asserts")
    if not isinstance(asserts, dict):
        problems.append("asserts 字段缺席/非对象(反 YAML-only)")
    else:
        for name in EXPECTED_HOST_ASSERTS:
            v = asserts.get(name)
            extras["assert_values"][name] = v
            if v is None:
                problems.append(f"断言字段缺席: {name}(反 YAML-only 空壳绿封死)")
            elif v is not True:
                problems.append(f"断言 {name} != true(={v!r})")
    return (not problems), problems, extras


def judge_device_doc(doc: dict | None) -> tuple[bool, list[str], dict]:
    """device 段 --json 判定:exit_ok + device 段非空 + 五断言全 true +
    nonzero_count==64 + first_values 与 [i*1.5+1.0] 逐位相等(前 4 值
    [1.0,2.5,4.0,5.5])+ device_name 非空。对拍类字段非 true 永远硬红。
    返回 (ok, problems, extras);extras 携带 measured 留证。纯函数。"""
    extras: dict = {}
    if not isinstance(doc, dict):
        return False, ["uc09 --features taichi-tirt --json 解析失败"], extras
    problems: list[str] = []
    if doc.get("exit_ok") is not True:
        problems.append("exit_ok != true(device 腿断言未全过,红不充绿)")
    dev = doc.get("device")
    if not isinstance(dev, dict):
        problems.append("JSON device 字段缺席(device gate real 真跑须出 device 段)")
        dev = {}
    if doc.get("device_status") != "ok":
        problems.append(
            f"device_status != ok(={doc.get('device_status')!r};真跑成立须 ok,SKIP 不充绿)"
        )
    extras["device_name"] = dev.get("device_name")
    extras["particle_count"] = dev.get("particle_count")
    extras["nonzero_count"] = dev.get("nonzero_count")
    extras["exported_buffer_size"] = dev.get("exported_buffer_size")
    extras["first_values"] = dev.get("first_values")
    asserts = dev.get("asserts")
    if not isinstance(asserts, dict):
        problems.append("device.asserts 字段缺席/非对象(反 YAML-only)")
    else:
        for name in EXPECTED_DEVICE_ASSERTS:
            v = asserts.get(name)
            extras.setdefault("assert_values", {})[name] = v
            if v is None:
                problems.append(f"device 断言字段缺席: {name}(反 YAML-only 空壳绿封死)")
            elif v is not True:
                problems.append(
                    f"device 断言 {name} != true(={v!r};对拍类非 true 永远硬红)"
                )
    nz = dev.get("nonzero_count")
    if nz != EXPECTED_PARTICLE_COUNT:
        problems.append(
            f"nonzero_count={nz!r} != {EXPECTED_PARTICLE_COUNT}"
            "(readback 非零 device 见证:64/64 元素非零)"
        )
    fv = dev.get("first_values")
    if not (isinstance(fv, list) and len(fv) == len(EXPECTED_FIRST_VALUES)
            and all(isinstance(x, (int, float)) and not isinstance(x, bool)
                    and x == want for x, want in zip(fv, EXPECTED_FIRST_VALUES))):
        problems.append(
            f"first_values={fv!r} 与契约 [i*1.5+1.0] 非逐位相等"
            f"(前 4 值须 == {EXPECTED_FIRST_VALUES},f32 精确可表)"
        )
    if not dev.get("device_name"):
        problems.append("device.device_name 空(device 真跑须实名留证)")
    return (not problems), problems, extras


# ————————————————————— IO 采集层 —————————————————————


def collect_tree_files(crate_dir: str) -> dict[str, str]:
    """crate 全树(Cargo.toml + src/**/*.rs)-> 相对路径/文本。"""
    files: dict[str, str] = {}
    manifest = ROOT / crate_dir / "Cargo.toml"
    if manifest.is_file():
        files[manifest.relative_to(ROOT).as_posix()] = manifest.read_text(encoding="utf-8")
    src = ROOT / crate_dir / "src"
    if src.is_dir():
        for p in sorted(src.rglob("*.rs")):
            files[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    return files


def parse_uc09_json(out: str) -> dict | None:
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}") and '"subject":"uc09_taichi_spike"' in line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                return None
    return None


def rustc_version() -> str:
    try:
        code, out, _err = run(["rustc", "--version"], timeout=60)
    except FileNotFoundError:
        return "rustc 不在 PATH"
    return out.strip() if code == 0 else "rustc --version 探测失败"


# ————————————————————— red 自检(反 YAML-only)—————————————————————


def _good_host_doc() -> dict:
    return {
        "subject": "uc09_taichi_spike",
        "mode": "host",
        "tcm_bytes": 3873,
        "tcm_sha256": "f" * 64,
        "registered_sha256": "f" * 64,
        "gen_script_present": True,
        "asserts": {n: True for n in EXPECTED_HOST_ASSERTS},
        "graph": {"pass_count": 1, "resource_count": 2, "copy_byte_size": 256},
        "device": None,
        "device_status": "feature_off",
        "device_skip_reason": "feature taichi-tirt 未启用",
        "exit_ok": True,
    }


def _good_device_doc() -> dict:
    return {
        "subject": "uc09_taichi_spike",
        "mode": "device",
        "asserts": {n: True for n in EXPECTED_HOST_ASSERTS},
        "device": {
            "device_name": "NVIDIA GeForce RTX 4070 Ti",
            "particle_count": 64,
            "nonzero_count": 64,
            "exported_buffer_size": 256,
            "first_values": [1.0, 2.5, 4.0, 5.5],
            "asserts": {n: True for n in EXPECTED_DEVICE_ASSERTS},
        },
        "device_status": "ok",
        "device_skip_reason": None,
        "exit_ok": True,
    }


def red_self_test() -> None:
    """合成数据断言各纯判定层能区分红绿;门失效即 exit 1。"""
    # 判据 1:AOT 资产
    if judge_asset(True, "a" * 64, "a" * 64, True):
        _fail("red 自检失败:合法资产三件套被误判红(门过严)")
    if not judge_asset(False, "a" * 64, "a" * 64, True):
        _fail("red 自检失败:particles.tcm 缺席未判红(门失效)")
    if not judge_asset(True, "b" * 64, "a" * 64, True):
        _fail("red 自检失败:sha256 实测!=登记未判红(门失效)")
    if not judge_asset(True, "a" * 64, "a" * 64, False):
        _fail("red 自检失败:生成脚本缺席未判红(门失效)")
    if not judge_asset(True, "a" * 64, "xyz", True):
        _fail("red 自检失败:登记形态非法未判红(门失效)")
    # 判据 2:feature 默认 off
    good_features = {"default": [], "vulkan": [], "taichi-tirt": ["vulkan"]}
    if judge_feature_off("rurix-rt", good_features):
        _fail("red 自检失败:合法 features(taichi-tirt 存在 + 默认 off)被误判红(门过严)")
    bad_default = {"default": ["taichi-tirt"], "taichi-tirt": ["vulkan"]}
    if not judge_feature_off("uc09-taichi-spike", bad_default):
        _fail("red 自检失败:default 集含 taichi-tirt 未判红(门失效)")
    missing_feature = {"default": [], "vulkan": []}
    if not judge_feature_off("rurix-rt", missing_feature):
        _fail("red 自检失败:feature taichi-tirt 缺席未判红(门失效)")
    if not judge_feature_off("ghost-pkg", None):
        _fail("red 自检失败:包缺席未判红(门失效)")
    # 判据 3a/3b:零 taichi
    clean_tree = {
        "src/rurix-physics/Cargo.toml": "[package]\nname = \"rurix-physics\"\n",
        "src/rurix-physics/src/lib.rs": "//! 物理库。\n#![forbid(unsafe_code)]\n",
    }
    if audit_no_taichi(clean_tree):
        _fail("red 自检失败:干净树被误判红(门过严)")
    if not audit_no_taichi({"src/rurix-physics/src/lib.rs": "// Taichi 引用\n"}):
        _fail("red 自检失败:physics 树 taichi 引用未判红(§4.E4,门失效)")
    if not audit_no_taichi({"src/rurix-render/Cargo.toml": "taichi = \"1\"\n"}):
        _fail("red 自检失败:render Cargo.toml taichi 依赖未判红(§4.E4,门失效)")
    # 判据 3b:forbid(unsafe_code)
    if audit_forbid_unsafe("//! 渲染库。\n#![forbid(unsafe_code)]\n"):
        _fail("red 自检失败:forbid(unsafe_code) 在位被误判红(门过严)")
    if not audit_forbid_unsafe("//! 渲染库。\n"):
        _fail("red 自检失败:forbid(unsafe_code) 缺失未判红(门失效)")
    if not audit_forbid_unsafe(None):
        _fail("red 自检失败:lib.rs 缺席未判红(门失效)")
    # 判据 3c:零 CUDA 主物理路径(nvcuda.dll 装载纪律镜像提及不算命中)
    clean_tirt = {
        TIRT_RS: "//! 动态装载(镜像 crate::sys nvcuda.dll / crate::vk vulkan-1.dll 纪律)\n",
        f"{UC09}/src/device.rs": "use rurix_rt::tirt;\n",
    }
    if audit_no_cuda_solver(clean_tirt):
        _fail("red 自检失败:nvcuda.dll 镜像提及被误判红(门过严)")
    for bad, label in (
        ("ti.init(arch=ti.cuda)", "ti.cuda"),
        ("TiArch::TI_ARCH_CUDA", "TI_ARCH_CUDA"),
        ("unsafe { cuLaunchKernel(f, 1, 1, 1, 1, 1, 1, 0, s, p, e) };", "cuLaunchKernel"),
        ("// cuda 求解面\n", "cuda 词"),
    ):
        if not audit_no_cuda_solver({TIRT_RS: bad + "\n"}):
            _fail(f"red 自检失败:{label} 未判红(§4.E4 CUDA 主物理路径,门失效)")
    # 判据 4:U43 登记
    if audit_u43("| U42 | x |\n| U43 | G6.5 TiRT FFI 边界 | 位置 | 契约 |\n"):
        _fail("red 自检失败:U43 条目在位被误判红(门过严)")
    if not audit_u43("| U42 | x |\n"):
        _fail("red 自检失败:U43 条目缺席未判红(门失效)")
    if not audit_u43(None):
        _fail("red 自检失败:unsafe-audit 文件缺席未判红(门失效)")
    # 判据 5c:关键字在源
    kw_files = {
        f"{UC09}/src/host.rs": " ".join(ASSERT_KEYWORD_FILES[f"{UC09}/src/host.rs"]),
        f"{UC09}/src/main.rs": " ".join(ASSERT_KEYWORD_FILES[f"{UC09}/src/main.rs"]),
        f"{UC09}/src/device.rs": " ".join(ASSERT_KEYWORD_FILES[f"{UC09}/src/device.rs"]),
    }
    if audit_keywords(kw_files):
        _fail("red 自检失败:关键字齐全被误判红(门过严)")
    shrunk = dict(kw_files)
    shrunk[f"{UC09}/src/device.rs"] = "use rurix_rt::tirt;\n"
    if not audit_keywords(shrunk):
        _fail("red 自检失败:断言关键字缺席未判红(反 YAML-only,门失效)")
    if not audit_keywords({}):
        _fail("red 自检失败:源文件全缺席未判红(门失效)")
    # 判据 5b:host 腿 JSON
    ok, probs, _extras = judge_host_doc(_good_host_doc())
    if not ok or probs:
        _fail(f"red 自检失败:合法 host JSON 被误判红(门过严): {probs}")
    doc = _good_host_doc()
    doc["subject"] = "something_else"
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:subject 错未判红(门失效)")
    doc = _good_host_doc()
    doc["mode"] = "device"
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:mode 错未判红(门失效)")
    doc = _good_host_doc()
    doc["exit_ok"] = False
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:exit_ok false 未判红(门失效)")
    doc = _good_host_doc()
    del doc["asserts"]["graph_copy_recorded"]
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:host 断言字段缺席未判红(反 YAML-only 失效)")
    doc = _good_host_doc()
    doc["asserts"]["asset_sha256_match"] = False
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:host 断言 false 未判红(门失效)")
    doc = _good_host_doc()
    doc["device_status"] = "ok"
    if judge_host_doc(doc)[0]:
        _fail("red 自检失败:host 模式 device_status 异常未判红(门失效)")
    if judge_host_doc(None)[0]:
        _fail("red 自检失败:非法 JSON 未判红(门失效)")
    # device 段 JSON
    ok, probs, _extras = judge_device_doc(_good_device_doc())
    if not ok or probs:
        _fail(f"red 自检失败:合法 device JSON 被误判红(门过严): {probs}")
    doc = _good_device_doc()
    doc["device"]["asserts"]["device_readback_nonzero"] = False
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:device 断言 false 未判红(对拍类永远硬红,门失效)")
    doc = _good_device_doc()
    del doc["device"]["asserts"]["device_launch_ok"]
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:device 断言字段缺席未判红(反 YAML-only 失效)")
    doc = _good_device_doc()
    doc["device"]["nonzero_count"] = 63
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:nonzero_count !=64 未判红(门失效)")
    doc = _good_device_doc()
    doc["device"]["first_values"] = [1.0, 2.5, 4.0, 5.5000001]
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:first_values 漂移未判红(逐位契约,门失效)")
    doc = _good_device_doc()
    doc["device"] = None
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:device 段缺席未判红(门失效)")
    doc = _good_device_doc()
    doc["device"]["device_name"] = ""
    if judge_device_doc(doc)[0]:
        _fail("red 自检失败:device_name 空未判红(门失效)")
    if judge_device_doc(None)[0]:
        _fail("red 自检失败:device JSON 非法未判红(门失效)")


# ————————————————————— 检查段 —————————————————————


def skip(msg: str) -> int:
    """device 段 provisioning 缺失 -> SKIP=dev-env-degrade 退 0 不充绿(uc08 体例;
    CI runner 无 taichi DLL 属预期;真失败不走本路,永远硬红)。"""
    print(f"[taichi_vulkan_spike_smoke] SKIP {msg}(dev-env-degrade,退出 0 不充绿)")
    return 0


def asset_section(results: dict, failures: list[str]) -> bool:
    """判据 1:AOT 资产核验(产物 + sha256 + 生成脚本,不依赖 GPU)。"""
    tcm_path = ROOT / ASSETS / "particles.tcm"
    sha_path = ROOT / ASSETS / "particles.tcm.sha256"
    gen_path = ROOT / ASSETS / "gen_particles_aot.py"
    tcm = tcm_path.read_bytes() if tcm_path.is_file() else b""
    measured = hashlib.sha256(tcm).hexdigest() if tcm else None
    registered = None
    if sha_path.is_file():
        registered = sha_path.read_text(encoding="utf-8").split()
        registered = registered[0].lower() if registered else None
    gen_present = gen_path.is_file()
    problems = judge_asset(bool(tcm), measured, registered, gen_present)
    results["aot_asset_present"] = bool(tcm)
    results["aot_sha256_match"] = measured is not None and measured == registered
    results["aot_gen_script_present"] = gen_present
    results["tcm_sha256"] = measured
    for p in problems:
        failures.append(f"资产段: {p}")
    print(
        f"[taichi_vulkan_spike_smoke] 资产段: tcm={len(tcm)}B, "
        f"sha256_match={results['aot_sha256_match']}, gen_script={gen_present}"
    )
    return not problems


def metadata_section(results: dict, failures: list[str]) -> bool:
    """判据 2:feature `taichi-tirt` 默认 off(cargo metadata 机验,非 grep)。"""
    try:
        code, out, err = run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                             timeout=600)
    except FileNotFoundError:
        results["feature_off_rurix_rt"] = False
        results["feature_off_uc09"] = False
        failures.append("metadata 段: cargo 不在 PATH")
        return False
    if code != 0:
        results["feature_off_rurix_rt"] = False
        results["feature_off_uc09"] = False
        failures.append(f"metadata 段: cargo metadata exit {code}:{err.strip()[:400]!r}")
        return False
    try:
        meta = json.loads(out)
    except json.JSONDecodeError:
        results["feature_off_rurix_rt"] = False
        results["feature_off_uc09"] = False
        failures.append("metadata 段: cargo metadata 输出非合法 JSON")
        return False
    ok = True
    for pkg_name, key in (("rurix-rt", "feature_off_rurix_rt"),
                          ("uc09-taichi-spike", "feature_off_uc09")):
        features = None
        for pkg in meta.get("packages", []):
            if pkg.get("name") == pkg_name:
                features = pkg.get("features") or {}
                break
        problems = judge_feature_off(pkg_name, features)
        results[key] = not problems
        for p in problems:
            failures.append(f"metadata 段: {p}")
        if problems:
            ok = False
    print(
        f"[taichi_vulkan_spike_smoke] metadata 段: rurix-rt 默认 off="
        f"{results['feature_off_rurix_rt']}, uc09 默认 off={results['feature_off_uc09']}"
    )
    return ok


def audit_section(results: dict, failures: list[str]) -> bool:
    """判据 3+4:§4.E4 三条禁止机器可核 + U43 登记核验。"""
    physics_files = collect_tree_files("src/rurix-physics")
    render_files = collect_tree_files("src/rurix-render")
    cuda_files: dict[str, str] = {}
    tirt = ROOT / TIRT_RS
    if tirt.is_file():
        cuda_files[TIRT_RS] = tirt.read_text(encoding="utf-8")
    else:
        cuda_files[TIRT_RS] = None  # type: ignore[assignment]
    uc09_src = ROOT / UC09 / "src"
    if uc09_src.is_dir():
        for p in sorted(uc09_src.rglob("*.rs")):
            cuda_files[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    lib_rs_path = ROOT / RENDER_LIB_RS
    lib_rs = lib_rs_path.read_text(encoding="utf-8") if lib_rs_path.is_file() else None
    audit_path = ROOT / UNSAFE_AUDIT_RT
    audit_text = audit_path.read_text(encoding="utf-8") if audit_path.is_file() else None
    checks = [
        ("audit_physics_zero_taichi", audit_no_taichi(physics_files)),
        ("audit_render_zero_taichi", audit_no_taichi(render_files)),
        ("audit_render_forbid_unsafe", audit_forbid_unsafe(lib_rs)),
        ("audit_no_cuda_solver_path",
         audit_no_cuda_solver({k: v for k, v in cuda_files.items() if v is not None})
         + ([f"{TIRT_RS} 缺席(tirt 模块须在树,feature taichi-tirt 默认 off)"]
            if cuda_files.get(TIRT_RS) is None else [])),
        ("audit_u43_registered", audit_u43(audit_text)),
    ]
    ok = True
    for key, problems in checks:
        results[key] = not problems
        for p in problems:
            failures.append(f"§4.E4/U43 审计门: {p}")
        if problems:
            ok = False
        print(
            f"[taichi_vulkan_spike_smoke] 审计 {key}: "
            f"{'PASS' if not problems else f'RED({len(problems)} 处)'}"
        )
    return ok


def cargo_section(results: dict, failures: list[str]) -> bool:
    """判据 5:cargo test exit 0 + host 腿真跑 --json 8 断言全 true + 关键字在源。"""
    ok = True
    try:
        code, out, err = run(["cargo", "test", "-p", "uc09-taichi-spike"])
    except FileNotFoundError:
        results["uc09_tests_pass"] = False
        results["uc09_test_count"] = 0
        failures.append("cargo 段: cargo 不在 PATH(uc09 单测未能执行)")
        code = None
    if code is not None:
        blob = out + err
        results["uc09_test_count"] = sum(int(x) for x in TEST_OK_RE.findall(blob))
        results["uc09_tests_pass"] = code == 0 and results["uc09_test_count"] > 0
        if code != 0:
            print("[taichi_vulkan_spike_smoke] cargo test 输出尾部:", file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            failures.append(f"cargo 段: `cargo test -p uc09-taichi-spike` exit {code}(单测红)")
            ok = False
        elif results["uc09_test_count"] == 0:
            failures.append("cargo 段: 通过计数为 0(测试面坍缩,反 vacuous-green)")
            ok = False
        print(
            f"[taichi_vulkan_spike_smoke] cargo test: rc={code}, "
            f"全过计数={results['uc09_test_count']}"
        )
    try:
        code, out, err = run(
            ["cargo", "run", "-q", "-p", "uc09-taichi-spike", "--", "--json"],
            timeout=1800,
        )
    except FileNotFoundError:
        results["host_run_exit_ok"] = False
        failures.append("cargo 段: cargo 不在 PATH(uc09 host 腿未能执行)")
        return False
    doc = parse_uc09_json(out)
    if code != 0 or doc is None:
        print("[taichi_vulkan_spike_smoke] host 腿 run 输出尾部:", file=sys.stderr)
        print((out + err)[-2400:], file=sys.stderr)
        results["host_run_exit_ok"] = False
        failures.append(
            f"cargo 段: uc09 host 腿未过(rc={code},JSON 解析={'ok' if doc else '失败'})"
        )
        return False
    jok, problems, extras = judge_host_doc(doc)
    results["host_run_exit_ok"] = True
    results["host_json_exit_ok"] = doc.get("exit_ok") is True
    for name in EXPECTED_HOST_ASSERTS:
        results[name] = extras["assert_values"].get(name)
    results["host_asserts_all_true"] = all(
        extras["assert_values"].get(n) is True for n in EXPECTED_HOST_ASSERTS
    )
    for p in problems:
        failures.append(f"cargo 段 host 腿: {p}")
    kw_files: dict[str, str] = {}
    for path in ASSERT_KEYWORD_FILES:
        p = ROOT / path
        if p.is_file():
            kw_files[path] = p.read_text(encoding="utf-8")
    kw_problems = audit_keywords(kw_files)
    results["host_assert_keywords_present"] = not kw_problems
    for p in kw_problems:
        failures.append(f"cargo 段关键字审计: {p}")
    print(
        f"[taichi_vulkan_spike_smoke] host 腿 run: rc=0, "
        f"asserts_all_true={results['host_asserts_all_true']}, "
        f"keywords={results['host_assert_keywords_present']}"
    )
    return ok and jok and not kw_problems


def device_section(results: dict, failures: list[str]) -> int:
    """device 段(gate real):RURIX_REQUIRE_REAL=1 且 RURIX_TAICHI_C_API_DLL 设位
    -> --features taichi-tirt 真跑,五断言 + nonzero==64 + first_values 逐位。
    缺 provisioning -> SKIP=dev-env degrade 退 0 不充绿;真失败永远硬红。"""
    dll = os.environ.get("RURIX_TAICHI_C_API_DLL", "")
    if not dll or not Path(dll).is_file():
        results["device_run_pass"] = "SKIP"
        results["toolchain_skip"] = "no-taichi-dll"
        return skip(
            "device 段: RURIX_TAICHI_C_API_DLL 未设/路径不在盘(taichi_c_api.dll 为本机 "
            "provisioning,不入仓不入 runner 镜像;device 真跑归 gate real,host 段已恒跑)"
        )
    if os.environ.get("RURIX_REQUIRE_REAL") != "1":
        results["device_run_pass"] = "SKIP"
        results["toolchain_skip"] = "require-real-off"
        return skip(
            "device 段: RURIX_REQUIRE_REAL 未置 1(device gate real 双要件:"
            "REQUIRE_REAL=1 且 DLL 设位;未 opt-in 真跑不充绿)"
        )
    try:
        code, out, err = run(
            ["cargo", "run", "-q", "-p", "uc09-taichi-spike",
             "--features", "taichi-tirt", "--", "--json"],
            env_extra={"RURIX_REQUIRE_REAL": "1"}, timeout=1800,
        )
    except FileNotFoundError:
        results["device_run_pass"] = False
        failures.append("device 段: cargo 不在 PATH(uc09 device 腿未能执行)")
        return 1
    doc = parse_uc09_json(out)
    if code != 0 or doc is None:
        print("[taichi_vulkan_spike_smoke] device 段输出尾部:", file=sys.stderr)
        print((out + err)[-2400:], file=sys.stderr)
        results["device_run_pass"] = False
        failures.append(
            f"device 段: uc09 device 腿未过(rc={code},JSON 解析={'ok' if doc else '失败'};"
            "provisioning 在位而真失败,永远硬红)"
        )
        return 1
    ok, problems, extras = judge_device_doc(doc)
    results["device_run_pass"] = ok
    values = extras.get("assert_values", {})
    results["device_asserts_all_true"] = all(
        values.get(n) is True for n in EXPECTED_DEVICE_ASSERTS
    )
    for name in EXPECTED_DEVICE_ASSERTS:
        results[name] = values.get(name)
    results["device_nonzero_count_64"] = extras.get("nonzero_count") == EXPECTED_PARTICLE_COUNT
    results["device_first_values_bitwise"] = extras.get("first_values") == EXPECTED_FIRST_VALUES
    results["device_name"] = extras.get("device_name")
    results["device_particle_count"] = extras.get("particle_count")
    results["device_nonzero_count"] = extras.get("nonzero_count")
    results["device_exported_buffer_size"] = extras.get("exported_buffer_size")
    results["device_first_values"] = json.dumps(extras.get("first_values"))
    for p in problems:
        failures.append(f"device 段: {p}")
    if ok:
        print(
            f"[taichi_vulkan_spike_smoke] device 段 PASS: {extras.get('device_name')} "
            f"nonzero={extras.get('nonzero_count')}/64 "
            f"first_values={extras.get('first_values')}"
        )
    return 0 if ok else 1


def write_evidence(results: dict, host_ok: bool, device_rc: int, machine: str) -> Path:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).replace(microsecond=0)
    device_skipped = results.get("device_run_pass") == "SKIP" or results.get("toolchain_skip") is not None
    # mock/SKIP 不充绿:_ok 要求 host 全绿且 device 段真跑判绿。
    subject_ok = host_ok and results.get("device_run_pass") is True
    doc = {
        "schema_version": 1,
        "subject": "taichi_vulkan_spike",
        "milestone": "G6.5 / G-G6-6 (RFC-0017 §4.E)",
        "step": 92,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "machine": machine,
        "checks": {k: results.get(k) for k in CHECK_KEYS if results.get(k) is not None},
        "taichi_vulkan_spike_ok": subject_ok,
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"taichi_vulkan_spike_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                  encoding="utf-8", newline="\n")
    print(f"[taichi_vulkan_spike_smoke] 写 evidence {ev.relative_to(ROOT)}; "
          f"run_url={doc['run_url']}")
    return ev


def validate_evidence_schema(ev: Path, failures: list[str]) -> bool:
    """判据 6:evidence 对 milestones/g6 schema 自校验(与 check_schemas 同一 schema,
    防 CHECK_KEYS/schema 漂移)。"""
    try:
        import jsonschema
    except ImportError:
        failures.append("schema 自校验: 缺 jsonschema 依赖(pip install -r requirements.txt)")
        return False
    try:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        failures.append(f"schema 自校验: schema 不可读/非法 {SCHEMA_PATH.name}: {e}")
        return False
    doc = json.loads(ev.read_text(encoding="utf-8"))
    errors = list(jsonschema.Draft7Validator(schema).iter_errors(doc))
    for v in errors:
        failures.append(
            f"schema 自校验: {'/'.join(str(p) for p in v.path)}: {v.message}"
        )
    print(
        f"[taichi_vulkan_spike_smoke] schema 自校验: "
        f"{'PASS' if not errors else f'RED({len(errors)} 处)'}"
    )
    return not errors


def main() -> int:
    if "--selftest" in sys.argv:
        red_self_test()
        print("[taichi_vulkan_spike_smoke] selftest PASS"
              "(红绿判别有效;未跑 cargo、未写 evidence)")
        return 0
    t0 = time.monotonic()
    machine = f"{platform.platform()}; {rustc_version()}"
    results: dict = {}
    failures: list[str] = []
    asset_ok = asset_section(results, failures)
    meta_ok = metadata_section(results, failures)
    audits_ok = audit_section(results, failures)
    cargo_ok = cargo_section(results, failures)
    results["step_time_secs"] = round(time.monotonic() - t0, 3)
    host_ok = asset_ok and meta_ok and audits_ok and cargo_ok
    device_rc = device_section(results, failures) if host_ok else 1
    ev = write_evidence(results, host_ok, device_rc, machine)
    validate_evidence_schema(ev, failures)
    if failures:
        print("[taichi_vulkan_spike_smoke] FAIL 判据红清单:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    if device_rc != 0:
        return device_rc
    print("[taichi_vulkan_spike_smoke] PASS(host 恒跑 + device gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
