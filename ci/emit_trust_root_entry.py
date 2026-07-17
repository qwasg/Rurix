#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""信任根登记条目生成器(EA1.2 / RFC-0012 §4.7 step 7,RXS-0218)。

release.yml 的信任根登记流(生成 channels/stable.json 新条目 → 自动开 PR → owner
合并人工门)调用本脚本产出/更新 `channels/stable.json`。**确定性 line-scan 形态**
(每字段独立行,无时间戳),与 `src/rurixup/src/fetch.rs` `Anchor::from_json` 解析
形态逐字对齐;保留既有 releases 条目(按 version 字典序),同版号覆盖(内容寻址唯一)。

用法:py -3 ci/emit_trust_root_entry.py <version> <channel_manifest_sha256> <base_url>

**演练本批不执行**——本脚本仅在真实 release run 内被 release.yml 调用。零真实外呼、
零第三方依赖(stdlib)。退出码:0=成功;非零=用法/IO 错误。
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ANCHOR = ROOT / "channels" / "stable.json"


def render(rels: list, latest: str) -> bytes:
    """确定性 line-scan 形态(与 fetch.rs Anchor::from_json 解析对齐,无时间戳)。"""
    lines = ["{", '  "schema_version": 1,', '  "channel": "stable",', '  "releases": [']
    for i, r in enumerate(rels):
        comma = "," if i + 1 < len(rels) else ""
        lines += [
            "    {",
            f'      "version": "{r["version"]}",',
            f'      "channel_manifest_sha256": "{r["channel_manifest_sha256"]}",',
            f'      "base_url": "{r["base_url"]}"',
            "    }" + comma,
        ]
    lines += ["  ],", f'  "latest": "{latest}"', "}", ""]
    return "\n".join(lines).encode("utf-8")


def main() -> int:
    if len(sys.argv) != 4:
        print("用法:py -3 ci/emit_trust_root_entry.py <version> <sha256> <base_url>",
              file=sys.stderr)
        return 2
    ver, digest, base_url = sys.argv[1], sys.argv[2], sys.argv[3]
    if ANCHOR.is_file():
        doc = json.loads(ANCHOR.read_text(encoding="utf-8"))
    else:
        doc = {"schema_version": 1, "channel": "stable", "releases": [], "latest": ""}
    # 同版号覆盖(内容寻址唯一)+ 追加新条目 + 版号字典序。
    rels = [r for r in doc.get("releases", []) if r.get("version") != ver]
    rels.append({"version": ver, "channel_manifest_sha256": digest, "base_url": base_url})
    rels.sort(key=lambda r: r["version"])
    ANCHOR.parent.mkdir(parents=True, exist_ok=True)
    ANCHOR.write_bytes(render(rels, ver))
    print(f"channels/stable.json 登记 {ver}(digest={digest[:12]}…,共 {len(rels)} 条,latest={ver})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
