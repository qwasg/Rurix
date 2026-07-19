//! `binding_layout` — G2.3 绑定布局推导 host 侧 safe 推导面(RXS-0163~0166;
//! 承 [RFC-0005](../../../rfcs/0005-binding-layout-inference.md),owner Approved
//! 2026-06-28)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`(复用,RFC-0005 §9 Q-Gate=A);
//! 未启用时整模块不编入 rurixc,PTX 路径(D-207)与默认构建零漂移。
//!
//! # 职责(PR-E2a:条款体 + host 侧 safe 推导 + 单测锚定)
//! 从着色阶段签名的资源句柄使用([`crate::mir::ResourceBinding`],承 RXS-0156
//! 资源句柄类型面)**纯 host/safe 推导** D3D12 绑定布局:
//! - **RXS-0163**:资源句柄 → SPIR-V 资源绑定降级面([`infer_spirv_bindings`];
//!   opaque 资源类型 + `DescriptorSet`/`Binding` 装饰,按声明序确定性)。
//! - **RXS-0164**:register/space 分配推导([`infer_register_assignments`];
//!   §9 Q-Space=B 按资源种类分轴,首期单 `space0`,CBV/SRV/UAV/Sampler→b/t/u/s
//!   各自从 0 递增)+ register/layout 冲突核验([`detect_register_conflict`])。
//! - **RXS-0165**:root signature 形态推导([`infer_root_signature`];§9
//!   Q-RootShape=B CBV root descriptor + SRV/UAV/Sampler descriptor table)+
//!   RTS0 容器序列化([`serialize_rts0`];D3D12 既定容器格式机械落字节)。
//! - **RXS-0166**:绑定布局推导一致性校验门([`check_binding_consistency`];
//!   PSV0 反射 vs 推导意图比对)+ strict-only 推导失败。
//!
//! # 纯 host/safe(零新 unsafe)
//! 全模块仅以 `Vec`/整数累积,无任何 `unsafe` 块(crate `unsafe_code = "deny"`),
//! 照 [`crate::dxil_sig_gate::signature_gate::check_stage_link`] host/safe 范本。
//!
//! # 不接线生产 emit(PR-E2b 归属)
//! 本模块**不**接 `emit_spirv` 资源绑定装饰生产路径、**不**接 register/space 生产
//! codegen、**不**接 PSV0 校验门生产 emit、**不**改 `registry/error_codes.json`
//! (错误码占位「6xxx」)、**不**落 golden、**不** device 真跑。以上归 PR-E2b /
//! owner 闸门(G-G2-3)。
//!
//! # 🔒 禁区(只引边界声明,不落语义本体)
//! 具体 register/space/mask/packing 数值物理布局、descriptor table 字节偏移、
//! root parameter DWORD 物理布局、纹理路径内存模型、DXIL/SPIR-V UB 均**不**在本
//! 模块冻结为 stable 语言保证。[`serialize_rts0`] 按 D3D12 既定容器格式机械落字节
//! (类比 RXS-0162 DXIL 容器),其布局为**实现确定、gate 后、非 stable**,不自创
//! ABI;真链 validator / device 核验归 PR-E2b。

use crate::mir::{MirResourceType, ResourceBinding, ResourceClass, ResourceCount};

/// 绑定布局推导失败(strict-only;RFC-0005 §4 / P-01,无运行期 fallback)。
///
/// **错误码(G2.3 PR-E2b-2 已落,owner 已裁)**:本枚举只定义推导失败的类型化语义,
/// **不**直接发码、**不**改 `registry/error_codes.json`、**不**接线生产 emit;落码与
/// emit 接线在 [`crate::dxil_codegen`] 边界([`DxilBError::Binding`] →
/// `emit_b_error` 按变体分派)。各变体专属码(避开 RX6014:agent 裁定 RX6014 给
/// RXS-0160 阶段间接口错链):`Unmappable` 复用 RX6013 `codegen.dxil_unmappable`
/// (bindless / unbounded RD-018,owner 已裁不新开);`RegisterConflict` → RX6015
/// `codegen.dxil_register_conflict`;`RootSignatureTooLarge` → RX6016
/// `codegen.dxil_root_signature_too_large`;`Psv0Mismatch` → RX6017
/// `codegen.dxil_psv0_mismatch`。🔒 诊断 message 只描述失败类别,不落 register/space/
/// packing 物理布局值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingInferError {
    /// 资源不可映射为合规有界绑定(bindless / unbounded descriptor array →
    /// RD-018 defer;或资源种类不可降级)。strict-only 拒绝,不发明 descriptor
    /// heap 编码(复用 RX6013 `codegen.dxil_unmappable`,owner 已裁不新开)。
    Unmappable {
        /// 不可映射构造的诊断上下文(资源名 / 种类 / 基数)。
        detail: String,
    },
    /// register/layout 冲突:两个资源占同一(种类轴, register, space)。
    /// strict-only 拒绝,无 fallback(RX6015 `codegen.dxil_register_conflict`)。
    RegisterConflict {
        /// 冲突的诊断上下文(两端资源名 / 轴 / register / space)。
        detail: String,
    },
    /// root signature 推导超 D3D12 64 DWORD 上限。strict-only 拒绝
    /// (RX6016 `codegen.dxil_root_signature_too_large`)。
    RootSignatureTooLarge {
        /// 推导出的 DWORD 成本。
        dwords: u32,
        /// D3D12 上限(64 DWORD)。
        limit: u32,
    },
    /// PSV0 反射与推导意图不一致(不可推导 / 篡改 / mismatch)。strict-only 拒绝
    /// (RX6017 `codegen.dxil_psv0_mismatch`)。
    Psv0Mismatch {
        /// 失配的诊断上下文。
        detail: String,
    },
}

impl std::fmt::Display for BindingInferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingInferError::Unmappable { detail } => {
                write!(f, "绑定布局不可推导(不可映射 / bindless RD-018): {detail}")
            }
            BindingInferError::RegisterConflict { detail } => {
                write!(f, "绑定布局 register/layout 冲突: {detail}")
            }
            BindingInferError::RootSignatureTooLarge { dwords, limit } => {
                write!(
                    f,
                    "root signature 推导超上限: {dwords} DWORD > {limit} DWORD"
                )
            }
            BindingInferError::Psv0Mismatch { detail } => {
                write!(f, "PSV0 反射与推导意图不一致: {detail}")
            }
        }
    }
}

impl std::error::Error for BindingInferError {}

/// D3D12 root signature DWORD 上限(64;RFC-0005 §4 / D3D12 既定)。
pub const ROOT_SIGNATURE_DWORD_LIMIT: u32 = 64;

// ── G3.4 bindless 独占 set/space 分配律常量(RXS-0233;RFC-0013 §4.C2/§4.0-1) ──
//
// 🔒(边界声明,沿 RXS-0163 先例):set/space **具体数值**为实现确定、gate 后、
// 非 stable,不冻结为 ABI;本层只承诺「独占性 / 声明序确定性 / 有界路零漂移」。

/// Vk-native 形态:无界表独占 descriptor set,**自 set4 起**按声明序递增
/// (类别轴 set0~3 之后首个空闲 set,§4.0-1)。
pub const VK_BINDLESS_SET_BASE: u32 = 4;

/// B 链形态(spirv-cross → HLSL):无界表独占 descriptor set,**自 set1 起**按声明
/// 序递增(bounded 恒 set0 之后;spirv-cross 默认 set→space 映射使其落
/// `register(t0, space{1+ord})`,§4.C3 DXIL 腿)。bounded 资源 set0 装饰**字节不动**。
pub const B_CHAIN_BINDLESS_SET_BASE: u32 = 1;

/// D3D12/RTS0 形态:无界表独占 register space,**自 space1 起**按声明序递增
/// (bounded 恒 space0)。
pub const RTS0_BINDLESS_SPACE_BASE: u32 = 1;

/// 无界 descriptor range 的 `NumDescriptors` 哨兵(D3D12 unbounded = `0xFFFFFFFF`)。
/// 独占 space 分配律使无界 range「吞轴」行为结构性无冲突(§4.C2)。
pub const UNBOUNDED_DESCRIPTOR_COUNT: u32 = 0xffff_ffff;

// ════════════════ RXS-0163:资源句柄 → SPIR-V 资源绑定降级面 ════════════════

