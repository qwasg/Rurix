//! `iface_extract` — 着色入口签名面的 AST 无损提取层(G8.2 M31,RXS-0304)。
//!
//! 本模块内容自 `mir_build::dxil_io` **机械搬迁**(`pub(super)` → `pub(crate)`,
//! 函数体逐字节零漂移):原提取层 gate 于 `dxil-backend`/`vulkan-backend`(device MIR
//! 附着用),M31 的 `--emit=reflection`(reflection v1)需要在**默认构建**(仅默认
//! feature `shader-stages`)下消费同一提取律,故提取层上提为 crate 内共享模块。
//! 消费侧两处:`mir_build`(图形阶段根 `stage`/`io_sig`/`resources`/`mesh_meta` 附着,
//! 调用点经 `use crate::iface_extract as dxil_io;` 别名原位维持)与
//! [`crate::reflection`](reflection v1 接口事实源)。
//!
//! HIR `FieldDef` 不携 `#[builtin(..)]`/`#[interpolate(..)]` 属性(那是 AST 面),
//! 故 I/O 签名意图须自 AST 提取。本模块**只读** AST(`cx.ast()`),按图形阶段
//! 函数的形参(`In`)/返回(`Out`)位置可达的 I/O 结构体字段,逐字段携带源码
//! 字段名 / builtin·interpolate·varying 种类 / 已建模类型 / 方向四维度。
//!
//! 类型映射(R1.9 边界):标量 prim → [`MirIoType::Scalar`]、向量约定名 →
//! [`MirIoType::Vector`];超出已建模子集的类型**不在此静默丢弃**——元素仍
//! 进 io_sig(字段名/种类/方向保真),不可映射的 6xxx 拒绝由编码器裁决。
//! 资源句柄(`Texture2D`/`Sampler`)非命名 I/O 结构体,自然不入 io_sig
//! (opaque handle 形态,RFC-0004 §4.6(b))。
//!
//! 本模块仅在 cargo feature `shader-stages`(默认启用)下编入(`AccelStruct`
//! 判定复用 [`crate::shader_stages::is_accel_struct`] 单一事实源,RXS-0245/0297)。
use std::collections::HashMap;

use crate::ast::{self, LitKind, MetaInner, MetaKind, ShaderStage, TyKind};
use crate::hir::PrimTy;
use crate::mir::{
    IoDir, IoSigElem, IoSigKind, MeshEntryMeta, MirIoType, MirResourceType, ResourceBinding,
    ResourceCount,
};

/// 提取指定图形阶段函数(名 + 阶段匹配)的 I/O 意图签名表。
pub(crate) fn io_sig_for(
    file: &ast::SourceFile,
    fn_name: &str,
    stage: ShaderStage,
) -> Vec<IoSigElem> {
    let mut structs: HashMap<String, &[ast::FieldDef]> = HashMap::new();
    collect_named_structs(&file.items, &mut structs);

    let mut out = Vec::new();
    let Some(f) = find_stage_fn(&file.items, fn_name, stage) else {
        return out;
    };
    // 形参 → In 方向(资源句柄等非命名 I/O 结构体自然跳过)。
    for p in &f.params {
        if let ast::ParamKind::Typed { ty, .. } = &p.kind
            && let Some(fields) = io_struct_fields(ty, &structs)
        {
            for fld in fields {
                out.push(field_to_elem(fld, IoDir::In));
            }
        }
    }
    // 返回类型 → Out 方向。
    if let Some(ret) = &f.ret
        && let Some(fields) = io_struct_fields(ret, &structs)
    {
        for fld in fields {
            out.push(field_to_elem(fld, IoDir::Out));
        }
    }
    out
}

