//! MIR → 文本 LLVM IR(M2.3 host codegen;07 §8 / D-209)。
//!
//! 绑定通道 = **文本 IR + 外部 clang**(M2_PLAN v1.3 选型留痕):本模块产出
//! `.ll`,驱动调 pin 的 clang 22.1.x(D-205)编为 x86-64 COFF `.obj`,
//! link.exe 链接(Microsoft x64 ABI)。
//!
//! debug info:CodeView module flag + DISubprogram/DILocation 行号
//! (D-209/D-237:PDB 断点经 clang `-g` 透传)。
//!
//! M2.3 codegen 约定:
//! - 全 local 进 alloca(mem2reg 交给 LLVM;不做自有优化);
//! - `bool` 以 i8 落内存,分支处 `icmp ne 0` 还原 i1;
//! - 字符串字面量 = NUL 终止全局常量,`&str` 即 `ptr`(len 携带随 M3 库化);
//! - `()` 与空元组零尺寸:不开 alloca、实参/形参位跳过;
//! - `main` 以 C ABI 暴露(CRT 启动调用),unit 返回 → `ret i32 0`。

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::{BinOp, UnOp};
use crate::hir::{self, Builtin, PrimTy};
use crate::mir::{
    Body, CallTarget, Const, Operand, Place, ProjElem, Rvalue, StatementKind, TerminatorKind,
};
use crate::source_map::SourceMap;
use crate::span::Span;
use crate::ty::Ty;

pub struct CodegenOpts<'a> {
    /// 模块名(诊断性;`; ModuleID`)。
    pub module_name: &'a str,
    /// DIFile filename(源文件名,cdb 源行断点锚点)。
    pub file_name: &'a str,
    /// DIFile directory(绝对路径)。
    pub directory: &'a str,
}

/// codegen 入口:全部单态化实例 → 单一 LLVM IR 模块文本。
pub fn emit_llvm_ir(
    bodies: &[Body],
    krate: &hir::Crate,
    sm: &SourceMap,
    opts: &CodegenOpts<'_>,
) -> String {
    let mut cg = Cg {
        krate,
        sm,
        fns: String::new(),
        globals: String::new(),
        strs: HashMap::new(),
        md: Vec::new(),
        locs: HashMap::new(),
        tmp: 0,
        cur_dbg: None,
        cur_sp: 0,
        need_puts: false,
        externs: HashMap::new(),
    };
    // 元数据骨架:!0 CU / !1 file / !2 !3 module flags / !4 !5 subroutine type
    cg.md.push(
        "distinct !DICompileUnit(language: DW_LANG_C99, file: !1, producer: \"rurixc\", \
         isOptimized: false, runtimeVersion: 0, emissionKind: FullDebug)"
            .to_owned(),
    );
    cg.md.push(format!(
        "!DIFile(filename: \"{}\", directory: \"{}\")",
        escape_md(opts.file_name),
        escape_md(opts.directory)
    ));
    cg.md.push("!{i32 2, !\"CodeView\", i32 1}".to_owned());
    cg.md
        .push("!{i32 2, !\"Debug Info Version\", i32 3}".to_owned());
    cg.md.push("!DISubroutineType(types: !5)".to_owned());
    cg.md.push("!{null}".to_owned());

    for b in bodies {
        cg.emit_body(b);
    }

    let mut out = String::new();
    let _ = writeln!(out, "; ModuleID = '{}'", opts.module_name);
    let _ = writeln!(out, "source_filename = \"{}\"", escape_md(opts.file_name));
    let _ = writeln!(out, "target triple = \"x86_64-pc-windows-msvc\"");
    out.push('\n');
    out.push_str(&cg.globals);
    if cg.need_puts {
        out.push_str("declare i32 @puts(ptr)\n");
    }
    for (sym, decl) in {
        let mut v: Vec<_> = cg.externs.iter().collect();
        v.sort();
        v
    } {
        let _ = writeln!(out, "declare {decl} ; extern {sym}");
    }
    out.push('\n');
    out.push_str(&cg.fns);
    out.push('\n');
    out.push_str("!llvm.dbg.cu = !{!0}\n!llvm.module.flags = !{!2, !3}\n");
    for (i, m) in cg.md.iter().enumerate() {
        let _ = writeln!(out, "!{i} = {m}");
    }
    out
}