/// 单个资源的 SPIR-V 资源绑定装饰意图(RXS-0163)。
///
/// 资源句柄降级为 SPIR-V opaque 资源类型 + `DescriptorSet`/`Binding` 装饰。首期
/// 单 set(`set == 0`),`binding` 按声明序确定性递增。具体 binding 数值为**实现
/// 确定、gate 后、非 stable**(不冻结为 ABI)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpirvBinding {
    /// 源码资源名(保名依据)。
    pub name: String,
    /// 资源种类轴(opaque 资源类型归类)。
    pub class: ResourceClass,
    /// `DescriptorSet` 装饰。有界:B 链恒 0 / Vk-native = 类别轴(0~3)。无界表
    /// (G3.4 RXS-0233):B 链自 set1、Vk-native 自 set4 独占(§4.C2)。
    pub set: u32,
    /// `Binding` 装饰(有界 = 声明序确定性递增;无界表 = 0 单点)。
    pub binding: u32,
}

/// RXS-0163:资源句柄 → SPIR-V 资源绑定降级面推导(纯 host/safe,确定性)。
///
/// 按 `resources` 声明序(io_sig 顺序)确定性导出每个资源的 SPIR-V 绑定装饰意图:
/// `DescriptorSet = 0`(首期单 set)、`Binding` 自 0 起按声明序递增(有界数组占
/// `count` 个连续 binding)。
///
/// # Errors
/// 无界**非-SRV-纹理**表(无界 Sampler/CBV/UAV)/ 零基数有界 →
/// [`BindingInferError::Unmappable`](RD-018/RX6013,strict-only)。首期无界仅
/// `[Texture2D<F>]` 合法(RXS-0231/0233,G3.4)。
pub fn infer_spirv_bindings(
    resources: &[ResourceBinding],
) -> Result<Vec<SpirvBinding>, BindingInferError> {
    // SPIR-V `Binding` 按资源种类轴(per-class)递增,与 register/space 推导
    // (`infer_register_assignments`)同口径——确保 SPIR-V binding → spirv-cross HLSL
    // register(t/s/b/u 各自从 0)→ RTS0 register 三者一致(RXS-0164)。RFC-0007 采样真
    // 用例暴露:旧全局递增 binding 会令 `(Texture2D, Sampler)` 的 sampler 落 SPIR-V binding 1
    // → spirv-cross HLSL `s1`,而 RTS0 推导 sampler 为 `s0` → device 描述符表 register 失配
    // (lighting pass 采样不到 G-buffer)。改 per-class 后 sampler binding 0 → `s0` ↔ RTS0 `s0`。
    let mut counters = AxisCounters::default();
    let mut unbounded_ord = 0u32;
    let mut out = Vec::with_capacity(resources.len());
    for r in resources {
        let class = r.res.class();
        match resource_multiplicity(r)? {
            Multiplicity::Bounded(span) => {
                let binding = counters.take(class, span);
                out.push(SpirvBinding {
                    name: r.name.clone(),
                    class,
                    set: 0,
                    binding,
                });
            }
            // G3.4(RXS-0233):无界 SRV 纹理表——B 链形态自 set1 独占 set、binding 0
            // (spirv-cross → `register(t0, space{1+ord})`)。bounded 资源 set0 装饰字节不动。
            Multiplicity::UnboundedTable => {
                let set = B_CHAIN_BINDLESS_SET_BASE + unbounded_ord;
                unbounded_ord += 1;
                out.push(SpirvBinding {
                    name: r.name.clone(),
                    class,
                    set,
                    binding: 0,
                });
            }
        }
    }
    Ok(out)
}

/// 资源的绑定基数(G3.4 RXS-0233;`Unbounded` SRV 纹理 = 合法无界表)。
enum Multiplicity {
    /// 有界:占 `n` 个连续 descriptor / register(单 = 1)。
    Bounded(u32),
    /// 无界 SRV 纹理表(`[Texture2D<F>]`,RXS-0231):独占 set / space。
    UnboundedTable,
}

/// 资源消费的绑定基数判别(RXS-0233;RFC-0013 §4.C2)。**Unbounded 翻转**:自
/// `descriptor_span` 旧「一律 Unmappable」翻转为「SRV 纹理无界 = 合法无界表,余
/// 维持 Unmappable/RX6013」。
fn resource_multiplicity(r: &ResourceBinding) -> Result<Multiplicity, BindingInferError> {
    match r.count {
        ResourceCount::One => Ok(Multiplicity::Bounded(1)),
        ResourceCount::Bounded(n) if n >= 1 => Ok(Multiplicity::Bounded(n)),
        ResourceCount::Bounded(_) => Err(BindingInferError::Unmappable {
            detail: format!("资源 `{}` 有界数组基数为 0(非法)", r.name),
        }),
        // G3.4 RXS-0233(RFC-0013 §4.C2):首期无界仅 SRV 纹理(`Texture2D<F>`)合法。
        ResourceCount::Unbounded if matches!(r.res, MirResourceType::Texture2D(_)) => {
            Ok(Multiplicity::UnboundedTable)
        }
        // 无界 Sampler / CBV / UAV(StructuredBuffer)表:维持 Unmappable/RX6013(§8,不新码)。
        ResourceCount::Unbounded => Err(BindingInferError::Unmappable {
            detail: format!(
                "资源 `{}` 为无界非-SRV-纹理表(首期无界仅 `[Texture2D<F>]`;无界 Sampler/CBV/UAV 维持 RD-018/RX6013)",
                r.name
            ),
        }),
    }
}

// ════════════════ RXS-0164:register/space 分配推导 ════════════════

/// 单个资源的 D3D12 register/space 分配意图(RXS-0164)。
///
/// §9 Q-Space=B 按资源种类分轴:CBV→`b` / SRV→`t` / UAV→`u` / Sampler→`s`,各轴
/// 自 0 起按声明序递增;首期单 `space0`。具体 register/space 数值为**实现确定、
/// gate 后、非 stable**(🔒 不冻结为 ABI 布局)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterAssignment {
    /// 源码资源名(保名依据)。
    pub name: String,
    /// 资源种类轴(决定 b/t/u/s 前缀)。
    pub class: ResourceClass,
    /// 轴内 register 基号(有界数组占 `span` 个连续号自此起)。
    pub register: u32,
    /// register space(首期恒 0)。
    pub space: u32,
    /// 连续 register 跨度(单 = 1,有界数组 = n)。
    pub span: u32,
}

impl RegisterAssignment {
    /// D3D12 register 轴前缀字符(`b`/`t`/`u`/`s`;诊断 / 文档用,非 ABI 冻结)。
    pub fn axis_prefix(&self) -> char {
        match self.class {
            ResourceClass::Cbv => 'b',
            ResourceClass::Srv => 't',
            ResourceClass::Uav => 'u',
            ResourceClass::Sampler => 's',
        }
    }
}

/// 四轴 register 计数器(CBV/SRV/UAV/Sampler 各自从 0 递增)。
#[derive(Default)]
struct AxisCounters {
    cbv: u32,
    srv: u32,
    uav: u32,
    sampler: u32,
}

impl AxisCounters {
    /// 取某轴当前基号并按 `span` 递增(返回分配前的基号)。
    fn take(&mut self, class: ResourceClass, span: u32) -> u32 {
        let slot = match class {
            ResourceClass::Cbv => &mut self.cbv,
            ResourceClass::Srv => &mut self.srv,
            ResourceClass::Uav => &mut self.uav,
            ResourceClass::Sampler => &mut self.sampler,
        };
        let base = *slot;
        *slot += span;
        base
    }
}

/// RXS-0164:register/space 分配推导(纯 host/safe,确定性)。
///
/// 按 `resources` 声明序,§9 Q-Space=B 各资源种类轴(b/t/u/s)自 0 起递增分配
/// register 基号,首期单 `space0`。有界数组占 `count` 个连续 register。
///
/// # Errors
/// 无界非-SRV-纹理表 / 非法基数 → [`BindingInferError::Unmappable`](RD-018/RX6013)。
///
/// G3.4(RXS-0233):无界 SRV 纹理表 = `register 0`、独占 `space{1+ord}`(自 space1
/// 按声明序)、`span = UNBOUNDED_DESCRIPTOR_COUNT`;bounded 恒 space0 零漂移。
pub fn infer_register_assignments(
    resources: &[ResourceBinding],
) -> Result<Vec<RegisterAssignment>, BindingInferError> {
    let mut counters = AxisCounters::default();
    let mut unbounded_ord = 0u32;
    let mut out = Vec::with_capacity(resources.len());
    for r in resources {
        let class = r.res.class();
        match resource_multiplicity(r)? {
            Multiplicity::Bounded(span) => {
                let register = counters.take(class, span);
                out.push(RegisterAssignment {
                    name: r.name.clone(),
                    class,
                    register,
                    space: 0,
                    span,
                });
            }
            // 无界 SRV 纹理表:base_register 0,独占 space1+,unbounded 计数哨兵。
            Multiplicity::UnboundedTable => {
                let space = RTS0_BINDLESS_SPACE_BASE + unbounded_ord;
                unbounded_ord += 1;
                out.push(RegisterAssignment {
                    name: r.name.clone(),
                    class,
                    register: 0,
                    space,
                    span: UNBOUNDED_DESCRIPTOR_COUNT,
                });
            }
        }
    }
    Ok(out)
}

