//! MIR 雏形(07 §1 / D-202 第四层;M2.3 host codegen 闭环作用面)。
//!
//! 形态:CFG 化 + 显式类型——locals 表(`_0` 返回槽 / `_1..=_n` 参数)、
//! 基本块(语句 + 终结子)、place/operand/rvalue 三层值模型。
//!
//! M2.3 取舍(IR golden guardrail 随 M3 MIR 定型再激活,CI_GATES §4):
//! - 作用面 = hello-world 闭环所需 host 子集(M2_PLAN v1.3 留痕);
//!   `for`/`?`/closure/`match`/数组索引等经 [`crate::mir_build`] 报 RX6001;
//! - 单态化实例(D-111 全单态化)即独立 [`Body`],泛型实参已代入显式类型;
//! - 借用检查/drop/TBIR 窄门均为 M3 职责,本层不建模。

use crate::ast::{BinOp, FnColor, UnOp};
use crate::hir::{Builtin, DefId, DeviceIntrinsic, PrimTy};
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::Ty;

/// body 内 local 序号(`_0` = 返回槽,`_1..=_arg_count` = 参数)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocalIdx(pub u32);

/// 基本块序号(`bb0` 为入口)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockIdx(pub u32);

/// 单态化后的函数 body(MIR 根;泛型实参已代入,类型全显式)。
#[derive(Debug)]
pub struct Body {
    pub def: DefId,
    /// 链接符号名(`main` 保留原名;其余经 [`mangle`])。
    pub symbol: String,
    /// 函数着色(M4.2,RXS-0070):codegen 分叉 host(`x86_64`)/ device
    /// (`ptx_kernel` 调用约定 / 普通 device fn)通道的依据。
    pub color: FnColor,
    /// 单态化实参(留痕;类型已代入 locals,codegen 不再消费)。
    pub generic_args: Vec<Ty>,
    pub locals: Vec<Local>,
    pub arg_count: usize,
    pub blocks: Vec<BasicBlock>,
    pub span: Span,
    /// 着色阶段类别(G2.2 图形=B,RXS-0161):`None` = 非着色阶段(host /
    /// compute / kernel 既有路径,PTX 收集与 codegen 行为零漂移)。仅 cargo
    /// feature `dxil-backend` 下的图形阶段根收集会置 `Some(Vertex|Fragment)`;
    /// 默认(PTX)路径恒为 `None`(RFC-0004 §4.1;R1.2/R6.7)。
    pub stage: Option<crate::ast::ShaderStage>,
    /// I/O 意图签名(G2.2 图形=B,RXS-0161):源码声明的、跨契约线可观察的
    /// 着色阶段 I/O 元素表(字段名 / builtin·interpolate·varying 种类 / 类型 /
    /// in|out 方向),作 B 路 SPIR-V 保名 by-construction 与签名一致性校验门的
    /// 意图侧依据。非着色阶段(含默认 PTX 路径)恒为空,行为零漂移。
    pub io_sig: Vec<IoSigElem>,
    /// 资源句柄绑定声明(G2.3 绑定布局推导,RXS-0163;PR-E2b 生产接线):着色阶段
    /// 签名里的资源句柄形参(`Texture2D<F>`/`Sampler`)按**声明序**提取,作 host
    /// 侧绑定布局推导(SPIR-V `DescriptorSet`/`Binding` 装饰 / register-space 分配 /
    /// root signature 形态 + RTS0 序列化)的确定性输入([`crate::binding_layout`])。
    /// 非着色阶段(含默认 PTX 路径)恒为空,行为零漂移(R1.2/R6.7)。
    pub resources: Vec<ResourceBinding>,
}

impl Body {
    pub fn local(&self, l: LocalIdx) -> &Local {
        &self.locals[l.0 as usize]
    }

    pub fn ret_ty(&self) -> &Ty {
        &self.locals[0].ty
    }
}

/// 着色阶段 I/O 元素方向(in|out;RXS-0161 意图签名维度之一)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IoDir {
    In,
    Out,
}

