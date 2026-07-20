//! `#[export(c)]` C ABI 导出 codegen 支撑(EI1.2,RFC-0014 Part A,RXS-0250~0255)。
//!
//! 单一事实源(§4.0-1):`#[export(c)]` 导出集经收集/校验后,**同一份 C 映射
//! 既产 link.exe `/EXPORT:` 参数([`export_directives`])、又产内建头文件声明
//! ([`generate_header`])**——导出符号与头声明恒逐一对应,无第二事实源
//! (否决 obj 内 `dllexport` 标注,RFC-0014 §7-1)。
//!
//! 覆盖 `.rx` 出口(生成路);手写路 RXS-0125/`src/rurix-interop` + RXS-0149 守卫
//! /`src/rurix-engine` 冻结覆盖 Rust crate 出口,两制共存(RXS-0254,§4.A5)。

use crate::ast::{
    Attr, Block, Expr, ExprKind, FnColor, Item, ItemKind, LitKind, MetaInner, MetaKind, Stmt,
    StmtKind, Ty, TyKind, Visibility,
};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::source_map::SourceMap;
use crate::span::Span;

/// `#[export(c)]` 属性挂载对象非法(仅 host `pub fn`;RXS-0250,§4.A1)。
pub const E_EXPORT_C_TARGET: ErrorCode = ErrorCode(6033); // RX6033
/// `#[export(c)]` 导出签名或体超出 C 兼容子集 v1(RXS-0251 签名 / RXS-0255 体,§4.A2/§4.A6)。
pub const E_EXPORT_C_SUBSET: ErrorCode = ErrorCode(6031); // RX6031
/// `--emit=dll` 但无 `#[export(c)]` 导出(空导出集;RXS-0252,§4.A4)。
pub const E_EXPORT_C_EMPTY: ErrorCode = ErrorCode(6032); // RX6032

/// C 兼容子集 v1 类型(RXS-0251 类型映射表,§4.A2)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    /// `*mut T` / `*const T`(T ∈ 标量);`is_const` = `*const`。documented unsafe
    /// FFI 边界(§4.A6):codegen 不引入隐式解引用,有效性/对齐/别名为调用方前置条件。
    Ptr {
        is_const: bool,
        pointee: Box<CType>,
    },
    /// unit `()`,仅返回位合法映射 `void`。
    Void,
}

impl CType {
    /// C 头声明文本(定宽整型 `<stdint.h>` / IEEE 浮点 / `void`)。
    pub fn c_decl(&self) -> String {
        match self {
            CType::I8 => "int8_t".to_owned(),
            CType::I16 => "int16_t".to_owned(),
            CType::I32 => "int32_t".to_owned(),
            CType::I64 => "int64_t".to_owned(),
            CType::U8 => "uint8_t".to_owned(),
            CType::U16 => "uint16_t".to_owned(),
            CType::U32 => "uint32_t".to_owned(),
            CType::U64 => "uint64_t".to_owned(),
            CType::F32 => "float".to_owned(),
            CType::F64 => "double".to_owned(),
            CType::Bool => "bool".to_owned(),
            CType::Void => "void".to_owned(),
            CType::Ptr { is_const, pointee } => {
                if *is_const {
                    format!("const {}*", pointee.c_decl())
                } else {
                    format!("{}*", pointee.c_decl())
                }
            }
        }
    }
}

/// 单个 `#[export(c)]` 导出(收集/校验完成后的规范化事实,§4.0-1 单一事实源)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CExport {
    /// 导出符号名(fn 名或 `name="…"` 覆写值);保名不 mangle,发射 IR 符号名。
    pub symbol: String,
    /// 源 fn 名(供 MIR 导出根 DefId 解析;`name=` 覆写时 ≠ `symbol`)。
    pub fn_name: String,
    pub params: Vec<CType>,
    pub ret: CType,
    pub span: Span,
}

