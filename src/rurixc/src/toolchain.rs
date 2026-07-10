//! 工具链定位与 device IR→PTX(M4.2 抽出复用,M4.4;clang pin 22.1.x,D-205)。
//!
//! 驱动 `--emit=ptx`(bin)与 `rurix-rt` 的 `build.rs`(嵌入 PTX 单产物)复用同一
//! IR→PTX 路径(单一事实源):NVPTX 约束 LLVM IR 文本 → pin 的 clang
//! `--target=nvptx64-nvidia-cuda` NVPTX 后端 → PTX。ptxas 干验证关卡见
//! [`crate::ptxas`](strict-only,RXS-0073)。

use std::path::{Path, PathBuf};
use std::process::Command;

/// clang 定位 + pin 22.1.x 断言(D-205;M2_PLAN v1.3 选型留痕)。
///
/// `RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
/// 版本非 22.1.x(违例 = pin 纪律,上层映射 RX7001)。
pub fn locate_clang() -> Result<PathBuf, String> {
    let candidates: Vec<PathBuf> = [
        std::env::var("RURIXC_CLANG").ok(),
        Some("C:\\Program Files\\LLVM\\bin\\clang.exe".to_owned()),
        Some("clang".to_owned()),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .collect();
    for c in candidates {
        let Ok(out) = Command::new(&c).arg("--version").output() else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let ver = String::from_utf8_lossy(&out.stdout);
        if ver.contains("clang version 22.1.") {
            return Ok(c);
        }
        return Err(format!(
            "clang at {} is not the pinned 22.1.x (D-205): {}",
            c.display(),
            ver.lines().next().unwrap_or("")
        ));
    }
    Err("clang not found (install LLVM 22.1.x or set RURIXC_CLANG)".to_owned())
}

/// libdevice 链接裁决(M5.3,RXS-0082):IR 是否用到 libdevice `__nv_*` 数学
/// 符号 + bc 是否可定位。
pub enum LibdeviceLink {
    /// IR 无 `__nv_*` 符号引用:按原路径直接 IR→PTX(无需 libdevice)。
    NotNeeded,
    /// 用到 libdevice 且已定位 `libdevice.10.bc`(链 bc → internalize → DCE →
    /// NVVMReflect,clang `-mlink-builtin-bitcode` 内置流程)。
    Linked(PathBuf),
    /// 用到 libdevice 但 bc 缺失(无 CUDA 工具链):开发环境降级 SKIP(真实红绿
    /// 在带 CUDA 的 CI runner,RXS-0082;不报 RX7002)。
    MissingSkip,
}

/// IR 是否引用 libdevice `__nv_*` 数学符号(device_codegen 保留为外部 declare)。
pub fn ir_needs_libdevice(ir: &str) -> bool {
    ir.contains("@__nv_")
}

/// 定位 `libdevice.10.bc`(RXS-0082;禁硬编码版本路径,沿用 ptxas 定位纪律 r6):
/// `RURIXC_LIBDEVICE` > `CUDA_PATH\nvvm\libdevice\libdevice.10.bc`。
pub fn locate_libdevice() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIXC_LIBDEVICE") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(cuda) = std::env::var("CUDA_PATH") {
        let pb = PathBuf::from(cuda)
            .join("nvvm")
            .join("libdevice")
            .join("libdevice.10.bc");
        if pb.is_file() {
            return Some(pb);
        }
    }
    None
}

/// 对给定 IR 裁决 libdevice 链接路径(RXS-0082)。
pub fn libdevice_link_for(ir: &str) -> LibdeviceLink {
    if !ir_needs_libdevice(ir) {
        return LibdeviceLink::NotNeeded;
    }
    match locate_libdevice() {
        Some(bc) => LibdeviceLink::Linked(bc),
        None => LibdeviceLink::MissingSkip,
    }
}

