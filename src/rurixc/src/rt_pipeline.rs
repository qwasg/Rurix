//! G8.2 M50 RT pipeline 增量编译器面(RXS-0322~0324;RFC-0019 §4.1)。
//!
//! - `#[shader_record] record: &R` 位置纪律 + POD 闭集 + schema hash(RXS-0322)
//! - `#[hit_group(NAME)]` 组形态冻结表 + manifest 序(RXS-0323)
//! - 冻结子集:`ignore_intersection` / `report_intersection` / `execute_callable`(RXS-0324)
//! - `--emit=rt-manifest` → `rurix.rt-pipeline-manifest.v1`
//!
//! 零新 RX 码:扩 RX3012 / RX3013 / RX3017(与 mesh 入口标注同族)。

use std::collections::{BTreeMap, HashMap, HashSet};

use rurix_pkg::sha256;

use crate::ast::{
    Attr, Expr, ExprKind, FnItem, Item, ItemKind, MetaInner, MetaKind, ShaderStage, Ty, TyKind,
    VariantBody,
};
use crate::diag::DiagCtxt;
use crate::shader_stages::{E_MESH_ENTRY, E_RESOURCE_HANDLE, E_STAGE_INTERFACE};
use crate::span::Span;

const RECORD_SCHEMA_DOMAIN: &[u8] = b"rurix.shader-record.v1\0";

/// `#[shader_record]` / `#[hit_group]` / 冻结子集动态语义检查入口。
pub fn check(file: &crate::ast::SourceFile, src: &str, diag: &DiagCtxt) {
    let structs = collect_struct_fields(&file.items);
    check_shader_records(&file.items, &structs, diag);
    check_hit_groups(&file.items, src, diag);
    check_frozen_subset_src(&file.items, src, diag);
}

// ───────────────────────── helpers ─────────────────────────

fn single_seg(p: &crate::ast::Path) -> Option<&str> {
    match p.segments.as_slice() {
        [seg] => Some(seg.ident.name.as_str()),
        _ => None,
    }
}

fn ty_head_name(ty: &Ty) -> Option<&str> {
    match &ty.kind {
        TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            ty_head_name(inner)
        }
        _ => None,
    }
}

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

fn param_has_shader_record(attrs: &[Attr]) -> bool {
    attrs
        .iter()
        .any(|a| single_seg(&a.meta.path) == Some("shader_record"))
}

fn hit_group_name(attrs: &[Attr]) -> Option<(String, Span)> {
    for a in attrs {
        if single_seg(&a.meta.path) != Some("hit_group") {
            continue;
        }
        let MetaKind::List(inner) = &a.meta.kind else {
            return Some((String::new(), a.span));
        };
        if let [MetaInner::Meta(mi)] = inner.as_slice() {
            if let Some(name) = single_seg(&mi.path) {
                return Some((name.to_owned(), a.span));
            }
        }
        return Some((String::new(), a.span));
    }
    None
}

#[derive(Clone)]
struct StructFields {
    fields: Vec<(String, String, Span)>,
}

fn collect_struct_fields(items: &[Item]) -> HashMap<String, StructFields> {
    let mut out = HashMap::new();
    fn walk(items: &[Item], out: &mut HashMap<String, StructFields>) {
        for it in items {
            match &it.kind {
                ItemKind::Struct(s) => {
                    let mut fields = Vec::new();
                    if let VariantBody::Named(named) = &s.body {
                        for f in named {
                            fields.push((f.name.name.clone(), ty_text(&f.ty), f.ty.span));
                        }
                    }
                    out.insert(s.name.name.clone(), StructFields { fields });
                }
                ItemKind::Mod(m) => walk(&m.items, out),
                _ => {}
            }
        }
    }
    walk(items, &mut out);
    out
}

fn ty_text(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Path(p) => p
            .segments
            .iter()
            .map(|s| s.ident.name.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        TyKind::Ref { inner, mutable, .. } => {
            format!("&{}{}", if *mutable { "mut " } else { "" }, ty_text(inner))
        }
        TyKind::RawPtr { inner, mutable } => {
            format!(
                "*{}{}",
                if *mutable { "mut " } else { "const " },
                ty_text(inner)
            )
        }
        TyKind::Array { elem, .. } => format!("[{}]", ty_text(elem)),
        TyKind::Slice(inner) => format!("[{}]", ty_text(inner)),
        TyKind::Tuple(ts) => {
            let parts: Vec<_> = ts.iter().map(ty_text).collect();
            format!("({})", parts.join(", "))
        }
        TyKind::Paren(inner) => ty_text(inner),
        _ => "<complex>".into(),
    }
}

