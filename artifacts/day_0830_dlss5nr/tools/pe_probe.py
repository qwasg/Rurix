# PE 导出表 + 目标字符串探测(DLSS5 NR 适配 Phase 0 静态探测面)
# 纯标准库:mmap 全映射,手写 PE32+ 头解析(导出目录 + 版本号),再对全文件做
# 目标 token 的 ASCII 字符串上下文扫描(定位 GetProcAddress 目标/参数键/架构 cubin)。
# 用法:
#   python pe_probe.py <pe文件> [--tokens t1,t2,...] [--max-hits N] [--json 输出路径]
import json
import mmap
import re
import struct
import sys
from pathlib import Path

# 缺省 token 集:NGX 入口面 / NR 参数键 / SM 架构(cubin 目标) / 图形 API 面
DEFAULT_TOKENS = [
    "NVSDK_NGX_D3D12_",
    "NVSDK_NGX_D3D11_",
    "NVSDK_NGX_VULKAN_",
    "NVSDK_NGX_CUDA_",
    "nvngx_dlssnr",
    "_nvngx.dll",
    "nvngx.dll",
    "DLSSNR.",
    "NeuralRendering",
    "Snippet",
    "PerfQualityValue",
    "sm_89",
    "sm_90",
    "sm_100",
    "sm_120",
    "NGXCore",
]


def parse_exports(mm: mmap.mmap):
    """解析 PE32+ 导出表,返回 (导出名列表, 节表, 诊断)。失败返回空表+原因。"""
    diag = []
    if mm[:2] != b"MZ":
        return [], [], ["非 MZ 文件"]
    (e_lfanew,) = struct.unpack_from("<I", mm, 0x3C)
    if mm[e_lfanew : e_lfanew + 4] != b"PE\x00\x00":
        return [], [], ["无 PE 签名"]
    coff = e_lfanew + 4
    (machine, nsec, _, _, _, opt_size, _) = struct.unpack_from("<HHIIIHH", mm, coff)
    opt = coff + 20
    (magic,) = struct.unpack_from("<H", mm, opt)
    if magic != 0x20B:
        return [], [], [f"非 PE32+(magic=0x{magic:x})"]
    # PE32+ 数据目录起点 = 可选头 +112;目录[0] = 导出表
    (exp_rva, exp_size) = struct.unpack_from("<II", mm, opt + 112)
    sec0 = opt + opt_size
    sections = []
    for i in range(nsec):
        off = sec0 + i * 40
        name = mm[off : off + 8].rstrip(b"\x00").decode("ascii", "replace")
        (vsize, vaddr, rsize, raw) = struct.unpack_from("<IIII", mm, off + 8)
        sections.append((name, vaddr, max(vsize, rsize), raw))

    def rva2off(rva: int):
        for _, vaddr, size, raw in sections:
            if vaddr <= rva < vaddr + size:
                return raw + (rva - vaddr)
        return None

    if exp_rva == 0:
        return [], sections, ["无导出目录"]
    ed = rva2off(exp_rva)
    if ed is None:
        return [], sections, ["导出目录 RVA 越界"]
    (_, _, _, _, name_rva, _, nfunc, nname, _, names_rva, _) = struct.unpack_from(
        "<IIHHIIIIIII", mm, ed
    )
    dllname_off = rva2off(name_rva)
    dllname = ""
    if dllname_off is not None:
        end = mm.find(b"\x00", dllname_off)
        dllname = mm[dllname_off:end].decode("ascii", "replace")
    names = []
    tbl = rva2off(names_rva)
    if tbl is not None:
        for i in range(nname):
            (nrva,) = struct.unpack_from("<I", mm, tbl + 4 * i)
            noff = rva2off(nrva)
            if noff is None:
                continue
            end = mm.find(b"\x00", noff)
            names.append(mm[noff:end].decode("ascii", "replace"))
    diag.append(f"machine=0x{machine:x} 节数={nsec} 导出dll名={dllname} 函数数={nfunc} 名字数={nname}")
    return names, sections, diag


def scan_tokens(mm: mmap.mmap, tokens, max_hits):
    """对全文件做 token 命中扫描,每个命中回吐所在 ASCII 字符串(前后扩展)。"""
    out = {}
    for tok in tokens:
        pat = re.escape(tok.encode("ascii"))
        hits = []
        seen = set()
        for m in re.finditer(pat, mm):
            a = m.start()
            # 向两侧扩到可打印 ASCII 边界(截 256 字节防巨串)
            lo = a
            while lo > 0 and a - lo < 128 and 0x20 <= mm[lo - 1] < 0x7F:
                lo -= 1
            hi = m.end()
            n = len(mm)
            while hi < n and hi - a < 192 and 0x20 <= mm[hi] < 0x7F:
                hi += 1
            s = mm[lo:hi].decode("ascii", "replace")
            if s not in seen:
                seen.add(s)
                hits.append(s)
            if len(hits) >= max_hits:
                break
        out[tok] = hits
    return out


def main():
    path = Path(sys.argv[1])
    tokens = list(DEFAULT_TOKENS)
    max_hits = 40
    json_out = None
    args = sys.argv[2:]
    i = 0
    while i < len(args):
        if args[i] == "--tokens":
            tokens = args[i + 1].split(",")
            i += 2
        elif args[i] == "--max-hits":
            max_hits = int(args[i + 1])
            i += 2
        elif args[i] == "--json":
            json_out = Path(args[i + 1])
            i += 2
        else:
            i += 1

    with open(path, "rb") as f:
        mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        exports, sections, diag = parse_exports(mm)
        token_hits = scan_tokens(mm, tokens, max_hits)
        mm.close()

    report = {
        "file": str(path),
        "size": path.stat().st_size,
        "pe_diag": diag,
        "sections": [
            {"name": n, "vaddr": v, "size": s, "raw": r} for (n, v, s, r) in sections
        ],
        "exports": exports,
        "token_hits": token_hits,
    }
    text = json.dumps(report, ensure_ascii=False, indent=1)
    if json_out:
        json_out.parent.mkdir(parents=True, exist_ok=True)
        json_out.write_text(text, encoding="utf-8")
        # stdout 只给摘要,完整面落 JSON
        print(f"[pe_probe] {path.name}: exports={len(exports)} -> {json_out}")
        for d in diag:
            print(f"  {d}")
        for tok, hits in token_hits.items():
            if hits:
                print(f"  token {tok!r}: {len(hits)} 组")
    else:
        print(text)


if __name__ == "__main__":
    main()
