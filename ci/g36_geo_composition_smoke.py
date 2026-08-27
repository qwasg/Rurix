#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G36 W1-W4 互斥项修复（geo 组合面）门（gate g36.wave1.geo_composition）。

判据闭集（facts 十项;判据字面 = 本 docstring,与 schema facts enum 逐字同序）：
① builds_green：g14_3_pipeline_perf / g34_full_lane / g35_particle_lane /
   g31_cluster_lod_bake / g31_wp_hlod_bake 构建必绿；
② packs_deterministic：RXCP/RXWH 双 bake 字节相等（double-build 确定性门）；
③ single_open_zero_drift：选层机核抽取（cluster_lod_select/wp_hlod_select）后
   单开路径零漂移——bench off == --cluster-lod leaf == --wp-hlod full 末帧
   digest 三臂位级一致（leaf/full 逐位锚 bin 内嵌 fail-closed 双证）；
④ combined_leafxfull_bitexact：--cluster-lod leaf × --wp-hlod full 组合极限
   == off 位级（W2 组合对拍锚,geo_assert_bitexact bin 内嵌 + GPU digest 双证）；
⑤ combined_mixed_deterministic：混合组合（cluster on 4px × wp on 0.25,
   5 Full/15 Hlod 混合互斥态）双跑末帧 digest 位级一致——覆盖机核（identity
   恰一次 + identity×粗簇域零交叠 + identity ∪ 粗簇域 ≡ WP Full 域恰等 +
   零双绘）bin 内嵌 fail-closed,跑绿即证；
⑥ dyn_skin_composition：--cluster-lod on × --dyn-demo refit 与 × --skin-demo
   组合 BENCH PASS（原互斥撤除;动态位置核验/蒙皮 device-host 逐顶点位级/MV
   三类硬门 bin 内嵌 fail-closed）；
⑦ g34_five_feature_leafxfull：g34 统一车道（纹理×slab×动态）× leaf×full
   digest_seq 逐帧 == --full 基线（五特性恒等排列锚;UV gather 位级锚 bin
   内嵌）；
⑧ g34_mixed_host_parity：g34 五特性混合组合 host 金标准对拍 p100 ≤ 冻结容差
   （milestones/g34/g34_budget.json g34.unified_lane.host_parity_tol 程序读,
   光线/材质/质感正确性机核——贴图采样×slab 预调制×代理常量面回退两臂一致）；
⑨ hzb_six_feature_culling：g34 --hzb on 六特性组合（HZB×cluster×wp×纹理×
   slab×动态）——重导出节点段进逐节点 BLAS 分解,金字塔 host/device 位级全等 +
   判定序列逐字节全等 + 零假阳性 bin 内嵌,真实剔除（occluded_p1 > 0）measured；
⑩ particles_oit_geo_deterministic：g35 粒子车道 --particles on --oit wboit ×
   cluster×wp 组合双跑 presented digest 位级一致（粒子/OIT 与场景重组正交）。

留窗如实登记（windowed_items,不冒充）：FIF×动态（RFC-0030 §4.3 L2 共享面
语义,须 RFC 修订行）/ FG 组合（G34 契约 out-of-scope「归后续波不预支」字面
管辖）/ HZB×蒙皮同车道（新 kernel 合并面）/ #96 代理属性保持简化 / 逐帧
device cut→AS 更新（#77/#89 合流窗）/ g31_window_present 冻结 bin 互斥维持
（五门回归锚,组合面在 g34/g35/g14_3 车道承载）。

三态：无 Vulkan/SPV/bistro 资产 → SKIP DEV_ENV_DEGRADE 退 0（非 fake pass;
RURIX_REQUIRE_REAL=1 翻硬 FAIL）。PASS-only evidence：过门才落
evidence/g36_geo_composition_gate_<ts>.json,FAIL 诊断落 .tmp 不污染。

用法：python ci/g36_geo_composition_smoke.py [--gate g36.wave1.geo_composition]
      [--frames 8] [--warmup 2] [--selftest]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = ROOT / "milestones" / "g36" / "g36_geo_composition_gate_evidence_schema.json"
