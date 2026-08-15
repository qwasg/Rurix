#!/usr/bin/env python3
"""G10.2 环境探测：GitDependencies --dry-run 过期文件清单 × Commit.gitdeps.xml Blob Size 联表统计。
只读分析，不启动下载。输入：K:\\g10_gitdeps_dryrun.txt（GitDependencies.exe --dry-run --force 输出）。
输出：Windows 子集/非 Windows 子集规模估计（路径启发式分类，登记为估计值非精确裁决）。
Assisted-by: Kimi-K3（G10.2 波）"""
import re, collections

XML = r"K:\moon_night_engine(update at May 1st)\references\UnrealEngine\Engine\Build\Commit.gitdeps.xml"
DRY = r"K:\g10_gitdeps_dryrun.txt"

blob_re = re.compile(r'<Blob Hash="([0-9a-f]+)" Size="(\d+)"')
hash2size = {}
with open(XML, "r", encoding="utf-8", errors="replace") as f:
    for line in f:
        m = blob_re.search(line)
        if m:
            hash2size[m.group(1)] = int(m.group(2))
print(f"blobs parsed: {len(hash2size)}  total bytes: {sum(hash2size.values())}")

# Pack 维度复核（spike「177.9 GB」数字来源核查）
pack_re = re.compile(r'<Pack Hash="[0-9a-f]+" Size="(\d+)" CompressedSize="(\d+)"')
pack_size = pack_csize = npack = 0
with open(XML, "r", encoding="utf-8", errors="replace") as f:
    for line in f:
        m = pack_re.search(line)
        if m:
            npack += 1
            pack_size += int(m.group(1))
            pack_csize += int(m.group(2))
print(f"packs parsed: {npack}  size bytes: {pack_size}  compressed bytes: {pack_csize}")
print(f"blob_sum+pack_size GB(dec): {(sum(hash2size.values())+pack_size)/1e9:.2f}")

file_re = re.compile(r'<File Name="([^"]+)" Hash="([0-9a-f]+)"')
name2hash = {}
with open(XML, "r", encoding="utf-8", errors="replace") as f:
    for line in f:
        m = file_re.search(line)
        if m:
            name2hash[m.group(1)] = m.group(2)
print(f"files parsed: {len(name2hash)}")

with open(DRY, "r", encoding="utf-8", errors="replace") as f:
    outdated = [l.strip() for l in f if l.strip()]
print(f"dry-run lines: {len(outdated)}")
print("--- sample dry-run lines ---")
for l in outdated[:8]:
    print(repr(l))

paths = []
for l in outdated:
    l = l.lstrip("﻿")
    m = re.match(r'(?:Add|Update|Remove)\s+(.+)$', l)
    if m:
        paths.append(m.group(1).strip())

NONWIN = re.compile(r'(?i)(/Mac/|/MacOSX|/Linux/|/LinuxArm64/|/Android/|/IOS/|/iPhoneOS|/TVOS/|/VisionOS/|osx-|osx/|/osx|linux-|/linux|/Unix/|HoloLens|/WinGDK/|/Xbox|/PS5|/PS4|/Switch)')
matched = unmatched = 0
win_bytes = nonwin_bytes = 0
win_files = nonwin_files = 0
unmatched_list = []
top_nonwin = collections.Counter()
for p in paths:
    key = p.replace("\\", "/")
    h = name2hash.get(key)
    if h is None:
        unmatched += 1
        if len(unmatched_list) < 10:
            unmatched_list.append(p)
        continue
    matched += 1
    sz = hash2size.get(h, 0)
    if NONWIN.search(key):
        nonwin_files += 1
        nonwin_bytes += sz
        seg = key.split("/")
        top_nonwin["/".join(seg[:4])] += sz
    else:
        win_files += 1
        win_bytes += sz

print(f"matched: {matched}  unmatched: {unmatched}")
if unmatched_list:
    print("unmatched samples:", unmatched_list)
GB = 1024**3
print(f"WINDOWS-NEEDED (heuristic): files={win_files} bytes={win_bytes} ({win_bytes/GB:.2f} GiB)")
print(f"NON-WINDOWS (heuristic):    files={nonwin_files} bytes={nonwin_bytes} ({nonwin_bytes/GB:.2f} GiB)")
print("--- top non-win prefixes by bytes ---")
for k, v in top_nonwin.most_common(15):
    print(f"  {v/GB:8.2f} GiB  {k}")
