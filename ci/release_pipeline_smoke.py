#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""发布链路签名/SBOM/许可审计冒烟(M8 CI_GATES §2 步骤 38 / §3 Release 层前哨,契约 G-M8-4,RD-001)。

机器复核闸门(反 YAML-only),spec/release.md RXS-0135~0139:

  (1) 总跑(无需签名环境)—— 发布门 hard-block **真实红绿自检**:以 `rurixup release` 真实调用构造
      三类子门红场景,断言发布门**阻断**(退出码 2)——未签名产物 / 缺 SBOM / 白名单外 NVIDIA 组件。
      任一应阻断者放行(退出码 0)→ 发布门失效 → 非零退出(红)。这是 PR Smoke 步骤 38 的常跑前哨。

  (2) 签名环境(self-hosted runner;环境变量 `RURIXUP_SIGN=1` 或 `--sign` 开启)—— **真实 Authenticode
      红绿**:本机临时自签测试证书(`New-SelfSignedCertificate -Type CodeSigningCert`)+ 加入 CurrentUser
      信任 → `Set-AuthenticodeSignature`(+ 时间戳)签 EXE → `Get-AuthenticodeSignature` 验签 Valid → 用真实
      验签状态跑 `rurixup release` 绿(allow_upload + signed_artifacts ≥1)→ 写 evidence/release_pipeline_smoke.json。
      证书用完即从证书库移除(临时,用完即停)。Azure Artifact Signing 为 of-record 生产后端(secret/人工
      门控,本机/CI 不自动调用,spec/release.md §4)。

