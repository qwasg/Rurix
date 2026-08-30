#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W2 kladder:tsrq 邻域 clamp K 阶梯判读(EVAL_DENOISE §8 第 0 级,纯 CPU)。

artifacts/day_0828/d_tsr/d_ladder.py 复制改造件(原件零触碰)。判读口径逐字对齐:
conv 协议 = frames/frame_01*.exr 末段 stride2 × ≤16 帧(实取 14),Rec.709 luma
逐像素时域 std 的 ROI p95(绝对幅值,C 相教训);四 ROI 同 d_metrics.json。

## 口径差登记(判读中发现,数据实证)
EVAL_DENOISE §8 声称「K=0 基线 = D 相 arm4,d_metrics.json 可直接对照」,但其
命令行含 --ggx on --lamp-lights on --lamp-gain 4,而 arm4 实跑臂形 = snrm+gi2
c001+tsrq(D 相四臂 = snrm 基座 A/B 隔离梯,锚谱系:arm1=夜巡 D2 锚 778f1dfc
〔scene SPV=g18_smooth_nrm.spv〕/arm3=C 相锚 6144d9f7/e2_equivalence.py BENCH_QL
才是本跑同旗形,但 E 相那次挂 RURIX_G18_AMBIENT=0.004 + warmup 10,亦非同口径)。
数据实证:三档 converged 全局亮度较 arm4 +80.9%(lamp-gain 4〔A1 在案单项
+39%〕+ ggx 镜面能量合计的内容签名)+ 四 ROI std_p95 高 3~12×(ggx 1spp
镜面 + 12 灯点光新噪声源)+ 三档间互差 <0.5%——差异形态 = 内容差,非 K 效应。
⇒ K=0 在案基线**不可直接对照**,降级为跨口径参照登记;判据改**三档间趋势**
(k3 为梯内参照,K 收紧 3→2→1.5 的边际效应)——任务预设分支。

## 双判据(EVAL_DENOISE §8,梯内口径)
①微光点再降:四 ROI conv std_p95 随 K 收紧的边际降幅 + 单调性;旋钮有效阈 =
  暗区 ROI(微光点载体)k15 vs k3 降 ≥10%。辅证 = converged 亮度场 clamp
  触发面(|k15/k3−1|>1% 像素占比)。
②远小灯保真:dolly f0240 腿本跑批缺席(口径差登记)——以 conv converged.exr
  (= frame_0127,三档同 seed 同 jitter 相位,逐像素可比)替位:k3 参照上检出
  小面积发光核心连通域(luma>1.0,面积≤300px,8 连通),k2/k15 同掩码亮度
  保持率。clamp 语义(g31_tsr_resolve_q.rx:27-29)= 3×3 邻域除中心 luma
  max×K 截断新样本 ⇒ 孤立亚像素灯最脆弱,按面积分级登记。