/// 提取指定图形阶段函数的资源句柄形参绑定声明(RXS-0163;PR-E2b 生产接线)。
///
/// 按**声明序**扫描阶段函数形参,命中资源句柄类型(RXS-0156 首批:`Texture2D<F>`
/// → SRV / `Sampler` → Sampler)者落 [`ResourceBinding`](源码形参名保名 + 资源
/// 类型 + 基数)。命名 I/O 结构体形参(varying)与原生类型形参不入(由
/// [`io_sig_for`] 各管其责)。首批无数组语法 → 基数恒 [`ResourceCount::One`]。
pub(crate) fn resources_for(
    file: &ast::SourceFile,
    fn_name: &str,
    stage: ShaderStage,
) -> Vec<ResourceBinding> {
    let mut out = Vec::new();
    let Some(f) = find_stage_fn(&file.items, fn_name, stage) else {
        return out;
    };
    for p in &f.params {
        if let ast::ParamKind::Typed { pat, ty } = &p.kind
            && let Some((res, count)) = ast_ty_to_resource(ty)
        {
            out.push(ResourceBinding {
                name: pat_binding_name(pat).unwrap_or_default(),
                res,
                count,
            });
        }
    }
    out
}

/// compute 签名 `AccelStruct` 形参 → local 下标表(G7.2 W3a,RXS-0297/0300)。
///
/// `stage` = `None`(`kernel fn`)时按 kernel 着色查找,`Some(Compute)` 时按
/// `compute fn` 查找。判定复用 [`crate::shader_stages::is_accel_struct`]
/// (AST 单一事实源,与位置纪律 RX3013 同函数)。返回值为 `locals` 域下标
/// (声明序 + 1;`locals[0]` = 返回槽)。
///
/// 「至多一个」纪律已由 `shader_stages` 预校验(第 2 个起 RX3013);本函数
/// 只做无损提取,不重复裁决(保守携带全部命中项,codegen 侧另有防御性拒)。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
pub(crate) fn accel_params_for(
    file: &ast::SourceFile,
    fn_name: &str,
    stage: Option<ShaderStage>,
) -> Vec<u32> {
    let mut out = Vec::new();
    let Some(f) = find_compute_fn(&file.items, fn_name, stage) else {
        return out;
    };
    for (i, p) in f.params.iter().enumerate() {
        if let ast::ParamKind::Typed { ty, .. } = &p.kind
            && crate::shader_stages::is_accel_struct(ty)
        {
            out.push(u32::try_from(i + 1).unwrap_or(u32::MAX));
        }
    }
    out
}

/// compute 根查找:`stage == None` → `kernel fn`(Kernel 着色 + 无 stage);
/// `stage == Some(Compute)` → `compute fn`。嵌套 `mod` 递归(同
/// [`find_stage_fn`] 体例)。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
fn find_compute_fn<'a>(
    items: &'a [ast::Item],
    name: &str,
    stage: Option<ShaderStage>,
) -> Option<&'a ast::FnItem> {
    find_compute_item(items, name, stage).and_then(|it| match &it.kind {
        ast::ItemKind::Fn(f) => Some(f),
        _ => None,
    })
}

