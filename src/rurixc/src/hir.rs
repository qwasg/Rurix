//! HIR(spec 条款 RXS-0032 实现要求;07 §1 / D-202)。
//!
//! 层职责:类型系统的主工作 IR——**item 与 body 分离**,为增量提供依赖边界;
//! 路径节点携带名称解析结果 [`Res`](不再有文本回溯);不做借用检查(MIR 职责)。
//!
//! M3.1 起 `for` / `?` 不再是 HIR 节点:AST→HIR lowering 按 RXS-0049/RXS-0050
//! 展开为 loop+match / match 等价形式(依赖 RXS-0048 编译器已知项最小面,
//! M2_PLAN v1.1/v1.2 推迟项收口);合成推进步以 [`ExprKind::SynthInt`] 表示
//! (无源文本支撑的字面量)。关联项路径仅支持 inherent impl(M2.1 取舍延续)。

use crate::ast::{BinOp, FnColor, UnOp};
use crate::span::Span;

/// 定义 id(分配制递增,模块树收集序)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DefId(pub u32);

/// body id(`Crate::bodies` 索引)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BodyId(pub u32);

/// body 内局部绑定 id(per-body 自增)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocalId(pub u32);

/// HIR 节点 id(crate 内全局递增,lowering 分配;[`crate::typeck::TypeckResults`]
/// 的键——M2.3 起类型检查结果按节点物化,供 MIR lowering 消费)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HirId(pub u32);

/// 内置原生类型(类型位置单段路径的保留名,RXS-0034)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PrimTy {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    F64,
    Bool,
    Char,
    Str,
}

impl PrimTy {
    pub fn from_name(name: &str) -> Option<Self> {
        use PrimTy::*;
        Some(match name {
            "i8" => I8,
            "i16" => I16,
            "i32" => I32,
            "i64" => I64,
            "u8" => U8,
            "u16" => U16,
            "u32" => U32,
            "u64" => U64,
            "usize" => Usize,
            "f32" => F32,
            "f64" => F64,
            "bool" => Bool,
            "char" => Char,
            "str" => Str,
            _ => return None,
        })
    }
}

/// 内建函数(M2.3 最小 prelude:无标准库形态下的 hello-world 闭环;
/// 库化/lang-item 体系随 M3+)。用户同名定义优先(resolve 兜底查找)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Builtin {
    /// `println(s: &str)`:行输出(host codegen 落到 CRT `puts`)。
    Println,
}

impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Println => "println",
        }
    }
}

/// 名称解析结果(RXS-0034 裁决产物)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Res {
    /// body 内局部绑定。
    Local(LocalId),
    /// 模块项 / 关联项 / enum 变体。
    Def(DefId),
    /// 当前 item 的第 n 个泛型参数。
    GenericParam(u32),
    /// 内置原生类型。
    PrimTy(PrimTy),
    /// impl 体内的 `Self` 类型。
    SelfTy,
    /// M2.1 容忍区:类型位置未知路径(草图类型,M2.2 typeck 裁决)与错误恢复。
    Err,
}

/// 定义类别(诊断与命名空间归属用)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DefKind {
    Mod,
    Fn,
    Struct,
    Enum,
    Variant,
    Trait,
    TypeAlias,
    Const,
    Static,
    AssocFn,
    AssocConst,
    AssocType,
    Field,
}

/// 可见性(RXS-0036;MVP 单 package:Pub 与 PubPackage 在包内等效可见)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vis {
    Private,
    Pub,
    PubPackage,
}

// ---------------------------------------------------------------------------
// crate / item
// ---------------------------------------------------------------------------

/// HIR 根:item 表(DefId 索引)+ body 表(BodyId 索引)——item/body 分离(D-202)。
#[derive(Debug, Default)]
pub struct Crate {
    pub items: Vec<Item>,
    pub bodies: Vec<Body>,
    /// 根模块的直接子 item。
    pub root_items: Vec<DefId>,
}

impl Crate {
    pub fn item(&self, id: DefId) -> &Item {
        &self.items[id.0 as usize]
    }

    pub fn body(&self, id: BodyId) -> &Body {
        &self.bodies[id.0 as usize]
    }
}

#[derive(Debug)]
pub struct Item {
    pub def_id: DefId,
    pub name: String,
    pub kind: ItemKind,
    pub vis: Vis,
    pub span: Span,
}