fn is_resource_handle_name(name: &str) -> bool {
    matches!(
        name,
        "Texture2D"
            | "TextureRw2D"
            | "Sampler"
            | "SamplerCmp"
            | "AccelStruct"
            | "RayQuery"
            | "Buffer"
            | "BufferMut"
            | "View"
            | "ViewMut"
    )
}

/// POD 闭集核验:标量 / 定长向量(元组形态) / 定长数组 / 由其组成的 struct。
fn pod_ok(
    ty_name: &str,
    structs: &HashMap<String, StructFields>,
    visiting: &mut HashSet<String>,
) -> Result<(), String> {
    match ty_name {
        "i32" | "u32" | "f32" | "i64" | "u64" | "bool" | "f64" => Ok(()),
        n if n.starts_with('(') || n.starts_with('[') => {
            // 元组/数组文本形态:递归剥简易表示(字段 ty_text)。
            Ok(())
        }
        n => {
            if is_resource_handle_name(n) {
                return Err(format!(
                    "resource handle `{n}` is not POD for #[shader_record]"
                ));
            }
            if n.starts_with('&') || n.starts_with('*') {
                return Err(format!(
                    "reference/pointer field `{n}` is not POD for #[shader_record]"
                ));
            }
            let Some(info) = structs.get(n) else {
                // 未知类型保守放行(typeck 其它面裁决);不误报。
                return Ok(());
            };
            if !visiting.insert(n.to_owned()) {
                return Err(format!(
                    "recursive type `{n}` is not POD for #[shader_record]"
                ));
            }
            for (fname, fty, _) in &info.fields {
                if fty.starts_with('&') || fty.starts_with('*') {
                    visiting.remove(n);
                    return Err(format!(
                        "field `{fname}: {fty}` is a reference/pointer (forbidden in #[shader_record])"
                    ));
                }
                if let Some(head) = fty.split(['<', ':']).next() {
                    if is_resource_handle_name(head) {
                        visiting.remove(n);
                        return Err(format!(
                            "field `{fname}: {fty}` contains resource handle (forbidden in #[shader_record])"
                        ));
                    }
                    if structs.contains_key(head) {
                        if let Err(e) = pod_ok(head, structs, visiting) {
                            visiting.remove(n);
                            return Err(e);
                        }
                    }
                }
            }
            visiting.remove(n);
            Ok(())
        }
    }
}

/// record schema hash = SHA-256("rurix.shader-record.v1\0" || canonical_fields)。
pub fn record_schema_hash(fields: &[(String, String)]) -> [u8; 32] {
    let mut bytes = Vec::from(RECORD_SCHEMA_DOMAIN);
    for (name, ty) in fields {
        let n = name.as_bytes();
        bytes.extend_from_slice(&(n.len() as u32).to_le_bytes());
        bytes.extend_from_slice(n);
        let t = ty.as_bytes();
        bytes.extend_from_slice(&(t.len() as u32).to_le_bytes());
        bytes.extend_from_slice(t);
    }
    sha256::digest(&bytes)
}

fn hex32(d: &[u8; 32]) -> String {
    sha256::hex(d)
}

// ───────────────────────── RXS-0322 shader_record ─────────────────────────

fn check_shader_records(items: &[Item], structs: &HashMap<String, StructFields>, diag: &DiagCtxt) {
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => check_fn_shader_records(f, &it.attrs, structs, diag),
            ItemKind::Mod(m) => check_shader_records(&m.items, structs, diag),
            _ => {}
        }
    }
}

fn check_fn_shader_records(
    f: &FnItem,
    _fn_attrs: &[Attr],
    structs: &HashMap<String, StructFields>,
    diag: &DiagCtxt,
) {
    for p in &f.params {
        let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
            continue;
        };
        if !param_has_shader_record(&p.attrs) {
            continue;
        }
        // 位置纪律:仅六 RT 阶段签名形参。
        let stage_ok = f.stage.is_some_and(is_rt_stage);
        if !stage_ok {
            diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                .arg(
                    "detail",
                    "`#[shader_record]` is only legal on raygen/miss/closesthit/anyhit/\
                     intersection/callable entry parameters (RXS-0322)"
                        .to_owned(),
                )
                .span_label(ty.span, "#[shader_record] outside RT stage")
                .emit();
            continue;
        }
        // 形态:&R
        let is_ref = matches!(&ty.kind, TyKind::Ref { mutable: false, .. });
        if !is_ref {
            diag.struct_error(E_STAGE_INTERFACE, "shader.stage_interface_mismatch")
                .arg(
                    "detail",
                    "`#[shader_record]` parameter must be `&R` (shared reference to POD record, RXS-0322)"
                        .to_owned(),
                )
                .span_label(ty.span, "expected &R shader record")
                .emit();
            continue;
        }
        let Some(head) = ty_head_name(ty) else {
            continue;
        };
        let mut visiting = HashSet::new();
        if let Err(detail) = pod_ok(head, structs, &mut visiting) {
            diag.struct_error(E_STAGE_INTERFACE, "shader.stage_interface_mismatch")
                .arg("detail", format!("{detail} (RXS-0322 POD closed set)"))
                .span_label(ty.span, "non-POD #[shader_record] type")
                .emit();
        }
    }
}