/// 着色阶段 I/O 元素种类(RXS-0161;决定 SPIR-V 装饰策略)。
///
/// 与前端 [`crate::shader_stages`] 的字段标注面对齐:`#[builtin(..)]` →
/// [`IoSigKind::Builtin`](emit `BuiltIn` 装饰)、`#[interpolate(..)]` →
/// [`IoSigKind::Interpolate`](插值 varying,emit `Location` 装饰)、无标注的
/// 普通 varying → [`IoSigKind::Varying`](emit `Location` 装饰)。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum IoSigKind {
    /// `#[builtin(name)]` 系统值(保留源码 builtin 名,如 `position`)。
    Builtin(String),
    /// `#[interpolate(mode)]` 插值 varying(保留插值限定名,如 `flat`)。
    Interpolate(String),
    /// 无插值标注的 location varying。
    Varying,
}

/// 着色阶段 I/O 意图签名元素类型(RXS-0161 已建模子集:标量 / 向量)。
///
/// 仅覆盖 [`crate::shader_stages`] RXS-0154 已建模的标量与向量子集;不可映射
/// 类型在 B 路编码器阶段触发 6xxx(strict-only,R1.9),本层不发明降级。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MirIoType {
    /// 标量(如 `f32`/`i32`/`u32`)。
    Scalar(PrimTy),
    /// 向量(分量类型 + 分量数,2..=4;如 `vec4<f32>`)。
    Vector(PrimTy, u8),
}

/// 着色阶段 I/O 意图签名元素(RXS-0161)。
///
/// 记录源码声明且跨契约线可观察的单个 I/O 元素:`field_name`(保名依据)、
/// `kind`(builtin / interpolate / varying)、`ty`(已建模类型子集)、`dir`
/// (in|out 方向)。B 路 SPIR-V 编码器据此 by-construction emit `UserSemantic`/
/// `Location`/`BuiltIn` 装饰,签名一致性校验门据此比对译后 DXIL ISG1/OSG1。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IoSigElem {
    /// 源码字段名(保名依据;非寄存器号/布局)。
    pub field_name: String,
    /// 元素种类(builtin / interpolate / varying)。
    pub kind: IoSigKind,
    /// 元素类型(已建模标量/向量子集)。
    pub ty: MirIoType,
    /// 方向(in|out)。
    pub dir: IoDir,
}

/// 资源种类轴(G2.3 绑定布局推导,RXS-0164;RFC-0005 §9 Q-Space=B 按资源种类分轴)。
///
/// 仅**数据建模**:把 RXS-0156 资源句柄类型面归类到 D3D12 的四个寄存器轴
/// (CBV→`b` / SRV→`t` / UAV→`u` / Sampler→`s`),供 host 侧 register/space 分配
/// 推导按声明序各轴独立递增。不改既有标量/向量 I/O 路径([`IoSigKind`]/[`MirIoType`])
/// 语义,不接线生产 emit;具体 register/space 数值物理布局属 🔒 ABI 禁区,不在此冻结。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ResourceClass {
    /// constant buffer view → `b` 轴。
    Cbv,
    /// shader resource view(只读纹理 / 只读 structured buffer)→ `t` 轴。
    Srv,
    /// unordered access view(可写 structured buffer 等)→ `u` 轴。
    Uav,
    /// sampler → `s` 轴。
    Sampler,
}

/// 资源句柄类型建模(G2.3 绑定布局推导,RXS-0163;承 RXS-0156 资源句柄类型面)。
///
/// 仅**数据建模**:把着色阶段签名里的资源句柄(`Texture2D<F>` / `Sampler` /
/// constant buffer / structured buffer)归约到绑定布局推导所需的最小信息,供
/// host 侧推导 SPIR-V 资源绑定装饰、register/space 分配与 root signature 形态。
/// 与 [`MirIoType`](标量/向量 I/O)并列、互不影响;纹理访问语义(采样 opcode /
/// 描述符编码 / 缓存 / LOD)属 🔒 禁区,在本层结构上不可达、不建模。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MirResourceType {
    /// `Texture2D<F>`(F = 已建模标量分量类型)→ SRV。
    Texture2D(PrimTy),
    /// `Sampler` → Sampler(RFC-0005 §9 Q-Sampler=B dynamic sampler)。
    Sampler,
    /// constant buffer → CBV。
    ConstantBuffer,
    /// structured buffer:`read_only` → SRV,否则 → UAV。
    StructuredBuffer {
        /// 只读(SRV)vs 可写(UAV)。
        read_only: bool,
    },
}