impl CExport {
    /// C 函数声明行(`int32_t rurix_add(int32_t, int32_t);`)。
    fn c_prototype(&self) -> String {
        let params = if self.params.is_empty() {
            "void".to_owned()
        } else {
            self.params
                .iter()
                .map(CType::c_decl)
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!("{} {}({});", self.ret.c_decl(), self.symbol, params)
    }
}

/// 标量类型名 → [`CType`](RXS-0251)。
fn scalar_ctype(name: &str) -> Option<CType> {
    Some(match name {
        "i8" => CType::I8,
        "i16" => CType::I16,
        "i32" => CType::I32,
        "i64" => CType::I64,
        "u8" => CType::U8,
        "u16" => CType::U16,
        "u32" => CType::U32,
        "u64" => CType::U64,
        "f32" => CType::F32,
        "f64" => CType::F64,
        "bool" => CType::Bool,
        _ => return None,
    })
}

/// [`Ty`] → C 兼容子集 v1 [`CType`];`allow_unit` 仅返回位为真(unit → `void`)。
/// `None` = 超出子集 v1(RXS-0251 守门:防未定义 ABI 布局静默逃逸)。
fn ty_to_ctype(ty: &Ty, allow_unit: bool) -> Option<CType> {
    match &ty.kind {
        TyKind::Paren(inner) => ty_to_ctype(inner, allow_unit),
        TyKind::Path(p) => {
            if p.segments.len() != 1 {
                return None;
            }
            scalar_ctype(p.segments[0].ident.name.as_str())
        }
        TyKind::RawPtr { mutable, inner } => {
            // 指针 pointee 必为标量(子集 v1);嵌套指针/复合超界。
            let TyKind::Path(p) = &inner.kind else {
                return None;
            };
            if p.segments.len() != 1 {
                return None;
            }
            let pointee = scalar_ctype(p.segments[0].ident.name.as_str())?;
            Some(CType::Ptr {
                is_const: !mutable,
                pointee: Box::new(pointee),
            })
        }
        TyKind::Tuple(elems) if elems.is_empty() && allow_unit => Some(CType::Void),
        _ => None,
    }
}

/// `#[export(c)]` 属性识别:返回 `Some(name_override)`;`name` 为 `name="…"` 覆写值
/// (无覆写 = `None`)。非 `#[export(c)]` 属性返回 `None`(经 outer 布尔与 `is_export` 分辨)。
fn parse_export_attr(attr: &Attr, sm: &SourceMap) -> Option<Option<String>> {
    if attr.inner || attr.meta.path.segments.len() != 1 {
        return None;
    }
    if attr.meta.path.segments[0].ident.name != "export" {
        return None;
    }
    // `#[export(c)]` / `#[export(c, name = "…")]`
    let MetaKind::List(inner) = &attr.meta.kind else {
        return Some(None); // `#[export]` 无参:后续合法性校验按无 `c` 处理
    };
    let mut name_override = None;
    for entry in inner {
        let MetaInner::Meta(mi) = entry else { continue };
        if mi.path.segments.len() != 1 {
            continue;
        }
        match mi.path.segments[0].ident.name.as_str() {
            "c" => {}
            "name" => {
                if let MetaKind::NameValue(lit) = &mi.kind
                    && lit.kind == LitKind::Str
                {
                    let v = sm.snippet(lit.span).trim_matches('"').to_owned();
                    if !v.is_empty() {
                        name_override = Some(v);
                    }
                }
            }
            _ => {}
        }
    }
    Some(name_override)
}

/// panic 面扫描(RXS-0255,§4.A6):subset v1 导出体仅 C 兼容算术,结构上不含可
/// panic 面。检出数组/切片索引(数组越界)、`?`、`unwrap`/`expect`(显式 panic 面)
/// → 返回 `Some(span)`(编译期 strict 拒)。「无 panic 面 by-construction」的诚实兑现:
/// 非运行期终止契约,而是编译期结构性保证。
fn find_panic_face(body: &Block) -> Option<Span> {
    fn scan_expr(e: &Expr) -> Option<Span> {
        match &e.kind {
            ExprKind::Index { .. } => Some(e.span), // 数组/切片索引 → 越界 panic 面
            ExprKind::Try(_) => Some(e.span),       // `?` → panic/传播面
            ExprKind::MethodCall { method, .. }
                if matches!(method.name.as_str(), "unwrap" | "expect") =>
            {
                Some(e.span)
            }
            // 递归子表达式
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. }
            | ExprKind::Paren(expr) => scan_expr(expr),
            ExprKind::Binary { lhs, rhs, .. } | ExprKind::Assign { lhs, rhs, .. } => {
                scan_expr(lhs).or_else(|| scan_expr(rhs))
            }
            ExprKind::Range { lo, hi, .. } => scan_expr(lo).or_else(|| scan_expr(hi)),
            ExprKind::Call { callee, args } => {
                scan_expr(callee).or_else(|| args.iter().find_map(scan_expr))
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                scan_expr(receiver).or_else(|| args.iter().find_map(scan_expr))
            }
            ExprKind::Tuple(xs) | ExprKind::Array(xs) => xs.iter().find_map(scan_expr),
            ExprKind::Repeat { elem, len } => scan_expr(elem).or_else(|| scan_expr(len)),
            ExprKind::StructLit { fields, .. } => fields
                .iter()
                .find_map(|f| f.expr.as_ref().and_then(scan_expr)),
            ExprKind::Block(b) | ExprKind::Unsafe(b) => scan_block(b),
            ExprKind::If {
                cond, then, else_, ..
            } => scan_expr(cond)
                .or_else(|| scan_block(then))
                .or_else(|| else_.as_deref().and_then(scan_expr)),
            // 循环 / match / 闭包体**必须递归**:`_ => None` 兜底曾令循环体内的
            // Index·`?`·unwrap 静默逃逸,使 RXS-0255「结构性保证」出洞(EI1.4 实证)。
            ExprKind::While { cond, body } => scan_expr(cond).or_else(|| scan_block(body)),
            ExprKind::Loop { body } => scan_block(body),
            ExprKind::For { iter, body, .. } => scan_expr(iter).or_else(|| scan_block(body)),
            ExprKind::Match { scrutinee, arms } => scan_expr(scrutinee).or_else(|| {
                arms.iter().find_map(|a| {
                    a.guard
                        .as_ref()
                        .and_then(scan_expr)
                        .or_else(|| scan_expr(&a.body))
                })
            }),
            ExprKind::Closure { body, .. } => scan_expr(body),
            ExprKind::Return(x) | ExprKind::Break(x) => x.as_deref().and_then(scan_expr),
            // **穷尽枚举(无 `_` 兜底)**:新增 ExprKind 变体须在此显式判档
            // (递归 or 判定为无 panic 面叶子),编译错即强制决策——fail-closed,
            // 防再次出现「静默跳过 = 保证出洞」(RXS-0255 结构性保证的守门纪律)。
            ExprKind::Lit(_) | ExprKind::Path(_) | ExprKind::Continue | ExprKind::Err => None,
        }
    }
    fn scan_block(b: &Block) -> Option<Span> {
        for s in &b.stmts {
            if let Some(sp) = scan_stmt(s) {
                return Some(sp);
            }
        }
        b.tail.as_deref().and_then(scan_expr)
    }
    fn scan_stmt(s: &Stmt) -> Option<Span> {
        match &s.kind {
            StmtKind::Expr { expr, .. } => scan_expr(expr),
            StmtKind::Let(l) => l.init.as_ref().and_then(scan_expr),
            _ => None,
        }
    }
    scan_block(body)
}

