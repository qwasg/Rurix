//! TBIR — typed body IR(07 §1 第三层 / D-202 窄门;M3.1 实体化)。
//!
//! 层职责(RXS-0048 ~ RXS-0052 实现要求):
//! - **全节点显式类型**:每个表达式/模式携带 [`Ty`](来自 typeck 物化结果,
//!   未单态化——`Ty::Param` 由 MIR lowering 按实例代入);
//! - **方法糖显式化**:`recv.m(args)` 改写为显式 [`ExprKind::Call`]
//!   (receiver 作首实参,按 `self` 形态 autoref/autoderef,RXS-0046);
//!   字段名/构造字段重排为**定义序下标**;一层 autoderef 落为显式 deref;
//! - **drop scope 显式化**(RXS-0052):body 携带 scope 树
//!   ([`Body::scopes`]),局部归属其声明 scope(语句级临时 scope 随 M3.2
//!   drop elaboration 追加);
//! - **模式穷尽性检查时点**(RXS-0051):TBIR 构造期(typeck 后、MIR 前)。
//!
//! 生存期纪律(D-202 峰值内存):TBIR 逐 body 即建即用,MIR 构造后立即释放,
//! 不进 query memo,不驻留全程。

use crate::ast::{self, BinOp, UnOp};
use crate::hir::{DefId, LocalId};
use crate::span::Span;
use crate::ty::Ty;

/// drop scope id([`Body::scopes`] 下标;0 = body 根 scope)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ScopeId(pub u32);

/// drop scope 节点(RXS-0052:函数体根 + 块嵌套)。
#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Body {
    pub owner: DefId,
    /// 局部声明表(LocalId 与 HIR body 对齐;类型已物化)。
    pub locals: Vec<LocalDecl>,
    pub params: Vec<Pat>,
    pub value: Expr,
    /// drop scope 树(RXS-0052;`ScopeId(0)` = 根)。
    pub scopes: Vec<Scope>,
}

#[derive(Debug)]
pub struct LocalDecl {
    pub name: String,
    pub mutable: bool,
    pub ty: Ty,
    pub span: Span,
    /// 归属 scope(声明所在块;参数归根,RXS-0052)。
    pub scope: ScopeId,
    /// `shared let` 声明(M5.3,RXS-0079;addrspace 3 block 共享存储)。
    pub shared: bool,
    /// 数组长度字面量 span(M5.3;`[T; N]` 标注的 N 字面量,device codegen 定形)。
    pub array_len: Option<Span>,
}