/// RXS-0164:register/layout 冲突核验(strict-only 推导失败判据)。
///
/// 核实任意两个分配不占同一(种类轴, space, register)区间:有界数组的
/// `[register, register + span)` 半开区间在同轴 + 同 space 内不得重叠。推导自身
/// (按轴递增)天然无冲突;本核验为一致性校验门(RXS-0166)与未来显式
/// `#[binding(...)]` 覆盖(后期)的 strict-only 兜底,直接对任意分配集判定。
///
/// # Errors
/// 同轴 + 同 space 内 register 区间重叠 → [`BindingInferError::RegisterConflict`]。
pub fn detect_register_conflict(
    assignments: &[RegisterAssignment],
) -> Result<(), BindingInferError> {
    for (i, a) in assignments.iter().enumerate() {
        for b in &assignments[i + 1..] {
            if a.class == b.class && a.space == b.space && ranges_overlap(a, b) {
                return Err(BindingInferError::RegisterConflict {
                    detail: format!(
                        "资源 `{}` 与 `{}` 在 {}{}..(space{}) register 区间重叠",
                        a.name,
                        b.name,
                        a.axis_prefix(),
                        a.register,
                        a.space
                    ),
                });
            }
        }
    }
    Ok(())
}

/// 两个同轴 + 同 space 分配的 `[register, register + span)` 半开区间是否重叠。
/// `saturating_add` 容纳无界表 `span = UNBOUNDED_DESCRIPTOR_COUNT`(无界表各占独立
/// space,`detect_register_conflict` 的同-space 前置使其永不进本比较,此处仅防溢出)。
fn ranges_overlap(a: &RegisterAssignment, b: &RegisterAssignment) -> bool {
    let a_end = a.register.saturating_add(a.span);
    let b_end = b.register.saturating_add(b.span);
    a.register < b_end && b.register < a_end
}

// ════════════════ RXS-0165:root signature 形态推导 + RTS0 序列化 ════════════════

/// D3D12 descriptor range(descriptor table 内的一段同种类连续 descriptor)。
///
/// 物理 `offset_from_table_start` 取 D3D12 既定 `APPEND` 哨兵([`RANGE_OFFSET_APPEND`]),
/// **不**在本层冻结具体字节偏移(🔒 ABI 禁区)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorRange {
    /// range 种类(SRV/UAV/Sampler;CBV 走 root descriptor 不入 table)。
    pub range_type: ResourceClass,
    /// 该 range 的 descriptor 数(同轴 span 之和)。
    pub num_descriptors: u32,
    /// 轴内基号(首期自 0 起)。
    pub base_register: u32,
    /// register space(首期恒 0)。
    pub space: u32,
}

/// root parameter 形态(§9 Q-RootShape=B)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootParameter {
    /// CBV root descriptor(每个 CBV 一个;成本 2 DWORD)。
    CbvRootDescriptor {
        /// `b` 轴 register 基号。
        register: u32,
        /// register space。
        space: u32,
    },
    /// descriptor table(SRV/UAV 合表 / Sampler 独表;成本 1 DWORD)。
    DescriptorTable {
        /// 表内 range 序列(确定性:SRV 先于 UAV)。
        ranges: Vec<DescriptorRange>,
    },
}

/// root signature 形态推导意图(§9 Q-RootShape=B)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSignature {
    /// root parameter 序列(确定性:CBV root descriptors → SRV/UAV 表 → Sampler 表)。
    pub parameters: Vec<RootParameter>,
    /// root signature flags(首期 0 = D3D12_ROOT_SIGNATURE_FLAG_NONE)。
    pub flags: u32,
}

/// D3D12 `D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND` 哨兵(不冻结物理偏移)。
pub const RANGE_OFFSET_APPEND: u32 = 0xffff_ffff;

/// RXS-0165:root signature 形态推导(纯 host/safe,确定性)。
///
/// §9 Q-RootShape=B:每个 CBV → CBV root descriptor;全部 SRV + UAV → 单一
/// descriptor table(SRV range 先于 UAV range);全部 Sampler → 独立 descriptor
/// table(D3D12 sampler 必须独表)。参数序确定:CBV root descriptors → SRV/UAV
/// 表 → Sampler 表。
///
/// # Errors
/// - `Unbounded` / 非法基数 → [`BindingInferError::Unmappable`](RD-018)。
/// - 推导成本超 64 DWORD → [`BindingInferError::RootSignatureTooLarge`]。
pub fn infer_root_signature(
    resources: &[ResourceBinding],
) -> Result<RootSignature, BindingInferError> {
    let assignments = infer_register_assignments(resources)?;
    // 推导自身按轴递增、天然无冲突;仍过 strict-only 冲突门(防御 + 一致性门复用)。
    detect_register_conflict(&assignments)?;

    let mut parameters = Vec::new();

    // CBV → 各自 root descriptor(声明序)。
    for a in assignments.iter().filter(|a| a.class == ResourceClass::Cbv) {
        parameters.push(RootParameter::CbvRootDescriptor {
            register: a.register,
            space: a.space,
        });
    }

    // SRV + UAV → 单一 descriptor table(SRV range 先于 UAV range)。
    let mut srv_uav_ranges = Vec::new();
    if let Some(range) = axis_range(&assignments, ResourceClass::Srv) {
        srv_uav_ranges.push(range);
    }
    if let Some(range) = axis_range(&assignments, ResourceClass::Uav) {
        srv_uav_ranges.push(range);
    }
    if !srv_uav_ranges.is_empty() {
        parameters.push(RootParameter::DescriptorTable {
            ranges: srv_uav_ranges,
        });
    }

    // Sampler → 独立 descriptor table。
    if let Some(range) = axis_range(&assignments, ResourceClass::Sampler) {
        parameters.push(RootParameter::DescriptorTable {
            ranges: vec![range],
        });
    }

    // G3.4(RXS-0233):每个无界 SRV 纹理表 = 独占 descriptor table(单 unbounded SRV
    // range,自 space1;`NumDescriptors = 0xFFFFFFFF`、`BaseShaderRegister = 0`)。独占
    // space 分配律使 unbounded range「吞轴」结构性无冲突(§4.C2)。声明序稳定(assignments
    // 保序,space 自 space1 递增即声明序)。
    for a in assignments
        .iter()
        .filter(|a| a.space >= RTS0_BINDLESS_SPACE_BASE)
    {
        parameters.push(RootParameter::DescriptorTable {
            ranges: vec![DescriptorRange {
                range_type: ResourceClass::Srv,
                num_descriptors: UNBOUNDED_DESCRIPTOR_COUNT,
                base_register: 0,
                space: a.space,
            }],
        });
    }

    let rs = RootSignature {
        parameters,
        flags: 0,
    };

    let dwords = root_signature_cost_dwords(&rs);
    if dwords > ROOT_SIGNATURE_DWORD_LIMIT {
        return Err(BindingInferError::RootSignatureTooLarge {
            dwords,
            limit: ROOT_SIGNATURE_DWORD_LIMIT,
        });
    }
    Ok(rs)
}

/// 把某轴的**有界(space0)**分配聚合为单个 descriptor range(无该轴 bounded 资源
/// → `None`)。无界表(space1+)独占各自 table,不进本聚合(§4.C2)。
fn axis_range(assignments: &[RegisterAssignment], class: ResourceClass) -> Option<DescriptorRange> {
    let mut total = 0u32;
    let mut base = u32::MAX;
    for a in assignments
        .iter()
        .filter(|a| a.class == class && a.space == 0)
    {
        total += a.span;
        base = base.min(a.register);
    }
    if total == 0 {
        return None;
    }
    Some(DescriptorRange {
        range_type: class,
        num_descriptors: total,
        base_register: base,
        space: 0,
    })
}

/// root signature DWORD 成本(D3D12 既定:CBV root descriptor = 2 DWORD;
/// descriptor table = 1 DWORD;root constant 后期不在本期形态)。
pub fn root_signature_cost_dwords(rs: &RootSignature) -> u32 {
    rs.parameters
        .iter()
        .map(|p| match p {
            RootParameter::CbvRootDescriptor { .. } => 2,
            RootParameter::DescriptorTable { .. } => 1,
        })
        .sum()
}

// ── RTS0 容器序列化(D3D12 既定容器格式机械落字节;非 stable,类比 RXS-0162) ──