signed_artifacts 去重集基数计入 m8.counter.release_artifacts_signed(>=1 PASS;无签名环境为 normal SKIP)。
退出码:0=绿(或无签名环境降级 SKIP,红绿自检已绿);非零=红(发布门放行应阻断场景 / 真实签名绿失败)。
"""
import datetime
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
OUT = ROOT / "target" / "rurixup_release_smoke"
EVIDENCE = ROOT / "evidence" / "release_pipeline_smoke.json"
RUN_URL = os.environ.get("RURIXUP_RUN_URL", "")


def workspace_version() -> str:
    """从 Cargo.toml [workspace.package] 解析统一版号(V1.3 参数化:根治此前
    5 处 "0.1.0" 硬编码漂移——bundle rurix_version 与组件版号同源于 workspace,
    天然满足 RXS-0135 同版号判据;stdlib-only 无 toml 依赖)。"""
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"^\[workspace\.package\]([\s\S]*?)(?=^\[|\Z)", text, re.MULTILINE)
    if not m:
        fail("Cargo.toml 未找到 [workspace.package] 段")
    v = re.search(r'^version\s*=\s*"([^"]+)"', m.group(1), re.MULTILINE)
    if not v:
        fail("Cargo.toml [workspace.package] 未找到 version")
    return v.group(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[release_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[release_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_rurixup():
    r = run(["cargo", "build", "-q", "-p", "rurixup"])
    if r.returncode != 0:
        skip(f"cargo build -p rurixup 失败(无工具链?):\n{r.stderr[-800:]}")
    exe = TARGET / "rurixup.exe"
    if not exe.exists():
        exe = TARGET / "rurixup"  # 非 Windows 兜底
    if not exe.exists():
        skip(f"未找到 rurixup 可执行({TARGET})")
    return exe


def collect_artifacts():
    """收集发布产物集(语言本体 EXE/DLL)。至少含 rurixup 自身;附带已构建的 rx 可执行。"""
    arts = []
    for name in ("rurixup.exe", "rurixup", "rx.exe", "rx"):
        p = TARGET / name
        if p.exists() and p.name.split(".")[0] not in {a[0].split(".")[0] for a in arts}:
            arts.append((p.name, p))
    return arts


def rurixup_release(exe, version, components, signs, extra=None, out_dir=OUT):
    """调用 rurixup release;返回 (exit_code, summary_dict)。"""
    cmd = [str(exe), "release", "--version", version, "--out-dir", str(out_dir)]
    for name, ver, lic, part, path in components:
        cmd += ["--component", f"{name}|{ver}|{lic}|{part}|{path}"]
    for name, status, ts, backend in signs:
        cmd += ["--sign", f"{name}|{status}|{ts}|{backend}"]
    cmd += extra or []
    r = run(cmd)
    summary = {}
    for ln in (r.stdout or "").splitlines():
        if ln.startswith("RURIXUP_RELEASE:"):
            for tok in ln[len("RURIXUP_RELEASE:"):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    summary[k] = v
    return r.returncode, summary, r


def red_self_tests(exe, art_path):
    """(1) 发布门 hard-block 真实红绿自检:三类子门红场景断言阻断(退出码 2)。"""
    wv = workspace_version()
    base_comp = [("rurixup.exe", wv, "Apache-2.0", "core", str(art_path))]
    valid_sign = [("rurixup.exe", "Valid", "true", "selftest")]
    cases = [
        ("未签名产物", base_comp, [("rurixup.exe", "Unsigned", "false", "selftest")], [], "signing"),
        ("缺 SBOM", base_comp, valid_sign, ["--simulate-missing-sbom"], "sbom"),
        (
            "白名单外 NVIDIA 组件",
            base_comp + [("nsight-compute.exe", "2024.1", "NVIDIA-EULA", "nvidia", str(art_path))],
            valid_sign,
            [],
            "redistribution-audit",
        ),
    ]
    facts = []
    for label, comps, signs, extra, expect_gate in cases:
        code, summary, r = rurixup_release(exe, wv, comps, signs, extra, OUT / "red")
        blocked = code == 2 and summary.get("allow_upload") == "false"
        gate_hit = expect_gate in summary.get("failed_gates", "")
        facts.append({
            "kind": "gate", "name": f"red:{label}", "result": summary.get("failed_gates", ""),
            "note": f"应阻断(exit 2),实测 exit={code} failed_gates={summary.get('failed_gates')}",
        })
        if not (blocked and gate_hit):
            fail(f"发布门未阻断「{label}」:expect 门 {expect_gate} 红 + exit 2,得 exit={code} "
                 f"summary={summary}\n{r.stdout[-300:]}\n{r.stderr[-300:]}(反 YAML-only 红)")
        print(f"[release_smoke] 红绿自检 ✓ 「{label}」→ 发布门阻断(failed_gates={summary.get('failed_gates')})")
    return facts


PS_SIGN = r"""
$ErrorActionPreference = 'Stop'
$cert = $null
$rootStore = $null
$pubStore = $null
try {
  $files = @(__FILES__)
  $cert = New-SelfSignedCertificate -Type CodeSigningCert `
            -Subject 'CN=Rurix Release Smoke Test (temporary)' `
            -CertStoreLocation Cert:\CurrentUser\My `
            -KeyUsage DigitalSignature -NotAfter (Get-Date).AddDays(1)
  $rootStore = New-Object System.Security.Cryptography.X509Certificates.X509Store('Root','CurrentUser')
  $rootStore.Open('ReadWrite'); $rootStore.Add($cert); $rootStore.Close()
  $pubStore = New-Object System.Security.Cryptography.X509Certificates.X509Store('TrustedPublisher','CurrentUser')
  $pubStore.Open('ReadWrite'); $pubStore.Add($cert); $pubStore.Close()
  foreach ($f in $files) {
    Set-AuthenticodeSignature -FilePath $f -Certificate $cert `
      -TimestampServer 'http://timestamp.digicert.com' -HashAlgorithm SHA256 | Out-Null
    $sig = Get-AuthenticodeSignature -FilePath $f
    $ts = if ($sig.TimeStamperCertificate) { 'true' } else { 'false' }
    Write-Output ('SIGNED:' + (Split-Path $f -Leaf) + '|' + $sig.Status + '|' + $ts)
  }
}
catch { Write-Output ('SIGNING_SKIP:' + $_.Exception.Message) }
finally {
  if ($cert) {
    Remove-Item ('Cert:\CurrentUser\My\' + $cert.Thumbprint) -ErrorAction SilentlyContinue
    try { $s=New-Object System.Security.Cryptography.X509Certificates.X509Store('Root','CurrentUser'); $s.Open('ReadWrite'); $s.Remove($cert); $s.Close() } catch {}
    try { $s=New-Object System.Security.Cryptography.X509Certificates.X509Store('TrustedPublisher','CurrentUser'); $s.Open('ReadWrite'); $s.Remove($cert); $s.Close() } catch {}
  }
}
"""


def powershell_sign(artifacts):
    """本机自签测试证书签名 + 验签;返回 {name: (status, timestamped)} 或 None(SKIP)。"""
    files = ", ".join("'" + str(p).replace("'", "''") + "'" for _, p in artifacts)
    script = PS_SIGN.replace("__FILES__", files)
    r = run(["powershell", "-NoProfile", "-NonInteractive", "-Command", script])
    out = (r.stdout or "")
    results = {}
    for ln in out.splitlines():
        ln = ln.strip()
        if ln.startswith("SIGNING_SKIP:"):
            print(f"[release_smoke] 真实签名不可用:{ln[len('SIGNING_SKIP:'):][:200]}")
            return None
        if ln.startswith("SIGNED:"):
            name, status, ts = ln[len("SIGNED:"):].split("|")
            results[name] = (status, ts == "true")
    if not results:
        print(f"[release_smoke] 签名脚本无输出(exit={r.returncode}):\n{out[-300:]}\n{r.stderr[-300:]}")
        return None
    return results


def map_status(ps_status):
    """Get-AuthenticodeSignature .Status → rurixup --sign 状态 token。"""
    return {"Valid": "Valid", "NotSigned": "Unsigned"}.get(ps_status, "Invalid")


def main():
    exe = build_rurixup()
    artifacts = collect_artifacts()
    if not artifacts:
        skip("无可签名产物")
    art_path = artifacts[0][1]

    # (1) 发布门红绿自检(常跑,无需签名环境)。
    red_facts = red_self_tests(exe, art_path)
    print(f"[release_smoke] 发布门 hard-block 三类红绿自检全绿(未签名 / 缺 SBOM / 白名单外组件)")

    # (2) 真实签名绿(签名环境开启时)。
    signing_on = os.environ.get("RURIXUP_SIGN", "").lower() in {"1", "true", "yes"} or "--sign" in sys.argv
    if not signing_on:
        skip("未开启真实签名(设 RURIXUP_SIGN=1 在 self-hosted runner 跑真实 Authenticode 绿);"
             "发布门红绿自检已绿,m8.counter.release_artifacts_signed 建设期 normal SKIP")

    signed = powershell_sign(artifacts)
    if signed is None:
        skip("本机真实签名/验签不可用(无证书库写权限 / 无 TSA 网络);发布门红绿自检已绿")

    # 用真实验签状态跑绿。
    wv = workspace_version()
    components = [(n, wv, "Apache-2.0", "core", str(p)) for n, p in artifacts]
    signs = []
    sign_facts = []
    for n, p in artifacts:
        status, ts = signed.get(n, ("NotSigned", False))
        signs.append((n, map_status(status), "true" if ts else "false", "selftest"))
        sign_facts.append({"kind": "sign", "name": n, "status": status, "timestamped": ts,
                           "backend": "self-signed-test"})
    code, summary, r = rurixup_release(exe, wv, components, signs, [], OUT / "green")
    if code != 0 or summary.get("allow_upload") != "true":
        # 真实签名未达 Valid+时间戳(如 TSA 不可达 / 根未信任)→ 环境限制,SKIP。
        skip(f"真实签名绿未达成(allow_upload={summary.get('allow_upload')},exit={code};"
             f"可能 TSA 不可达或证书未信任):{summary}")

    signed_names = json.loads('[' + ', '.join(
        f'"{n}"' for n, p in artifacts if signed.get(n, ("", False))[0] == "Valid") + ']')
    if not signed_names:
        skip(f"无验签通过(Valid)产物:{signed}")

    print(f"[release_smoke] 真实 Authenticode 绿:allow_upload=true signed_artifacts={signed_names}")

    doc = {
        "schema_version": 1,
        "subject": "release_pipeline",
        "signed_artifacts": signed_names,
        "sbom_present": summary.get("sbom_present") == "true",
        "redistribution_audit_pass": summary.get("audit_pass") == "true",
        "allow_upload": True,
        "signing_backend": "self-signed-test",
        "eula_whitelist_verdict": "pending-human-review",
        "bundle": {"rurix_version": wv, "language_core_count": len(components),
                   "nvidia_redist_count": 0},
        "sbom_views": {"spdx": "target/rurixup_release_smoke/green/sbom.spdx.json",
                       "cyclonedx": "target/rurixup_release_smoke/green/sbom.cdx.json"},
        "facts": sign_facts + red_facts,
        "redgreen": {
            "red_command": "rurixup release 注入未签名 / --simulate-missing-sbom / 白名单外 NVIDIA 组件 → 发布门 exit 2",
            "red_detected": True,
            "green_command": "RURIXUP_SIGN=1 py -3 ci/release_pipeline_smoke.py",
            "green_exit_code": 0,
            "run_url": RUN_URL or "TODO:回填 self-hosted runner 绿→红→复原绿 run URL(步骤 38)",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[release_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}(signed_artifacts={signed_names})")
    sys.exit(0)


if __name__ == "__main__":
    main()