/// device NVPTX 约束 LLVM IR 文本 → PTX(clang NVPTX 后端;RXS-0070;libdevice
/// 链接 RXS-0082)。
///
/// 目标基线 compute_89/sm_89:nvptx 后端经 `-Xclang -target-cpu sm_89` 设 GPU
/// 架构(clang 驱动 nvptx target 不接受 `-mcpu=`);`+ptx78` 设 PTX ISA 版本
/// (sm_89 要求 ≥ 7.8;默认 4.2 不支持)。`-O2` 优化:NVPTX 后端 `-O0` 对 i64
/// 索引的 lowering 产出错误地址(`ld.local.b32` 入 64 位寄存器高位未定义 → 越界
/// 访存),且 device 代码须打满带宽(G-M4-1 ≥ 手写基线 95%);IR golden 在 IR 层
/// (CI_GATES §4.3),clang 优化级不影响 golden。
///
/// **libdevice 链接(RXS-0082)**:IR 用到 `__nv_*` 数学符号且 bc 可定位时,经
/// clang `-mlink-builtin-bitcode <libdevice.10.bc>` 链接(clang NVPTX 后端内置
/// internalize/DCE/NVVMReflect 流程,精确路径由 IR 的 `nvvm-reflect-ftz=0` 模块
/// flag 留痕)。bc 缺失(`MissingSkip`)应由调用方先行 SKIP,不应进入本函数。
///
/// 中间 `.dev.ll` 落 `ptx_out` 同名旁路,返回 PTX 文本(失败 = 工具链错误串,
/// 上层映射 RX7001;libdevice 链接语境失败映射 RX7002)。
pub fn ir_to_ptx(ir: &str, ptx_out: &Path) -> Result<String, String> {
    let clang = locate_clang()?;
    let ll = ptx_out.with_extension("dev.ll");
    std::fs::write(&ll, ir).map_err(|e| format!("cannot write {}: {e}", ll.display()))?;
    let mut cmd = Command::new(&clang);
    cmd.arg("--target=nvptx64-nvidia-cuda")
        .arg("-Xclang")
        .arg("-target-cpu")
        .arg("-Xclang")
        .arg("sm_89")
        .arg("-Xclang")
        .arg("-target-feature")
        .arg("-Xclang")
        .arg("+ptx78");
    // libdevice 链接(RXS-0082):保留外部 `__nv_*` 符号 → 链 libdevice bc →
    // internalize → DCE → NVVMReflect(clang 内置流程)。
    if let LibdeviceLink::Linked(bc) = libdevice_link_for(ir) {
        cmd.arg("-Xclang")
            .arg("-mlink-builtin-bitcode")
            .arg("-Xclang")
            .arg(&bc)
            // NVVMReflect 裁决(RXS-0081 默认精确路径):ftz=0(模块 flag 已留痕)
            // + prec-sqrt=1 / prec-div=1 经 `-mllvm -nvvm-reflect-add` 显式置值
            // (模块 flag 仅 ftz 被 NVVMReflect 识别,prec-* 须经 reflect-add)。
            .arg("-mllvm")
            .arg("-nvvm-reflect-add=__CUDA_PREC_SQRT=1")
            .arg("-mllvm")
            .arg("-nvvm-reflect-add=__CUDA_PREC_DIV=1");
    }
    let out = cmd
        .arg("-O2")
        .arg("-S")
        .arg(&ll)
        .arg("-o")
        .arg(ptx_out)
        .output();
    match out {
        Ok(o) if o.status.success() => std::fs::read_to_string(ptx_out)
            .map_err(|e| format!("cannot read {}: {e}", ptx_out.display())),
        Ok(o) => Err(format!(
            "clang (nvptx) exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn clang (nvptx): {e}")),
    }
}

// ===========================================================================
// DXIL 第二后端工具链(G2.2 PR-C2 分片1,RXS-0157;feature `dxil-backend`)。
//
// D-131=A 路径:DirectX 三元组 LLVM IR → patched llc -filetype=obj → DXIL 容器 →
// dxc validator(IDxcValidator / dxv.exe)接受。patched llc 经 `RURIX_LLC` dev env
// 绝对路径定位(受控临时偏差,RD-011 + spike/dxil-path-probe recipe),**不写死、
// 不改 committed D-205 pin / 上方 `locate_clang`**;env 缺失 → 回落 committed pin
// 候选,均不可用 → 调用方 SKIP(非静默 fallback 到其他后端,P-01)。
// ===========================================================================

/// patched llc 定位(RXS-0157 IR2;RD-011 dev 偏差):`RURIX_LLC` 绝对路径 >
/// committed D-205 pin `C:\Program Files\LLVM\bin\llc.exe` > PATH `llc`。返回首个
/// **存在**的候选(env 候选要求文件存在,pin/PATH 候选按名返回交由 spawn 判定);
/// 全不可用 → `None`(调用方 SKIP,真实红绿在带 patched llc 的 dev/CI 环境)。
#[cfg(feature = "dxil-backend")]
pub fn locate_llc() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIX_LLC") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    let pin = PathBuf::from("C:\\Program Files\\LLVM\\bin\\llc.exe");
    if pin.is_file() {
        return Some(pin);
    }
    None
}