/// [`find_compute_fn`] 的 `ast::Item` 携带变体(属性面在 Item 上;G14.3
/// compute `#[numthreads]` 提取消费)。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
fn find_compute_item<'a>(
    items: &'a [ast::Item],
    name: &str,
    stage: Option<ShaderStage>,
) -> Option<&'a ast::Item> {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) if f.name.name == name && f.stage == stage => {
                // `stage == None` 时另核着色为 Kernel(排除同名 host fn)。
                if stage.is_some() || matches!(f.color, crate::ast::FnColor::Kernel) {
                    return Some(it);
                }
            }
            ast::ItemKind::Mod(m) => {
                if let Some(found) = find_compute_item(&m.items, name, stage) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// compute 根(`kernel fn`/`compute fn`)`#[numthreads(x, y, z)]` 标注提取
/// (G14.3 生产管线性能波;`wg` 标注系首片——compute 面 workgroup 尺寸契约)。
/// 查找/字面量机械与 mesh/task 面同一([`find_compute_item`] + [`parse_numthreads`]);
/// 无标注/形态非法 → `None`(codegen 落既有 `(1, 1, 1)` 默认,既有 kernel SPV
/// 字节零漂移)。本函数只做无损提取不发诊断(同 [`accel_params_for`] 纪律)。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
pub(crate) fn compute_numthreads_for(
    file: &ast::SourceFile,
    src: &str,
    fn_name: &str,
    stage: Option<ShaderStage>,
) -> Option<(u32, u32, u32)> {
    let item = find_compute_item(&file.items, fn_name, stage)?;
    parse_numthreads(&item.attrs, src)
}

/// 简单绑定形参名(`name: Ty` → "name");非简单绑定模式 → None。
pub(crate) fn pat_binding_name(pat: &ast::Pat) -> Option<String> {
    match &pat.kind {
        ast::PatKind::Binding { name, .. } => Some(name.name.clone()),
        _ => None,
    }
}

/// AST 类型 → 资源句柄建模 + 绑定基数(RXS-0156 `Texture2D<F>`/`Sampler`;
/// RXS-0223 扩 `TextureRw2D<F>`/`SamplerCmp`;G3.4 RXS-0231 扩无界句柄数组
/// `[Texture2D<F>]` → [`ResourceCount::Unbounded`]);非资源句柄类型 → None。
fn ast_ty_to_resource(ty: &ast::Ty) -> Option<(MirResourceType, ResourceCount)> {
    let ty = unwrap_ty(ty);
    // G3.4 无界句柄数组 `[Texture2D<F>]`(RXS-0231;切片样式文法,无新 token)→
    // 无界基数(binding 推导 RXS-0233 自 Unmappable 翻转;首期无界仅 SRV 纹理,
    // 非-SRV-纹理无界维持 RX6013,binding_layout 兜底)。
    if let TyKind::Slice(inner) = &ty.kind {
        let res = scalar_resource(unwrap_ty(inner))?;
        return Some((res, ResourceCount::Unbounded));
    }
    Some((scalar_resource(ty)?, ResourceCount::One))
}

/// 标量(单)资源句柄类型建模。`Texture2D`/`TextureRw2D` 取首个类型实参的头
/// prim 作分量类型(缺省 `f32`);非资源句柄类型 → None。
fn scalar_resource(ty: &ast::Ty) -> Option<MirResourceType> {
    let head = ty_head_name(ty)?;
    let elem_prim = || {
        if let TyKind::Path(p) = &ty.kind {
            p.segments
                .last()
                .and_then(vector_elem_prim)
                .unwrap_or(PrimTy::F32)
        } else {
            PrimTy::F32
        }
    };
    match head {
        "Texture2D" => Some(MirResourceType::Texture2D(elem_prim())),
        "Sampler" => Some(MirResourceType::Sampler),
        // RXS-0223:storage image(UAV 轴)+ 比较采样器(Sampler 轴)。
        "TextureRw2D" => Some(MirResourceType::TextureRw2D(elem_prim())),
        "SamplerCmp" => Some(MirResourceType::SamplerCmp),
        _ => None,
    }
}

/// 收集全 crate(含嵌套 mod)命名字段结构体 → 字段切片(按名;同名取首个)。
fn collect_named_structs<'a>(
    items: &'a [ast::Item],
    out: &mut HashMap<String, &'a [ast::FieldDef]>,
) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Struct(s) => {
                if let ast::VariantBody::Named(fields) = &s.body {
                    out.entry(s.name.name.clone()).or_insert(fields.as_slice());
                }
            }
            ast::ItemKind::Mod(m) => collect_named_structs(&m.items, out),
            _ => {}
        }
    }
}

/// 按名 + 阶段查找图形阶段函数(含嵌套 mod)。
fn find_stage_fn<'a>(
    items: &'a [ast::Item],
    name: &str,
    stage: ShaderStage,
) -> Option<&'a ast::FnItem> {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) if f.stage == Some(stage) && f.name.name == name => {
                return Some(f);
            }
            ast::ItemKind::Mod(m) => {
                if let Some(found) = find_stage_fn(&m.items, name, stage) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// 类型若命中命名结构体(I/O varying 结构体)→ 其字段切片;否则 None
/// (资源句柄 / 原生类型等非 I/O 结构体)。
fn io_struct_fields<'a>(
    ty: &ast::Ty,
    structs: &HashMap<String, &'a [ast::FieldDef]>,
) -> Option<&'a [ast::FieldDef]> {
    let head = ty_head_name(ty)?;
    structs.get(head).copied()
}

/// 单个 AST I/O 字段 → MIR 意图签名元素(四维度保真)。
fn field_to_elem(f: &ast::FieldDef, dir: IoDir) -> IoSigElem {
    IoSigElem {
        field_name: f.name.name.clone(),
        kind: field_anno_kind(f),
        ty: ast_ty_to_mir_io(&f.ty),
        dir,
    }
}

