//! 着色阶段类型面前端检查(spec 条款 RXS-0153 ~ RXS-0156,spec/shader_stages.md;
//! RFC-0002;cargo feature `shader-stages`)。**AST 层检查**(纯类型面/语法面,无
//! codegen / 绑定布局推导 / 纹理内存模型映射——分属 G2.2/G2.3 与 06 §4.2 禁区)。
//!
//! - **RXS-0153 着色阶段函数着色**:着色阶段(`vertex`/`fragment`/`compute`/`mesh`/
//!   `task` + RT `raygen`/`closesthit`/`anyhit`/`miss`)取 **kernel 入口着色**
//!   (parser 置 [`FnColor::Kernel`] + `stage` 标记);直接调用着色阶段入口 / 着色阶段
//!   体内调 host 着色函数等跨着色非法调用**复用既有 `RX3001`**(见 [`crate::coloring`]),
//!   本模块不另发码(RFC-0002 §5「复用既有 RX3001」)。
//! - **RXS-0154 阶段专属 I/O 语义类型** → `RX3011`:着色阶段 I/O 聚合类型的字段须携带
//!   `#[interpolate(..)]` 或 `#[builtin(..)]`(**无标注字段编译期拒绝**,RFC-0002 §9 Q2,
//!   P-01 strict-only);未知 builtin 名 / 未知插值限定 → `RX3011`。
//! - **RXS-0155 阶段间接口类型契约** → `RX3012`:`vertex` 输出与 `fragment` 输入的
//!   插值 varying 字段(名 + 类型 + 插值限定)须兼容;不兼容 → `RX3012`。
//! - **RXS-0156 资源句柄 / 纹理采样器参数化类型面** → `RX3013`:`Texture2D<F>` /
//!   `Sampler` 仅可作**着色阶段签名形参**;出现在返回位置 / 结构体字段 → `RX3013`;
//!   首批仅 `Texture2D<F>` + `Sampler`,其余纹理维度(`Texture1D`/`Texture3D`/
//!   `TextureCube`/`*Array`)**defer** → `RX3013`(RFC-0002 §9 Q4)。
//!   **纹理/采样器仅类型形态,不承诺任何采样/内存语义**(06 §4.2 禁区)。
//! - **RXS-0223 采样超集句柄增补**(G3.3,RFC-0013 §4.B1):支持集扩为
//!   `Texture2D<F>`/`TextureRw2D<F>` + `Sampler`/`SamplerCmp`(位置纪律同
//!   RXS-0156:仅着色阶段签名形参);方法族 typeck 面见 [`crate::typeck`]
//!   (RX3014 扩类别)。

use std::collections::{HashMap, HashSet};

use crate::ast::{
    Attr, FieldDef, FnItem, Item, ItemKind, LitKind, MetaInner, MetaKind, ShaderStage, SourceFile,
    Ty, TyKind, VariantBody,
};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::span::Span;

pub const E_STAGE_IO: ErrorCode = ErrorCode(3011); // RX3011(RXS-0154)
pub const E_STAGE_INTERFACE: ErrorCode = ErrorCode(3012); // RX3012(RXS-0155 / RXS-0244 扩类别)
pub const E_RESOURCE_HANDLE: ErrorCode = ErrorCode(3013); // RX3013(RXS-0156 / RXS-0245 扩类别)
pub const E_MESH_ENTRY: ErrorCode = ErrorCode(3017); // RX3017(RXS-0243;mesh/task 入口标注)

/// 已知内建变量名(RFC-0002 §4.2;type-level 内建变量类型化)。RT builtins(RFC-0013
/// §4.E4,RXS-0245)沿 compute builtins snake_case 谱系加入已知集(阶段×合法性矩阵的
/// 阶段维度落 body/coloring 层,本类型面仅承认名字合法、防误判 RX3011)。
const KNOWN_BUILTINS: &[&str] = &[
    "position",
    "vertex_id",
    "instance_id",
    "frag_coord",
    "thread_id",
    "front_facing",
    "depth",
    "primitive_id",
    // RT builtins(RXS-0245;全 RT 阶段)。
    "launch_id",
    "launch_size",
    // RT builtins(intersection/anyhit/closesthit/miss)。
    "world_ray_origin",
    "world_ray_direction",
    "ray_t_min",
    // RT builtins(anyhit/closesthit)。
    "hit_t",
    "hit_kind",
    // RT builtins(intersection/anyhit/closesthit)。
    "primitive_index",
];

/// 已知插值限定(RFC-0002 §4.2;片元输入插值方式)。
const KNOWN_INTERP: &[&str] = &[
    "perspective",
    "linear",
    "flat",
    "centroid",
    "noperspective",
    "sample",
];

/// 支持的资源句柄类型名(RFC-0002 §9 Q4 首批 `Texture2D`/`Sampler`;G3.3 RXS-0223
/// 增补 `TextureRw2D<F>` storage image + `SamplerCmp` 比较采样器。其余纹理维度
/// 〔`Texture1D`/`Texture3D`/`TextureCube`/`*Array`〕维持 defer → RX3013)。
const SUPPORTED_HANDLES: &[&str] = &["Texture2D", "TextureRw2D", "Sampler", "SamplerCmp"];

