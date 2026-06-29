//! DXIL golden guardrail(G2.2 PR-C2 分片1,RXS-0157;RFC-0003 §9 Q-Golden;cargo
//! feature `dxil-backend`)。两层 golden:
//! - **`.dxil-ll`**(always-on):rurixc 自有 DirectX 三元组 LLVM IR 文本产物
//!   (确定性、无外部工具依赖,对齐 ptx_golden 取 IR 层的纪律);
//! - **`.dxil-disasm`**(工具链关卡):经 patched llc `-filetype=obj` 产 DXIL 容器 +
//!   dxc validator **接受后**的文本反汇编(RFC-0003 §9 Q-Golden);patched llc
//!   (`RURIX_LLC`)/ dxc validator(`RURIX_DXC_DIR`)缺失 → SKIP(开发环境降级,真实
//!   红绿在带工具链环境,对齐 RXS-0073 ptxas 干验证 SKIP 纪律)。
//!
//! **bless 是审批动作**:`RURIX_BLESS=1` 重写 golden;变更须伴随 `tests/dxil/
//! bless_log.md` 追加记录(ci/check_guardrails.py 核对)。
#![cfg(feature = "dxil-backend")]

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn dxil_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dxil")
}

fn rx_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dxil_dir())
        .expect("读取 tests/dxil 失败")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rx"))
        .collect();
    out.sort();
    out
}

fn bless_mode() -> bool {
    std::env::var("RURIX_BLESS").is_ok_and(|v| v == "1")
}

/// 全管线产出 device DXIL DirectX 三元组 LLVM IR 文本(`kernel fn` 根;0 诊断断言)。
fn dxil_ir(src: &str, module: &str) -> String {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "DXIL golden 语料须 0 诊断");
    rurixc::dxil_codegen::build_and_emit_dxil(&cx, module).expect("应产出 DXIL IR")
}