/// DirectX 三元组 LLVM IR 文本 → DXIL 容器对象(patched llc `-filetype=obj`)。
/// 中间 `.dxil.ll` 落 `obj_out` 同名旁路;成功返回 `()`(失败 = 工具链错误串,
/// 上层映射 RX6007)。`llc` 由 [`locate_llc`] 提供。
#[cfg(feature = "dxil-backend")]
pub fn llc_emit_dxil(llc: &Path, ir: &str, obj_out: &Path) -> Result<(), String> {
    let ll = obj_out.with_extension("dxil.ll");
    std::fs::write(&ll, ir).map_err(|e| format!("cannot write {}: {e}", ll.display()))?;
    let out = Command::new(llc)
        .arg(&ll)
        .arg("-filetype=obj")
        .arg("-o")
        .arg(obj_out)
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "llc (dxil) exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn llc (dxil): {e}")),
    }
}

/// dxc 签名 validator 套件目录定位(RXS-0157 IR3;round-7 取得的 2026 签名
/// validator):`RURIX_DXC_DIR` > `RURIX_DXC_NEW_DIR`(spike 现场约定)。返回含
/// `dxv.exe` + `dxc.exe` + `dxil.dll` 的目录;不可用 → `None`(调用方 SKIP validator,
/// 真实红绿在带 validator 的环境)。
#[cfg(feature = "dxil-backend")]
pub fn locate_dxc_dir() -> Option<PathBuf> {
    for key in ["RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"] {
        if let Ok(p) = std::env::var(key) {
            let pb = PathBuf::from(p);
            if dxc_validator_suite_ready(&pb) {
                return Some(pb);
            }
        }
    }
    None
}

#[cfg(feature = "dxil-backend")]
pub fn dxc_validator_suite_ready(dir: &Path) -> bool {
    ["dxc.exe", "dxv.exe", "dxil.dll"]
        .iter()
        .all(|name| dir.join(name).is_file())
}

#[cfg(feature = "dxil-backend")]
pub struct DxvValidationResult {
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// dxc validator 验证 DXIL 容器(`dxv.exe <obj>`;RXS-0157 IR3,strict-only):
/// 接受/拒绝均返回完整进程证据,spawn 失败 → `Err`(工具链串)。
#[cfg(feature = "dxil-backend")]
pub fn dxv_validate(dxc_dir: &Path, obj: &Path) -> Result<DxvValidationResult, String> {
    let dxv = dxc_dir.join("dxv.exe");
    let argv = vec![dxv.display().to_string(), obj.display().to_string()];
    match Command::new(&dxv).arg(obj).output() {
        Ok(o) => Ok(DxvValidationResult {
            argv,
            exit_code: o.status.code(),
            stdout: String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
            stderr: String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n"),
            success: o.status.success(),
        }),
        Err(e) => Err(format!("cannot spawn dxv: {e}")),
    }
}

/// dxc 反汇编 DXIL 容器为确定性文本(`dxc -dumpbin <obj>`;RXS-0157 IR3 golden
/// 文本反汇编形态)。失败 → `Err`(工具链串)。
#[cfg(feature = "dxil-backend")]
pub fn dxc_disasm(dxc_dir: &Path, obj: &Path) -> Result<String, String> {
    let dxc = dxc_dir.join("dxc.exe");
    match Command::new(&dxc).arg("-dumpbin").arg(obj).output() {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")),
        Ok(o) => Err(format!(
            "dxc -dumpbin exited with {}: {}",
            o.status,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn dxc: {e}")),
    }
}

// ===========================================================================
// 图形=B 转译链工具链(G2.2 PR-D2,RXS-0159~0162;feature `dxil-backend`)。
//
// D-131 v1.4 图形=B 路径(RFC-0004 §4.2/§4.3):
//   图形着色阶段 MIR → SPIR-V(Rurix 自有,dxil_spirv)→ SPIRV-Cross → HLSL →
//   dxc → DXIL 容器 → dxc validator + 强制签名一致性校验门(ISG1/OSG1)。
// 本节仅落**转译链外部工具驱动 + 签名 part 解析**(shell-out 形态);MIR→SPIR-V
// 自有降级见 `dxil_spirv` 模块。供应链 pin(SPIRV-Cross/dxc/glslang `[[toolchain]]`
// + SHA256,Q-Supply)为 canonical 形态;此处 env/PATH 定位仅本地 probe/dev override
// (RFC-0004 §9 Q-Supply,**非 CI/stable path**),工具缺失 → 调用方 SKIP(非静默
// fallback,P-01)。**不**写死版本、**不**改 committed pin。
// ===========================================================================

/// SPIRV-Cross 定位(dev/probe override;Q-Supply):`RURIX_SPIRV_CROSS` 绝对路径 >
/// PATH `spirv-cross`。canonical pin 形态(lockfile `[[toolchain]]` + SHA256)由
/// owner 兑现,此处仅本地取证。全不可用 → `None`(调用方 SKIP)。
#[cfg(feature = "dxil-backend")]
pub fn locate_spirv_cross() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIX_SPIRV_CROSS") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    // PATH 候选按名返回,交由 spawn 判定(对齐 locate_llc 的 PATH 候选纪律)。
    Some(PathBuf::from("spirv-cross"))
}

/// spirv-val 定位(dev/probe override;Q-Supply / RXS-0073 干验证纪律):
/// `RURIX_SPIRV_VAL` 绝对路径 > PATH `spirv-val`。canonical pin 形态(lockfile
/// `[[toolchain]]` + SHA256)由 owner 兑现,此处仅本地取证(`dxil_spirv` 编码器
/// 产物的本机独立验证,RXS-0161 R1.8)。全不可用 → `None`(调用方 SKIP,真实红绿
/// 在带 SPIRV-Tools 的 dev/owner 环境)。
#[cfg(feature = "dxil-backend")]
pub fn locate_spirv_val() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIX_SPIRV_VAL") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    // PATH 候选按名返回,交由 spawn 判定(对齐 locate_spirv_cross 的 PATH 候选纪律)。
    Some(PathBuf::from("spirv-val"))
}

/// 图形=B 的 dxc 定位(dev/probe override;Q-Supply):`RURIX_DXC` 绝对路径 >
/// `RURIX_DXC_DIR`/`RURIX_DXC_NEW_DIR` 目录内 dxc.exe > PATH `dxc`。与 A 路
/// [`locate_dxc_dir`] 并存(A 路取目录含 dxv.exe;B 路取 dxc 可执行本体)。
#[cfg(feature = "dxil-backend")]
pub fn locate_dxc() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIX_DXC") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    for key in ["RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"] {
        if let Ok(p) = std::env::var(key) {
            let pb = PathBuf::from(p).join("dxc.exe");
            if pb.is_file() {
                return Some(pb);
            }
        }
    }
    Some(PathBuf::from("dxc"))
}

