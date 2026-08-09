//! `capability_check` — M32 capability profile 硬门
//! (G8.2 硬门 `g8.p0.m32.capability_profile`;RFC-0019 §4.5;
//! spec/shader_stages.md RXS-0311 + spec/rendering_platform.md RXS-0312/0313)。
//! 纯 host、safe。
//!
//! 五子面:
//!
//! - **capability ID 闭集**(RXS-0311,v1 冻结十项 + G9.2 加性两位实位,
//!   RXS-0349):`rt.pipeline` /
//!   `rt.sbt_user_data` / `rt.any_hit` / `rt.intersection.procedural` /
//!   `rt.callable` / `rt.ray_query` / `mesh.task` / `sync.timeline_semaphore` /
//!   `queue.dedicated_transfer` / `queue.dedicated_compute` / `submit.dgc` /
//!   `bindless.descriptor_buffer`。backend extension 名
//!   不作为 ID(RFC-0019 §4.5.1 逐字)。
//! - **`#[requires("id", ...)]` 声明面**(RXS-0311):fn 级 attr,字符串字面量
//!   列表,多条可叠加取并集;闭集外 ID / 非字符串实参 / 空列表 → **RX3023**
//!   `capability.unknown_id`(加性冻结第五 key);附着非函数 item、host-only
//!   函数(host/const 着色)或非 device 着色关联函数 → 同类拒。经
//!   [`crate::shader_stages::check`] 挂接(resolve 后 typeck 前)。
//! - **隐式推导 + 调用图并集**(RXS-0311):intrinsic 映射(trace_ray →
//!   rt.pipeline;RayQuery 局部 / ray_query_initialize → rt.ray_query;
//!   report_intersection → rt.intersection.procedural;execute_callable →
//!   rt.callable;ignore_intersection → rt.any_hit)+ stage 映射(raygen/miss/
//!   closesthit → rt.pipeline,anyhit → rt.any_hit,intersection →
//!   rt.intersection.procedural,callable → rt.callable,task → mesh.task)+
//!   形参映射(`#[shader_record]` → rt.sbt_user_data;`AccelStruct` compute
//!   签名形参 → rt.ray_query)。entry 有效 requirement 集 = 显式 ∪ 隐式 ∪
//!   全部静态可达 device callee 有效集之并;可达性以 resolve 的 DefId 级
//!   call facts 为准(路径调用经 `path_res` → `Res::Def(DefId)`,**禁名字
//!   匹配**;v1 诚实边界:固有 impl 方法调用的接收者类型解析在 TBIR 期,
//!   不在 AST 调用事实面)。泛型着色函数不产 entry(RXS-0304 口径),其
//!   requirement 随单态化并入调用方(调用图照常传播)。
//! - **profile 闭集与构建期选择律**(RXS-0312):profile v1 JSON(字段闭集
//!   `{schema: "rurix.profile.v1", name, version, required[], optional[],
//!   forbidden[], fallbacks: {}}`;三集两两不相交否则拒;闭集外 ID 拒
//!   (RX3023 同类);其余形态非法 = RX7001 工具段)。canonical bytes 沿
//!   CanonW 律(版本前缀 + name/version length-prefix + 三集字节序 +
//!   fallbacks 按键字节序);`selected_profile_digest = SHA-256(
//!   "rurix.profile.v1\0" || canonical 去前缀段)`(本实现 canonical 字节
//!   自带前缀,digest = SHA-256(完整 canonical 字节),与 spec 定义逐字节
//!   等价);**无 `--profile` 恒** `SHA-256("rurix.profile-none.v1\0")`
//!   (RXS-0304 空编码 0 漂移)。选择律(每 entry 独立):有效集 ∩
//!   forbidden ≠ ∅ → **RX3021** `capability.forbidden_used`;有效集 ⊆
//!   required ∪ optional → 发射;缺能力:fallbacks 有映射且接口契约兼容 →
//!   选 fallback(主 variant 不发射,fallback 自身递归判定,链深度 1);
//!   无映射 → **RX3020** `capability.missing_required`(消息携带缺失 ID +
//!   首个引入它的可达 callee);不兼容 → **RX3022**
//!   `capability.fallback_incompatible`(消息给出不兼容字段)。
//! - **fallback 接口契约兼容判定**(RXS-0312 v1.3 精确化):stage 相同 +
//!   `io`/`push_constants`/`execution_modes` 结构相等;`resources` 按声明
//!   序比对(主表 class == `accel` 且缺失能力 ∈ {rt.ray_query, rt.pipeline}
//!   的条目允许 fallback 缺席;其余逐项相等、相对序一致;fallback 不得多
//!   出条目;缺席条目之后的 binding 号按 fallback 自身声明序独立推导,不
//!   构成不兼容)。接口事实提取 = reflection 同一提取律(单一事实源)。
//! - **运行期 snapshot 核验原语**(RXS-0313):[`verify_profile_snapshot`]
//!   host/safe 纯函数(镜像 RXS-0307 `verify_interface_pair` 体例):产物
//!   profile 事实(digest + required 集)对照 snapshot 可用集,缺失 →
//!   typed `Err` 携带缺失 ID 表与 symbolic key 字面
//!   `capability.runtime_snapshot_mismatch`(库层 typed Err,不占 RX 码);
//!   函数体内不存在任何修复/重编/换 profile 路径(by construction)。
//!   M32 只落该 host 原语与单测;device 腿消费归 M50/M89。
//!
//! SHA-256 复用 `rurix-pkg` 手写实现(RXS-0306 同源);编码沿 RXS-0305
//! CanonW 律(u32 LE 定宽、length-prefix UTF-8 字符串、u32 计数列表)。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use crate::ast::{self, FnColor, LitKind, MetaInner, MetaKind, ShaderStage};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{DefId, DefKind, Res};
use crate::iface_extract as iface;
use crate::resolve::Resolutions;
use crate::span::Span;
use rurix_pkg::sha256;

/// RX3020(RXS-0312;capability 必需能力缺失,typeck 段)。
pub const E_CAP_MISSING: ErrorCode = ErrorCode(3020);
/// RX3021(RXS-0312;capability 违禁能力使用,typeck 段)。
pub const E_CAP_FORBIDDEN: ErrorCode = ErrorCode(3021);
/// RX3022(RXS-0312;capability fallback 接口契约不兼容,typeck 段)。
pub const E_CAP_FALLBACK_INCOMPAT: ErrorCode = ErrorCode(3022);
/// RX3023(RXS-0311;capability ID 闭集外引用 / `#[requires]` 附着违例,typeck 段)。
pub const E_CAP_UNKNOWN_ID: ErrorCode = ErrorCode(3023);

/// profile canonical bytes 版本前缀(RXS-0312)。
const PROFILE_V1: &[u8] = b"rurix.profile.v1\0";
/// 无 `--profile` 的规范 digest 定义域(RXS-0304 空编码;与 M31 既有常量逐字节
/// 一致,0 漂移)。
const PROFILE_NONE_DOMAIN: &[u8] = b"rurix.profile-none.v1\0";

/// profile v1 schema 常量(RXS-0312 字段闭集)。
pub const PROFILE_SCHEMA_ID: &str = "rurix.profile.v1";
/// selection manifest 产物 schema 标识(RXS-0312 报告律)。
pub const MANIFEST_SCHEMA_ID: &str = "rurix.capability-selection.v1";
/// 运行期 snapshot 核验失败的 symbolic key 字面(RFC-0019 §4.5.1 冻结四键之一;
/// 库层 typed Err 文本携带,不占 RX 数字码,RXS-0313)。
pub const SNAPSHOT_MISMATCH_KEY: &str = "capability.runtime_snapshot_mismatch";

/// 无 `--profile` 的规范 digest(RXS-0304/0312 空编码 0 漂移;M31 基线常量)。
pub fn profile_none_digest() -> [u8; 32] {
    sha256::digest(PROFILE_NONE_DOMAIN)
}

// ═══════════════════════ capability ID 闭集(RXS-0311) ═══════════════════════

/// capability ID 闭集(v1 冻结十项 + G9.2 加性两位实位,RXS-0311/RXS-0349)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CapabilityId {
    /// `rt.pipeline` — RT pipeline 六执行模型(raygen/miss/closesthit 与 trace_ray)。
    RtPipeline,
    /// `rt.sbt_user_data` — SBT typed shader-record 用户数据(`#[shader_record]`,M50)。
    RtSbtUserData,
    /// `rt.any_hit` — any-hit 阶段与 ignore_intersection。
    RtAnyHit,
    /// `rt.intersection.procedural` — procedural intersection 阶段与 report_intersection。
    RtIntersectionProcedural,
    /// `rt.callable` — callable 阶段与 execute_callable。
    RtCallable,
    /// `rt.ray_query` — compute 内联遍历(RayQuery/ray_query_initialize,RXS-0297~0299)。
    RtRayQuery,
    /// `mesh.task` — task 阶段(mesh 前置放大)。
    MeshTask,
    /// `sync.timeline_semaphore` — timeline semaphore 同步原语(M59 前置)。
    SyncTimelineSemaphore,
    /// `queue.dedicated_transfer` — 独立 transfer queue class。
    QueueDedicatedTransfer,
    /// `queue.dedicated_compute` — 独立 compute queue class。
    QueueDedicatedCompute,
    /// `submit.dgc` — DGC device-generated commands 抽象面(M102,RXS-0348/0349;
    /// `VK_EXT_device_generated_commands` 的 capability 门控 ID)。
    SubmitDgc,
    /// `bindless.descriptor_buffer` — `VK_EXT_descriptor_buffer` 单一大表(M103,
    /// RXS-0347 索引空间预算的 profile 承载位)。
    BindlessDescriptorBuffer,
}

impl CapabilityId {
    /// 冻结字符串字面(RXS-0311 闭集表逐字)。
    pub fn name(self) -> &'static str {
        match self {
            CapabilityId::RtPipeline => "rt.pipeline",
            CapabilityId::RtSbtUserData => "rt.sbt_user_data",
            CapabilityId::RtAnyHit => "rt.any_hit",
            CapabilityId::RtIntersectionProcedural => "rt.intersection.procedural",
            CapabilityId::RtCallable => "rt.callable",
            CapabilityId::RtRayQuery => "rt.ray_query",
            CapabilityId::MeshTask => "mesh.task",
            CapabilityId::SyncTimelineSemaphore => "sync.timeline_semaphore",
            CapabilityId::QueueDedicatedTransfer => "queue.dedicated_transfer",
            CapabilityId::QueueDedicatedCompute => "queue.dedicated_compute",
            CapabilityId::SubmitDgc => "submit.dgc",
            CapabilityId::BindlessDescriptorBuffer => "bindless.descriptor_buffer",
        }
    }

    /// 字符串 → 闭集成员(闭集外 → None = RX3023 拒)。
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "rt.pipeline" => CapabilityId::RtPipeline,
            "rt.sbt_user_data" => CapabilityId::RtSbtUserData,
            "rt.any_hit" => CapabilityId::RtAnyHit,
            "rt.intersection.procedural" => CapabilityId::RtIntersectionProcedural,
            "rt.callable" => CapabilityId::RtCallable,
            "rt.ray_query" => CapabilityId::RtRayQuery,
            "mesh.task" => CapabilityId::MeshTask,
            "sync.timeline_semaphore" => CapabilityId::SyncTimelineSemaphore,
            "queue.dedicated_transfer" => CapabilityId::QueueDedicatedTransfer,
            "queue.dedicated_compute" => CapabilityId::QueueDedicatedCompute,
            "submit.dgc" => CapabilityId::SubmitDgc,
            "bindless.descriptor_buffer" => CapabilityId::BindlessDescriptorBuffer,
            _ => return None,
        })
    }

    /// 闭集全表(冻结序 = 条款表序;RXS-0349 G9.2 加性两位实位居尾)。
    pub const ALL: [CapabilityId; 12] = [
        CapabilityId::RtPipeline,
        CapabilityId::RtSbtUserData,
        CapabilityId::RtAnyHit,
        CapabilityId::RtIntersectionProcedural,
        CapabilityId::RtCallable,
        CapabilityId::RtRayQuery,
        CapabilityId::MeshTask,
        CapabilityId::SyncTimelineSemaphore,
        CapabilityId::QueueDedicatedTransfer,
        CapabilityId::QueueDedicatedCompute,
        CapabilityId::SubmitDgc,
        CapabilityId::BindlessDescriptorBuffer,
    ];
}