impl MirResourceType {
    /// 资源种类轴归类(RXS-0164;CBV→b / SRV→t / UAV→u / Sampler→s)。
    pub fn class(&self) -> ResourceClass {
        match self {
            MirResourceType::Texture2D(_) => ResourceClass::Srv,
            MirResourceType::Sampler => ResourceClass::Sampler,
            MirResourceType::ConstantBuffer => ResourceClass::Cbv,
            MirResourceType::StructuredBuffer { read_only: true } => ResourceClass::Srv,
            MirResourceType::StructuredBuffer { read_only: false } => ResourceClass::Uav,
        }
    }
}

/// 资源绑定基数(G2.3 绑定布局推导,RXS-0163;RFC-0005 §9 Q-Bindless=A→RD-018)。
///
/// 本期收敛**有界** descriptor 布局:`One` 单 descriptor、`Bounded(n)` 有界数组
/// (消费 n 个连续寄存器)。`Unbounded` = bindless / unbounded descriptor array,
/// agent 自主裁决 defer 至 RD-018——本层把它建模为**显式不可映射**输入,推导侧以
/// strict-only 占位「6xxx」拒绝(无 fallback),不发明 descriptor heap 编码。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ResourceCount {
    /// 单 descriptor。
    One,
    /// 有界 descriptor 数组(`n` 个连续寄存器;`n >= 1`)。
    Bounded(u32),
    /// unbounded / bindless(RD-018 defer;推导侧 strict-only 拒绝)。
    Unbounded,
}

/// 资源绑定声明元素(G2.3 绑定布局推导输入,RXS-0163)。
///
/// 记录着色阶段签名里单个资源句柄形参的源码名(保名依据,非寄存器号/布局)、
/// 资源类型与基数。**声明序即确定性分配序**:host 侧推导按 `Vec<ResourceBinding>`
/// 的顺序确定性导出 SPIR-V 绑定 / register/space / root signature 形态。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResourceBinding {
    /// 源码形参名(保名依据;非寄存器号/物理布局)。
    pub name: String,
    /// 资源类型(归约到绑定布局推导所需最小信息)。
    pub res: MirResourceType,
    /// 资源基数(单 / 有界数组 / unbounded;RD-018)。
    pub count: ResourceCount,
}

#[derive(Debug)]
pub struct Local {
    pub ty: Ty,
    /// 源码名(temp 为 None;debug info 用)。
    pub name: Option<String>,
    pub span: Span,
    /// `shared let`(M5.3,RXS-0079):device codegen 落 addrspace(3) 模块级 global。
    pub shared: bool,
    /// 数组长度(M5.3;`[T; N]` 的 N,device codegen 定 `[N x T]` 形状)。
    pub array_len: Option<u64>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum StatementKind {
    Assign(Place, Rvalue),
}

/// 位置 = local + 投影链。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Place {
    pub local: LocalIdx,
    pub proj: Vec<ProjElem>,
}

impl Place {
    pub fn local(l: LocalIdx) -> Place {
        Place {
            local: l,
            proj: Vec::new(),
        }
    }
}

/// 借用种类(RXS-0057:共享 `&` / 独占 `&mut`;借用检查数据流输入)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BorrowKind {
    Shared,
    Mut,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjElem {
    Deref,
    /// 字段序(struct 定义序 / 元组位置)。
    Field(u32),
    /// `View`/`ViewMut` 容器索引(M4.2,RXS-0071):base 为地址空间指针,
    /// 按 `index`(usize local)偏移 `getelementptr` 得元素 place;device
    /// codegen 作用面(host MIR 不产出 —— host 数组索引仍报 RX6001)。
    Index(LocalIdx),
    /// enum 变体载荷字段(M3.1 扁平布局:`base` = 该变体首载荷的布局下标,
    /// 见 [`enum_variant_layout`];`field` = 变体内字段序)。
    VariantField {
        variant: DefId,
        base: u32,
        field: u32,
    },
}

#[derive(Clone, Debug)]
pub enum Operand {
    Copy(Place),
    /// 按值消耗非 Copy place(RXS-0053 move 时点;move/init 数据流的输入)。
    Move(Place),
    Const(Const),
}