/// 着色阶段类型面检查入口(driver / ui_golden / conformance 复用)。`src` = 主源文本,
/// 供读取标注字面量值(`#[numthreads(x,y,z)]` / `#[outputs(...)]`,RXS-0243)。
pub fn check(file: &SourceFile, src: &str, diag: &DiagCtxt) {
    let structs = collect_structs(&file.items);
    let mut io_struct_names: HashSet<String> = HashSet::new();
    let mut vertex_out_interp: Vec<(String, String, String)> = Vec::new();
    collect_stage_io(
        &file.items,
        &structs,
        &mut io_struct_names,
        &mut vertex_out_interp,
    );

    // RXS-0154:着色阶段 I/O 结构体字段标注(无标注 / 未知名 → RX3011)。
    for name in &io_struct_names {
        if let Some(info) = structs.get(name) {
            for f in &info.fields {
                check_io_field(f, diag);
            }
        }
    }

    // RXS-0155 / RXS-0156:逐 item 走查(接口契约 + 资源句柄位置)。
    check_items(&file.items, &structs, &vertex_out_interp, diag);

    // RXS-0243:mesh/task 入口标注契约(#[numthreads] + #[outputs] 必备/合法 → RX3017)。
    check_stage_entries(&file.items, src, diag);

    // RXS-0244:RT payload / hit attribute / callable data 显式类型契约逐字段比对
    // (单编译单元三件套配对域;错配 → RX3012 扩类别)。
    check_payload_contracts(&file.items, &structs, diag);
}

/// span → 源切片(标注字面量值读取;越界 / 非主文件 → 空串,保守不误报)。
fn snippet(src: &str, span: Span) -> &str {
    src.get(span.lo.0 as usize..span.hi.0 as usize)
        .unwrap_or("")
}

/// 命名字段结构体信息(owned;字段名 / 类型文本 / 标注 / span)。
struct StructInfo {
    fields: Vec<FieldInfo>,
}

struct FieldInfo {
    name: String,
    ty_text: String,
    anno: Anno,
    span: Span,
}

#[derive(PartialEq, Eq)]
enum Anno {
    Builtin(String),
    Interpolate(String),
    None,
}

/// 收集全 crate(含嵌套 mod)命名字段结构体 → 信息表(按名;同名以首个为准)。
fn collect_structs(items: &[Item]) -> HashMap<String, StructInfo> {
    let mut out = HashMap::new();
    collect_structs_rec(items, &mut out);
    out
}

fn collect_structs_rec(items: &[Item], out: &mut HashMap<String, StructInfo>) {
    for it in items {
        match &it.kind {
            ItemKind::Struct(s) => {
                if let VariantBody::Named(fields) = &s.body {
                    let info = StructInfo {
                        fields: fields.iter().map(field_info).collect(),
                    };
                    out.entry(s.name.name.clone()).or_insert(info);
                }
            }
            ItemKind::Mod(m) => collect_structs_rec(&m.items, out),
            _ => {}
        }
    }
}

fn field_info(f: &FieldDef) -> FieldInfo {
    FieldInfo {
        name: f.name.name.clone(),
        ty_text: render_ty(&f.ty),
        anno: field_anno(f),
        span: f.span,
    }
}

/// 字段的着色阶段 I/O 标注(首个 `#[builtin(..)]` / `#[interpolate(..)]`)。
fn field_anno(f: &FieldDef) -> Anno {
    for attr in &f.attrs {
        let segs = &attr.meta.path.segments;
        let [seg] = segs.as_slice() else { continue };
        let key = seg.ident.name.as_str();
        if key != "builtin" && key != "interpolate" {
            continue;
        }
        // 取列表首个 meta 名(`#[builtin(position)]` → "position")。
        let arg = match &attr.meta.kind {
            MetaKind::List(inner) => inner.iter().find_map(|mi| match mi {
                MetaInner::Meta(m) => m.path.segments.last().map(|s| s.ident.name.clone()),
                MetaInner::Lit(_) => None,
            }),
            _ => None,
        };
        let arg = arg.unwrap_or_default();
        return if key == "builtin" {
            Anno::Builtin(arg)
        } else {
            Anno::Interpolate(arg)
        };
    }
    Anno::None
}

/// 收集着色阶段 I/O 结构体名 + vertex 输出插值 varying 字段(供接口契约比对)。
fn collect_stage_io(
    items: &[Item],
    structs: &HashMap<String, StructInfo>,
    io_names: &mut HashSet<String>,
    vertex_out_interp: &mut Vec<(String, String, String)>,
) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                let Some(stage) = f.stage else { continue };
                // 形参中的 varying I/O 结构体(排除资源句柄 + RT payload 类聚合)。
                // payload/hit_attribute/callable_data/task_payload 形参为 POD 数据契约,
                // 非插值 varying——不参与 RXS-0154 字段标注校验(RXS-0244 逐字段比对另走)。
                for p in &f.params {
                    if param_payload_kind(&p.attrs).is_some() {
                        continue;
                    }
                    if let crate::ast::ParamKind::Typed { ty, .. } = &p.kind
                        && let Some(n) = io_struct_name(ty, structs)
                    {
                        io_names.insert(n);
                    }
                }
                // 返回类型的 I/O 结构体。
                if let Some(ret) = &f.ret
                    && let Some(n) = io_struct_name(ret, structs)
                {
                    io_names.insert(n.clone());
                    // vertex / mesh 输出 → 接口契约上游(RFC-0002 §4.3)。
                    if matches!(stage, ShaderStage::Vertex | ShaderStage::Mesh)
                        && let Some(info) = structs.get(&n)
                    {
                        for fld in &info.fields {
                            if let Anno::Interpolate(mode) = &fld.anno {
                                vertex_out_interp.push((
                                    fld.name.clone(),
                                    fld.ty_text.clone(),
                                    mode.clone(),
                                ));
                            }
                        }
                    }
                }
            }
            ItemKind::Mod(m) => collect_stage_io(&m.items, structs, io_names, vertex_out_interp),
            _ => {}
        }
    }
}

