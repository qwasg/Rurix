//! const 求值 MIR 解释器(spec/consteval.md RXS-0062 ~ RXS-0065;D-111 标量优先)。
//!
//! 输入 = const item / const fn 实例的 MIR body([`crate::mir_build::build_for_const_eval`]
//! 构建);在 MIR 上做确定性求值,产出 [`ConstVal`](整数 / bool / 浮点 / char / str
//! 标量,M3.4 子集)。失败经 [`ConstError`] 映射 5xxx(RX5001 溢出 / RX5002 越界 /
//! RX5003 非 const)。
//!
//! 范围裁决(标量优先,M3.4):聚合 / 数组值 / 引用 / 索引 / 投影等非标量构造在
//! const 求值中报 RX5003(运行期数组 codegen 随 M4+,登记已知限制,07 §4);
//! 数组长度本身是标量 usize 表达式,经本解释器求值(RXS-0064)。
//!
//! 终止性:步数预算 [`STEP_BUDGET`] + 递归深度预算 [`DEPTH_BUDGET`];超限报 RX5003
//! (不可终止 const 求值,RXS-0063)。const item 间的环引用经 [`crate::query::QueryCtx`]
//! 的 in-progress 集检出(RXS-0063)。

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{BinOp, FnColor, UnOp};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{self, DefId, PrimTy};
use crate::mir::{
    BlockIdx, Body, CallTarget, Const, Operand, Place, Rvalue, StatementKind, TerminatorKind,
};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::ty::Ty;

pub const E_CONST_OVERFLOW: ErrorCode = ErrorCode(5001); // RX5001
pub const E_CONST_INDEX_OOB: ErrorCode = ErrorCode(5002); // RX5002
pub const E_CONST_NON_CONST: ErrorCode = ErrorCode(5003); // RX5003

/// 单次 const 求值的语句/终结子步数上限(RXS-0063 不可终止保守上界)。
const STEP_BUDGET: u64 = 4_000_000;
/// const fn 调用递归深度上限。
const DEPTH_BUDGET: u32 = 256;

/// const 求值产出的标量值(M3.4 子集;形态对齐 [`crate::mir::Const`])。
#[derive(Clone, Debug, PartialEq)]
pub enum ConstVal {
    Int(i128, PrimTy),
    Float(f64, PrimTy),
    Bool(bool),
    Str(String),
    Char(char),
    Unit,
}

impl ConstVal {
    /// 落 MIR 常量(运行期使用点经 [`Operand::Const`] 内联)。
    pub fn to_mir_const(&self) -> Const {
        match self {
            ConstVal::Int(v, p) => Const::Int(*v, *p),
            ConstVal::Float(v, p) => Const::Float(*v, *p),
            ConstVal::Bool(b) => Const::Bool(*b),
            ConstVal::Str(s) => Const::Str(s.clone()),
            ConstVal::Char(c) => Const::Char(*c),
            ConstVal::Unit => Const::Unit,
        }
    }

    fn truthy(&self) -> bool {
        match self {
            ConstVal::Bool(b) => *b,
            ConstVal::Int(v, _) => *v != 0,
            _ => false,
        }
    }

    /// 非负 usize 取值(数组长度 / const 泛型实参消费,RXS-0064)。
    pub fn as_usize(&self) -> Option<u64> {
        match self {
            ConstVal::Int(v, _) if *v >= 0 => u64::try_from(*v).ok(),
            _ => None,
        }
    }
}

/// const 求值失败(映射 5xxx;span 指向触发失败的源构件,RXS-0065)。
#[derive(Clone, Debug)]
pub enum ConstError {
    Overflow { span: Span, op: String },
    IndexOob { span: Span, index: i128, len: i128 },
    NonConst { span: Span, what: String },
}

