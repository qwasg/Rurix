# V1.3 v1.0.0 发行机器事实归档(2026-07-14)

> 所属:milestones/v1/V1_CONTRACT.md D-V1-4 / D-V1-5(G-V1-4 / G-V1-5)。evidence/ 只增不删。

## 1. 发行链路留痕

| 项 | 值 |
|---|---|
| annotated tag | `v1.0.0`(锚定 main merge `316124b5`,PR #124) |
| release workflow run | <https://github.com/qwasg/Rurix/actions/runs/29328321309>(**全量 success**) |
| GitHub Release(首个) | <https://github.com/qwasg/Rurix/releases/tag/v1.0.0> |
| FCP-lite 公示 | <https://github.com/qwasg/Rurix/issues/121>(保持开放;发布回填评论 issuecomment-4968659208) |

## 2. 机器发布门(10 §6 / 08 §9)兑现明细(run 29328321309)

schema check / guardrails / trace freshness / NVIDIA 白名单审计 / budget --strict(pre-release)/ cargo fmt/clippy/test(conformance + UI golden 全绿)/ **Release pipeline sign/SBOM/audit gate(RURIXUP_SIGN=1 真实 Authenticode)** / channel manifest smoke(步骤 50 同款)/ budget release counter(strict)/ upload-artifact——**任一红即不上传,实测全绿**。

run 日志关键行(真实输出):

```
[release_smoke] 真实 Authenticode 绿:allow_upload=true signed_artifacts=['rurixup.exe', 'rx.exe']
```

## 3. 版号一致性三点核验(RXS-0135 同版号判据)

| 锚点 | 值 |
|---|---|
| git tag | v1.0.0 |
| Cargo.toml workspace.version | 1.0.0 |
| bundle.json `rurix_version`(run artifact) | 1.0.0 |
| channel_manifest.json `rurix_version` / `channel`(run artifact) | 1.0.0 / **stable** |
| evidence(run artifact)`bundle.rurix_version` + `run_url` | 1.0.0 + run 29328321309 |

## 4. GitHub Release 附件与 SHA-256(逐字节)

```
af0949202fc417f9f9798f1b9da009e1ebf02f066c030842c2c9a0d67975a773  bundle.json
2c057c6c9d0d909fbff90f3b9a4ba7d95ad1e540dbefd19ab4ccf7541c77a5ef  channel_manifest.json
42332b0714d500fd537bda99c7e23ccec470750641611466bdc7c9981492f79d  gate_decision.json
10efe2fd9843c1ef9bec3d8c4b88d58cd8cef946fa28d8e8c20487996805e393  release_pipeline_smoke.json
14434f8778f523cba7666eff7a41ccfa39fa5cd6c640c3ee2655162c0a7aeb73  rurixup.exe
803a84ba3a28cf7c5b880e8e71f0e7d0bcf6ee13d45b7b07f8917ad863c8a8b1  rx.exe
1d0415c6afbf491ee25cb052e8c0d9fa4688ef78c13c7b238d5c804ce5cd41be  sbom.cdx.json
b1656cca9fd57cd6e5392073da5c6f305e28efcfb24d19a7688bb2f221cc731c  sbom.spdx.json
3a92911f544dec9191dc0b7e65d301d522a754a435777f8ee479a62893202c8e  signing_manifest.json
```

## 5. 签名状态(诚实标注)

- 附件二进制经**自签测试证书**真实 Authenticode 签名 + RFC 3161 时间戳(签名主体 `CN=Rurix Release Smoke Test (temporary)`);run 内验签 `Valid`,run 后临时证书按设计自证书库移除,故事后链验证为 untrusted——与 Release 说明「SmartScreen 会告警,非生产证书」口径一致。
- of-record 生产签名(Azure Artifact Signing)维持 secret + 人工门控(spec/release.md §4),不作 1.0 阻断门(V1_CONTRACT §7 ⑤,用户 2026-07-14 裁决)。
- Release body 定稿见 [gh_release_body.md](gh_release_body.md)。