/// 收集并校验 crate 内全部 `#[export(c)]` 导出(RXS-0250~0251/0255)。
/// 非法项经 `diag` 报编译期 strict 诊断并跳过;合法项规范化入返回集。
/// 递归进 `mod` 项。
//@ spec: RXS-0250, RXS-0251, RXS-0255
pub fn collect_c_exports(items: &[Item], sm: &SourceMap, diag: &DiagCtxt) -> Vec<CExport> {
    let mut out = Vec::new();
    collect_into(items, sm, diag, &mut out);
    out
}

fn collect_into(items: &[Item], sm: &SourceMap, diag: &DiagCtxt, out: &mut Vec<CExport>) {
    for item in items {
        if let ItemKind::Mod(m) = &item.kind {
            collect_into(&m.items, sm, diag, out);
            continue;
        }
        // 找 `#[export(c)]`
        let mut export_attr: Option<(Option<String>, Span)> = None;
        for attr in &item.attrs {
            if let Some(name_override) = parse_export_attr(attr, sm) {
                export_attr = Some((name_override, attr.span));
                break;
            }
        }
        let Some((name_override, attr_span)) = export_attr else {
            continue;
        };

        // RXS-0250 合法性:仅 host `pub fn`。
        let ItemKind::Fn(f) = &item.kind else {
            diag.struct_error(E_EXPORT_C_TARGET, "export_c.attr_target")
                .arg("detail", "属性只能挂在函数项上")
                .span_label(attr_span, "`#[export(c)]` 挂载对象非 fn item")
                .emit();
            continue;
        };
        if f.color != FnColor::Host {
            diag.struct_error(E_EXPORT_C_TARGET, "export_c.attr_target")
                .arg(
                    "detail",
                    "属性只能挂在 host 函数上(不可 device/kernel/着色阶段)",
                )
                .span_label(attr_span, "`#[export(c)]` 挂载对象非 host fn")
                .emit();
            continue;
        }
        if !matches!(item.vis, Visibility::Pub(_)) {
            diag.struct_error(E_EXPORT_C_TARGET, "export_c.attr_target")
                .arg("detail", "属性只能挂在 `pub` 函数上")
                .span_label(attr_span, "`#[export(c)]` 挂载对象非 `pub` fn")
                .emit();
            continue;
        }

        let fn_name = f.name.name.clone();
        let symbol = name_override.unwrap_or_else(|| fn_name.clone());

        // RXS-0251 C 兼容签名子集 v1。
        let mut params = Vec::new();
        let mut sig_ok = true;
        for p in &f.params {
            let crate::ast::ParamKind::Typed { ty, .. } = &p.kind else {
                // `self` 参数不可能出现在 host free fn 导出;保守拒。
                diag.struct_error(E_EXPORT_C_SUBSET, "export_c.subset")
                    .arg("detail", "导出函数不支持 `self` 接收者")
                    .span_label(p.span, "非 C 兼容参数")
                    .emit();
                sig_ok = false;
                break;
            };
            match ty_to_ctype(ty, false) {
                Some(ct) => params.push(ct),
                None => {
                    diag.struct_error(E_EXPORT_C_SUBSET, "export_c.subset")
                        .arg(
                            "detail",
                            "参数类型超出子集 v1(仅标量 / `*mut T`·`*const T`〔T 标量〕)",
                        )
                        .span_label(ty.span, "非 C 兼容参数类型")
                        .emit();
                    sig_ok = false;
                    break;
                }
            }
        }
        if !sig_ok {
            continue;
        }
        // 返回类型:`None`(无 `-> T`) = unit;允许 unit → void。
        let ret = match &f.ret {
            None => CType::Void,
            Some(ty) => match ty_to_ctype(ty, true) {
                Some(ct) => ct,
                None => {
                    diag.struct_error(E_EXPORT_C_SUBSET, "export_c.subset")
                        .arg(
                            "detail",
                            "返回类型超出子集 v1(仅标量 / `*mut T`·`*const T` / unit)",
                        )
                        .span_label(ty.span, "非 C 兼容返回类型")
                        .emit();
                    continue;
                }
            },
        };

        // RXS-0255 panic 面 by-construction:导出体禁含可 panic 面。
        if let Some(body) = &f.body
            && let Some(sp) = find_panic_face(body)
        {
            diag.struct_error(E_EXPORT_C_SUBSET, "export_c.subset")
                .arg(
                    "detail",
                    "导出体含可 panic 面(数组越界 / `?` / `unwrap`·`expect`);subset v1 须无 panic 面(RXS-0255)",
                )
                .span_label(sp, "可 panic 面")
                .emit();
            continue;
        }

        out.push(CExport {
            symbol,
            fn_name,
            params,
            ret,
            span: attr_span,
        });
    }
}

