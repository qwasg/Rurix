#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 压测语料共享库（spec/external_reference.md RXS-0381/RXS-0382/RXS-0383；
RFC-0027 §4.2/§4.3/§4.4）。

供 ci/g10_asset_license_registry_smoke.py（M131）/ ci/g10_corpus_loading_smoke.py
（M132）/ ci/g10_corpus_list_freeze_smoke.py（M133）三门复用的纯判定层：

  - 缓存根解析序闭集（RURIX_G10_CACHE_ROOT → g10_cache_root.local.json →
    缺省 K:\\rurix_g10_cache；根不可达 fail-closed，禁静默回退其他盘符）；
  - 逐文件 SHA-256 + 清单级 canonical digest（相对路径正斜杠稳定排序清单再
    sha256，沿 RFC-0020 §4.2 同构子集：LF 行、零时间戳、零主机路径）；
  - 许可注册表按类闭集校验（external 五元组 / generated 六字段 + 通用字段 +
    attribution 子字段闭集 + SPDX 受限表达式 + 白名单闭集 + 两类互冒充拦截）；
  - git 零二进制守卫（扩展名闭集 + measured 体积阈值双判 + magic-bytes 嗅探 +
    白名单路径闭集豁免留痕）；
  - 场景清单校验（行闭集 / 按 scene_id 稳定排序 / 清单 digest 注册在树 /
    只追加修订 / ready 下界 ≥2 vacuous 拦截）。

全部函数纯 host、零网络；校验函数返回 failure 字符串列表（空 = 通过），
不直接 sys.exit——门脚本自行汇总进 checks 闭集。
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY_PATH = ROOT / "milestones" / "g10" / "g10_asset_license_registry.json"
MANIFEST_PATH = ROOT / "milestones" / "g10" / "g10_corpus_scene_manifest.json"
LOCAL_CONFIG = ROOT / "g10_cache_root.local.json"
DEFAULT_CACHE_ROOT = Path(r"K:\rurix_g10_cache")