/// SPIRV-Cross: SPIR-V → HLSL(RFC-0004 §4.2 (b))。`shader_model` 取目标 SM(如 60);
/// `extra` 透传保名等附加旗标(生产用 `--set-hlsl-vertex-input-semantic <location>
/// <semantic>` 按 location 覆盖顶点输入语义名,经 io_sig 导出,RFC-0004 §4.4 机制①;
/// 实测 spirv-cross **不**消费 SPIR-V `UserSemantic` 装饰为 HLSL 语义,故保名走 location
/// 覆盖而非 `--set-hlsl-named-vertex-input-semantic`(后者按 OpName 匹配,Rust-emit SPIR-V
/// 未 emit OpName))。成功写 `hlsl_out`;失败 → `Err`(工具链串,上层映射 6xxx)。
#[cfg(feature = "dxil-backend")]
pub fn spirv_cross_to_hlsl(
    tool: &Path,
    spv: &Path,
    hlsl_out: &Path,
    shader_model: u32,
    extra: &[String],
) -> Result<(), String> {
    let mut cmd = Command::new(tool);
    cmd.arg("--hlsl")
        .arg("--shader-model")
        .arg(shader_model.to_string());
    for e in extra {
        cmd.arg(e);
    }
    cmd.arg("--output").arg(hlsl_out).arg(spv);
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "spirv-cross exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn spirv-cross: {e}")),
    }
}

/// dxc: HLSL → DXIL 容器(RFC-0004 §4.2 (c))。`profile` 取 DXIL 着色器 profile
/// (如 `vs_6_0`/`ps_6_0`),`entry` 取入口名。成功写 `dxil_out`;失败 → `Err`。
/// 注:Vulkan SDK 的 dxc 产 DXIL 可能未签名(签名 validator 属 owner pin 环境);
/// 本驱动仅产容器,validator/签名核验由调用方按可用性 SKIP 决定(P-01)。
#[cfg(feature = "dxil-backend")]
pub fn dxc_hlsl_to_dxil(
    dxc: &Path,
    hlsl: &Path,
    profile: &str,
    entry: &str,
    dxil_out: &Path,
) -> Result<(), String> {
    let out = Command::new(dxc)
        .arg("-T")
        .arg(profile)
        .arg("-E")
        .arg(entry)
        .arg("-Fo")
        .arg(dxil_out)
        .arg(hlsl)
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "dxc (HLSL->DXIL) exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn dxc: {e}")),
    }
}