/// 排序一律按 ID 字符串字节序(RXS-0304「排序 ID 表」/RXS-0312 三集编码律)。
impl Ord for CapabilityId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name().as_bytes().cmp(other.name().as_bytes())
    }
}

impl PartialOrd for CapabilityId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// ID 集 → 排序字符串表(字节序;报告/reflection 渲染面)。
fn sorted_names(set: &BTreeSet<CapabilityId>) -> Vec<String> {
    set.iter().map(|c| c.name().to_owned()).collect()
}

fn join_names(set: &BTreeSet<CapabilityId>) -> String {
    set.iter()
        .map(|c| format!("`{}`", c.name()))
        .collect::<Vec<_>>()
        .join(", ")
}

// ═══════════════════════ `#[requires]` 声明面(RXS-0311) ═══════════════════════

/// `#[requires]` 解析/附着违例(RX3023 载体;fail-closed)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CapInvalid {
    /// 诊断上下文({detail} 参数)。
    pub detail: String,
    /// 诊断锚点(违例属性/实参 span)。
    pub span: Span,
}

fn invalid(detail: String, span: Span) -> CapInvalid {
    CapInvalid { detail, span }
}

/// 单段路径名(非单段 → None)。
fn single_seg(p: &ast::Path) -> Option<&str> {
    match p.segments.as_slice() {
        [seg] => Some(seg.ident.name.as_str()),
        _ => None,
    }
}

/// 字符串字面量的源文本值(剥引号;capability ID 无转义序列,源切片即真值)。
fn lit_str<'a>(src: &'a str, lit: &ast::Lit) -> Option<&'a str> {
    if lit.kind != LitKind::Str {
        return None;
    }
    let text = src.get(lit.span.lo.0 as usize..lit.span.hi.0 as usize)?;
    text.strip_prefix('"')?.strip_suffix('"')
}

/// 自 fn 属性表提取显式 capability 声明(RXS-0311):`#[requires("id", ...)]`
/// 多条可叠加取**并集**;无标注 → Ok(空集)。闭集外 ID / 非列表形态 / 非
/// 字符串实参 / 空实参表 → Err(RX3023)。
pub fn extract_requires(
    attrs: &[ast::Attr],
    src: &str,
) -> Result<BTreeSet<CapabilityId>, CapInvalid> {
    let mut out: BTreeSet<CapabilityId> = BTreeSet::new();
    for attr in attrs {
        if single_seg(&attr.meta.path) != Some("requires") {
            continue;
        }
        let MetaKind::List(inner) = &attr.meta.kind else {
            return Err(invalid(
                "capability.unknown_id: `#[requires]` 须为列表形态 `#[requires(\"capability.id\", ...)]`(RXS-0311)".to_owned(),
                attr.span,
            ));
        };
        if inner.is_empty() {
            return Err(invalid(
                "capability.unknown_id: `#[requires(...)]` 实参须为 ≥1 个 capability ID 字符串字面量(空列表非法,RXS-0311)"
                    .to_owned(),
                attr.span,
            ));
        }
        for entry in inner {
            let MetaInner::Lit(lit) = entry else {
                return Err(invalid(
                    "capability.unknown_id: `#[requires(...)]` 实参须为字符串字面量(标识符/子句不在闭集,RXS-0311)"
                        .to_owned(),
                    attr.span,
                ));
            };
            let Some(id_text) = lit_str(src, lit) else {
                return Err(invalid(
                    "capability.unknown_id: `#[requires(...)]` 实参须为字符串字面量(RXS-0311)"
                        .to_owned(),
                    lit.span,
                ));
            };
            let Some(id) = CapabilityId::from_name(id_text) else {
                return Err(invalid(
                    format!(
                        "capability.unknown_id: 未知 capability ID `{id_text}`(闭集十二项 = {};RXS-0311/0349)",
                        CapabilityId::ALL
                            .iter()
                            .map(|c| format!("`{}`", c.name()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    lit.span,
                ));
            };
            out.insert(id);
        }
    }
    Ok(out)
}

/// `#[requires]` 附着合法性 + 解析校验(RXS-0311;AST 层,与 `#[permutation]`
/// 家族同一机械,经 [`crate::shader_stages::check`] 挂接)。可附着对象 =
/// 着色入口函数(着色阶段 fn / `kernel fn`)与 `device fn`;host-only 函数
/// (host/const 着色)、非函数 item、非 device 着色关联函数 → RX3023 同类拒。
/// 泛型着色函数不产 entry(RXS-0304),其上标注仍校验闭集合法性(诊断面不豁免)。
//@ spec: RXS-0311
pub fn check_requires(file: &ast::SourceFile, src: &str, diag: &DiagCtxt) {
    check_requires_rec(&file.items, src, diag);
}

/// fn 着色/阶段是否为 `#[requires]` 合法附着对象(RXS-0311:着色入口 + device fn)。
fn legal_requires_target(f: &ast::FnItem) -> bool {
    match f.stage {
        Some(_) => f.color == FnColor::Kernel,
        None => matches!(f.color, FnColor::Kernel | FnColor::Device),
    }
}

fn emit_cap_invalid(diag: &DiagCtxt, inv: &CapInvalid, label: &str) {
    diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
        .arg("detail", inv.detail.clone())
        .span_label(inv.span, label)
        .emit();
}

fn check_fn_requires(f: &ast::FnItem, attrs: &[ast::Attr], src: &str, diag: &DiagCtxt) {
    let Some(req_attr) = attrs
        .iter()
        .find(|a| single_seg(&a.meta.path) == Some("requires"))
    else {
        return;
    };
    if !legal_requires_target(f) {
        diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
            .arg(
                "detail",
                format!(
                    "capability.unknown_id: `#[requires]` 仅可附着着色入口函数与 device function;`{}` 为 host-only 函数或非函数 item(RXS-0311)",
                    f.name.name
                ),
            )
            .span_label(req_attr.span, "invalid #[requires] attachment")
            .emit();
        return;
    }
    if let Err(inv) = extract_requires(attrs, src) {
        emit_cap_invalid(diag, &inv, "invalid #[requires] capability id");
    }
}

fn check_requires_rec(items: &[ast::Item], src: &str, diag: &DiagCtxt) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => check_fn_requires(f, &it.attrs, src, diag),
            ast::ItemKind::Mod(m) => {
                if let Some(req_attr) = it
                    .attrs
                    .iter()
                    .find(|a| single_seg(&a.meta.path) == Some("requires"))
                {
                    diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
                        .arg(
                            "detail",
                            "capability.unknown_id: `#[requires]` 仅可附着着色入口函数与 device function;`mod` 为非法附着对象(RXS-0311)"
                                .to_owned(),
                        )
                        .span_label(req_attr.span, "invalid #[requires] attachment")
                        .emit();
                }
                check_requires_rec(&m.items, src, diag);
            }
            ast::ItemKind::Impl(im) => {
                for a in &im.items {
                    if let ast::AssocItemKind::Fn(f) = &a.kind {
                        // 关联函数仅 device 着色为合法附着对象(着色入口不可能为
                        // 关联项;host/trait 声明 → RX3023 同类拒)。
                        let Some(req_attr) = a
                            .attrs
                            .iter()
                            .find(|at| single_seg(&at.meta.path) == Some("requires"))
                        else {
                            continue;
                        };
                        if f.color != FnColor::Device || f.stage.is_some() {
                            diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
                                .arg(
                                    "detail",
                                    format!(
                                        "capability.unknown_id: `#[requires]` 仅可附着着色入口函数与 device function;关联函数 `{}` 非 device 着色(RXS-0311)",
                                        f.name.name
                                    ),
                                )
                                .span_label(req_attr.span, "invalid #[requires] attachment")
                                .emit();
                            continue;
                        }
                        if let Err(inv) = extract_requires(&a.attrs, src) {
                            emit_cap_invalid(diag, &inv, "invalid #[requires] capability id");
                        }
                    } else if let Some(req_attr) = a
                        .attrs
                        .iter()
                        .find(|at| single_seg(&at.meta.path) == Some("requires"))
                    {
                        diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
                            .arg(
                                "detail",
                                "capability.unknown_id: `#[requires]` 仅可附着着色入口函数与 device function;关联常量/关联类型为非法附着对象(RXS-0311)"
                                    .to_owned(),
                            )
                            .span_label(req_attr.span, "invalid #[requires] attachment")
                            .emit();
                    }
                }
            }
            _ => {
                // 非函数 item 上的 `#[requires]` → RX3023 同类拒。
                if let Some(req_attr) = it
                    .attrs
                    .iter()
                    .find(|a| single_seg(&a.meta.path) == Some("requires"))
                {
                    diag.struct_error(E_CAP_UNKNOWN_ID, "capability.unknown_id")
                        .arg(
                            "detail",
                            "capability.unknown_id: `#[requires]` 仅可附着着色入口函数与 device function;非函数 item 为非法附着对象(RXS-0311)"
                                .to_owned(),
                        )
                        .span_label(req_attr.span, "invalid #[requires] attachment")
                        .emit();
                }
            }
        }
    }
}

// ═══════════════════════ 隐式推导映射表(RXS-0311 冻结面) ═══════════════════════

/// 四个 RT intrinsic 的编译器已知自由函数名 → capability(RXS-0311 映射表;
/// 该四名的已知签名在 spec 层冻结(RXS-0245 系),编译器当前未注册可调 lang
/// item(RT 体 lowering 归 M50)——识别 = 单段路径名匹配 + 「未解析到用户
/// DefId」守卫(用户同名定义遮蔽时不推导;resolve 结果在场时 DefId 级判定)。
const RT_INTRINSIC_CAPS: &[(&str, CapabilityId)] = &[
    ("trace_ray", CapabilityId::RtPipeline),
    (
        "report_intersection",
        CapabilityId::RtIntersectionProcedural,
    ),
    ("execute_callable", CapabilityId::RtCallable),
    ("ignore_intersection", CapabilityId::RtAnyHit),
];

/// stage 映射(RXS-0311 冻结表):着色阶段 → 隐式 requirement。
fn stage_caps(stage: ShaderStage) -> &'static [CapabilityId] {
    match stage {
        ShaderStage::RayGen | ShaderStage::Miss | ShaderStage::ClosestHit => {
            &[CapabilityId::RtPipeline]
        }
        ShaderStage::AnyHit => &[CapabilityId::RtAnyHit],
        ShaderStage::Intersection => &[CapabilityId::RtIntersectionProcedural],
        ShaderStage::Callable => &[CapabilityId::RtCallable],
        ShaderStage::Task => &[CapabilityId::MeshTask],
        _ => &[],
    }
}

/// compute 签名判定(kernel fn / compute fn;RXS-0297 AccelStruct 加性扩展面)。
fn is_compute_signature(f: &ast::FnItem) -> bool {
    (f.color == FnColor::Kernel && f.stage.is_none()) || f.stage == Some(ShaderStage::Compute)
}

/// 单 fn 的隐式 requirement 集(RXS-0311 映射表三族:stage / 形参 / intrinsic)。
/// `res` 在场时 intrinsic 识别为 DefId 级(ray_query_initialize lang item;
/// 四 RT intrinsic 未注册可调 lang item,以名识别 + 用户遮蔽守卫)。
//@ spec: RXS-0311
pub fn implicit_for_fn(
    f: &ast::FnItem,
    attrs: &[ast::Attr],
    res: Option<&Resolutions>,
) -> BTreeSet<CapabilityId> {
    let _ = attrs;
    let mut out: BTreeSet<CapabilityId> = BTreeSet::new();
    // stage 映射。
    if let Some(stage) = f.stage {
        out.extend(stage_caps(stage).iter().copied());
    }
    // 形参映射:`AccelStruct` compute 签名形参 → rt.ray_query;
    // `#[shader_record]` 形参(M50)→ rt.sbt_user_data。
    for p in &f.params {
        if is_compute_signature(f)
            && let ast::ParamKind::Typed { ty, .. } = &p.kind
            && crate::shader_stages::is_accel_struct(iface::unwrap_ty(ty))
        {
            out.insert(CapabilityId::RtRayQuery);
        }
        if p.attrs
            .iter()
            .any(|a| single_seg(&a.meta.path) == Some("shader_record"))
        {
            out.insert(CapabilityId::RtSbtUserData);
        }
    }
    // intrinsic 映射(函数体走查)。
    if let Some(body) = &f.body {
        let mut visitor = IntrinsicVisitor {
            res,
            caps: &mut out,
        };
        visitor.walk_block(body);
    }
    out
}