WHITELIST_SPDX = ("CC0-1.0", "CC-BY-3.0", "CC-BY-4.0")
EXTERNAL_REQUIRED = ("asset_id", "spdx_id", "source_url", "attribution", "digest")
GENERATED_REQUIRED = (
    "asset_id",
    "spdx_id",
    "source_url",
    "generator_script",
    "generator_script_digest",
    "generator_params_digest",
    "digest",
)
COMMON_REQUIRED = (
    "license_snapshot",
    "checked_at",
    "upstream_ref",
    "cache_rel",
    "file_count",
    "byte_len",
)
ATTRIBUTION_KEYS = (
    "creator",
    "title",
    "source_uri",
    "license_uri",
    "copyright_notice",
    "modified_flag",
)
# git 零二进制守卫闭集（RXS-0382 L1 全量列出；扩展走只追加修订行）。
GUARD_EXTENSIONS = (
    ".glb",
    ".gltf",
    ".bin",
    ".fbx",
    ".obj",
    ".exr",
    ".hdr",
    ".ktx2",
    ".zip",
    ".png",
    ".tga",
    ".dds",
    ".tif",
    ".tiff",
)
# 白名单路径闭集豁免（既有合法夹具/资产，留痕；RXS-0382 L1）。
GUARD_EXEMPT_PREFIXES = (
    "conformance/asset/gltf/",
    "tests/geom_pages/golden/",
    "apps/uc09-taichi-spike/assets/particles.tcm",
)
# magic-bytes 已知签名（防改扩展名绕过）。
MAGIC_BYTES: tuple[tuple[str, bytes], ...] = (
    ("glb", b"glTF"),
    ("png", b"\x89PNG\r\n\x1a\n"),
    ("zip", b"PK\x03\x04"),
    ("dds", b"DDS "),
    ("hdr", b"#?RADIANCE"),
    ("ktx2", b"\xabKTX 20\xbb\r\n\x1a\n"),
)
_SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
_SPDX_EXPR_RE = re.compile(r"^(CC0-1\.0|CC-BY-3\.0|CC-BY-4\.0|LicenseRef-[A-Za-z0-9-]+)( AND (CC0-1\.0|CC-BY-3\.0|CC-BY-4\.0|LicenseRef-[A-Za-z0-9-]+))*$")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def canonical_json_bytes(doc) -> bytes:
    return (json.dumps(doc, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def canonical_json_digest(doc) -> str:
    return "sha256:" + hashlib.sha256(canonical_json_bytes(doc)).hexdigest()


def manifest_level_digest(base: Path) -> tuple[str, int, int, list[str]]:
    """清单级 canonical digest：逐文件 `相对路径 sha256`（正斜杠）按路径
    稳定排序，逐行 LF 连接再 sha256。返回 (digest, file_count, byte_len, files)。"""
    lines: list[tuple[str, str]] = []
    total = 0
    for p in sorted(base.rglob("*")):
        if not p.is_file():
            continue
        rel = p.relative_to(base).as_posix()
        lines.append((rel, sha256_file(p)))
        total += p.stat().st_size
    lines.sort(key=lambda t: t[0].encode("utf-8"))
    blob = "".join(f"{rel} {digest}\n" for rel, digest in lines).encode("utf-8")
    return "sha256:" + hashlib.sha256(blob).hexdigest(), len(lines), total, [rel for rel, _ in lines]


def resolve_cache_root() -> tuple[Path | None, str]:
    """缓存根解析序闭集（RXS-0382 L2）。返回 (root|None, source)。"""
    env = os.environ.get("RURIX_G10_CACHE_ROOT")
    if env:
        root = Path(env)
        return (root, "env:RURIX_G10_CACHE_ROOT") if root.is_dir() else (None, f"env:RURIX_G10_CACHE_ROOT 不可达({env})")
    if LOCAL_CONFIG.is_file():
        try:
            doc = json.loads(LOCAL_CONFIG.read_text(encoding="utf-8"))
            root = Path(str(doc.get("cache_root", "")))
        except (OSError, json.JSONDecodeError) as e:
            return None, f"机器局部配置不可读: {e}"
        return (root, "local_config") if root.is_dir() else (None, f"local_config cache_root 不可达({root})")
    return (DEFAULT_CACHE_ROOT, "default") if DEFAULT_CACHE_ROOT.is_dir() else (None, f"缺省缓存根不可达({DEFAULT_CACHE_ROOT})")


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def validate_spdx_expr(spdx: str, snapshot: object) -> str | None:
    """SPDX 受限表达式子集校验（RXS-0381 L1）；合法返回 None，否则返回失败原因。"""
    if not isinstance(spdx, str) or not _SPDX_EXPR_RE.match(spdx):
        return f"spdx_id {spdx!r} 非受限表达式子集（白名单 id 的 AND 组合 + LicenseRef-<name>）"
    ids = [t.strip() for t in spdx.split(" AND ")]
    for i in ids:
        if i.startswith("LicenseRef-"):
            if not (isinstance(snapshot, str) and snapshot and snapshot != "NONE"):
                return f"LicenseRef 行 {i} 缺 license_snapshot"
        elif i not in WHITELIST_SPDX:
            return f"spdx id {i} 不在白名单闭集 {list(WHITELIST_SPDX)}"
    return None


def validate_asset_row(row: dict, *, index: int) -> list[str]:
    """按类登记闭集校验（RXS-0381 L2/L3/L4）；返回 failure 列表。"""
    fails: list[str] = []
    who = f"assets[{index}]({row.get('asset_id', '?')})"
    cls = row.get("class")
    if cls not in ("external", "generated"):
        fails.append(f"{who} class ∈ {{external,generated}} 之外: {cls!r}")
        return fails
    if not isinstance(row.get("asset_id"), str) or not row["asset_id"]:
        fails.append(f"{who} asset_id 空")
    spdx = row.get("spdx_id")
    if cls == "external":
        for k in EXTERNAL_REQUIRED:
            if k not in row or row[k] in (None, ""):
                fails.append(f"{who} external 五元组缺字段 {k}")
        if spdx == "NONE":
            fails.append(f"{who} external 类谎报 spdx_id=NONE（两类互冒充）")
        elif isinstance(spdx, str):
            err = validate_spdx_expr(spdx, row.get("license_snapshot"))
            if err:
                fails.append(f"{who} {err}")
        src = row.get("source_url")
        if not (isinstance(src, str) and src.startswith("https://")):
            fails.append(f"{who} external 类 source_url 非 https URL: {src!r}")
        if "generator_script_digest" in row or "generator_params_digest" in row:
            fails.append(f"{who} external 类夹带 generated 类字段（两类互冒充）")
        attr = row.get("attribution")
        if not isinstance(attr, dict):
            fails.append(f"{who} attribution 非结构化对象")
        else:
            for k in ATTRIBUTION_KEYS:
                if k not in attr:
                    fails.append(f"{who} attribution 子字段缺失 {k}")
            if "modified_flag" in attr and not isinstance(attr["modified_flag"], bool):
                fails.append(f"{who} attribution.modified_flag 非布尔")
            if attr.get("modified_flag") is True and not attr.get("modification_note"):
                fails.append(f"{who} modified_flag=true 缺修改说明行")
            extra = set(attr) - set(ATTRIBUTION_KEYS) - {"modification_note"}
            if extra:
                fails.append(f"{who} attribution 闭集外字段 {sorted(extra)}")
    else:
        for k in GENERATED_REQUIRED:
            if k not in row or row[k] in (None, ""):
                fails.append(f"{who} generated 六字段缺字段 {k}")
        if spdx != "NONE":
            fails.append(f"{who} generated 类 spdx_id ≠ NONE（谎报外部许可 {spdx!r}）")
        if row.get("source_url") != "NONE":
            fails.append(f"{who} generated 类 source_url ≠ NONE（谎报外部来源）")
        for k in ("generator_script_digest", "generator_params_digest"):
            v = row.get(k)
            if isinstance(v, str) and not _SHA256_RE.match(v):
                fails.append(f"{who} {k} 非 sha256:<64hex> 形态")
    for k in COMMON_REQUIRED:
        if k not in row or row[k] in (None, ""):
            fails.append(f"{who} 通用字段缺失 {k}")
    d = row.get("digest")
    if isinstance(d, str) and not _SHA256_RE.match(d):
        fails.append(f"{who} digest 非 sha256:<64hex> 形态")
    cr = row.get("cache_rel")
    if isinstance(cr, str):
        if "\\" in cr or re.match(r"^[A-Za-z]:", cr) or cr.startswith("/"):
            fails.append(f"{who} cache_rel 含主机绝对路径/反斜杠（禁入签名面）: {cr!r}")
    for k in ("file_count", "byte_len"):
        v = row.get(k)
        if not isinstance(v, int) or v < 0 or (k == "file_count" and v < 1):
            fails.append(f"{who} {k} 非正整数")
    return fails


def validate_registry(doc: dict) -> list[str]:
    """注册表结构闭集校验（不触缓存）。返回 failure 列表。"""
    fails: list[str] = []
    if doc.get("schema") != "rurix.g10.asset_license_registry.v1":
        fails.append(f"schema ≠ rurix.g10.asset_license_registry.v1: {doc.get('schema')!r}")
    if doc.get("spec_anchor") != "RXS-0381":
        fails.append(f"spec_anchor ≠ RXS-0381: {doc.get('spec_anchor')!r}")
    if tuple(doc.get("whitelist_spdx", [])) != WHITELIST_SPDX:
        fails.append(f"whitelist_spdx ≠ {list(WHITELIST_SPDX)}: {doc.get('whitelist_spdx')!r}")
    guard = doc.get("git_binary_guard")
    if not isinstance(guard, dict):
        fails.append("缺 git_binary_guard 面")
    else:
        if tuple(guard.get("extension_closed_set", [])) != GUARD_EXTENSIONS:
            fails.append("git_binary_guard.extension_closed_set 与 RXS-0382 L1 闭集不符")
        if not isinstance(guard.get("threshold_bytes"), int) or guard.get("threshold_bytes", 0) < 1:
            fails.append("git_binary_guard.threshold_bytes 非正整数（须 measured 标定）")
        if "measured" not in str(guard.get("threshold_provenance", "")):
            fails.append("git_binary_guard.threshold_provenance 缺 measured 标定来源（P-09 禁手写）")
        if tuple(guard.get("exempt_paths", [])) != GUARD_EXEMPT_PREFIXES:
            fails.append("git_binary_guard.exempt_paths 与白名单路径闭集不符")
    assets = doc.get("assets")
    if not isinstance(assets, list) or not assets:
        fails.append("assets 空（注册表零行不成立）")
        return fails
    seen: set[str] = set()
    for i, row in enumerate(assets):
        if not isinstance(row, dict):
            fails.append(f"assets[{i}] 非对象")
            continue
        aid = row.get("asset_id")
        if aid in seen:
            fails.append(f"asset_id 重复: {aid!r}")
        seen.add(aid)
        fails.extend(validate_asset_row(row, index=i))
    return fails


def verify_asset_cache(row: dict, cache_root: Path) -> list[str]:
    """逐文件实算 + 清单级 canonical digest 复算比对（RXS-0382 L4）。"""
    fails: list[str] = []
    aid = row.get("asset_id", "?")
    base = cache_root / str(row.get("cache_rel", ""))
    if not base.is_dir():
        return [f"{aid} 缓存目录不可达: {base}"]
    scope = row.get("digest_scope")
    if isinstance(scope, list) and scope:
        # 限定子路径闭集（如 bistro: zip + extracted/）。
        lines: list[tuple[str, str]] = []
        total = 0
        for item in sorted(scope):
            p = base / item
            if p.is_file():
                lines.append((item.replace("\\", "/"), sha256_file(p)))
                total += p.stat().st_size
            elif p.is_dir():
                d, n, b, files = manifest_level_digest(p)
                for rel in files:
                    full = p / rel
                    lines.append((f"{item.rstrip('/')}/{rel}".replace("\\", "/"), sha256_file(full)))
                    total += full.stat().st_size
            else:
                fails.append(f"{aid} digest_scope 项缺失: {item}")
        lines.sort(key=lambda t: t[0].encode("utf-8"))
        blob = "".join(f"{rel} {dg}\n" for rel, dg in lines).encode("utf-8")
        digest = "sha256:" + hashlib.sha256(blob).hexdigest()
        count, blen = len(lines), total
    else:
        digest, count, blen, _ = manifest_level_digest(base)
    if digest != row.get("digest"):
        fails.append(f"{aid} 清单级 canonical digest 不符（登记 {row.get('digest')} ≠ 实算 {digest}）")
    if count != row.get("file_count"):
        fails.append(f"{aid} file_count 不符（登记 {row.get('file_count')} ≠ 实算 {count}）")
    if blen != row.get("byte_len"):
        fails.append(f"{aid} byte_len 不符（登记 {row.get('byte_len')} ≠ 实算 {blen}）")
    gen_script = row.get("generator_script")
    if isinstance(gen_script, str):
        sp = ROOT / gen_script
        if not sp.is_file():
            fails.append(f"{aid} 生成器脚本缺失: {gen_script}")
        elif "sha256:" + sha256_file(sp) != row.get("generator_script_digest"):
            fails.append(f"{aid} generator_script_digest 与仓内脚本实算不符")
    derived = row.get("derived")
    if isinstance(derived, dict):
        dbase = base / str(derived.get("output_rel", ""))
        if not dbase.is_dir():
            fails.append(f"{aid} derived 产物目录不可达: {dbase}")
        else:
            dd, dn, db, _ = manifest_level_digest(dbase)
            if dd != derived.get("digest"):
                fails.append(f"{aid} derived digest 不符（登记 {derived.get('digest')} ≠ 实算 {dd}）")
            if dn != derived.get("file_count") or db != derived.get("byte_len"):
                fails.append(f"{aid} derived file_count/byte_len 不符")
    return fails


def git_binary_guard(registry_doc: dict) -> list[str]:
    """git 零二进制守卫（RXS-0382 L1）：扩展名闭集 + measured 体积阈值双判 +
    magic-bytes 嗅探；白名单路径闭集豁免。返回 failure 列表。"""
    guard = registry_doc.get("git_binary_guard", {})
    threshold = int(guard.get("threshold_bytes", 0) or 0)
    fails: list[str] = []
    r = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        return [f"git ls-files 失败: {r.stderr.strip()}"]
    for rel in r.stdout.splitlines():
        rel = rel.strip()
        if not rel or rel.startswith(GUARD_EXEMPT_PREFIXES):
            continue
        ext = os.path.splitext(rel)[1].lower()
        p = ROOT / rel
        if ext in GUARD_EXTENSIONS:
            size = p.stat().st_size if p.is_file() else 0
            fails.append(f"git 二进制守卫命中（扩展名闭集 {ext}）: {rel} ({size}B)")
            continue
        if p.is_file() and threshold > 0 and p.stat().st_size >= threshold:
            fails.append(f"git 二进制守卫命中（≥ measured 阈值 {threshold}B）: {rel}")
            continue
        if p.is_file():
            head = p.open("rb").read(16)
            for name, magic in MAGIC_BYTES:
                if head.startswith(magic):
                    fails.append(f"git 二进制守卫命中（magic-bytes {name}）: {rel}")
                    break
    return fails


def manifest_scenes_digest(scenes: list[dict]) -> str:
    """清单 canonical digest（RXS-0383 L2）：行按 scene_id 稳定排序后 canonical JSON。"""
    ordered = sorted(scenes, key=lambda r: str(r.get("scene_id", "")))
    return canonical_json_digest({"scenes": ordered})


def validate_manifest(doc: dict, registry_doc: dict) -> list[str]:
    """场景清单校验（RXS-0383 L1~L5；不触 M132 加载 evidence）。返回 failure 列表。"""
    fails: list[str] = []
    if doc.get("schema") != "rurix.g10.corpus_scene_manifest.v1":
        fails.append(f"schema ≠ rurix.g10.corpus_scene_manifest.v1: {doc.get('schema')!r}")
    if doc.get("spec_anchor") != "RXS-0383":
        fails.append(f"spec_anchor ≠ RXS-0383: {doc.get('spec_anchor')!r}")
    scenes = doc.get("scenes")
    if not isinstance(scenes, list) or not scenes:
        fails.append("scenes 空（空清单 vacuous truth 不构 PASS）")
        return fails
    registered = {a.get("asset_id") for a in registry_doc.get("assets", [])}
    seen: set[str] = set()
    for i, row in enumerate(scenes):
        who = f"scenes[{i}]({row.get('scene_id', '?')})"
        for k in ("scene_id", "asset_id", "camera_ref", "lighting_ref", "status"):
            if k not in row or row[k] in (None, ""):
                fails.append(f"{who} 缺字段 {k}")
        sid = row.get("scene_id")
        if sid in seen:
            fails.append(f"scene_id 重复: {sid!r}")
        seen.add(sid)
        if row.get("asset_id") not in registered:
            fails.append(f"{who} asset_id 未在许可注册表登记（门序硬约束）: {row.get('asset_id')!r}")
        if row.get("status") not in ("ready", "not-ready"):
            fails.append(f"{who} status ∉ {{ready, not-ready}}: {row.get('status')!r}")
        for k in ("camera_ref", "lighting_ref"):
            ref = row.get(k)
            if isinstance(ref, str) and not (ROOT / "milestones" / "g10" / ref).is_file():
                fails.append(f"{who} {k} 引用文件缺失: {ref}")
    ready = [r for r in scenes if r.get("status") == "ready"]
    if len(ready) < 2:
        fails.append(f"ready 场景数 {len(ready)} < 首发清单基数 2（vacuous 拦截）")
    revisions = doc.get("revisions")
    if not isinstance(revisions, list) or not revisions:
        fails.append("revisions 空（未注册 digest 冒充冻结）")
        return fails
    ids = [r.get("revision") for r in revisions]
    if ids != sorted(ids) or len(set(ids)) != len(ids) or ids[0] != 1:
        fails.append(f"revision 序列非自 1 单调递增无重复: {ids}")
    latest = revisions[-1]
    want = manifest_scenes_digest(scenes)
    if latest.get("manifest_digest") != want:
        fails.append(
            f"清单 digest 与最新修订注册不符（原地改/未注册冒充检测）：注册 {latest.get('manifest_digest')} ≠ 实算 {want}"
        )
    for r in revisions:
        for k in ("revision", "manifest_digest", "changed_at", "change_note"):
            if k not in r or r[k] in (None, ""):
                fails.append(f"修订行缺字段 {k}: {r.get('revision')!r}")
        d = r.get("manifest_digest")
        if isinstance(d, str) and not _SHA256_RE.match(d):
            fails.append(f"修订行 manifest_digest 形态非法: {d!r}")
    return fails