impl Operand {
    /// 引用的 place(Const 无)。
    pub fn place(&self) -> Option<&Place> {
        match self {
            Operand::Copy(p) | Operand::Move(p) => Some(p),
            Operand::Const(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Const {
    Int(i128, PrimTy),
    Float(f64, PrimTy),
    Bool(bool),
    /// 字符串字面量(codegen 落为 NUL 终止全局常量,M2.3 口径)。
    Str(String),
    Char(char),
    Unit,
}

#[derive(Debug)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnOp, Operand),
    /// `&place` / `&mut place`(RXS-0057:携带借用种类,作为借用检查数据流输入)。
    Ref(BorrowKind, Place),
    /// 数值/bool/char 转换(RXS-0046 合法面;目标类型显式)。
    Cast(Operand, Ty),
    /// struct / 元组构造(operand 按定义序/位置序)。
    Aggregate(Ty, Vec<Operand>),
    /// enum 变体构造(M3.1 扁平布局:tag 落下标 0,载荷自 `base` 起顺排)。
    VariantAggregate {
        ty: Ty,
        tag: u32,
        base: u32,
        ops: Vec<Operand>,
    },
    /// enum 判别读取(i32;match 降级的测试输入)。
    Discriminant(Place),
    /// 纹理采样(G2.4,RXS-0175;RFC-0007 §4.4):对 `texture_local` 指向的
    /// `Texture2D<F>` 句柄、用 `sampler_local` 指向的 `Sampler`、在 `coord`
    /// (`vec2<f32>`)处采样,产 `vec4<F>`。`texture_local`/`sampler_local` 为
    /// **资源句柄形参的 local 下标**(句柄非值,不进 `local_values`;codegen 按
    /// local 名匹配 `resources` 解析 SPIR-V 资源变量)。仅图形=B(`dxil-backend`)
    /// 着色 body 产出;首期显式 LOD 0(规避隐式导数,RFC-0007 §4.6)。
    ResourceSample {
        /// `Texture2D<F>` 句柄形参的 local 下标。
        texture_local: LocalIdx,
        /// `Sampler` 句柄形参的 local 下标。
        sampler_local: LocalIdx,
        /// 归一化 UV 坐标(`vec2<f32>` 值)。
        coord: Operand,
    },
}

#[derive(Debug)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TerminatorKind {
    Goto(BlockIdx),
    /// 条件分支(M2.3 仅 bool 判别:0 → `else_`,其余 → `then`)。
    SwitchBool {
        discr: Operand,
        then: BlockIdx,
        else_: BlockIdx,
    },
    Call {
        target: CallTarget,
        args: Vec<Operand>,
        dest: Place,
        next: BlockIdx,
    },
    /// 析构点(RXS-0055 drop elaboration 产物):若 place 此刻持有所有权,
    /// 执行其 drop 动作(Drop::drop + 字段递归;条件持有经 drop flag 在
    /// elaboration 期降为 SwitchBool 守卫)。codegen 展开为调用序列。
    Drop {
        place: Place,
        next: BlockIdx,
    },
    Return,
    /// 发散语句后的死块封口(`return`/`break` 之后)。
    Unreachable,
}

#[derive(Clone, Debug)]
pub enum CallTarget {
    /// 用户函数(单态化实例经符号名对接)。
    Fn {
        def: DefId,
        symbol: String,
    },
    Builtin(Builtin),
    /// device 线程上下文 intrinsic(M4.2,RXS-0072;`ThreadCtx` 方法 →
    /// NVPTX sreg / barrier intrinsics)。host codegen 不产出。
    DeviceIntrinsic(DeviceIntrinsic),
    /// device 数学 intrinsic(M5.3,RXS-0081/0082;`f32`/`f64` 数学方法 →
    /// 保留的 libdevice 外部符号 `__nv_*`,经 libdevice bc 链接解析)。
    /// host codegen 不产出。
    Libdevice {
        symbol: String,
    },
}

/// enum 扁平布局(M3.1 取舍:`{ i32 tag, 变体0载荷…, 变体1载荷…, … }`,
/// 各变体载荷顺排**不重叠**——以空间换实现简单,无 union/字节级尺寸计算;
/// 紧凑布局登记为已知限制随 M4+ 评估)。返回 (变体, 首载荷布局下标) 列表。
pub fn enum_variant_layout(krate: &crate::hir::Crate, enum_def: DefId) -> Vec<(DefId, u32)> {
    let crate::hir::ItemKind::Enum { variants } = &krate.item(enum_def).kind else {
        return Vec::new();
    };
    let mut base = 1u32; // 0 = tag
    variants
        .iter()
        .map(|v| {
            let cur = base;
            if let crate::hir::ItemKind::Variant { fields } = &krate.item(*v).kind {
                base += fields.len() as u32;
            }
            (*v, cur)
        })
        .collect()
}

/// 单态化实例符号名:`main` 保留;其余 `rx_{名}_{DefId}`,泛型实参追加
/// `__{mangle(ty)}`(COFF 安全字符集)。
pub fn mangle(name: &str, def: DefId, args: &[Ty]) -> String {
    if name == "main" && args.is_empty() {
        return "main".to_owned();
    }
    let mut s = format!("rx_{name}_{}", def.0);
    for a in args {
        s.push_str("__");
        s.push_str(&mangle_ty(a));
    }
    s
}

fn mangle_ty(t: &Ty) -> String {
    match t {
        Ty::Prim(p) => prim_short(*p).to_owned(),
        Ty::Adt(d, args) => {
            let mut s = format!("adt{}", d.0);
            for a in args {
                s.push('_');
                s.push_str(&mangle_ty(a));
            }
            s
        }
        Ty::Tuple(v) if v.is_empty() => "unit".to_owned(),
        Ty::Tuple(v) => {
            let mut s = "tup".to_owned();
            for a in v {
                s.push('_');
                s.push_str(&mangle_ty(a));
            }
            s
        }
        Ty::Ref(t, m) => format!("ref{}_{}", if *m { "m" } else { "" }, mangle_ty(t)),
        Ty::RawPtr(t, m) => format!("ptr{}_{}", if *m { "m" } else { "" }, mangle_ty(t)),
        Ty::Array(t) => format!("arr_{}", mangle_ty(t)),
        Ty::Slice(t) => format!("slc_{}", mangle_ty(t)),
        Ty::FnPtr(..) => "fnptr".to_owned(),
        Ty::Param(i) => format!("p{i}"),
        Ty::Const(n) => format!("c{n}"),
        Ty::Infer(_) | Ty::Err => "err".to_owned(),
    }
}

fn prim_short(p: PrimTy) -> &'static str {
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

// ---------------------------------------------------------------------------
// 文本打印(快照单测与 `--emit=mir` 形态;非稳定面,IR golden 随 M3)
// ---------------------------------------------------------------------------

pub fn pretty(body: &Body, res: &Resolutions) -> String {
    let mut out = String::new();
    let params: Vec<String> = (1..=body.arg_count)
        .map(|i| {
            let l = &body.locals[i];
            format!("_{i}: {}", l.ty.render_plain(res))
        })
        .collect();
    out.push_str(&format!(
        "fn {}({}) -> {} {{\n",
        body.symbol,
        params.join(", "),
        body.ret_ty().render_plain(res)
    ));
    for (i, l) in body.locals.iter().enumerate() {
        let name = l
            .name
            .as_ref()
            .map(|n| format!(" // {n}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "    let _{i}: {};{name}\n",
            l.ty.render_plain(res)
        ));
    }
    for (bi, bb) in body.blocks.iter().enumerate() {
        out.push_str(&format!("bb{bi}:\n"));
        for s in &bb.stmts {
            match &s.kind {
                StatementKind::Assign(p, rv) => {
                    out.push_str(&format!(
                        "    {} = {};\n",
                        print_place(p),
                        print_rvalue(rv, res)
                    ));
                }
            }
        }
        out.push_str(&format!("    {};\n", print_term(&bb.terminator.kind)));
    }
    out.push_str("}\n");
    out
}

fn print_place(p: &Place) -> String {
    let mut s = format!("_{}", p.local.0);
    for e in &p.proj {
        match e {
            ProjElem::Deref => s = format!("(*{s})"),
            ProjElem::Field(i) => s.push_str(&format!(".{i}")),
            ProjElem::Index(l) => s.push_str(&format!("[_{}]", l.0)),
            ProjElem::VariantField { base, field, .. } => {
                s.push_str(&format!(".v[{}+{}]", base, field));
            }
        }
    }
    s
}

fn print_operand(o: &Operand) -> String {
    match o {
        Operand::Copy(p) => print_place(p),
        Operand::Move(p) => format!("move {}", print_place(p)),
        Operand::Const(c) => print_const(c),
    }
}

fn print_const(c: &Const) -> String {
    match c {
        Const::Int(v, p) => format!("const {v}{}", prim_short(*p)),
        Const::Float(v, p) => format!("const {v}{}", prim_short(*p)),
        Const::Bool(b) => format!("const {b}"),
        Const::Str(s) => format!("const {s:?}"),
        Const::Char(c) => format!("const {c:?}"),
        Const::Unit => "const ()".to_owned(),
    }
}

fn print_rvalue(rv: &Rvalue, res: &Resolutions) -> String {
    match rv {
        Rvalue::Use(o) => print_operand(o),
        Rvalue::BinaryOp(op, a, b) => format!(
            "{}({}, {})",
            binop_name(*op),
            print_operand(a),
            print_operand(b)
        ),
        Rvalue::UnaryOp(op, a) => format!("{}({})", unop_name(*op), print_operand(a)),
        Rvalue::Ref(BorrowKind::Shared, p) => format!("&{}", print_place(p)),
        Rvalue::Ref(BorrowKind::Mut, p) => format!("&mut {}", print_place(p)),
        Rvalue::Cast(o, t) => format!("{} as {}", print_operand(o), t.render_plain(res)),
        Rvalue::Aggregate(t, ops) => {
            let parts: Vec<String> = ops.iter().map(print_operand).collect();
            format!("{} {{ {} }}", t.render_plain(res), parts.join(", "))
        }
        Rvalue::VariantAggregate { ty, tag, ops, .. } => {
            let parts: Vec<String> = ops.iter().map(print_operand).collect();
            format!("{}#{tag} {{ {} }}", ty.render_plain(res), parts.join(", "))
        }
        Rvalue::Discriminant(p) => format!("discriminant({})", print_place(p)),
        Rvalue::ResourceSample {
            texture_local,
            sampler_local,
            coord,
        } => format!(
            "sample(_{}, _{}, {})",
            texture_local.0,
            sampler_local.0,
            print_operand(coord)
        ),
    }
}

fn print_term(t: &TerminatorKind) -> String {
    match t {
        TerminatorKind::Goto(b) => format!("goto -> bb{}", b.0),
        TerminatorKind::SwitchBool { discr, then, else_ } => format!(
            "switch({}) -> [true: bb{}, false: bb{}]",
            print_operand(discr),
            then.0,
            else_.0
        ),
        TerminatorKind::Call {
            target,
            args,
            dest,
            next,
        } => {
            let name = match target {
                CallTarget::Fn { symbol, .. } => symbol.clone(),
                CallTarget::Builtin(b) => format!("builtin {}", b.name()),
                CallTarget::DeviceIntrinsic(d) => format!("device {}", d.name()),
                CallTarget::Libdevice { symbol } => format!("libdevice {symbol}"),
            };
            let a: Vec<String> = args.iter().map(print_operand).collect();
            format!(
                "{} = {name}({}) -> bb{}",
                print_place(dest),
                a.join(", "),
                next.0
            )
        }
        TerminatorKind::Drop { place, next } => {
            format!("drop({}) -> bb{}", print_place(place), next.0)
        }
        TerminatorKind::Return => "return".to_owned(),
        TerminatorKind::Unreachable => "unreachable".to_owned(),
    }
}

fn binop_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "Add",
        BinOp::Sub => "Sub",
        BinOp::Mul => "Mul",
        BinOp::Div => "Div",
        BinOp::Rem => "Rem",
        BinOp::BitAnd => "BitAnd",
        BinOp::BitOr => "BitOr",
        BinOp::BitXor => "BitXor",
        BinOp::Shl => "Shl",
        BinOp::Shr => "Shr",
        BinOp::Eq => "Eq",
        BinOp::Ne => "Ne",
        BinOp::Lt => "Lt",
        BinOp::Gt => "Gt",
        BinOp::Le => "Le",
        BinOp::Ge => "Ge",
        BinOp::And => "LazyAnd",
        BinOp::Or => "LazyOr",
    }
}