struct IntrinsicVisitor<'a> {
    res: Option<&'a Resolutions>,
    caps: &'a mut BTreeSet<CapabilityId>,
}

impl IntrinsicVisitor<'_> {
    /// 调用点识别:直调路径 callee 映射 intrinsic 表;随后递归实参。
    fn visit_call(&mut self, callee: &ast::Expr, args: &[ast::Expr]) {
        if let ast::ExprKind::Path(p) = &callee.kind
            && let [seg] = p.segments.as_slice()
        {
            let name = seg.ident.name.as_str();
            // ray_query_initialize → rt.ray_query(DefId 级优先)。
            let is_rq_init = match self.res {
                Some(res) => matches!(
                    res.path_res.get(&p.span),
                    Some(Res::Def(d)) if res.lang_items.is_ray_query_initialize(*d)
                ),
                None => name == "ray_query_initialize",
            };
            if is_rq_init {
                self.caps.insert(CapabilityId::RtRayQuery);
            }
            // 四 RT intrinsic(名识别;resolve 在场且解析到用户 DefId = 用户遮蔽,
            // 不推导)。
            if let Some((_, cap)) = RT_INTRINSIC_CAPS.iter().find(|(n, _)| *n == name) {
                let shadowed = match self.res {
                    Some(res) => matches!(res.path_res.get(&p.span), Some(Res::Def(_))),
                    None => false,
                };
                if !shadowed {
                    self.caps.insert(*cap);
                }
            }
        }
        self.walk_expr(callee);
        for a in args {
            self.walk_expr(a);
        }
    }

    /// `RayQuery` 局部变量(类型标注头名;initialize 调用另行覆盖无标注形态)。
    fn visit_let(&mut self, let_: &ast::LetStmt) {
        if let Some(ty) = &let_.ty
            && iface::ty_head_name(ty) == Some("RayQuery")
        {
            self.caps.insert(CapabilityId::RtRayQuery);
        }
        if let Some(init) = &let_.init {
            self.walk_expr(init);
        }
    }

    fn walk_block(&mut self, b: &ast::Block) {
        for s in &b.stmts {
            match &s.kind {
                // 嵌套 item 的调用不归属于外层 fn(各自为图节点),不下潜。
                ast::StmtKind::Item(_) => {}
                ast::StmtKind::Let(l) => self.visit_let(l),
                ast::StmtKind::Expr { expr, .. } => self.walk_expr(expr),
                ast::StmtKind::Empty => {}
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t);
        }
    }

    fn walk_expr(&mut self, e: &ast::Expr) {
        match &e.kind {
            ast::ExprKind::Call { callee, args } => self.visit_call(callee, args),
            ast::ExprKind::MethodCall { receiver, args, .. } => {
                self.walk_expr(receiver);
                for a in args {
                    self.walk_expr(a);
                }
            }
            ast::ExprKind::Unary { expr, .. }
            | ast::ExprKind::Borrow { expr, .. }
            | ast::ExprKind::Cast { expr, .. }
            | ast::ExprKind::Field { expr, .. }
            | ast::ExprKind::TupleField { expr, .. }
            | ast::ExprKind::Try(expr)
            | ast::ExprKind::Paren(expr) => self.walk_expr(expr),
            ast::ExprKind::Binary { lhs, rhs, .. } | ast::ExprKind::Assign { lhs, rhs, .. } => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            ast::ExprKind::Range { lo, hi, .. } => {
                self.walk_expr(lo);
                self.walk_expr(hi);
            }
            ast::ExprKind::Index { expr, index } => {
                self.walk_expr(expr);
                self.walk_expr(index);
            }
            ast::ExprKind::Tuple(xs) | ast::ExprKind::Array(xs) => {
                for x in xs {
                    self.walk_expr(x);
                }
            }
            ast::ExprKind::Repeat { elem, len } => {
                self.walk_expr(elem);
                self.walk_expr(len);
            }
            ast::ExprKind::StructLit { fields, .. } => {
                for f in fields {
                    if let Some(x) = &f.expr {
                        self.walk_expr(x);
                    }
                }
            }
            ast::ExprKind::Block(b) | ast::ExprKind::Unsafe(b) => self.walk_block(b),
            ast::ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond);
                self.walk_block(then);
                if let Some(eb) = else_ {
                    self.walk_expr(eb);
                }
            }
            ast::ExprKind::While { cond, body } => {
                self.walk_expr(cond);
                self.walk_block(body);
            }
            ast::ExprKind::For { iter, body, .. } => {
                self.walk_expr(iter);
                self.walk_block(body);
            }
            ast::ExprKind::Loop { body } => self.walk_block(body),
            ast::ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.walk_expr(g);
                    }
                    self.walk_expr(&arm.body);
                }
            }
            ast::ExprKind::Return(Some(x)) | ast::ExprKind::Break(Some(x)) => self.walk_expr(x),
            ast::ExprKind::Closure { body, .. } => self.walk_expr(body),
            ast::ExprKind::Lit(_)
            | ast::ExprKind::Path(_)
            | ast::ExprKind::Return(None)
            | ast::ExprKind::Break(None)
            | ast::ExprKind::Continue
            | ast::ExprKind::Err => {}
        }
    }
}

// ═══════════════════════ 调用图并集(RXS-0311) ═══════════════════════

/// v1 可枚举着色入口判定(与 `reflection::enumerable_stage` 同一口径:RT 阶段
/// 与 task 不可枚举;`kernel fn` 归 Compute)。
fn enumerable_stage(stage: Option<ShaderStage>, color: FnColor) -> Option<ShaderStage> {
    match stage {
        None if color == FnColor::Kernel => Some(ShaderStage::Compute),
        Some(
            s @ (ShaderStage::Vertex
            | ShaderStage::Fragment
            | ShaderStage::Compute
            | ShaderStage::Mesh),
        ) => Some(s),
        _ => None,
    }
}

/// 一个 fn 节点(图顶点;DefId 级)。
struct FnNode<'a> {
    /// 源级名称路径(嵌套 mod / 外层 fn 以 `::` 连接)。
    name_path: String,
    /// DefId(resolve `item_defs`;块级 fn 亦有登记)。
    def_id: DefId,
    /// AST fn 项。
    f: &'a ast::FnItem,
    /// 自身显式 ∪ 隐式 requirement(不含 callee 并集)。
    own: BTreeSet<CapabilityId>,
    /// 直调 callee DefId 表(首现源序,去重;DefId 级 call facts,禁名字匹配)。
    callees: Vec<DefId>,
}

/// 收集 fn 节点(条目级 mod 递归 + fn 体语句级嵌套 item;resolve `item_defs`
/// 取 DefId,未登记 = 保守跳过——正常单元不可能,防御 fail-closed 不猜)。
fn collect_fn_nodes<'a>(
    items: &'a [ast::Item],
    prefix: &str,
    res: &Resolutions,
    src: &str,
    out: &mut Vec<FnNode<'a>>,
) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => {
                if let Some(def_id) = res.item_defs.get(&it.span).copied() {
                    let name_path = format!("{prefix}{}", f.name.name);
                    let mut own = extract_requires(&it.attrs, src).unwrap_or_default();
                    own.extend(implicit_for_fn(f, &it.attrs, Some(res)));
                    let mut node = FnNode {
                        name_path,
                        def_id,
                        f,
                        own,
                        callees: Vec::new(),
                    };
                    collect_callees(f, res, &mut node.callees);
                    out.push(node);
                }
                // fn 体语句级嵌套 fn(item_defs 同表登记)。
                if let Some(body) = &f.body {
                    collect_nested_fn_nodes(
                        body,
                        &format!("{prefix}{}::", f.name.name),
                        res,
                        src,
                        out,
                    );
                }
            }
            ast::ItemKind::Mod(m) => {
                collect_fn_nodes(
                    &m.items,
                    &format!("{prefix}{}::", m.name.name),
                    res,
                    src,
                    out,
                );
            }
            _ => {}
        }
    }
}

fn collect_nested_fn_nodes<'a>(
    body: &'a ast::Block,
    prefix: &str,
    res: &Resolutions,
    src: &str,
    out: &mut Vec<FnNode<'a>>,
) {
    for s in &body.stmts {
        let ast::StmtKind::Item(item) = &s.kind else {
            continue;
        };
        if let ast::ItemKind::Fn(f) = &item.kind
            && let Some(def_id) = res.item_defs.get(&item.span).copied()
        {
            let mut own = extract_requires(&item.attrs, src).unwrap_or_default();
            own.extend(implicit_for_fn(f, &item.attrs, Some(res)));
            let mut node = FnNode {
                name_path: format!("{prefix}{}", f.name.name),
                def_id,
                f,
                own,
                callees: Vec::new(),
            };
            collect_callees(f, res, &mut node.callees);
            out.push(node);
        }
    }
}

/// 直调 callee 收集(DefId 级:路径 callee 经 resolve `path_res` →
/// `Res::Def(DefId)`,DefKind ∈ {Fn, AssocFn};首现源序去重,禁名字匹配)。
/// 方法调用(receiver 类型解析在 TBIR 期)不在 AST 调用事实面(v1 诚实边界)。
fn collect_callees(f: &ast::FnItem, res: &Resolutions, out: &mut Vec<DefId>) {
    struct W<'a> {
        res: &'a Resolutions,
        out: &'a mut Vec<DefId>,
    }
    impl W<'_> {
        fn call(&mut self, callee: &ast::Expr, args: &[ast::Expr]) {
            if let ast::ExprKind::Path(p) = &callee.kind
                && let Some(Res::Def(d)) = self.res.path_res.get(&p.span)
                && matches!(
                    self.res.defs[d.0 as usize].kind,
                    DefKind::Fn | DefKind::AssocFn
                )
                && !self.out.contains(d)
            {
                self.out.push(*d);
            }
            self.expr(callee);
            for a in args {
                self.expr(a);
            }
        }
        fn block(&mut self, b: &ast::Block) {
            for s in &b.stmts {
                match &s.kind {
                    // 嵌套 item 体不归属于外层 fn 的调用事实。
                    ast::StmtKind::Item(_) => {}
                    ast::StmtKind::Let(l) => {
                        if let Some(init) = &l.init {
                            self.expr(init);
                        }
                    }
                    ast::StmtKind::Expr { expr, .. } => self.expr(expr),
                    ast::StmtKind::Empty => {}
                }
            }
            if let Some(t) = &b.tail {
                self.expr(t);
            }
        }
        fn expr(&mut self, e: &ast::Expr) {
            match &e.kind {
                ast::ExprKind::Call { callee, args } => self.call(callee, args),
                ast::ExprKind::MethodCall { receiver, args, .. } => {
                    self.expr(receiver);
                    for a in args {
                        self.expr(a);
                    }
                }
                ast::ExprKind::Unary { expr, .. }
                | ast::ExprKind::Borrow { expr, .. }
                | ast::ExprKind::Cast { expr, .. }
                | ast::ExprKind::Field { expr, .. }
                | ast::ExprKind::TupleField { expr, .. }
                | ast::ExprKind::Try(expr)
                | ast::ExprKind::Paren(expr) => self.expr(expr),
                ast::ExprKind::Binary { lhs, rhs, .. } | ast::ExprKind::Assign { lhs, rhs, .. } => {
                    self.expr(lhs);
                    self.expr(rhs);
                }
                ast::ExprKind::Range { lo, hi, .. } => {
                    self.expr(lo);
                    self.expr(hi);
                }
                ast::ExprKind::Index { expr, index } => {
                    self.expr(expr);
                    self.expr(index);
                }
                ast::ExprKind::Tuple(xs) | ast::ExprKind::Array(xs) => {
                    for x in xs {
                        self.expr(x);
                    }
                }
                ast::ExprKind::Repeat { elem, len } => {
                    self.expr(elem);
                    self.expr(len);
                }
                ast::ExprKind::StructLit { fields, .. } => {
                    for f in fields {
                        if let Some(x) = &f.expr {
                            self.expr(x);
                        }
                    }
                }
                ast::ExprKind::Block(b) | ast::ExprKind::Unsafe(b) => self.block(b),
                ast::ExprKind::If { cond, then, else_ } => {
                    self.expr(cond);
                    self.block(then);
                    if let Some(eb) = else_ {
                        self.expr(eb);
                    }
                }
                ast::ExprKind::While { cond, body } => {
                    self.expr(cond);
                    self.block(body);
                }
                ast::ExprKind::For { iter, body, .. } => {
                    self.expr(iter);
                    self.block(body);
                }
                ast::ExprKind::Loop { body } => self.block(body),
                ast::ExprKind::Match { scrutinee, arms } => {
                    self.expr(scrutinee);
                    for arm in arms {
                        if let Some(g) = &arm.guard {
                            self.expr(g);
                        }
                        self.expr(&arm.body);
                    }
                }
                ast::ExprKind::Return(Some(x)) | ast::ExprKind::Break(Some(x)) => self.expr(x),
                ast::ExprKind::Closure { body, .. } => self.expr(body),
                ast::ExprKind::Lit(_)
                | ast::ExprKind::Path(_)
                | ast::ExprKind::Return(None)
                | ast::ExprKind::Break(None)
                | ast::ExprKind::Continue
                | ast::ExprKind::Err => {}
            }
        }
    }
    if let Some(body) = &f.body {
        let mut w = W { res, out };
        w.block(body);
    }
}