/// 字段标注 → I/O 种类(首个 `#[builtin(..)]`/`#[interpolate(..)]`;无标注
/// 落 [`IoSigKind::Varying`])。与 [`crate::shader_stages`] 的 `field_anno`
/// 同口径(builtin/interpolate 取列表首个 meta 名)。
fn field_anno_kind(f: &ast::FieldDef) -> IoSigKind {
    for attr in &f.attrs {
        let [seg] = attr.meta.path.segments.as_slice() else {
            continue;
        };
        let key = seg.ident.name.as_str();
        if key != "builtin" && key != "interpolate" {
            continue;
        }
        let arg = match &attr.meta.kind {
            MetaKind::List(inner) => inner.iter().find_map(|mi| match mi {
                MetaInner::Meta(m) => m.path.segments.last().map(|s| s.ident.name.clone()),
                MetaInner::Lit(_) => None,
            }),
            _ => None,
        }
        .unwrap_or_default();
        return if key == "builtin" {
            IoSigKind::Builtin(arg)
        } else {
            IoSigKind::Interpolate(arg)
        };
    }
    IoSigKind::Varying
}

/// AST 类型 → 已建模 MIR I/O 类型(标量 / 向量)。超出子集的类型不在此
/// 静默丢弃(元素仍携),保守落 [`MirIoType::Scalar`] 占位 —— 真正的不可
/// 映射 6xxx 拒绝由 B 路编码器(任务 2/4)裁决(strict-only,R1.9)。
fn ast_ty_to_mir_io(ty: &ast::Ty) -> MirIoType {
    let ty = unwrap_ty(ty);
    if let TyKind::Path(p) = &ty.kind
        && let Some(seg) = p.segments.last()
    {
        let name = seg.ident.name.as_str();
        if let Some(prim) = PrimTy::from_name(name) {
            return MirIoType::Scalar(prim);
        }
        if let Some(n) = vector_arity(name) {
            let elem = vector_elem_prim(seg).unwrap_or(PrimTy::F32);
            return MirIoType::Vector(elem, n);
        }
    }
    // 不可映射类型占位:意图侧字段名/种类/方向已保真,类型由编码器复核。
    MirIoType::Scalar(PrimTy::F32)
}

/// 向量约定名 → 分量数(`vec2/vec3/vec4`,2..=4;非向量名返回 None)。
fn vector_arity(name: &str) -> Option<u8> {
    match name {
        "vec2" => Some(2),
        "vec3" => Some(3),
        "vec4" => Some(4),
        _ => None,
    }
}

/// 向量分量 prim(末段 `<elem>` 首个类型实参的头 prim;缺省 None)。
fn vector_elem_prim(seg: &ast::PathSegment) -> Option<PrimTy> {
    let args = seg.args.as_ref()?;
    for a in &args.args {
        if let ast::GenericArg::Type(t) = a {
            return ty_head_name(t).and_then(PrimTy::from_name);
        }
    }
    None
}

/// 类型头名(`Texture2D<f32>` → "Texture2D";`&T`/`*T`/`(T)` 取内层头;
/// 非路径类型 → None)。
pub(crate) fn ty_head_name(ty: &ast::Ty) -> Option<&str> {
    match &ty.kind {
        TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            ty_head_name(inner)
        }
        _ => None,
    }
}

/// 剥 `&T`/`*T`/`(T)` 外层,取内层类型(用于类型映射)。
pub(crate) fn unwrap_ty(ty: &ast::Ty) -> &ast::Ty {
    match &ty.kind {
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            unwrap_ty(inner)
        }
        _ => ty,
    }
}

// ---- mesh 入口标注元数据提取(G4.2,RXS-0275) -------------------------------
//
// `#[numthreads(x,y,z)]` + `#[outputs(topology="triangles",max_vertices=N,
// max_primitives=M)]` 自 AST 属性提取为 [`MeshEntryMeta`],供 Vulkan codegen
// `lower_mesh` 发射 `LocalSize`/`OutputVertices`/`OutputPrimitivesEXT`/
// `OutputTrianglesEXT` execution modes。属性合法性由 `shader_stages::
// check_mesh_entry` 预先校验(RXS-0243 → RX3017);本函数仅做提取,缺失 /
// 非法则返回 None(保守不误报,零新诊断)。