#[derive(Debug)]
pub enum ItemKind {
    Fn(FnDecl),
    Struct {
        fields: Vec<FieldDef>,
    },
    Enum {
        variants: Vec<DefId>,
    },
    Variant {
        fields: Vec<FieldDef>,
    },
    Trait {
        items: Vec<DefId>,
    },
    Impl {
        self_res: Res,
        items: Vec<DefId>,
    },
    Mod {
        items: Vec<DefId>,
    },
    /// `use`:HIR 保留已解析目标与导出名(RXS-0035 实现要求)。
    Use {
        target: Res,
    },
    Const {
        ty: Ty,
        body: BodyId,
    },
    Static {
        mutable: bool,
        ty: Ty,
        body: BodyId,
    },
    TypeAlias {
        ty: Ty,
    },
    AssocType,
    /// 解析/降级错误占位。
    Err,
}

#[derive(Debug)]
pub struct FnDecl {
    pub color: FnColor,
    /// 泛型参数名(序号即 `Res::GenericParam` 索引)。
    pub generic_params: Vec<String>,
    pub params: Vec<Param>,
    pub ret: Option<Ty>,
    /// `None` = 签名声明(extern/trait)。
    pub body: Option<BodyId>,
}

#[derive(Debug)]
pub struct Param {
    pub pat: Pat,
    pub ty: Option<Ty>,
    pub span: Span,
}

#[derive(Debug)]
pub struct FieldDef {
    pub name: String,
    pub vis: Vis,
    pub ty: Ty,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// body(item/body 分离的 body 侧)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Body {
    /// 所属 item。
    pub owner: DefId,
    /// 局部绑定声明表(LocalId 索引)。
    pub locals: Vec<LocalDecl>,
    pub params: Vec<Pat>,
    pub value: Expr,
}

#[derive(Debug)]
pub struct LocalDecl {
    pub name: String,
    pub mutable: bool,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// 类型 / 模式 / 表达式(贴近 AST 形态,路径换 Res)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TyKind {
    /// 已解析路径类型 + 类型实参(末段 `<…>` 的 Type 实参,M2.2 起携带)。
    Res(Res, Vec<Ty>),
    Ref {
        mutable: bool,
        inner: Box<Ty>,
    },
    RawPtr {
        mutable: bool,
        inner: Box<Ty>,
    },
    Tuple(Vec<Ty>),
    Array {
        elem: Box<Ty>,
    },
    Slice(Box<Ty>),
    FnPtr {
        params: Vec<Ty>,
        ret: Option<Box<Ty>>,
    },
    Infer,
    Err,
}

#[derive(Debug)]
pub struct Pat {
    pub hir_id: HirId,
    pub kind: PatKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum PatKind {
    Wild,
    Binding {
        local: LocalId,
    },
    Lit,
    Range,
    At {
        local: LocalId,
        pat: Box<Pat>,
    },
    Ref {
        pat: Box<Pat>,
    },
    Tuple(Vec<Pat>),
    Slice(Vec<Pat>),
    /// 单元变体/常量模式(已解析)。
    Res(Res),
    TupleStruct {
        res: Res,
        elems: Vec<Pat>,
    },
    Struct {
        res: Res,
        fields: Vec<(String, Option<Pat>)>,
        rest: bool,
    },
    Err,
}

#[derive(Debug)]
pub struct Expr {
    pub hir_id: HirId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    /// 字面量(种类/后缀供 typeck 定型,RXS-0039;复用 AST 节点)。
    Lit(crate::ast::Lit),
    /// desugar 合成的整数字面量(无源文本支撑;RXS-0049 推进步)。
    /// 定型同无后缀整数字面量(数值类约束 + RXS-0039 默认化)。
    SynthInt(i128),
    /// 已解析路径(变量/常量/单元变体/fn 引用)。
    Res(Res),
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
    Cast {
        expr: Box<Expr>,
        ty: Ty,
    },
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
        inclusive: bool,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// 方法名解析依赖接收者类型,留待 M2.2 typeck(此处保留文本名)。
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Field {
        expr: Box<Expr>,
        field: String,
    },
    /// 元组/元组结构体位置字段。
    TupleField {
        expr: Box<Expr>,
        index: u32,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    Repeat {
        elem: Box<Expr>,
        len: Box<Expr>,
    },
    StructLit {
        res: Res,
        fields: Vec<(String, Option<Expr>)>,
    },
    Block(Block),
    Unsafe(Block),
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
    Break(Option<Box<Expr>>),
    Continue,
    Closure {
        params: Vec<Pat>,
        body: Box<Expr>,
    },
    Err,
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmt {
    /// 嵌套 item 以 DefId 引用(item/body 分离)。
    Item(DefId),
    Let {
        pat: Pat,
        ty: Option<Ty>,
        init: Option<Expr>,
        shared: bool,
    },
    Expr(Expr),
}

#[derive(Debug)]
pub struct Arm {
    pub pats: Vec<Pat>,
    pub guard: Option<Expr>,
    pub body: Expr,
}