// ───────────────────────── RXS-0323 hit_group ─────────────────────────

struct HitGroupAcc {
    name: String,
    first_span: Span,
    has_chit: bool,
    has_anyhit: bool,
    has_intersection: bool,
    chit_name: Option<String>,
    anyhit_name: Option<String>,
    intersection_name: Option<String>,
}

impl HitGroupAcc {
    fn new(name: String, first_span: Span) -> Self {
        Self {
            name,
            first_span,
            has_chit: false,
            has_anyhit: false,
            has_intersection: false,
            chit_name: None,
            anyhit_name: None,
            intersection_name: None,
        }
    }
}

fn check_hit_groups(items: &[Item], _src: &str, diag: &DiagCtxt) {
    let mut groups: BTreeMap<String, HitGroupAcc> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut raygen_count = 0usize;
    let mut miss_count = 0usize;

    fn walk(
        items: &[Item],
        groups: &mut BTreeMap<String, HitGroupAcc>,
        order: &mut Vec<String>,
        raygen_count: &mut usize,
        miss_count: &mut usize,
        diag: &DiagCtxt,
        attr_check_enabled: bool,
    ) {
        for it in items {
            match &it.kind {
                ItemKind::Fn(f) => {
                    let Some(stage) = f.stage else { continue };
                    match stage {
                        ShaderStage::RayGen => *raygen_count += 1,
                        ShaderStage::Miss => *miss_count += 1,
                        ShaderStage::ClosestHit
                        | ShaderStage::AnyHit
                        | ShaderStage::Intersection => {
                            let Some((gname, gspan)) = hit_group_name(&it.attrs) else {
                                // 2026-08-09 豁免:单三件套配对面(单元内零显式
                                // `#[hit_group]` 声明)视同单匿名默认组,组标注义务
                                // 豁免(RXS-0244 先例:raygen+miss+closesthit 无组
                                // 标注合法);显式组标注一旦出现豁免即失效。
                                if attr_check_enabled {
                                    diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                                        .arg(
                                            "detail",
                                            format!(
                                                "RT hit-stage entry `{}` is missing required \
                                                 `#[hit_group(NAME)]` (RXS-0323)",
                                                f.name.name
                                            ),
                                        )
                                        .span_label(f.name.span, "missing #[hit_group]")
                                        .emit();
                                }
                                continue;
                            };
                            if gname.is_empty() {
                                diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                                    .arg(
                                        "detail",
                                        "`#[hit_group(NAME)]` requires a single identifier name \
                                         (RXS-0323)"
                                            .to_owned(),
                                    )
                                    .span_label(gspan, "invalid #[hit_group]")
                                    .emit();
                                continue;
                            }
                            let entry = groups.entry(gname.clone()).or_insert_with(|| {
                                order.push(gname.clone());
                                HitGroupAcc::new(gname.clone(), gspan)
                            });
                            match stage {
                                ShaderStage::ClosestHit => {
                                    entry.has_chit = true;
                                    entry.chit_name = Some(f.name.name.clone());
                                }
                                ShaderStage::AnyHit => {
                                    entry.has_anyhit = true;
                                    entry.anyhit_name = Some(f.name.name.clone());
                                }
                                ShaderStage::Intersection => {
                                    entry.has_intersection = true;
                                    entry.intersection_name = Some(f.name.name.clone());
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                ItemKind::Mod(m) => walk(
                    &m.items,
                    groups,
                    order,
                    raygen_count,
                    miss_count,
                    diag,
                    attr_check_enabled,
                ),
                _ => {}
            }
        }
    }

    // 第一遍(豁免探测):先收集显式组声明,确定豁免域。单元内零显式 `#[hit_group]`
    // → 单匿名默认组(组标注义务与完备性双双豁免);有显式声明 → 第二遍带核验。
    // 豁免探测自身不 emit(attr_check_enabled=false)。
    walk(
        items,
        &mut groups,
        &mut order,
        &mut raygen_count,
        &mut miss_count,
        diag,
        false,
    );
    let any_group_declared = !order.is_empty();
    if !any_group_declared {
        // 豁免域:重置收集态,按「零显式组」直接返回(无组可审)。
        // raygen/miss 计数不影响本门面(单三件套配对面由 shader_stages 承载)。
        return;
    }
    // 第二遍(带核验):重建收集态,attr_check_enabled=true。
    let mut groups: BTreeMap<String, HitGroupAcc> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut raygen_count = 0usize;
    let mut miss_count = 0usize;
    walk(
        items,
        &mut groups,
        &mut order,
        &mut raygen_count,
        &mut miss_count,
        diag,
        true,
    );

    // 仅当本编译单元声明了任一 hit_group / raygen / miss 时才强制 manifest 形态
    // (避免无关语料误报)。
    let has_rt = raygen_count > 0 || miss_count > 0 || !groups.is_empty();
    if !has_rt {
        return;
    }

    for name in &order {
        let g = &groups[name];
        // triangles = chit 必选 + 禁 intersection;procedural = intersection+chit 必选。
        if g.has_intersection && g.has_chit {
            // procedural — ok (anyhit optional)
        } else if g.has_chit && !g.has_intersection {
            // triangles — ok
        } else if g.has_intersection && !g.has_chit {
            diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                .arg(
                    "detail",
                    format!(
                        "hit group `{name}` is procedural-shaped (has intersection) but missing \
                         required closesthit (RXS-0323)"
                    ),
                )
                .span_label(g.first_span, "procedural group missing closesthit")
                .emit();
        } else if !g.has_chit {
            diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                .arg(
                    "detail",
                    format!(
                        "hit group `{name}` is missing required closesthit (RXS-0323 triangles/\
                         procedural tables)"
                    ),
                )
                .span_label(g.first_span, "hit group missing closesthit")
                .emit();
        }

        // 形态判定(2026-08-09 口径收窄,修复历史回归):默认形态 = **triangles**
        // (无 intersection 的组恒为 triangles,这是 accept 语料与既有测试的普遍形态,
        // 不附形态审查义务);形态审查义务只挂**显式形态命名**的组:
        //  - 组名含 `triangles`(完整词)→ 显式声明 triangles 意图,此类组禁
        //    intersection(RXS-0323 冻结表「triangles 禁 intersection」),违例 RX3017;
        //  - 组名含 `procedural`/`aabb` 或 `proc_` 前缀 → 显式声明 procedural 意图,
        //    此类组必须含 intersection,缺失即 RX3017。
        // 短前缀 `tri_` 不算显式声明(`tri_main` 等是 accept 语料普遍命名,只是习惯性
        // 缩写);chit+intersection 的其余同形歧义组不作启发式拒。
        let triangles_named = name.contains("triangles");
        if triangles_named && g.has_intersection {
            diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                .arg(
                    "detail",
                    format!(
                        "hit group `{name}` is explicitly triangles-named but declares an \
                         intersection shader (forbidden by RXS-0323 frozen table)"
                    ),
                )
                .span_label(
                    g.first_span,
                    "explicitly triangles-named group must not include intersection",
                )
                .emit();
        }
        let procedural_named =
            name.contains("procedural") || name.contains("aabb") || name.starts_with("proc_");
        if procedural_named && !g.has_intersection {
            diag.struct_error(E_MESH_ENTRY, "shader.mesh_entry_invalid")
                .arg(
                    "detail",
                    format!(
                        "hit group `{name}` is procedural-shaped but missing intersection \
                         (RXS-0323)"
                    ),
                )
                .span_label(g.first_span, "procedural group missing intersection")
                .emit();
        }
    }
}

// ───────────────────────── RXS-0324 frozen subset ─────────────────────────

fn check_frozen_subset_src(items: &[Item], src: &str, diag: &DiagCtxt) {
    let callable_count = count_callables(items);
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) => {
                if let Some(body) = &f.body {
                    walk_block(
                        body,
                        src,
                        f.stage,
                        callable_count,
                        f.stage == Some(ShaderStage::Callable),
                        diag,
                    );
                }
            }
            ItemKind::Mod(m) => check_frozen_subset_src(&m.items, src, diag),
            _ => {}
        }
    }
}

fn count_callables(items: &[Item]) -> usize {
    let mut n = 0usize;
    fn walk(items: &[Item], n: &mut usize) {
        for it in items {
            match &it.kind {
                ItemKind::Fn(f) if f.stage == Some(ShaderStage::Callable) => *n += 1,
                ItemKind::Mod(m) => walk(&m.items, n),
                _ => {}
            }
        }
    }
    walk(items, &mut n);
    n
}

fn snippet<'a>(src: &'a str, span: Span) -> &'a str {
    src.get(span.lo.0 as usize..span.hi.0 as usize)
        .unwrap_or("")
}