struct Cg<'a> {
    krate: &'a hir::Crate,
    sm: &'a SourceMap,
    fns: String,
    globals: String,
    /// 字符串字面量 → 全局序号(去重)。
    strs: HashMap<String, usize>,
    /// 编号元数据节点(序号即 `!N`)。
    md: Vec<String>,
    /// (行, 列, 函数 SP) → DILocation 序号(去重)。
    locs: HashMap<(u32, u32, usize), usize>,
    tmp: u32,
    /// 当前语句的 `!dbg` 尾缀。
    cur_dbg: Option<usize>,
    /// 当前函数 DISubprogram 序号。
    cur_sp: usize,
    need_puts: bool,
    /// 无 body 的外部 fn:符号 → declare 文本。
    externs: HashMap<String, String>,
}

impl Cg<'_> {
    // -- 类型映射 ---------------------------------------------------------------

    fn llty(&self, t: &Ty) -> String {
        match t {
            Ty::Prim(p) => prim_llty(*p).to_owned(),
            Ty::Ref(..) | Ty::RawPtr(..) | Ty::FnPtr(..) => "ptr".to_owned(),
            Ty::Tuple(v) => {
                let parts: Vec<String> = v.iter().map(|x| self.llty(x)).collect();
                format!("{{ {} }}", parts.join(", "))
            }
            Ty::Adt(d, args) => {
                let fields = crate::typeck::adt_field_tys(self.krate, *d, args);
                let parts: Vec<String> = fields.iter().map(|x| self.llty(x)).collect();
                format!("{{ {} }}", parts.join(", "))
            }
            // 作用面外形态(RX6001 已拦截;防御性兜底)
            Ty::Array(_) | Ty::Slice(_) | Ty::Param(_) | Ty::Infer(_) | Ty::Err => "i8".to_owned(),
        }
    }

    fn is_zst(&self, t: &Ty) -> bool {
        matches!(t, Ty::Tuple(v) if v.is_empty())
    }

    // -- 工具 -------------------------------------------------------------------

    fn fresh(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }

    fn dbg_suffix(&self) -> String {
        match self.cur_dbg {
            Some(n) => format!(", !dbg !{n}"),
            None => String::new(),
        }
    }

    fn set_dbg(&mut self, span: Span) {
        let lc = self.sm.lookup(span.file, span.lo);
        let key = (lc.line, lc.col, self.cur_sp);
        let next = self.md.len();
        let idx = *self.locs.entry(key).or_insert_with(|| {
            next // 占位;真正 push 在下方(entry 闭包内不可变借 self.md)
        });
        if idx == next {
            self.md.push(format!(
                "!DILocation(line: {}, column: {}, scope: !{})",
                lc.line, lc.col, self.cur_sp
            ));
        }
        self.cur_dbg = Some(idx);
    }

    fn str_global(&mut self, s: &str) -> String {
        let next = self.strs.len();
        let id = *self.strs.entry(s.to_owned()).or_insert(next);
        if id == next {
            let bytes = s.as_bytes();
            let mut enc = String::new();
            for b in bytes {
                match b {
                    0x20..=0x7e if *b != b'"' && *b != b'\\' => enc.push(*b as char),
                    _ => enc.push_str(&format!("\\{b:02X}")),
                }
            }
            enc.push_str("\\00");
            let _ = writeln!(
                self.globals,
                "@.str.{id} = private unnamed_addr constant [{} x i8] c\"{enc}\"",
                bytes.len() + 1
            );
        }
        format!("@.str.{id}")
    }

    // -- 函数 ---------------------------------------------------------------------

    fn emit_body(&mut self, b: &Body) {
        self.tmp = 0;
        let is_main = b.symbol == "main";
        let ret_ty = b.ret_ty().clone();
        let ret_zst = self.is_zst(&ret_ty);
        let ll_ret = if is_main {
            "i32".to_owned()
        } else if ret_zst {
            "void".to_owned()
        } else {
            self.llty(&ret_ty)
        };

        // DISubprogram
        let line = self.sm.lookup(b.span.file, b.span.lo).line;
        let sp = self.md.len();
        self.md.push(format!(
            "distinct !DISubprogram(name: \"{0}\", linkageName: \"{0}\", scope: !1, file: !1, \
             line: {line}, type: !4, scopeLine: {line}, flags: DIFlagPrototyped, \
             spFlags: DISPFlagDefinition, unit: !0)",
            escape_md(&b.symbol)
        ));
        self.cur_sp = sp;
        self.cur_dbg = None;

        // 形参(unit 形参跳过;与调用点约定一致)
        let mut params = Vec::new();
        for i in 1..=b.arg_count {
            let l = &b.locals[i];
            if self.is_zst(&l.ty) {
                continue;
            }
            params.push(format!("{} %arg{i}", self.llty(&l.ty)));
        }
        let _ = writeln!(
            self.fns,
            "define {ll_ret} @{}({}) !dbg !{sp} {{",
            b.symbol,
            params.join(", ")
        );

        // 入口:allocas + 形参落栈
        self.fns.push_str("start:\n");
        for (i, l) in b.locals.iter().enumerate() {
            if self.is_zst(&l.ty) {
                continue;
            }
            let _ = writeln!(self.fns, "  %l{i} = alloca {}", self.llty(&l.ty));
        }
        for i in 1..=b.arg_count {
            let l = &b.locals[i];
            if self.is_zst(&l.ty) {
                continue;
            }
            let _ = writeln!(self.fns, "  store {} %arg{i}, ptr %l{i}", self.llty(&l.ty));
        }
        self.fns.push_str("  br label %bb0\n");

        for (bi, bb) in b.blocks.iter().enumerate() {
            let _ = writeln!(self.fns, "bb{bi}:");
            for s in &bb.stmts {
                self.set_dbg(s.span);
                match &s.kind {
                    StatementKind::Assign(p, rv) => self.emit_assign(b, p, rv),
                }
            }
            self.set_dbg(bb.terminator.span);
            self.emit_terminator(b, &bb.terminator.kind, is_main, &ret_ty);
        }
        self.fns.push_str("}\n\n");
    }

    // -- place / operand ----------------------------------------------------------

    /// place → (指针值, 指向类型)。
    fn place_ptr(&mut self, b: &Body, p: &Place) -> (String, Ty) {
        let mut ptr = format!("%l{}", p.local.0);
        let mut ty = b.local(p.local).ty.clone();
        for elem in &p.proj {
            match elem {
                ProjElem::Deref => {
                    let t = self.fresh();
                    let _ = writeln!(self.fns, "  {t} = load ptr, ptr {ptr}{}", self.dbg_suffix());
                    ptr = t;
                    ty = match ty {
                        Ty::Ref(inner, _) | Ty::RawPtr(inner, _) => *inner,
                        other => other,
                    };
                }
                ProjElem::Field(i) => {
                    let agg = self.llty(&ty);
                    let t = self.fresh();
                    let _ = writeln!(
                        self.fns,
                        "  {t} = getelementptr inbounds {agg}, ptr {ptr}, i32 0, i32 {i}{}",
                        self.dbg_suffix()
                    );
                    ptr = t;
                    ty = match &ty {
                        Ty::Tuple(v) => v.get(*i as usize).cloned().unwrap_or(Ty::Err),
                        Ty::Adt(d, args) => crate::typeck::adt_field_tys(self.krate, *d, args)
                            .get(*i as usize)
                            .cloned()
                            .unwrap_or(Ty::Err),
                        _ => Ty::Err,
                    };
                }
            }
        }
        (ptr, ty)
    }

    /// operand → (LLVM 类型, 值);unit → None。
    fn operand(&mut self, b: &Body, o: &Operand) -> Option<(String, String, Ty)> {
        match o {
            Operand::Copy(p) => {
                // 零尺寸 place 无 alloca,先以 peek 短路(不发指令)
                let (_, peeked) = self.place_ptr_peek(b, p);
                if self.is_zst(&peeked) {
                    return None;
                }
                let (ptr, ty) = self.place_ptr(b, p);
                let ll = self.llty(&ty);
                let t = self.fresh();
                let _ = writeln!(
                    self.fns,
                    "  {t} = load {ll}, ptr {ptr}{}",
                    self.dbg_suffix()
                );
                Some((ll, t, ty))
            }
            Operand::Const(c) => match c {
                Const::Unit => None,
                Const::Int(v, p) => Some((
                    prim_llty(*p).to_owned(),
                    format!("{}", wrap_signed(*v, prim_width(*p))),
                    Ty::Prim(*p),
                )),
                Const::Float(v, p) => {
                    let bits = match p {
                        PrimTy::F32 => f64::from(*v as f32).to_bits(),
                        _ => v.to_bits(),
                    };
                    Some((
                        prim_llty(*p).to_owned(),
                        format!("0x{bits:016X}"),
                        Ty::Prim(*p),
                    ))
                }
                Const::Bool(v) => Some((
                    "i8".to_owned(),
                    if *v { "1" } else { "0" }.to_owned(),
                    Ty::Prim(PrimTy::Bool),
                )),
                Const::Char(c) => Some((
                    "i32".to_owned(),
                    format!("{}", *c as u32),
                    Ty::Prim(PrimTy::Char),
                )),
                Const::Str(s) => {
                    let g = self.str_global(s);
                    Some((
                        "ptr".to_owned(),
                        g,
                        Ty::Ref(Box::new(Ty::Prim(PrimTy::Str)), false),
                    ))
                }
            },
        }
    }

    // -- 语句 -----------------------------------------------------------------------

    fn emit_assign(&mut self, b: &Body, place: &Place, rv: &Rvalue) {
        match rv {
            Rvalue::Use(o) => {
                let Some((ll, v, _)) = self.operand(b, o) else {
                    return; // unit:零尺寸,无落栈
                };
                let (ptr, _) = self.place_ptr(b, place);
                let _ = writeln!(self.fns, "  store {ll} {v}, ptr {ptr}{}", self.dbg_suffix());
            }
            Rvalue::Ref(src) => {
                let (src_ptr, _) = self.place_ptr(b, src);
                let (ptr, _) = self.place_ptr(b, place);
                let _ = writeln!(
                    self.fns,
                    "  store ptr {src_ptr}, ptr {ptr}{}",
                    self.dbg_suffix()
                );
            }
            Rvalue::BinaryOp(op, a, c) => {
                let Some((lla, va, ta)) = self.operand(b, a) else {
                    return;
                };
                let Some((_, vc, _)) = self.operand(b, c) else {
                    return;
                };
                let signed = ty_signed(&ta);
                let is_float = ta.is_float();
                let t = self.fresh();
                let (result, result_is_i1) = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        let inst = arith_inst(*op, is_float, signed);
                        let _ = writeln!(
                            self.fns,
                            "  {t} = {inst} {lla} {va}, {vc}{}",
                            self.dbg_suffix()
                        );
                        (t.clone(), false)
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        let inst = bit_inst(*op, signed);
                        let _ = writeln!(
                            self.fns,
                            "  {t} = {inst} {lla} {va}, {vc}{}",
                            self.dbg_suffix()
                        );
                        (t.clone(), false)
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        let inst = if is_float {
                            format!("fcmp {}", fcmp_cond(*op))
                        } else {
                            format!("icmp {}", icmp_cond(*op, signed))
                        };
                        let _ = writeln!(
                            self.fns,
                            "  {t} = {inst} {lla} {va}, {vc}{}",
                            self.dbg_suffix()
                        );
                        (t.clone(), true)
                    }
                    // 短路形态在 MIR 已 CFG 化(lower_short_circuit),此处不出现
                    BinOp::And | BinOp::Or => {
                        let _ = writeln!(
                            self.fns,
                            "  {t} = and {lla} {va}, {vc}{}",
                            self.dbg_suffix()
                        );
                        (t.clone(), false)
                    }
                };
                let val = if result_is_i1 {
                    let z = self.fresh();
                    let _ = writeln!(self.fns, "  {z} = zext i1 {result} to i8");
                    z
                } else {
                    result
                };
                let (ptr, pty) = self.place_ptr(b, place);
                let ll = self.llty(&pty);
                let _ = writeln!(
                    self.fns,
                    "  store {ll} {val}, ptr {ptr}{}",
                    self.dbg_suffix()
                );
            }
            Rvalue::UnaryOp(op, a) => {
                let Some((lla, va, ta)) = self.operand(b, a) else {
                    return;
                };
                let t = self.fresh();
                match op {
                    UnOp::Neg if ta.is_float() => {
                        let _ = writeln!(self.fns, "  {t} = fneg {lla} {va}{}", self.dbg_suffix());
                    }
                    UnOp::Neg => {
                        let _ =
                            writeln!(self.fns, "  {t} = sub {lla} 0, {va}{}", self.dbg_suffix());
                    }
                    UnOp::Not if matches!(ta, Ty::Prim(PrimTy::Bool)) => {
                        let _ =
                            writeln!(self.fns, "  {t} = xor {lla} {va}, 1{}", self.dbg_suffix());
                    }
                    UnOp::Not => {
                        let _ =
                            writeln!(self.fns, "  {t} = xor {lla} {va}, -1{}", self.dbg_suffix());
                    }
                    UnOp::Deref => unreachable!("Deref 经 place 投影"),
                }
                let (ptr, _) = self.place_ptr(b, place);
                let _ = writeln!(
                    self.fns,
                    "  store {lla} {t}, ptr {ptr}{}",
                    self.dbg_suffix()
                );
            }
            Rvalue::Cast(o, target) => {
                let Some((lla, va, ta)) = self.operand(b, o) else {
                    return;
                };
                let ll_to = self.llty(target);
                let val = if lla == ll_to {
                    va
                } else {
                    let inst = cast_inst(&ta, target);
                    let t = self.fresh();
                    let _ = writeln!(
                        self.fns,
                        "  {t} = {inst} {lla} {va} to {ll_to}{}",
                        self.dbg_suffix()
                    );
                    t
                };
                let (ptr, _) = self.place_ptr(b, place);
                let _ = writeln!(
                    self.fns,
                    "  store {ll_to} {val}, ptr {ptr}{}",
                    self.dbg_suffix()
                );
            }
            Rvalue::Aggregate(ty, ops) => {
                // 逐字段落位(dest GEP + store;避免 insertvalue 链)
                let agg = self.llty(ty);
                let (base, _) = self.place_ptr(b, place);
                for (i, o) in ops.iter().enumerate() {
                    let Some((ll, v, _)) = self.operand(b, o) else {
                        continue;
                    };
                    let g = self.fresh();
                    let _ = writeln!(
                        self.fns,
                        "  {g} = getelementptr inbounds {agg}, ptr {base}, i32 0, i32 {i}{}",
                        self.dbg_suffix()
                    );
                    let _ = writeln!(self.fns, "  store {ll} {v}, ptr {g}{}", self.dbg_suffix());
                }
            }
        }
    }

    // -- 终结子 -----------------------------------------------------------------------

    fn emit_terminator(&mut self, b: &Body, t: &TerminatorKind, is_main: bool, ret_ty: &Ty) {
        match t {
            TerminatorKind::Goto(bb) => {
                let _ = writeln!(self.fns, "  br label %bb{}{}", bb.0, self.dbg_suffix());
            }
            TerminatorKind::SwitchBool { discr, then, else_ } => {
                let Some((ll, v, _)) = self.operand(b, discr) else {
                    let _ = writeln!(self.fns, "  br label %bb{}{}", else_.0, self.dbg_suffix());
                    return;
                };
                let c = self.fresh();
                let _ = writeln!(self.fns, "  {c} = icmp ne {ll} {v}, 0{}", self.dbg_suffix());
                let _ = writeln!(
                    self.fns,
                    "  br i1 {c}, label %bb{}, label %bb{}{}",
                    then.0,
                    else_.0,
                    self.dbg_suffix()
                );
            }
            TerminatorKind::Call {
                target,
                args,
                dest,
                next,
            } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    if let Some((ll, v, _)) = self.operand(b, a) {
                        arg_vals.push(format!("{ll} {v}"));
                    }
                }
                let dest_ty = {
                    let (_, t) = self.place_ptr_peek(b, dest);
                    t
                };
                let dest_zst = self.is_zst(&dest_ty);
                match target {
                    CallTarget::Builtin(Builtin::Println) => {
                        self.need_puts = true;
                        let t = self.fresh();
                        let _ = writeln!(
                            self.fns,
                            "  {t} = call i32 @puts({}){}",
                            arg_vals.join(", "),
                            self.dbg_suffix()
                        );
                    }
                    CallTarget::Fn { def, symbol } => {
                        let ll_ret = if dest_zst {
                            "void".to_owned()
                        } else {
                            self.llty(&dest_ty)
                        };
                        // 无 body 的外部 fn:补 declare
                        let is_extern = matches!(
                            &self.krate.item(*def).kind,
                            hir::ItemKind::Fn(decl) if decl.body.is_none()
                        );
                        if is_extern {
                            let param_tys: Vec<String> = arg_vals
                                .iter()
                                .map(|a| a.split(' ').next().unwrap_or("i8").to_owned())
                                .collect();
                            self.externs.insert(
                                symbol.clone(),
                                format!("{ll_ret} @{symbol}({})", param_tys.join(", ")),
                            );
                        }
                        if dest_zst {
                            let _ = writeln!(
                                self.fns,
                                "  call void @{symbol}({}){}",
                                arg_vals.join(", "),
                                self.dbg_suffix()
                            );
                        } else {
                            let t = self.fresh();
                            let _ = writeln!(
                                self.fns,
                                "  {t} = call {ll_ret} @{symbol}({}){}",
                                arg_vals.join(", "),
                                self.dbg_suffix()
                            );
                            let (ptr, _) = self.place_ptr(b, dest);
                            let _ = writeln!(
                                self.fns,
                                "  store {ll_ret} {t}, ptr {ptr}{}",
                                self.dbg_suffix()
                            );
                        }
                    }
                }
                let _ = writeln!(self.fns, "  br label %bb{}{}", next.0, self.dbg_suffix());
            }
            TerminatorKind::Return => {
                if is_main {
                    if matches!(ret_ty, Ty::Prim(PrimTy::I32)) {
                        let t = self.fresh();
                        let _ =
                            writeln!(self.fns, "  {t} = load i32, ptr %l0{}", self.dbg_suffix());
                        let _ = writeln!(self.fns, "  ret i32 {t}{}", self.dbg_suffix());
                    } else {
                        let _ = writeln!(self.fns, "  ret i32 0{}", self.dbg_suffix());
                    }
                } else if self.is_zst(ret_ty) {
                    let _ = writeln!(self.fns, "  ret void{}", self.dbg_suffix());
                } else {
                    let ll = self.llty(ret_ty);
                    let t = self.fresh();
                    let _ = writeln!(self.fns, "  {t} = load {ll}, ptr %l0{}", self.dbg_suffix());
                    let _ = writeln!(self.fns, "  ret {ll} {t}{}", self.dbg_suffix());
                }
            }
            TerminatorKind::Unreachable => {
                self.fns.push_str("  unreachable\n");
            }
        }
    }

    /// place 的指向类型(不发指令;仅 dest 类型预查)。
    fn place_ptr_peek(&self, b: &Body, p: &Place) -> (String, Ty) {
        let mut ty = b.local(p.local).ty.clone();
        for elem in &p.proj {
            ty = match elem {
                ProjElem::Deref => match ty {
                    Ty::Ref(inner, _) | Ty::RawPtr(inner, _) => *inner,
                    other => other,
                },
                ProjElem::Field(i) => match &ty {
                    Ty::Tuple(v) => v.get(*i as usize).cloned().unwrap_or(Ty::Err),
                    Ty::Adt(d, args) => crate::typeck::adt_field_tys(self.krate, *d, args)
                        .get(*i as usize)
                        .cloned()
                        .unwrap_or(Ty::Err),
                    _ => Ty::Err,
                },
            };
        }
        (String::new(), ty)
    }
}