/// `.dxil-ll` golden:rurixc 自有 DirectX 三元组 LLVM IR(确定性,always-on)。
#[test]
fn dxil_ll_golden_matches() {
    let bless = bless_mode();
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let ir = dxil_ir(&src, &stem);
        let golden = path.with_extension("dxil-ll");
        if bless {
            fs::write(&golden, &ir).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == ir => {}
            Ok(s) => mismatches.push(format!(
                "{}: DXIL IR golden 漂移\n--- expected ---\n{}\n--- actual ---\n{ir}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-ll golden(新语料需 RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    assert!(
        mismatches.is_empty(),
        "DXIL IR golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}

/// `.dxil-disasm` golden:patched llc → DXIL 容器 → dxc validator **接受** → dxc
/// 反汇编(RFC-0003 §9 Q-Golden)。工具链缺失 → SKIP。
#[test]
fn dxil_disasm_golden_matches_when_toolchain_present() {
    let (Some(llc), Some(dxc_dir)) = (
        rurixc::toolchain::locate_llc(),
        rurixc::toolchain::locate_dxc_dir(),
    ) else {
        eprintln!("dxil_disasm_golden: patched llc / dxc validator 不可用 → SKIP(RXS-0157)");
        return;
    };
    let bless = bless_mode();
    let tmp = std::env::temp_dir().join(format!("rxdxilgold_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("临时目录");
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let ir = dxil_ir(&src, &stem);
        let obj = tmp.join(format!("{stem}.dxc"));
        rurixc::toolchain::llc_emit_dxil(&llc, &ir, &obj).expect("patched llc emit DXIL 失败");
        // strict-only:入 golden 前 validator 必须接受(不合规 DXIL 不得入 golden)。
        assert!(
            rurixc::toolchain::dxv_validate(&dxc_dir, &obj).expect("dxv 调用失败"),
            "{}: DXIL 容器未通过 dxc validator(不得入 golden)",
            stem
        );
        let disasm = rurixc::toolchain::dxc_disasm(&dxc_dir, &obj).expect("dxc 反汇编失败");
        let golden = path.with_extension("dxil-disasm");
        if bless {
            fs::write(&golden, &disasm).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == disasm => {}
            Ok(s) => mismatches.push(format!(
                "{}: DXIL 反汇编 golden 漂移\n--- expected ---\n{}\n--- actual ---\n{disasm}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-disasm golden(RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        mismatches.is_empty(),
        "DXIL 反汇编 golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn dxil_corpus_carries_spec_anchor() {
    for path in rx_files() {
        let src = fs::read_to_string(&path).expect("读取语料失败");
        assert!(
            src.lines()
                .next()
                .unwrap_or("")
                .starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头",
            path.display()
        );
    }
}

// ═══════════════════ 图形=B DXIL golden(G2.2 PR-D2,RXS-0162) ═══════════════════
//
// B 路 golden 置于 `tests/dxil/graphics/`(子目录;A 路 `rx_files()` 用 `read_dir`
// **非递归**,自然不收;本组用独立 lister)。形态:DXIL 文本反汇编(`.dxil-disasm`),
// 经 B 全链(dxil_spirv::emit_spirv→SPIRV-Cross→dxc→dumpbin)产出。validator gate:
// 若签名 validator 目录(`RURIX_DXC_DIR` 含 dxv.exe)可用则入 golden 前 dxv 验证;owner
// pin 环境带签名 validator bless 后,本 golden 锁当前已登记 RD-013/RD-017 缺口下的
// B 路文本形态。版本噪声行(shader hash / dxc ident)规范化,使
// golden 不写死工具版本布局为语言保证(硬约束;RXS-0162 IR5)。spirv-cross/dxc 缺失 →
// SKIP(开发环境降级,exit 0,对齐 RXS-0073)。`RURIX_BLESS=1` 重写 + bless_log 留痕。

#[cfg(feature = "shader-stages")]
fn graphics_rx_files() -> Vec<PathBuf> {
    let dir = dxil_dir().join("graphics");
    let mut out: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "rx"))
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out
}

/// 规范化 dxc 反汇编中的版本噪声行已下沉为 `dxil_codegen::emit_dxil_b_disasm` 的内部
/// 步骤(golden 单一真相源,RXS-0162/0171/0172);本测试不再手搓链/手搓规范化。
///
/// 从图形 golden 语料源码取首个 vertex/fragment 阶段根的完整 MIR `Body`(供生产忠实
/// 链 `emit_dxil_b_disasm` 消费——body 降级 + varying 保名都依赖完整 Body)。
#[cfg(feature = "shader-stages")]
fn graphics_stage_body(src: &str) -> Option<rurixc::mir::Body> {
    use rurixc::ast::ShaderStage;
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "DXIL B golden 语料须 0 诊断");
    let bodies = cx.device_mir_crate();
    bodies
        .into_iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
}

/// `.dxil-disasm` golden(B 路):驱动**生产忠实**链 `dxil_codegen::emit_dxil_b_disasm`
/// (emit_spirv_body 体降级 + RXS-0172 varying 保名 + 强制 signature_gate)→ golden
/// 比对。golden 字节 = 校验门所验产物本身。工具缺失 → SKIP。
#[cfg(feature = "shader-stages")]
#[test]
fn dxil_b_disasm_golden_matches_when_toolchain_present() {
    // 版本相关 golden:仅在**显式配置**的 pin 工具(env 指向真实文件)下
    // 跑字节比对——`locate_*` 的 PATH by-name 回落(spawn 决定)不触发,避免随机 PATH
    // 工具产不同反汇编致误红。env 未设 → SKIP(对齐 A 路 .dxil-disasm 经 RURIX_DXC_DIR
    // 显式门控的纪律;真实红绿在带 pin B 工具链的 dev/owner 环境)。
    let (Some(_spvx), Some(_dxc)) = (
        rurixc::toolchain::locate_spirv_cross().filter(|p| p.is_file()),
        rurixc::toolchain::locate_dxc().filter(|p| p.is_file()),
    ) else {
        eprintln!(
            "dxil_b_disasm_golden: 未显式配置 pin B 工具链(RURIX_SPIRV_CROSS / RURIX_DXC \
             指向真实文件)→ SKIP(开发环境降级,RXS-0162;真实红绿在带 pin B 工具链环境)"
        );
        return;
    };
    let bless = bless_mode();
    let header = concat!(
        "; OWNER BLESSED — RXS-0162 图形=B DXIL 反汇编 golden。\n",
        "; owner pin 环境签名 validator(dxv.exe/dxil.dll)已验证;本文件为当前 B 路文本形态基线。\n",
        "; 版本噪声行(shader hash / dxc ident)已规范化为占位,不写死工具版本布局为语言保证。\n",
        "; 生产忠实 B 链:RXS-0171 入口 body I/O 数据流降级 + RXS-0172 输出/片元 varying 用户语义名保名(uv/normal 等保真,非 TEXCOORD 退化)。\n",
    );
    let mut mismatches = Vec::new();
    for path in graphics_rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let Some(body) = graphics_stage_body(&src) else {
            mismatches.push(format!(
                "{}: 未收到 vertex/fragment 图形阶段根",
                path.display()
            ));
            continue;
        };
        // 生产忠实链(emit_spirv_body 体降级 + 顶点输入保名 + RXS-0172 输出/片元 varying
        // 保名 + 强制 signature_gate)产规范化反汇编;golden 字节 = 校验门所验产物本身,
        // 不再手搓「签名-only emit_spirv + 空旗标 + 跳过保名」第二条链。
        let produced = match rurixc::dxil_codegen::emit_dxil_b_disasm(&body) {
            Ok(Some(d)) => format!("{header}{d}\n"),
            Ok(None) => {
                // 预检已确认 pin 工具在位;此处 None = 链中途环境降级(spawn 等)→ SKIP 该语料。
                eprintln!("{stem}: 生产 B 链工具链中途不可用 → SKIP");
                continue;
            }
            Err(e) => panic!("{stem}: 生产 B 链失败(编码器/校验门 strict-only): {e:?}"),
        };
        let golden = path.with_extension("dxil-disasm");
        if bless {
            fs::write(&golden, &produced).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == produced => {}
            Ok(s) => mismatches.push(format!(
                "{}: B DXIL 反汇编 golden 漂移\n--- expected ---\n{}\n--- actual ---\n{produced}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-disasm golden(新语料需 RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    assert!(
        mismatches.is_empty(),
        "B DXIL 反汇编 golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}

// ═══════════════ 绑定布局推导产物 golden(G2.3 PR-E2b-3,RXS-0165/0166) ═══════════════
//
// E2b-1 已贯通生产 emit(资源装饰 + RTS0),E2b-2 落真实 6xxx 错误码;本组 golden 把
// 覆盖面从 io-only 扩到**绑定布局推导产物**:着色阶段资源句柄(`Texture2D<F>`/`Sampler`,
// 生产可达子集——`emit_spirv` 先拒其余类型)→ ① SPIR-V 资源绑定装饰
// (`DescriptorSet`/`Binding` + opaque 资源类型)② host 侧 `infer_root_signature` →
// `serialize_rts0` 的 RTS0 容器字节。golden 固化二者的**确定性 SHA-256 digest** + 可读
// 形态摘要(set/binding 列表 + root parameter 形态),作 device 真跑(E2b-4)前的
// CI 可验证回归锚。
//
// 🔒 RFC-0005 §4.5 / RXS-0162 先例:digest 仅作**确定性回归锚**,**非** stable 语言/ABI
// 保证。具体 register/space/binding 物理布局与 RTS0 字节布局**不**冻结为 stable;golden
// 只锁当前实现确定、gate 后产物的回归基线,真链 validator / device 核验归 E2b-4。
//
// always-on(纯 host,无外部工具依赖,对齐 `.dxil-ll` 纪律);`RURIX_BLESS=1` 重写 golden
// + `tests/dxil/bless_log.md` 追加 owner 审批行(bless 是 owner 门,非 AI 代签)。

/// 自含 SHA-256(FIPS 180-4;`rurixc` 零外部依赖,对齐仓内 `rurix-pkg` 手写 SHA-256
/// 纪律)。仅 golden digest 锚用;调用点先以已知答案向量(KAT)自检守护实现本身。
#[cfg(feature = "shader-stages")]
mod gold_sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// 十六进制 SHA-256 摘要(确定性;小写 64 字符)。
    pub fn hex(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let bitlen = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bitlen.to_be_bytes());
        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (i, wi) in w.iter_mut().enumerate().take(16) {
                let o = i * 4;
                *wi = u32::from_be_bytes([chunk[o], chunk[o + 1], chunk[o + 2], chunk[o + 3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        let mut out = String::with_capacity(64);
        for v in h {
            out.push_str(&format!("{v:08x}"));
        }
        out
    }
}

/// 从 SPIR-V 字流提取资源绑定装饰((descriptor_sets, bindings) 按出现序)与
/// opaque 资源类型存在性(image/sampler),供 golden 可读摘要 + 篡改可见 diff。
#[cfg(feature = "shader-stages")]
fn spirv_binding_summary(spv: &[u32]) -> (Vec<u32>, Vec<u32>, bool, bool) {
    const OP_DECORATE: u16 = 71;
    const OP_TYPE_IMAGE: u16 = 25;
    const OP_TYPE_SAMPLER: u16 = 26;
    const DECORATION_BINDING: u32 = 33;
    const DECORATION_DESCRIPTOR_SET: u32 = 34;
    let (mut sets, mut bindings, mut has_image, mut has_sampler) =
        (Vec::new(), Vec::new(), false, false);
    let mut i = 5; // 跳过 5 字 header
    while i < spv.len() {
        let word = spv[i];
        let wc = (word >> 16) as usize;
        let op = (word & 0xFFFF) as u16;
        if wc == 0 || i + wc > spv.len() {
            break;
        }
        let ops = &spv[i + 1..i + wc];
        match op {
            OP_DECORATE if ops.get(1) == Some(&DECORATION_DESCRIPTOR_SET) => sets.push(ops[2]),
            OP_DECORATE if ops.get(1) == Some(&DECORATION_BINDING) => bindings.push(ops[2]),
            OP_TYPE_IMAGE => has_image = true,
            OP_TYPE_SAMPLER => has_sampler = true,
            _ => {}
        }
        i += wc;
    }
    (sets, bindings, has_image, has_sampler)
}

/// 把推导出的 root signature 形态渲染为确定性可读摘要(篡改 root 形态即现文本 diff)。
#[cfg(feature = "shader-stages")]
fn root_signature_summary(rs: &rurixc::binding_layout::RootSignature) -> String {
    use rurixc::binding_layout::RootParameter;
    use rurixc::mir::ResourceClass;
    fn axis(c: ResourceClass) -> char {
        match c {
            ResourceClass::Cbv => 'b',
            ResourceClass::Srv => 't',
            ResourceClass::Uav => 'u',
            ResourceClass::Sampler => 's',
        }
    }
    let mut parts = Vec::new();
    for p in &rs.parameters {
        match p {
            RootParameter::CbvRootDescriptor { register, space } => {
                parts.push(format!("CbvRootDescriptor(b{register} space{space})"));
            }
            RootParameter::DescriptorTable { ranges } => {
                let rs: Vec<String> = ranges
                    .iter()
                    .map(|r| {
                        format!(
                            "{}{}..+{} space{}",
                            axis(r.range_type),
                            r.base_register,
                            r.num_descriptors,
                            r.space
                        )
                    })
                    .collect();
                parts.push(format!("DescriptorTable[{}]", rs.join(", ")));
            }
        }
    }
    format!("[{}] flags=0x{:08x}", parts.join(", "), rs.flags)
}

/// 绑定布局推导产物 golden 锚(`.binding-golden`):经真实前端贯通生产路径
/// (fragment 资源句柄签名 → device MIR → `Body.resources`)→ ① `emit_spirv` 资源
/// 绑定装饰 ② `infer_root_signature` → `serialize_rts0` RTS0 容器,固化二者
/// 确定性 SHA-256 + 可读形态摘要。篡改推导(set/binding 或 root signature 形态)→
/// golden 红;复原 → 绿(真红绿,非 YAML-only)。`RURIX_BLESS=1` 重写 + owner bless 留痕。
//@ spec: RXS-0165, RXS-0166
#[cfg(feature = "shader-stages")]
#[test]
fn binding_layout_digest_golden_matches() {
    use rurixc::ast::ShaderStage;

    // 0) 守护 digest 实现本身:SHA-256 已知答案向量(FIPS 180-4)。
    assert_eq!(
        gold_sha256::hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "SHA-256 KAT 失败:digest 实现不可信,拒绝据其落 golden"
    );
    assert_eq!(
        gold_sha256::hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "SHA-256 空串 KAT 失败"
    );

    // 1) 真实前端贯通:fragment(Texture2D<f32> tex + Sampler samp)→ device MIR。
    //    与 mir_build::e2b1 同源,锚定生产可达资源子集(emit_spirv 先拒其余类型)。
    let src = "struct FsOut {\n\
        \x20   color: f32,\n\
         }\n\
         fragment fn fs_main(tex: Texture2D<f32>, samp: Sampler) -> FsOut {\n\
        \x20   FsOut { color: 0.0 }\n\
         }\n\
         fn main() {}";
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "绑定 golden 语料须 0 诊断");
    let device = cx.device_mir_crate();
    let fs = device
        .iter()
        .find(|b| b.stage == Some(ShaderStage::Fragment))
        .expect("fragment 阶段根应进入 device MIR");

    // 2) ① SPIR-V 资源绑定装饰(生产 emit)。
    let spv = rurixc::dxil_spirv::emit_spirv(ShaderStage::Fragment, &fs.io_sig, &fs.resources)
        .expect("带资源的 fragment emit 应 Ok");
    // 确定性:同输入二次 emit 字节全等(golden digest 的前置条件)。
    assert_eq!(
        spv,
        rurixc::dxil_spirv::emit_spirv(ShaderStage::Fragment, &fs.io_sig, &fs.resources).unwrap(),
        "emit_spirv 非确定性(同输入字节漂移)"
    );
    let (sets, bindings, has_image, has_sampler) = spirv_binding_summary(&spv);
    assert!(has_image && has_sampler, "应含 OpTypeImage + OpTypeSampler");
    let mut spv_bytes = Vec::with_capacity(spv.len() * 4);
    for w in &spv {
        spv_bytes.extend_from_slice(&w.to_le_bytes());
    }

    // 3) ② root signature 形态推导 + RTS0 容器序列化(host 侧,工具链无关)。
    let rs = rurixc::binding_layout::infer_root_signature(&fs.resources)
        .expect("Texture2D/Sampler 应可推导 root signature");
    let rts0 = rurixc::binding_layout::serialize_rts0(&rs);
    assert_eq!(&rts0[0..4], b"DXBC", "RTS0 外层 DXBC 容器 fourcc");
    assert!(
        rts0.windows(4).any(|w| w == b"RTS0"),
        "容器应含 RTS0 part fourcc"
    );
    assert_eq!(
        rts0,
        rurixc::binding_layout::serialize_rts0(&rs),
        "RTS0 序列化非确定性"
    );

    // 4) 渲染确定性 golden 文本(可读摘要 + digest 锚)。
    let res_summary: Vec<String> = fs
        .resources
        .iter()
        .map(|r| format!("{}={:?}({:?})", r.name, r.res, r.res.class()))
        .collect();
    let header = concat!(
        "; OWNER-BLESS-REQUIRED — RXS-0165/0166 绑定布局推导产物确定性 golden(G2.3 PR-E2b-3)。\n",
        "; 锚定:fragment(Texture2D<f32> tex + Sampler samp,生产可达资源子集)→\n",
        ";   ① emit_spirv 资源绑定装饰(DescriptorSet/Binding + opaque 资源类型)\n",
        ";   ② infer_root_signature → serialize_rts0(RTS0 DXBC 容器)。\n",
        "; 🔒 digest 为确定性回归锚,非 stable 语言/ABI 保证(RFC-0005 §4.5;RXS-0162 先例):\n",
        ";   register/space/binding 物理布局与 RTS0 字节布局不冻结为 stable;golden 仅锁当前\n",
        ";   实现确定、gate 后产物回归基线,真链 validator / device 核验归 E2b-4。\n",
    );
    let produced = format!(
        "{header}\
         resources: {}\n\
         spirv.descriptor_sets: {:?}\n\
         spirv.bindings: {:?}\n\
         spirv.bytes.len: {}\n\
         spirv.bytes.sha256: {}\n\
         root_signature.params: {}\n\
         rts0.bytes.len: {}\n\
         rts0.bytes.sha256: {}\n",
        res_summary.join(" "),
        sets,
        bindings,
        spv_bytes.len(),
        gold_sha256::hex(&spv_bytes),
        root_signature_summary(&rs),
        rts0.len(),
        gold_sha256::hex(&rts0),
    );

    // 5) bless 重写 / 比对(缺 golden → 提示 bless;漂移 → 红)。
    let golden = dxil_dir()
        .join("binding")
        .join("fs_tex_samp.binding-golden");
    if bless_mode() {
        fs::create_dir_all(golden.parent().unwrap()).expect("建 binding golden 目录");
        fs::write(&golden, &produced).expect("bless 写入失败");
        return;
    }
    match fs::read_to_string(&golden) {
        Ok(s) if s.replace("\r\n", "\n") == produced => {}
        Ok(s) => panic!(
            "{}: 绑定布局推导产物 golden 漂移\n--- expected ---\n{}\n--- actual ---\n{produced}",
            golden.display(),
            s.replace("\r\n", "\n")
        ),
        Err(_) => panic!(
            "{}: 缺 .binding-golden(新 golden 需 RURIX_BLESS=1 + bless_log.md owner 留痕)",
            golden.display()
        ),
    }
}