WORK_DIR = ROOT / ".tmp" / "g36_gates" / "wave1_geo_composition"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g36.wave1.geo_composition"
TAG = "g36_geo_composition"
SCHEMA_ID = "rurix.g36.geo_composition_gate_evidence.v1"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
G34_SPV_SCENE = ROOT / ".tmp" / "g34_gates" / "unified" / "g34_unified_gi.spv"
G34_HZB_SPV = (
    ROOT / ".tmp" / "g34_gates" / "hzb" / "g34_unified_primary.spv",
    ROOT / ".tmp" / "g34_gates" / "hzb" / "g34_unified_shade.spv",
)
G35_SPV_PROBE = ROOT / ".tmp" / "g35_gates" / "render" / "g35_render_splat.spv"
G35_OIT_SPV_PROBE = ROOT / ".tmp" / "g35_gates" / "sort_oit" / "g35_oit_wboit_accum.spv"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
SLAB_ASSET = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"

FACT_IDS = [
    "builds_green",
    "packs_deterministic",
    "single_open_zero_drift",
    "combined_leafxfull_bitexact",
    "combined_mixed_deterministic",
    "dyn_skin_composition",
    "g34_five_feature_leafxfull",
    "g34_mixed_host_parity",
    "hzb_six_feature_culling",
    "particles_oit_geo_deterministic",
]

REQUIRED_KEYS = [
    "schema",
    "gate",
    "verdict",
    "facts",
    "scene",
    "tier",
    "packs",
    "single_open",
    "combined",
    "dyn_skin",
    "g34_lane",
    "particles",
    "windowed_items",
    "commands",
    "notes",
]

WINDOWED_ITEMS = [
    "FIF×动态（--inflight ≥2 × tlas_update/blas_refit）留窗：RFC-0030 §4.3 L2 定义 TLAS instance buffer / BLAS 顶点缓冲为共享 host 写面（在飞帧 ray query 读取中不可改写）——真修复 = 每槽实例/TLAS 副本 + 每槽 AS 描述符集 + provenance 逐槽 AS 代追踪,触冻结确定性协议面须 RFC 修订行（TODO #90 字面维持,如实拒跑不冒充）",
    "FG 组合（--fg × geo/纹理/slab/HZB）留窗：G34 契约 out-of-scope「FG/MFG 合流归后续波不预支」字面管辖（active 契约越权即违规）;g31_window_present --fg 臂维持既有闭集（冻结 bin,五门回归锚）",
    "HZB×蒙皮同车道留窗：G34-2/G34-3 并行分区面——合并需新 kernel（g34_unified_primary/shade 掩码双 TLAS × gi_skin hit 通道/蒙皮分派合体）+ host 金标准扩面,归后续波;g14_3 MegaSkin×cluster/wp 组合已在本门 ⑥ 验证",
    "#96 代理属性保持简化留窗：粗簇/cell 代理三角 tritex 强制 −1 走常量面回退（cluster/cell 面积加权均值;UV/法线经 QEM 属性保持简化后方可贴图采样——meshopt simplifyWithAttributes 族评估窗）",
    "逐帧 device cut→AS 更新留窗（#77/#89 合流窗）：出帧几何冻结于装配期契约相机 cut/选层（g31 车道同纪律,统计 sidecar 如实登记不冒充逐帧 Nanite）",
    "g31_window_present 闭集互斥维持（冻结 bin = 五门回归锚）：其 --cluster-lod/--wp-hlod × --hzb/--textures/--slab-table 互斥字面不回写,组合面由 g34_full_lane（窗口）/ g14_3_pipeline_perf（bench）/ g35_particle_lane（粒子）承载",
]

FAILURES: list[str] = []
COMMANDS: list[dict] = []

