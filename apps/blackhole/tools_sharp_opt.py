#!/usr/bin/env python3
"""Sharpness-directed search: maximize edge/HF score, keep MAE bounded."""
from __future__ import annotations
import json, random, re, shutil, subprocess
from pathlib import Path
from PIL import Image
import numpy as np

ROOT = Path(r"H:/rurix")
PARAMS = ROOT / "apps/blackhole/src/params.rx"
RX = ROOT / "target/release/rx.exe"
ITERS = ROOT / "apps/blackhole/iters"
REF = ROOT / "apps/blackhole/ref_gargantua.png"
PPM = ROOT / "apps/blackhole/frames/f_0000.ppm"
FINAL = ROOT / "apps/blackhole/frame_final.png"
LOG = ITERS / "sharp_opt_log.jsonl"

KEYS = {
    "PT_H0": (0.018, 0.05),
    "PT_SIGMA_A": (0.8, 2.8),
    "PT_SIGMA_S": (0.2, 1.2),
    "PT_EMIT": (0.6, 2.0),
    "PT_INSCAT": (0.15, 0.9),
    "PT_JITTER": (0.05, 0.45),
    "DISK_EXPOSURE": (1.0, 2.4),
    "DISK_ALPHA": (0.65, 0.97),
    "DISK_T_IN": (5200.0, 7000.0),
    "TURB_AMP": (1.2, 2.2),
    "TURB_FREQ": (1.1, 2.0),
    "TURB_FIL_W": (0.55, 0.88),
    "BLOOM_STRENGTH": (0.25, 0.75),
    "BLOOM_RADIUS": (5.0, 12.0),
    "BLOOM_THRESH": (2.5, 4.5),
    "CAM_ROLL": (-0.24, -0.12),
    "VIEW_SCALE": (0.44, 0.49),
}


def set_const(text: str, name: str, value: float) -> str:
    vi = str(int(round(value)))
    vf = f"{value:.6g}"
    if "." not in vf and "e" not in vf.lower():
        vf += ".0"
    for t in ("usize", "u32", "i32", "f32"):
        pat = rf"(pub const {name}: {t} = )([^;]+)(;)"
        if re.search(pat, text):
            v = vi if t != "f32" else vf
            return re.sub(pat, rf"\g<1>{v}\g<3>", text, count=1)
    return text


def apply(vals: dict) -> None:
    text = PARAMS.read_text(encoding="utf-8")
    for k, v in vals.items():
        text = set_const(text, k, float(v))
    text = set_const(text, "OFFLINE_FRAMES", 1)
    text = set_const(text, "OFFLINE_SSAA", 2)
    PARAMS.write_text(text, encoding="utf-8")


def snapshot() -> dict:
    text = PARAMS.read_text(encoding="utf-8")
    out = {}
    for k in KEYS:
        m = re.search(rf"pub const {k}: \w+ = ([^;]+);", text)
        out[k] = float(m.group(1))
    return out


def score_pair(cur: Image.Image, ref: Image.Image) -> dict:
    w, h = cur.size
    ref = ref.resize((w, h), Image.Resampling.LANCZOS)
    cw, ch = 1000, 700
    cx, cy = w // 2, h // 2
    box = (cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2)
    a = np.array(cur.crop(box)).astype(np.float64)
    b = np.array(ref.crop(box)).astype(np.float64)
    mae = float(np.abs(a - b).mean())
    la = a.mean(2)
    lb = b.mean(2)
    # high-freq energy (disk band)
    disk = la[ch // 2 - 40 : ch // 2 + 90, 60:940]
    gx = np.abs(np.diff(disk, axis=1)).mean()
    gy = np.abs(np.diff(disk, axis=0)).mean()
    hf = float(gx + gy)
    # shadow edge: max radial-ish gradient in upper-center dark region
    sh = la[40:260, 280:720]
    edge = float(np.abs(np.diff(sh, axis=0)).max() + np.abs(np.diff(sh, axis=1)).max())
    # contrast
    p5, p95 = np.percentile(la, [5, 95])
    contrast = float(p95 - p5)
    # reference HF for calibration
    disk_b = lb[ch // 2 - 40 : ch // 2 + 90, 60:940]
    hf_ref = float(np.abs(np.diff(disk_b, axis=1)).mean() + np.abs(np.diff(disk_b, axis=0)).mean())
    # composite: lower better; penalize softness & mae
    # want hf near ref, high edge, high contrast, low mae
    soft_pen = max(0.0, (hf_ref - hf) / (hf_ref + 1e-6))
    score = 0.35 * (mae / 80.0) + 0.40 * soft_pen + 0.15 * max(0.0, 1.0 - edge / 80.0) + 0.10 * max(0.0, 1.0 - contrast / 180.0)
    return {
        "score": score,
        "mae": mae,
        "hf": hf,
        "hf_ref": hf_ref,
        "edge": edge,
        "contrast": contrast,
    }


def eval_tag(tag: str) -> dict:
    cur = Image.open(PPM).convert("RGB")
    cur.save(FINAL)
    cur.save(ITERS / f"{tag}_full.png")
    ref = Image.open(REF).convert("RGB")
    m = score_pair(cur, ref)
    w, h = cur.size
    ref2 = ref.resize((w, h), Image.Resampling.LANCZOS)
    cw, ch = 1000, 700
    cx, cy = w // 2, h // 2
    cc = cur.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    rr = ref2.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    side = Image.new("RGB", (cw * 2 + 20, ch), (12, 12, 16))
    side.paste(cc, (0, 0))
    side.paste(rr, (cw + 20, 0))
    side.save(ITERS / f"{tag}_center.png")
    (ITERS / f"{tag}_metrics.txt").write_text(json.dumps(m, indent=2), encoding="utf-8")
    return m


def kill():
    subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "Get-Process offline -ErrorAction SilentlyContinue | Stop-Process -Force"],
        cwd=str(ROOT), capture_output=True,
    )