/// D3D12 `D3D12_ROOT_PARAMETER_TYPE`(序列化机械码,非语言 ABI 冻结)。
const PARAM_TYPE_DESCRIPTOR_TABLE: u32 = 0;
const PARAM_TYPE_CBV: u32 = 2;
/// `D3D12_SHADER_VISIBILITY_ALL`。
const SHADER_VISIBILITY_ALL: u32 = 0;
/// `D3D12_DESCRIPTOR_RANGE_TYPE`(SRV=0 / UAV=1 / CBV=2 / SAMPLER=3)。
fn range_type_code(class: ResourceClass) -> u32 {
    match class {
        ResourceClass::Srv => 0,
        ResourceClass::Uav => 1,
        ResourceClass::Cbv => 2,
        ResourceClass::Sampler => 3,
    }
}

/// RTS0 序列化版本(v1.0 = 1;D3D12 既定)。
pub const RTS0_VERSION_1_0: u32 = 1;

/// 小端 u32 追加(机械序列化基件)。
fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// RXS-0165:root signature → RTS0 容器字节序列化(纯 host/safe,确定性)。
///
/// 按 D3D12 既定容器格式机械落字节:外层 DXBC 容器(fourcc `DXBC` + 16 字节摘要
/// 占位 + 版本 + 总长 + part 计数 + part 偏移表)+ 单一 `RTS0` part,其载荷为
/// versioned root signature 序列化形态(v1.0:版本 / 参数计数 / 参数偏移 / 静态
/// sampler 计数 / flags + 参数记录 + 参数载荷 + descriptor range 数组)。
///
/// **非 stable**:容器 16 字节摘要为零占位(真实 DXBC 摘要归 PR-E2b);具体字节
/// 布局为实现确定、gate 后产物,**不**冻结为 stable 语言 ABI;真链 validator /
/// device 核验归 PR-E2b。确定性:相同 `rs` 两次序列化字节全等。
pub fn serialize_rts0(rs: &RootSignature) -> Vec<u8> {
    let payload = serialize_rts0_payload(rs);

    // 外层 DXBC 容器:header(20+4+4+4)+ part 偏移表(1)+ part(fourcc+size+payload)。
    const DXBC_HEADER: u32 = 4 + 16 + 4 + 4 + 4; // fourcc + digest + version + size + partCount
    let part_offset = DXBC_HEADER + 4; // 单 part:偏移表后即 part fourcc
    let part_size = payload.len() as u32;
    let total = part_offset + 4 + 4 + part_size; // partOffset 表(4)... 已含于 part_offset

    let mut buf = Vec::with_capacity(total as usize);
    buf.extend_from_slice(b"DXBC");
    buf.extend_from_slice(&[0u8; 16]); // 摘要占位(真实 DXBC hash 归 PR-E2b)
    push_u32(&mut buf, 1); // 容器版本
    push_u32(&mut buf, total); // 容器总字节
    push_u32(&mut buf, 1); // partCount
    push_u32(&mut buf, part_offset); // partOffsets[0]
    buf.extend_from_slice(b"RTS0"); // part fourcc
    push_u32(&mut buf, part_size); // part 载荷字节
    buf.extend_from_slice(&payload);
    buf
}

/// RTS0 part 载荷(versioned root signature v1.0 序列化形态)。
fn serialize_rts0_payload(rs: &RootSignature) -> Vec<u8> {
    let n = rs.parameters.len() as u32;
    const HEADER: u32 = 6 * 4; // Version/NumParameters/ParametersOffset/NumStaticSamplers/StaticSamplersOffset/Flags
    const PARAM_RECORD: u32 = 3 * 4; // type + visibility + payloadOffset
    const PARAM_STRUCT: u32 = 2 * 4; // CBV(reg,space)与 table(num,rangesOff)均 8 字节
    const RANGE: u32 = 5 * 4; // rangeType+num+base+space+offset

    let params_offset = HEADER;
    let structs_start = params_offset + PARAM_RECORD * n;

    // 一遍:为每个参数算载荷偏移 + 为每个 table 算其 range 数组偏移。
    let mut struct_cursor = structs_start;
    let mut ranges_cursor = structs_start + PARAM_STRUCT * n;
    let mut param_payload_off = Vec::with_capacity(rs.parameters.len());
    let mut table_ranges_off = Vec::with_capacity(rs.parameters.len());
    for p in &rs.parameters {
        param_payload_off.push(struct_cursor);
        struct_cursor += PARAM_STRUCT;
        match p {
            RootParameter::CbvRootDescriptor { .. } => table_ranges_off.push(0),
            RootParameter::DescriptorTable { ranges } => {
                table_ranges_off.push(ranges_cursor);
                ranges_cursor += RANGE * ranges.len() as u32;
            }
        }
    }

    let mut buf = Vec::with_capacity(ranges_cursor as usize);
    // header
    push_u32(&mut buf, RTS0_VERSION_1_0);
    push_u32(&mut buf, n);
    push_u32(&mut buf, params_offset);
    push_u32(&mut buf, 0); // NumStaticSamplers
    push_u32(&mut buf, 0); // StaticSamplersOffset
    push_u32(&mut buf, rs.flags);
    // parameter records
    for (i, p) in rs.parameters.iter().enumerate() {
        let ptype = match p {
            RootParameter::CbvRootDescriptor { .. } => PARAM_TYPE_CBV,
            RootParameter::DescriptorTable { .. } => PARAM_TYPE_DESCRIPTOR_TABLE,
        };
        push_u32(&mut buf, ptype);
        push_u32(&mut buf, SHADER_VISIBILITY_ALL);
        push_u32(&mut buf, param_payload_off[i]);
    }
    // parameter structs
    for (i, p) in rs.parameters.iter().enumerate() {
        match p {
            RootParameter::CbvRootDescriptor { register, space } => {
                push_u32(&mut buf, *register);
                push_u32(&mut buf, *space);
            }
            RootParameter::DescriptorTable { ranges } => {
                push_u32(&mut buf, ranges.len() as u32);
                push_u32(&mut buf, table_ranges_off[i]);
            }
        }
    }
    // descriptor range 数组
    for p in &rs.parameters {
        if let RootParameter::DescriptorTable { ranges } = p {
            for r in ranges {
                push_u32(&mut buf, range_type_code(r.range_type));
                push_u32(&mut buf, r.num_descriptors);
                push_u32(&mut buf, r.base_register);
                push_u32(&mut buf, r.space);
                push_u32(&mut buf, RANGE_OFFSET_APPEND);
            }
        }
    }
    buf
}

// ════════════════ RXS-0166:绑定布局推导一致性校验门 + strict-only ════════════════

/// PSV0 反射出的单个资源绑定(`gate` 后从产物反射读回的绑定占位)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Psv0Resource {
    /// 资源种类轴。
    pub class: ResourceClass,
    /// 轴内 register 基号。
    pub register: u32,
    /// register space。
    pub space: u32,
    /// 连续 register 跨度(单 = 1,有界数组 = n)。
    pub count: u32,
}

/// PSV0 资源绑定反射(产物侧反射读回的绑定集合)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Psv0Reflection {
    /// 反射出的资源绑定集合(顺序无关)。
    pub resources: Vec<Psv0Resource>,
}

/// RXS-0166:绑定布局推导一致性校验门(纯 host/safe,strict-only,不可裁剪)。
///
/// 比对**推导意图**(`intent`,RXS-0164 register/space 分配)与 **PSV0 反射**
/// (`reflected`,产物侧反射读回)的资源绑定:每个推导意图须在反射中以等价
/// (种类轴, register, space, span)出现,且反射不得多出/缺失资源——否则推导意图
/// 未在产物兑现 → strict-only 失败。比对**顺序无关**(按 (class,register,space,count)
/// 搜索,不取容器内排序)。
///
/// register/space 数值为**实现确定、gate 后、非 stable**:本门核实「推导意图 ↔
/// 产物反射」内部一致(无静默漂移),**不**把具体数值冻结为 stable 语言 ABI。
///
/// # Errors
/// 反射缺失 / 多出 / 与意图失配 → [`BindingInferError::Psv0Mismatch`](strict-only;
/// 上层映射 6xxx 并终止该产物,落码归 PR-E2b)。
pub fn check_binding_consistency(
    intent: &[RegisterAssignment],
    reflected: &Psv0Reflection,
) -> Result<(), BindingInferError> {
    // 反射资源数须与推导意图数一致(无多出 / 缺失)。
    if reflected.resources.len() != intent.len() {
        return Err(BindingInferError::Psv0Mismatch {
            detail: format!(
                "PSV0 反射资源数 {} 与推导意图数 {} 不一致",
                reflected.resources.len(),
                intent.len()
            ),
        });
    }
    // 每个推导意图须在反射中以等价 (class,register,space,span) 出现(顺序无关)。
    for a in intent {
        let found = reflected.resources.iter().any(|r| {
            r.class == a.class
                && r.register == a.register
                && r.space == a.space
                && r.count == a.span
        });
        if !found {
            return Err(BindingInferError::Psv0Mismatch {
                detail: format!(
                    "推导意图 `{}`({}{}..+{} space{})未在 PSV0 反射中等价出现",
                    a.name,
                    a.axis_prefix(),
                    a.register,
                    a.span,
                    a.space
                ),
            });
        }
    }
    Ok(())
}