/// 单 entry 的 capability 事实(RXS-0311 调用图并集律产物)。
#[derive(Clone, Debug)]
pub struct EntryCaps {
    /// entry identity(源级名称路径)。
    pub name: String,
    /// 规范化阶段(`kernel fn` → Compute;反射同口径)。
    pub stage: ShaderStage,
    /// 有效 requirement 集(显式 ∪ 隐式 ∪ 全部静态可达 callee 有效集之并)。
    pub effective: BTreeSet<CapabilityId>,
    /// 逐 capability 的首个引入者(entry 自身优先,其后 callee 按调用点首现
    /// 源序 BFS;RXS-0311 诊断形态「首个引入它的可达 callee」)。
    pub introducer: BTreeMap<CapabilityId, String>,
    /// entry fn 的 DefId(codegen 根过滤键)。
    pub def_id: DefId,
    /// 诊断锚点(entry 名字 span)。
    pub span: Span,
}

/// 编译单元级 capability 事实(全部可枚举 entry,规范键排序)。
#[derive(Clone, Debug)]
pub struct UnitCapabilities {
    /// entry 事实表(按 `(name, stage_tag)` 规范键排序)。
    pub entries: Vec<EntryCaps>,
}

/// 构建编译单元的调用图并集 capability 事实(RXS-0311)。可达闭包 = DefId 级
/// BFS(entry 自身 → 调用点首现源序逐 callee;确定性)。泛型着色函数不产
/// entry(RXS-0304);其余 fn(含泛型 device fn、块级 fn)照常作图节点,
/// 其 requirement 随调用边并入调用方。
//@ spec: RXS-0311
pub fn build_unit_caps(file: &ast::SourceFile, src: &str, res: &Resolutions) -> UnitCapabilities {
    let mut nodes: Vec<FnNode> = Vec::new();
    collect_fn_nodes(&file.items, "", res, src, &mut nodes);
    let by_def: HashMap<DefId, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.def_id, i))
        .collect();

    let mut entries: Vec<EntryCaps> = Vec::new();
    for n in &nodes {
        let Some(stage) = enumerable_stage(n.f.stage, n.f.color) else {
            continue;
        };
        // 泛型着色函数不产 entry(RXS-0304 口径)。
        if !n.f.generics.params.is_empty() {
            continue;
        }
        // BFS 可达闭包(entry 自身为第 0 个访问者 → 自身引入优先)。
        let mut effective: BTreeSet<CapabilityId> = BTreeSet::new();
        let mut introducer: BTreeMap<CapabilityId, String> = BTreeMap::new();
        let mut visited: HashSet<DefId> = HashSet::new();
        let mut queue: std::collections::VecDeque<DefId> = std::collections::VecDeque::new();
        visited.insert(n.def_id);
        queue.push_back(n.def_id);
        while let Some(d) = queue.pop_front() {
            let Some(&idx) = by_def.get(&d) else {
                continue;
            };
            let node = &nodes[idx];
            for cap in &node.own {
                effective.insert(*cap);
                introducer
                    .entry(*cap)
                    .or_insert_with(|| node.name_path.clone());
            }
            for c in &node.callees {
                if visited.insert(*c) {
                    queue.push_back(*c);
                }
            }
        }
        entries.push(EntryCaps {
            name: n.name_path.clone(),
            stage,
            effective,
            introducer,
            def_id: n.def_id,
            span: n.f.name.span,
        });
    }
    entries.sort_by(|a, b| {
        (a.name.as_str(), crate::codegen::stage_tag(a.stage))
            .cmp(&(b.name.as_str(), crate::codegen::stage_tag(b.stage)))
    });
    UnitCapabilities { entries }
}

// ═══════════════════════ canonical 编码(RXS-0305 CanonW 律) ═══════════════════════

/// canonical bytes 写入器(与 reflection.rs/permutation.rs CanonW 同一律:u32
/// 小端定宽、length-prefix 字符串、u32 计数列表;本模块自持有副本)。
struct CanonW {
    buf: Vec<u8>,
}

impl CanonW {
    fn new() -> Self {
        CanonW { buf: Vec::new() }
    }
    fn u32v(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn strv(&mut self, s: &str) {
        self.u32v(u32::try_from(s.len()).unwrap_or(u32::MAX));
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
}

// ═══════════════════════ profile 闭集(RXS-0312) ═══════════════════════

/// profile v1 模型(版本化闭集;由项目/构建 manifest 选择,**不从当前开发机
/// 自动生成**,RFC-0019 §4.5.2 逐字)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Profile {
    /// profile 名。
    pub name: String,
    /// profile 版本字串。
    pub version: String,
    /// 必需 capability 集。
    pub required: BTreeSet<CapabilityId>,
    /// 可选 capability 集。
    pub optional: BTreeSet<CapabilityId>,
    /// 禁用 capability 集。
    pub forbidden: BTreeSet<CapabilityId>,
    /// fallback 映射(逻辑 entry 名 → fallback entry 名;键字节序)。
    pub fallbacks: BTreeMap<String, String>,
}

/// profile 装载/解析失败。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProfileError {
    /// 文件不可读 / JSON 形态非法 / schema 常量不符 / 三集两两相交
    /// (RX7001 工具段;profile 为构建 manifest 输入,非语言诊断)。
    Malformed(String),
    /// 闭集外 capability ID(RX3023,`capability.unknown_id` 同类)。
    UnknownId(String),
}

impl Profile {
    /// 规范 profile 字节(RXS-0312):版本前缀起始 + name/version
    /// length-prefix + 三集各按字节序排序编码 + fallbacks 按键字节序编码
    /// (CanonW 律)。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = CanonW::new();
        w.bytes(PROFILE_V1);
        w.strv(&self.name);
        w.strv(&self.version);
        for set in [&self.required, &self.optional, &self.forbidden] {
            w.u32v(set.len() as u32);
            for c in set {
                w.strv(c.name());
            }
        }
        w.u32v(self.fallbacks.len() as u32);
        for (k, v) in &self.fallbacks {
            w.strv(k);
            w.strv(v);
        }
        w.buf
    }

    /// `selected_profile_digest = SHA-256("rurix.profile.v1\0" || canonical
    /// 去前缀段)`(RXS-0312)。canonical 字节以前缀起始,故 digest =
    /// SHA-256(完整 canonical 字节),与 spec 定义逐字节等价。
    pub fn digest(&self) -> [u8; 32] {
        sha256::digest(&self.canonical_bytes())
    }

    /// profile 提供集(required ∪ optional;选择律分支二判定面)。
    pub fn provided(&self) -> BTreeSet<CapabilityId> {
        self.required.union(&self.optional).copied().collect()
    }
}

/// 装载 profile JSON 文件(RXS-0312;纯 host IO + 解析)。
pub fn load_profile(path: &Path) -> Result<Profile, ProfileError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        ProfileError::Malformed(format!("cannot read profile {}: {e}", path.display()))
    })?;
    parse_profile(&text)
}

/// JSON 数组切片 → 顶层元素表(字符串感知逗号切分;profile 元素为简单
/// 字符串字面量)。
fn split_json_top_level(slice: &str, open: char, close: char) -> Option<Vec<String>> {
    let body = slice.trim_start();
    let body = body.strip_prefix(open)?.strip_suffix(close)?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut escaped = false;
    let mut depth = 0i32;
    for ch in body.chars() {
        if in_str {
            cur.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                cur.push(ch);
            }
            '{' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    let last = cur.trim();
    if !last.is_empty() {
        out.push(last.to_owned());
    }
    Some(out)
}

/// 剥 JSON 字符串元素引号(profile 元素无转义序列)。
fn unquote(s: &str) -> Option<&str> {
    s.strip_prefix('"')?.strip_suffix('"')
}

/// capability ID 字符串数组 → 闭集成员集(闭集外 → UnknownId = RX3023 同类)。
fn parse_id_set(slice: &str, field: &str) -> Result<BTreeSet<CapabilityId>, ProfileError> {
    let elems = split_json_top_level(slice, '[', ']')
        .ok_or_else(|| ProfileError::Malformed(format!("profile 字段 `{field}` 须为 JSON 数组")))?;
    let mut out = BTreeSet::new();
    for e in elems {
        let id_text = unquote(&e).ok_or_else(|| {
            ProfileError::Malformed(format!("profile 字段 `{field}` 元素须为字符串字面量"))
        })?;
        let Some(id) = CapabilityId::from_name(id_text) else {
            return Err(ProfileError::UnknownId(format!(
                "capability.unknown_id: profile 字段 `{field}` 引用闭集外 capability ID `{id_text}`(闭集十二项,RXS-0311/0312/0349)"
            )));
        };
        out.insert(id);
    }
    Ok(out)
}

/// 解析 profile JSON 文本(RXS-0312 字段闭集):schema 常量 / name / version /
/// required / optional / forbidden / fallbacks;三集两两不相交否则拒。
pub fn parse_profile(text: &str) -> Result<Profile, ProfileError> {
    use crate::tooling::json_util as ju;
    let malformed = |d: &str| ProfileError::Malformed(d.to_owned());
    let schema = ju::json_str_field(text, "schema")
        .ok_or_else(|| malformed("profile 缺 `schema` 字段(RXS-0312 字段闭集)"))?;
    if schema != PROFILE_SCHEMA_ID {
        return Err(malformed(
            "profile `schema` 常量须为 \"rurix.profile.v1\"(RXS-0312 版本化闭集)",
        ));
    }
    let name = ju::json_str_field(text, "name")
        .ok_or_else(|| malformed("profile 缺 `name` 字段(RXS-0312 字段闭集)"))?;
    let version = ju::json_str_field(text, "version")
        .ok_or_else(|| malformed("profile 缺 `version` 字段(RXS-0312 字段闭集)"))?;
    let required = parse_id_set(
        ju::json_array_field(text, "required")
            .ok_or_else(|| malformed("profile 缺 `required` 数组(RXS-0312 字段闭集)"))?,
        "required",
    )?;
    let optional = parse_id_set(
        ju::json_array_field(text, "optional")
            .ok_or_else(|| malformed("profile 缺 `optional` 数组(RXS-0312 字段闭集)"))?,
        "optional",
    )?;
    let forbidden = parse_id_set(
        ju::json_array_field(text, "forbidden")
            .ok_or_else(|| malformed("profile 缺 `forbidden` 数组(RXS-0312 字段闭集)"))?,
        "forbidden",
    )?;
    let fallbacks_slice = ju::json_object_field(text, "fallbacks")
        .ok_or_else(|| malformed("profile 缺 `fallbacks` 对象(RXS-0312 字段闭集)"))?;
    let mut fallbacks = BTreeMap::new();
    for e in split_json_top_level(fallbacks_slice, '{', '}')
        .ok_or_else(|| malformed("profile `fallbacks` 须为 JSON 对象"))?
    {
        let Some((k, v)) = e.split_once(':') else {
            return Err(malformed(
                "profile `fallbacks` 条目须为 \"逻辑 entry 名\": \"fallback entry 名\"",
            ));
        };
        let key =
            unquote(k.trim()).ok_or_else(|| malformed("profile `fallbacks` 键须为字符串字面量"))?;
        let val =
            unquote(v.trim()).ok_or_else(|| malformed("profile `fallbacks` 值须为字符串字面量"))?;
        if fallbacks.insert(key.to_owned(), val.to_owned()).is_some() {
            return Err(malformed("profile `fallbacks` 键重复(逻辑 entry 名须唯一)"));
        }
    }
    // 三集两两不相交(交集非空 = profile 非法,确定性拒,RXS-0312)。
    for (a, an, b, bn) in [
        (&required, "required", &optional, "optional"),
        (&required, "required", &forbidden, "forbidden"),
        (&optional, "optional", &forbidden, "forbidden"),
    ] {
        let inter: Vec<String> = a
            .intersection(b)
            .map(|c| format!("`{}`", c.name()))
            .collect();
        if !inter.is_empty() {
            return Err(ProfileError::Malformed(format!(
                "profile 三集须两两不相交:`{an}` 与 `{bn}` 交集非空({};RXS-0312)",
                inter.join(", ")
            )));
        }
    }
    Ok(Profile {
        name,
        version,
        required,
        optional,
        forbidden,
        fallbacks,
    })
}

// ═══════════════════════ 构建期选择律(RXS-0312) ═══════════════════════

/// entry 选择状态(RXS-0312 manifest `status` 字段闭集)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryStatus {
    /// 有效集 ⊆ provided 且 ∩ forbidden = ∅ → 照常发射。
    Emitted,
    /// 缺能力但 fallback 映射兼容 → 选 fallback,主 variant 不发射。
    Fallback,
}