/// 提取 mesh 入口标注元数据(G4.2,RXS-0275)。
pub(crate) fn mesh_meta_for(
    file: &ast::SourceFile,
    src: &str,
    fn_name: &str,
) -> Option<MeshEntryMeta> {
    let item = find_stage_item(&file.items, fn_name, ShaderStage::Mesh)?;
    let ast::ItemKind::Fn(_) = &item.kind else {
        return None;
    };
    let attrs = &item.attrs;
    let numthreads = parse_numthreads(attrs, src)?;
    let (max_vertices, max_primitives) = parse_outputs(attrs, src)?;
    Some(MeshEntryMeta {
        numthreads,
        max_vertices,
        max_primitives,
    })
}

/// 按名 + 阶段查找图形阶段函数所在 Item(含 attrs;含嵌套 mod)。
fn find_stage_item<'a>(
    items: &'a [ast::Item],
    name: &str,
    stage: ShaderStage,
) -> Option<&'a ast::Item> {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) if f.stage == Some(stage) && f.name.name == name => {
                return Some(it);
            }
            ast::ItemKind::Mod(m) => {
                if let Some(found) = find_stage_item(&m.items, name, stage) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// 单段路径的名字(`#[numthreads]` → "numthreads";多段 → None)。
fn single_seg_path(p: &ast::Path) -> Option<&str> {
    match p.segments.as_slice() {
        [seg] => Some(seg.ident.name.as_str()),
        _ => None,
    }
}

/// 属性列表中按名查找(单段路径匹配)。
fn attr_by_name<'a>(attrs: &'a [ast::Attr], name: &str) -> Option<&'a ast::Attr> {
    attrs
        .iter()
        .find(|a| single_seg_path(&a.meta.path) == Some(name))
}

/// 正整数字面量值(非 Int / 非正 / 解析失败 → None)。数字后缀(`64u32`)容忍。
fn lit_pos_int(src: &str, lit: &ast::Lit) -> Option<u32> {
    if lit.kind != LitKind::Int {
        return None;
    }
    let text = src.get(lit.span.lo.0 as usize..lit.span.hi.0 as usize)?;
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

/// `#[numthreads(x, y, z)]` → 三正整数字面量。
fn parse_numthreads(attrs: &[ast::Attr], src: &str) -> Option<(u32, u32, u32)> {
    let nt = attr_by_name(attrs, "numthreads")?;
    let MetaKind::List(inner) = &nt.meta.kind else {
        return None;
    };
    let dims: Vec<u32> = inner
        .iter()
        .filter_map(|e| match e {
            MetaInner::Lit(l) => lit_pos_int(src, l),
            _ => None,
        })
        .collect();
    match dims.as_slice() {
        [x, y, z] => Some((*x, *y, *z)),
        _ => None,
    }
}

/// `#[outputs(topology="triangles", max_vertices=N, max_primitives=M)]`
/// → (max_vertices, max_primitives)。topology 非 triangles / 缺字段 → None
/// (合法性由 shader_stages 预校验;本函数保守返回 None 不发诊断)。
fn parse_outputs(attrs: &[ast::Attr], src: &str) -> Option<(u32, u32)> {
    let outputs = attr_by_name(attrs, "outputs")?;
    let MetaKind::List(inner) = &outputs.meta.kind else {
        return None;
    };
    let mut topology: Option<String> = None;
    let mut max_vertices: Option<u32> = None;
    let mut max_primitives: Option<u32> = None;
    for entry in inner {
        let MetaInner::Meta(mi) = entry else { continue };
        let (Some(key), MetaKind::NameValue(lit)) = (single_seg_path(&mi.path), &mi.kind) else {
            continue;
        };
        match key {
            "topology" if lit.kind == LitKind::Str => {
                topology = Some(
                    src.get(lit.span.lo.0 as usize..lit.span.hi.0 as usize)?
                        .trim_matches('"')
                        .to_owned(),
                );
            }
            "max_vertices" => max_vertices = lit_pos_int(src, lit),
            "max_primitives" => max_primitives = lit_pos_int(src, lit),
            _ => {}
        }
    }
    if topology.as_deref() != Some("triangles") {
        return None;
    }
    Some((max_vertices?, max_primitives?))
}
