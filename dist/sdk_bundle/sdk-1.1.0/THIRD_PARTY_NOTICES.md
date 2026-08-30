<!-- Assisted-by: Cursor:Fable（G37 商业化收官 W5 license_gaps：GAP-01 闭合件） -->
# Rurix 第三方声明与许可文本集合（THIRD_PARTY_NOTICES）

> **性质**：本文件 = Rurix 发布件随附的第三方许可声明与许可文本集合，是
> `milestones/g31/g31_vendor_license_matrix.json` 缺口 **GAP-01**（发布 bundle 未随附
> 许可文本与第三方声明）的闭合件。随每次分发形态携带（接线 =
> `.github/workflows/release.yml` 发布编排 `--component` 与资产清单；机器核验 =
> `ci/g31_vendor_license_smoke.py --gate g31.waveC.license` closure 腿）。
> **事实源**：`Cargo.lock`（锁定版本 + crates.io 包 checksum 字面）；各许可文本 =
> 本机 cargo registry 源码包内 LICENSE 文件**逐字复制**（零改写、零凭记忆重造）。
> **对账面**：矩阵 JSON（16 项 vendor 盘点）+ `dist/sbom/third_party_embedded.cdx.json`
> （内嵌第三方库级 SBOM 补充视图，GAP-03 闭合件）。

---

## 1. Rurix 本体许可

Rurix 以 **MIT OR Apache-2.0** 双许可发布（`Cargo.toml` workspace.package license 字面；
D-003）。完整文本 = 仓根 `LICENSE-MIT` 与 `LICENSE-APACHE`，两件随发布资产同批分发
（release.yml 组件 `LICENSE-MIT` / `LICENSE-APACHE`）。

## 2. 现发行面内嵌第三方组件（语言工具链 release：rx.exe / rurixup.exe / rurix_rt_cabi.lib）

| 发行组件 | 内嵌第三方 crate | 版本（Cargo.lock 锁定） | 许可 | 上游 | 包 checksum（Cargo.lock 字面，sha256） |
|---|---|---|---|---|---|
| rx.exe | rowan | 0.15.18 | MIT OR Apache-2.0 | https://crates.io/crates/rowan | `62f509095fc8cc0c8c8564016771d458079c11a8d857e65861f045145c0d3208` |
| rx.exe | countme | 3.0.1 | MIT OR Apache-2.0 | https://crates.io/crates/countme | `7704b5fdd17b18ae31c4c1da5a2e0305a2bf17b5249300a9ee9ed7b72114c636` |
| rx.exe | hashbrown | 0.14.5 | MIT OR Apache-2.0 | https://crates.io/crates/hashbrown | `e5274423e17b7c9fc20b6e7e208532f9b19825d82dfd615708b70edd83df41f1` |
| rx.exe | memoffset | 0.9.1 | MIT | https://crates.io/crates/memoffset | `488016bfae457b036d996092f6cb448677611ce4449e970ceaf42695203f218a` |
| rx.exe | rustc-hash | 1.1.0 | Apache-2.0 OR MIT | https://crates.io/crates/rustc-hash | `08d43f7aa6b08d49f382cde6a7982047c3426db949b1424bc4b7ec9ae12c6ce2` |
| rx.exe | text-size | 1.1.1 | MIT OR Apache-2.0 | https://crates.io/crates/text-size | `f18aa187839b2bdb1ad2fa35ead8c4c2976b64e4363c386d45ac0f7ee85c9233` |
| rurixup.exe | —（零第三方内嵌） | — | — | — | — |
| rurix_rt_cabi.lib | —（零第三方内嵌） | — | — | — | — |

登记说明（诚实字面）：

- **rowan 版本口径**：矩阵条目 `rust_rowan` 登记 `version=0.15.1` 为 `src/rurixc/Cargo.toml`
  需求字面（`rowan = "0.15.1"`）；本表 0.15.18 为 `Cargo.lock` semver 解析的**实际内嵌版本**，
  两者同一依赖同一义务面。
- **传递闭包**：countme / hashbrown / memoffset / rustc-hash / text-size 五件由 rowan 引入
  （`Cargo.lock` rowan dependencies 字面），与 rowan 同批静态编译进 `rx.exe`。
  memoffset 的 `autocfg` 为构建期依赖（build.rs），不进产物，不在随附义务面。
- **rurixup.exe** 依赖闭包 = `rurix-pkg`（自有 crate），零第三方内嵌。
- **rurix_rt_cabi.lib** 依赖闭包 = `image-io` + `rurix-rt`（均自有）；`rurix-rt` 对 `rurixc`
  仅为 `[build-dependencies]`（build.rs 产 PTX 嵌入，产物为自研语言编译输出），`rurix-d3d12`
  可选默认关、其 `cc` 为构建期——**产物面零第三方 crate（rowan 不在其中）**。
- **双许可组件的条款选择**：上表 MIT OR Apache-2.0 双许可组件，本分发按 **MIT** 条款履行
  声明保留义务（§4 逐字文本随附）；Apache-2.0 备选条款文本 = 本分发随附之 `LICENSE-APACHE`
  （Apache License 2.0 规范全文）与各 crate 源码包内 `LICENSE-APACHE`。