impl EntryStatus {
    fn name(self) -> &'static str {
        match self {
            EntryStatus::Emitted => "emitted",
            EntryStatus::Fallback => "fallback",
        }
    }
}

/// 单 entry 的选择记录(manifest per-entry 字段闭集)。
#[derive(Clone, Debug)]
pub struct SelectionRecord {
    /// 逻辑 entry identity。
    pub name: String,
    /// 有效 requirement 集(排序 ID 表)。
    pub effective: Vec<String>,
    /// 选择状态。
    pub status: EntryStatus,
    /// 实际发射 entry identity(emitted = 自身;fallback = fallback entry 名)。
    pub selected_entry: String,
    /// 缺失 capability ID 表(emitted 恒空)。
    pub missing: Vec<String>,
    /// 违禁命中 ID 表(绿路径恒空;命中即 RX3021 fail-closed 无 manifest)。
    pub forbidden_hits: Vec<String>,
}

/// 编译单元级选择结果(RXS-0312)。
#[derive(Clone, Debug)]
pub struct SelectionOutcome {
    /// per-entry 记录(按逻辑名规范键排序)。
    pub records: Vec<SelectionRecord>,
    /// 本 profile 规范 digest。
    pub profile_digest: [u8; 32],
    /// 主 variant 不发射的逻辑 entry 名表(codegen 根过滤键)。
    pub suppressed_names: BTreeSet<String>,
    /// 主 variant 不发射的 entry DefId 表(codegen 根过滤键)。
    pub suppressed_defs: HashSet<DefId>,
}

/// 选择律违例(RX3020/3021/3022 载体;fail-closed,不产部分 manifest/产物)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelectionError {
    /// 诊断数字码(typeck 段)。
    pub code: u16,
    /// message key(en/zh 成对)。
    pub key: &'static str,
    /// 诊断上下文({detail} 参数;携带 symbolic key 字面 + 判据要求事实)。
    pub detail: String,
    /// 诊断锚点(entry 名字 span)。
    pub span: Span,
}

/// fallback 接口契约兼容判定(RXS-0312 v1.3 精确化逐字):
/// 1. stage 相同,且 `io`/`push_constants`/`execution_modes` 结构相等;
/// 2. `resources` 按声明序比对:主表 class == `accel` 且缺失能力 ∈
///    {rt.ray_query, rt.pipeline} 的条目允许 fallback 缺席;其余逐项相等
///    (binding 号按 fallback 自身声明序独立推导,不构成不兼容)、相对序
///    一致;fallback 不得多出条目。
/// 返回 Err(不兼容字段名)。
//@ spec: RXS-0312
pub(crate) fn fallback_compatible(
    main: &crate::reflection::InterfaceFacts,
    fallback: &crate::reflection::InterfaceFacts,
    missing: &BTreeSet<CapabilityId>,
) -> Result<(), &'static str> {
    if main.stage != fallback.stage {
        return Err("stage");
    }
    if main.io != fallback.io {
        return Err("io");
    }
    if main.push_constants != fallback.push_constants {
        return Err("push_constants");
    }
    if main.execution_modes != fallback.execution_modes {
        return Err("execution_modes");
    }
    // 「缺失 capability 对应资源类」缺席豁免(v1 冻结映射 accel ↔
    // {rt.ray_query, rt.pipeline}):低 profile 环境无此类资源可绑,fallback
    // 缺席恰为正确外部接口。
    let droppable = |r: &crate::reflection::ResourceEntry| {
        r.class == "accel"
            && missing
                .iter()
                .any(|c| matches!(c, CapabilityId::RtRayQuery | CapabilityId::RtPipeline))
    };
    // binding 号按 fallback 自身声明序独立推导(缺席条目的自然结果)——比对面
    // = name/class/set/count/access/format/visibility;序由声明序双指针保持。
    let compatible_entry = |a: &crate::reflection::ResourceEntry,
                            b: &crate::reflection::ResourceEntry| {
        a.name == b.name
            && a.class == b.class
            && a.set == b.set
            && a.count == b.count
            && a.access == b.access
            && a.format == b.format
            && a.visibility == b.visibility
    };
    let fbs = &fallback.resources;
    let mut fi = 0usize;
    for m in &main.resources {
        if fi < fbs.len() && compatible_entry(m, &fbs[fi]) {
            fi += 1;
            continue;
        }
        if droppable(m) {
            continue; // 允许在 fallback 表缺席
        }
        return Err("resources");
    }
    if fi != fbs.len() {
        return Err("resources"); // fallback 不得多出条目
    }
    Ok(())
}

/// entry 名 → AST fn 项(兼容判定提取面;嵌套 mod 递归,裸名按首个匹配——
/// 与 reflection 裸名查找同一律;v1 语料不覆盖跨 mod 同名形态)。
fn find_fn<'a>(items: &'a [ast::Item], want: &str) -> Option<&'a ast::FnItem> {
    let bare = want.rsplit("::").next().unwrap_or(want);
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => {
                if f.name.name == bare {
                    return Some(f);
                }
            }
            ast::ItemKind::Mod(m) => {
                if let Some(f) = find_fn(&m.items, want) {
                    return Some(f);
                }
            }
            _ => {}
        }
    }
    None
}

/// 选择律单 entry 判定(RXS-0312 四分支;`allow_fallback` = 链深度余量,
/// fallback 自身递归判定时为 false——fallback 的 fallback 不支持,v1 冻结)。
#[allow(clippy::too_many_arguments)]
fn select_one(
    entry: &EntryCaps,
    unit: &UnitCapabilities,
    profile: &Profile,
    file: &ast::SourceFile,
    src: &str,
    allow_fallback: bool,
    out: &mut SelectionOutcome,
    errors: &mut Vec<SelectionError>,
) {
    // 分支一:有效集 ∩ forbidden ≠ ∅ → RX3021。
    let forbidden_hits: BTreeSet<CapabilityId> = entry
        .effective
        .intersection(&profile.forbidden)
        .copied()
        .collect();
    if !forbidden_hits.is_empty() {
        errors.push(SelectionError {
            code: E_CAP_FORBIDDEN.0,
            key: "capability.forbidden_used",
            detail: format!(
                "capability.forbidden_used: entry `{}` 的有效 requirement 集命中 profile `{}` 禁用能力 {}(RXS-0312 选择律分支一)",
                entry.name,
                profile.name,
                join_names(&forbidden_hits)
            ),
            span: entry.span,
        });
        return;
    }
    // 分支二:有效集 ⊆ provided → 照常发射。
    let missing: BTreeSet<CapabilityId> = entry
        .effective
        .difference(&profile.provided())
        .copied()
        .collect();
    if missing.is_empty() {
        out.records.push(SelectionRecord {
            name: entry.name.clone(),
            effective: sorted_names(&entry.effective),
            status: EntryStatus::Emitted,
            selected_entry: entry.name.clone(),
            missing: Vec::new(),
            forbidden_hits: Vec::new(),
        });
        return;
    }
    // 分支三:缺能力 → fallback 映射判定(链深度 1)。
    if allow_fallback && let Some(fb_name) = profile.fallbacks.get(&entry.name).cloned() {
        let Some(fb) = unit.entries.iter().find(|e| e.name == fb_name) else {
            errors.push(SelectionError {
                code: E_CAP_FALLBACK_INCOMPAT.0,
                key: "capability.fallback_incompatible",
                detail: format!(
                    "capability.fallback_incompatible: entry `{entry}` 的 fallback 映射目标 `{fb_name}` 不在编译单元 entry 集内(不兼容字段 = `fallback_entry`,RXS-0312)",
                    entry = entry.name
                ),
                span: entry.span,
            });
            return;
        };
        // 接口契约兼容判定(reflection 同一提取律,单一事实源)。
        let compat: Option<String> = match (
            find_fn(&file.items, &entry.name),
            find_fn(&file.items, &fb.name),
        ) {
            (Some(main_f), Some(fb_f)) => {
                match (
                    crate::reflection::extract_interface_facts(file, src, main_f),
                    crate::reflection::extract_interface_facts(file, src, fb_f),
                ) {
                    (Ok(main_facts), Ok(fb_facts)) => {
                        fallback_compatible(&main_facts, &fb_facts, &missing)
                            .err()
                            .map(|field| format!("`{field}`"))
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        Some(format!("interface extraction failed: {}", e.detail()))
                    }
                }
            }
            _ => Some("entry lookup failed".to_owned()),
        };
        if let Some(field) = compat {
            errors.push(SelectionError {
                code: E_CAP_FALLBACK_INCOMPAT.0,
                key: "capability.fallback_incompatible",
                detail: format!(
                    "capability.fallback_incompatible: entry `{}` → fallback `{}` 接口契约不兼容(不兼容字段 = {};RXS-0312 v1.3)",
                    entry.name, fb.name, field
                ),
                span: entry.span,
            });
            return;
        }
        // fallback 自身有效集仍须满足本选择律(递归判定,链深度 1)。
        let err_before = errors.len();
        select_one(fb, unit, profile, file, src, false, out, errors);
        if errors.len() != err_before {
            return;
        }
        out.records.push(SelectionRecord {
            name: entry.name.clone(),
            effective: sorted_names(&entry.effective),
            status: EntryStatus::Fallback,
            selected_entry: fb.name.clone(),
            missing: sorted_names(&missing),
            forbidden_hits: Vec::new(),
        });
        out.suppressed_names.insert(entry.name.clone());
        out.suppressed_defs.insert(entry.def_id);
        return;
    }
    // 无映射(或链深度耗尽)→ RX3020(消息携带缺失 ID + 首个引入 callee)。
    let introduced: Vec<String> = missing
        .iter()
        .map(|c| {
            format!(
                "`{}`(首个引入 callee: `{}`)",
                c.name(),
                entry
                    .introducer
                    .get(c)
                    .map_or(entry.name.as_str(), String::as_str)
            )
        })
        .collect();
    errors.push(SelectionError {
        code: E_CAP_MISSING.0,
        key: "capability.missing_required",
        detail: format!(
            "capability.missing_required: entry `{}` 缺失必需 capability {}(profile `{}` 未提供且无兼容 fallback 映射;RXS-0311/0312)",
            entry.name,
            introduced.join(", "),
            profile.name
        ),
        span: entry.span,
    });
}

