//! device MIR → NVPTX 约束 LLVM IR 文本(M4.2,RXS-0070~0073;07 §7 / D-205·D-207)。
//!
//! 绑定通道延续 host(M2.3 文本 IR + 外部 LLVM 工具,M2_PLAN v1.3):本模块产
//! `nvptx64-nvidia-cuda` 三元组的文本 IR,驱动调 pin 的 clang 22.1.x
//! `--target=nvptx64-nvidia-cuda -mcpu=sm_89 -S` 经 NVPTX 后端汇编为 PTX。
//!
//! M4.2 codegen 约定(SAXPY 雏形子集):
//! - `kernel fn` → `define ptx_kernel void @sym(...)`(`ptx_kernel` 调用约定 →
//!   PTX `.entry`,RXS-0070);`device fn` → `define internal`;
//! - `View<space, T>`/`ViewMut<space, T>` 形参 ABI = `ptr addrspace(N)`,索引
//!   `v[i]` → `getelementptr` + `load`/`store`(RXS-0071);
//! - `ThreadCtx<DIM>` 形参零尺寸(不占 ABI 槽位),其方法 → sreg/barrier
//!   intrinsics(RXS-0072,DIM=1 取 `.x` 维);
//! - 全 local 进 addrspace(5)(NVPTX 栈/寄存器空间)alloca,mem2reg 交给后端;
//! - bool 存 i8,分支 `icmp ne i8 .. 0`(与 host codegen 同口径);
//! - 作用面外构造 → `RX6003`(不支持构造)/ `RX6005`(超出 NVPTX 约束子集)。

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::ast::{BinOp, FnColor, UnOp};
use crate::codegen::{
    arith_inst, bit_inst, cast_inst, fcmp_cond, icmp_cond, prim_llty, prim_width, ty_signed,
    wrap_signed,
};
use crate::diag::ErrorCode;
use crate::hir::{self, DeviceIntrinsic, PrimTy};
use crate::mir::{
    Body, CallTarget, Const, LocalIdx, Operand, Place, ProjElem, Rvalue, StatementKind,
    TerminatorKind,
};
use crate::query::QueryCtx;
use crate::resolve::{ADDR_SPACES, Resolutions};
use crate::span::Span;
use crate::ty::{Ty, thread_ctx_dim};

/// device codegen 失败(NVPTX 约束子集外构造)。驱动/测试转结构化诊断:
/// `Unsupported` → `RX6003`、`Constraint` → `RX6005`(RXS-0073)。
#[derive(Debug, Clone)]
pub struct DeviceCodegenError {
    pub span: Span,
    /// `6003` = 不支持构造;`6005` = 超出 NVPTX 约束子集。
    pub code: u16,
    pub message_key: &'static str,
    pub detail: String,
}

impl DeviceCodegenError {
    fn unsupported(span: Span, detail: impl Into<String>) -> Self {
        DeviceCodegenError {
            span,
            code: 6003,
            message_key: "codegen.device_unsupported",
            detail: detail.into(),
        }
    }
    fn constraint(span: Span, detail: impl Into<String>) -> Self {
        DeviceCodegenError {
            span,
            code: 6005,
            message_key: "codegen.device_constraint",
            detail: detail.into(),
        }
    }
}

/// 驱动 / UI 通道入口:构建 device MIR(`kernel fn` 为根)+ NVPTX codegen。
/// 无 kernel → `None`(无 device 产物);codegen 失败 → 经 `cx.diag()` 落结构化
/// 诊断(`RX6003`/`RX6005`,RXS-0073)并返回 `None`;成功 → `Some(NVPTX IR)`。
/// ptxas 干验证关卡(`RX6004`)由驱动在产 PTX 后另行实施(RXS-0073)。
pub fn build_and_emit(cx: &QueryCtx<'_>, module_name: &str) -> Option<String> {
    let bodies = cx.device_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    // device MIR 构建已报错(RX6001 等作用面外构造)→ 不级联 codegen(防一错多报)。
    if cx.diag().has_errors() {
        return None;
    }
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    match emit_nvptx_ir(&bodies, &krate, &res, module_name) {
        Ok(ir) => Some(ir),
        Err(e) => {
            cx.diag()
                .struct_error(ErrorCode(e.code), e.message_key)
                .arg("detail", e.detail.clone())
                .span_label(e.span, "in device (kernel) code")
                .emit();
            None
        }
    }
}

