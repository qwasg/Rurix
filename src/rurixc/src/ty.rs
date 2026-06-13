//! 类型表示(spec 条款 RXS-0039 ~ RXS-0047,spec/types.md;07 §3)。
//!
//! 树形 MVP(intern 化随 M2.3 评估);[`Ty::Err`] 为容忍区类型:来自 M2.1
//! 名称解析容忍口径(`Res::Err`)或错误恢复,参与一切检查时静默通过
//! (RXS-0047 "Err 容忍不级联")。

use crate::hir::{DefId, PrimTy};
use crate::resolve::Resolutions;

/// 推断变量 id([`crate::typeck`] 的 InferCtxt 槽位)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TyVid(pub u32);

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ty {
    Prim(PrimTy),
    /// struct / enum + 类型实参(单态化实例,RXS-0045)。
    Adt(DefId, Vec<Ty>),
    /// `()` = `Tuple([])`。
    Tuple(Vec<Ty>),
    Ref(Box<Ty>, bool),
    RawPtr(Box<Ty>, bool),
    /// 数组(长度不入类型,const eval 随 M3)。
    Array(Box<Ty>),
    Slice(Box<Ty>),
    FnPtr(Vec<Ty>, Box<Ty>),
    /// 泛型参数(单态化替换点,序号 = `Res::GenericParam`)。
    Param(u32),
    /// 推断变量(body 内短期存在,RXS-0041)。
    Infer(TyVid),
    /// 容忍区(RXS-0047:参与检查静默通过,不级联)。
    Err,
}