/// 构建期选择律(RXS-0312;每 entry 独立判定,违例全量收集后 fail-closed)。
//@ spec: RXS-0312
pub fn select_entries(
    unit: &UnitCapabilities,
    profile: &Profile,
    file: &ast::SourceFile,
    src: &str,
) -> Result<SelectionOutcome, Vec<SelectionError>> {
    let mut out = SelectionOutcome {
        records: Vec::new(),
        profile_digest: profile.digest(),
        suppressed_names: BTreeSet::new(),
        suppressed_defs: HashSet::new(),
    };
    let mut errors: Vec<SelectionError> = Vec::new();
    let mut recorded: HashSet<String> = HashSet::new();
    for entry in &unit.entries {
        if recorded.contains(&entry.name) {
            continue;
        }
        let before = out.records.len();
        select_one(entry, unit, profile, file, src, true, &mut out, &mut errors);
        for r in &out.records[before..] {
            recorded.insert(r.name.clone());
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    out.records
        .sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Ok(out)
}

// ═══════════════════════ 运行期 snapshot 核验(RXS-0313) ═══════════════════════

/// 产物 profile 事实(核验输入面;digest + required 集)。
#[derive(Clone, Copy, Debug)]
pub struct ArtifactProfileFacts<'a> {
    /// 产物所选 profile 规范 digest。
    pub digest: &'a [u8; 32],
    /// 产物所选 profile 的 required capability 集。
    pub required: &'a [CapabilityId],
}

/// device capability snapshot 事实(运行期实测输入面)。
#[derive(Clone, Copy, Debug)]
pub struct SnapshotFacts<'a> {
    /// 运行期可用 capability 集。
    pub available: &'a BTreeSet<CapabilityId>,
}

/// snapshot 核验失败(typed `Err`;携带缺失 ID 表与 symbolic key 字面;by
/// construction 不含任何修复/重编/换 profile 路径,RXS-0313)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProfileSnapshotMismatch {
    /// snapshot 中缺失的 required capability ID 表(字节序)。
    pub missing: Vec<String>,
}

impl std::fmt::Display for ProfileSnapshotMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{SNAPSHOT_MISMATCH_KEY}: required capabilities missing from device capability snapshot: [{}] (fail-closed, RXS-0313)",
            self.missing.join(", ")
        )
    }
}

impl std::error::Error for ProfileSnapshotMismatch {}

/// 装配期/装载期核验原语(RXS-0313,镜像 RXS-0307 `verify_interface_pair`
/// 体例;host/safe 纯函数):产物 profile 的 required 集逐一对照 snapshot
/// 可用集——全部在场 → `Ok`;任一缺失 → `Err(ProfileSnapshotMismatch)`
/// 携带缺失 ID 表。**禁止**临时重编、**禁止**静默换 profile、**禁止**
/// 「尽力而为」降级;本函数体内不存在任何修复路径(by construction)。
/// 核验先于任何 pipeline 创建/资源绑定;失败后不产生部分装配状态。
//@ spec: RXS-0313
pub fn verify_profile_snapshot(
    artifact: &ArtifactProfileFacts<'_>,
    snapshot: &SnapshotFacts<'_>,
) -> Result<(), ProfileSnapshotMismatch> {
    let _ = artifact.digest; // digest 为产物身份事实,核验面 = required 集对照。
    let missing: Vec<String> = artifact
        .required
        .iter()
        .filter(|c| !snapshot.available.contains(c))
        .map(|c| c.name().to_owned())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ProfileSnapshotMismatch { missing })
    }
}

// ═══════════════════════ selection manifest(RXS-0312 报告律) ═══════════════════════

/// 编译单元级 selection manifest(`--emit=capabilities` 产物)。
#[derive(Clone, Debug)]
pub struct CapManifest {
    /// 本 profile 规范 digest(无 `--profile` = 空编码常量)。
    pub profile_digest: [u8; 32],
    /// per-entry 记录(按逻辑名规范键排序)。
    pub records: Vec<SelectionRecord>,
}

/// 构建 selection manifest(RXS-0312):有 profile = 选择律结果;无 profile
/// = 全 entry emitted(选择律不触发,行为与 M32 前逐字节一致,0 漂移)。
pub fn build_manifest(
    unit: &UnitCapabilities,
    selection: Option<&SelectionOutcome>,
) -> CapManifest {
    match selection {
        Some(sel) => CapManifest {
            profile_digest: sel.profile_digest,
            records: sel.records.clone(),
        },
        None => CapManifest {
            profile_digest: profile_none_digest(),
            records: unit
                .entries
                .iter()
                .map(|e| SelectionRecord {
                    name: e.name.clone(),
                    effective: sorted_names(&e.effective),
                    status: EntryStatus::Emitted,
                    selected_entry: e.name.clone(),
                    missing: Vec::new(),
                    forbidden_hits: Vec::new(),
                })
                .collect(),
        },
    }
}

