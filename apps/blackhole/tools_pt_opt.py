#!/usr/bin/env python3
"""50-round auto optimizer for blackhole path-traced render vs ref_gargantua."""
from __future__ import annotations
import os, re, shutil, subprocess, random, json
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
LOG = ITERS / "pt_opt_log.jsonl"

# Tunable keys and (lo, hi) ranges
KEYS = {
    "PT_H0": (0.03, 0.09),
    "PT_SIGMA_A": (1.2, 4.5),
    "PT_SIGMA_S": (0.6, 3.0),
    "PT_EMIT": (0.8, 3.2),
    "PT_INSCAT": (0.4, 1.8),
    "DISK_EXPOSURE": (0.4, 1.6),
    "DISK_ALPHA": (0.25, 0.85),
    "DISK_T_IN": (4800.0, 7200.0),
    "GLOW_J0": (0.05, 0.4),
    "TURB_AMP": (0.7, 2.0),
    "TURB_FREQ": (0.7, 1.8),
    "TURB_FIL_W": (0.3, 0.8),
    "BLOOM_STRENGTH": (0.5, 1.6),
    "BLOOM_RADIUS": (8.0, 24.0),
    "BLOOM_THRESH": (1.5, 3.5),
    "CAM_ROLL": (-0.28, -0.08),
    "VIEW_SCALE": (0.42, 0.50),
}

INT_KEYS = {"BLOOM_RADIUS"}


def read_params() -> str:
    return PARAMS.read_text(encoding="utf-8")


def set_const(text: str, name: str, value: float) -> str:
    if name in INT_KEYS or name in ("OFFLINE_FRAMES", "OFFLINE_SSAA", "MAX_STEPS"):
        v = str(int(round(value)))
        for t in ("usize", "u32", "i32"):
            pat = rf"(pub const {name}: {t} = )([^;]+)(;)"
            if re.search(pat, text):
                return re.sub(pat, rf"\g<1>{v}\g<3>", text, count=1)
    v = f"{value:.6g}"
    if "." not in v and "e" not in v.lower():
        v += ".0"
    for t in ("f32", "i32", "u32", "usize"):
        pat = rf"(pub const {name}: {t} = )([^;]+)(;)"
        if re.search(pat, text):
            if t in ("i32", "u32", "usize"):
                v = str(int(round(value)))
            return re.sub(pat, rf"\g<1>{v}\g<3>", text, count=1)
    return text


def get_const(text: str, name: str) -> float:
    m = re.search(rf"pub const {name}: \w+ = ([^;]+);", text)
    if not m:
        raise KeyError(name)
    return float(m.group(1).replace("f32", "").strip())


def write_params(text: str) -> None:
    PARAMS.write_text(text, encoding="utf-8")


def eval_mae(tag: str) -> float:
    cur = Image.open(PPM).convert("RGB")
    cur.save(FINAL)
    cur.save(ITERS / f"{tag}_full.png")
    w, h = cur.size
    ref = Image.open(REF).convert("RGB").resize((w, h), Image.Resampling.LANCZOS)
    cw, ch = 1000, 700
    cx, cy = w // 2, h // 2
    cc = cur.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    rr = ref.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    side = Image.new("RGB", (cw * 2 + 20, ch), (12, 12, 16))
    side.paste(cc, (0, 0))
    side.paste(rr, (cw + 20, 0))
    side.save(ITERS / f"{tag}_center.png")
    a = np.array(cc).astype(np.float64)
    b = np.array(rr).astype(np.float64)
    mae = float(np.abs(a - b).mean())
    lum = a.mean(2)
    strip = lum[ch // 2 - 50 : ch // 2 + 50, 80:380]
    ghost = float(
        np.abs(np.diff(strip, axis=1)).mean()
        / (np.abs(np.diff(strip, axis=0)).mean() + 1e-6)
    )
    (ITERS / f"{tag}_metrics.txt").write_text(f"mae={mae}\nghost={ghost}\n", encoding="utf-8")
    return mae


def render(tag: str) -> float:
    # kill stale exe lock
    subprocess.run(
        ["powershell", "-NoProfile", "-Command", "Get-Process offline -ErrorAction SilentlyContinue | Stop-Process -Force"],
        cwd=str(ROOT),
        capture_output=True,
    )
    r = subprocess.run(
        [str(RX), "run", "apps/blackhole/src/offline.rx"],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        timeout=600,
    )
    if r.returncode != 0:
        print(f"{tag} FAIL compile/run:\n{r.stderr[-2000:]}\n{r.stdout[-1000:]}")
        return 1e9
    return eval_mae(tag)


def perturb(base: dict[str, float], scale: float, rng: random.Random) -> dict[str, float]:
    out = dict(base)
    # change 3–6 keys each round
    keys = list(KEYS.keys())
    rng.shuffle(keys)
    n = rng.randint(3, 6)
    for k in keys[:n]:
        lo, hi = KEYS[k]
        span = hi - lo
        delta = rng.uniform(-scale, scale) * span
        out[k] = min(hi, max(lo, out[k] + delta))
    return out


def apply(vals: dict[str, float]) -> None:
    text = read_params()
    for k, v in vals.items():
        text = set_const(text, k, v)
    # keep opt fast
    text = set_const(text, "OFFLINE_FRAMES", 1.0)
    text = set_const(text, "OFFLINE_SSAA", 2.0)
    write_params(text)


def snapshot_vals() -> dict[str, float]:
    text = read_params()
    return {k: get_const(text, k) for k in KEYS}


def main() -> None:
    ITERS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(20260721)
    best_vals = snapshot_vals()
    apply(best_vals)
    best_mae = render("pt00")
    print(f"baseline pt00 mae={best_mae:.3f}")
    with LOG.open("w", encoding="utf-8") as log:
        log.write(json.dumps({"tag": "pt00", "mae": best_mae, "vals": best_vals}) + "\n")
        log.flush()
        scale = 0.35
        stagnant = 0
        for i in range(1, 51):
            tag = f"pt{i:02d}"
            cand = perturb(best_vals, scale, rng)
            apply(cand)
            mae = render(tag)
            rec = {"tag": tag, "mae": mae, "scale": scale, "vals": cand}
            log.write(json.dumps(rec) + "\n")
            log.flush()
            print(f"{tag} mae={mae:.3f} best={best_mae:.3f} scale={scale:.3f}")
            if mae + 0.05 < best_mae:
                best_mae = mae
                best_vals = cand
                shutil.copy(ITERS / f"{tag}_full.png", FINAL)
                shutil.copy(ITERS / f"{tag}_center.png", ITERS / "latest_vs_ref.png")
                stagnant = 0
                scale = max(0.12, scale * 0.92)
            else:
                stagnant += 1
                if stagnant >= 4:
                    scale = min(0.55, scale * 1.25)
                    stagnant = 0
        # restore best + final SSAA4 polish
        apply(best_vals)
        text = read_params()
        text = set_const(text, "OFFLINE_SSAA", 4.0)
        write_params(text)
        mae = render("pt_final")
        print(f"FINAL mae={mae:.3f} (best during search {best_mae:.3f})")
        shutil.copy(ITERS / "pt_final_full.png", FINAL)
        shutil.copy(ITERS / "pt_final_center.png", ITERS / "latest_vs_ref.png")
        (ITERS / "pt_best.json").write_text(
            json.dumps({"mae": mae, "search_best": best_mae, "vals": best_vals}, indent=2),
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
