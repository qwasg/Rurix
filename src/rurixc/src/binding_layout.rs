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

use crate::mir::{ResourceBinding, ResourceClass, ResourceCount};

/// 绑定布局推导失败(strict-only;RFC-0005 §4 / P-01,无运行期 fallback)。
///
/// **错误码占位「6xxx」(判档点,落码归 PR-E2b / owner)**:本枚举只定义推导失败的
/// 类型化语义,**不**直接发码、**不**改 `registry/error_codes.json`、**不**接线生产
/// emit。各变体最终 6xxx 段位由 owner 在 PR-E2b 按真实可达类别裁定(避开 RX6014
/// 与 RXS-0160 争号);`Unmappable` 计划复用 RX6013 `codegen.dxil_unmappable`
/// (PR-E1 §3),其余为新真实可达类别新开码——均归 PR-E2b。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingInferError {
    /// 资源不可映射为合规有界绑定(bindless / unbounded descriptor array →
    /// RD-018 defer;或资源种类不可降级)。strict-only 拒绝,不发明 descriptor
    /// heap 编码(占位「6xxx」,计划复用 RX6013)。
    Unmappable {
        /// 不可映射构造的诊断上下文(资源名 / 种类 / 基数)。
        detail: String,
    },
    /// register/layout 冲突:两个资源占同一(种类轴, register, space)。
    /// strict-only 拒绝,无 fallback(占位「6xxx」新开码,PR-E2b)。
    RegisterConflict {
        /// 冲突的诊断上下文(两端资源名 / 轴 / register / space)。
        detail: String,
    },
    /// root signature 推导超 D3D12 64 DWORD 上限。strict-only 拒绝(占位「6xxx」
    /// 新开码,PR-E2b)。
    RootSignatureTooLarge {
        /// 推导出的 DWORD 成本。
        dwords: u32,
        /// D3D12 上限(64 DWORD)。
        limit: u32,
    },
    /// PSV0 反射与推导意图不一致(不可推导 / 篡改 / mismatch)。strict-only 拒绝
    /// (占位「6xxx」新开码,PR-E2b)。
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
    /// `DescriptorSet` 装饰(首期单 set,恒 0)。
    pub set: u32,
    /// `Binding` 装饰(按声明序确定性递增)。
    pub binding: u32,
}

/// RXS-0163:资源句柄 → SPIR-V 资源绑定降级面推导(纯 host/safe,确定性)。
///
/// 按 `resources` 声明序(io_sig 顺序)确定性导出每个资源的 SPIR-V 绑定装饰意图:
/// `DescriptorSet = 0`(首期单 set)、`Binding` 自 0 起按声明序递增(有界数组占
/// `count` 个连续 binding)。
///
/// # Errors
/// `Unbounded`(bindless / unbounded descriptor array)→ [`BindingInferError::Unmappable`]
/// (RD-018 defer,strict-only,不发明 descriptor heap 编码)。
pub fn infer_spirv_bindings(
    resources: &[ResourceBinding],
) -> Result<Vec<SpirvBinding>, BindingInferError> {
    let mut out = Vec::with_capacity(resources.len());
    let mut next_binding: u32 = 0;
    for r in resources {
        let span = descriptor_span(r)?;
        out.push(SpirvBinding {
            name: r.name.clone(),
            class: r.res.class(),
            set: 0,
            binding: next_binding,
        });
        next_binding += span;
    }
    Ok(out)
}

/// 资源消费的连续 descriptor / register 跨度(有界基数;`Unbounded` → 不可映射)。
fn descriptor_span(r: &ResourceBinding) -> Result<u32, BindingInferError> {
    match r.count {
        ResourceCount::One => Ok(1),
        ResourceCount::Bounded(n) if n >= 1 => Ok(n),
        ResourceCount::Bounded(_) => Err(BindingInferError::Unmappable {
            detail: format!("资源 `{}` 有界数组基数为 0(非法)", r.name),
        }),
        ResourceCount::Unbounded => Err(BindingInferError::Unmappable {
            detail: format!(
                "资源 `{}` 为 unbounded / bindless descriptor array(RD-018 defer,本期不推导)",
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
/// `Unbounded` / 非法基数 → [`BindingInferError::Unmappable`](RD-018,strict-only)。
pub fn infer_register_assignments(
    resources: &[ResourceBinding],
) -> Result<Vec<RegisterAssignment>, BindingInferError> {
    let mut counters = AxisCounters::default();
    let mut out = Vec::with_capacity(resources.len());
    for r in resources {
        let span = descriptor_span(r)?;
        let class = r.res.class();
        let register = counters.take(class, span);
        out.push(RegisterAssignment {
            name: r.name.clone(),
            class,
            register,
            space: 0,
            span,
        });
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
fn ranges_overlap(a: &RegisterAssignment, b: &RegisterAssignment) -> bool {
    let a_end = a.register + a.span;
    let b_end = b.register + b.span;
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

/// 把某轴的全部分配聚合为单个 descriptor range(无该轴资源 → `None`)。
fn axis_range(assignments: &[RegisterAssignment], class: ResourceClass) -> Option<DescriptorRange> {
    let mut total = 0u32;
    let mut base = u32::MAX;
    for a in assignments.iter().filter(|a| a.class == class) {
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

    /// accept:声明序确定性 → DescriptorSet 恒 0,Binding 自 0 递增。
    //@ spec: RXS-0163
    #[test]
    fn spirv_bindings_deterministic_by_declaration_order() {
        let bindings = infer_spirv_bindings(&mixed()).expect("混合资源应可推导");
        assert_eq!(bindings.len(), 4);
        for (i, b) in bindings.iter().enumerate() {
            assert_eq!(b.set, 0, "首期单 set");
            assert_eq!(b.binding, i as u32, "binding 按声明序递增");
        }
        assert_eq!(bindings[0].class, ResourceClass::Cbv);
        assert_eq!(bindings[1].class, ResourceClass::Srv);
        assert_eq!(bindings[2].class, ResourceClass::Sampler);
        assert_eq!(bindings[3].class, ResourceClass::Uav);
        // 确定性:两次推导字段全等。
        assert_eq!(bindings, infer_spirv_bindings(&mixed()).unwrap());
    }

    /// accept:有界 descriptor 数组占多个连续 binding。
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
        assert_eq!(bindings[0].binding, 0);
        // texs 占 binding 0..3 → samp 起于 binding 3。
        assert_eq!(bindings[1].binding, 3);
    }

    /// reject:unbounded / bindless → Unmappable(RD-018 defer,strict-only)。
    //@ spec: RXS-0163
    #[test]
    fn spirv_bindings_unbounded_is_unmappable() {
        let resources = vec![rb_n(
            "heap",
            MirResourceType::Texture2D(PrimTy::F32),
            ResourceCount::Unbounded,
        )];
        match infer_spirv_bindings(&resources) {
            Err(BindingInferError::Unmappable { .. }) => {}
            other => panic!("unbounded 应 Unmappable(RD-018),实得 {other:?}"),
        }
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
}