/// 类型若为已知命名结构体(非资源句柄)→ 其名(着色阶段 varying I/O 结构体候选)。
fn io_struct_name(ty: &Ty, structs: &HashMap<String, StructInfo>) -> Option<String> {
    let head = ty_head_name(ty)?;
    if structs.contains_key(head) {
        Some(head.to_owned())
    } else {
        None
    }
}

/// RXS-0154:着色阶段 I/O 字段标注合法性(无标注 / 未知名 → RX3011)。
fn check_io_field(f: &FieldInfo, diag: &DiagCtxt) {
    let detail = match &f.anno {
        Anno::None => Some(format!(
            "field `{}` carries no `#[interpolate(..)]` or `#[builtin(..)]` annotation \
             (shader stage I/O fields must be annotated)",
            f.name
        )),
        Anno::Builtin(name) if !KNOWN_BUILTINS.contains(&name.as_str()) => Some(format!(
            "field `{}` has unknown builtin variable `{}`",
            f.name, name
        )),
        Anno::Interpolate(mode) if !KNOWN_INTERP.contains(&mode.as_str()) => Some(format!(
            "field `{}` has unknown interpolation qualifier `{}`",
            f.name, mode
        )),
        _ => None,
    };
    if let Some(detail) = detail {
        diag.struct_error(E_STAGE_IO, "shader.stage_io_invalid")
            .arg("detail", detail)
            .span_label(f.span, "invalid shader stage I/O field")
            .emit();
    }
}

/// RXS-0155 / RXS-0156:逐 item 走查(接口契约 + 资源句柄位置)。
fn check_items(
    items: &[Item],
    structs: &HashMap<String, StructInfo>,
    vertex_out_interp: &[(String, String, String)],
    diag: &DiagCtxt,
) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => check_fn(f, structs, vertex_out_interp, diag),
            ItemKind::Struct(s) => {
                // RXS-0156:资源句柄不得作结构体字段(仅签名位置)。
                if let VariantBody::Named(fields) = &s.body {
                    for fld in fields {
                        check_handle_in_field(&fld.ty, diag);
                        // RXS-0245:AccelStruct 亦仅签名形参(结构体字段 → RX3013)。
                        check_accel_in_field(&fld.ty, diag);
                    }
                }
            }
            ItemKind::Mod(m) => check_items(&m.items, structs, vertex_out_interp, diag),
            _ => {}
        }
    }
}

/// RT 阶段判定(RXS-0245;AccelStruct 阶段合法性 + trace_ray 可达域)。
fn is_rt_stage(stage: ShaderStage) -> bool {
    matches!(
        stage,
        ShaderStage::RayGen
            | ShaderStage::ClosestHit
            | ShaderStage::AnyHit
            | ShaderStage::Miss
            | ShaderStage::Intersection
            | ShaderStage::Callable
    )
}

/// `AccelStruct` 不透明句柄(RXS-0245):头名匹配即是,首期无泛型参数化。
fn is_accel_struct(ty: &Ty) -> bool {
    ty_head_name(ty) == Some("AccelStruct")
}

fn check_accel_return(ty: &Ty, diag: &DiagCtxt) {
    if is_accel_struct(ty) {
        emit_handle(
            ty.span,
            "`AccelStruct` cannot appear in return position (acceleration structure handles are \
             input-only RT-stage signature parameters, RXS-0245)"
                .to_owned(),
            diag,
        );
    }
}

fn check_accel_in_field(ty: &Ty, diag: &DiagCtxt) {
    if is_accel_struct(ty) {
        emit_handle(
            ty.span,
            "`AccelStruct` cannot appear as a struct field (acceleration structure handles enter \
             only RT-stage signatures, RXS-0245)"
                .to_owned(),
            diag,
        );
    }
}

/// RXS-0245:`AccelStruct` 仅可作 RT 阶段签名形参(非 RT 阶段 / 非着色阶段签名 → RX3013)。
fn check_accel_param(ty: &Ty, stage: Option<ShaderStage>, diag: &DiagCtxt) {
    if is_accel_struct(ty) && !stage.is_some_and(is_rt_stage) {
        emit_handle(
            ty.span,
            "`AccelStruct` may only appear as a ray-tracing stage signature parameter \
             (raygen / closesthit / anyhit / miss / intersection / callable, RXS-0245)"
                .to_owned(),
            diag,
        );
    }
}