impl ConstError {
    pub fn emit(&self, diag: &DiagCtxt) {
        match self {
            ConstError::Overflow { span, op } => {
                diag.struct_error(E_CONST_OVERFLOW, "consteval.overflow")
                    .arg("op", op.clone())
                    .span_label(*span, "this operation overflows during constant evaluation")
                    .emit();
            }
            ConstError::IndexOob { span, index, len } => {
                diag.struct_error(E_CONST_INDEX_OOB, "consteval.index_out_of_bounds")
                    .arg("index", index.to_string())
                    .arg("len", len.to_string())
                    .span_label(*span, "constant index out of bounds")
                    .emit();
            }
            ConstError::NonConst { span, what } => {
                diag.struct_error(E_CONST_NON_CONST, "consteval.non_const_operation")
                    .arg("what", what.clone())
                    .span_label(*span, "not allowed in a constant context")
                    .emit();
            }
        }
    }
}

/// const item 求值入口(查询记忆化在 [`QueryCtx::eval_const`];环检测在该层)。
pub fn eval_const_item(cx: &QueryCtx<'_>, def: DefId) -> Result<ConstVal, ConstError> {
    let body = crate::mir_build::build_for_const_eval(cx, def, Vec::new())?;
    let mut ev = Evaluator::new(cx);
    ev.eval_body(&body, &[])
}

struct Evaluator<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    steps: u64,
    depth: u32,
    /// 非泛型 const fn 实例 MIR 缓存(同一实例避免重复构建)。
    bodies: HashMap<DefId, Rc<Body>>,
}

impl<'a, 'q> Evaluator<'a, 'q> {
    fn new(cx: &'a QueryCtx<'q>) -> Self {
        Evaluator {
            cx,
            steps: 0,
            depth: 0,
            bodies: HashMap::new(),
        }
    }

    fn tick(&mut self, span: Span) -> Result<(), ConstError> {
        self.steps += 1;
        if self.steps > STEP_BUDGET {
            return Err(ConstError::NonConst {
                span,
                what: "non-terminating constant evaluation".to_owned(),
            });
        }
        Ok(())
    }

    fn eval_body(&mut self, body: &Body, args: &[ConstVal]) -> Result<ConstVal, ConstError> {
        let mut locals: Vec<Option<ConstVal>> = vec![None; body.locals.len()];
        for (i, a) in args.iter().enumerate() {
            if i + 1 < locals.len() {
                locals[i + 1] = Some(a.clone());
            }
        }
        let mut block = BlockIdx(0);
        loop {
            let bb = &body.blocks[block.0 as usize];
            for stmt in &bb.stmts {
                self.tick(stmt.span)?;
                let StatementKind::Assign(place, rv) = &stmt.kind;
                let v = self.eval_rvalue(rv, &locals, stmt.span)?;
                write_place(place, v, &mut locals, stmt.span)?;
            }
            let term = &bb.terminator;
            self.tick(term.span)?;
            match &term.kind {
                TerminatorKind::Goto(b) => block = *b,
                TerminatorKind::SwitchBool { discr, then, else_ } => {
                    let d = self.eval_operand(discr, &locals, term.span)?;
                    block = if d.truthy() { *then } else { *else_ };
                }
                TerminatorKind::Call {
                    target,
                    args,
                    dest,
                    next,
                } => {
                    let mut argvals = Vec::with_capacity(args.len());
                    for o in args {
                        argvals.push(self.eval_operand(o, &locals, term.span)?);
                    }
                    let r = self.eval_call(target, &argvals, term.span)?;
                    write_place(dest, r, &mut locals, term.span)?;
                    block = *next;
                }
                TerminatorKind::Return => {
                    return Ok(locals.first().cloned().flatten().unwrap_or(ConstVal::Unit));
                }
                // Drop 在标量 const 求值中应不出现(无 needs-drop);防御性透传
                TerminatorKind::Drop { next, .. } => block = *next,
                TerminatorKind::Unreachable => {
                    return Err(ConstError::NonConst {
                        span: term.span,
                        what: "evaluation reached unreachable code".to_owned(),
                    });
                }
            }
        }
    }

