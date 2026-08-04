//! G7.5b PR-3(RFC-0018 §E3 / 验收门 G-G7-7 编码腿;RXS-0301~0303 实现期锚定):
//! HW 光栅 VisBuffer 语料 `.rx` → Vulkan 原生 SPIR-V(`emit_spirv_body_vulkan`
//! 两遍编译——FS 经第二遍 ExtendedBodyLowerer〔RXS-0301 扩展白名单〕,VS 恒走
//! 第一遍最小切片〔字节零漂移路径〕)→ 机器断言:
//!   ① **DXIL 路必拒**(RXS-0171 L4 冻结锁:同一 FS body 经 `emit_spirv_body`
//!      〔provenance=true〕必 `DxilError`,RXS-0301 L1 target-conditional 分叉边界);
//!   ② SPIR-V 版本字恒 **1.0**(RXS-0302 L4);
//!   ③ capability 集合:FS == {Shader, Int64, Int64Atomics}(按需声明,不用不发),
//!      VS == {Shader}(RXS-0302 L3);
//!   ④ 资源布局(RXS-0302 L1 与 compute 同一分配律):`AtomicView` SSBO →
//!      `DescriptorSet 0` + `Binding 0`(Uniform+BufferBlock,SPIR-V 1.0 形态),
//!      标量 `width` → 单 push constant 块 member 0 `Offset 0`;
//!   ⑤ `spirv-val --target-env vulkan1.0` accept(工具在位;缺工具 SKIP 三态,
//!      dev-env degrade 非 fake pass,退出码判定非 grep)。

#![cfg(feature = "shader-stages")]

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::ast::ShaderStage;
use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// conformance/vulkan/accept(CARGO_MANIFEST_DIR = src/rurixc → repo root)。
fn accept_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vulkan/accept")
}

const FS_STEM: &str = "vk_hw_raster_visbuffer_fs";
const VS_STEM: &str = "vk_hw_raster_visbuffer_vs";

// SPIR-V core 枚举(本测试消费面;与编码器同值,spec 数值)。
const OP_CAPABILITY: u16 = 17;
const OP_VARIABLE: u16 = 59;
const OP_DECORATE: u16 = 71;
const OP_MEMBER_DECORATE: u16 = 72;
const CAP_SHADER: u32 = 1;
const CAP_INT64: u32 = 11;
const CAP_INT64_ATOMICS: u32 = 12;
const STORAGE_UNIFORM: u32 = 2;
const STORAGE_PUSH_CONSTANT: u32 = 9;
const DECO_BINDING: u32 = 33;
const DECO_DESCRIPTOR_SET: u32 = 34;
const DECO_OFFSET: u32 = 35;