/// device codegen 入口:device MIR 实例集 → 单一 NVPTX LLVM IR 模块文本。
/// `bodies` 来自 [`crate::mir_build::build_device_crate`](kernel 为根)。
pub fn emit_nvptx_ir(
    bodies: &[Body],
    krate: &hir::Crate,
    res: &Resolutions,
    module_name: &str,
) -> Result<String, DeviceCodegenError> {
    let _ = krate;
    let mut cg = Cg {
        res,
        fns: String::new(),
        globals: String::new(),
        tmp: 0,
        intrinsics: BTreeSet::new(),
        array_base: HashMap::new(),
        uses_libdevice: false,
        cur_span: Span::new(crate::span::SourceId(0), 0, 0, crate::span::Edition::Rx0),
        annotation_nodes: Vec::new(),
    };
    for b in bodies {
        cg.emit_body(b)?;
    }

    let mut out = String::new();
    let _ = writeln!(out, "; ModuleID = '{module_name}'");
    let _ = writeln!(out, "source_filename = \"{module_name}\"");
    let _ = writeln!(out, "target triple = \"nvptx64-nvidia-cuda\"");
    out.push('\n');
    for decl in &cg.intrinsics {
        out.push_str(decl);
        out.push('\n');
    }
    if !cg.intrinsics.is_empty() {
        out.push('\n');
    }
    // shared 数组(addrspace 3)/ 局部数组(addrspace 5)模块级 global(M5.3,
    // RXS-0079);先于函数定义,供 place_ptr 引用。
    if !cg.globals.is_empty() {
        out.push_str(&cg.globals);
        out.push('\n');
    }
    out.push_str(&cg.fns);
    if !cg.annotation_nodes.is_empty() {
        out.push('\n');
        let refs: Vec<String> = (0..cg.annotation_nodes.len())
            .map(|i| format!("!{i}"))
            .collect();
        let _ = writeln!(out, "!nvvm.annotations = !{{{}}}", refs.join(", "));
        for (i, node) in cg.annotation_nodes.iter().enumerate() {
            let _ = writeln!(out, "!{i} = {node}");
        }
    }
    // NVVMReflect 精确路径留痕(RXS-0081/0082):仅在用到 libdevice `__nv_*` 时
    // 发模块 flag,声明默认精确数值语义(ftz=0;prec-sqrt/div 由 libdevice 默认精确
    // 变体提供);FASTMATH 双通道为后续编译器开关。无 libdevice 时不发(保 SAXPY
    // 等既有 IR golden 形态不变)。
    if cg.uses_libdevice {
        out.push('\n');
        out.push_str("!llvm.module.flags = !{!0}\n");
        out.push_str("!0 = !{i32 4, !\"nvvm-reflect-ftz\", i32 0}\n");
    }
    Ok(out)
}

fn isqrt_u64(n: u64) -> Option<u64> {
    if n == 0 {
        return Some(0);
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    if x * x == n { Some(x) } else { None }
}

struct Cg<'a> {
    res: &'a Resolutions,
    fns: String,
    /// 模块级 global(shared addrspace 3 / 局部数组 addrspace 5;M5.3,RXS-0079)。
    globals: String,
    tmp: u32,
    /// 已用 NVPTX intrinsic 的 `declare` 行(去重有序)。
    intrinsics: BTreeSet<String>,
    /// 当前 body 内数组型 local 的基址(local idx → (base 寄存器/全局名, addrspace));
    /// shared 数组 = addrspace 3 模块 global,局部数组 = addrspace 5 alloca。每 body 重置。
    array_base: HashMap<u32, (String, u32)>,
    /// 是否用到 libdevice `__nv_*` 数学符号(决定是否发 NVVMReflect 模块 flag)。
    uses_libdevice: bool,
    /// kernel launch bounds 元数据节点(`nvvm.annotations`,M5.3 review fix)。
    annotation_nodes: Vec<String>,
    /// 当前 body span(llty 等无 span 入参处的错误锚点)。
    cur_span: Span,
}

/// place 解析产物:指针寄存器 + 其地址空间 + 指向类型。
struct PlacePtr {
    reg: String,
    addrspace: u32,
    ty: Ty,
}

