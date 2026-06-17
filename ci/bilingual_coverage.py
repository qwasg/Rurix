#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""诊断消息中英双语全量覆盖核对(M8 CI_GATES §2 步骤 37,契约 G-M8-5,RD-006;CPU-only,check_ 守卫风格)。

解析 src/rurixc/src/messages/{en,zh}.messages,断言 zh 与 en 的 message-key 集**完全对齐**
(zh 缺键 / zh 多键即红,反 YAML-only)。内置 red 自检:构造一对故意不对齐的合成表,断言比较器
判为「不对齐」——证明门真的在比对 key 集、能区分「对齐 vs 缺/多键」,而非空过。

全对齐 → 写 evidence/bilingual_diagnostic_coverage.json(coverage_complete=true + en/zh key 计数 +
facts + redgreen)+ 退出 0(绿);coverage_complete=true 计入 m8.counter.bilingual_diagnostic_coverage
(ci/budget_eval.py;>=1 则 PASS,双语全量回填前无证据 → 0 → normal SKIP 属预期)。不对齐 → 打印
缺键/多键清单 + 非零退出(红);仅绿写证据(契合 evidence/ 只增不删不改)。
"""
import datetime
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EN = ROOT / "src" / "rurixc" / "src" / "messages" / "en.messages"
ZH = ROOT / "src" / "rurixc" / "src" / "messages" / "zh.messages"
EVIDENCE = ROOT / "evidence" / "bilingual_diagnostic_coverage.json"
RUN_URL_TODO = "TODO:回填 self-hosted runner 绿→红(缺键)→补译复绿 run URL(步骤 37,CI_GATES §6 第 4 项)"


def fail(msg):
    print(f"[bilingual] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def parse_keys(text, label):
    """解析行格式 `key = 模板`(与 src/rurixc/src/messages.rs::MessageTable::parse 一致):
    跳过空行 / `#` 注释;`split('=', 1)`;key 去空白、非空、无内部空白;重复 key / 缺 `=` → 表损坏即红。
    返回 key 集合(set)。"""
    keys = set()
    for lineno, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            fail(f"{label} 第 {lineno} 行缺 '=': {line!r}")
        key = line.split("=", 1)[0].strip()
        if not key or any(c.isspace() for c in key):
            fail(f"{label} 第 {lineno} 行 key 非法: {key!r}")
        if key in keys:
            fail(f"{label} 第 {lineno} 行 key 重复: {key!r}")
        keys.add(key)
    return keys


def diff(en_keys, zh_keys):
    """返回 (zh 缺键 = en−zh, zh 多键 = zh−en),均有序。"""
    return sorted(en_keys - zh_keys), sorted(zh_keys - en_keys)


def red_self_test():
    """red 自检(反 YAML-only):合成 en(2 key)与 zh(缺 b.y、多 c.z),断言比较器判「不对齐」。
    若误判「对齐」→ 门空过失效 → 红。"""
    en = parse_keys("a.x = 1\nb.y = 2\n", "<self-test-en>")
    zh = parse_keys("a.x = 1\nc.z = 3\n", "<self-test-zh>")
    missing, extra = diff(en, zh)
    if missing != ["b.y"] or extra != ["c.z"]:
        fail(f"red 自检失败:比较器未识别缺键/多键(missing={missing},extra={extra},门失效)")


def preserved_run_url():
    """保留人工/CI 回填过的 run URL,避免后续绿跑把证据刷回 TODO。"""
    if not EVIDENCE.is_file():
        return RUN_URL_TODO
    try:
        prior = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return RUN_URL_TODO
    run_url = prior.get("redgreen", {}).get("run_url")
    if isinstance(run_url, str) and run_url and not run_url.startswith("TODO:"):
        return run_url
    return RUN_URL_TODO


def main():
    red_self_test()
    if not EN.is_file():
        fail(f"缺 {EN.relative_to(ROOT)}")
    if not ZH.is_file():
        fail(f"缺 {ZH.relative_to(ROOT)}(双语全量覆盖未回填,RD-006)")

    en_keys = parse_keys(EN.read_text(encoding="utf-8"), "en.messages")
    zh_keys = parse_keys(ZH.read_text(encoding="utf-8"), "zh.messages")
    missing_in_zh, extra_in_zh = diff(en_keys, zh_keys)
    if missing_in_zh or extra_in_zh:
        fail(
            "zh/en message-key 集不对齐(反 YAML-only 红):\n"
            f"  zh 缺键({len(missing_in_zh)}): {missing_in_zh}\n"
            f"  zh 多键({len(extra_in_zh)}): {extra_in_zh}"
        )

    doc = {
        "schema_version": 1,
        "subject": "bilingual_diagnostic_coverage",
        "coverage_complete": True,
        "en_key_count": len(en_keys),
        "zh_key_count": len(zh_keys),
        "missing_in_zh": [],
        "extra_in_zh": [],
        "facts": [
            {
                "kind": "coverage",
                "name": "zh_en_key_set_aligned",
                "en_key_count": len(en_keys),
                "zh_key_count": len(zh_keys),
                "note": "zh 与 en message-key 集完全对齐(无缺键 / 多键)",
            }
        ],
        "redgreen": {
            "red_command": "删/注释 zh.messages 任一 key(en 有 zh 无)→ py -3 ci/bilingual_coverage.py 退出 1",
            "red_detected": True,
            "green_command": "py -3 ci/bilingual_coverage.py",
            "green_exit_code": 0,
            "run_url": preserved_run_url(),
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[bilingual] PASS 写 {EVIDENCE.relative_to(ROOT)}"
        f"(coverage_complete=true,zh/en key 集对齐 {len(zh_keys)}/{len(en_keys)})"
    )
    sys.exit(0)


if __name__ == "__main__":
    main()