fn read_corpus(stem: &str) -> String {
    let path = accept_dir().join(format!("{stem}.rx"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {stem}.rx 失败: {e}"))
        .replace("\r\n", "\n")
}

/// `.rx` → (前端 0 诊断门后的) 编译上下文,交给 `f` 消费(QueryCtx 借源串,
/// 无法跨函数返还,故闭包式)。
fn with_frontend<R>(stem: &str, f: impl FnOnce(&QueryCtx<'_>) -> R) -> R {
    let src = read_corpus(stem);
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    // 阶段化镜像 driver:着色阶段 AST 面先于 typeck(RXS-0153~0156)。
    cx.check_shader_stages();
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{stem} 应 0 前端诊断: {:?}",
        diag.emitted()
            .iter()
            .filter_map(|d| d.code)
            .collect::<Vec<_>>()
    );
    f(&cx)
}

/// `.rx` → Vulkan 原生 SPIR-V 字流(两遍编译入口;0 诊断门)。
fn emit_vulkan_words(stem: &str, stage: ShaderStage) -> Vec<u32> {
    with_frontend(stem, |cx| {
        let bodies = cx.device_mir_crate();
        let res = cx.resolutions();
        let body = bodies
            .iter()
            .find(|b| b.stage == Some(stage))
            .unwrap_or_else(|| panic!("{stem} 无 {stage:?} 图形阶段根"));
        rurixc::dxil_spirv::emit_spirv_body_vulkan(stage, body, &res)
            .unwrap_or_else(|e| panic!("{stem} emit_spirv_body_vulkan 失败: {e:?}"))
    })
}

/// 指令流扫描:全部 (opcode, operands)(跳 5 字 header)。
fn instructions(words: &[u32]) -> Vec<(u16, Vec<u32>)> {
    let mut out = Vec::new();
    let mut i = 5;
    while i < words.len() {
        let wc = (words[i] >> 16) as usize;
        if wc == 0 {
            break;
        }
        out.push(((words[i] & 0xffff) as u16, words[i + 1..i + wc].to_vec()));
        i += wc;
    }
    out
}

fn capabilities(words: &[u32]) -> Vec<u32> {
    let mut caps: Vec<u32> = instructions(words)
        .iter()
        .filter(|(op, _)| *op == OP_CAPABILITY)
        .map(|(_, ops)| ops[0])
        .collect();
    caps.sort_unstable();
    caps
}

/// ① DXIL 路(provenance=true)必拒:RXS-0171 L4 最小白名单冻结 0-byte 的机器锚
/// (RXS-0301 L1;同一 FS body,`emit_spirv_body` 恒 Err——扩展仅限
/// `emit_spirv_body_vulkan` provenance=false 路径)。
//@ spec: RXS-0301
#[test]
fn hw_raster_fs_dxil_target_still_rejected() {
    with_frontend(FS_STEM, |cx| {
        let bodies = cx.device_mir_crate();
        let body = bodies
            .iter()
            .find(|b| b.stage == Some(ShaderStage::Fragment))
            .expect("FS 语料须有 fragment 根");
        let r = rurixc::dxil_spirv::emit_spirv_body(ShaderStage::Fragment, body);
        assert!(
            r.is_err(),
            "DXIL 路(provenance=true)对 HW 光栅 FS 必须仍拒(RXS-0171 L4 冻结,\
             RXS-0301 L1 分叉边界),实得 Ok({} 字)",
            r.map(|w| w.len()).unwrap_or(0)
        );
    });
}

/// ②③ 版本字恒 1.0(RXS-0302 L4)+ capability 集合断言(RXS-0302 L3:
/// FS == {Shader, Int64, Int64Atomics},VS == {Shader})。
//@ spec: RXS-0302
#[test]
fn hw_raster_version_and_capability_sets() {
    let fs = emit_vulkan_words(FS_STEM, ShaderStage::Fragment);
    assert_eq!(fs[0], 0x0723_0203, "FS magic");
    assert_eq!(fs[1], 0x0001_0000, "FS SPIR-V 版本字恒 1.0(RXS-0302 L4)");
    assert_eq!(
        capabilities(&fs),
        vec![CAP_SHADER, CAP_INT64, CAP_INT64_ATOMICS],
        "FS capability 集合 == {{Shader, Int64, Int64Atomics}}"
    );

    let vs = emit_vulkan_words(VS_STEM, ShaderStage::Vertex);
    assert_eq!(vs[0], 0x0723_0203, "VS magic");
    assert_eq!(
        vs[1], 0x0001_0000,
        "VS SPIR-V 版本字恒 1.0(第一遍零漂移路径)"
    );
    assert_eq!(
        capabilities(&vs),
        vec![CAP_SHADER],
        "VS capability 集合 == {{Shader}}(不用不发)"
    );
}

/// ④ 资源布局断言(RXS-0302 L1 与 compute 同一分配律):
/// - 恰 1 个 Uniform 存储类变量(SSBO,SPIR-V 1.0 BufferBlock 形态)且装饰
///   `DescriptorSet 0` + `Binding 0`(buffer 形参声明序;set0-flat 与
///   render_exec `Bindings::storage_buffers` 字面对齐);
/// - 恰 1 个 PushConstant 存储类变量(标量 `width` 聚合块),块 member 0
///   `Offset 0`(4 字节对齐顺排首槽)。
//@ spec: RXS-0302
#[test]
fn hw_raster_fs_binding_and_push_constant_layout() {
    let fs = emit_vulkan_words(FS_STEM, ShaderStage::Fragment);
    let instrs = instructions(&fs);
    let vars_of = |storage: u32| -> Vec<u32> {
        instrs
            .iter()
            .filter(|(op, ops)| *op == OP_VARIABLE && ops.get(2) == Some(&storage))
            .map(|(_, ops)| ops[1])
            .collect()
    };
    let ssbo = vars_of(STORAGE_UNIFORM);
    assert_eq!(ssbo.len(), 1, "恰 1 个 SSBO 变量(AtomicView vis)");
    let deco = |target: u32, deco: u32| -> Option<u32> {
        instrs.iter().find_map(|(op, ops)| {
            (*op == OP_DECORATE && ops.first() == Some(&target) && ops.get(1) == Some(&deco))
                .then(|| ops.get(2).copied().unwrap_or(0))
        })
    };
    assert_eq!(
        deco(ssbo[0], DECO_DESCRIPTOR_SET),
        Some(0),
        "SSBO DescriptorSet 0"
    );
    assert_eq!(
        deco(ssbo[0], DECO_BINDING),
        Some(0),
        "SSBO Binding 0(声明序)"
    );

    let pc = vars_of(STORAGE_PUSH_CONSTANT);
    assert_eq!(pc.len(), 1, "恰 1 个 push constant 块变量(标量 width)");
    // push constant 块 struct = pc 变量指针的 pointee;member 0 Offset 0 断言经
    // OpMemberDecorate 全局唯一性简化:模块内全部 member-Offset 装饰均为 Offset 0
    // (SSBO 块 member 0 + PC 块 member 0 各一)。
    let member_offsets: Vec<(u32, u32)> = instrs
        .iter()
        .filter(|(op, ops)| *op == OP_MEMBER_DECORATE && ops.get(2) == Some(&DECO_OFFSET))
        .map(|(_, ops)| (ops[1], ops[3]))
        .collect();
    assert_eq!(
        member_offsets.len(),
        2,
        "member Offset 装饰恰 2 条(SSBO 块 + PC 块)"
    );
    assert!(
        member_offsets.iter().all(|&(m, off)| m == 0 && off == 0),
        "两块的 member 0 均 Offset 0(width 为 PC 首槽): {member_offsets:?}"
    );
}

/// ⑤ FS/VS `.spv` 过 `spirv-val --target-env vulkan1.0`(工具在位 accept /
/// 缺工具 SKIP 三态;退出码判定非 grep)。
//@ spec: RXS-0303
#[test]
fn hw_raster_modules_pass_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    for (stem, stage) in [
        (FS_STEM, ShaderStage::Fragment),
        (VS_STEM, ShaderStage::Vertex),
    ] {
        let words = emit_vulkan_words(stem, stage);
        let mut bytes = Vec::with_capacity(words.len() * 4);
        for w in &words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let spv =
            std::env::temp_dir().join(format!("rurix_g75b_{}_{stem}.spv", std::process::id()));
        if std::fs::write(&spv, &bytes).is_err() {
            eprintln!("[SKIP] 写临时 .spv 失败(dev-env degrade)");
            return;
        }
        let out = Command::new(&tool)
            .arg("--target-env")
            .arg("vulkan1.0")
            .arg(&spv)
            .output();
        let _ = std::fs::remove_file(&spv);
        match out {
            Err(_) => {
                eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                return;
            }
            Ok(o) if o.status.success() => {
                eprintln!("[OK] spirv-val --target-env vulkan1.0 accept: {stem}");
            }
            Ok(o) => panic!(
                "spirv-val 拒绝 {stem}: stdout={} stderr={}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
        }
    }
}

/// 确定性:同源 ×2 emit 字流全等(RXS-0301 Dynamic Semantics「两遍均确定性」;
/// 承 RXS-0200 恒跑判据)。
//@ spec: RXS-0301
#[test]
fn hw_raster_emit_is_deterministic() {
    for (stem, stage) in [
        (FS_STEM, ShaderStage::Fragment),
        (VS_STEM, ShaderStage::Vertex),
    ] {
        let a = emit_vulkan_words(stem, stage);
        let b = emit_vulkan_words(stem, stage);
        assert_eq!(a, b, "{stem} 同源 ×2 emit 字流不等(非确定性)");
    }
}