fn check_fn(
    f: &FnItem,
    structs: &HashMap<String, StructInfo>,
    vertex_out_interp: &[(String, String, String)],
    diag: &DiagCtxt,
) {
    // RXS-0156:返回位置不得为资源句柄(句柄是输入形参,非可返回值)。
    if let Some(ret) = &f.ret {
        check_handle_return(ret, diag);
        // RXS-0245:AccelStruct 返回位置违例 → RX3013。
        check_accel_return(ret, diag);
    }
    // 形参:着色阶段允许 `Texture2D<F>`/`Sampler` 作签名形参;未支持纹理维度 → RX3013。
    // 非着色阶段函数不得携带资源句柄形参(首批仅着色阶段签名,RFC-0002 §4.4)。
    for p in &f.params {
        if let crate::ast::ParamKind::Typed { ty, .. } = &p.kind {
            check_handle_param(ty, f.stage.is_some(), diag);
            // RXS-0245:AccelStruct 仅 RT 阶段签名形参。
            check_accel_param(ty, f.stage, diag);
        }
    }
    // RXS-0155:fragment 输入 varying 须与上游 vertex 输出兼容。
    if f.stage == Some(ShaderStage::Fragment) {
        check_fragment_interface(f, structs, vertex_out_interp, diag);
    }
}

/// 纹理/采样器分类:`Some(true)` = 已支持(`Texture2D`/`TextureRw2D`/`Sampler`/
/// `SamplerCmp`,RXS-0156 + RXS-0223;含 G3.4 无界表 `[Texture2D<F>]`,RXS-0231);
/// `Some(false)` = 资源句柄但未支持维度(defer);`None` = 非资源句柄类型。
fn texture_kind(ty: &Ty) -> Option<bool> {
    let head = resource_head_name(ty)?;
    if SUPPORTED_HANDLES.contains(&head) {
        Some(true)
    } else if head.starts_with("Texture") {
        Some(false) // Texture1D/Texture3D/TextureCube/*Array 等:首批不支持
    } else {
        None
    }
}

/// 资源句柄头名:剥一层无界句柄数组 `[Texture2D<F>]`(RXS-0231,切片样式文法)后
/// 取句柄头名,否则同 [`ty_head_name`]。无界表在**位置面**与标量句柄同纪律
/// (仅着色阶段签名形参合法;返回/字段/非阶段函数 → RX3013);无界基数由 mir_build
/// `ResourceCount::Unbounded` 承载、binding 推导 RXS-0233 翻转。
fn resource_head_name(ty: &Ty) -> Option<&str> {
    match &ty.kind {
        TyKind::Slice(inner) => ty_head_name(inner),
        _ => ty_head_name(ty),
    }
}

fn check_handle_return(ty: &Ty, diag: &DiagCtxt) {
    if let Some(supported) = texture_kind(ty) {
        let detail = if supported {
            format!(
                "resource handle `{}` cannot appear in return position \
                 (handles are input-only shader stage parameters)",
                resource_head_name(ty).unwrap_or("")
            )
        } else {
            format!(
                "unsupported texture type `{}` (supported handles: `Texture2D<F>`/`TextureRw2D<F>` + `Sampler`/`SamplerCmp` (RXS-0156/RXS-0223); other dimensions are deferred)",
                resource_head_name(ty).unwrap_or("")
            )
        };
        emit_handle(ty.span, detail, diag);
    }
}

fn check_handle_param(ty: &Ty, in_stage_fn: bool, diag: &DiagCtxt) {
    match texture_kind(ty) {
        Some(true) if in_stage_fn => {} // 着色阶段签名形参:合法
        Some(true) => emit_handle(
            ty.span,
            format!(
                "resource handle `{}` may only appear as a shader stage signature parameter",
                resource_head_name(ty).unwrap_or("")
            ),
            diag,
        ),
        Some(false) => emit_handle(
            ty.span,
            format!(
                "unsupported texture type `{}` (supported handles: `Texture2D<F>`/`TextureRw2D<F>` + `Sampler`/`SamplerCmp` (RXS-0156/RXS-0223); other dimensions are deferred)",
                resource_head_name(ty).unwrap_or("")
            ),
            diag,
        ),
        None => {}
    }
}

fn check_handle_in_field(ty: &Ty, diag: &DiagCtxt) {
    if let Some(_supported) = texture_kind(ty) {
        emit_handle(
            ty.span,
            format!(
                "resource handle `{}` cannot appear as a struct field \
                 (handles enter only shader stage signatures)",
                resource_head_name(ty).unwrap_or("")
            ),
            diag,
        );
    }
}

fn emit_handle(span: Span, detail: String, diag: &DiagCtxt) {
    diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
        .arg("detail", detail)
        .span_label(span, "invalid resource handle position")
        .emit();
}

/// RXS-0155:fragment 输入 varying 与上游 vertex 输出兼容性(编译期类型契约)。
fn check_fragment_interface(
    f: &FnItem,
    structs: &HashMap<String, StructInfo>,
    vertex_out_interp: &[(String, String, String)],
    diag: &DiagCtxt,
) {
    // 上游无 vertex 输出(无可比对契约)→ 跳过(保守:不误报)。
    if vertex_out_interp.is_empty() {
        return;
    }
    for p in &f.params {
        let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
            continue;
        };
        let Some(name) = io_struct_name(ty, structs) else {
            continue;
        };
        let Some(info) = structs.get(&name) else {
            continue;
        };
        for fld in &info.fields {
            let Anno::Interpolate(mode) = &fld.anno else {
                continue;
            };
            // 名 + 类型 + 插值限定逐一匹配上游 vertex 输出 varying。
            let compatible = vertex_out_interp
                .iter()
                .any(|(vn, vt, vm)| *vn == fld.name && *vt == fld.ty_text && *vm == *mode);
            if !compatible {
                diag.struct_error(E_STAGE_INTERFACE, "shader.stage_interface_mismatch")
                    .arg(
                        "detail",
                        format!(
                            "fragment input varying `{}: {}` (#[interpolate({})]) has no compatible \
                             upstream vertex output varying",
                            fld.name, fld.ty_text, mode
                        ),
                    )
                    .span_label(ty.span, "incompatible fragment stage input interface")
                    .emit();
                return; // 单接口单报(防一错多报)
            }
        }
    }
}