// ---------------------------------------------------------------------------
// 指令选择表
// ---------------------------------------------------------------------------

fn prim_llty(p: PrimTy) -> &'static str {
    match p {
        PrimTy::I8 | PrimTy::U8 | PrimTy::Bool => "i8",
        PrimTy::I16 | PrimTy::U16 => "i16",
        PrimTy::I32 | PrimTy::U32 | PrimTy::Char => "i32",
        PrimTy::I64 | PrimTy::U64 | PrimTy::Usize => "i64",
        PrimTy::F32 => "float",
        PrimTy::F64 => "double",
        PrimTy::Str => "ptr",
    }
}

fn ty_signed(t: &Ty) -> bool {
    matches!(
        t,
        Ty::Prim(PrimTy::I8 | PrimTy::I16 | PrimTy::I32 | PrimTy::I64)
    )
}

fn arith_inst(op: BinOp, is_float: bool, signed: bool) -> &'static str {
    match (op, is_float) {
        (BinOp::Add, true) => "fadd",
        (BinOp::Add, false) => "add",
        (BinOp::Sub, true) => "fsub",
        (BinOp::Sub, false) => "sub",
        (BinOp::Mul, true) => "fmul",
        (BinOp::Mul, false) => "mul",
        (BinOp::Div, true) => "fdiv",
        (BinOp::Div, false) if signed => "sdiv",
        (BinOp::Div, false) => "udiv",
        (BinOp::Rem, true) => "frem",
        (BinOp::Rem, false) if signed => "srem",
        (BinOp::Rem, false) => "urem",
        _ => unreachable!(),
    }
}