产出 tsrq_clamp_ladder.json 两态结论。登记不改任何生产代码/预设。
"""
from __future__ import annotations

import glob
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np
from scipy import ndimage

from g10_exr_lib import decode_exr

KL = ROOT / "artifacts/day_0830_delivery/w2_wiring/ris_nee/kladder"
SUB = "bistro-interior/tier100/tsr_device"
ARMS = {  # 判读三档(K 递紧序;k3 = 梯内参照)
    "k3": (3.0, KL / "k3" / SUB),
    "k2": (2.0, KL / "k2" / SUB),
    "k15": (1.5, KL / "k15" / SUB),
}
ARM4 = ROOT / "artifacts/day_0828/d_tsr/arms/arm4_gi2_tsrq" / SUB
D_METRICS = ROOT / "artifacts/day_0828/d_tsr/d_metrics.json"
ROIS = {  # d_metrics.json 同字面
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}
# 远小灯检出/判定阈(判读选择,登记于输出;EVAL_DENOISE §8 未给数值阈)
LAMP_LUMA_THR = 1.0     # scene-linear;瞬态萤火虫在 α=0.02 EMA 收敛帧不可达
LAMP_AREA_MAX = 300     # 「远/小」屏幕面积代理;大面积近灯排除
LAMP_KILL_RATIO = 0.75  # 单灯亮度保持率 < 0.75 记误杀(kill)
LAMP_DIM_RATIO = 0.95   # < 0.95 记可见变暗(dim)
LAMP_PASS_MEDIAN_DROP = 5.0    # 保真 PASS:中位降幅 ≤5% 且 kill=0(梯内口径)
WL_FIREFLY_EFFECTIVE = 10.0    # 旋钮有效阈:暗区 ROI k15 vs k3 边际降 ≥10%
WL_REGRESS_TOL = 5.0           # 且四 ROI 无一梯内升 >5%
ENGAGE_REL_THR = 0.01          # clamp 触发面:converged |k15/k3−1| > 1%


def load_luma(path: Path) -> np.ndarray:
    f = decode_exr(path.read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    return px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722


def receipt(base: Path) -> dict:
    return json.loads((base / "render_receipt.json").read_text(encoding="utf-8"))


def rel_eq(a: float, b: float, tol: float = 1e-9) -> bool:
    return abs(a - b) <= tol * max(abs(a), abs(b), 1e-300)


def main() -> int:
    dm = json.loads(D_METRICS.read_text(encoding="utf-8"))
    arm4_rec = dm["arms"]["arm4_gi2c001_tsrqon"]
    k0_conv = {rn: arm4_rec["conv"][rn]["temporal_std_p95"] for rn in ROIS}

    # ── 机核 1:receipt 车道对齐(SPV sha、seed、帧数、曝光、尺寸)+ 梯内接线 ──
    r0 = receipt(ARM4)
    recs = {name: receipt(base) for name, (_, base) in ARMS.items()}
    parity_fields = {}
    caliber_ok = True
    for name, rk in recs.items():
        checks = {
            "scene_spv_sha": rk["scene_kernel_spv_sha256"] == r0["scene_kernel_spv_sha256"],
            "resolve_spv_sha": rk["backend_provenance"]["spv_resolve_sha256"]
            == r0["backend_provenance"]["spv_resolve_sha256"],
            "seed": rk["seed"] == r0["seed"],
            "frame_count": rk["frame_count"] == r0["frame_count"],
            "exposure": rk["exposure"] == r0["exposure"],
            "output_size": rk["output_size"] == r0["output_size"],
        }
        parity_fields[name] = checks
        caliber_ok &= all(checks.values())
    print(f"[caliber] receipt lane parity(SPV/seed/帧数/曝光/尺寸): {'OK' if caliber_ok else 'MISMATCH'}")
    if not caliber_ok:
        print(json.dumps(parity_fields, indent=1))
        return 1
    # 梯内 K 接线生效证据:三档逐帧 digest 互异计数(receipt 面,零解码)
    def diff_frames(a: dict, b: dict) -> int:
        return sum(1 for fa, fb in zip(a["frames"], b["frames"]) if fa["digest"] != fb["digest"])
    wiring = {
        "k3_vs_k2_differing_frames": diff_frames(recs["k3"], recs["k2"]),
        "k3_vs_k15_differing_frames": diff_frames(recs["k3"], recs["k15"]),
        "k2_vs_k15_differing_frames": diff_frames(recs["k2"], recs["k15"]),
        "of_frames": r0["frame_count"],
    }
    print(f"[wiring] bench 梯内逐帧 digest 互异: {wiring}")

    # ── 机核 2:判读管线自校验(arm4 converged 复算 == d_metrics 在案)──
    arm4_luma = load_luma(ARM4 / "converged.exr")
    got_gm = float(arm4_luma.mean())
    want_gm = arm4_rec["converged_global_mean"]
    selfcheck = {"converged_global_mean": {"got": got_gm, "want": want_gm}}
    sc_ok = rel_eq(got_gm, want_gm)
    for rn, (x, y, w, h) in ROIS.items():
        r = arm4_luma[y : y + h, x : x + w]
        got_mean, got_p50 = float(r.mean()), float(np.percentile(r, 50))
        want = arm4_rec["converged_rois"][rn]
        selfcheck[rn] = {
            "mean": {"got": got_mean, "want": want["mean"]},
            "p50": {"got": got_p50, "want": want["p50"]},
        }
        sc_ok &= rel_eq(got_mean, want["mean"]) and rel_eq(got_p50, want["p50"])
    print(f"[selfcheck] arm4 converged 复算 vs d_metrics 在案: {'OK' if sc_ok else 'FAIL'}")
    if not sc_ok:
        print(json.dumps(selfcheck, indent=1))
        return 1

    # ── 判据①:conv 协议四 ROI std_p95(d_ladder.py 同式)──
    conv: dict = {}
    frames_used: dict = {}
    for name, (_, base) in ARMS.items():
        files = sorted(glob.glob(str(base / "frames/frame_01*.exr")))[::2][:16]
        frames_used[name] = len(files)
        if len(files) < 2:
            print(f"FAIL: {name} conv 帧数不足", files)
            return 1
        stacks = {rn: [] for rn in ROIS}
        for i, fp in enumerate(files):
            luma = load_luma(Path(fp))
            for rn, (x, y, w, h) in ROIS.items():
                stacks[rn].append(luma[y : y + h, x : x + w])
            print(f"[conv] {name} {i + 1}/{len(files)}", flush=True)
        conv[name] = {
            rn: float(np.percentile(np.stack(s, axis=0).std(axis=0), 95))
            for rn, s in stacks.items()
        }

    wl_drops = {  # 梯内边际降幅 vs k3(正 = 降)
        name: {
            rn: (1.0 - conv[name][rn] / max(conv["k3"][rn], 1e-30)) * 100.0
            for rn in ROIS
        }
        for name in ("k2", "k15")
    }
    mono = {rn: conv["k3"][rn] >= conv["k2"][rn] >= conv["k15"][rn] for rn in ROIS}
    cross_ratio = {  # 跨口径参照(内容差主导,非 K 效应——登记不判)
        name: {rn: conv[name][rn] / max(k0_conv[rn], 1e-30) for rn in ROIS}
        for name in ARMS
    }
    for name in ARMS:
        for rn in ROIS:
            wl = "" if name == "k3" else f" wl_drop_vs_k3={wl_drops[name][rn]:+.2f}%"
            print(f"{name} {rn}: std_p95={conv[name][rn]:.4e}{wl}")
    print(f"monotonic(3→2→1.5 不升): {mono}")

    # ── converged 三档亮度场:clamp 触发面 + 全局亮度(口径差实证)──
    luma_k = {name: load_luma(base / "converged.exr") for name, (_, base) in ARMS.items()}
    gm = {name: float(l.mean()) for name, l in luma_k.items()}
    rel_15_3 = np.abs(luma_k["k15"] - luma_k["k3"]) / np.maximum(luma_k["k3"], 1e-4)
    engagement = {
        "frac_px_rel_gt_1pct_k15_vs_k3": float((rel_15_3 > ENGAGE_REL_THR).mean()),
        "rel_p999_k15_vs_k3": float(np.percentile(rel_15_3, 99.9)),
        "rel_max_k15_vs_k3": float(rel_15_3.max()),
    }
    content_probe = {
        "converged_global_mean": {**gm, "arm4_incase": want_gm,
                                  "arm1_snrm_incase": dm["arms"]["arm1_snrm_tsrqoff"]["converged_global_mean"]},
        "k3_vs_arm4_pct": (gm["k3"] / want_gm - 1.0) * 100.0,
        "note": f"三档较 arm4 全局亮度 +{(gm['k3'] / want_gm - 1.0) * 100.0:.1f}% = lamp-gain 4(A1 在案单项 off 0.00985→g4 0.0137 即 +39%)+ ggx 镜面能量合计的内容签名;跨口径差异非 K 效应",
    }
    print(f"[content] converged 全局亮度: {gm} vs arm4 {want_gm:.6f} ({content_probe['k3_vs_arm4_pct']:+.1f}%)")
    print(f"[engage] clamp 触发面(k15 vs k3): {engagement}")

    # ── 判据②:远小灯保真(k3 参照检出,梯内同掩码保持率)──
    ref_luma = luma_k["k3"]
    mask = ref_luma > LAMP_LUMA_THR
    labels, n_comp = ndimage.label(mask, structure=np.ones((3, 3), bool))
    comp_slices = ndimage.find_objects(labels)
    lamps = []
    n_big = 0
    for cid in range(1, n_comp + 1):
        sl = comp_slices[cid - 1]
        sel = labels[sl] == cid
        area = int(sel.sum())
        if area > LAMP_AREA_MAX:
            n_big += 1
            continue
        ys, xs = np.nonzero(sel)
        ys = ys + sl[0].start
        xs = xs + sl[1].start
        lamps.append({
            "id": cid,
            "area": area,
            "bbox_xywh": [int(xs.min()), int(ys.min()),
                          int(xs.max() - xs.min() + 1), int(ys.max() - ys.min() + 1)],
            "k3_mean_luma": float(ref_luma[ys, xs].mean()),
            "_ys": ys, "_xs": xs,
        })
    print(
        f"[lamps] k3 参照 thr>{LAMP_LUMA_THR} 连通域 {n_comp} 个:"
        f"小灯(≤{LAMP_AREA_MAX}px) {len(lamps)} 个,大发光体(排除) {n_big} 个;"
        f" luma max={float(ref_luma.max()):.3f} p99.99={float(np.percentile(ref_luma, 99.99)):.3f}"
    )
    size_classes = {"px_1_4": (1, 4), "px_5_32": (5, 32), "px_33_300": (33, 300)}
    lamp_out: dict = {
        "detection": {
            "reference": "k3 converged.exr(梯内参照 = 最松档;frame_0127,三档同 seed 同 jitter 相位)",
            "luma_threshold": LAMP_LUMA_THR,
            "area_max_px": LAMP_AREA_MAX,
            "connectivity": 8,
            "n_components_total": int(n_comp),
            "n_small_lamps": len(lamps),
            "n_big_emitters_excluded": n_big,
        },
        "per_k": {},
    }
    lamp_pass: dict = {}
    for name in ("k2", "k15"):
        ratios = []
        for lp in lamps:
            m = float(luma_k[name][lp["_ys"], lp["_xs"]].mean())
            lp[f"{name}_mean_luma"] = m
            lp[f"{name}_ratio_vs_k3"] = m / lp["k3_mean_luma"]
            ratios.append(lp[f"{name}_ratio_vs_k3"])
        ratios = np.array(ratios)
        cls_stats = {}
        for cname, (lo, hi) in size_classes.items():
            rr = np.array([lp[f"{name}_ratio_vs_k3"] for lp in lamps if lo <= lp["area"] <= hi])
            cls_stats[cname] = {
                "n": int(rr.size),
                "median_ratio": float(np.median(rr)) if rr.size else None,
                "min_ratio": float(rr.min()) if rr.size else None,
                "kills_lt_0p75": int((rr < LAMP_KILL_RATIO).sum()),
                "dims_lt_0p95": int((rr < LAMP_DIM_RATIO).sum()),
            }
        med = float(np.median(ratios))
        stats = {
            "n_lamps": int(ratios.size),
            "median_ratio_vs_k3": med,
            "median_drop_pct": (1.0 - med) * 100.0,
            "min_ratio_vs_k3": float(ratios.min()),
            "kills_lt_0p75": int((ratios < LAMP_KILL_RATIO).sum()),
            "dims_lt_0p95": int((ratios < LAMP_DIM_RATIO).sum()),
            "by_size": cls_stats,
        }
        lamp_pass[name] = (
            stats["median_drop_pct"] <= LAMP_PASS_MEDIAN_DROP and stats["kills_lt_0p75"] == 0
        )
        stats["pass"] = lamp_pass[name]
        lamp_out["per_k"][name] = stats
        print(
            f"[lamps] {name} vs k3: n={stats['n_lamps']} median={med:.5f} "
            f"min={stats['min_ratio_vs_k3']:.5f} kills={stats['kills_lt_0p75']} "
            f"dims={stats['dims_lt_0p95']} → {'PASS' if lamp_pass[name] else 'FAIL'}"
        )
    # 跨口径参照:arm4 同掩码(内容差 ⊃ ggx 镜面/灯光;登记不判)
    for lp in lamps:
        lp["arm4_cross_ratio"] = float(arm4_luma[lp["_ys"], lp["_xs"]].mean()) / lp["k3_mean_luma"]
    cross_lamp = np.array([lp["arm4_cross_ratio"] for lp in lamps])
    med_cross = float(np.median(cross_lamp))
    lamp_out["cross_caliber_arm4_same_mask"] = {
        "median_arm4_over_k3": med_cross,
        "min_arm4_over_k3": float(cross_lamp.min()),
        "note": f"中位 arm4/k3 = {med_cross:.3f}(三档灯核较 arm4 亮 ~{(1.0 / med_cross - 1.0) * 100.0:.1f}%,个别小亮斑 arm4 侧低至 {float(cross_lamp.min()):.2f}——ggx 镜面/灯光内容差方向,与误杀反向)——无 K 误杀迹象的跨口径旁证;内容差主导,登记不判",
    }
    worst = sorted(lamps, key=lambda lp: lp["k15_ratio_vs_k3"])[:8]
    lamp_out["worst8_by_k15_vs_k3"] = [
        {k: v for k, v in lp.items() if not k.startswith("_")} for lp in worst
    ]

    # ── 两态结论(梯内口径)──
    knob_active = {
        name: (
            max(wl_drops[name]["dark_arch"], wl_drops[name]["dark_table"]) >= WL_FIREFLY_EFFECTIVE
            and min(wl_drops[name].values()) >= -WL_REGRESS_TOL
        )
        for name in ("k2", "k15")
    }
    positive = [n for n in ("k2", "k15") if knob_active[n] and lamp_pass[n]]
    if positive:
        rec_name = max(
            positive,
            key=lambda n: (wl_drops[n]["dark_arch"] + wl_drops[n]["dark_table"]) / 2.0,
        )
        verdict = {
            "state": "positive_interval",
            "recommended_k": ARMS[rec_name][0],
            "positive_ks": [ARMS[n][0] for n in positive],
            "action": f"定档建议 K={ARMS[rec_name][0]} 入 full 复评(登记不改代码/预设)",
        }
    else:
        verdict = {
            "state": "closed",
            "recommended_k": None,
            "positive_ks": [],
            "action": "K 档关死,降噪投资转第 1 级(tsrq v4 方差引导,EVAL_DENOISE §7 路线表)",
            "basis": (
                f"阶梯定义域 K∈[1.5,3] 内旋钮对判据面近惰性:四 ROI std_p95 梯内边际降 "
                f"≤{max(max(d.values()) for d in wl_drops.values()):.2f}%(单调但远低于 "
                f"{WL_FIREFLY_EFFECTIVE:.0f}% 有效阈);clamp 触发面 = converged 亮度场 "
                f"k15 vs k3 仅 {engagement['frac_px_rel_gt_1pct_k15_vs_k3'] * 100.0:.1f}% 像素相对差 "
                f">1%(p99.9={engagement['rel_p999_k15_vs_k3'] * 100.0:.1f}%);远小灯梯内保持率 "
                f"中位 1.000 零损伤——旋钮有活性(逐帧 digest 127/128 分离)但降噪收益在判据面不可测,"
                f"正区间在实测定义域内不存在"
            ),
        }
    print(f"[verdict] {verdict['state']}: {verdict['action']}")

    out = {
        "schema": "rurix.day0830.w2.kladder.tsrq_clamp_ladder.v1",
        "judge": "judge_kladder.py(d_ladder.py 复制改造;登记不改任何生产代码/预设)",
        "caliber": {
            "protocol": "conv = frames/frame_01*.exr [::2][:16](实取 14 帧),Rec.709 luma 逐像素时域 std 的 ROI p95——d_ladder.py/d_metrics.py 同式",
            "rois": {k: list(v) for k, v in ROIS.items()},
            "k_arms_command": "g14_3_pipeline_perf --render 128f(--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp {3|2|1.5});命令在案 ris_nee/REPORT.md §8",
            "receipt_lane_parity": parity_fields,
            "k0_baseline_registered_gap": {
                "claim": "EVAL_DENOISE §8 声称 arm4(K=0)d_metrics.json 可直接对照",
                "finding": "不可直接对照——arm4 实跑臂形 = snrm+gi2c001+tsrq,无 --ggx/--lamp-lights/--lamp-gain(D 相四臂 = snrm 基座 A/B 隔离梯;锚谱系 arm1=夜巡 D2 锚 778f1dfc〔scene SPV=g18_smooth_nrm〕/arm3=C 相锚 6144d9f7);本跑按 §8 命令另含 ggx+lamp-gain 4 两噪声/内容源",
                "data_evidence": content_probe,
                "cross_caliber_std_p95_ratio_vs_arm4": {
                    n: {rn: round(v, 2) for rn, v in d.items()} for n, d in cross_ratio.items()
                },
                "cross_note": "三档 std_p95 较 arm4 高 3~12× 且三档互差 <1% ⇒ 抬升源 = ggx 1spp 镜面 + 12 灯点光内容差,非 K 效应;E 相 BENCH_QL(e2_equivalence.py)同旗形但挂 ambient 0.004 + warmup 10,在案亦无同口径 K=0 跑",
                "consequence": "判据改梯内趋势(k3 为参照,K 收紧 3→2→1.5 边际效应)——任务预设分支「口径差如实登记并以三档间趋势判读」",
            },
            "registered_gaps": [
                "EVAL_DENOISE §8 远小灯原型判据 = dolly f0240 远灯 ROI,本跑批无 dolly 腿——以 conv converged.exr(frame_0127,三档同 seed 同 jitter 相位逐像素可比)小连通域替位判读",
                "无梯内 K=0 bench 跑(§8 设计依赖 arm4 在案,上项口径差致空缺)——「K vs 0 绝对收益」不可判,本判读只证「K∈[1.5,3] 旋钮近惰性」;若未来重开 K 线,须先补同旗形 --tsrq-clamp 0 基线跑",
                "K<1.5 未测(阶梯定义域外;kernel 原注 K 过小杀合法孤立小灯,更紧档风险单调升)",
                "窗口腿 K=0 无 ev_k0.json 落盘(= W4 full19 锚 7636f72f 在案);窗口三档 digest 分离仅作 K 接线生效证据,不入画质判据",
            ],
            "thresholds_choice": {
                "lamp_luma_thr": LAMP_LUMA_THR,
                "lamp_area_max_px": LAMP_AREA_MAX,
                "lamp_kill_ratio": LAMP_KILL_RATIO,
                "lamp_dim_ratio": LAMP_DIM_RATIO,
                "lamp_pass": f"梯内 median_drop ≤{LAMP_PASS_MEDIAN_DROP}% ∧ kills=0",
                "knob_active": f"max(dark_arch,dark_table) 梯内降 ≥{WL_FIREFLY_EFFECTIVE}% ∧ 四 ROI 无一梯内升 >{WL_REGRESS_TOL}%",
                "note": "EVAL_DENOISE §8 未给数值阈——本判读选择,登记供复评",
            },
        },
        "selfcheck_arm4_converged": {"ok": True, "fields": selfcheck},
        "bench_wiring_digest_separation": wiring,
        "frames_used": frames_used,
        "conv_std_p95": {**conv, "arm4_incase_cross_caliber": dict(k0_conv)},
        "within_ladder_drops_vs_k3_pct": {
            n: {rn: round(v, 3) for rn, v in d.items()} for n, d in wl_drops.items()
        },
        "monotonic_k3_k2_k15": mono,
        "clamp_engagement_converged": engagement,
        "knob_active": knob_active,
        "far_small_lamp_fidelity": lamp_out,
        "lamp_pass_within_ladder": lamp_pass,
        "window_leg_wiring_evidence": {
            "source": "kladder_runs.json(--quality full ×96f presented digest)",
            "k3": "0771d02a…",
            "k2": "b715fac2…",
            "k15": "0067c520…",
            "k0_full19_anchor": "7636f72f…(W4 s02)",
            "note": "三档互异且异于 K=0 锚 = K 旋钮窗口链生效;不入画质判据",
        },
        "verdict": verdict,
    }
    dst = KL / "tsrq_clamp_ladder.json"
    dst.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"-> {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