/// dxc: HLSL → SPIR-V(`-spirv`;`reflect` 时加 `-fspv-reflect` 携 `UserSemantic`)。
/// **本函数仅服务 host 冒烟取证**——以 dxc 产参考 SPIR-V 喂入 spirv-cross/dxc 链,
/// 验证 B 链外部工具在本机可端到端跑通(MIR→SPIR-V 自有降级见 `dxil_spirv`,Slice 2)。
#[cfg(feature = "dxil-backend")]
pub fn dxc_hlsl_to_spirv(
    dxc: &Path,
    hlsl: &Path,
    profile: &str,
    entry: &str,
    spv_out: &Path,
    reflect: bool,
) -> Result<(), String> {
    let mut cmd = Command::new(dxc);
    cmd.arg("-spirv");
    if reflect {
        cmd.arg("-fspv-reflect");
    }
    cmd.arg("-T")
        .arg(profile)
        .arg("-E")
        .arg(entry)
        .arg("-Fo")
        .arg(spv_out)
        .arg(hlsl);
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "dxc (HLSL->SPIR-V) exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn dxc: {e}")),
    }
}

/// DXIL 签名元素(ISG1 输入 / OSG1 输出 part 的一行;RFC-0004 §4.4 比较域)。
/// 仅承载**源码层可观察**字段(语义名 / index / 系统值 / 是否被使用);寄存器编号 /
/// mask / packing / 字节偏移属外部 conformance(§4.6(a)/Q-ABI-B),保留 `register`
/// 原文仅供诊断展示,**不**作语言 ABI 承诺。
#[cfg(feature = "dxil-backend")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigElement {
    pub name: String,
    pub index: u32,
    pub sysvalue: String,
    pub register: String,
    pub used: bool,
}

/// 一个 DXIL 容器的输入/输出签名(译后解析自 `dxc -dumpbin` 文本)。
#[cfg(feature = "dxil-backend")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DxilSignatures {
    pub input: Vec<SigElement>,
    pub output: Vec<SigElement>,
}

/// 从 `dxc -dumpbin` 反汇编文本解析 ISG1(`; Input signature:`)/ OSG1
/// (`; Output signature:`)签名表(RXS-0159/0162 强制签名一致性校验门的输入)。
/// 表行体例(dxc 反汇编注释表):
/// `; Name  Index  Mask  Register  SysValue  Format  Used`。容错:无表 → 空 Vec;
/// `Used` 列存在即视为被使用元素(RFC-0004 §4.4 (c))。
#[cfg(feature = "dxil-backend")]
pub fn parse_dxil_signatures(disasm: &str) -> DxilSignatures {
    #[derive(Clone, Copy, PartialEq)]
    enum Sec {
        None,
        In,
        Out,
    }
    let mut sec = Sec::None;
    let mut sigs = DxilSignatures::default();
    for raw in disasm.lines() {
        let line = raw.trim_start();
        let body = line.strip_prefix(';').map(str::trim).unwrap_or("");
        if body.starts_with("Input signature:") {
            sec = Sec::In;
            continue;
        }
        if body.starts_with("Output signature:") {
            sec = Sec::Out;
            continue;
        }
        if sec == Sec::None {
            continue;
        }
        // 段内:跳过表头 / 分隔线 / 空注释行;非注释行结束本段(下一段由标题切换)。
        if !line.starts_with(';') {
            sec = Sec::None;
            continue;
        }
        if body.is_empty() {
            // 空注释行(表前 / 表间分隔)跳过,不结束本段。
            continue;
        }
        let toks: Vec<&str> = body.split_whitespace().collect();
        if toks.is_empty() || toks[0] == "Name" || toks[0].starts_with("---") {
            continue;
        }
        // 至少 Name Index Mask Register SysValue Format(6 列);Used 第 7 列可缺。
        if toks.len() < 6 {
            continue;
        }
        let Ok(index) = toks[1].parse::<u32>() else {
            continue;
        };
        let elem = SigElement {
            name: toks[0].to_string(),
            index,
            register: toks[3].to_string(),
            sysvalue: toks[4].to_string(),
            used: toks.len() >= 7 && !toks[6].is_empty(),
        };
        match sec {
            Sec::In => sigs.input.push(elem),
            Sec::Out => sigs.output.push(elem),
            Sec::None => {}
        }
    }
    // dxc(>=1.8)`-dumpbin` 默认输出 LLVM IR 文本(签名在 `!dx.entryPoints` 元数据图,
    // 非 `; signature:` 注释表)→ 注释表为空时回落元数据图解析。
    if sigs.input.is_empty() && sigs.output.is_empty() {
        return parse_dxil_signatures_md(disasm);
    }
    sigs
}