    fn eval_call(
        &mut self,
        target: &CallTarget,
        args: &[ConstVal],
        span: Span,
    ) -> Result<ConstVal, ConstError> {
        let def = match target {
            CallTarget::Fn { def, .. } => *def,
            CallTarget::Builtin(_) => {
                return Err(ConstError::NonConst {
                    span,
                    what: "call to a builtin".to_owned(),
                });
            }
        };
        let krate = self.cx.hir_crate();
        let item = krate.item(def);
        let hir::ItemKind::Fn(decl) = &item.kind else {
            return Err(ConstError::NonConst {
                span,
                what: "call to a non-function".to_owned(),
            });
        };
        if decl.color != FnColor::Const {
            return Err(ConstError::NonConst {
                span,
                what: format!("call to non-`const fn` `{}`", item.name),
            });
        }
        if !decl.generic_params.is_empty() {
            return Err(ConstError::NonConst {
                span,
                what: "call to a generic `const fn`".to_owned(),
            });
        }
        if decl.body.is_none() {
            return Err(ConstError::NonConst {
                span,
                what: "call to a body-less `const fn`".to_owned(),
            });
        }
        self.depth += 1;
        if self.depth > DEPTH_BUDGET {
            self.depth -= 1;
            return Err(ConstError::NonConst {
                span,
                what: "constant evaluation recursion too deep".to_owned(),
            });
        }
        let body = self.body_of(def)?;
        let r = self.eval_body(&body, args);
        self.depth -= 1;
        r
    }

    fn body_of(&mut self, def: DefId) -> Result<Rc<Body>, ConstError> {
        if let Some(b) = self.bodies.get(&def) {
            return Ok(Rc::clone(b));
        }
        let b = Rc::new(crate::mir_build::build_for_const_eval(
            self.cx,
            def,
            Vec::new(),
        )?);
        self.bodies.insert(def, Rc::clone(&b));
        Ok(b)
    }

    fn eval_operand(
        &self,
        op: &Operand,
        locals: &[Option<ConstVal>],
        span: Span,
    ) -> Result<ConstVal, ConstError> {
        match op {
            Operand::Const(c) => Ok(mir_const_to_val(c)),
            Operand::Copy(p) | Operand::Move(p) => read_place(locals, p, span),
        }
    }

    fn eval_rvalue(
        &self,
        rv: &Rvalue,
        locals: &[Option<ConstVal>],
        span: Span,
    ) -> Result<ConstVal, ConstError> {
        match rv {
            Rvalue::Use(o) => self.eval_operand(o, locals, span),
            Rvalue::BinaryOp(op, a, b) => {
                let av = self.eval_operand(a, locals, span)?;
                let bv = self.eval_operand(b, locals, span)?;
                eval_binop(*op, av, bv, span)
            }
            Rvalue::UnaryOp(op, a) => {
                let av = self.eval_operand(a, locals, span)?;
                eval_unop(*op, av, span)
            }
            Rvalue::Cast(o, ty) => {
                let v = self.eval_operand(o, locals, span)?;
                eval_cast(v, ty, span)
            }
            Rvalue::Ref(..) => Err(ConstError::NonConst {
                span,
                what: "reference".to_owned(),
            }),
            Rvalue::Aggregate(..) | Rvalue::VariantAggregate { .. } => Err(ConstError::NonConst {
                span,
                what: "aggregate construction".to_owned(),
            }),
            Rvalue::Discriminant(_) => Err(ConstError::NonConst {
                span,
                what: "discriminant read".to_owned(),
            }),
        }
    }
}

fn read_place(
    locals: &[Option<ConstVal>],
    place: &Place,
    span: Span,
) -> Result<ConstVal, ConstError> {
    if !place.proj.is_empty() {
        return Err(ConstError::NonConst {
            span,
            what: "place projection".to_owned(),
        });
    }
    locals
        .get(place.local.0 as usize)
        .cloned()
        .flatten()
        .ok_or_else(|| ConstError::NonConst {
            span,
            what: "use of an uninitialized value".to_owned(),
        })
}

fn write_place(
    place: &Place,
    v: ConstVal,
    locals: &mut [Option<ConstVal>],
    span: Span,
) -> Result<(), ConstError> {
    if !place.proj.is_empty() {
        return Err(ConstError::NonConst {
            span,
            what: "place projection assignment".to_owned(),
        });
    }
    locals[place.local.0 as usize] = Some(v);
    Ok(())
}

fn mir_const_to_val(c: &Const) -> ConstVal {
    match c {
        Const::Int(v, p) => ConstVal::Int(*v, *p),
        Const::Float(v, p) => ConstVal::Float(*v, *p),
        Const::Bool(b) => ConstVal::Bool(*b),
        Const::Str(s) => ConstVal::Str(s.clone()),
        Const::Char(c) => ConstVal::Char(*c),
        Const::Unit => ConstVal::Unit,
    }
}