/// JSON 串转义(与 reflection.rs 同一防御面)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_str_list(xs: &[String]) -> String {
    if xs.is_empty() {
        return "[]".to_owned();
    }
    format!(
        "[{}]",
        xs.iter()
            .map(|x| format!("\"{}\"", json_escape(x)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// manifest → 确定性 JSON 产物(键序固定、UTF-8、LF 行尾、整数不浮点;无
/// 绝对路径/文件名/时间戳/进程因素,RXS-0305 禁用面同律)。双次生成逐字节相等。
pub fn to_manifest_json(m: &CapManifest) -> String {
    let compiler_version = env!("CARGO_PKG_VERSION");
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{MANIFEST_SCHEMA_ID}\",\n"));
    s.push_str("  \"schema_version\": 1,\n");
    s.push_str("  \"compiler\": \"rurixc\",\n");
    s.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(compiler_version)
    ));
    s.push_str("  \"edition\": \"Rx0\",\n");
    s.push_str(&format!(
        "  \"selected_profile_digest\": \"{}\",\n",
        sha256::hex(&m.profile_digest)
    ));
    if m.records.is_empty() {
        s.push_str("  \"entries\": []\n");
    } else {
        s.push_str("  \"entries\": [\n");
        for (k, r) in m.records.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"name\": \"{}\", \"effective_requirements\": {}, \"status\": \"{}\", \"selected_entry\": \"{}\", \"missing\": {}, \"forbidden_hits\": {}}}{}\n",
                json_escape(&r.name),
                json_str_list(&r.effective),
                r.status.name(),
                json_escape(&r.selected_entry),
                json_str_list(&r.missing),
                json_str_list(&r.forbidden_hits),
                if k + 1 == m.records.len() { "" } else { "," },
            ));
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::source_map::SourceMap;
    use crate::span::{Edition, SourceId};

    fn parse_src(src: &str) -> (ast::SourceFile, SourceId) {
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        assert!(!diag.has_errors(), "测试源须解析干净");
        (file, id)
    }

    fn parse_and_resolve(src: &str) -> (ast::SourceFile, Resolutions) {
        let (file, _) = parse_src(src);
        let diag = DiagCtxt::new();
        let res = crate::resolve::resolve(&file, &diag);
        assert!(
            !diag.has_errors(),
            "测试源须解析+resolve 干净: {:?}",
            diag.emitted()
        );
        (file, res)
    }

    fn caps_of(src: &str) -> UnitCapabilities {
        let (file, res) = parse_and_resolve(src);
        build_unit_caps(&file, src, &res)
    }

    fn entry<'a>(unit: &'a UnitCapabilities, name: &str) -> &'a EntryCaps {
        unit.entries
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "entry {name} 应在枚举面: {:?}",
                    unit.entries.iter().map(|e| &e.name).collect::<Vec<_>>()
                )
            })
    }

    fn names(set: &BTreeSet<CapabilityId>) -> Vec<&'static str> {
        set.iter().map(|c| c.name()).collect()
    }

    /// 显式 `#[requires]`:多条叠加取并集、重复 ID 去重、顺序无关。
    //@ spec: RXS-0311
    #[test]
    fn explicit_requires_union_across_attrs() {
        let src = r#"
#[requires("rt.pipeline", "mesh.task")]
#[requires("rt.pipeline", "rt.ray_query")]
kernel fn kmain() {}
"#;
        let (file, _) = parse_src(src);
        let ast::ItemKind::Fn(_) = &file.items[0].kind else {
            panic!("首项须为 fn");
        };
        let req = extract_requires(&file.items[0].attrs, src).expect("解析须成功");
        assert_eq!(names(&req), ["mesh.task", "rt.pipeline", "rt.ray_query"]);
    }

    /// 闭集外 ID / 空列表 / 非字符串实参 → RX3023 拒(经 check_requires 端到端)。
    //@ spec: RXS-0311
    #[test]
    fn unknown_id_and_malformed_requires_rejected() {
        // 注:裸路径形态 `#[requires]` 与多段路径实参 `#[requires(rt.pipeline)]`
        // 由 parser 层先拒(RX0008),不进 capability 检查面;单段标识符实参
        // `#[requires(rt)]` 解析为 MetaInner::Meta,由本表面拒(非字符串字面量)。
        let cases: &[&str] = &[
            "#[requires(\"rt.magic_boost\")]\nkernel fn k() {}",
            "#[requires()]\nkernel fn k() {}",
            "#[requires(rt)]\nkernel fn k() {}",
        ];
        for src in cases {
            let src = *src;
            let diag = DiagCtxt::new();
            let mut sm = SourceMap::new();
            let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
            let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
            let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
            check_requires(&file, src, &diag);
            let codes: Vec<u16> = diag
                .emitted()
                .iter()
                .filter_map(|d| d.code.map(|c| c.0))
                .collect();
            assert_eq!(codes, [3023], "{src} 须恰一发 RX3023: {codes:?}");
            let text = diag
                .emitted()
                .iter()
                .flat_map(|d| d.args.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                text.contains("capability.unknown_id"),
                "诊断须携带 symbolic key 字面: {text}"
            );
        }
    }

    /// 附着纪律:host fn / const fn / 非函数 item / 非 device 关联函数 →
    /// RX3023;kernel/device/着色阶段 fn 合法。
    //@ spec: RXS-0311
    #[test]
    fn requires_attachment_target_discipline() {
        let cases: &[(&str, usize)] = &[
            ("#[requires(\"rt.pipeline\")]\nfn host_fn() {}", 1),
            ("#[requires(\"rt.pipeline\")]\nconst fn c() -> u32 { 1 }", 1),
            ("#[requires(\"rt.pipeline\")]\nstruct S { x: f32 }", 1),
            ("#[requires(\"rt.pipeline\")]\nmod m {}", 1),
            ("#[requires(\"rt.pipeline\")]\nkernel fn k() {}", 0),
            ("#[requires(\"rt.pipeline\")]\ndevice fn d() {}", 0),
            ("#[requires(\"rt.pipeline\")]\nraygen fn rg() {}", 0),
            ("#[requires(\"rt.pipeline\")]\ntask fn t() {}", 0),
        ];
        for (src, want) in cases {
            let src = *src;
            let diag = DiagCtxt::new();
            let mut sm = SourceMap::new();
            let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
            let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
            let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
            assert!(!diag.has_errors(), "解析须干净: {src}");
            check_requires(&file, src, &diag);
            let n = diag.emitted().len();
            assert_eq!(n, *want, "{src} 诊断数不符");
            assert!(
                diag.emitted()
                    .iter()
                    .all(|d| d.code == Some(ErrorCode(3023))),
                "全为 RX3023"
            );
        }
        // 泛型着色函数:标注仍校验闭集(未知 ID 拒)。
        let src = "#[requires(\"rt.magic\")]\nkernel fn g<T>() {}";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        check_requires(&file, src, &diag);
        assert_eq!(diag.emitted().len(), 1, "泛型 fn 未知 ID 仍拒");
    }

    /// RXS-0349:G9.2 加性两位实位(`submit.dgc`/`bindless.descriptor_buffer`)
    /// 进闭集;预留位(`bindless.descriptor_heap`/`submit.execution_set`)**不在**
    /// 闭集(只预留不实现——消费性引用 = 闭集外 ID → RX3023)。
    //@ spec: RXS-0349
    #[test]
    fn g92_additive_two_real_ids() {
        // 两位实位解析合法。
        assert_eq!(
            CapabilityId::from_name("submit.dgc"),
            Some(CapabilityId::SubmitDgc)
        );
        assert_eq!(
            CapabilityId::from_name("bindless.descriptor_buffer"),
            Some(CapabilityId::BindlessDescriptorBuffer)
        );
        assert_eq!(CapabilityId::SubmitDgc.name(), "submit.dgc");
        assert_eq!(
            CapabilityId::BindlessDescriptorBuffer.name(),
            "bindless.descriptor_buffer"
        );
        // 闭集恰十二项(v1 十项 0-byte + 加性两位)。
        assert_eq!(CapabilityId::ALL.len(), 12);
        // 预留位不在闭集(消费性引用 = RX3023;只预留不实现)。
        assert_eq!(CapabilityId::from_name("bindless.descriptor_heap"), None);
        assert_eq!(CapabilityId::from_name("submit.execution_set"), None);
        // 预留位进 #[requires] = 闭集外 ID 拒(消费行为不存在)。
        let src = "#[requires(\"submit.execution_set\")]\nkernel fn k() {}";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        check_requires(&file, src, &diag);
        assert!(
            diag.emitted()
                .iter()
                .any(|d| d.code == Some(ErrorCode(3023))),
            "预留位 submit.execution_set 消费性引用须 RX3023 拒"
        );
    }

    /// 隐式推导:stage 映射(raygen/anyhit/intersection/callable/task/miss/
    /// closesthit)+ AccelStruct compute 签名形参;compute/mesh/kernel 无 stage 推导。
    //@ spec: RXS-0311
    #[test]
    fn implicit_stage_and_param_mapping() {
        let (file, _) = parse_src(
            "raygen fn rg() {}\nanyhit fn ah() {}\nintersection fn is_() {}\ncallable fn cb() {}\ntask fn tk() {}\nmiss fn mi() {}\nclosesthit fn ch() {}\ncompute fn cp() {}\nkernel fn km(tlas: AccelStruct) {}\n",
        );
        let fns: Vec<&ast::FnItem> = file
            .items
            .iter()
            .filter_map(|it| match &it.kind {
                ast::ItemKind::Fn(f) => Some(f),
                _ => None,
            })
            .collect();
        let caps = |i: usize| implicit_for_fn(fns[i], &[], None);
        assert_eq!(names(&caps(0)), ["rt.pipeline"], "raygen");
        assert_eq!(names(&caps(1)), ["rt.any_hit"], "anyhit");
        assert_eq!(
            names(&caps(2)),
            ["rt.intersection.procedural"],
            "intersection"
        );
        assert_eq!(names(&caps(3)), ["rt.callable"], "callable");
        assert_eq!(names(&caps(4)), ["mesh.task"], "task");
        assert_eq!(names(&caps(5)), ["rt.pipeline"], "miss");
        assert_eq!(names(&caps(6)), ["rt.pipeline"], "closesthit");
        assert!(caps(7).is_empty(), "compute 阶段无 stage 推导");
        assert_eq!(
            names(&caps(8)),
            ["rt.ray_query"],
            "AccelStruct compute 签名形参"
        );
    }

    /// 隐式推导:intrinsic 映射(trace_ray → rt.pipeline;ray_query_initialize →
    /// rt.ray_query;report_intersection → rt.intersection.procedural;
    /// execute_callable → rt.callable;ignore_intersection → rt.any_hit;
    /// `RayQuery` 局部变量 → rt.ray_query)。用户同名遮蔽不推导。
    //@ spec: RXS-0311
    #[test]
    fn implicit_intrinsic_mapping() {
        let (file, _) = parse_src(
            "kernel fn k() { trace_ray(); report_intersection(); execute_callable(); ignore_intersection(); let q: RayQuery; ray_query_initialize(); }\n",
        );
        let ast::ItemKind::Fn(f) = &file.items[0].kind else {
            panic!("首项须为 fn");
        };
        let caps = implicit_for_fn(f, &[], None);
        assert_eq!(
            names(&caps),
            [
                "rt.any_hit",
                "rt.callable",
                "rt.intersection.procedural",
                "rt.pipeline",
                "rt.ray_query"
            ]
        );
        // 用户同名遮蔽:resolve 在场时解析到用户 DefId → 不推导。
        let (file2, res) = parse_and_resolve(
            "fn trace_ray() {}\ndevice fn d() { trace_ray(); }\nkernel fn k() { d(); }\n",
        );
        let ast::ItemKind::Fn(f2) = &file2.items[1].kind else {
            panic!("次项须为 fn");
        };
        let caps2 = implicit_for_fn(f2, &[], Some(&res));
        assert!(
            caps2.is_empty(),
            "用户同名 trace_ray 非 RT intrinsic,不推导"
        );
    }

    /// 调用图并集律:entry 有效集 = 显式 ∪ 隐式 ∪ 全部静态可达 callee 并集;
    /// 首个引入 callee 记录(自身引入 = entry 自身名;两级链传递)。
    //@ spec: RXS-0311
    #[test]
    fn call_graph_union_and_introducer() {
        let src = r#"
#[requires("rt.sbt_user_data")]
device fn leaf() {}

device fn mid() { leaf(); }

#[requires("rt.pipeline")]
device fn other() {}

kernel fn kmain() { mid(); other(); }
"#;
        let unit = caps_of(src);
        let k = entry(&unit, "kmain");
        assert_eq!(
            names(&k.effective),
            ["rt.pipeline", "rt.sbt_user_data"],
            "有效集 = 显式∪隐式∪可达 callee 并集(两级链)"
        );
        assert_eq!(
            k.introducer
                .get(&CapabilityId::RtSbtUserData)
                .map(String::as_str),
            Some("leaf"),
            "首个引入 callee = leaf(链尾)"
        );
        assert_eq!(
            k.introducer
                .get(&CapabilityId::RtPipeline)
                .map(String::as_str),
            Some("other")
        );
        // 自身引入优先于 callee:entry 自带 requires 时引入者 = entry 自身。
        let src2 = "#[requires(\"mesh.task\")]\nkernel fn k2() { h(); }\n#[requires(\"mesh.task\")]\ndevice fn h() {}\n";
        let unit2 = caps_of(src2);
        let k2 = entry(&unit2, "k2");
        assert_eq!(
            k2.introducer
                .get(&CapabilityId::MeshTask)
                .map(String::as_str),
            Some("k2"),
            "自身引入时引入者 = entry 自身名"
        );
        // 不可达 fn 的 requirement 不并入。
        let unit3 =
            caps_of("#[requires(\"rt.pipeline\")]\ndevice fn orphan() {}\nkernel fn k3() {}\n");
        assert!(
            entry(&unit3, "k3").effective.is_empty(),
            "不可达 callee 不并入"
        );
    }

    /// 泛型着色函数不产 entry(RXS-0304 口径);泛型 device callee 的
    /// requirement 随调用边并入调用方。
    //@ spec: RXS-0311
    #[test]
    fn generic_entry_excluded_generic_callee_propagates() {
        let src = "#[requires(\"rt.pipeline\")]\ndevice fn g<T>() {}\nkernel fn kmain() { g::<u32>(); }\n#[requires(\"mesh.task\")]\nkernel fn gen<T>() {}\n";
        let unit = caps_of(src);
        let k = entry(&unit, "kmain");
        assert_eq!(names(&k.effective), ["rt.pipeline"], "泛型 callee 并集照常");
        assert!(
            unit.entries.iter().all(|e| e.name != "gen"),
            "泛型着色函数不产 entry"
        );
    }

    /// profile digest 确定性:同 profile 双次解析 digest 相等;JSON 键序/集内
    /// 序扰动 digest 不变;任一字段变化 digest 必变;无 profile 恒空编码常量。
    //@ spec: RXS-0312
    #[test]
    fn profile_digest_deterministic() {
        let text = r#"{
  "schema": "rurix.profile.v1",
  "name": "high",
  "version": "1.0.0",
  "required": ["rt.pipeline", "rt.ray_query"],
  "optional": ["mesh.task"],
  "forbidden": [],
  "fallbacks": {"kmain": "kmain_fallback"}
}"#;
        let p1 = parse_profile(text).expect("profile 解析须成功");
        let p2 = parse_profile(text).expect("双次解析");
        assert_eq!(p1.canonical_bytes(), p2.canonical_bytes());
        assert_eq!(p1.digest(), p2.digest());
        assert!(p1.canonical_bytes().starts_with(b"rurix.profile.v1\0"));
        // 集内元素序扰动 → 排序编码,digest 不变。
        let shuffled = text.replace(
            "\"required\": [\"rt.pipeline\", \"rt.ray_query\"]",
            "\"required\": [\"rt.ray_query\", \"rt.pipeline\"]",
        );
        let p3 = parse_profile(&shuffled).expect("扰序解析");
        assert_eq!(p1.digest(), p3.digest(), "集内序不影响 digest");
        // 字段变化 → digest 必变。
        let changed = text.replace("\"rt.ray_query\"", "\"rt.any_hit\"");
        let p4 = parse_profile(&changed).expect("变更解析");
        assert_ne!(p1.digest(), p4.digest());
        // 无 profile 恒空编码常量(M31 基线字面,0 漂移)。
        assert_eq!(
            sha256::hex(&profile_none_digest()),
            "2997fd21a324a39e63cd1da6970db88c511e8d025d24fbce0bbb94c5ea8c28b6"
        );
    }

    /// profile 闭集违例:三集两两相交拒 / schema 常量不符拒 / 闭集外 ID 拒
    /// (UnknownId = RX3023 同类,与 Malformed 分型)。
    //@ spec: RXS-0312
    #[test]
    fn profile_closed_set_violations_rejected() {
        let base = |required: &str, optional: &str, forbidden: &str| {
            format!(
                r#"{{"schema": "rurix.profile.v1", "name": "p", "version": "1", "required": {required}, "optional": {optional}, "forbidden": {forbidden}, "fallbacks": {{}}}}"#
            )
        };
        // required ∩ forbidden 非空。
        assert!(matches!(
            parse_profile(&base("[\"rt.pipeline\"]", "[]", "[\"rt.pipeline\"]")),
            Err(ProfileError::Malformed(_))
        ));
        // required ∩ optional 非空。
        assert!(matches!(
            parse_profile(&base("[\"rt.pipeline\"]", "[\"rt.pipeline\"]", "[]")),
            Err(ProfileError::Malformed(_))
        ));
        // optional ∩ forbidden 非空。
        assert!(matches!(
            parse_profile(&base("[]", "[\"rt.pipeline\"]", "[\"rt.pipeline\"]")),
            Err(ProfileError::Malformed(_))
        ));
        // schema 常量不符。
        assert!(matches!(
            parse_profile(
                &base("[]", "[]", "[]")
                    .replace("rurix.profile.v1", "rurix.profile.v9")
                    .as_str()
            ),
            Err(ProfileError::Malformed(_))
        ));
        // 闭集外 ID → UnknownId(RX3023 同类)。
        let Err(ProfileError::UnknownId(d)) = parse_profile(&base("[\"rt.magic\"]", "[]", "[]"))
        else {
            panic!("闭集外 ID 须 UnknownId 拒");
        };
        assert!(d.contains("capability.unknown_id") && d.contains("rt.magic"));
    }

    /// 选择律四分支(RXS-0312):② 有效集 ⊆ provided → emitted;① ∩ forbidden
    /// ≠ ∅ → RX3021(key + 违禁 ID);③ 无映射 → RX3020(key + 缺失 ID + 首个
    /// 引入 callee);fallback 兼容 → 选 fallback 主 variant 不发射。
    //@ spec: RXS-0312
    #[test]
    fn selection_law_four_branches() {
        let src = r#"
kernel fn kmain(tlas: AccelStruct, out: ViewMut<global, f32>, n: u32) {
    let i = n;
    out[i] = 1.0;
}

kernel fn kmain_fallback(out: ViewMut<global, f32>, n: u32) {
    out[n] = 0.0;
}

kernel fn plain(out: ViewMut<global, f32>) {
    out[0] = 1.0;
}
"#;
        let (file, res) = parse_and_resolve(src);
        let unit = build_unit_caps(&file, src, &res);
        let mk =
            |required: &[CapabilityId], forbidden: &[CapabilityId], fb: &[(&str, &str)]| Profile {
                name: "p".to_owned(),
                version: "1".to_owned(),
                required: required.iter().copied().collect(),
                optional: BTreeSet::new(),
                forbidden: forbidden.iter().copied().collect(),
                fallbacks: fb
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            };
        // ② 全提供:三 entry emitted,fallback 映射不触发。
        let high = mk(
            &[CapabilityId::RtRayQuery],
            &[],
            &[("kmain", "kmain_fallback")],
        );
        let out = select_entries(&unit, &high, &file, src).expect("高 profile 须全绿");
        assert_eq!(out.records.len(), 3);
        assert!(out.records.iter().all(|r| r.status == EntryStatus::Emitted));
        assert!(out.suppressed_names.is_empty());
        // ③ 缺能力有映射且兼容 → fallback;主 variant 不发射。
        let low_fb = mk(&[], &[], &[("kmain", "kmain_fallback")]);
        let out = select_entries(&unit, &low_fb, &file, src).expect("fallback 须选中");
        let k = out
            .records
            .iter()
            .find(|r| r.name == "kmain")
            .expect("kmain 记录");
        assert_eq!(k.status, EntryStatus::Fallback);
        assert_eq!(k.selected_entry, "kmain_fallback");
        assert_eq!(k.missing, ["rt.ray_query"]);
        assert!(out.suppressed_names.contains("kmain"));
        // ① 违禁:命中 forbidden → RX3021(key + 违禁 ID)。
        let forb = mk(
            &[CapabilityId::RtRayQuery],
            &[CapabilityId::RtRayQuery],
            &[],
        );
        let errs = select_entries(&unit, &forb, &file, src).expect_err("违禁须红");
        let e = errs.iter().find(|e| e.code == 3021).expect("RX3021");
        assert!(e.detail.contains("capability.forbidden_used"));
        assert!(e.detail.contains("rt.ray_query"));
        // ③ 无映射 → RX3020(key + 缺失 ID + 首个引入 callee = entry 自身)。
        let low = mk(&[], &[], &[]);
        let errs = select_entries(&unit, &low, &file, src).expect_err("缺能力须红");
        let e = errs.iter().find(|e| e.code == 3020).expect("RX3020");
        assert!(e.detail.contains("capability.missing_required"));
        assert!(e.detail.contains("rt.ray_query"));
        assert!(
            e.detail.contains("kmain"),
            "引入者 = entry 自身名: {}",
            e.detail
        );
    }

    /// 兼容判定:accel 条目在 fallback 缺席合法(缺失 ∈ {rt.ray_query,
    /// rt.pipeline});fallback 多出条目非法;io/push_constants 不等非法且字段名正确。
    //@ spec: RXS-0312
    #[test]
    fn fallback_compat_accel_absence_and_extra_and_io() {
        let main_src = "kernel fn kmain(tlas: AccelStruct, out: ViewMut<global, f32>, n: u32) { out[n] = 1.0; }\n";
        let fb_src =
            "kernel fn kmain_fallback(out: ViewMut<global, f32>, n: u32) { out[n] = 0.0; }\n";
        let (mf, _) = parse_src(main_src);
        let (ff, _) = parse_src(fb_src);
        let ast::ItemKind::Fn(main_fn) = &mf.items[0].kind else {
            panic!()
        };
        let ast::ItemKind::Fn(fb_fn) = &ff.items[0].kind else {
            panic!()
        };
        let main_facts =
            crate::reflection::extract_interface_facts(&mf, main_src, main_fn).expect("主接口提取");
        let fb_facts = crate::reflection::extract_interface_facts(&ff, fb_src, fb_fn)
            .expect("fallback 接口提取");
        let missing: BTreeSet<CapabilityId> = [CapabilityId::RtRayQuery].into_iter().collect();
        assert!(
            fallback_compatible(&main_facts, &fb_facts, &missing).is_ok(),
            "accel 缺席合法(缺失 rt.ray_query)"
        );
        // 缺失能力 ∉ {rt.ray_query, rt.pipeline} → accel 缺席不豁免。
        let other_missing: BTreeSet<CapabilityId> = [CapabilityId::MeshTask].into_iter().collect();
        assert_eq!(
            fallback_compatible(&main_facts, &fb_facts, &other_missing),
            Err("resources"),
            "非映射缺失下 accel 缺席 = resources 不兼容"
        );
        // fallback 多出条目非法(主表不含的条目,push_constants 一致以隔离 resources 轴)。
        let bare_main_src =
            "kernel fn kmain(out: ViewMut<global, f32>, n: u32) { out[n] = 1.0; }\n";
        let extra_src = "kernel fn kmain_fallback(out: ViewMut<global, f32>, extra: ViewMut<global, f32>, n: u32) { out[n] = 0.0; }\n";
        let (bf, _) = parse_src(bare_main_src);
        let (ef, _) = parse_src(extra_src);
        let ast::ItemKind::Fn(bare_fn) = &bf.items[0].kind else {
            panic!()
        };
        let ast::ItemKind::Fn(extra_fn) = &ef.items[0].kind else {
            panic!()
        };
        let bare_facts = crate::reflection::extract_interface_facts(&bf, bare_main_src, bare_fn)
            .expect("裸主提取");
        let extra_facts = crate::reflection::extract_interface_facts(&ef, extra_src, extra_fn)
            .expect("多出行提取");
        assert_eq!(
            fallback_compatible(&bare_facts, &extra_facts, &missing),
            Err("resources"),
            "fallback 多出条目非法"
        );
        // push_constants 不等非法(字段名 = push_constants)。
        let pc_src =
            "kernel fn kmain_fallback(out: ViewMut<global, f32>, n: u64) { out[0] = 0.0; }\n";
        let (pf, _) = parse_src(pc_src);
        let ast::ItemKind::Fn(pc_fn) = &pf.items[0].kind else {
            panic!()
        };
        let pc_facts =
            crate::reflection::extract_interface_facts(&pf, pc_src, pc_fn).expect("pc 提取");
        assert_eq!(
            fallback_compatible(&main_facts, &pc_facts, &missing),
            Err("push_constants")
        );
        // stage 不等非法(字段名 = stage)。
        let vs_src = "vertex fn kmain_fallback() {}\n";
        let (vf, _) = parse_src(vs_src);
        let ast::ItemKind::Fn(vs_fn) = &vf.items[0].kind else {
            panic!()
        };
        let vs_facts =
            crate::reflection::extract_interface_facts(&vf, vs_src, vs_fn).expect("vs 提取");
        assert_eq!(
            fallback_compatible(&main_facts, &vs_facts, &missing),
            Err("stage")
        );
    }

    /// fallback 链深度 1:fallback 自身缺能力 → 其 fallback 不再支持(RX3020
    /// 归于 fallback entry;不再递归)。
    //@ spec: RXS-0312
    #[test]
    fn fallback_chain_depth_one() {
        let src = r#"
#[requires("rt.pipeline")]
kernel fn kmain() {}

#[requires("rt.ray_query")]
kernel fn kmain_fallback() {}
"#;
        let (file, res) = parse_and_resolve(src);
        let unit = build_unit_caps(&file, src, &res);
        let profile = Profile {
            name: "p".to_owned(),
            version: "1".to_owned(),
            required: BTreeSet::new(),
            optional: BTreeSet::new(),
            forbidden: BTreeSet::new(),
            fallbacks: [("kmain".to_owned(), "kmain_fallback".to_owned())]
                .into_iter()
                .collect(),
        };
        let errs = select_entries(&unit, &profile, &file, src).expect_err("深度 1 须红");
        assert!(
            errs.iter()
                .any(|e| e.code == 3020 && e.detail.contains("kmain_fallback")),
            "fallback 自身缺能力 → RX3020 归 fallback entry(链深度 1): {errs:?}"
        );
    }

    /// verify_profile_snapshot:满足 → Ok;缺失 → typed Err 携带缺失 ID 表与
    /// symbolic key 字面;体内无修复路径(by construction)。
    //@ spec: RXS-0313
    #[test]
    fn verify_profile_snapshot_fail_closed() {
        let digest = profile_none_digest();
        let required = [CapabilityId::RtPipeline, CapabilityId::RtRayQuery];
        let artifact = ArtifactProfileFacts {
            digest: &digest,
            required: &required,
        };
        let full: BTreeSet<CapabilityId> = required.iter().copied().collect();
        assert!(
            verify_profile_snapshot(&artifact, &SnapshotFacts { available: &full }).is_ok(),
            "全量可用 → Ok"
        );
        let partial: BTreeSet<CapabilityId> = [CapabilityId::RtPipeline].into_iter().collect();
        let err = verify_profile_snapshot(
            &artifact,
            &SnapshotFacts {
                available: &partial,
            },
        )
        .expect_err("缺失须 typed Err");
        assert_eq!(err.missing, ["rt.ray_query"], "缺失 ID 表");
        let text = err.to_string();
        assert!(text.contains("capability.runtime_snapshot_mismatch"));
        assert!(text.contains("rt.ray_query"));
        // 空 snapshot:全缺失(序 = required 声明序)。
        let empty: BTreeSet<CapabilityId> = BTreeSet::new();
        let err = verify_profile_snapshot(&artifact, &SnapshotFacts { available: &empty })
            .expect_err("空 snapshot 须 Err");
        assert_eq!(err.missing.len(), 2);
    }

    /// 无 `--profile` 路径:选择律不触发,manifest 全 entry emitted 且 digest
    /// 恒空编码常量;manifest JSON 双次逐字节相等、无路径/CRLF(RXS-0312/0305)。
    //@ spec: RXS-0312
    #[test]
    fn no_profile_manifest_zero_drift() {
        let unit = caps_of("kernel fn plain(out: ViewMut<global, f32>) { out[0] = 1.0; }\n");
        let m1 = build_manifest(&unit, None);
        let m2 = build_manifest(&unit, None);
        assert_eq!(m1.profile_digest, profile_none_digest());
        let (j1, j2) = (to_manifest_json(&m1), to_manifest_json(&m2));
        assert_eq!(j1, j2, "manifest JSON 双次逐字节相等");
        assert!(j1.ends_with("}\n") && !j1.contains('\r'));
        assert!(!j1.contains("test.rx"));
        assert_eq!(m1.records.len(), 1);
        assert_eq!(m1.records[0].status, EntryStatus::Emitted);
        assert_eq!(m1.records[0].selected_entry, "plain");
        assert!(m1.records[0].effective.is_empty());
        // 含 requirement 的 entry:无 profile 下 effective 真值化进 manifest,
        // 选择律仍不触发(status = emitted)。
        let unit2 = caps_of("kernel fn k(tlas: AccelStruct) {}\n");
        let m3 = build_manifest(&unit2, None);
        assert_eq!(m3.records[0].effective, ["rt.ray_query"]);
        assert_eq!(m3.records[0].status, EntryStatus::Emitted);
    }

    /// manifest 报告面:fallback 选择进记录(逻辑名 → fallback 实体映射可查);
    /// 键序固定;per-entry 字段闭集(RXS-0312)。
    //@ spec: RXS-0312
    #[test]
    fn manifest_records_fallback_selection() {
        let src = "kernel fn kmain(tlas: AccelStruct, out: ViewMut<global, f32>, n: u32) { out[n] = 1.0; }\nkernel fn kmain_fallback(out: ViewMut<global, f32>, n: u32) { out[n] = 0.0; }\n";
        let (file, res) = parse_and_resolve(src);
        let unit = build_unit_caps(&file, src, &res);
        let profile = Profile {
            name: "low".to_owned(),
            version: "1".to_owned(),
            required: BTreeSet::new(),
            optional: BTreeSet::new(),
            forbidden: BTreeSet::new(),
            fallbacks: [("kmain".to_owned(), "kmain_fallback".to_owned())]
                .into_iter()
                .collect(),
        };
        let sel = select_entries(&unit, &profile, &file, src).expect("fallback 须选中");
        let m = build_manifest(&unit, Some(&sel));
        assert_eq!(m.profile_digest, profile.digest());
        let j = to_manifest_json(&m);
        assert!(j.contains("\"status\": \"fallback\""));
        assert!(j.contains("\"selected_entry\": \"kmain_fallback\""));
        assert!(j.contains("\"missing\": [\"rt.ray_query\"]"));
        assert!(j.contains("\"schema\": \"rurix.capability-selection.v1\""));
        // 逻辑名 → 实体映射可查:kmain → fallback;kmain_fallback → 自身。
        let kmain = m
            .records
            .iter()
            .find(|r| r.name == "kmain")
            .expect("kmain 记录");
        assert_eq!(kmain.selected_entry, "kmain_fallback");
        let fb = m
            .records
            .iter()
            .find(|r| r.name == "kmain_fallback")
            .expect("fallback 记录");
        assert_eq!(fb.selected_entry, "kmain_fallback");
    }
}
