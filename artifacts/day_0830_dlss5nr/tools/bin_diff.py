# 等长二进制差异量化(DLSS5 NR 双变体补丁范围取证)
# 两个同尺寸文件逐字节比较,统计:差异字节总数/占比、差异块分布(合并成区段)、
# 首末差异偏移。用途:证明「40系v1」是就地等长 cubin 补丁而非重打包。
# 用法:python bin_diff.py <a> <b> [--block 65536] [--json 输出]
import json
import mmap
import sys
from pathlib import Path


def main():
    a, b = Path(sys.argv[1]), Path(sys.argv[2])
    block = 65536
    json_out = None
    args = sys.argv[3:]
    i = 0
    while i < len(args):
        if args[i] == "--block":
            block = int(args[i + 1]); i += 2
        elif args[i] == "--json":
            json_out = Path(args[i + 1]); i += 2
        else:
            i += 1

    sa, sb = a.stat().st_size, b.stat().st_size
    report = {"a": str(a), "b": str(b), "size_a": sa, "size_b": sb}
    if sa != sb:
        report["equal_length"] = False
        report["note"] = "长度不同,非就地补丁"
        _emit(report, json_out)
        return
    report["equal_length"] = True

    with open(a, "rb") as fa, open(b, "rb") as fb:
        ma = mmap.mmap(fa.fileno(), 0, access=mmap.ACCESS_READ)
        mb = mmap.mmap(fb.fileno(), 0, access=mmap.ACCESS_READ)
        diff_bytes = 0
        first_off = -1
        last_off = -1
        dirty_blocks = []  # 有差异的块索引
        n = sa
        pos = 0
        bi = 0
        while pos < n:
            ca = ma[pos : pos + block]
            cb = mb[pos : pos + block]
            if ca != cb:
                dirty_blocks.append(bi)
                for j in range(len(ca)):
                    if ca[j] != cb[j]:
                        diff_bytes += 1
                        off = pos + j
                        if first_off < 0:
                            first_off = off
                        last_off = off
            pos += block
            bi += 1
        ma.close(); mb.close()

    # 相邻脏块合并成连续区段(块粒度)
    ranges = []
    for blk in dirty_blocks:
        if ranges and blk == ranges[-1][1] + 1:
            ranges[-1][1] = blk
        else:
            ranges.append([blk, blk])
    seg = [
        {"byte_start": r[0] * block, "byte_end_excl": min((r[1] + 1) * block, sa)}
        for r in ranges
    ]

    report.update(
        {
            "block": block,
            "diff_bytes": diff_bytes,
            "diff_ratio": round(diff_bytes / sa, 8),
            "dirty_block_count": len(dirty_blocks),
            "total_blocks": bi,
            "first_diff_off": first_off,
            "last_diff_off": last_off,
            "diff_segments": seg,
        }
    )
    _emit(report, json_out)


def _emit(report, json_out):
    text = json.dumps(report, ensure_ascii=False, indent=1)
    if json_out:
        json_out.parent.mkdir(parents=True, exist_ok=True)
        json_out.write_text(text, encoding="utf-8")
        r = report
        if r.get("equal_length"):
            print(
                f"[bin_diff] 等长={r['size_a']}B 差异={r['diff_bytes']}B "
                f"({r['diff_ratio']*100:.4f}%) 脏块={r['dirty_block_count']}/{r['total_blocks']} "
                f"区段={len(r['diff_segments'])} 首={r['first_diff_off']} 末={r['last_diff_off']}"
            )
        else:
            print(f"[bin_diff] {r.get('note')}")
    else:
        print(text)


if __name__ == "__main__":
    main()