CL_BAKE_RE = re.compile(r"bake OK blocks=\d+ \(degraded=(\d+)\).*?sha256=([0-9a-f]{64}) bake_ms=[0-9.]+")
WP_BAKE_RE = re.compile(r"sha256=([0-9a-f]{64}) bake_ms=[0-9.]+")
GEO_BENCH_RE = re.compile(
    r"geo 组合（cluster×wp）identity=(\d+) coarse=(\d+)（(\d+) 簇）straddle_fallback=(\d+)（(\d+) 簇）wp_proxy=(\d+) out=(\d+)"
)
GEO_G35_RE = re.compile(
    r"geo 组合 identity=(\d+) coarse=(\d+) straddle_fallback=(\d+) wp_proxy=(\d+) out=(\d+)"
)
SRC_TRIS_RE = re.compile(r"tris: src=(\d+)")
PARITY_RE = re.compile(r"host 金标准对拍 p100=([0-9.eE+-]+)（tol=([0-9.eE+-]+)")
HZB_NODES_RE = re.compile(r"\[hzb\] 装配 .*? nodes=(\d+)")
HZB_OCCL_RE = re.compile(r"occluded_p1=(\d+)")
HZB_PARITY_RE = re.compile(r"接线态对拍 mips=\d+ 位级全等")
G35_PRESENTED_RE = re.compile(r"presented=(sha256:[0-9a-f]{64})")


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def target_dir() -> Path:
    return Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))


