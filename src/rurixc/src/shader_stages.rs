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

use std::collections::{HashMap, HashSet};

use crate::ast::{
    FieldDef, FnItem, Item, ItemKind, MetaInner, MetaKind, ShaderStage, SourceFile, Ty, TyKind,
    VariantBody,
};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::span::Span;

pub const E_STAGE_IO: ErrorCode = ErrorCode(3011); // RX3011(RXS-0154)
pub const E_STAGE_INTERFACE: ErrorCode = ErrorCode(3012); // RX3012(RXS-0155)
pub const E_RESOURCE_HANDLE: ErrorCode = ErrorCode(3013); // RX3013(RXS-0156)

/// 已知内建变量名(RFC-0002 §4.2;type-level 内建变量类型化)。
const KNOWN_BUILTINS: &[&str] = &[
    "position",
    "vertex_id",
    "instance_id",
    "frag_coord",
    "thread_id",
    "front_facing",
    "depth",
    "primitive_id",
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

/// 首批支持的纹理类型名(RFC-0002 §9 Q4:仅 `Texture2D`,其余维度 defer)。
const SUPPORTED_TEXTURE: &str = "Texture2D";
/// GRX-009:compute-kernel UAV 纹理类型名(`RWTexture2D<F>`)。
const SUPPORTED_RWTEXTURE: &str = "RWTexture2D";
/// 采样器类型名(RFC-0002 §4.4)。
const SAMPLER: &str = "Sampler";

/// 着色阶段类型面检查入口(driver / ui_golden / conformance 复用)。
pub fn check(file: &SourceFile, diag: &DiagCtxt) {
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
                // 形参中的 varying I/O 结构体(排除资源句柄)。
                for p in &f.params {
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
                    }
                }
            }
            ItemKind::Mod(m) => check_items(&m.items, structs, vertex_out_interp, diag),
            _ => {}
        }
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
    }
    // 形参:着色阶段允许 `Texture2D<F>`/`Sampler` 作签名形参;未支持纹理维度 → RX3013。
    // GRX-009:`kernel fn` 同样允许 `Texture2D<f32>`/`RWTexture2D<f32>` 作计算内核签名形参
    // (compute-kernel SRV/UAV 纹理句柄);非着色/非 kernel 函数不得携带资源句柄形参
    // (RFC-0002 §4.4)。
    let allow_handle_param = f.stage.is_some() || f.color == crate::ast::FnColor::Kernel;
    for p in &f.params {
        if let crate::ast::ParamKind::Typed { ty, .. } = &p.kind {
            check_handle_param(ty, allow_handle_param, diag);
        }
    }
    // RXS-0155:fragment 输入 varying 须与上游 vertex 输出兼容。
    if f.stage == Some(ShaderStage::Fragment) {
        check_fragment_interface(f, structs, vertex_out_interp, diag);
    }
}

/// 纹理/采样器分类:`Some(true)` = 首批支持(`Texture2D`/`Sampler`);
/// `Some(false)` = 资源句柄但未支持维度(defer);`None` = 非资源句柄类型。
fn texture_kind(ty: &Ty) -> Option<bool> {
    let head = ty_head_name(ty)?;
    if head == SUPPORTED_TEXTURE || head == SUPPORTED_RWTEXTURE || head == SAMPLER {
        Some(true) // `Texture2D`/`RWTexture2D`/`Sampler`:首批支持
    } else if head.starts_with("Texture") {
        Some(false) // Texture1D/Texture3D/TextureCube/*Array 等:首批不支持
    } else {
        None
    }
}

fn check_handle_return(ty: &Ty, diag: &DiagCtxt) {
    if let Some(supported) = texture_kind(ty) {
        let detail = if supported {
            format!(
                "resource handle `{}` cannot appear in return position \
                 (handles are input-only shader stage parameters)",
                ty_head_name(ty).unwrap_or("")
            )
        } else {
            format!(
                "unsupported texture type `{}` (first batch supports only `Texture2D<F>` + `Sampler`; other dimensions are deferred)",
                ty_head_name(ty).unwrap_or("")
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
                ty_head_name(ty).unwrap_or("")
            ),
            diag,
        ),
        Some(false) => emit_handle(
            ty.span,
            format!(
                "unsupported texture type `{}` (first batch supports only `Texture2D<F>` + `Sampler`; other dimensions are deferred)",
                ty_head_name(ty).unwrap_or("")
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
                ty_head_name(ty).unwrap_or("")
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
        super::check(&file, &diag);
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
}