// ───────────────────────── RXS-0243 mesh/task 入口标注契约 ─────────────────────────

fn emit_mesh_entry(span: Span, detail: String, diag: &DiagCtxt) {
    diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
        .arg("detail", detail)
        .span_label(span, "invalid mesh/task entry annotation")
        .emit();
}

/// 单段路径的名字(`#[numthreads]` → "numthreads";多段 → None)。
fn single_seg(p: &crate::ast::Path) -> Option<&str> {
    match p.segments.as_slice() {
        [seg] => Some(seg.ident.name.as_str()),
        _ => None,
    }
}

/// 属性列表中按名查找(单段路径匹配)。
fn attr_by_name<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a Attr> {
    attrs
        .iter()
        .find(|a| single_seg(&a.meta.path) == Some(name))
}

/// 正整数字面量值(非 Int / 非正 / 解析失败 → None)。数字后缀(`64u32`)容忍。
fn lit_pos_int(src: &str, lit: &crate::ast::Lit) -> Option<u32> {
    if lit.kind != LitKind::Int {
        return None;
    }
    let text = snippet(src, lit.span);
    let digits: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .filter(|c| *c != '_')
        .collect();
    match digits.parse::<u32>() {
        Ok(v) if v > 0 => Some(v),
        _ => None,
    }
}

/// `#[numthreads(x, y, z)]`:恰三正整数字面量(mesh/task 共用)。合法 → true。
fn check_numthreads(f: &FnItem, attrs: &[Attr], src: &str, kind: &str, diag: &DiagCtxt) -> bool {
    let name = &f.name.name;
    let Some(nt) = attr_by_name(attrs, "numthreads") else {
        emit_mesh_entry(
            f.name.span,
            format!(
                "{kind} entry `{name}` is missing required `#[numthreads(x, y, z)]` (RXS-0243)"
            ),
            diag,
        );
        return false;
    };
    let MetaKind::List(inner) = &nt.meta.kind else {
        emit_mesh_entry(
            nt.span,
            format!("{kind} entry `{name}` `#[numthreads(...)]` must list three integer literals"),
            diag,
        );
        return false;
    };
    let dims: Vec<u32> = inner
        .iter()
        .filter_map(|e| match e {
            MetaInner::Lit(l) => lit_pos_int(src, l),
            _ => None,
        })
        .collect();
    if inner.len() != 3 || dims.len() != 3 {
        emit_mesh_entry(
            nt.span,
            format!(
                "{kind} entry `{name}` `#[numthreads(...)]` must have exactly three positive \
                 integer literals (RXS-0243)"
            ),
            diag,
        );
        return false;
    }
    true
}

/// RXS-0243:mesh/task 入口标注契约走查(mesh 须 numthreads + outputs;task 须 numthreads)。
fn check_stage_entries(items: &[Item], src: &str, diag: &DiagCtxt) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                let Some(stage) = f.stage else { continue };
                match stage {
                    ShaderStage::Mesh => check_mesh_entry(f, &it.attrs, src, diag),
                    ShaderStage::Task => {
                        check_numthreads(f, &it.attrs, src, "task", diag);
                    }
                    _ => {}
                }
            }
            ItemKind::Mod(m) => check_stage_entries(&m.items, src, diag),
            _ => {}
        }
    }
}

/// mesh 入口:`#[numthreads]` + `#[outputs(topology = "triangles", max_vertices = N,
/// max_primitives = M)]`(triangles-only,N/M 正整数字面量);缺任一 / 未知拓扑 / 非正
/// 字面量 → RX3017(RXS-0243,Q-M-MeshTopology)。
fn check_mesh_entry(f: &FnItem, attrs: &[Attr], src: &str, diag: &DiagCtxt) {
    let name = &f.name.name;
    check_numthreads(f, attrs, src, "mesh", diag);
    let Some(outputs) = attr_by_name(attrs, "outputs") else {
        emit_mesh_entry(
            f.name.span,
            format!(
                "mesh entry `{name}` is missing required `#[outputs(topology = \"triangles\", \
                 max_vertices = N, max_primitives = M)]` (RXS-0243)"
            ),
            diag,
        );
        return;
    };
    let MetaKind::List(inner) = &outputs.meta.kind else {
        emit_mesh_entry(
            outputs.span,
            format!("mesh entry `{name}` `#[outputs(...)]` must list `key = value` pairs"),
            diag,
        );
        return;
    };
    let mut topology: Option<String> = None;
    let mut max_vertices: Option<u32> = None;
    let mut max_primitives: Option<u32> = None;
    for entry in inner {
        let MetaInner::Meta(mi) = entry else { continue };
        let (Some(key), MetaKind::NameValue(lit)) = (single_seg(&mi.path), &mi.kind) else {
            continue;
        };
        match key {
            "topology" if lit.kind == LitKind::Str => {
                topology = Some(snippet(src, lit.span).trim_matches('"').to_owned());
            }
            "max_vertices" => max_vertices = lit_pos_int(src, lit),
            "max_primitives" => max_primitives = lit_pos_int(src, lit),
            _ => {}
        }
    }
    match topology.as_deref() {
        Some("triangles") => {}
        Some(other) => {
            emit_mesh_entry(
                outputs.span,
                format!(
                    "mesh entry `{name}` `#[outputs]` topology `{other}` is not supported \
                     (first period is triangles-only, RXS-0243/Q-M-MeshTopology)"
                ),
                diag,
            );
            return;
        }
        None => {
            emit_mesh_entry(
                outputs.span,
                format!("mesh entry `{name}` `#[outputs]` is missing `topology = \"triangles\"`"),
                diag,
            );
            return;
        }
    }
    if max_vertices.is_none() {
        emit_mesh_entry(
            outputs.span,
            format!("mesh entry `{name}` `#[outputs]` needs a positive `max_vertices = N`"),
            diag,
        );
        return;
    }
    if max_primitives.is_none() {
        emit_mesh_entry(
            outputs.span,
            format!("mesh entry `{name}` `#[outputs]` needs a positive `max_primitives = M`"),
            diag,
        );
    }
}