// ---------------------------------------------------------------------------
// 算术(整数溢出经类型范围裁决,RXS-0063;浮点不做溢出门)
// ---------------------------------------------------------------------------

fn eval_binop(op: BinOp, a: ConstVal, b: ConstVal, span: Span) -> Result<ConstVal, ConstError> {
    match (a, b) {
        (ConstVal::Int(x, ty), ConstVal::Int(y, _)) => int_binop(op, x, y, ty, span),
        (ConstVal::Float(x, ty), ConstVal::Float(y, _)) => float_binop(op, x, y, ty, span),
        (ConstVal::Bool(x), ConstVal::Bool(y)) => bool_binop(op, x, y, span),
        _ => Err(ConstError::NonConst {
            span,
            what: "binary operation on non-scalar constants".to_owned(),
        }),
    }
}

fn int_binop(op: BinOp, x: i128, y: i128, ty: PrimTy, span: Span) -> Result<ConstVal, ConstError> {
    let ov = |name: &str| ConstError::Overflow {
        span,
        op: name.to_owned(),
    };
    let checked = |r: Option<i128>, name: &str| -> Result<ConstVal, ConstError> {
        let v = r.ok_or_else(|| ov(name))?;
        if int_fits(ty, v) {
            Ok(ConstVal::Int(v, ty))
        } else {
            Err(ov(name))
        }
    };
    match op {
        BinOp::Add => checked(x.checked_add(y), "add"),
        BinOp::Sub => checked(x.checked_sub(y), "subtract"),
        BinOp::Mul => checked(x.checked_mul(y), "multiply"),
        BinOp::Div => checked(x.checked_div(y), "divide"),
        BinOp::Rem => checked(x.checked_rem(y), "remainder"),
        BinOp::BitAnd => Ok(ConstVal::Int(x & y, ty)),
        BinOp::BitOr => Ok(ConstVal::Int(x | y, ty)),
        BinOp::BitXor => Ok(ConstVal::Int(x ^ y, ty)),
        BinOp::Shl => {
            let s = u32::try_from(y).ok().filter(|s| *s < 128);
            checked(s.and_then(|s| x.checked_shl(s)), "shift-left")
        }
        BinOp::Shr => {
            let s = u32::try_from(y).ok().filter(|s| *s < 128);
            checked(s.and_then(|s| x.checked_shr(s)), "shift-right")
        }
        BinOp::Eq => Ok(ConstVal::Bool(x == y)),
        BinOp::Ne => Ok(ConstVal::Bool(x != y)),
        BinOp::Lt => Ok(ConstVal::Bool(x < y)),
        BinOp::Gt => Ok(ConstVal::Bool(x > y)),
        BinOp::Le => Ok(ConstVal::Bool(x <= y)),
        BinOp::Ge => Ok(ConstVal::Bool(x >= y)),
        BinOp::And | BinOp::Or => Err(ConstError::NonConst {
            span,
            what: "lazy boolean operator on integers".to_owned(),
        }),
    }
}

fn float_binop(op: BinOp, x: f64, y: f64, ty: PrimTy, span: Span) -> Result<ConstVal, ConstError> {
    Ok(match op {
        BinOp::Add => ConstVal::Float(x + y, ty),
        BinOp::Sub => ConstVal::Float(x - y, ty),
        BinOp::Mul => ConstVal::Float(x * y, ty),
        BinOp::Div => ConstVal::Float(x / y, ty),
        BinOp::Rem => ConstVal::Float(x % y, ty),
        BinOp::Eq => ConstVal::Bool(x == y),
        BinOp::Ne => ConstVal::Bool(x != y),
        BinOp::Lt => ConstVal::Bool(x < y),
        BinOp::Gt => ConstVal::Bool(x > y),
        BinOp::Le => ConstVal::Bool(x <= y),
        BinOp::Ge => ConstVal::Bool(x >= y),
        _ => {
            return Err(ConstError::NonConst {
                span,
                what: "this operator on floating-point constants".to_owned(),
            });
        }
    })
}