// ════════════════ RXS-0230:Vk-native set-per-class 分配策略 ════════════════
// (E-3:单一 binding-号事实源 + 按目标两套 set 分配策略)。

/// Vk-native 形态 set 轴映射(RXS-0230 L2 / RFC-0013 §4.0-1):
/// `set = 类别轴`(0=CBV / 1=SRV / 2=UAV / 3=Sampler);bindless 无界表自 set4 起
/// (§4.C,本函数不涉及)。
pub fn class_to_vk_set(class: ResourceClass) -> u32 {
    match class {
        ResourceClass::Cbv => 0,
        ResourceClass::Srv => 1,
        ResourceClass::Uav => 2,
        ResourceClass::Sampler => 3,
    }
}

/// RXS-0230(E-3):Vk-native descriptor set 分配策略——**binding 号与
/// [`infer_spirv_bindings`] 同一事实源**(per-class 递增),但 `set = 类别轴`
/// (0=CBV/1=SRV/2=UAV/3=Sampler)而非硬编码 set0。原生 Vulkan 消费下四类轴各占
/// 独立 set,不再 binding 0 互撞(承 binding_layout.rs:144-147 device bug 教训)。
///
/// **B 链形态(`infer_spirv_bindings`)装饰字节不动**(零 golden 重 bless);两套策略
/// 共享同一 binding-号推导(单一事实源,非「一处推导两形态」的含糊)。
///
/// G3.4(RXS-0233):无界 SRV 纹理表在 Vk-native 形态独占 descriptor set **自 set4**
/// 按声明序递增(类别轴 set0~3 之后首个空闲 set,§4.0-1),表内 binding 0。
///
/// **binding 号与 [`infer_spirv_bindings`] 单一事实源**(bounded per-class 递增、
/// unbounded 恒 0);两形态仅 set 分配策略不同(E-3)。
///
/// # Errors
/// 同 [`infer_spirv_bindings`]:无界非-SRV-纹理 → [`BindingInferError::Unmappable`](RD-018)。
pub fn infer_spirv_bindings_vk_native(
    resources: &[ResourceBinding],
) -> Result<Vec<SpirvBinding>, BindingInferError> {
    let mut counters = AxisCounters::default();
    let mut unbounded_ord = 0u32;
    let mut out = Vec::with_capacity(resources.len());
    for r in resources {
        let class = r.res.class();
        match resource_multiplicity(r)? {
            // bounded:set = 类别轴(0=CBV/1=SRV/2=UAV/3=Sampler),binding = 类内序(同 B 链)。
            Multiplicity::Bounded(span) => {
                let binding = counters.take(class, span);
                out.push(SpirvBinding {
                    name: r.name.clone(),
                    class,
                    set: class_to_vk_set(class),
                    binding,
                });
            }
            // unbounded SRV 纹理表:独占 set4+(声明序),binding 0。
            Multiplicity::UnboundedTable => {
                let set = VK_BINDLESS_SET_BASE + unbounded_ord;
                unbounded_ord += 1;
                out.push(SpirvBinding {
                    name: r.name.clone(),
                    class,
                    set,
                    binding: 0,
                });
            }
        }
    }
    Ok(out)
}

// ════════════════ RXS-0224:sampler 状态空间 + 静态 sampler 序列化 ════════════════

/// sampler 过滤模式(min/mag/mip 三合一;RXS-0224 状态空间,与宿主 `SamplerDesc`
/// 〔RXS-0225〕镜像同一状态空间)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerFilter {
    /// 最近邻(point)。
    Nearest,
    /// 线性(min/mag/mip 线性)。
    Linear,
}

/// sampler 寻址模式(RXS-0224)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerAddress {
    /// clamp-to-edge。
    Clamp,
    /// wrap / repeat。
    Wrap,
    /// mirror。
    Mirror,
    /// border(色限三预置)。
    Border,
}

/// 比较函数(仅 `SamplerCmp`;RXS-0224)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerCompare {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// sampler 状态空间(RXS-0224 单一事实源;静态属性 `#[sampler(...)]` 与宿主
/// `SamplerDesc`〔RXS-0225〕镜像同一状态集)。`lod_bias` 钳 [-16,16)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerState {
    /// 过滤模式(min/mag/mip 三合一)。
    pub filter: SamplerFilter,
    /// 寻址模式(UVW 三向同值,首期)。
    pub address: SamplerAddress,
    /// 各向异性(1=off;>1 时 device 探测 `samplerAnisotropy`)。
    pub max_anisotropy: u32,
    /// LOD bias(钳 [-16,16))。
    pub lod_bias: f32,
    /// min LOD。
    pub min_lod: f32,
    /// max LOD。
    pub max_lod: f32,
    /// 比较函数(仅 `SamplerCmp`;`None` = 普通采样)。
    pub compare: Option<SamplerCompare>,
}

impl Default for SamplerState {
    /// 无属性 = 现行静态默认(linear + clamp,RXS-0176 DS4 向后一致)。
    fn default() -> Self {
        SamplerState {
            filter: SamplerFilter::Linear,
            address: SamplerAddress::Clamp,
            max_anisotropy: 1,
            lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: f32::MAX,
            compare: None,
        }
    }
}

impl SamplerState {
    /// 状态合法性(RXS-0224 Legality:`lod_bias` 钳 [-16,16)、`max_anisotropy` ≥ 1)。
    /// 非法状态组合 → `false`(前端并入 RX3014 扩类别,strict-only)。
    pub fn is_valid(&self) -> bool {
        (-16.0..16.0).contains(&self.lod_bias) && self.max_anisotropy >= 1
    }
}

/// D3D12 `D3D12_STATIC_SAMPLER_DESC` 静态 sampler(RXS-0224:`#[sampler(...)]` 常量折叠
/// → RTS0 static sampler,不占 descriptor table 槽位;s 轴与动态 sampler 共序 RXS-0164)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticSamplerDesc {
    /// sampler 状态(常量折叠自 `#[sampler(...)]`)。
    pub state: SamplerState,
    /// `s` 轴 register 基号(与动态 sampler 共声明序,RXS-0164)。
    pub shader_register: u32,
    /// register space(首期 0)。
    pub space: u32,
}

impl StaticSamplerDesc {
    /// 序列化为 `D3D12_STATIC_SAMPLER_DESC`(13 × u32 = 52 字节,确定性;RXS-0224)。
    /// 具体枚举数值取 D3D12 既定常量(实现确定、gate 后、非 stable;🔒 不冻结为 ABI)。
    pub fn serialize(&self) -> [u8; 52] {
        // D3D12_FILTER:MIN_MAG_MIP_POINT=0 / MIN_MAG_MIP_LINEAR=0x15;比较型 +0x80。
        let base_filter = match self.state.filter {
            SamplerFilter::Nearest => 0x0,
            SamplerFilter::Linear => 0x15,
        };
        let filter = if self.state.compare.is_some() {
            base_filter | 0x80 // COMPARISON 位。
        } else {
            base_filter
        };
        // D3D12_TEXTURE_ADDRESS_MODE:WRAP=1 / MIRROR=2 / CLAMP=3 / BORDER=4。
        let addr = match self.state.address {
            SamplerAddress::Wrap => 1,
            SamplerAddress::Mirror => 2,
            SamplerAddress::Clamp => 3,
            SamplerAddress::Border => 4,
        };
        // D3D12_COMPARISON_FUNC:LESS=2 / LESS_EQUAL=4 / GREATER=5 / GREATER_EQUAL=7;
        // 无比较 → NEVER=1(D3D12 static sampler 恒填有效枚举)。
        let cmp = match self.state.compare {
            Some(SamplerCompare::Less) => 2,
            Some(SamplerCompare::LessEqual) => 4,
            Some(SamplerCompare::Greater) => 5,
            Some(SamplerCompare::GreaterEqual) => 7,
            None => 1,
        };
        let mut out = [0u8; 52];
        let mut w = |i: usize, v: u32| out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        w(0, filter);
        w(1, addr); // AddressU
        w(2, addr); // AddressV
        w(3, addr); // AddressW
        w(4, self.state.lod_bias.to_bits());
        w(5, self.state.max_anisotropy);
        w(6, cmp);
        w(7, 0); // BorderColor = OPAQUE_BLACK(0)。
        w(8, self.state.min_lod.to_bits());
        w(9, self.state.max_lod.to_bits());
        w(10, self.shader_register);
        w(11, self.space);
        w(12, SHADER_VISIBILITY_ALL);
        out
    }
}