impl Ty {
    pub fn unit() -> Ty {
        Ty::Tuple(Vec::new())
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, Ty::Tuple(v) if v.is_empty())
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Ty::Err)
    }

    pub fn is_int(&self) -> bool {
        matches!(
            self,
            Ty::Prim(
                PrimTy::I8
                    | PrimTy::I16
                    | PrimTy::I32
                    | PrimTy::I64
                    | PrimTy::U8
                    | PrimTy::U16
                    | PrimTy::U32
                    | PrimTy::U64
                    | PrimTy::Usize
            )
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::Prim(PrimTy::F32 | PrimTy::F64))
    }

    pub fn is_numeric(&self) -> bool {
        self.is_int() || self.is_float()
    }

    /// 泛型参数替换(单态化实例化,RXS-0045)。
    pub fn subst(&self, args: &[Ty]) -> Ty {
        match self {
            Ty::Param(i) => args.get(*i as usize).cloned().unwrap_or(Ty::Err),
            Ty::Adt(d, a) => Ty::Adt(*d, a.iter().map(|t| t.subst(args)).collect()),
            Ty::Tuple(v) => Ty::Tuple(v.iter().map(|t| t.subst(args)).collect()),
            Ty::Ref(t, m) => Ty::Ref(Box::new(t.subst(args)), *m),
            Ty::RawPtr(t, m) => Ty::RawPtr(Box::new(t.subst(args)), *m),
            Ty::Array(t) => Ty::Array(Box::new(t.subst(args))),
            Ty::Slice(t) => Ty::Slice(Box::new(t.subst(args))),
            Ty::FnPtr(ps, r) => Ty::FnPtr(
                ps.iter().map(|t| t.subst(args)).collect(),
                Box::new(r.subst(args)),
            ),
            _ => self.clone(),
        }
    }

    /// 诊断渲染(RXS-0047:期待/实际类型文本)。
    pub fn render(&self, res: &Resolutions) -> String {
        match self {
            Ty::Prim(p) => prim_name(*p).to_owned(),
            Ty::Adt(d, args) => {
                let name = res
                    .defs
                    .get(d.0 as usize)
                    .map(|dd| dd.name.clone())
                    .unwrap_or_else(|| "<adt>".to_owned());
                if args.is_empty() {
                    format!("`{name}`")
                } else {
                    let a: Vec<String> = args.iter().map(|t| t.render_inner(res)).collect();
                    format!("`{name}<{}>`", a.join(", "))
                }
            }
            _ => format!("`{}`", self.render_inner(res)),
        }
    }

    /// 无引号渲染(MIR 文本等非诊断场景)。
    pub fn render_plain(&self, res: &Resolutions) -> String {
        self.render_inner(res)
    }

    fn render_inner(&self, res: &Resolutions) -> String {
        match self {
            Ty::Prim(p) => prim_name(*p).to_owned(),
            Ty::Adt(d, args) => {
                let name = res
                    .defs
                    .get(d.0 as usize)
                    .map(|dd| dd.name.clone())
                    .unwrap_or_else(|| "<adt>".to_owned());
                if args.is_empty() {
                    name
                } else {
                    let a: Vec<String> = args.iter().map(|t| t.render_inner(res)).collect();
                    format!("{name}<{}>", a.join(", "))
                }
            }
            Ty::Tuple(v) if v.is_empty() => "()".to_owned(),
            Ty::Tuple(v) => {
                let a: Vec<String> = v.iter().map(|t| t.render_inner(res)).collect();
                format!("({})", a.join(", "))
            }
            Ty::Ref(t, m) => format!("&{}{}", if *m { "mut " } else { "" }, t.render_inner(res)),
            Ty::RawPtr(t, m) => format!(
                "*{} {}",
                if *m { "mut" } else { "const" },
                t.render_inner(res)
            ),
            Ty::Array(t) => format!("[{}; _]", t.render_inner(res)),
            Ty::Slice(t) => format!("[{}]", t.render_inner(res)),
            Ty::FnPtr(ps, r) => {
                let a: Vec<String> = ps.iter().map(|t| t.render_inner(res)).collect();
                let ret = if r.is_unit() {
                    String::new()
                } else {
                    format!(" -> {}", r.render_inner(res))
                };
                format!("fn({}){ret}", a.join(", "))
            }
            Ty::Param(i) => format!("<T{i}>"),
            Ty::Infer(_) => "_".to_owned(),
            Ty::Err => "{unknown}".to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Copy / needs-drop 判定(RXS-0053 / RXS-0055)
// ---------------------------------------------------------------------------

/// Copy 判定(RXS-0053):标量/共享引用/裸指针/fn 指针内建 Copy;
/// 元组/数组逐组件;ADT 看 `#[derive(Copy)]` 标注(合法性已由定义处
/// 检查裁决,RX2008);`Err` 容忍为 Copy(RXS-0047 不级联——不产生 move 诊断)。
pub fn is_copy(krate: &crate::hir::Crate, ty: &Ty) -> bool {
    match ty {
        Ty::Prim(_) => true,
        Ty::Ref(_, mutable) => !*mutable,
        Ty::RawPtr(..) | Ty::FnPtr(..) => true,
        Ty::Tuple(v) => v.iter().all(|t| is_copy(krate, t)),
        Ty::Array(t) => is_copy(krate, t),
        Ty::Slice(_) => false,
        Ty::Adt(d, _) => krate.has_copy_derive(*d),
        // 单态化后不应残留;保守按非 Copy(move 语义)
        Ty::Param(_) | Ty::Infer(_) => false,
        Ty::Err => true,
    }
}

/// needs-drop 判定(RXS-0055,传递):自身携带 Drop impl,或聚合存在
/// needs-drop 组件;`Err` 容忍为不 needs-drop。
pub fn needs_drop(krate: &crate::hir::Crate, ty: &Ty) -> bool {
    needs_drop_inner(krate, ty, &mut Vec::new())
}

fn needs_drop_inner(krate: &crate::hir::Crate, ty: &Ty, seen: &mut Vec<DefId>) -> bool {
    match ty {
        Ty::Adt(d, args) => {
            if krate.drop_impl_of(*d).is_some() {
                return true;
            }
            // 递归 ADT 防环(按值递归本身非法,容忍为不 drop)
            if seen.contains(d) {
                return false;
            }
            seen.push(*d);
            let out = adt_component_tys(krate, *d, args)
                .iter()
                .any(|t| needs_drop_inner(krate, t, seen));
            seen.pop();
            out
        }
        Ty::Tuple(v) => v.iter().any(|t| needs_drop_inner(krate, t, seen)),
        Ty::Array(t) => needs_drop_inner(krate, t, seen),
        _ => false,
    }
}

/// ADT 组件类型展开(struct/变体 = 字段;enum = 全变体字段并集;实参已代入)。
pub fn adt_component_tys(krate: &crate::hir::Crate, def: DefId, args: &[Ty]) -> Vec<Ty> {
    match &krate.item(def).kind {
        crate::hir::ItemKind::Struct { .. } | crate::hir::ItemKind::Variant { .. } => {
            crate::typeck::adt_field_tys(krate, def, args)
        }
        crate::hir::ItemKind::Enum { variants } => variants
            .iter()
            .flat_map(|v| crate::typeck::adt_field_tys(krate, *v, args))
            .collect(),
        _ => Vec::new(),
    }
}

fn prim_name(p: PrimTy) -> &'static str {
    match p {
        PrimTy::I8 => "i8",
        PrimTy::I16 => "i16",
        PrimTy::I32 => "i32",
        PrimTy::I64 => "i64",
        PrimTy::U8 => "u8",
        PrimTy::U16 => "u16",
        PrimTy::U32 => "u32",
        PrimTy::U64 => "u64",
        PrimTy::Usize => "usize",
        PrimTy::F32 => "f32",
        PrimTy::F64 => "f64",
        PrimTy::Bool => "bool",
        PrimTy::Char => "char",
        PrimTy::Str => "str",
    }
}

/// 函数签名(`fn_sig` query 产物,RXS-0040/0042)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FnSig {
    /// 泛型参数个数(`Ty::Param` 的实例化槽位数,RXS-0045)。
    pub generics_count: u32,
    /// 是否携带 `self` 接收者(显式实参核对不计入,RXS-0042)。
    pub has_self: bool,
    /// 显式形参类型。
    pub inputs: Vec<Ty>,
    pub output: Ty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subst_replaces_params() {
        let t = Ty::Ref(Box::new(Ty::Param(0)), false);
        assert_eq!(
            t.subst(&[Ty::Prim(PrimTy::I32)]),
            Ty::Ref(Box::new(Ty::Prim(PrimTy::I32)), false)
        );
        assert_eq!(Ty::Param(3).subst(&[Ty::Prim(PrimTy::Bool)]), Ty::Err);
    }

    #[test]
    fn numeric_classification() {
        assert!(Ty::Prim(PrimTy::Usize).is_int());
        assert!(Ty::Prim(PrimTy::F32).is_float());
        assert!(!Ty::Prim(PrimTy::Bool).is_numeric());
        assert!(Ty::unit().is_unit());
    }
}