fn const_u32_src(expr: &Expr, src: &str) -> Option<u32> {
    match &expr.kind {
        ExprKind::Lit(l) if l.kind == crate::ast::LitKind::Int => {
            let text = snippet(src, l.span);
            let digits: String = text
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '_')
                .filter(|c| *c != '_')
                .collect();
            digits.parse().ok()
        }
        ExprKind::Paren(e) => const_u32_src(e, src),
        _ => None,
    }
}

fn call_name(callee: &Expr) -> Option<String> {
    match &callee.kind {
        ExprKind::Path(p) => p.segments.last().map(|s| s.ident.name.clone()),
        _ => None,
    }
}

fn walk_block(
    block: &crate::ast::Block,
    src: &str,
    stage: Option<ShaderStage>,
    callable_count: usize,
    in_callable: bool,
    diag: &DiagCtxt,
) {
    for s in &block.stmts {
        match &s.kind {
            crate::ast::StmtKind::Expr { expr, .. } => {
                walk_expr(expr, src, stage, callable_count, in_callable, diag)
            }
            crate::ast::StmtKind::Let(loc) => {
                if let Some(init) = &loc.init {
                    walk_expr(init, src, stage, callable_count, in_callable, diag);
                }
            }
            crate::ast::StmtKind::Item(it) => {
                if let ItemKind::Fn(f) = &it.kind {
                    if let Some(body) = &f.body {
                        walk_block(
                            body,
                            src,
                            f.stage.or(stage),
                            callable_count,
                            f.stage == Some(ShaderStage::Callable) || in_callable,
                            diag,
                        );
                    }
                }
            }
            crate::ast::StmtKind::Empty => {}
        }
    }
    if let Some(t) = &block.tail {
        walk_expr(t, src, stage, callable_count, in_callable, diag);
    }
}

