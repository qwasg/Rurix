from PIL import Image
import numpy as np, sys, os

def cmp(tag):
    os.makedirs(r'apps/blackhole/iters', exist_ok=True)
    cur = Image.open(r'apps/blackhole/frames/f_0000.ppm').convert('RGB')
    cur.save(r'apps/blackhole/frame_final.png')
    cur.save(rf'apps/blackhole/iters/{tag}_full.png')
    w, h = cur.size
    ref = Image.open(r'apps/blackhole/ref_gargantua.png').convert('RGB')
    ref = ref.resize((w, h), Image.Resampling.LANCZOS)
    canvas = Image.new('RGB', (w * 2 + 40, h), (12, 12, 16))
    canvas.paste(cur, (0, 0))
    canvas.paste(ref, (w + 40, 0))
    canvas.save(rf'apps/blackhole/iters/{tag}_side.png')
    cw, ch = 1000, 700
    cx, cy = w // 2, h // 2
    cc = cur.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    rr = ref.crop((cx - cw // 2, cy - ch // 2, cx + cw // 2, cy + ch // 2))
    side = Image.new('RGB', (cw * 2 + 20, ch), (12, 12, 16))
    side.paste(cc, (0, 0))
    side.paste(rr, (cw + 20, 0))
    side.save(rf'apps/blackhole/iters/{tag}_center.png')
    a = np.array(cc).astype(float)
    b = np.array(rr).astype(float)
    mae = np.abs(a - b).mean()
    lum = a.mean(2)
    strip = lum[ch // 2 - 50: ch // 2 + 50, 80:380]
    ghost = np.abs(np.diff(strip, axis=1)).mean() / (np.abs(np.diff(strip, axis=0)).mean() + 1e-6)
    sh = lum[80:220, 380:620]
    print(f'{tag} mae={mae:.2f} ghost_vh={ghost:.3f} shadow={sh.mean():.1f}/{sh.std():.1f}')

if __name__ == '__main__':
    cmp(sys.argv[1] if len(sys.argv) > 1 else 'iter')