fn bit_inst(op: BinOp, signed: bool) -> &'static str {
    match op {
        BinOp::BitAnd => "and",
        BinOp::BitOr => "or",
        BinOp::BitXor => "xor",
        BinOp::Shl => "shl",
        BinOp::Shr if signed => "ashr",
        BinOp::Shr => "lshr",
        _ => unreachable!(),
    }
}

fn icmp_cond(op: BinOp, signed: bool) -> &'static str {
    match (op, signed) {
        (BinOp::Eq, _) => "eq",
        (BinOp::Ne, _) => "ne",
        (BinOp::Lt, true) => "slt",
        (BinOp::Lt, false) => "ult",
        (BinOp::Gt, true) => "sgt",
        (BinOp::Gt, false) => "ugt",
        (BinOp::Le, true) => "sle",
        (BinOp::Le, false) => "ule",
        (BinOp::Ge, true) => "sge",
        (BinOp::Ge, false) => "uge",
        _ => unreachable!(),
    }
}

fn fcmp_cond(op: BinOp) -> &'static str {
    match op {
        BinOp::Eq => "oeq",
        BinOp::Ne => "une",
        BinOp::Lt => "olt",
        BinOp::Gt => "ogt",
        BinOp::Le => "ole",
        BinOp::Ge => "oge",
        _ => unreachable!(),
    }
}