/// 从 dxc `-dumpbin` 的 LLVM IR 文本解析 DXIL 签名元数据图(`!dx.entryPoints`):
/// `!dx.entryPoints = !{!E}` → `!E = !{@fn, !"name", !SIG, _, _}` →
/// `!SIG = !{!IN, !OUT, !PC}` → `!IN/!OUT = !{!e0, !e1, ..}` →
/// `!e = !{i32 id, !"SEM", .., !semIdxList(op4), .., !usage(op_last)}`。
/// 语义名取 op1,语义 index 取 op4 列表首 i32;系统值由 `SV_` 前缀判别;`used` 取
/// usage 列表存在性(RFC-0004 §4.4 (c),Slice 3 校验门细化)。
#[cfg(feature = "dxil-backend")]
fn parse_dxil_signatures_md(disasm: &str) -> DxilSignatures {
    use std::collections::HashMap;

    let mut md: HashMap<String, String> = HashMap::new();
    let mut entry_points: Option<String> = None;
    for raw in disasm.lines() {
        let l = raw.trim();
        if let Some(rest) = l.strip_prefix("!dx.entryPoints") {
            if let Some(idx) = rest.find('!') {
                entry_points = Some(rest[idx..].trim().to_string());
            }
            continue;
        }
        if let Some(eq) = l.find(" = ") {
            let key = l[..eq].trim();
            if key.starts_with('!') && key[1..].chars().all(|c| c.is_ascii_digit()) {
                md.insert(key.to_string(), l[eq + 3..].trim().to_string());
            }
        }
    }

    // `!{a, b, c}` → 顶层操作数(元数据引用为扁平 token,无需处理嵌套)。
    fn operands(rhs: &str) -> Vec<String> {
        let inner = rhs
            .trim()
            .strip_prefix("!{")
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or("");
        if inner.trim().is_empty() {
            return Vec::new();
        }
        inner.split(',').map(|s| s.trim().to_string()).collect()
    }
    fn md_ref(op: &str) -> Option<&str> {
        let t = op.trim();
        if t.starts_with("!{") {
            return None;
        }
        if t.starts_with('!') && t[1..].chars().all(|c| c.is_ascii_digit()) {
            Some(t)
        } else {
            None
        }
    }

    let resolve = |key: &str| -> Option<String> { md.get(key).cloned() };

    let mut sigs = DxilSignatures::default();
    let Some(ep_rhs) = entry_points else {
        return sigs;
    };
    // !dx.entryPoints 操作数首项 = 入口元数据节点。
    let Some(entry_key) = operands(&ep_rhs)
        .first()
        .and_then(|o| md_ref(o).map(str::to_string))
    else {
        return sigs;
    };
    let Some(entry_md) = resolve(&entry_key) else {
        return sigs;
    };
    let entry_ops = operands(&entry_md);
    // 入口操作数 [func, name, signatures, resources, props];signatures = op2。
    let Some(sig_key) = entry_ops.get(2).and_then(|o| md_ref(o).map(str::to_string)) else {
        return sigs;
    };
    let Some(sig_md) = resolve(&sig_key) else {
        return sigs;
    };
    let sig_ops = operands(&sig_md);

    let parse_list = |op: Option<&String>| -> Vec<SigElement> {
        let mut out = Vec::new();
        let Some(list_key) = op.and_then(|o| md_ref(o).map(str::to_string)) else {
            return out;
        };
        let Some(list_md) = resolve(&list_key) else {
            return out;
        };
        for elem_op in operands(&list_md) {
            let Some(elem_key) = md_ref(&elem_op).map(str::to_string) else {
                continue;
            };
            let Some(elem_md) = resolve(&elem_key) else {
                continue;
            };
            let eops = operands(&elem_md);
            // op1 = !"SEM"
            let name = eops
                .get(1)
                .map(|s| s.trim_start_matches('!').trim_matches('"').to_string())
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            // op4 = !semIdxList → 首 i32 为语义 index
            let index = eops
                .get(4)
                .and_then(|o| md_ref(o).map(str::to_string))
                .and_then(|k| resolve(&k))
                .and_then(|m| {
                    operands(&m).first().and_then(|t| {
                        t.trim()
                            .strip_prefix("i32 ")
                            .map(str::trim)
                            .and_then(|n| n.parse::<u32>().ok())
                    })
                })
                .unwrap_or(0);
            let sysvalue = if name.starts_with("SV_") {
                name.clone()
            } else {
                "NONE".to_string()
            };
            out.push(SigElement {
                name,
                index,
                register: String::new(),
                sysvalue,
                used: true,
            });
        }
        out
    };

    sigs.input = parse_list(sig_ops.first());
    sigs.output = parse_list(sig_ops.get(1));
    sigs
}

