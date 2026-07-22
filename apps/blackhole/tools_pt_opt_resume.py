#!/usr/bin/env python3
"""Resume path-tracing optimizer from best so far; finish through pt50 + final."""
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
LOG = ITERS / "pt_opt_log.jsonl"

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
INT_KEYS = {"BLOOM_RADIUS", "OFFLINE_FRAMES", "OFFLINE_SSAA"}


def set_const(text: str, name: str, value: float) -> str:
    v_int = str(int(round(value)))
    v_f = f"{value:.6g}"
    if "." not in v_f and "e" not in v_f.lower():
        v_f += ".0"
    for t in ("usize", "u32", "i32", "f32"):
        pat = rf"(pub const {name}: {t} = )([^;]+)(;)"
        if re.search(pat, text):
            v = v_int if t in ("usize", "u32", "i32") else v_f
            return re.sub(pat, rf"\g<1>{v}\g<3>", text, count=1)
    return text


def apply(vals: dict) -> None:
    text = PARAMS.read_text(encoding="utf-8")
    for k, v in vals.items():
        text = set_const(text, k, float(v))
    text = set_const(text, "OFFLINE_FRAMES", 1)
    text = set_const(text, "OFFLINE_SSAA", 2)
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
    (ITERS / f"{tag}_metrics.txt").write_text(f"mae={mae}\n", encoding="utf-8")
    return mae


def kill_offline():
    subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "Get-Process offline -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 400"],
        cwd=str(ROOT), capture_output=True,
    )


def render(tag: str) -> float:
    kill_offline()
    r = subprocess.run(
        [str(RX), "run", "apps/blackhole/src/offline.rx"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=600,
    )
    if r.returncode != 0:
        err = (r.stderr or "")[-1500:] + "\n" + (r.stdout or "")[-500:]
        (ITERS / f"{tag}_fail.txt").write_text(err, encoding="utf-8")
        print(f"{tag} FAIL rc={r.returncode}")
        return 1e9
    return eval_mae(tag)


def perturb(base: dict, scale: float, rng: random.Random) -> dict:
    out = dict(base)
    keys = list(KEYS)
    rng.shuffle(keys)
    for k in keys[: rng.randint(3, 6)]:
        lo, hi = KEYS[k]
        out[k] = min(hi, max(lo, out[k] + rng.uniform(-scale, scale) * (hi - lo)))
    return out


def load_best():
    best = None
    best_mae = 1e9
    done = set()
    with LOG.open(encoding="utf-8") as f:
        for line in f:
            rec = json.loads(line)
            done.add(rec["tag"])
            if rec["mae"] < best_mae:
                best_mae = rec["mae"]
                best = rec
    return best, best_mae, done


def main():
    ITERS.mkdir(parents=True, exist_ok=True)
    best_rec, best_mae, done = load_best()
    assert best_rec is not None
    best_vals = {k: float(best_rec["vals"][k]) for k in KEYS}
    print(f"resume from {best_rec['tag']} mae={best_mae:.4f}")
    apply(best_vals)
    shutil.copy(ITERS / f"{best_rec['tag']}_full.png", FINAL)
    shutil.copy(ITERS / f"{best_rec['tag']}_center.png", ITERS / "latest_vs_ref.png")

    rng = random.Random(20260721 + 100)
    scale = float(best_rec.get("scale", 0.3))
    stagnant = 0
    with LOG.open("a", encoding="utf-8") as log:
        for i in range(1, 51):
            tag = f"pt{i:02d}"
            if tag in done and tag != "pt20":
                # skip completed successful rounds; retry fails
                if tag != "pt20":
                    continue
            cand = perturb(best_vals, scale, rng)
            apply(cand)
            mae = render(tag)
            rec = {"tag": tag, "mae": mae, "scale": scale, "vals": cand}
            log.write(json.dumps(rec) + "\n")
            log.flush()
            print(f"{tag} mae={mae:.4f} best={best_mae:.4f} scale={scale:.3f}", flush=True)
            if mae < 1e8 and mae + 0.02 < best_mae:
                best_mae = mae
                best_vals = cand
                shutil.copy(ITERS / f"{tag}_full.png", FINAL)
                shutil.copy(ITERS / f"{tag}_center.png", ITERS / "latest_vs_ref.png")
                stagnant = 0
                scale = max(0.10, scale * 0.9)
            else:
                stagnant += 1
                apply(best_vals)  # restore after fail/worse
                if stagnant >= 4:
                    scale = min(0.5, scale * 1.2)
                    stagnant = 0

        apply(best_vals)
        text = PARAMS.read_text(encoding="utf-8")
        text = set_const(text, "OFFLINE_SSAA", 4)
        PARAMS.write_text(text, encoding="utf-8")
        mae = render("pt_final")
        print(f"FINAL mae={mae:.4f} search_best={best_mae:.4f}", flush=True)
        if mae < 1e8:
            shutil.copy(ITERS / "pt_final_full.png", FINAL)
            shutil.copy(ITERS / "pt_final_center.png", ITERS / "latest_vs_ref.png")
        (ITERS / "pt_best.json").write_text(
            json.dumps({"mae": mae, "search_best": best_mae, "vals": best_vals}, indent=2),
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