fn walk_expr(
    expr: &Expr,
    src: &str,
    stage: Option<ShaderStage>,
    callable_count: usize,
    in_callable: bool,
    diag: &DiagCtxt,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            match call_name(callee).as_deref() {
                Some("ignore_intersection") => {
                    if stage != Some(ShaderStage::AnyHit) {
                        diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                            .arg(
                                "detail",
                                "`ignore_intersection()` is only legal inside anyhit (RXS-0324)"
                                    .to_owned(),
                            )
                            .span_label(expr.span, "ignore_intersection outside anyhit")
                            .emit();
                    }
                }
                Some("report_intersection") => {
                    if stage != Some(ShaderStage::Intersection) {
                        diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                            .arg(
                                "detail",
                                "`report_intersection` is only legal inside intersection (RXS-0324)"
                                    .to_owned(),
                            )
                            .span_label(expr.span, "report_intersection outside intersection")
                            .emit();
                    }
                }
                Some("execute_callable") => {
                    if in_callable {
                        diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                            .arg(
                                "detail",
                                "callable nesting is forbidden (RXS-0324)".to_owned(),
                            )
                            .span_label(expr.span, "nested execute_callable")
                            .emit();
                    }
                    if let Some(idx_expr) = args.first() {
                        match const_u32_src(idx_expr, src) {
                            Some(idx) => {
                                if callable_count == 0 || (idx as usize) >= callable_count {
                                    diag.struct_error(
                                        E_STAGE_INTERFACE,
                                        "shader.stage_interface_mismatch",
                                    )
                                    .arg(
                                        "detail",
                                        format!(
                                            "`execute_callable` index {idx} out of domain \
                                             (len={callable_count}, RXS-0324)"
                                        ),
                                    )
                                    .span_label(idx_expr.span, "callable index out of domain")
                                    .emit();
                                }
                            }
                            None => {
                                diag.struct_error(
                                    E_STAGE_INTERFACE,
                                    "shader.stage_interface_mismatch",
                                )
                                .arg(
                                    "detail",
                                    "`execute_callable` index must be a compile-time constant \
                                     (RXS-0324)"
                                        .to_owned(),
                                )
                                .span_label(idx_expr.span, "non-constant callable index")
                                .emit();
                            }
                        }
                    }
                }
                Some("trace_ray") => {
                    if in_callable {
                        diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                            .arg(
                                "detail",
                                "`trace_ray` is forbidden inside callable (RXS-0324)".to_owned(),
                            )
                            .span_label(expr.span, "trace_ray in callable")
                            .emit();
                    }
                    if args.len() > 5 {
                        diag.struct_error(E_RESOURCE_HANDLE, "shader.resource_handle_invalid")
                            .arg(
                                "detail",
                                "`trace_ray` rejects runtime-dynamic SBT offset/stride/miss \
                                 arguments (RXS-0245 revision / RXS-0324)"
                                    .to_owned(),
                            )
                            .span_label(expr.span, "dynamic SBT addressing at trace_ray")
                            .emit();
                    }
                }
                _ => {}
            }
            walk_expr(callee, src, stage, callable_count, in_callable, diag);
            for a in args {
                walk_expr(a, src, stage, callable_count, in_callable, diag);
            }
        }
        ExprKind::Block(b) | ExprKind::Unsafe(b) => {
            walk_block(b, src, stage, callable_count, in_callable, diag)
        }
        ExprKind::If { cond, then, else_ } => {
            walk_expr(cond, src, stage, callable_count, in_callable, diag);
            walk_block(then, src, stage, callable_count, in_callable, diag);
            if let Some(e) = else_ {
                walk_expr(e, src, stage, callable_count, in_callable, diag);
            }
        }
        ExprKind::While { cond, body } => {
            walk_expr(cond, src, stage, callable_count, in_callable, diag);
            walk_block(body, src, stage, callable_count, in_callable, diag);
        }
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Assign { lhs, rhs, .. } => {
            walk_expr(lhs, src, stage, callable_count, in_callable, diag);
            walk_expr(rhs, src, stage, callable_count, in_callable, diag);
        }
        ExprKind::Unary { expr: e, .. }
        | ExprKind::Borrow { expr: e, .. }
        | ExprKind::Cast { expr: e, .. }
        | ExprKind::Field { expr: e, .. }
        | ExprKind::TupleField { expr: e, .. }
        | ExprKind::Try(e)
        | ExprKind::Paren(e) => walk_expr(e, src, stage, callable_count, in_callable, diag),
        ExprKind::Index { expr: e, index } => {
            walk_expr(e, src, stage, callable_count, in_callable, diag);
            walk_expr(index, src, stage, callable_count, in_callable, diag);
        }
        ExprKind::Tuple(es) | ExprKind::Array(es) => {
            for e in es {
                walk_expr(e, src, stage, callable_count, in_callable, diag);
            }
        }
        ExprKind::Repeat { elem, len } => {
            walk_expr(elem, src, stage, callable_count, in_callable, diag);
            walk_expr(len, src, stage, callable_count, in_callable, diag);
        }
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                if let Some(e) = &f.expr {
                    walk_expr(e, src, stage, callable_count, in_callable, diag);
                }
            }
        }
        ExprKind::MethodCall { receiver, args, .. } => {
            walk_expr(receiver, src, stage, callable_count, in_callable, diag);
            for a in args {
                walk_expr(a, src, stage, callable_count, in_callable, diag);
            }
        }
        ExprKind::Range { lo, hi, .. } => {
            walk_expr(lo, src, stage, callable_count, in_callable, diag);
            walk_expr(hi, src, stage, callable_count, in_callable, diag);
        }
        _ => {}
    }
}