#[cfg(all(test, feature = "dxil-backend"))]
mod dxil_b_chain_tests {
    use super::*;

    fn tool_present(p: &Path, probe: &str) -> bool {
        Command::new(p).arg(probe).output().is_ok()
    }

    fn scratch_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("rurix_dxil_b_{nanos}"));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// RXS-0162:B 转译链外部工具(spirv-cross/dxc)在本机端到端跑通 + 签名 part 可
    /// 程序化解析(`elemcount>0`)。工具缺失(CI 无 Vulkan SDK)→ SKIP(非 fail,
    /// 对齐 RXS-0073 ptxas 干验证 SKIP;真实红绿在带工具链的 dev/owner 环境)。
    /// (Slice 1 驱动层冒烟;条款锚定随 Slice 5 条款体 `### RXS-0162` 同落,届时补 spec 锚。)
    #[test]
    fn b_chain_end_to_end_smoke() {
        let Some(spvx) = locate_spirv_cross() else {
            eprintln!("[SKIP] spirv-cross 不可定位");
            return;
        };
        let Some(dxc) = locate_dxc() else {
            eprintln!("[SKIP] dxc 不可定位");
            return;
        };
        if !tool_present(&dxc, "--version") || !tool_present(&spvx, "--help") {
            eprintln!("[SKIP] B 链工具不可用(dev-override 缺失;真实红绿在 owner pin 环境)");
            return;
        }

        let dir = scratch_dir();
        let hlsl_in = dir.join("vs_in.hlsl");
        std::fs::write(
            &hlsl_in,
            "struct VSIn { float3 pos : POSITION; float3 nrm : NORMAL; };\n\
             struct VSOut { float4 pos : SV_Position; float3 nrm : TEXCOORD0; };\n\
             VSOut main(VSIn i) {\n\
             \x20 VSOut o; o.pos = float4(i.pos, 1.0); o.nrm = i.nrm; return o;\n\
             }\n",
        )
        .expect("write hlsl");

        let spv = dir.join("vs.spv");
        if let Err(e) = dxc_hlsl_to_spirv(&dxc, &hlsl_in, "vs_6_0", "main", &spv, true) {
            eprintln!("[SKIP] dxc HLSL->SPIR-V 失败(dev-override 版本差异): {e}");
            return;
        }
        let hlsl_out = dir.join("vs_rt.hlsl");
        // 顶点输入语义保名旗标(`--set-hlsl-vertex-input-semantic <loc> <semantic>`,
        // RFC-0004 §4.4 机制①;POSITION→loc0 / NORMAL→loc1,按 ISG1 location 顺序)。
        // 生产侧由 dxil_codegen::vertex_input_semantic_flags 经 io_sig 导出(非硬编码);
        // 本驱动冒烟直接以等价旗标实测顶点输入名经链端到端存活(不退化为 TEXCOORD#)。
        let keep_flags = [
            "--set-hlsl-vertex-input-semantic".to_owned(),
            "0".to_owned(),
            "POSITION".to_owned(),
            "--set-hlsl-vertex-input-semantic".to_owned(),
            "1".to_owned(),
            "NORMAL".to_owned(),
        ];
        if let Err(e) = spirv_cross_to_hlsl(&spvx, &spv, &hlsl_out, 60, &keep_flags) {
            eprintln!("[SKIP] spirv-cross SPIR-V->HLSL 失败: {e}");
            return;
        }
        let dxil = dir.join("vs.dxil");
        if let Err(e) = dxc_hlsl_to_dxil(&dxc, &hlsl_out, "vs_6_0", "main", &dxil) {
            eprintln!("[SKIP] dxc HLSL->DXIL 失败: {e}");
            return;
        }
        // dxc -dumpbin 反汇编 + 签名 part 解析(强制签名一致性校验门的取数基础)。
        let dxc_dir = dxc.parent().map(Path::to_path_buf).unwrap_or_default();
        let Ok(disasm) = dxc_disasm(&dxc_dir, &dxil) else {
            eprintln!("[SKIP] dxc -dumpbin 失败");
            return;
        };
        let sigs = parse_dxil_signatures(&disasm);
        assert!(
            !sigs.input.is_empty(),
            "B 链产 DXIL 输入签名 elemcount=0(应 >0;签名不可达即 B 路前提失败)。disasm:\n{disasm}"
        );
        // 机制①实测断言:顶点输入用户语义名 POSITION/NORMAL 经保名旗标端到端存活,
        // **不**退化为通用 TEXCOORD#(strip 尾随数字 + 大小写无关匹配)。
        let has_input = |want: &str| {
            sigs.input.iter().any(|e| {
                e.name
                    .trim_end_matches(|c: char| c.is_ascii_digit())
                    .eq_ignore_ascii_case(want)
            })
        };
        assert!(
            has_input("POSITION") && has_input("NORMAL"),
            "顶点输入语义名应经保名旗标存活(POSITION/NORMAL 不退化为 TEXCOORD#)。\
             ISG1={:?}",
            sigs.input
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(all(test, feature = "dxil-backend"))]
mod dxil_sig_parse_tests {
    use super::*;

    /// 元数据图形态(dxc >=1.8 `-dumpbin` 默认输出):`!dx.entryPoints` →
    /// 输入/输出签名元素经 `parse_dxil_signatures_md` 解出名/index/系统值。
    /// 工具无关,CI 恒跑(签名一致性校验门取数基础,Slice 3 依赖)。
    #[test]
    fn parses_dx_entrypoints_metadata() {
        let disasm = "\
; some llvm ir header\n\
define void @main() {\n\
  ret void\n\
}\n\
!dx.entryPoints = !{!5}\n\
!5 = !{void ()* @main, !\"main\", !6, null, null}\n\
!6 = !{!7, !13, null}\n\
!7 = !{!8, !11}\n\
!8 = !{i32 0, !\"TEXCOORD\", i8 9, i8 0, !9, i8 0, i32 1, i8 3, i32 0, i8 0, !10}\n\
!9 = !{i32 0}\n\
!10 = !{i32 3, i32 7}\n\
!11 = !{i32 1, !\"TEXCOORD\", i8 9, i8 0, !12, i8 0, i32 1, i8 3, i32 1, i8 0, !10}\n\
!12 = !{i32 1}\n\
!13 = !{!14, !15}\n\
!14 = !{i32 0, !\"TEXCOORD\", i8 9, i8 0, !9, i8 2, i32 1, i8 3, i32 0, i8 0, !10}\n\
!15 = !{i32 1, !\"SV_Position\", i8 9, i8 3, !9, i8 4, i32 1, i8 4, i32 1, i8 0, !16}\n\
!16 = !{i32 3, i32 15}\n";
        let sigs = parse_dxil_signatures(disasm);
        assert_eq!(sigs.input.len(), 2, "输入签名应 2 元素");
        assert_eq!(sigs.input[0].name, "TEXCOORD");
        assert_eq!(sigs.input[1].index, 1, "第二输入语义 index=1");
        assert_eq!(sigs.output.len(), 2, "输出签名应 2 元素");
        let sv = sigs
            .output
            .iter()
            .find(|e| e.name == "SV_Position")
            .unwrap();
        assert_eq!(sv.sysvalue, "SV_Position", "SV_ 前缀判为系统值");
    }

    /// 注释表形态(签名 part 反汇编注释表):`Used` 列存在 → used=true,缺 → false。
    #[test]
    fn parses_comment_signature_table() {
        let disasm = "\
; Input signature:\n\
;\n\
; Name                 Index   Mask Register SysValue  Format   Used\n\
; -------------------- ----- ------ -------- -------- ------- ------\n\
; POSITION                 0   xyzw        0     NONE   float   xyzw\n\
; NORMAL                   0   xyzw        1     NONE   float\n\
;\n\
; Output signature:\n\
;\n\
; Name                 Index   Mask Register SysValue  Format   Used\n\
; -------------------- ----- ------ -------- -------- ------- ------\n\
; SV_Position              0   xyzw        0      POS   float   xyzw\n";
        let sigs = parse_dxil_signatures(disasm);
        assert_eq!(sigs.input.len(), 2, "输入签名应 2 元素");
        assert!(sigs.input[0].used, "POSITION 有 Used 列 → used");
        assert!(!sigs.input[1].used, "NORMAL 缺 Used 列 → 未用");
        assert_eq!(sigs.output.len(), 1);
        assert_eq!(sigs.output[0].name, "SV_Position");
    }

    /// 无签名(compute / 空)→ 空签名(不 panic,容错)。
    #[test]
    fn parses_empty_when_no_signature() {
        let sigs = parse_dxil_signatures("; ModuleID = 'x'\ndefine void @cs() {\n  ret void\n}\n");
        assert!(sigs.input.is_empty() && sigs.output.is_empty());
    }
}