def run_cmd(argv: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def bench_digest(out_root: Path) -> str | None:
    p = out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    if not p.is_file():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8")).get("last_frame_digest")
    except json.JSONDecodeError:
        return None


def seq_sha(seq: list[str]) -> str:
    import hashlib

    return "sha256:" + hashlib.sha256("\n".join(seq).encode()).hexdigest()


def run_selftest() -> int:
    """离线自证：正则六联 GREEN + schema required/facts 闭集互核（无 GPU 面）。"""
    fails = []
    m = GEO_BENCH_RE.search(
        "geo 组合（cluster×wp）identity=236894 coarse=11524（1901 簇）straddle_fallback=3270（79 簇）wp_proxy=388975 out=637393"
    )
    if not m or m.group(7) != "637393":
        fails.append("GEO_BENCH_RE 解析失败")
    m = GEO_G35_RE.search("geo 组合 identity=211365 coarse=16686 straddle_fallback=7193 wp_proxy=388975 out=617026")
    if not m or m.group(5) != "617026":
        fails.append("GEO_G35_RE 解析失败")
    m = PARITY_RE.search("帧 3 host 金标准对拍 p100=2.599419e-4（tol=7.937318e-4,budget x）")
    if not m or m.group(2) != "7.937318e-4":
        fails.append("PARITY_RE 解析失败")
    m = HZB_NODES_RE.search("[g34_full_lane]: [hzb] 装配 scene=bistro-interior tris=637393 quads=0 points=4 nodes=337 output=x")
    if not m or m.group(1) != "337":
        fails.append("HZB_NODES_RE 解析失败")
    if not HZB_OCCL_RE.search("tested=2248 occluded_p1=58 flipped_p2=0"):
        fails.append("HZB_OCCL_RE 解析失败")
    if not G35_PRESENTED_RE.search("presented=sha256:" + "d" * 64):
        fails.append("G35_PRESENTED_RE 解析失败")
    if not SCHEMA_PATH.is_file():
        fails.append(f"schema 缺失 {SCHEMA_PATH}")
    else:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        if set(schema.get("required", [])) != set(REQUIRED_KEYS):
            fails.append("schema required 与校验键集不等")
        enum = schema["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        if enum != FACT_IDS:
            fails.append("schema facts enum 与 FACT_IDS 不等（闭集漂移）")
    if len(WINDOWED_ITEMS) < 5:
        fails.append("windowed_items 少于 5（留窗登记面不完整）")
    if fails:
        for f in fails:
            print(f"[{TAG}] selftest FAIL: {f}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (6 正则 GREEN + schema/facts 闭集互核 + 留窗登记面)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=2)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    # ── schema 在树 + required/facts 闭集互核 ──
    check(SCHEMA_PATH.is_file(), f"schema 文件缺失: {SCHEMA_PATH}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        check(set(schema.get("required", [])) == set(REQUIRED_KEYS), "schema required 与校验键集不等")
        check(
            schema["properties"]["facts"]["items"]["properties"]["id"]["enum"] == FACT_IDS,
            "schema facts enum 与 FACT_IDS 不等",
        )

    # ── ① builds_green ──
    r = run_cmd([
        "cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", "g14_3_pipeline_perf", "--bin", "g34_full_lane", "--bin", "g35_particle_lane",
        "--quiet",
    ], timeout=7200)
    check(r.returncode == 0, f"rurix-render 构建失败: {(r.stdout + r.stderr)[-600:]}")
    r = run_cmd([
        "cargo", "build", "--release", "-p", "rurix-asset",
        "--bin", "g31_cluster_lod_bake", "--bin", "g31_wp_hlod_bake", "--quiet",
    ], timeout=7200)
    check(r.returncode == 0, f"rurix-asset bake 构建失败: {(r.stdout + r.stderr)[-600:]}")
    rel = target_dir() / "release"
    perf = rel / f"g14_3_pipeline_perf{EXE_SUFFIX}"
    g34 = rel / f"g34_full_lane{EXE_SUFFIX}"
    g35 = rel / f"g35_particle_lane{EXE_SUFFIX}"
    cl_bake = rel / f"g31_cluster_lod_bake{EXE_SUFFIX}"
    wp_bake = rel / f"g31_wp_hlod_bake{EXE_SUFFIX}"
    for p in (perf, g34, g35, cl_bake, wp_bake):
        check(p.is_file(), f"产物缺失: {p}")

    # ── device 前置面（SPV 依赖既有构建产物;缺失 = 三态 SKIP,不在本门现编
    #    ——g34/g35 kernel 编译面归各自门脚本保障）──
    degrade_reasons: list[str] = []
    for f in SPV_FILES:
        if not (SPV_DIR / f).is_file():
            degrade_reasons.append(f"车道 SPV 缺失 {SPV_DIR / f}（先跑 ci/g31_cluster_lod_smoke.py 编译面）")
    if not G34_SPV_SCENE.is_file():
        degrade_reasons.append(f"g34 统一 kernel SPV 缺失 {G34_SPV_SCENE}（先跑 ci/g34_unified_lane_smoke.py 编译面）")
    for p in G34_HZB_SPV:
        if not p.is_file():
            degrade_reasons.append(f"g34 HZB kernel SPV 缺失 {p}（先跑 ci/g34_hzb_unified_smoke.py 编译面）")
    if not G35_SPV_PROBE.is_file() or not G35_OIT_SPV_PROBE.is_file():
        degrade_reasons.append("g35 粒子/OIT kernel SPV 缺失（先跑 ci/g35_render_smoke.py / ci/g35_sort_oit_smoke.py 编译面）")
    if not BISTRO_GLTF.is_file():
        degrade_reasons.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not SLAB_ASSET.is_file():
        degrade_reasons.append(f"slab 侧表资产缺失 {SLAB_ASSET}")

    ran = False
    ev: dict = {}
    if not FAILURES and not degrade_reasons:
        WORK_DIR.mkdir(parents=True, exist_ok=True)
        rxcs = WORK_DIR / "bistro.rxcs"
        rxcp = WORK_DIR / "bistro.rxcp"
        rxwh = WORK_DIR / "bistro.rxwh"
        env = dict(os.environ)

        # ── ② packs_deterministic（纯 host,不占 GPU 锁）──
        rd = run_cmd([str(perf), "--dump-scene", "--scene", "bistro-interior", "--out", str(rxcs)], timeout=1800)
        check(rd.returncode == 0, f"dump-scene 非零退出: {(rd.stdout + rd.stderr)[-400:]}")
        rb = run_cmd([str(cl_bake), "--scene-dump", str(rxcs), "--out", str(rxcp), "--double-build"], timeout=3600)
        outb = rb.stdout + rb.stderr
        check(rb.returncode == 0, f"cluster bake 非零退出: {outb[-400:]}")
        check("double-build 字节相等 OK" in outb, "cluster bake double-build 确定性门缺证")
        mcl = CL_BAKE_RE.search(outb)
        check(mcl is not None, f"cluster bake OK 行不可解析: {outb[-300:]}")
        rxcp_sha = mcl.group(2) if mcl else ""
        if mcl:
            check(mcl.group(1) == "0", f"cluster bake 降级块 {mcl.group(1)} ≠ 0")
        rw = run_cmd([
            str(wp_bake), "--scene-dump", str(rxcs), "--out", str(rxwh),
            "--cell-size", "4.0", "--levels", "4", "--double-build",
        ], timeout=3600)
        outw = rw.stdout + rw.stderr
        check(rw.returncode == 0, f"wp bake 非零退出: {outw[-400:]}")
        check("double-build 字节相等 OK" in outw, "wp bake double-build 确定性门缺证")
        mwp = WP_BAKE_RE.search(outw)
        check(mwp is not None, f"wp bake OK 行不可解析: {outw[-300:]}")
        rxwh_sha = mwp.group(1) if mwp else ""

        cl_args = ["--cluster-pack", str(rxcp)]
        wp_args = ["--wp-pack", str(rxwh)]
        single_open: dict = {}
        combined: dict = {}
        dyn_skin: dict = {}
        g34_doc: dict = {}
        particles: dict = {}

        with gpu_device_lock(purpose="g36 wave1 geo 组合 device 腿"):
            def bench(
                extra: list[str], out_sub: str, frames: int | None = None, warmup: int | None = None
            ) -> tuple[str, Path, int]:
                out_root = WORK_DIR / out_sub
                argv = [
                    str(perf), "--bench", "--scene", "bistro-interior", "--tier", "100",
                    "--backend", "tsr_device", "--frames", str(frames or args.frames),
                    "--warmup", str(warmup if warmup is not None else args.warmup),
                    "--out-root", str(out_root),
                ] + extra
                rr = run_cmd(argv, timeout=3600, env=env)
                return rr.stdout + rr.stderr, out_root, rr.returncode

            # ── ③ single_open_zero_drift ──
            out_off, root_off, rc = bench([], "off")
            if "SKIP DEV_ENV_DEGRADE" in out_off or "skipped_dev_env" in out_off:
                degrade_reasons.append(f"bench off dev_env 降级: {out_off.strip()[-300:]}")
            else:
                check(rc == 0, f"bench off 非零退出: {out_off[-400:]}")
                out_leaf, root_leaf, rc = bench(["--cluster-lod", "leaf"] + cl_args, "leaf")
                check(rc == 0, f"bench leaf 非零退出: {out_leaf[-400:]}")
                out_wpf, root_wpf, rc = bench(["--wp-hlod", "full"] + wp_args, "wpfull")
                check(rc == 0, f"bench wpfull 非零退出: {out_wpf[-400:]}")
                d_off, d_leaf, d_wpf = bench_digest(root_off), bench_digest(root_leaf), bench_digest(root_wpf)
                check(all(x is not None for x in (d_off, d_leaf, d_wpf)), "off/leaf/wpfull receipt digest 缺失")
                if d_off and d_leaf and d_wpf:
                    check(d_off == d_leaf == d_wpf, f"单开零漂移破坏: off={d_off} leaf={d_leaf} wpfull={d_wpf}")
                    single_open = {
                        "off_digest": d_off,
                        "leaf_digest": d_leaf,
                        "wpfull_digest": d_wpf,
                        "bitexact": d_off == d_leaf == d_wpf,
                    }

                # ── ④ combined_leafxfull_bitexact ──
                out_lxf, root_lxf, rc = bench(
                    ["--cluster-lod", "leaf"] + cl_args + ["--wp-hlod", "full"] + wp_args, "leafxfull"
                )
                check(rc == 0, f"bench leaf×full 非零退出: {out_lxf[-400:]}")
                d_lxf = bench_digest(root_lxf)
                check(d_lxf is not None, "leaf×full receipt digest 缺失")
                if d_off and d_lxf:
                    check(d_off == d_lxf, f"组合极限锚破坏: off={d_off} ≠ leaf×full={d_lxf}")

                # ── ⑤ combined_mixed_deterministic ──
                mixed_extra = (
                    ["--cluster-lod", "on", "--cluster-error-px", "4.0"] + cl_args
                    + ["--wp-hlod", "on", "--wp-threshold-l0", "0.25"] + wp_args
                )
                out_ma, root_ma, rc = bench(mixed_extra, "mixed_a")
                check(rc == 0, f"bench mixed#1 非零退出: {out_ma[-400:]}")
                out_mb, root_mb, rc = bench(mixed_extra, "mixed_b")
                check(rc == 0, f"bench mixed#2 非零退出: {out_mb[-400:]}")
                mg = GEO_BENCH_RE.search(out_ma)
                check(mg is not None, f"mixed geo 组合行不可解析: {out_ma[-400:]}")
                ms = SRC_TRIS_RE.search(out_ma)
                d_ma, d_mb = bench_digest(root_ma), bench_digest(root_mb)
                check(d_ma is not None and d_mb is not None, "mixed receipt digest 缺失")
                if d_ma and d_mb:
                    check(d_ma == d_mb, f"mixed 双跑 digest 漂移: {d_ma} ≠ {d_mb}")
                if mg and ms and d_ma and d_mb:
                    out_tris = int(mg.group(7))
                    src_tris = int(ms.group(1))
                    check(out_tris < src_tris, f"mixed 三角未下降: {out_tris} ≥ {src_tris}")
                    combined = {
                        "leafxfull_digest": d_lxf or "",
                        "leafxfull_bitexact": bool(d_off and d_lxf and d_off == d_lxf),
                        "mixed": {
                            "wp_threshold_l0": 0.25,
                            "cluster_error_px": 4.0,
                            "identity_tris": int(mg.group(1)),
                            "coarse_tris": int(mg.group(2)),
                            "coarse_clusters": int(mg.group(3)),
                            "straddle_clusters": int(mg.group(5)),
                            "straddle_fallback_tris": int(mg.group(4)),
                            "wp_proxy_tris": int(mg.group(6)),
                            "out_tris": out_tris,
                            "src_tris": src_tris,
                            "digest_a": d_ma,
                            "digest_b": d_mb,
                            "deterministic": d_ma == d_mb,
                        },
                    }

                # ── ⑥ dyn_skin_composition（蒙皮臂 = B5 标定口径 100+10 帧——
                #    ci/g31_skinning_wiring_smoke.py「≥100 帧硬线」字面:窗级
                #    聚合真动门（max host_motion ≥1px）需双谐波高动相位入窗,
                #    短窗低动相位（如 10 帧 0.51px）为合法动画相位非组合缺陷）──
                out_dyn, _, rc = bench(
                    ["--cluster-lod", "on", "--cluster-error-px", "4.0"] + cl_args + ["--dyn-demo", "refit"],
                    "cl_dyn",
                )
                check(rc == 0 and "BENCH PASS" in out_dyn, f"cluster×dyn 组合失败: {out_dyn[-400:]}")
                out_skin, _, rc = bench(
                    ["--cluster-lod", "on", "--cluster-error-px", "4.0"] + cl_args + ["--skin-demo"],
                    "cl_skin",
                    frames=100,
                    warmup=10,
                )
                check(rc == 0 and "BENCH PASS" in out_skin, f"cluster×skin 组合失败: {out_skin[-400:]}")
                dyn_skin = {
                    "cluster_dyn_pass": "BENCH PASS" in out_dyn,
                    "cluster_skin_pass": "BENCH PASS" in out_skin,
                }

                # ── ⑦/⑧ g34 五特性（真窗口 --hidden）──
                def g34_run(extra: list[str], ev_name: str) -> tuple[str, dict | None, int]:
                    ev_path = WORK_DIR / ev_name
                    argv = [
                        str(g34), "--frames", "12", "--warmup", "2", "--tier", "100",
                        "--full", "--slab-table", str(SLAB_ASSET), "--auto-move", "orbit",
                        "--hidden", "--evidence", str(ev_path),
                    ] + extra
                    rr = run_cmd(argv, timeout=3600, env=env)
                    doc = None
                    if ev_path.is_file():
                        try:
                            doc = json.loads(ev_path.read_text(encoding="utf-8"))
                        except json.JSONDecodeError:
                            doc = None
                    return rr.stdout + rr.stderr, doc, rr.returncode

                out_base, doc_base, rc = g34_run([], "g34_base.json")
                check(rc == 0 and doc_base is not None, f"g34 --full 基线失败: {out_base[-400:]}")
                out_g34lxf, doc_lxf, rc = g34_run(
                    ["--cluster-lod", "leaf"] + cl_args + ["--wp-hlod", "full"] + wp_args, "g34_leafxfull.json"
                )
                check(rc == 0 and doc_lxf is not None, f"g34 leaf×full 失败: {out_g34lxf[-400:]}")
                seq_equal = False
                if doc_base and doc_lxf:
                    seq_equal = doc_base.get("digest_seq") == doc_lxf.get("digest_seq") and bool(doc_base.get("digest_seq"))
                    check(seq_equal, "g34 五特性 leaf×full digest_seq ≠ --full 基线（恒等排列锚破坏）")
                out_g34mix, doc_mix, rc = g34_run(mixed_extra, "g34_mixed.json")
                check(rc == 0 and doc_mix is not None, f"g34 混合组合失败: {out_g34mix[-400:]}")
                mp = PARITY_RE.search(out_g34mix)
                check(mp is not None, f"g34 host parity 行不可解析: {out_g34mix[-400:]}")
                p100, tol = (float(mp.group(1)), float(mp.group(2))) if mp else (1.0, 0.0)
                check(p100 <= tol, f"g34 混合组合 host parity 越容差: {p100} > {tol}")

                # ── ⑨ hzb_six_feature_culling ──
                out_hzb, _, rc = g34_run(["--hzb", "on"] + mixed_extra, "g34_hzb_mixed.json")
                check(rc == 0, f"g34 HZB 六特性组合失败: {out_hzb[-400:]}")
                mn = HZB_NODES_RE.search(out_hzb)
                occl = [int(x) for x in HZB_OCCL_RE.findall(out_hzb)]
                check(mn is not None, "HZB 重导出节点数不可解析")
                check(HZB_PARITY_RE.search(out_hzb) is not None, "HZB 金字塔位级对拍见证行缺失")
                check(bool(occl) and max(occl) > 0, f"HZB 真实剔除未发生: occluded_p1 全 0（{occl[:4]}…）")
                g34_doc = {
                    "base_digest_seq_sha": seq_sha(doc_base.get("digest_seq", [])) if doc_base else "",
                    "leafxfull_digest_seq_sha": seq_sha(doc_lxf.get("digest_seq", [])) if doc_lxf else "",
                    "five_feature_seq_equal": seq_equal,
                    "host_parity": {
                        "p100": p100,
                        "tol": tol,
                        "in_tol": p100 <= tol,
                        "source": "milestones/g34/g34_budget.json g34.unified_lane.host_parity_tol（冻结容差程序读禁手写）",
                    },
                    "hzb": {
                        "regrouped_nodes": int(mn.group(1)) if mn else 0,
                        "occluded_p1": max(occl) if occl else 0,
                        "pyramid_parity_witnessed": HZB_PARITY_RE.search(out_hzb) is not None,
                    },
                }

                # ── ⑩ particles_oit_geo_deterministic（t50 键域合规档）──
                def g35_run(name: str) -> tuple[str, int]:
                    argv = [
                        str(g35), "--frames", "24", "--warmup", "4", "--tier", "50",
                        "--particles", "on", "--oit", "wboit",
                        "--cluster-lod", "on", "--cluster-error-px", "4.0",
                    ] + cl_args + ["--wp-hlod", "on", "--wp-threshold-l0", "0.25"] + wp_args + [
                        "--evidence", str(WORK_DIR / name),
                    ]
                    rr = run_cmd(argv, timeout=3600, env=env)
                    return rr.stdout + rr.stderr, rr.returncode

                out_pa, rc = g35_run("g35_geo_a.json")
                check(rc == 0, f"g35 粒子×OIT×geo #1 失败: {out_pa[-400:]}")
                out_pb, rc = g35_run("g35_geo_b.json")
                check(rc == 0, f"g35 粒子×OIT×geo #2 失败: {out_pb[-400:]}")
                pa = G35_PRESENTED_RE.search(out_pa)
                pb = G35_PRESENTED_RE.search(out_pb)
                pg = GEO_G35_RE.search(out_pa)
                check(pa is not None and pb is not None, "g35 presented digest 不可解析")
                check(pg is not None, "g35 geo 组合行不可解析")
                if pa and pb and pg:
                    check(pa.group(1) == pb.group(1), f"g35 双跑 presented digest 漂移: {pa.group(1)} ≠ {pb.group(1)}")
                    particles = {
                        "oit": "wboit",
                        "tier": 50,
                        "digest_a": pa.group(1),
                        "digest_b": pb.group(1),
                        "deterministic": pa.group(1) == pb.group(1),
                        "out_tris": int(pg.group(5)),
                    }

                if not FAILURES:
                    ran = True
                    ev = {
                        "schema": SCHEMA_ID,
                        "gate": GATE_KEY,
                        "verdict": "PASS",
                        "facts": [{"id": fid, "status": "PASS"} for fid in FACT_IDS],
                        "scene": "bistro-interior",
                        "tier": 100,
                        "packs": {
                            "rxcp_sha256": rxcp_sha,
                            "rxcp_double_build_equal": True,
                            "rxwh_sha256": rxwh_sha,
                            "rxwh_double_build_equal": True,
                        },
                        "single_open": single_open,
                        "combined": combined,
                        "dyn_skin": dyn_skin,
                        "g34_lane": g34_doc,
                        "particles": particles,
                        "windowed_items": WINDOWED_ITEMS,
                        "commands": COMMANDS,
                        "notes": [
                            "W1 provenance 事实源：TriProvenance（Src|簇粗代理|cell 代理）+ geo_rebuild 统一重建 + 侧表 gather（UV 位保真/代理 tritex 强制 −1 常量面回退）+ regroup_nodes 节点段重导出（AABB 自重建几何精确重算）;选层机核抽取共用（cluster_lod_select/wp_hlod_select）,单开路径 0-语义漂移由本门 ③ 三臂 digest 机核",
                            "W2 组合语义：WP cell 互斥选层先行 → Full 域内簇 cut → 组共享多父 DAG 下粗簇集合化判定（⊆F 出帧/跨界叶级回退差集化/全外域归 cell 代理）;覆盖机核（identity 恰一次 + identity×粗簇域零交叠 + ≡ F 恰等）bin 内嵌 fail-closed",
                            "光线/材质/质感机核：g34 host 金标准对拍（贴图双线性×〔mod×R_slot〕/常量×R_slot/代理常量回退两臂同一补丁后数组）≤ 冻结容差;HZB 金字塔 host/device 位级 + 零假阳性;动态位置核验/蒙皮逐顶点位级/MV 三类硬门维持",
                            "出帧几何冻结于装配期契约相机 cut/选层（#77/#89 合流窗）;粗簇级源覆盖多父可重叠为 DAG 冻结语义（与单开 on 模式同信任基,EXR diff 门在案）,组合面新增双绘由覆盖机核拒绝",
                        ],
                    }

    if degrade_reasons:
        for d in degrade_reasons:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL;构建/schema 面仍真跑）")
        return 0
    if FAILURES:
        diag = WORK_DIR / "fail_diagnostic.json"
        diag.parent.mkdir(parents=True, exist_ok=True)
        diag.write_text(
            json.dumps({"failures": FAILURES, "commands": COMMANDS}, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    if not ran:
        print(f"[{TAG}] FAIL: device 腿未真跑（无 degrade 原因但无真跑证据）", file=sys.stderr)
        return 1
    ts = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    ev_path = ROOT / "evidence" / f"g36_geo_composition_gate_{ts}.json"
    ev_path.write_text(json.dumps(ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（十 facts 全绿：构建/双包确定/单开零漂移/leaf×full 位级锚/"
        f"混合双跑位级/dyn+skin 组合/g34 五特性恒等锚/host parity 入容差/HZB 六特性真剔除/粒子×OIT×geo 位级）"
        f" evidence={ev_path}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