fn bool_binop(op: BinOp, x: bool, y: bool, span: Span) -> Result<ConstVal, ConstError> {
    Ok(match op {
        BinOp::Eq => ConstVal::Bool(x == y),
        BinOp::Ne => ConstVal::Bool(x != y),
        BinOp::BitAnd => ConstVal::Bool(x & y),
        BinOp::BitOr => ConstVal::Bool(x | y),
        BinOp::BitXor => ConstVal::Bool(x ^ y),
        _ => {
            return Err(ConstError::NonConst {
                span,
                what: "this operator on boolean constants".to_owned(),
            });
        }
    })
}

fn eval_unop(op: UnOp, v: ConstVal, span: Span) -> Result<ConstVal, ConstError> {
    match (op, v) {
        (UnOp::Neg, ConstVal::Int(x, ty)) => {
            let r = x.checked_neg().filter(|r| int_fits(ty, *r));
            r.map(|r| ConstVal::Int(r, ty)).ok_or(ConstError::Overflow {
                span,
                op: "negate".to_owned(),
            })
        }
        (UnOp::Neg, ConstVal::Float(x, ty)) => Ok(ConstVal::Float(-x, ty)),
        (UnOp::Not, ConstVal::Bool(b)) => Ok(ConstVal::Bool(!b)),
        _ => Err(ConstError::NonConst {
            span,
            what: "this unary operation".to_owned(),
        }),
    }
}

fn eval_cast(v: ConstVal, ty: &Ty, span: Span) -> Result<ConstVal, ConstError> {
    let Ty::Prim(target) = ty else {
        return Err(ConstError::NonConst {
            span,
            what: "cast to a non-primitive type".to_owned(),
        });
    };
    let non_const = || ConstError::NonConst {
        span,
        what: "this cast".to_owned(),
    };
    match v {
        ConstVal::Int(x, _) => {
            if is_int_prim(*target) {
                Ok(ConstVal::Int(wrap_int(x, *target), *target))
            } else if matches!(target, PrimTy::F32 | PrimTy::F64) {
                Ok(ConstVal::Float(x as f64, *target))
            } else if *target == PrimTy::Bool {
                Ok(ConstVal::Bool(x != 0))
            } else {
                Err(non_const())
            }
        }
        ConstVal::Float(x, _) => {
            if matches!(target, PrimTy::F32 | PrimTy::F64) {
                Ok(ConstVal::Float(x, *target))
            } else if is_int_prim(*target) {
                Ok(ConstVal::Int(wrap_int(x as i128, *target), *target))
            } else {
                Err(non_const())
            }
        }
        ConstVal::Bool(b) => {
            if is_int_prim(*target) {
                Ok(ConstVal::Int(b as i128, *target))
            } else {
                Err(non_const())
            }
        }
        _ => Err(non_const()),
    }
}

// ---------------------------------------------------------------------------
// 整数类型范围(二进制补码;usize = 64 位目标,D-209)
// ---------------------------------------------------------------------------