/// 数值/bool/char 转换指令(RXS-0046 合法面;同 LLVM 类型在调用方短路)。
fn cast_inst(from: &Ty, to: &Ty) -> &'static str {
    let (Ty::Prim(f), Ty::Prim(t)) = (from, to) else {
        return "bitcast";
    };
    let fw = prim_width(*f);
    let tw = prim_width(*t);
    match (from.is_float(), to.is_float()) {
        (true, true) => {
            if fw < tw {
                "fpext"
            } else {
                "fptrunc"
            }
        }
        (true, false) => {
            if ty_signed(to) {
                "fptosi"
            } else {
                "fptoui"
            }
        }
        (false, true) => {
            // bool/char/unsigned → uitofp;有符号整数 → sitofp
            if ty_signed(from) { "sitofp" } else { "uitofp" }
        }
        (false, false) => {
            if fw > tw {
                "trunc"
            } else if ty_signed(from) && !matches!(f, PrimTy::Bool | PrimTy::Char) {
                "sext"
            } else {
                "zext"
            }
        }
    }
}

fn prim_width(p: PrimTy) -> u32 {
    match p {
        PrimTy::I8 | PrimTy::U8 | PrimTy::Bool => 8,
        PrimTy::I16 | PrimTy::U16 => 16,
        PrimTy::I32 | PrimTy::U32 | PrimTy::Char | PrimTy::F32 => 32,
        PrimTy::I64 | PrimTy::U64 | PrimTy::Usize | PrimTy::F64 => 64,
        PrimTy::Str => 64,
    }
}