#[derive(Debug)]
pub struct Expr {
    pub ty: Ty,
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug)]
pub enum ExprKind {
    Lit(ast::Lit),
    /// desugar 合成整数(RXS-0049 推进步;HIR 同名节点透传)。
    SynthInt(i128),
    Local(LocalId),
    /// const/static/fn 值引用(M3.1 MIR 作用面外,RX6001)。
    Def(DefId),
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Borrow {
        mutable: bool,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Assign {
        op: Option<BinOp>,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// 转换(目标类型 = 自身 `ty`)。
    Cast(Box<Expr>),
    /// 独立区间表达式(作用面外;`for` 区间已在 lower 层 desugar)。
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
    },
    /// 显式直调(方法糖已显式化:receiver 为 args[0];builtin 同通道)。
    Call {
        def: DefId,
        generic_args: Vec<Ty>,
        args: Vec<Expr>,
    },
    /// fn 指针间接调用(作用面外)。
    CallIndirect {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// device 线程上下文 intrinsic(M4.2,RXS-0072;`ThreadCtx` 方法 →
    /// sreg/barrier intrinsic,接收者无副作用故不保留)。
    DeviceCall(crate::hir::DeviceIntrinsic),
    /// device 数学 intrinsic(M5.3,RXS-0081;`f32`/`f64` 方法 → libdevice
    /// `__nv_*` 外部符号调用)。`args[0]` 为 receiver,后续为方法实参。
    DeviceMathCall {
        op: crate::hir::DeviceMathFn,
        is_f32: bool,
        args: Vec<Expr>,
    },
    /// 纹理采样(G2.4,RXS-0174/0175;RFC-0007;`tex.sample(samp, coord)` →
    /// 采样表达式,产 `vec4<F>`)。`texture`/`sampler` 为资源句柄形参引用
    /// (MIR lowering 取其 local 下标);`coord` 为 `vec2<f32>` 值。
    ResourceSample {
        texture: Box<Expr>,
        sampler: Box<Expr>,
        coord: Box<Expr>,
    },
    /// 采样方法族(G3.3,RXS-0223/0226;RFC-0013 §4.B1):新方法(`sample_lod`/
    /// `sample_grad`/`sample_bias`/`load`/`load_lod`/`sample_cmp`/`gather`/
    /// `TextureRw2D` load·store)调用 → MIR `Rvalue::ResourceSample{method, extra}`。
    /// 既有 `.sample()` 走 [`ExprKind::ResourceSample`](byte-preserving,
    /// Q-S-SampleName)。`sampler` 仅 sample 族携带(`load`/`store` 无);`extra` =
    /// 方法族额外实参(lod / ddx·ddy / bias / dref / component / store 值)。
    ResourceMethodCall {
        method: crate::mir::ResourceMethod,
        texture: Box<Expr>,
        sampler: Option<Box<Expr>>,
        /// G3.4 bindless(RXS-0232):无界表动态索引 `table[nonuniform(idx)]` 的
        /// 索引值表达式(`Some` = 无界表元素采样;`None` = 单句柄采样)。`texture`
        /// 此时为 `[Texture2D<F>]` 无界表句柄形参引用。
        table_index: Option<Box<Expr>>,
        coord: Box<Expr>,
        extra: Vec<Expr>,
    },
    /// 宿主 GPU 编排调用(MS1.2,RXS-0189/0191):`args[0]` 为 receiver 句柄
    /// (`CtxCreate` 无 receiver,args 为空);upload/download 的 `&pinned` 借用
    /// 实参已剥壳为句柄表达式。MIR lowering 降级为 `rxrt_*` 字面符号调用
    /// (RXS-0194)+ 失败终止检查(RXS-0193)。
    GpuCall {
        op: crate::hir::GpuHostOp,
        args: Vec<Expr>,
    },
    /// launch 宿主 lowering(MS1.2,RXS-0191):kernel 编译期绑定(device MIR
    /// 同源 mangle 符号)+ 维度分量(缺轴补 1)+ 实参元组(🔒 slot+kinds
    /// marshalling 由 MIR lowering 物化)。
    GpuLaunch {
        stream: Box<Expr>,
        kernel: DefId,
        grid: Vec<Expr>,
        block: Vec<Expr>,
        args: Vec<Expr>,
    },
    /// 字段访问(字段名已解析为定义序/元组位置下标)。
    Field {
        base: Box<Expr>,
        index: u32,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    Repeat {
        elem: Box<Expr>,
        len: Box<Expr>,
    },
    /// struct / 元组结构体 / enum 变体构造(字段已按定义序重排齐全)。
    Aggregate {
        def: DefId,
        fields: Vec<Expr>,
    },
    Block(Block),
    If {
        cond: Box<Expr>,
        then: Block,
        else_: Option<Box<Expr>>,
    },
    While {
        cond: Box<Expr>,
        body: Block,
    },
    Loop {
        body: Block,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    Return(Option<Box<Expr>>),
    Break,
    /// 带值 break(作用面外)。
    BreakValue(Box<Expr>),
    Continue,
    /// 闭包(作用面外;不保留内部结构)。
    Closure,
    Err,
}

#[derive(Debug)]
pub struct Block {
    /// 本块的 drop scope(RXS-0052)。
    pub scope: ScopeId,
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmt {
    Let { pat: Pat, init: Option<Expr> },
    Expr(Expr),
}

#[derive(Debug)]
pub struct Arm {
    pub pats: Vec<Pat>,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug)]
pub struct Pat {
    pub ty: Ty,
    pub span: Span,
    pub kind: PatKind,
}

#[derive(Debug)]
pub enum PatKind {
    Wild,
    /// 绑定(`x` / `x @ p`,sub = `@` 子模式)。
    Binding {
        local: LocalId,
        sub: Option<Box<Pat>>,
    },
    Lit {
        negated: bool,
        lit: ast::Lit,
    },
    /// 区间模式(作用面外;穷尽性按"需通配兜底"裁决,RXS-0051)。
    Range,
    /// 引用模式(解引用后匹配;HIR `Ref` 模式显式化)。
    Deref(Box<Pat>),
    Tuple(Vec<Pat>),
    Slice(Vec<Pat>),
    /// struct / 元组结构体模式(字段名已解析为定义序下标)。
    Struct {
        def: DefId,
        fields: Vec<(u32, Pat)>,
    },
    /// enum 变体模式(判别下标 = 变体在 enum 定义序中的位置)。
    Variant {
        enum_def: DefId,
        variant: DefId,
        index: u32,
        fields: Vec<(u32, Pat)>,
    },
    Err,
}