fn is_int_prim(p: PrimTy) -> bool {
    matches!(
        p,
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
}

fn int_range(p: PrimTy) -> Option<(i128, i128)> {
    Some(match p {
        PrimTy::I8 => (i8::MIN as i128, i8::MAX as i128),
        PrimTy::I16 => (i16::MIN as i128, i16::MAX as i128),
        PrimTy::I32 => (i32::MIN as i128, i32::MAX as i128),
        PrimTy::I64 => (i64::MIN as i128, i64::MAX as i128),
        PrimTy::U8 => (0, u8::MAX as i128),
        PrimTy::U16 => (0, u16::MAX as i128),
        PrimTy::U32 => (0, u32::MAX as i128),
        PrimTy::U64 | PrimTy::Usize => (0, u64::MAX as i128),
        _ => return None,
    })
}

fn int_fits(p: PrimTy, v: i128) -> bool {
    match int_range(p) {
        Some((lo, hi)) => v >= lo && v <= hi,
        None => true,
    }
}

/// 截断到目标整数类型(cast 语义:二进制补码 wrap)。
fn wrap_int(v: i128, p: PrimTy) -> i128 {
    let bits = match p {
        PrimTy::I8 | PrimTy::U8 => 8u32,
        PrimTy::I16 | PrimTy::U16 => 16,
        PrimTy::I32 | PrimTy::U32 => 32,
        PrimTy::I64 | PrimTy::U64 | PrimTy::Usize => 64,
        _ => return v,
    };
    let mask: i128 = if bits == 128 { -1 } else { (1i128 << bits) - 1 };
    let raw = v & mask;
    let signed = matches!(p, PrimTy::I8 | PrimTy::I16 | PrimTy::I32 | PrimTy::I64);
    if signed && bits < 128 && (raw & (1i128 << (bits - 1))) != 0 {
        raw - (1i128 << bits) // 符号扩展
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 求值首个名为 `name` 的 const item;返回 (值结果, 诊断码集)。
    fn eval_named(src: &str, name: &str) -> (Result<ConstVal, ConstError>, Vec<u16>) {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        let res = cx.resolutions();
        let def = res
            .defs
            .iter()
            .position(|d| d.name == name)
            .map(|i| DefId(i as u32))
            .expect("const 未找到");
        let r = cx.eval_const(def);
        let codes = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        (r, codes)
    }

    //@ spec: RXS-0063
    #[test]
    fn const_arithmetic_evaluates() {
        let (r, _) = eval_named("const A: i32 = 6 * 7;", "A");
        assert_eq!(r.unwrap(), ConstVal::Int(42, PrimTy::I32));
    }

    //@ spec: RXS-0062
    #[test]
    fn const_fn_call_evaluates() {
        let (r, _) = eval_named(
            "const fn square(n: i32) -> i32 { n * n }\nconst A: i32 = square(5);",
            "A",
        );
        assert_eq!(r.unwrap(), ConstVal::Int(25, PrimTy::I32));
    }

    //@ spec: RXS-0063
    #[test]
    fn const_branch_and_ref_chain() {
        let (r, _) = eval_named(
            "const fn pick(n: i32) -> i32 { if n > 8 { 8 } else { n } }\n\
             const SIDE: i32 = 4;\nconst A: i32 = pick(SIDE) + pick(20);",
            "A",
        );
        assert_eq!(r.unwrap(), ConstVal::Int(12, PrimTy::I32));
    }

    //@ spec: RXS-0063
    #[test]
    fn const_loop_evaluates() {
        let (r, _) = eval_named(
            "const fn sum_to(n: i32) -> i32 {\n    let mut acc = 0;\n    let mut i = 0;\n    while i < n {\n        acc = acc + i;\n        i = i + 1;\n    }\n    acc\n}\nconst A: i32 = sum_to(5);",
            "A",
        );
        assert_eq!(r.unwrap(), ConstVal::Int(10, PrimTy::I32));
    }

    //@ spec: RXS-0065
    #[test]
    fn const_overflow_is_rx5001() {
        let (r, _) = eval_named("const A: u8 = 200 + 100;", "A");
        assert!(matches!(r, Err(ConstError::Overflow { .. })));
    }

    //@ spec: RXS-0065
    #[test]
    fn non_const_fn_call_is_rx5003() {
        let (r, _) = eval_named(
            "fn runtime(n: i32) -> i32 { n }\nconst A: i32 = runtime(3);",
            "A",
        );
        assert!(matches!(r, Err(ConstError::NonConst { .. })));
    }

    //@ spec: RXS-0063
    #[test]
    fn const_cycle_is_rx5003() {
        let (r, _) = eval_named("const A: i32 = B;\nconst B: i32 = A;", "A");
        assert!(matches!(r, Err(ConstError::NonConst { .. })));
    }

    /// RXS-0064 实参求值子集:const 泛型实参类的 `usize` 常量表达式经 RXS-0063
    /// 求值为非负 usize 值(可作数组长度 / const 参数实参;值的运行期单态化
    /// 接入随 M4+,RD-007)。
    //@ spec: RXS-0064
    #[test]
    fn const_generic_arg_usize_evaluates() {
        let (r, _) = eval_named("const N: usize = 4 + 4;", "N");
        let v = r.expect("usize const 求值");
        assert_eq!(v, ConstVal::Int(8, PrimTy::Usize));
        assert_eq!(v.as_usize(), Some(8));
    }
}