fn unop_name(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "Neg",
        UnOp::Not => "Not",
        UnOp::Deref => "Deref",
    }
}

// ---------------------------------------------------------------------------
// 单测:I/O 意图签名携带(RXS-0161,R1.1)与默认路径中立性(R1.2/R6.7)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FnColor, ShaderStage};
    use crate::hir::{DefId, PrimTy};
    use crate::span::{Edition, SourceId};

    fn dummy_span() -> Span {
        Span::new(SourceId(0), 0, 0, Edition::Rx0)
    }

    /// 无 body 内容的最小骨架(仅用于验证 `Body` 携带 stage / io_sig 的字段面)。
    fn skeleton(stage: Option<ShaderStage>, io_sig: Vec<IoSigElem>) -> Body {
        Body {
            def: DefId(0),
            symbol: "rx_vs_main".to_owned(),
            color: FnColor::Kernel,
            generic_args: Vec::new(),
            locals: Vec::new(),
            arg_count: 0,
            blocks: Vec::new(),
            span: dummy_span(),
            stage,
            io_sig,
            resources: Vec::new(),
        }
    }

    /// 图形阶段 `Body` 可携带 stage 与逐元素 I/O 意图签名(字段名 / 种类 / 类型 /
    /// 方向四维度全保真),为 B 路保名 by-construction 与校验门提供意图侧依据。
    #[test]
    fn graphics_stage_body_carries_io_signature() {
        let io_sig = vec![
            IoSigElem {
                field_name: "position".to_owned(),
                kind: IoSigKind::Builtin("position".to_owned()),
                ty: MirIoType::Vector(PrimTy::F32, 4),
                dir: IoDir::Out,
            },
            IoSigElem {
                field_name: "color".to_owned(),
                kind: IoSigKind::Interpolate("flat".to_owned()),
                ty: MirIoType::Vector(PrimTy::F32, 4),
                dir: IoDir::Out,
            },
            IoSigElem {
                field_name: "uv".to_owned(),
                kind: IoSigKind::Varying,
                ty: MirIoType::Vector(PrimTy::F32, 2),
                dir: IoDir::In,
            },
        ];
        let body = skeleton(Some(ShaderStage::Vertex), io_sig.clone());

        assert_eq!(body.stage, Some(ShaderStage::Vertex));
        assert_eq!(body.io_sig.len(), 3);

        // builtin 保留源码 builtin 名 + out 方向。
        assert_eq!(body.io_sig[0].field_name, "position");
        assert_eq!(
            body.io_sig[0].kind,
            IoSigKind::Builtin("position".to_owned())
        );
        assert_eq!(body.io_sig[0].ty, MirIoType::Vector(PrimTy::F32, 4));
        assert_eq!(body.io_sig[0].dir, IoDir::Out);

        // interpolate 保留插值限定名。
        assert_eq!(
            body.io_sig[1].kind,
            IoSigKind::Interpolate("flat".to_owned())
        );

        // 普通 varying + in 方向。
        assert_eq!(body.io_sig[2].kind, IoSigKind::Varying);
        assert_eq!(body.io_sig[2].dir, IoDir::In);
        assert_eq!(body.io_sig[2].ty, MirIoType::Vector(PrimTy::F32, 2));
    }

    /// 默认(非着色阶段)`Body` 的 stage 为 `None` 且 io_sig 为空——默认 PTX 路径
    /// 构造行为中立,无任何图形阶段意图携带(R1.2/R6.7 零漂移的字段面保证)。
    #[test]
    fn default_path_body_has_neutral_signature_fields() {
        let body = skeleton(None, Vec::new());
        assert_eq!(body.stage, None);
        assert!(body.io_sig.is_empty());
    }

    /// 标量类型亦在已建模子集内(标量 / 向量两形态均可表示)。
    #[test]
    fn io_sig_supports_scalar_and_vector_types() {
        let scalar = IoSigElem {
            field_name: "depth".to_owned(),
            kind: IoSigKind::Builtin("depth".to_owned()),
            ty: MirIoType::Scalar(PrimTy::F32),
            dir: IoDir::Out,
        };
        assert_eq!(scalar.ty, MirIoType::Scalar(PrimTy::F32));
    }
}