/// RXS-0224:静态 sampler 经 s 轴与动态 sampler **共声明序**分配 register(RXS-0164);
/// 静态者**不占 descriptor table 槽位**(降级 RTS0 static sampler)。给定动态 sampler
/// 已消费的 s 轴 register 数 `dynamic_sampler_count`,静态 sampler 自其后按 `states`
/// 声明序递增分配。
pub fn assign_static_sampler_registers(
    states: &[SamplerState],
    dynamic_sampler_count: u32,
) -> Vec<StaticSamplerDesc> {
    states
        .iter()
        .enumerate()
        .map(|(i, state)| StaticSamplerDesc {
            state: *state,
            shader_register: dynamic_sampler_count + i as u32,
            space: 0,
        })
        .collect()
}

/// RXS-0224:RTS0 header 的 `NumStaticSamplers` 字段值(现 `serialize_rts0` 恒写 0
/// 的扩展点)。静态 sampler 存在时 = `samplers.len()`,序列化载荷为各
/// [`StaticSamplerDesc::serialize`] 连缀(确定性)。
pub fn serialize_static_samplers(samplers: &[StaticSamplerDesc]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(samplers.len() * 52);
    for s in samplers {
        buf.extend_from_slice(&s.serialize());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::PrimTy;
    use crate::mir::{MirResourceType, ResourceBinding, ResourceClass, ResourceCount};

    // 说明:`MirResourceType`/`ResourceBinding`/`ResourceClass`/`ResourceCount` 亦经
    // 父模块 `use` 引入,此处显式 `use` 以本测试模块自洽(同一 item,无歧义)。

    /// 便捷构造单 descriptor 资源绑定。
    fn rb(name: &str, res: MirResourceType) -> ResourceBinding {
        ResourceBinding {
            name: name.to_owned(),
            res,
            count: ResourceCount::One,
        }
    }

    /// 便捷构造带基数的资源绑定。
    fn rb_n(name: &str, res: MirResourceType, count: ResourceCount) -> ResourceBinding {
        ResourceBinding {
            name: name.to_owned(),
            res,
            count,
        }
    }

    /// 混合资源基线:cbv(CBV)+ tex(SRV)+ samp(Sampler)+ rw(UAV)。
    fn mixed() -> Vec<ResourceBinding> {
        vec![
            rb("cbv", MirResourceType::ConstantBuffer),
            rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
            rb("samp", MirResourceType::Sampler),
            rb("rw", MirResourceType::StructuredBuffer { read_only: false }),
        ]
    }

    // ──────────────── RXS-0163:资源句柄 → SPIR-V 资源绑定降级面 ────────────────

    /// accept:声明序确定性 → DescriptorSet 恒 0,Binding **按资源种类轴(per-class)**
    /// 自 0 递增(RXS-0164;与 register/RTS0 同口径,RFC-0007 对齐)。
    //@ spec: RXS-0163
    #[test]
    fn spirv_bindings_deterministic_by_declaration_order() {
        let bindings = infer_spirv_bindings(&mixed()).expect("混合资源应可推导");
        assert_eq!(bindings.len(), 4);
        // cbv/tex/samp/rw 各为不同种类轴(CBV/SRV/Sampler/UAV),per-class binding 各从 0。
        for b in &bindings {
            assert_eq!(b.set, 0, "首期单 set");
            assert_eq!(
                b.binding, 0,
                "每种类轴首个资源 binding 从 0(per-class,RXS-0164)"
            );
        }
        assert_eq!(bindings[0].class, ResourceClass::Cbv);
        assert_eq!(bindings[1].class, ResourceClass::Srv);
        assert_eq!(bindings[2].class, ResourceClass::Sampler);
        assert_eq!(bindings[3].class, ResourceClass::Uav);
        // 确定性:两次推导字段全等。
        assert_eq!(bindings, infer_spirv_bindings(&mixed()).unwrap());
    }

    /// accept:有界 descriptor 数组占多个连续 binding(per-class 轴内递增)。
    //@ spec: RXS-0163
    #[test]
    fn spirv_bindings_bounded_array_spans_multiple() {
        let resources = vec![
            rb_n(
                "texs",
                MirResourceType::Texture2D(PrimTy::F32),
                ResourceCount::Bounded(3),
            ),
            rb("samp", MirResourceType::Sampler),
        ];
        let bindings = infer_spirv_bindings(&resources).unwrap();
        // texs 占 SRV 轴 binding 0..3;samp 为 Sampler 轴 → 独立从 binding 0 起(per-class)。
        assert_eq!(bindings[0].binding, 0);
        assert_eq!(bindings[1].binding, 0);
    }

    /// accept(G3.4 翻转):无界 SRV 纹理表 `[Texture2D<F>]` = 合法无界表——B 链独占
    /// set1(bounded set0 之后)、binding 0(RXS-0233;自 Unmappable 翻转为合法路)。
    //@ spec: RXS-0233
    #[test]
    fn spirv_bindings_unbounded_srv_texture_is_legal_bindless_set() {
        let resources = vec![
            rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
            rb_n(
                "table",
                MirResourceType::Texture2D(PrimTy::F32),
                ResourceCount::Unbounded,
            ),
        ];
        let b = infer_spirv_bindings(&resources).expect("无界 SRV 纹理表应合法(RXS-0233)");
        // bounded tex:set0 装饰字节不动。
        assert_eq!((b[0].set, b[0].binding), (0, 0));
        // 无界 table:B 链自 set1 独占,binding 0。
        assert_eq!((b[1].set, b[1].binding), (B_CHAIN_BINDLESS_SET_BASE, 0));
        assert_eq!(b[1].class, ResourceClass::Srv);
    }

    /// reject(维持):无界**非-SRV-纹理**表(无界 Sampler)→ Unmappable/RX6013(§8,不新码)。
    //@ spec: RXS-0233
    #[test]
    fn spirv_bindings_unbounded_non_texture_still_unmappable() {
        let resources = vec![rb_n(
            "samps",
            MirResourceType::Sampler,
            ResourceCount::Unbounded,
        )];
        match infer_spirv_bindings(&resources) {
            Err(BindingInferError::Unmappable { .. }) => {}
            other => panic!("无界非纹理应维持 Unmappable(§8),实得 {other:?}"),
        }
    }

    /// accept(G3.4):Vk-native 无界表独占 set4+;RTS0 无界表独占 space1+;有界路
    /// (bounded set0~3 / space0)零漂移;多表按声明序递增(独占分配律,RXS-0233)。
    //@ spec: RXS-0233
    #[test]
    fn bindless_exclusive_set_space_allocation_law() {
        // 混合:1 CBV + 1 有界 SRV + 2 无界 SRV 纹理表 + 1 Sampler + 1 UAV。
        let resources = vec![
            rb("cbv", MirResourceType::ConstantBuffer),
            rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
            rb_n(
                "tableA",
                MirResourceType::Texture2D(PrimTy::F32),
                ResourceCount::Unbounded,
            ),
            rb_n(
                "tableB",
                MirResourceType::Texture2D(PrimTy::F32),
                ResourceCount::Unbounded,
            ),
            rb("samp", MirResourceType::Sampler),
            rb("rw", MirResourceType::StructuredBuffer { read_only: false }),
        ];

        // Vk-native:bounded 类别轴 set0~3;无界表 tableA=set4、tableB=set5(声明序)。
        let vk = infer_spirv_bindings_vk_native(&resources).unwrap();
        let vk_by: std::collections::HashMap<&str, (u32, u32)> = vk
            .iter()
            .map(|b| (b.name.as_str(), (b.set, b.binding)))
            .collect();
        assert_eq!(vk_by["cbv"], (0, 0)); // CBV → set0
        assert_eq!(vk_by["tex"], (1, 0)); // 有界 SRV → set1
        assert_eq!(vk_by["samp"], (3, 0)); // Sampler → set3
        assert_eq!(vk_by["rw"], (2, 0)); // UAV → set2
        assert_eq!(vk_by["tableA"], (VK_BINDLESS_SET_BASE, 0)); // set4
        assert_eq!(vk_by["tableB"], (VK_BINDLESS_SET_BASE + 1, 0)); // set5

        // RTS0/register:bounded 恒 space0;无界表 tableA=space1、tableB=space2。
        let reg = infer_register_assignments(&resources).unwrap();
        let reg_by: std::collections::HashMap<&str, (u32, u32, u32)> = reg
            .iter()
            .map(|a| (a.name.as_str(), (a.register, a.space, a.span)))
            .collect();
        assert_eq!(reg_by["tex"], (0, 0, 1)); // 有界 SRV t0 space0
        assert_eq!(
            reg_by["tableA"],
            (0, RTS0_BINDLESS_SPACE_BASE, UNBOUNDED_DESCRIPTOR_COUNT)
        );
        assert_eq!(
            reg_by["tableB"],
            (0, RTS0_BINDLESS_SPACE_BASE + 1, UNBOUNDED_DESCRIPTOR_COUNT)
        );
        // 独占 space → 推导集无冲突(unbounded 各占独立 space)。
        assert!(detect_register_conflict(&reg).is_ok());

        // root signature:每个无界表 = 独占 descriptor table(单 unbounded SRV range)。
        let rs = infer_root_signature(&resources).unwrap();
        let unbounded_tables: Vec<u32> = rs
            .parameters
            .iter()
            .filter_map(|p| match p {
                RootParameter::DescriptorTable { ranges }
                    if ranges.len() == 1
                        && ranges[0].num_descriptors == UNBOUNDED_DESCRIPTOR_COUNT =>
                {
                    Some(ranges[0].space)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            unbounded_tables,
            vec![1, 2],
            "两无界表独占 space1/space2 各自 table"
        );

        // 确定性:两次推导字段全等。
        assert_eq!(vk, infer_spirv_bindings_vk_native(&resources).unwrap());
        assert_eq!(reg, infer_register_assignments(&resources).unwrap());
    }

    /// 有界路零漂移:纯有界签名的 B 链推导与「加入无界表前」逐字段全等(无界表
    /// 不扰动 bounded 资源 set0/binding,承 §4.C2「B 链字节不动」合入门语义)。
    //@ spec: RXS-0233
    #[test]
    fn bounded_path_zero_drift_when_table_added() {
        let bounded_only = mixed();
        let base = infer_spirv_bindings(&bounded_only).unwrap();
        // 在末尾追加一个无界表后,bounded 前四项的 (set,binding,class) 不变。
        let mut with_table = mixed();
        with_table.push(rb_n(
            "table",
            MirResourceType::Texture2D(PrimTy::F32),
            ResourceCount::Unbounded,
        ));
        let after = infer_spirv_bindings(&with_table).unwrap();
        assert_eq!(&after[..4], &base[..], "bounded 资源 B 链装饰零漂移");
    }

    // ──────────────── RXS-0164:register/space 分配推导 ────────────────

    /// accept:§9 Q-Space=B 按资源种类分轴,各轴自 0 递增,首期单 space0。
    //@ spec: RXS-0164
    #[test]
    fn register_assignments_per_class_axis() {
        // 两个 CBV + 两个 SRV(纹理 + 只读 structured)+ 一个 Sampler + 一个 UAV。
        let resources = vec![
            rb("cb0", MirResourceType::ConstantBuffer),
            rb("cb1", MirResourceType::ConstantBuffer),
            rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
            rb("ro", MirResourceType::StructuredBuffer { read_only: true }),
            rb("samp", MirResourceType::Sampler),
            rb("rw", MirResourceType::StructuredBuffer { read_only: false }),
        ];
        let a = infer_register_assignments(&resources).unwrap();
        // CBV 轴 b0/b1。
        assert_eq!((a[0].axis_prefix(), a[0].register, a[0].space), ('b', 0, 0));
        assert_eq!((a[1].axis_prefix(), a[1].register, a[1].space), ('b', 1, 0));
        // SRV 轴 t0/t1。
        assert_eq!((a[2].axis_prefix(), a[2].register), ('t', 0));
        assert_eq!((a[3].axis_prefix(), a[3].register), ('t', 1));
        // Sampler 轴 s0。
        assert_eq!((a[4].axis_prefix(), a[4].register), ('s', 0));
        // UAV 轴 u0。
        assert_eq!((a[5].axis_prefix(), a[5].register), ('u', 0));
        // 确定性。
        assert_eq!(a, infer_register_assignments(&resources).unwrap());
    }

    /// accept:有界数组占连续 register,后继同轴资源接其后。
    //@ spec: RXS-0164
    #[test]
    fn register_assignments_bounded_array_consumes_span() {
        let resources = vec![
            rb_n(
                "texs",
                MirResourceType::Texture2D(PrimTy::F32),
                ResourceCount::Bounded(4),
            ),
            rb("tex2", MirResourceType::Texture2D(PrimTy::F32)),
        ];
        let a = infer_register_assignments(&resources).unwrap();
        assert_eq!((a[0].register, a[0].span), (0, 4));
        assert_eq!(a[1].register, 4, "有界数组后同轴资源接其后");
        // 推导自身无冲突。
        assert!(detect_register_conflict(&a).is_ok());
    }

    /// reject:同轴 + 同 space register 区间重叠 → RegisterConflict(strict-only)。
    //@ spec: RXS-0164
    #[test]
    fn register_conflict_detected() {
        // 手构两个 SRV 同占 t0(模拟篡改 / 未来显式覆盖冲突)。
        let conflicting = vec![
            RegisterAssignment {
                name: "a".to_owned(),
                class: ResourceClass::Srv,
                register: 0,
                space: 0,
                span: 2,
            },
            RegisterAssignment {
                name: "b".to_owned(),
                class: ResourceClass::Srv,
                register: 1, // 落入 a 的 [0,2) 区间。
                space: 0,
                span: 1,
            },
        ];
        match detect_register_conflict(&conflicting) {
            Err(BindingInferError::RegisterConflict { .. }) => {}
            other => panic!("区间重叠应 RegisterConflict,实得 {other:?}"),
        }
    }

    /// accept(ABI 中立旁证):不同轴同 register 不冲突(b0 与 t0 互不干扰)。
    //@ spec: RXS-0164
    #[test]
    fn register_no_conflict_across_axes() {
        let a = infer_register_assignments(&mixed()).unwrap();
        assert!(
            detect_register_conflict(&a).is_ok(),
            "不同种类轴同号不构成冲突"
        );
    }

    /// reject:unbounded → Unmappable(RD-018)。
    //@ spec: RXS-0164
    #[test]
    fn register_assignments_unbounded_is_unmappable() {
        let resources = vec![rb_n(
            "heap",
            MirResourceType::Sampler,
            ResourceCount::Unbounded,
        )];
        assert!(matches!(
            infer_register_assignments(&resources),
            Err(BindingInferError::Unmappable { .. })
        ));
    }

    // ──────────────── RXS-0165:root signature 形态 + RTS0 序列化 ────────────────

    /// accept:§9 Q-RootShape=B 形态——CBV root descriptor + SRV/UAV 表 + Sampler 表。
    //@ spec: RXS-0165
    #[test]
    fn root_signature_shape_q_rootshape_b() {
        let rs = infer_root_signature(&mixed()).unwrap();
        // 参数序:CBV root descriptor → SRV/UAV 表 → Sampler 表。
        assert_eq!(rs.parameters.len(), 3);
        assert!(matches!(
            rs.parameters[0],
            RootParameter::CbvRootDescriptor {
                register: 0,
                space: 0
            }
        ));
        // SRV/UAV 合表:SRV range 先于 UAV range。
        match &rs.parameters[1] {
            RootParameter::DescriptorTable { ranges } => {
                assert_eq!(ranges.len(), 2);
                assert_eq!(ranges[0].range_type, ResourceClass::Srv);
                assert_eq!(ranges[1].range_type, ResourceClass::Uav);
            }
            other => panic!("第二参数应为 SRV/UAV descriptor table,实得 {other:?}"),
        }
        // Sampler 独表。
        match &rs.parameters[2] {
            RootParameter::DescriptorTable { ranges } => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].range_type, ResourceClass::Sampler);
            }
            other => panic!("第三参数应为 Sampler descriptor table,实得 {other:?}"),
        }
        // 成本:1 CBV(2)+ 2 表(2)= 4 DWORD,远低于 64。
        assert_eq!(root_signature_cost_dwords(&rs), 4);
    }

    /// accept:RTS0 序列化确定性 + DXBC/RTS0 容器结构 + 载荷头可解码回参数计数。
    //@ spec: RXS-0165
    #[test]
    fn rts0_serialization_deterministic_and_structured() {
        let rs = infer_root_signature(&mixed()).unwrap();
        let bytes = serialize_rts0(&rs);
        // 确定性:两次序列化字节全等。
        assert_eq!(bytes, serialize_rts0(&rs));
        // 外层 DXBC 容器 fourcc。
        assert_eq!(&bytes[0..4], b"DXBC");
        // partCount = 1(偏移 28)。
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 1);
        // partOffsets[0](偏移 32)指向 RTS0 part fourcc。
        let part_off = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
        assert_eq!(&bytes[part_off..part_off + 4], b"RTS0");
        // RTS0 载荷头:Version=1.0 + NumParameters=3。
        let payload = part_off + 8; // fourcc(4) + partSize(4)
        assert_eq!(
            u32::from_le_bytes(bytes[payload..payload + 4].try_into().unwrap()),
            RTS0_VERSION_1_0
        );
        assert_eq!(
            u32::from_le_bytes(bytes[payload + 4..payload + 8].try_into().unwrap()),
            rs.parameters.len() as u32
        );
        // 容器总长字段(偏移 24)与实际字节数一致。
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize,
            bytes.len()
        );
    }

    /// reject:root signature 推导超 64 DWORD 上限 → RootSignatureTooLarge。
    //@ spec: RXS-0165
    #[test]
    fn root_signature_over_64_dwords_rejected() {
        // 33 个 CBV root descriptor × 2 DWORD = 66 DWORD > 64。
        let resources: Vec<ResourceBinding> = (0..33)
            .map(|i| rb(&format!("cb{i}"), MirResourceType::ConstantBuffer))
            .collect();
        match infer_root_signature(&resources) {
            Err(BindingInferError::RootSignatureTooLarge { dwords, limit }) => {
                assert_eq!(dwords, 66);
                assert_eq!(limit, ROOT_SIGNATURE_DWORD_LIMIT);
            }
            other => panic!("超 64 DWORD 应 RootSignatureTooLarge,实得 {other:?}"),
        }
    }

    // ──────────────── RXS-0166:一致性校验门 + strict-only ────────────────

    /// 由推导意图程序化合成保真 PSV0 反射(by-construction 一致)。
    fn reflect_faithful(intent: &[RegisterAssignment]) -> Psv0Reflection {
        Psv0Reflection {
            resources: intent
                .iter()
                .map(|a| Psv0Resource {
                    class: a.class,
                    register: a.register,
                    space: a.space,
                    count: a.span,
                })
                .collect(),
        }
    }

    /// accept:PSV0 反射与推导意图一致(顺序无关)→ Ok。
    //@ spec: RXS-0166
    #[test]
    fn binding_consistency_faithful_passes() {
        let intent = infer_register_assignments(&mixed()).unwrap();
        let mut reflected = reflect_faithful(&intent);
        reflected.resources.reverse(); // 顺序无关。
        assert!(check_binding_consistency(&intent, &reflected).is_ok());
    }

    /// reject:PSV0 反射 register 与推导意图失配 → Psv0Mismatch(strict-only)。
    //@ spec: RXS-0166
    #[test]
    fn binding_consistency_register_mismatch_rejected() {
        let intent = infer_register_assignments(&mixed()).unwrap();
        let mut reflected = reflect_faithful(&intent);
        reflected.resources[0].register += 7; // 篡改一个 register。
        match check_binding_consistency(&intent, &reflected) {
            Err(BindingInferError::Psv0Mismatch { .. }) => {}
            other => panic!("register 失配应 Psv0Mismatch,实得 {other:?}"),
        }
    }

    /// reject:PSV0 反射资源数与推导意图不一致(缺失 / 多出)→ Psv0Mismatch。
    //@ spec: RXS-0166
    #[test]
    fn binding_consistency_count_mismatch_rejected() {
        let intent = infer_register_assignments(&mixed()).unwrap();
        let mut reflected = reflect_faithful(&intent);
        reflected.resources.pop(); // 反射缺一个资源。
        assert!(matches!(
            check_binding_consistency(&intent, &reflected),
            Err(BindingInferError::Psv0Mismatch { .. })
        ));
    }

    // ── RXS-0230:Vk-native set-per-class 两套 set 分配策略(E-3) ──

    /// RXS-0230:Vk-native 形态 set = 类别轴(0=CBV/1=SRV/2=UAV/3=Sampler),binding 号
    /// 与 B 链**同一事实源**(单一 binding-号 + 两套 set 策略)。
    #[test]
    fn vk_native_set_per_class_shares_binding_source() {
        //@ spec: RXS-0230
        let resources = vec![
            rb("cbv", MirResourceType::ConstantBuffer),
            rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
            rb("rw", MirResourceType::TextureRw2D(PrimTy::F32)),
            rb("samp", MirResourceType::Sampler),
        ];
        let b_chain = infer_spirv_bindings(&resources).expect("B 链推导");
        let vk = infer_spirv_bindings_vk_native(&resources).expect("Vk-native 推导");

        // B 链形态:set 恒 0(字节不动,零 golden 重 bless)。
        assert!(b_chain.iter().all(|b| b.set == 0), "B 链 set 恒 0");

        // Vk-native 形态:set = 类别轴。
        let sets: Vec<(String, u32)> = vk.iter().map(|b| (b.name.clone(), b.set)).collect();
        assert_eq!(
            sets,
            vec![
                ("cbv".to_owned(), 0),  // CBV → set 0
                ("tex".to_owned(), 1),  // SRV → set 1
                ("rw".to_owned(), 2),   // UAV(TextureRw2D)→ set 2
                ("samp".to_owned(), 3)  // Sampler → set 3
            ]
        );

        // 单一 binding-号事实源:两形态 binding 号逐一相等(仅 set 策略不同)。
        for (bc, v) in b_chain.iter().zip(vk.iter()) {
            assert_eq!(bc.name, v.name);
            assert_eq!(bc.binding, v.binding, "binding 号两策略同源");
            assert_eq!(bc.class, v.class);
        }
    }

    // ── RXS-0224:sampler 状态空间 + 静态 sampler(#[sampler(...)] 常量折叠) ──

    /// RXS-0224:sampler 状态空间合法性(lod_bias 钳 [-16,16)、max_anisotropy ≥ 1)。
    #[test]
    fn sampler_state_validity() {
        //@ spec: RXS-0224
        assert!(SamplerState::default().is_valid(), "默认 linear+clamp 合法");
        let bad_bias = SamplerState {
            lod_bias: 16.0, // 越界(钳 [-16,16))。
            ..SamplerState::default()
        };
        assert!(!bad_bias.is_valid(), "lod_bias 越界应非法");
        let bad_aniso = SamplerState {
            max_anisotropy: 0,
            ..SamplerState::default()
        };
        assert!(!bad_aniso.is_valid(), "max_anisotropy 0 应非法");
    }

    /// RXS-0224:静态 sampler s 轴与动态 sampler 共声明序(RXS-0164)+ NumStaticSamplers
    /// 序列化确定性 + 不占 descriptor table 槽位。
    #[test]
    fn static_sampler_shares_s_axis_and_serializes() {
        //@ spec: RXS-0224
        // 场景:1 个动态 sampler(消费 s0)+ 2 个静态 sampler → 静态自 s1/s2。
        let statics = vec![
            SamplerState {
                filter: SamplerFilter::Nearest,
                address: SamplerAddress::Wrap,
                ..SamplerState::default()
            },
            SamplerState {
                compare: Some(SamplerCompare::LessEqual),
                ..SamplerState::default()
            },
        ];
        let descs = assign_static_sampler_registers(&statics, /*dynamic_sampler_count=*/ 1);
        assert_eq!(descs.len(), 2);
        assert_eq!(descs[0].shader_register, 1, "静态 sampler 自 s1(动态占 s0)");
        assert_eq!(descs[1].shader_register, 2, "第二静态 sampler s2");

        // NumStaticSamplers 序列化:2 × 52 字节,确定性(两次逐字节一致)。
        let bytes = serialize_static_samplers(&descs);
        assert_eq!(bytes.len(), 2 * 52, "每 static sampler 52 字节");
        assert_eq!(bytes, serialize_static_samplers(&descs), "序列化确定性");

        // 静态者不占 descriptor table 槽位:仅动态 sampler 入 root signature table。
        // (root signature 推导只消费 resources 中的动态 sampler;静态 sampler 走 RTS0
        //  static sampler 段,不产生 DescriptorTable range。)
        let rs = infer_root_signature(&[rb("dyn_samp", MirResourceType::Sampler)])
            .expect("单动态 sampler root sig");
        let sampler_tables = rs
            .parameters
            .iter()
            .filter(|p| {
                matches!(p, RootParameter::DescriptorTable { ranges }
                if ranges.iter().any(|r| r.range_type == ResourceClass::Sampler))
            })
            .count();
        assert_eq!(sampler_tables, 1, "仅动态 sampler 入 table;静态不占槽位");

        // 比较型静态 sampler 序列化含 COMPARISON filter 位(0x80)。
        let cmp_filter = u32::from_le_bytes(bytes[52..56].try_into().unwrap());
        assert!(
            cmp_filter & 0x80 != 0,
            "SamplerCmp 静态 sampler filter 含比较位"
        );
    }
}