/// 整数常量按位宽回卷为有符号文本(LLVM IR 整数常量为有符号解读;位宽 ≤64)。
fn wrap_signed(v: i128, width: u32) -> i128 {
    let m = v & ((1i128 << width) - 1);
    if m >= (1i128 << (width - 1)) {
        m - (1i128 << width)
    } else {
        m
    }
}

fn escape_md(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::Edition;

    fn ir_for(src: &str) -> String {
        let diag = DiagCtxt::new();
        let mut sm = crate::source_map::SourceMap::new();
        let id = sm.add_file("test.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, id, Edition::Rx0, &diag);
        cx.check_crate();
        let mir = cx.mir_crate();
        assert!(diag.emitted().is_empty(), "诊断非空: {:?}", diag.emitted());
        let krate = cx.hir_crate();
        emit_llvm_ir(
            &mir,
            &krate,
            &sm,
            &CodegenOpts {
                module_name: "test",
                file_name: "test.rx",
                directory: "H:\\tmp",
            },
        )
    }

    #[test]
    fn hello_world_ir_essentials() {
        let ir =
            ir_for("fn main() {\n    let greeting = \"hello, rurix\";\n    println(greeting);\n}");
        assert!(ir.contains("define i32 @main()"), "{ir}");
        assert!(ir.contains("call i32 @puts(ptr"), "{ir}");
        assert!(
            ir.contains("c\"hello, rurix\\00\""),
            "字符串全局缺失:\n{ir}"
        );
        assert!(ir.contains("ret i32 0"), "{ir}");
        assert!(ir.contains("\"CodeView\", i32 1"), "{ir}");
        assert!(ir.contains("!DISubprogram(name: \"main\""), "{ir}");
        assert!(
            ir.contains("!DILocation(line: 3"),
            "println 行号缺失:\n{ir}"
        );
    }

    #[test]
    fn arithmetic_and_calls_emit() {
        let ir = ir_for(
            "fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() {\n    let s = add(40, 2);\n    if s == 42 {\n        println(\"ok\");\n    }\n}",
        );
        assert!(ir.contains("define i32 @rx_add_"), "{ir}");
        assert!(ir.contains("= add i32"), "{ir}");
        assert!(ir.contains("icmp eq i32"), "{ir}");
        assert!(ir.contains("br i1"), "{ir}");
    }
}
