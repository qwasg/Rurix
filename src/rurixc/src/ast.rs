//! AST(spec 条款 RXS-0011 ~ RXS-0029,spec/syntax.md)。
//!
//! 层职责(D-202,07 §1):贴近用户写下的语法,不做类型/数据流;
//! 全部节点携带 [`Span`]。名称以源文本切片存为 `String`(MVP 取舍,
//! intern 通道随 M2 名称解析评估)。

use crate::span::Span;

/// 标识符(RXS-0004;含上下文关键字按标识符产出的场合)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// 生命周期标记(RXS-0008),`name` 不含前导 `'`。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Lifetime {
    pub name: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// 属性(RXS-0012)
// ---------------------------------------------------------------------------

/// `#[meta]`(外部)或 `#![meta]`(内部)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Attr {
    pub inner: bool,
    pub meta: MetaItem,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MetaItem {
    pub path: Path,
    pub kind: MetaKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MetaKind {
    /// `#[allow]`
    Path,
    /// `#[derive(Copy, Clone)]` / `#[link(name = "x")]`
    List(Vec<MetaInner>),
    /// `#[key = "value"]`
    NameValue(Lit),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MetaInner {
    Meta(MetaItem),
    Lit(Lit),
}

// ---------------------------------------------------------------------------
// 路径与可见性(RXS-0013)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Path {
    pub segments: Vec<PathSegment>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PathSegment {
    pub ident: Ident,
    /// 类型位置直接 `<…>`;表达式位置经 turbofish `::<…>`(RXS-0013)。
    pub args: Option<GenericArgs>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Visibility {
    /// 无 `pub`。
    Inherited,
    Pub(Span),
    /// `pub(package)`(05 §10)。
    PubPackage(Span),
}

// ---------------------------------------------------------------------------
// 泛型(RXS-0020 / RXS-0021)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Generics {
    pub params: Vec<GenericParam>,
    pub where_preds: Vec<WherePred>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GenericParam {
    pub kind: GenericParamKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GenericParamKind {
    Lifetime(Lifetime),
    Type {
        name: Ident,
        bounds: Vec<Bound>,
        default: Option<Ty>,
    },
    Const {
        name: Ident,
        ty: Ty,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WherePred {
    pub ty: Ty,
    pub bounds: Vec<Bound>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Bound {
    Lifetime(Lifetime),
    /// trait bound(路径形态;trait 角色由名称解析层裁决)。
    Trait(Path),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GenericArgs {
    pub args: Vec<GenericArg>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GenericArg {
    Lifetime(Lifetime),
    Type(Ty),
    /// `{ expr }` / `-1` 形态的 const 实参(裸整数字面量经 TyKind::ConstArg 覆盖)。
    Const(Expr),
}

// ---------------------------------------------------------------------------
// item(RXS-0011, RXS-0014 ~ RXS-0019)
// ---------------------------------------------------------------------------

/// 源文件根节点(RXS-0011)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceFile {
    pub attrs: Vec<Attr>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Item {
    pub attrs: Vec<Attr>,
    pub vis: Visibility,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Fn(FnItem),
    Struct(StructItem),
    Enum(EnumItem),
    Trait(TraitItem),
    Impl(ImplItem),
    Mod(ModItem),
    Use(UseItem),
    Static(StaticItem),
    Const(ConstItem),
    TypeAlias(TypeAlias),
    ExternBlock(ExternBlock),
    /// 错误恢复占位(RXS-0030)。
    Err,
}

/// 函数着色(RXS-0014,D-102)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FnColor {
    Host,
    Kernel,
    Device,
    Const,
}

/// 着色阶段类别(RXS-0153,spec/shader_stages.md;RFC-0002 §9 Q1 前缀式
/// `<stage> fn`)。着色阶段是 kernel 着色的扩展(新 coloring 类别):它们与
/// `kernel fn` 同享"非直接可调用入口 + 设备上下文体"的着色语义(着色检查复用
/// kernel 规则,见 [`crate::coloring`]),`compute` 阶段在 D3D12 语境直接复用既有
/// kernel 着色(Q1)。`stage` 标记仅用于着色阶段专属类型面检查
/// (I/O 标注 / 阶段间接口 / 资源句柄,见 `crate::shader_stages`)与 device codegen
/// 收集排除(本 PR 仅类型面,着色阶段不进 PTX 收集根)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Mesh,
    Task,
    RayGen,
    ClosestHit,
    AnyHit,
    Miss,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FnItem {
    pub color: FnColor,
    /// 着色阶段标记(RXS-0153);`None` = 普通 host/kernel/device/const 函数。
    /// 着色阶段函数的 `color` 取 [`FnColor::Kernel`](入口着色语义),`stage`
    /// 记录具体阶段类别供着色阶段类型面检查消费(RFC-0002 §9 Q1)。
    pub stage: Option<ShaderStage>,
    pub name: Ident,
    pub generics: Generics,
    pub params: Vec<Param>,
    pub ret: Option<Ty>,
    /// `None` = 签名声明(trait 体/extern 块,RXS-0014)。
    pub body: Option<Block>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Param {
    pub attrs: Vec<Attr>,
    pub kind: ParamKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParamKind {
    /// `self` / `&self` / `&'a mut self` / `mut self`。
    SelfParam {
        by_ref: bool,
        lifetime: Option<Lifetime>,
        mutable: bool,
    },
    Typed {
        pat: Pat,
        ty: Ty,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StructItem {
    pub name: Ident,
    pub generics: Generics,
    pub body: VariantBody,
}

/// struct 体 / enum 变体体共用形态(RXS-0015)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VariantBody {
    Named(Vec<FieldDef>),
    Tuple(Vec<TupleField>),
    Unit,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldDef {
    pub attrs: Vec<Attr>,
    pub vis: Visibility,
    pub name: Ident,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TupleField {
    pub attrs: Vec<Attr>,
    pub vis: Visibility,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EnumItem {
    pub name: Ident,
    pub generics: Generics,
    pub variants: Vec<Variant>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant {
    pub attrs: Vec<Attr>,
    pub name: Ident,
    pub body: VariantBody,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TraitItem {
    pub name: Ident,
    pub generics: Generics,
    pub items: Vec<AssocItem>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ImplItem {
    pub generics: Generics,
    /// `impl Trait for Type` 的 `Trait`(无 `for` 时为 `None`,固有 impl)。
    pub trait_ty: Option<Ty>,
    pub self_ty: Ty,
    pub items: Vec<AssocItem>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssocItem {
    pub attrs: Vec<Attr>,
    pub vis: Visibility,
    pub kind: AssocItemKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AssocItemKind {
    Fn(FnItem),
    /// `type Name (: bounds)? (= ty)? ;`(RXS-0016)。
    Type {
        name: Ident,
        bounds: Vec<Bound>,
        default: Option<Ty>,
    },
    Const(ConstItem),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ModItem {
    pub name: Ident,
    pub items: Vec<Item>,
    /// `mod name;` out-of-line 形态(RXS-0196):parser 阶段 `items` 为空,由
    /// driver 装配 pass([`crate::mod_assembly`])按「当前文件同目录 `name.rx`」
    /// 加载后回填为内联 mod 等价形态(缺失/IO/循环 → RX1005)。
    pub out_of_line: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UseItem {
    pub path: Path,
    pub alias: Option<Ident>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StaticItem {
    pub mutable: bool,
    pub name: Ident,
    pub ty: Ty,
    pub init: Expr,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConstItem {
    pub name: Ident,
    pub ty: Ty,
    pub init: Expr,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypeAlias {
    pub name: Ident,
    pub generics: Generics,
    pub ty: Ty,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternBlock {
    /// ABI 字符串字面量(首批 `"C"`,RXS-0019)。
    pub abi: String,
    pub abi_span: Span,
    pub items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// 类型(RXS-0022)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TyKind {
    Path(Path),
    Ref {
        lifetime: Option<Lifetime>,
        mutable: bool,
        inner: Box<Ty>,
    },
    RawPtr {
        mutable: bool,
        inner: Box<Ty>,
    },
    /// `()` 为空向量;单元素必带尾逗号(RXS-0022)。
    Tuple(Vec<Ty>),
    Paren(Box<Ty>),
    Array {
        elem: Box<Ty>,
        len: Box<Expr>,
    },
    Slice(Box<Ty>),
    FnPtr {
        params: Vec<Ty>,
        ret: Option<Box<Ty>>,
    },
    /// `_`
    Infer,
    /// 类型位置整数字面量(shape 元组等 const 实参形态,RXS-0021/0022)。
    ConstArg(Lit),
    Err,
}

// ---------------------------------------------------------------------------
// 模式(RXS-0023)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PatKind {
    Wild,
    Binding {
        mutable: bool,
        name: Ident,
    },
    Lit {
        /// 前导 `-`(仅数值字面量)。
        negated: bool,
        lit: Lit,
    },
    Range {
        lo: Box<Pat>,
        hi: Box<Pat>,
        inclusive: bool,
    },
    At {
        name: Ident,
        pat: Box<Pat>,
    },
    Ref {
        mutable: bool,
        pat: Box<Pat>,
    },
    Tuple(Vec<Pat>),
    Slice(Vec<Pat>),
    /// 多段路径(单元变体/常量;单段小写在名称解析层重分类为绑定)。
    Path(Path),
    TupleStruct {
        path: Path,
        elems: Vec<Pat>,
    },
    Struct {
        path: Path,
        fields: Vec<FieldPat>,
        /// 尾部 `..`。
        rest: bool,
    },
    Err,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldPat {
    pub name: Ident,
    /// `None` = 简写(`{ pos }` ≡ `{ pos: pos }`)。
    pub pat: Option<Pat>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// 字面量
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Lit {
    pub kind: LitKind,
    /// 字面量后缀(`1f32` / `255u8`;无后缀经推断定型,RXS-0039)。
    pub suffix: Option<LitSuffix>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LitKind {
    Int,
    Float,
    Str,
    Char,
    Bool(bool),
}

/// 数值字面量后缀(RXS-0006/0007 后缀集的 AST 侧表示)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LitSuffix {
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
}

// ---------------------------------------------------------------------------
// 语句与块(RXS-0024)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// 尾表达式(块的值,无 `;`)。
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StmtKind {
    Item(Box<Item>),
    Let(LetStmt),
    Expr {
        expr: Expr,
        /// 是否以 `;` 终结(块尾语句的分号可省,RXS-0024)。
        semi: bool,
    },
    Empty,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LetStmt {
    /// `shared let`(05 §5,RXS-0024)。
    pub shared: bool,
    pub pat: Pat,
    pub ty: Option<Ty>,
    pub init: Option<Expr>,
}

// ---------------------------------------------------------------------------
// 表达式(RXS-0025 ~ RXS-0029)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Expr {
    /// 前置外部属性(RXS-0026;通常为空)。
    pub attrs: Vec<Attr>,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `!`
    Not,
    /// `*`
    Deref,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Path(Path),
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
    /// `lhs = rhs` / `lhs op= rhs`(`op` 为复合赋值的算子)。
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
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        generic_args: Option<GenericArgs>,
        args: Vec<Expr>,
    },
    Field {
        expr: Box<Expr>,
        field: Ident,
    },
    /// 元组字段访问 `.0`。
    TupleField {
        expr: Box<Expr>,
        index: u32,
        index_span: Span,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    /// `expr?`
    Try(Box<Expr>),
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    Repeat {
        elem: Box<Expr>,
        len: Box<Expr>,
    },
    StructLit {
        path: Path,
        fields: Vec<FieldInit>,
    },
    Paren(Box<Expr>),
    Block(Block),
    Unsafe(Block),
    If {
        cond: Box<Expr>,
        then: Block,
        /// `Block` 或嵌套 `If`。
        else_: Option<Box<Expr>>,
    },
    While {
        cond: Box<Expr>,
        body: Block,
    },
    For {
        pat: Pat,
        iter: Box<Expr>,
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
    /// 闭包(feature gate `closures` 后,RXS-0031)。
    Closure {
        is_move: bool,
        params: Vec<ClosureParam>,
        body: Box<Expr>,
    },
    /// 错误恢复占位(RXS-0030)。
    Err,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClosureParam {
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldInit {
    pub name: Ident,
    /// `None` = 简写。
    pub expr: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Arm {
    pub attrs: Vec<Attr>,
    /// `|` 分隔的顶层 or-模式(RXS-0029)。
    pub pats: Vec<Pat>,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

impl Expr {
    /// 块形态表达式(作语句时分号可省,RXS-0024)。
    pub fn is_block_like(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Block(_)
                | ExprKind::Unsafe(_)
                | ExprKind::If { .. }
                | ExprKind::While { .. }
                | ExprKind::For { .. }
                | ExprKind::Loop { .. }
                | ExprKind::Match { .. }
        )
    }
}