impl Cg<'_> {
    fn fresh(&mut self) -> String {
        let t = format!("%t{}", self.tmp);
        self.tmp += 1;
        t
    }

    // -- 类型 -------------------------------------------------------------------

    /// 标量 / 指针 LLVM 类型串(View 族 → 其地址空间指针)。
    fn llty(&self, ty: &Ty) -> Result<String, DeviceCodegenError> {
        match ty {
            Ty::Prim(p) => Ok(prim_llty(*p).to_owned()),
            Ty::Adt(d, args) if self.res.lang_items.view_mutable(*d).is_some() => {
                let n = self.view_addrspace(args, self.cur_span)?;
                Ok(format!("ptr addrspace({n})"))
            }
            // 引用 / 聚合 / 其它非标量值类型 = NVPTX codegen 作用面外构造(RX6003)
            _ => Err(DeviceCodegenError::unsupported(
                self.cur_span,
                "this value type",
            )),
        }
    }

    /// `View`/`ViewMut` 首类型实参(地址空间标记)→ NVPTX addrspace 号(RXS-0071)。
    fn view_addrspace(&self, args: &[Ty], span: Span) -> Result<u32, DeviceCodegenError> {
        let space = args.first().ok_or_else(|| {
            DeviceCodegenError::constraint(span, "view type missing address-space argument")
        })?;
        let Ty::Adt(sd, _) = space else {
            return Err(DeviceCodegenError::constraint(
                span,
                "view address-space marker is not a known space",
            ));
        };
        let idx = self
            .res
            .lang_items
            .addr_spaces
            .iter()
            .position(|s| *s == Some(*sd))
            .ok_or_else(|| DeviceCodegenError::constraint(span, "unknown address-space marker"))?;
        // ADDR_SPACES 序 → NVPTX addrspace 号(RXS-0071):
        // global(0)→1, shared(1)→3, constant(2)→4, local(3)→5, host(4)→不支持
        match ADDR_SPACES[idx] {
            "global" => Ok(1),
            "shared" => Ok(3),
            "constant" => Ok(4),
            "local" => Ok(5),
            other => Err(DeviceCodegenError::constraint(
                span,
                format!("address space `{other}` is not codegen-able as a device view"),
            )),
        }
    }

    /// 零尺寸(不占 ABI / 不开 alloca):unit 与 `ThreadCtx`。
    fn is_zst(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Tuple(v) => v.is_empty(),
            Ty::Adt(d, _) => self.res.lang_items.is_thread_ctx(*d),
            _ => false,
        }
    }

    // -- 函数 -------------------------------------------------------------------

    fn emit_body(&mut self, b: &Body) -> Result<(), DeviceCodegenError> {
        self.tmp = 0;
        self.cur_span = b.span;
        self.array_base.clear();
        let is_kernel = b.color == FnColor::Kernel;
        let ret_ty = b.ret_ty().clone();
        let ret_void = is_kernel || self.is_zst(&ret_ty);

        // 形参(MIR _1..=arg_count;跳过 ZST = ThreadCtx/unit)
        let mut params = Vec::new();
        for i in 1..=b.arg_count {
            let lty = &b.locals[i].ty;
            if self.is_zst(lty) {
                continue;
            }
            params.push(format!("{} %arg{i}", self.llty(lty)?));
        }

        let cc = if is_kernel {
            "ptx_kernel "
        } else {
            "internal "
        };
        let ret_ll = if ret_void {
            "void".to_owned()
        } else {
            self.llty(&ret_ty)?
        };
        let _ = writeln!(
            self.fns,
            "define {cc}{ret_ll} @{}({}) {{",
            b.symbol,
            params.join(", ")
        );
        self.fns.push_str("entry:\n");

        // alloca 全部非 ZST local(addrspace(5));返回槽 _0 仅 device fn 非 ZST 时。
        // 数组型 local(`shared let [T; N]` 等,M5.3,RXS-0079):shared → 模块级
        // addrspace(3) global;非 shared → `[N x T]` addrspace(5) alloca;基址入
        // array_base,索引经数组 gep(place_ptr)。
        for (i, l) in b.locals.iter().enumerate() {
            if i == 0 && ret_void {
                continue;
            }
            if self.is_zst(&l.ty) {
                continue;
            }
            if l.shared && !matches!(&l.ty, Ty::Array(_)) {
                return Err(DeviceCodegenError::constraint(
                    l.span,
                    "shared let requires a fixed-size array type (RXS-0071/0079)",
                ));
            }
            if let Ty::Array(elem) = &l.ty {
                let n = l.array_len.ok_or_else(|| {
                    DeviceCodegenError::constraint(
                        l.span,
                        "array local without a static length (const-generic length is RD-007)",
                    )
                })?;
                let elem_ll = self.llty(elem)?;
                if l.shared {
                    let gsym = format!("__shared_{}_{i}", b.symbol);
                    let _ = writeln!(
                        self.globals,
                        "@{gsym} = internal addrspace(3) global [{n} x {elem_ll}] undef"
                    );
                    self.array_base.insert(i as u32, (format!("@{gsym}"), 3));
                } else {
                    let _ = writeln!(self.fns, "  %l{i} = alloca [{n} x {elem_ll}], addrspace(5)");
                    self.array_base.insert(i as u32, (format!("%l{i}"), 5));
                }
                continue;
            }
            let _ = writeln!(
                self.fns,
                "  %l{i} = alloca {}, addrspace(5)",
                self.llty(&l.ty)?
            );
        }
        // 形参落 alloca
        for i in 1..=b.arg_count {
            let lty = &b.locals[i].ty;
            if self.is_zst(lty) {
                continue;
            }
            let _ = writeln!(
                self.fns,
                "  store {} %arg{i}, ptr addrspace(5) %l{i}",
                self.llty(lty)?
            );
        }
        let _ = writeln!(self.fns, "  br label %bb0");

        for (bi, bb) in b.blocks.iter().enumerate() {
            let _ = writeln!(self.fns, "bb{bi}:");
            for s in &bb.stmts {
                let StatementKind::Assign(place, rv) = &s.kind;
                self.emit_assign(b, place, rv)?;
            }
            self.emit_terminator(
                b,
                &bb.terminator.kind,
                bb.terminator.span,
                ret_void,
                &ret_ty,
            )?;
        }
        self.fns.push_str("}\n\n");
        if is_kernel {
            self.emit_launch_bounds(b)?;
        }
        Ok(())
    }

    fn kernel_thread_ctx_dim(&self, b: &Body) -> Option<u8> {
        for i in 1..=b.arg_count {
            if let Some(d) = thread_ctx_dim(&b.locals[i].ty, &self.res.lang_items) {
                return Some(d);
            }
        }
        None
    }

    fn emit_launch_bounds(&mut self, b: &Body) -> Result<(), DeviceCodegenError> {
        let shared_lens: Vec<u64> = b
            .locals
            .iter()
            .filter(|l| l.shared)
            .map(|l| {
                l.array_len.ok_or_else(|| {
                    DeviceCodegenError::constraint(
                        l.span,
                        "shared array without static length cannot infer launch bounds",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if shared_lens.is_empty() {
            return Ok(());
        }
        let first = shared_lens[0];
        if !shared_lens.iter().all(|&n| n == first) {
            return Err(DeviceCodegenError::constraint(
                b.span,
                "inconsistent shared array sizes for launch bounds inference",
            ));
        }
        let dim = self.kernel_thread_ctx_dim(b).unwrap_or(1);
        let sym = &b.symbol;
        match dim {
            1 => {
                self.annotation_nodes
                    .push(format!("!{{ptr @{sym}, !\"reqntidx\", i32 {first}}}"));
            }
            2 => {
                let side = isqrt_u64(first).ok_or_else(|| {
                    DeviceCodegenError::constraint(
                        b.span,
                        format!(
                            "shared array length {first} is not a square tile for 2D launch bounds"
                        ),
                    )
                })?;
                self.annotation_nodes.push(format!(
                    "!{{ptr @{sym}, !\"reqntidx\", i32 {side}, !\"reqntidy\", i32 {side}}}"
                ));
            }
            _ => {}
        }
        Ok(())
    }

    // -- place -----------------------------------------------------------------

    /// place → 指针(走 alloca / shared global 起点 + 投影链)。
    fn place_ptr(&mut self, b: &Body, p: &Place) -> Result<PlacePtr, DeviceCodegenError> {
        // 数组型 local 起点 = array_base(shared global addrspace 3 / 局部 alloca
        // addrspace 5);其余 local 起点 = `%l{idx}` addrspace 5。
        let (mut reg, mut addrspace) = match self.array_base.get(&p.local.0) {
            Some((r, a)) => (r.clone(), *a),
            None => (format!("%l{}", p.local.0), 5u32),
        };
        let mut ty = b.local(p.local).ty.clone();
        for elem in &p.proj {
            match elem {
                ProjElem::Index(idx_local) => {
                    match &ty {
                        // 数组元素(M5.3):base 即存储(alloca/global),元素 gep 不 load。
                        Ty::Array(elem_ty) => {
                            let elem_ty = (**elem_ty).clone();
                            let iv = self.load_local_usize(*idx_local);
                            let ell = self.llty(&elem_ty)?;
                            let e = self.fresh();
                            let _ = writeln!(
                                self.fns,
                                "  {e} = getelementptr {ell}, ptr addrspace({addrspace}) {reg}, i64 {iv}"
                            );
                            reg = e;
                            ty = elem_ty;
                            // addrspace 不变(数组在原空间内偏移)。
                        }
                        // View 族(M4.2,RXS-0071):base 是地址空间指针(存于 alloca),
                        // 先 load 出指针再按 index gep。
                        Ty::Adt(d, args)
                            if self.res.lang_items.view_mutable(*d).is_some()
                                && args.len() >= 2 =>
                        {
                            let space_n = self.view_addrspace(args, b.local(p.local).span)?;
                            let elem_ty = args[1].clone();
                            let base = self.fresh();
                            let _ = writeln!(
                                self.fns,
                                "  {base} = load ptr addrspace({space_n}), ptr addrspace({addrspace}) {reg}"
                            );
                            let iv = self.load_local_usize(*idx_local);
                            let ell = self.llty(&elem_ty)?;
                            let e = self.fresh();
                            let _ = writeln!(
                                self.fns,
                                "  {e} = getelementptr {ell}, ptr addrspace({space_n}) {base}, i64 {iv}"
                            );
                            reg = e;
                            addrspace = space_n;
                            ty = elem_ty;
                        }
                        _ => {
                            return Err(DeviceCodegenError::constraint(
                                b.local(p.local).span,
                                "indexed place is not a device view or array",
                            ));
                        }
                    }
                }
                ProjElem::Deref => {
                    return Err(DeviceCodegenError::constraint(
                        b.local(p.local).span,
                        "reference deref is out of the device codegen subset",
                    ));
                }
                ProjElem::Field(_) | ProjElem::VariantField { .. } => {
                    return Err(DeviceCodegenError::constraint(
                        b.local(p.local).span,
                        "aggregate field projection is out of the device codegen subset",
                    ));
                }
            }
        }
        Ok(PlacePtr { reg, addrspace, ty })
    }

    /// 载入一个 usize local 的值(`ProjElem::Index` 下标;无投影)。
    fn load_local_usize(&mut self, l: LocalIdx) -> String {
        let t = self.fresh();
        let _ = writeln!(self.fns, "  {t} = load i64, ptr addrspace(5) %l{}", l.0);
        t
    }

    /// operand → (LLVM 类型, 值, ty);unit/ZST → None。
    fn operand(
        &mut self,
        b: &Body,
        o: &Operand,
    ) -> Result<Option<(String, String, Ty)>, DeviceCodegenError> {
        match o {
            Operand::Copy(p) | Operand::Move(p) => {
                let pp = self.place_ptr(b, p)?;
                if self.is_zst(&pp.ty) {
                    return Ok(None);
                }
                let ll = self.llty(&pp.ty)?;
                let t = self.fresh();
                let _ = writeln!(
                    self.fns,
                    "  {t} = load {ll}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(Some((ll, t, pp.ty)))
            }
            Operand::Const(c) => Ok(match c {
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
                Const::Str(_) => {
                    return Err(DeviceCodegenError::unsupported(
                        b.span,
                        "string literal in device code",
                    ));
                }
            }),
        }
    }

    // -- 语句 -------------------------------------------------------------------

    fn emit_assign(
        &mut self,
        b: &Body,
        place: &Place,
        rv: &Rvalue,
    ) -> Result<(), DeviceCodegenError> {
        match rv {
            Rvalue::Use(o) => {
                let Some((ll, v, _)) = self.operand(b, o)? else {
                    return Ok(());
                };
                let pp = self.place_ptr(b, place)?;
                let _ = writeln!(
                    self.fns,
                    "  store {ll} {v}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
            Rvalue::BinaryOp(op, a, c) => {
                let Some((lla, va, ta)) = self.operand(b, a)? else {
                    return Ok(());
                };
                let Some((_, vc, _)) = self.operand(b, c)? else {
                    return Ok(());
                };
                let signed = ty_signed(&ta);
                let is_float = ta.is_float();
                let t = self.fresh();
                let (result, is_i1) = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        let inst = arith_inst(*op, is_float, signed);
                        let _ = writeln!(self.fns, "  {t} = {inst} {lla} {va}, {vc}");
                        (t.clone(), false)
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        let inst = bit_inst(*op, signed);
                        let _ = writeln!(self.fns, "  {t} = {inst} {lla} {va}, {vc}");
                        (t.clone(), false)
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        let inst = if is_float {
                            format!("fcmp {}", fcmp_cond(*op))
                        } else {
                            format!("icmp {}", icmp_cond(*op, signed))
                        };
                        let _ = writeln!(self.fns, "  {t} = {inst} {lla} {va}, {vc}");
                        (t.clone(), true)
                    }
                    BinOp::And | BinOp::Or => {
                        let _ = writeln!(self.fns, "  {t} = and {lla} {va}, {vc}");
                        (t.clone(), false)
                    }
                };
                let val = if is_i1 {
                    let z = self.fresh();
                    let _ = writeln!(self.fns, "  {z} = zext i1 {result} to i8");
                    z
                } else {
                    result
                };
                let pp = self.place_ptr(b, place)?;
                let ll = self.llty(&pp.ty)?;
                let _ = writeln!(
                    self.fns,
                    "  store {ll} {val}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
            Rvalue::UnaryOp(op, a) => {
                let Some((lla, va, ta)) = self.operand(b, a)? else {
                    return Ok(());
                };
                let t = self.fresh();
                match op {
                    UnOp::Neg if ta.is_float() => {
                        let _ = writeln!(self.fns, "  {t} = fneg {lla} {va}");
                    }
                    UnOp::Neg => {
                        let _ = writeln!(self.fns, "  {t} = sub {lla} 0, {va}");
                    }
                    UnOp::Not if matches!(ta, Ty::Prim(PrimTy::Bool)) => {
                        let _ = writeln!(self.fns, "  {t} = xor {lla} {va}, 1");
                    }
                    UnOp::Not => {
                        let _ = writeln!(self.fns, "  {t} = xor {lla} {va}, -1");
                    }
                    UnOp::Deref => {
                        return Err(DeviceCodegenError::constraint(
                            b.span,
                            "deref operator in device code",
                        ));
                    }
                }
                let pp = self.place_ptr(b, place)?;
                let ll = self.llty(&pp.ty)?;
                let _ = writeln!(
                    self.fns,
                    "  store {ll} {t}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
            Rvalue::Cast(o, target) => {
                let Some((lla, va, ta)) = self.operand(b, o)? else {
                    return Ok(());
                };
                let to_ll = self.llty(target)?;
                let val = if lla == to_ll {
                    va
                } else {
                    let inst = cast_inst(&ta, target);
                    let t = self.fresh();
                    let _ = writeln!(self.fns, "  {t} = {inst} {lla} {va} to {to_ll}");
                    t
                };
                let pp = self.place_ptr(b, place)?;
                let _ = writeln!(
                    self.fns,
                    "  store {to_ll} {val}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
            Rvalue::Ref(..) => Err(DeviceCodegenError::constraint(
                b.span,
                "taking a reference is out of the device codegen subset",
            )),
            Rvalue::Aggregate(..) | Rvalue::VariantAggregate { .. } => Err(
                DeviceCodegenError::constraint(b.span, "aggregate construction in device code"),
            ),
            Rvalue::Discriminant(_) => Err(DeviceCodegenError::constraint(
                b.span,
                "enum discriminant read in device code",
            )),
        }
    }

    // -- 终结子 -----------------------------------------------------------------

    fn emit_terminator(
        &mut self,
        b: &Body,
        t: &TerminatorKind,
        span: Span,
        ret_void: bool,
        ret_ty: &Ty,
    ) -> Result<(), DeviceCodegenError> {
        match t {
            TerminatorKind::Goto(bb) => {
                let _ = writeln!(self.fns, "  br label %bb{}", bb.0);
            }
            TerminatorKind::SwitchBool { discr, then, else_ } => {
                let Some((ll, v, _)) = self.operand(b, discr)? else {
                    return Err(DeviceCodegenError::constraint(
                        span,
                        "switch on zero-sized value",
                    ));
                };
                let c = self.fresh();
                let _ = writeln!(self.fns, "  {c} = icmp ne {ll} {v}, 0");
                let _ = writeln!(
                    self.fns,
                    "  br i1 {c}, label %bb{}, label %bb{}",
                    then.0, else_.0
                );
            }
            TerminatorKind::Call {
                target,
                args,
                dest,
                next,
            } => {
                self.emit_call(b, target, args, dest, span)?;
                let _ = writeln!(self.fns, "  br label %bb{}", next.0);
            }
            TerminatorKind::Drop { place: _, next } => {
                // device 子集无 needs-drop 类型(View/标量);drop_elab 不产 Drop
                // 于此类 body,保守降为 no-op 跳转(RXS-0070 作用面)。
                let _ = writeln!(self.fns, "  br label %bb{}", next.0);
            }
            TerminatorKind::Return => {
                if ret_void {
                    let _ = writeln!(self.fns, "  ret void");
                } else {
                    let ll = self.llty(ret_ty)?;
                    let v = self.fresh();
                    let _ = writeln!(self.fns, "  {v} = load {ll}, ptr addrspace(5) %l0");
                    let _ = writeln!(self.fns, "  ret {ll} {v}");
                }
            }
            TerminatorKind::Unreachable => {
                let _ = writeln!(self.fns, "  unreachable");
            }
        }
        Ok(())
    }

    fn emit_call(
        &mut self,
        b: &Body,
        target: &CallTarget,
        args: &[Operand],
        dest: &Place,
        span: Span,
    ) -> Result<(), DeviceCodegenError> {
        match target {
            CallTarget::DeviceIntrinsic(intr) => self.emit_intrinsic(b, *intr, dest, span),
            CallTarget::Fn { symbol, def } => {
                // device fn 直调(MVP 内联交给后端;此处仍发普通 call)
                let mut arg_vals = Vec::new();
                for a in args {
                    if let Some((ll, v, _)) = self.operand(b, a)? {
                        arg_vals.push(format!("{ll} {v}"));
                    }
                }
                let dest_ty = b.local(dest.local).ty.clone();
                let is_unit = self.is_zst(&dest_ty);
                // 被调 device fn 的着色由收集保证(build_device_crate)
                let _ = def;
                if is_unit {
                    let _ = writeln!(self.fns, "  call void @{symbol}({})", arg_vals.join(", "));
                } else {
                    let ll = self.llty(&dest_ty)?;
                    let t = self.fresh();
                    let _ = writeln!(
                        self.fns,
                        "  {t} = call {ll} @{symbol}({})",
                        arg_vals.join(", ")
                    );
                    let pp = self.place_ptr(b, dest)?;
                    let _ = writeln!(
                        self.fns,
                        "  store {ll} {t}, ptr addrspace({}) {}",
                        pp.addrspace, pp.reg
                    );
                }
                Ok(())
            }
            CallTarget::Libdevice { symbol } => {
                // device 数学 intrinsic(RXS-0081):call 保留的外部 `__nv_*` 符号,
                // declare 入模块头(去重),由 libdevice bc 链接解析(RXS-0082)。
                let mut arg_lls = Vec::new();
                let mut arg_vals = Vec::new();
                for a in args {
                    let Some((ll, v, _)) = self.operand(b, a)? else {
                        return Err(DeviceCodegenError::constraint(
                            span,
                            "zero-sized argument to libdevice math intrinsic",
                        ));
                    };
                    arg_lls.push(ll.clone());
                    arg_vals.push(format!("{ll} {v}"));
                }
                let dest_ty = b.local(dest.local).ty.clone();
                let ret_ll = self.llty(&dest_ty)?;
                self.intrinsics.insert(format!(
                    "declare {ret_ll} @{symbol}({})",
                    arg_lls.join(", ")
                ));
                self.uses_libdevice = true;
                let t = self.fresh();
                let _ = writeln!(
                    self.fns,
                    "  {t} = call {ret_ll} @{symbol}({})",
                    arg_vals.join(", ")
                );
                let pp = self.place_ptr(b, dest)?;
                let _ = writeln!(
                    self.fns,
                    "  store {ret_ll} {t}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
            CallTarget::Builtin(_) => Err(DeviceCodegenError::unsupported(
                span,
                "host builtin call in device code",
            )),
        }
    }

    /// device intrinsic(RXS-0072):sreg 组合 / barrier。
    fn emit_intrinsic(
        &mut self,
        b: &Body,
        intr: DeviceIntrinsic,
        dest: &Place,
        _span: Span,
    ) -> Result<(), DeviceCodegenError> {
        match intr {
            DeviceIntrinsic::Barrier => {
                self.intrinsics
                    .insert("declare void @llvm.nvvm.barrier0()".to_owned());
                let _ = writeln!(self.fns, "  call void @llvm.nvvm.barrier0()");
                Ok(())
            }
            DeviceIntrinsic::ThreadIndexX
            | DeviceIntrinsic::ThreadIndexY
            | DeviceIntrinsic::ThreadIndexZ
            | DeviceIntrinsic::BlockIndexX
            | DeviceIntrinsic::BlockIndexY
            | DeviceIntrinsic::BlockIndexZ
            | DeviceIntrinsic::BlockDimX
            | DeviceIntrinsic::BlockDimY
            | DeviceIntrinsic::BlockDimZ
            | DeviceIntrinsic::GlobalIdX
            | DeviceIntrinsic::GlobalIdY
            | DeviceIntrinsic::GlobalIdZ => {
                let val = self.emit_index_intrinsic(intr);
                // 索引类返回 usize(i64);结果存入 dest
                let pp = self.place_ptr(b, dest)?;
                let _ = writeln!(
                    self.fns,
                    "  store i64 {val}, ptr addrspace({}) {}",
                    pp.addrspace, pp.reg
                );
                Ok(())
            }
        }
    }

    /// sreg 取值(i32)→ zext i64;`global_id.{axis}` = ctaid.a*ntid.a + tid.a
    /// (M5.3:DIM≥2 取 .y/.z 维,RXS-0072)。
    fn emit_index_intrinsic(&mut self, intr: DeviceIntrinsic) -> String {
        let read = |cg: &mut Self, name: &str| -> String {
            cg.intrinsics.insert(format!("declare i32 @{name}()"));
            let t = cg.fresh();
            let _ = writeln!(cg.fns, "  {t} = call i32 @{name}()");
            t
        };
        let tid = |a: char| format!("llvm.nvvm.read.ptx.sreg.tid.{a}");
        let ctaid = |a: char| format!("llvm.nvvm.read.ptx.sreg.ctaid.{a}");
        let ntid = |a: char| format!("llvm.nvvm.read.ptx.sreg.ntid.{a}");
        let global = |cg: &mut Self, a: char| -> String {
            let c = read(cg, &ctaid(a));
            let n = read(cg, &ntid(a));
            let t = read(cg, &tid(a));
            let m = cg.fresh();
            let _ = writeln!(cg.fns, "  {m} = mul i32 {c}, {n}");
            let s = cg.fresh();
            let _ = writeln!(cg.fns, "  {s} = add i32 {m}, {t}");
            s
        };
        let i32_val = match intr {
            DeviceIntrinsic::ThreadIndexX => read(self, &tid('x')),
            DeviceIntrinsic::ThreadIndexY => read(self, &tid('y')),
            DeviceIntrinsic::ThreadIndexZ => read(self, &tid('z')),
            DeviceIntrinsic::BlockIndexX => read(self, &ctaid('x')),
            DeviceIntrinsic::BlockIndexY => read(self, &ctaid('y')),
            DeviceIntrinsic::BlockIndexZ => read(self, &ctaid('z')),
            DeviceIntrinsic::BlockDimX => read(self, &ntid('x')),
            DeviceIntrinsic::BlockDimY => read(self, &ntid('y')),
            DeviceIntrinsic::BlockDimZ => read(self, &ntid('z')),
            DeviceIntrinsic::GlobalIdX => global(self, 'x'),
            DeviceIntrinsic::GlobalIdY => global(self, 'y'),
            DeviceIntrinsic::GlobalIdZ => global(self, 'z'),
            DeviceIntrinsic::Barrier => unreachable!("barrier 不取值"),
        };
        let z = self.fresh();
        let _ = writeln!(self.fns, "  {z} = zext i32 {i32_val} to i64");
        z
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 全管线产 device NVPTX IR(无诊断;`emit=ptx` 同源)。
    fn nvptx_ir(src: &str) -> String {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(!diag.has_errors(), "typeck 应无诊断");
        cx.check_coloring();
        cx.check_crate_patterns();
        let ir = super::build_and_emit(&cx, "test").expect("应产出 device IR");
        assert!(!diag.has_errors(), "device codegen 应无诊断");
        ir
    }

    const SAXPY: &str = "kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>, y: View<global, f32>, a: f32, n: usize, t: ThreadCtx<1>) {\n    let i = t.global_id();\n    if i < n {\n        out[i] = a * x[i] + y[i];\n    }\n}\nfn main() {}";

    //@ spec: RXS-0070
    #[test]
    fn kernel_uses_ptx_kernel_cc_and_target() {
        let ir = nvptx_ir(SAXPY);
        assert!(
            ir.contains("target triple = \"nvptx64-nvidia-cuda\""),
            "{ir}"
        );
        assert!(ir.contains("define ptx_kernel void @"), "{ir}");
    }

    //@ spec: RXS-0071
    #[test]
    fn global_view_params_are_addrspace_1() {
        let ir = nvptx_ir(SAXPY);
        assert!(ir.contains("ptr addrspace(1)"), "global view ptr: {ir}");
        assert!(ir.contains("getelementptr float"), "index gep: {ir}");
    }

    //@ spec: RXS-0072
    #[test]
    fn global_id_lowers_to_sreg_composite() {
        let ir = nvptx_ir(SAXPY);
        assert!(
            ir.contains("@llvm.nvvm.read.ptx.sreg.tid.x"),
            "tid sreg: {ir}"
        );
        assert!(
            ir.contains("@llvm.nvvm.read.ptx.sreg.ctaid.x")
                && ir.contains("@llvm.nvvm.read.ptx.sreg.ntid.x"),
            "global_id composite: {ir}"
        );
    }

    //@ spec: RXS-0073
    #[test]
    fn host_array_index_in_kernel_is_rx6003() {
        // host 数组索引在 device codegen 作用面外 → RX6001(MIR lowering 先拦)
        // 此处验证不支持构造路径不 ICE(诊断码裁决见 mir_build / driver)。
        let src = "kernel fn k(t: ThreadCtx<1>) {\n    let a = [1, 2, 3];\n    let _x = a[t.global_id()];\n}\nfn main() {}";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        let _ = super::build_and_emit(&cx, "test");
        assert!(diag.has_errors(), "数组索引应触发诊断(RX6001/RX6003)");
    }
}