// ───────────────────────── RXS-0244 RT payload / attribute / callable data ─────────

/// payload 类标注(RXS-0244;着色阶段间数据契约的超集类别,RX3012 扩,SC-3)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum PayloadKind {
    Payload,
    HitAttribute,
    CallableData,
    TaskPayload,
}

impl PayloadKind {
    fn attr_name(self) -> &'static str {
        match self {
            PayloadKind::Payload => "payload",
            PayloadKind::HitAttribute => "hit_attribute",
            PayloadKind::CallableData => "callable_data",
            PayloadKind::TaskPayload => "task_payload",
        }
    }
}

/// 形参标注中的首个 payload 类标注(RXS-0244)。
fn param_payload_kind(attrs: &[Attr]) -> Option<PayloadKind> {
    for a in attrs {
        match single_seg(&a.meta.path) {
            Some("payload") => return Some(PayloadKind::Payload),
            Some("hit_attribute") => return Some(PayloadKind::HitAttribute),
            Some("callable_data") => return Some(PayloadKind::CallableData),
            Some("task_payload") => return Some(PayloadKind::TaskPayload),
            _ => {}
        }
    }
    None
}

/// 一个 payload 类形参声明(承载结构体名 + span)。
struct PayloadDecl {
    kind: PayloadKind,
    struct_name: String,
    span: Span,
}

fn collect_payload_decls(items: &[Item], out: &mut Vec<PayloadDecl>) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                for p in &f.params {
                    let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
                        continue;
                    };
                    let Some(kind) = param_payload_kind(&p.attrs) else {
                        continue;
                    };
                    if let Some(head) = ty_head_name(ty) {
                        out.push(PayloadDecl {
                            kind,
                            struct_name: head.to_owned(),
                            span: ty.span,
                        });
                    }
                }
            }
            ItemKind::Mod(m) => collect_payload_decls(&m.items, out),
            _ => {}
        }
    }
}

/// 结构体字段序(名 + 类型文本;比对基础)。未知结构体 → None(不可判定,不误报)。
fn field_signature(
    structs: &HashMap<String, StructInfo>,
    name: &str,
) -> Option<Vec<(String, String)>> {
    structs.get(name).map(|info| {
        info.fields
            .iter()
            .map(|f| (f.name.clone(), f.ty_text.clone()))
            .collect()
    })
}

/// RXS-0244:同编译单元内每类 payload 契约逐字段比对(单三件套配对域)。
/// closesthit/anyhit/miss 的 `#[payload]`、intersection/hit 的 `#[hit_attribute]`、
/// callable 的 `#[callable_data]`、mesh 的 `#[task_payload]` 各自成组,组内字段序须一致;
/// 错配 → RX3012 扩类别(raygen↔hit/miss 经 trace_ray 调用点比对为 body 层,归后续
/// mir_build 接线,首期类型面比对声明域一致性)。
fn check_payload_contracts(items: &[Item], structs: &HashMap<String, StructInfo>, diag: &DiagCtxt) {
    let mut decls = Vec::new();
    collect_payload_decls(items, &mut decls);
    for kind in [
        PayloadKind::Payload,
        PayloadKind::HitAttribute,
        PayloadKind::CallableData,
        PayloadKind::TaskPayload,
    ] {
        let group: Vec<&PayloadDecl> = decls.iter().filter(|d| d.kind == kind).collect();
        // 参照 = 组内首个可判定(已知结构体)声明的字段序。
        let Some(reference) = group
            .iter()
            .find_map(|d| field_signature(structs, &d.struct_name))
        else {
            continue;
        };
        for d in &group {
            let Some(fields) = field_signature(structs, &d.struct_name) else {
                continue;
            };
            if fields != reference {
                diag.struct_error(E_STAGE_INTERFACE, "shader.stage_interface_mismatch")
                    .arg(
                        "detail",
                        format!(
                            "`#[{}]` type `{}` has a field layout incompatible with the pipeline's \
                             other `#[{}]` declarations (single-triple pairing domain, RXS-0244)",
                            kind.attr_name(),
                            d.struct_name,
                            kind.attr_name()
                        ),
                    )
                    .span_label(d.span, "incompatible ray-tracing data contract")
                    .emit();
                break; // 单契约单报(防一错多报)
            }
        }
    }
}