def render(tag: str) -> dict:
    kill()
    r = subprocess.run(
        [str(RX), "run", "apps/blackhole/src/offline.rx"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=600,
    )
    if r.returncode != 0:
        (ITERS / f"{tag}_fail.txt").write_text((r.stderr or "")[-2000:], encoding="utf-8")
        print(f"{tag} FAIL")
        return {"score": 1e9, "mae": 1e9, "hf": 0, "edge": 0, "contrast": 0}
    return eval_tag(tag)


def perturb(base: dict, scale: float, rng: random.Random) -> dict:
    out = dict(base)
    keys = list(KEYS)
    rng.shuffle(keys)
    for k in keys[: rng.randint(3, 7)]:
        lo, hi = KEYS[k]
        out[k] = min(hi, max(lo, out[k] + rng.uniform(-scale, scale) * (hi - lo)))
    return out


def main():
    ITERS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(777021)
    best_vals = snapshot()
    apply(best_vals)
    m0 = render("sh00")
    best_score = m0["score"]
    best_vals_keep = dict(best_vals)
    print(f"sh00 score={best_score:.4f} mae={m0['mae']:.2f} hf={m0['hf']:.2f} edge={m0['edge']:.1f}", flush=True)
    with LOG.open("w", encoding="utf-8") as log:
        log.write(json.dumps({"tag": "sh00", **m0, "vals": best_vals}) + "\n")
        scale = 0.4
        stagnant = 0
        for i in range(1, 31):
            tag = f"sh{i:02d}"
            cand = perturb(best_vals_keep, scale, rng)
            apply(cand)
            m = render(tag)
            rec = {"tag": tag, **m, "scale": scale, "vals": cand}
            log.write(json.dumps(rec) + "\n")
            log.flush()
            print(
                f"{tag} score={m['score']:.4f} mae={m['mae']:.2f} hf={m['hf']:.2f} edge={m['edge']:.1f} best={best_score:.4f}",
                flush=True,
            )
            if m["score"] < best_score - 0.002:
                best_score = m["score"]
                best_vals_keep = cand
                shutil.copy(ITERS / f"{tag}_full.png", FINAL)
                shutil.copy(ITERS / f"{tag}_center.png", ITERS / "latest_vs_ref.png")
                stagnant = 0
                scale = max(0.12, scale * 0.9)
            else:
                stagnant += 1
                apply(best_vals_keep)
                if stagnant >= 4:
                    scale = min(0.55, scale * 1.2)
                    stagnant = 0
        apply(best_vals_keep)
        text = PARAMS.read_text(encoding="utf-8")
        text = set_const(text, "OFFLINE_SSAA", 4)
        PARAMS.write_text(text, encoding="utf-8")
        m = render("sh_final")
        print(f"FINAL score={m['score']:.4f} mae={m['mae']:.2f} hf={m['hf']:.2f}", flush=True)
        if m["score"] < 1e8:
            shutil.copy(ITERS / "sh_final_full.png", FINAL)
            shutil.copy(ITERS / "sh_final_center.png", ITERS / "latest_vs_ref.png")
        (ITERS / "sharp_best.json").write_text(
            json.dumps({"score": best_score, "final": m, "vals": best_vals_keep}, indent=2),
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