/// link.exe `/EXPORT:name` 指令序列(driver 从导出集拼参数传 link.exe,§4.A3)。
/// 与 [`generate_header`] 同源单一事实源(§4.0-1)。
//@ spec: RXS-0252
pub fn export_directives(exports: &[CExport]) -> Vec<String> {
    exports
        .iter()
        .map(|e| format!("/EXPORT:{}", e.symbol))
        .collect()
}

/// 内建头文件确定性生成(RXS-0253,§4.A5):LF 行尾、无时间戳、无绝对路径、
/// 两次逐字节一致(幂等)。每声明 ↔ 恰一 DLL 导出符号(承 RXS-0149 逐一对应)。
/// CI 再生成逐字节比对守卫覆盖 `.rx` 出口(RXS-0254)。`guard` = include guard 宏名。
//@ spec: RXS-0253, RXS-0254
pub fn generate_header(exports: &[CExport], guard: &str) -> String {
    let mut s = String::new();
    s.push_str("/* Generated by rurixc --emit=dll (RXS-0253). Do not edit. */\n");
    s.push_str(&format!("#ifndef {guard}\n#define {guard}\n"));
    s.push_str("#include <stdint.h>\n");
    s.push_str("#include <stdbool.h>\n");
    s.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
    for e in exports {
        s.push_str(&e.c_prototype());
        s.push('\n');
    }
    s.push_str("\n#ifdef __cplusplus\n}\n#endif\n");
    s.push_str(&format!("#endif /* {guard} */\n"));
    s
}