/// 类型头名(`Texture2D<f32>` → "Texture2D";`&T` 取内层头;非路径类型 → None)。
fn ty_head_name(ty: &Ty) -> Option<&str> {
    match &ty.kind {
        TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            ty_head_name(inner)
        }
        _ => None,
    }
}

/// 类型文本渲染(接口字段比对用;稳定、与诊断无关)。
fn render_ty(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Path(p) => render_path(p),
        TyKind::Ref { mutable, inner, .. } => {
            format!(
                "&{}{}",
                if *mutable { "mut " } else { "" },
                render_ty(inner)
            )
        }
        TyKind::RawPtr { mutable, inner } => {
            format!(
                "*{} {}",
                if *mutable { "mut" } else { "const" },
                render_ty(inner)
            )
        }
        TyKind::Tuple(v) => {
            format!(
                "({})",
                v.iter().map(render_ty).collect::<Vec<_>>().join(", ")
            )
        }
        TyKind::Paren(t) => render_ty(t),
        TyKind::Array { elem, .. } => format!("[{}; _]", render_ty(elem)),
        TyKind::Slice(t) => format!("[{}]", render_ty(t)),
        TyKind::FnPtr { params, ret } => {
            let ps = params.iter().map(render_ty).collect::<Vec<_>>().join(", ");
            match ret {
                Some(r) => format!("fn({ps}) -> {}", render_ty(r)),
                None => format!("fn({ps})"),
            }
        }
        TyKind::Infer => "_".to_owned(),
        TyKind::ConstArg(_) => "<const>".to_owned(),
        TyKind::Err => "{err}".to_owned(),
    }
}