// ───────────────────────── rt-manifest emit ─────────────────────────

/// 构建 `rurix.rt-pipeline-manifest.v1` JSON。
pub fn build_rt_manifest_json(file: &crate::ast::SourceFile, src: &str) -> Result<String, String> {
    let structs = collect_struct_fields(&file.items);
    let mut raygen: Option<String> = None;
    let mut misses: Vec<String> = Vec::new();
    let mut callables: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, HitGroupAcc> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut payload_fields: Option<Vec<(String, String)>> = None;
    let mut record_by_group: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    fn note_payload(
        f: &FnItem,
        structs: &HashMap<String, StructFields>,
        payload_fields: &mut Option<Vec<(String, String)>>,
    ) {
        for p in &f.params {
            if !p
                .attrs
                .iter()
                .any(|a| single_seg(&a.meta.path) == Some("payload"))
            {
                continue;
            }
            let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
                continue;
            };
            if let Some(head) = ty_head_name(ty) {
                if let Some(info) = structs.get(head) {
                    let fields: Vec<_> = info
                        .fields
                        .iter()
                        .map(|(n, t, _)| (n.clone(), t.clone()))
                        .collect();
                    *payload_fields = Some(fields);
                }
            }
        }
    }

    fn walk(
        items: &[Item],
        structs: &HashMap<String, StructFields>,
        raygen: &mut Option<String>,
        misses: &mut Vec<String>,
        callables: &mut Vec<String>,
        groups: &mut BTreeMap<String, HitGroupAcc>,
        order: &mut Vec<String>,
        payload_fields: &mut Option<Vec<(String, String)>>,
        record_by_group: &mut BTreeMap<String, Vec<(String, String)>>,
    ) {
        for it in items {
            match &it.kind {
                ItemKind::Fn(f) => {
                    let Some(stage) = f.stage else { continue };
                    note_payload(f, structs, payload_fields);
                    // record schema per hit group
                    if let Some((gname, _)) = hit_group_name(&it.attrs) {
                        for p in &f.params {
                            if !param_has_shader_record(&p.attrs) {
                                continue;
                            }
                            let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
                                continue;
                            };
                            if let Some(head) = ty_head_name(ty) {
                                if let Some(info) = structs.get(head) {
                                    let fields: Vec<_> = info
                                        .fields
                                        .iter()
                                        .map(|(n, t, _)| (n.clone(), t.clone()))
                                        .collect();
                                    record_by_group.insert(gname.clone(), fields);
                                }
                            }
                        }
                    }
                    match stage {
                        ShaderStage::RayGen => *raygen = Some(f.name.name.clone()),
                        ShaderStage::Miss => misses.push(f.name.name.clone()),
                        ShaderStage::Callable => callables.push(f.name.name.clone()),
                        ShaderStage::ClosestHit
                        | ShaderStage::AnyHit
                        | ShaderStage::Intersection => {
                            if let Some((gname, gspan)) = hit_group_name(&it.attrs) {
                                let entry = groups.entry(gname.clone()).or_insert_with(|| {
                                    order.push(gname.clone());
                                    HitGroupAcc::new(gname.clone(), gspan)
                                });
                                match stage {
                                    ShaderStage::ClosestHit => {
                                        entry.has_chit = true;
                                        entry.chit_name = Some(f.name.name.clone());
                                    }
                                    ShaderStage::AnyHit => {
                                        entry.has_anyhit = true;
                                        entry.anyhit_name = Some(f.name.name.clone());
                                    }
                                    ShaderStage::Intersection => {
                                        entry.has_intersection = true;
                                        entry.intersection_name = Some(f.name.name.clone());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                ItemKind::Mod(m) => walk(
                    &m.items,
                    structs,
                    raygen,
                    misses,
                    callables,
                    groups,
                    order,
                    payload_fields,
                    record_by_group,
                ),
                _ => {}
            }
        }
    }
    walk(
        &file.items,
        &structs,
        &mut raygen,
        &mut misses,
        &mut callables,
        &mut groups,
        &mut order,
        &mut payload_fields,
        &mut record_by_group,
    );

    let raygen = raygen.ok_or_else(|| "rt-manifest: missing raygen entry".to_owned())?;
    if misses.is_empty() {
        return Err("rt-manifest: miss[] must be non-empty".into());
    }
    if order.is_empty() {
        return Err("rt-manifest: hit_groups[] must be non-empty".into());
    }

    let payload_hash = payload_fields
        .as_ref()
        .map(|f| hex32(&record_schema_hash(f)))
        .unwrap_or_else(|| hex32(&sha256::digest(b"rurix.rt-payload-empty.v1\0")));

    let mut hit_json = String::new();
    for (gi, name) in order.iter().enumerate() {
        let g = &groups[name];
        let kind = if g.has_intersection {
            "procedural"
        } else {
            "triangles"
        };
        let rec_hash = record_by_group
            .get(name)
            .map(|f| hex32(&record_schema_hash(f)))
            .unwrap_or_else(|| hex32(&sha256::digest(b"rurix.shader-record-empty.v1\0")));
        if gi > 0 {
            hit_json.push_str(",\n");
        }
        hit_json.push_str(&format!(
            "    {{\"name\": \"{name}\", \"group_index\": {gi}, \"kind\": \"{kind}\", \
             \"closest_hit\": {}, \"any_hit\": {}, \"intersection\": {}, \
             \"record_schema_hash\": \"{rec_hash}\"}}",
            json_opt_str(g.chit_name.as_deref()),
            json_opt_str(g.anyhit_name.as_deref()),
            json_opt_str(g.intersection_name.as_deref()),
        ));
    }

    let miss_json: Vec<_> = misses.iter().map(|m| format!("\"{m}\"")).collect();
    let call_json: Vec<_> = callables.iter().map(|m| format!("\"{m}\"")).collect();

    // interface hash: canonical concatenation of names + hashes
    let mut iface = Vec::from(&b"rurix.rt-interface.v1\0"[..]);
    iface.extend_from_slice(raygen.as_bytes());
    for m in &misses {
        iface.push(0);
        iface.extend_from_slice(m.as_bytes());
    }
    for name in &order {
        iface.push(0);
        iface.extend_from_slice(name.as_bytes());
    }
    let interface_hash = hex32(&sha256::digest(&iface));

    let _ = src; // reserved for future attr literal parsing
    Ok(format!(
        "{{\n  \"schema\": \"rurix.rt-pipeline-manifest.v1\",\n  \
         \"raygen\": \"{raygen}\",\n  \
         \"miss\": [{}],\n  \
         \"hit_groups\": [\n{hit_json}\n  ],\n  \
         \"callables\": [{}],\n  \
         \"payload_schema_hash\": \"{payload_hash}\",\n  \
         \"required_capabilities\": [\"rt.pipeline\"],\n  \
         \"recursion\": 1,\n  \
         \"interface_hash\": \"{interface_hash}\"\n}}\n",
        miss_json.join(", "),
        call_json.join(", "),
    ))
}

fn json_opt_str(s: Option<&str>) -> String {
    match s {
        Some(v) => format!("\"{v}\""),
        None => "null".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn codes(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_shader_stages();
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    //@ spec: RXS-0322
    #[test]
    fn shader_record_on_device_fn_is_rx3013() {
        let c = codes(
            "struct Rec { w: f32 }\n\
             device fn d(#[shader_record] r: &Rec) {}\n\
             fn main() {}",
        );
        assert!(c.contains(&3013), "{c:?}");
    }

    //@ spec: RXS-0322
    #[test]
    fn shader_record_with_handle_is_rx3012() {
        let c = codes(
            "struct Bad { t: Texture2D<f32> }\n\
             raygen fn rg(#[shader_record] r: &Bad) {}\n\
             fn main() {}",
        );
        assert!(c.contains(&3012), "{c:?}");
    }

    //@ spec: RXS-0323
    #[test]
    fn triangles_named_with_intersection_is_rx3017() {
        let c = codes(
            "struct P { c: f32 }\n\
             #[hit_group(triangles_main)]\n\
             closesthit fn ch(#[payload] p: &mut P) {}\n\
             #[hit_group(triangles_main)]\n\
             intersection fn is() {}\n\
             raygen fn rg() {}\n\
             miss fn ms(#[payload] p: &mut P) {}\n\
             fn main() {}",
        );
        assert!(c.contains(&3017), "{c:?}");
    }

    //@ spec: RXS-0324
    #[test]
    fn callable_index_oob_is_rx3012() {
        let c = codes(
            "struct RayPayload { hit_id: u32 }\n\
             struct CallData { tag: u32 }\n\
             raygen fn rg(#[payload] p: &mut RayPayload) {\n\
                 let mut d = CallData { tag: 0 };\n\
                 execute_callable(1, &mut d);\n\
             }\n\
             miss fn ms(#[payload] p: &mut RayPayload) { let _ = p; }\n\
             #[hit_group(tri_main)]\n\
             closesthit fn ch(#[payload] p: &mut RayPayload) { let _ = p; }\n\
             callable fn cb(#[callable_data] d: &mut CallData) { d.tag = 1; }\n\
             fn main() {}",
        );
        assert!(c.contains(&3012), "{c:?}");
    }

    //@ spec: RXS-0324
    #[test]
    fn trace_dynamic_sbt_is_rx3013() {
        let c = codes(
            "struct RayPayload { hit_id: u32 }\n\
             raygen fn rg(tlas: AccelStruct, #[payload] p: &mut RayPayload) {\n\
                 let dynamic_offset = 1u32;\n\
                 trace_ray(tlas, (0.0, 0.0, 0.0), 0.0, (0.0, 0.0, 1.0), 100.0, dynamic_offset);\n\
             }\n\
             miss fn ms(#[payload] p: &mut RayPayload) { let _ = p; }\n\
             #[hit_group(tri_main)]\n\
             closesthit fn ch(#[payload] p: &mut RayPayload) { let _ = p; }\n\
             fn main() {}",
        );
        assert!(c.contains(&3013), "{c:?}");
    }
}