## 3. SDK bundle 面与外部 SDK / 未来捆绑面（矩阵义务同链登记）

| 面 | 组件 | 许可 | 义务与随附件 |
|---|---|---|---|
| 渲染器 SDK bundle（`g31.waveC.dist` 16 组件） | `rurix_renderer_sdk.dll` 静态内嵌 **basis_universal 1.16.4**（tag 1.16.4 @ `900e40fb5d2502927360fe2f31762bdbb624455f`） | Apache-2.0 | 许可文本与 NOTICE 在树：`src/rurix-basis-sys/vendor/basis_universal/LICENSE`、`LICENSES/Apache-2.0.txt`、`src/rurix-basis-sys/NOTICE`；SDK bundle 分发时随附本文件 + 上述 NOTICE（Apache-2.0 §4.1/§4.4） |
| NGX / Streamline（DLSS SR，运行时装载不捆绑） | `sl.interposer.dll` + `nvngx_dlss.dll` 等 | NVIDIA RTX SDKs LICENSE（非 OSI；owner 接受在案，`milestones/g13/design/vendor_upscale_license_clearance.md`，引用不复制） | 当前零再分发；商用捆绑触发时随附许可文本与 NVIDIA 归属声明（矩阵 `streamline_ngx_dlss` obligations 字面） |
| FSR（FidelityFX SDK 2.0.0，运行时装载不捆绑） | `amd_fidelityfx_*_dx12.dll` | MIT | 当前零再分发；捆绑触发时随附 `license.md`（矩阵 `fsr_fidelityfx` 字面） |
| Taichi AOT 运行时（用户自备，不分发） | `taichi_c_api.dll` | Apache-2.0 | 捆绑触发时随附 Apache-2.0 文本与归属声明（矩阵 `taichi_aot_runtime` 字面） |
| Jolt 物理面（当前发布 bundle 不含物理组件） | JoltC ×2 线 / JoltPhysics 5.3.0 / 5.6.0 | MIT OR Apache-2.0 / MIT | 许可文本在树（`src/rurix-physics-sys*/vendor/JoltC/…`）；嵌 Jolt 的二进制分发触发时随附 |
| rapier3d（feature `rapier` 默认 off，不在分发面） | rapier3d =0.33.0 + 传递闭包 | Apache-2.0 族 | 启用分发触发时随附声明（矩阵 `rust_rapier3d` 字面） |
| NVIDIA CUDA 面（零捆绑） | libdevice.10.bc / cublas64_* | NVIDIA CUDA EULA Attachment A | 白名单机制在案（`ci/check_redistribution.py` + `audit_redistribution`）；捆绑限白名单最小集 |

## 4. 第三方许可文本（上游源码包逐字）

以下文本逐字复制自各 crate 的 crates.io 源码包（本机 cargo registry 缓存，包 checksum
见 §2 表）。rowan / countme / rustc-hash / text-size 四件上游 `LICENSE-MIT` 文件本身不含
独立版权行（逐字如源）；hashbrown 与 memoffset 版权行逐字保留如下。

### 4.1 rowan 0.15.18 — LICENSE-MIT（MIT OR Apache-2.0 双许可，本分发选择 MIT）

```
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 4.2 countme 3.0.1 — LICENSE-MIT（MIT OR Apache-2.0 双许可，本分发选择 MIT）

```
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 4.3 hashbrown 0.14.5 — LICENSE-MIT（MIT OR Apache-2.0 双许可，本分发选择 MIT）

```
Copyright (c) 2016 Amanieu d'Antras

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 4.4 memoffset 0.9.1 — LICENSE（MIT）

```
Copyright (c) 2017 Gilad Naaman

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### 4.5 rustc-hash 1.1.0 — LICENSE-MIT（Apache-2.0 OR MIT 双许可，本分发选择 MIT）

```
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 4.6 text-size 1.1.1 — LICENSE-MIT（MIT OR Apache-2.0 双许可，本分发选择 MIT）

```
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 4.7 basis_universal 1.16.4 — Apache-2.0（SDK bundle 面，static-in-dll）

完整 Apache License 2.0 文本与 NOTICE 在树随源分发：
`src/rurix-basis-sys/vendor/basis_universal/LICENSE`、
`src/rurix-basis-sys/vendor/basis_universal/LICENSES/Apache-2.0.txt`、
`src/rurix-basis-sys/NOTICE`（版权归属 Binomial LLC，SBOM 与双 digest 见
`src/rurix-basis-sys/SBOM.md`）。SDK bundle 分发形态随附上述文件。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-29 | 初版（G37 W5 license_gaps，GAP-01 闭合件）：现发行面 3 二进制内嵌第三方闭包逐项登记（rowan 0.15.18 + 传递闭包 5 crate，上游源包 LICENSE 逐字随附）+ SDK bundle / 外部 SDK / 未来捆绑面义务同链登记 |