fn render_path(p: &crate::ast::Path) -> String {
    let mut out = String::new();
    for (i, seg) in p.segments.iter().enumerate() {
        if i > 0 {
            out.push_str("::");
        }
        out.push_str(&seg.ident.name);
        if let Some(args) = &seg.args {
            let inner: Vec<String> = args
                .args
                .iter()
                .map(|a| match a {
                    crate::ast::GenericArg::Type(t) => render_ty(t),
                    crate::ast::GenericArg::Lifetime(_) => "'_".to_owned(),
                    crate::ast::GenericArg::Const(_) => "<const>".to_owned(),
                })
                .collect();
            if !inner.is_empty() {
                out.push('<');
                out.push_str(&inner.join(", "));
                out.push('>');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::span::{Edition, SourceId};

    /// parse + 着色阶段检查,返回错误码序列。
    fn check_codes(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let file = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        assert!(
            diag.emitted().is_empty(),
            "前置 parse 诊断: {:?}",
            diag.emitted().iter().map(|d| d.code).collect::<Vec<_>>()
        );
        super::check(&file, src, &diag);
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    //@ spec: RXS-0153
    #[test]
    fn legal_stage_declarations_are_clean() {
        // 着色阶段声明本身合法(直接调用拦截属 coloring/RX3001,见 coloring.rs)。
        let codes = check_codes(
            "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n\
             fragment fn fs(inp: VsOut) -> VsOut { inp }\n\
             compute fn cs() {}\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0154
    #[test]
    fn unannotated_io_field_is_rx3011() {
        let codes = check_codes(
            "struct VsOut { pos: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { pos: 0.0 } }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3011]);
    }

    //@ spec: RXS-0154
    #[test]
    fn unknown_builtin_is_rx3011() {
        let codes = check_codes(
            "struct VsOut { #[builtin(teleport)] pos: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { pos: 0.0 } }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3011]);
    }

    //@ spec: RXS-0154
    #[test]
    fn unknown_interpolation_is_rx3011() {
        let codes = check_codes(
            "struct VsOut { #[interpolate(wobbly)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { uv: 0.0 } }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3011]);
    }

    //@ spec: RXS-0155
    #[test]
    fn incompatible_fragment_interface_is_rx3012() {
        let codes = check_codes(
            "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] uv: f32 }\n\
             struct FsIn { #[interpolate(perspective)] color: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n\
             fragment fn fs(inp: FsIn) -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3012]);
    }

    //@ spec: RXS-0155
    #[test]
    fn shared_interface_struct_is_clean() {
        let codes = check_codes(
            "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n\
             fragment fn fs(inp: VsOut) -> VsOut { inp }\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0156
    #[test]
    fn texture_sampler_params_are_clean() {
        let codes = check_codes(
            "struct VsOut { #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { uv: 0.0 } }\n\
             fragment fn fs(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut { inp }\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0156
    #[test]
    fn handle_in_return_position_is_rx3013() {
        let codes = check_codes(
            "fragment fn fs() -> Texture2D<f32> { }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    //@ spec: RXS-0223
    #[test]
    fn sampling_superset_handles_are_clean() {
        // G3.3 增补句柄(RXS-0223):TextureRw2D<F> + SamplerCmp 作着色阶段签名
        // 形参 = 合法(位置纪律同 RXS-0156)。
        let codes = check_codes(
            "struct VsOut { #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { uv: 0.0 } }\n\
             fragment fn fs(inp: VsOut, rw: TextureRw2D<f32>, sc: SamplerCmp) -> VsOut { inp }\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0223
    #[test]
    fn rw_handle_in_return_position_is_rx3013() {
        // 位置纪律承 RXS-0156:TextureRw2D 返回位置违例 → RX3013。
        let codes = check_codes(
            "fragment fn fs() -> TextureRw2D<f32> { }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    //@ spec: RXS-0223
    #[test]
    fn sampler_cmp_in_struct_field_is_rx3013() {
        // 位置纪律承 RXS-0156:SamplerCmp 结构体字段违例 → RX3013。
        let codes = check_codes(
            "struct Bag { sc: SamplerCmp }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    //@ spec: RXS-0156
    #[test]
    fn unsupported_texture_dimension_is_rx3013() {
        let codes = check_codes(
            "struct VsOut { #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs() -> VsOut { VsOut { uv: 0.0 } }\n\
             fragment fn fs(inp: VsOut, tex: Texture3D<f32>) -> VsOut { inp }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    // ── RXS-0242:intersection / callable 阶段全集补齐 ──

    //@ spec: RXS-0242
    #[test]
    fn intersection_callable_stages_declare_clean() {
        // 前缀式 `intersection fn` / `callable fn` 入 stage 集(RXS-0153 修订行)。
        let codes = check_codes(
            "intersection fn isect() {}\n\
             callable fn call() {}\n\
             raygen fn rg() {}\n\
             miss fn ms() {}\n\
             closesthit fn ch() {}\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    // ── RXS-0243:mesh/task 入口标注契约 → RX3017 ──

    //@ spec: RXS-0243
    #[test]
    fn mesh_task_entry_full_annotations_are_clean() {
        let codes = check_codes(
            "#[numthreads(32, 1, 1)]\n\
             #[outputs(topology = \"triangles\", max_vertices = 64, max_primitives = 42)]\n\
             mesh fn ms() {}\n\
             #[numthreads(32, 1, 1)]\n\
             task fn tk() {}\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0243
    #[test]
    fn mesh_missing_numthreads_is_rx3017() {
        let codes = check_codes(
            "#[outputs(topology = \"triangles\", max_vertices = 64, max_primitives = 42)]\n\
             mesh fn ms() {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3017]);
    }

    //@ spec: RXS-0243
    #[test]
    fn mesh_missing_outputs_is_rx3017() {
        let codes = check_codes(
            "#[numthreads(32, 1, 1)]\n\
             mesh fn ms() {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3017]);
    }

    //@ spec: RXS-0243
    #[test]
    fn mesh_bad_topology_is_rx3017() {
        // triangles-only 首期收敛(Q-M-MeshTopology);lines → 编译期拒。
        let codes = check_codes(
            "#[numthreads(32, 1, 1)]\n\
             #[outputs(topology = \"lines\", max_vertices = 64, max_primitives = 42)]\n\
             mesh fn ms() {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3017]);
    }

    //@ spec: RXS-0243
    #[test]
    fn task_missing_numthreads_is_rx3017() {
        let codes = check_codes(
            "task fn tk() {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3017]);
    }

    // ── RXS-0244:RT payload / hit attribute / callable data 契约逐字段比对 → RX3012 ──

    //@ spec: RXS-0244
    #[test]
    fn rt_payload_pair_is_clean() {
        // closesthit + miss 同一 payload 类型 → 0 诊断(单三件套配对域)。
        let codes = check_codes(
            "struct RayPayload { color: f32, dist: f32 }\n\
             raygen fn rg(tlas: AccelStruct) {}\n\
             closesthit fn ch(#[payload] p: &mut RayPayload) {}\n\
             miss fn ms(#[payload] p: &mut RayPayload) {}\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0244
    #[test]
    fn rt_payload_mismatch_is_rx3012() {
        // closesthit 与 miss 的 payload 字段序不一致 → RX3012 扩类别。
        let codes = check_codes(
            "struct HitPayload { color: f32, dist: f32 }\n\
             struct MissPayload { color: f32 }\n\
             closesthit fn ch(#[payload] p: &mut HitPayload) {}\n\
             miss fn ms(#[payload] p: &mut MissPayload) {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3012]);
    }

    // ── RXS-0245:AccelStruct 句柄位置纪律 → RX3013 ──

    //@ spec: RXS-0245
    #[test]
    fn accelstruct_in_raygen_param_is_clean() {
        let codes = check_codes(
            "raygen fn rg(tlas: AccelStruct) {}\n\
             fn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0245
    #[test]
    fn accelstruct_return_is_rx3013() {
        let codes = check_codes(
            "raygen fn rg() -> AccelStruct { }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    //@ spec: RXS-0245
    #[test]
    fn accelstruct_in_non_rt_stage_is_rx3013() {
        // AccelStruct 仅 RT 阶段签名形参;fragment 签名 → RX3013。
        let codes = check_codes(
            "fragment fn fs(tlas: AccelStruct) {}\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }

    //@ spec: RXS-0245
    #[test]
    fn accelstruct_in_struct_field_is_rx3013() {
        let codes = check_codes(
            "struct Bag { tlas: AccelStruct }\n\
             fn main() {}",
        );
        assert_eq!(codes, vec![3013]);
    }
}
