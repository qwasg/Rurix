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

/// device 线程上下文 intrinsics(M4.2,RXS-0072;`ThreadCtx<DIM>` 的方法 →
/// NVPTX special-register / barrier intrinsics)。DIM=1 作用面(`.x` 维),
/// 完整维度随 M4.3。typeck 在接收者为 `ThreadCtx` lang item 时识别;device
/// codegen 落到 `llvm.nvvm.read.ptx.sreg.*` / `llvm.nvvm.barrier0`。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DeviceIntrinsic {
    /// `thread_index[_x]()` → `tid.x`(block 内线程索引,返回 usize)。
    ThreadIndexX,
    /// `thread_index_y()` → `tid.y`(M5.3,RXS-0072 DIM≥2)。
    ThreadIndexY,
    /// `thread_index_z()` → `tid.z`(M5.3,RXS-0072 DIM=3)。
    ThreadIndexZ,
    /// `block_index[_x]()` → `ctaid.x`(block 索引,返回 usize)。
    BlockIndexX,
    /// `block_index_y()` → `ctaid.y`(M5.3)。
    BlockIndexY,
    /// `block_index_z()` → `ctaid.z`(M5.3)。
    BlockIndexZ,
    /// `block_dim[_x]()` → `ntid.x`(block 维度,返回 usize)。
    BlockDimX,
    /// `block_dim_y()` → `ntid.y`(M5.3)。
    BlockDimY,
    /// `block_dim_z()` → `ntid.z`(M5.3)。
    BlockDimZ,
    /// `global_id[_x]()` → `ctaid.x * ntid.x + tid.x`(全局线程索引,返回 usize)。
    GlobalIdX,
    /// `global_id_y()` → `ctaid.y * ntid.y + tid.y`(M5.3)。
    GlobalIdY,
    /// `global_id_z()` → `ctaid.z * ntid.z + tid.z`(M5.3)。
    GlobalIdZ,
    /// `sync()` → `llvm.nvvm.barrier0`(block barrier,返回 unit;扩展点)。
    Barrier,
}

impl DeviceIntrinsic {
    /// `ThreadCtx` 方法名 → intrinsic(RXS-0072;无后缀 = `.x` 维,`_x`/`_y`/`_z`
    /// 显式取轴 DIM≥2,M5.3)。
    pub fn from_method(name: &str) -> Option<Self> {
        Some(match name {
            "thread_index" | "thread_idx" | "thread_id" | "thread_index_x" => {
                DeviceIntrinsic::ThreadIndexX
            }
            "thread_index_y" => DeviceIntrinsic::ThreadIndexY,
            "thread_index_z" => DeviceIntrinsic::ThreadIndexZ,
            "block_index" | "block_idx" | "block_index_x" => DeviceIntrinsic::BlockIndexX,
            "block_index_y" => DeviceIntrinsic::BlockIndexY,
            "block_index_z" => DeviceIntrinsic::BlockIndexZ,
            "block_dim" | "block_dim_x" => DeviceIntrinsic::BlockDimX,
            "block_dim_y" => DeviceIntrinsic::BlockDimY,
            "block_dim_z" => DeviceIntrinsic::BlockDimZ,
            "global_id" | "global_id_x" => DeviceIntrinsic::GlobalIdX,
            "global_id_y" => DeviceIntrinsic::GlobalIdY,
            "global_id_z" => DeviceIntrinsic::GlobalIdZ,
            "sync" => DeviceIntrinsic::Barrier,
            _ => return None,
        })
    }

    /// 调用该 intrinsic 所需的 `ThreadCtx<DIM>` 最小维数(M5.3;X 轴=1,Y=2,Z=3)。
    pub fn min_dim(self) -> u8 {
        match self {
            DeviceIntrinsic::ThreadIndexY
            | DeviceIntrinsic::BlockIndexY
            | DeviceIntrinsic::BlockDimY
            | DeviceIntrinsic::GlobalIdY => 2,
            DeviceIntrinsic::ThreadIndexZ
            | DeviceIntrinsic::BlockIndexZ
            | DeviceIntrinsic::BlockDimZ
            | DeviceIntrinsic::GlobalIdZ => 3,
            _ => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DeviceIntrinsic::ThreadIndexX => "thread_index",
            DeviceIntrinsic::ThreadIndexY => "thread_index_y",
            DeviceIntrinsic::ThreadIndexZ => "thread_index_z",
            DeviceIntrinsic::BlockIndexX => "block_index",
            DeviceIntrinsic::BlockIndexY => "block_index_y",
            DeviceIntrinsic::BlockIndexZ => "block_index_z",
            DeviceIntrinsic::BlockDimX => "block_dim",
            DeviceIntrinsic::BlockDimY => "block_dim_y",
            DeviceIntrinsic::BlockDimZ => "block_dim_z",
            DeviceIntrinsic::GlobalIdX => "global_id",
            DeviceIntrinsic::GlobalIdY => "global_id_y",
            DeviceIntrinsic::GlobalIdZ => "global_id_z",
            DeviceIntrinsic::Barrier => "sync",
        }
    }

    /// 返回 unit(barrier)还是 usize(索引类)。
    pub fn returns_unit(self) -> bool {
        matches!(self, DeviceIntrinsic::Barrier)
    }
}

/// device 数学函数 intrinsic(M5.3,RXS-0081;f32/f64 初等函数 → libdevice
/// ABI 外部符号 `__nv_<name>`)。typeck 在接收者为 `f32`/`f64` 原生类型且
/// 方法名命中时识别(原生类型无用户 inherent impl,无遮蔽问题);device
/// codegen 落 `call` 到保留的外部 `__nv_*` 符号,经 libdevice bc 链接解析
/// (RXS-0082,clang `-mlink-builtin-bitcode`)。映射目标为精确路径
/// (NVVMReflect ftz=0 / prec-sqrt/div=1,RXS-0081 Dynamic Semantics)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DeviceMathFn {
    Sqrt,
    Rsqrt,
    Cbrt,
    Exp,
    Exp2,
    Ln,
    Log2,
    Log10,
    Sin,
    Cos,
    Tan,
    Floor,
    Ceil,
    Trunc,
    Round,
    Abs,
    Powf,
    Min,
    Max,
    Fma,
}

impl DeviceMathFn {
    /// `f32`/`f64` 方法名 → 数学 intrinsic(RXS-0081;非数学方法返回 None)。
    pub fn from_method(name: &str) -> Option<Self> {
        Some(match name {
            "sqrt" => DeviceMathFn::Sqrt,
            "rsqrt" => DeviceMathFn::Rsqrt,
            "cbrt" => DeviceMathFn::Cbrt,
            "exp" => DeviceMathFn::Exp,
            "exp2" => DeviceMathFn::Exp2,
            "ln" | "log" => DeviceMathFn::Ln,
            "log2" => DeviceMathFn::Log2,
            "log10" => DeviceMathFn::Log10,
            "sin" => DeviceMathFn::Sin,
            "cos" => DeviceMathFn::Cos,
            "tan" => DeviceMathFn::Tan,
            "floor" => DeviceMathFn::Floor,
            "ceil" => DeviceMathFn::Ceil,
            "trunc" => DeviceMathFn::Trunc,
            "round" => DeviceMathFn::Round,
            "abs" | "fabs" => DeviceMathFn::Abs,
            "powf" | "pow" => DeviceMathFn::Powf,
            "min" | "fmin" => DeviceMathFn::Min,
            "max" | "fmax" => DeviceMathFn::Max,
            "fma" => DeviceMathFn::Fma,
            _ => return None,
        })
    }

    /// 方法元数(**含 receiver**):一元=1、二元=2、三元(fma)=3。
    pub fn arity(self) -> usize {
        match self {
            DeviceMathFn::Powf | DeviceMathFn::Min | DeviceMathFn::Max => 2,
            DeviceMathFn::Fma => 3,
            _ => 1,
        }
    }

    /// libdevice 符号基名(不含 `__nv_` 前缀与 f32 后缀);RXS-0081 映射表。
    pub fn nv_base(self) -> &'static str {
        match self {
            DeviceMathFn::Sqrt => "sqrt",
            DeviceMathFn::Rsqrt => "rsqrt",
            DeviceMathFn::Cbrt => "cbrt",
            DeviceMathFn::Exp => "exp",
            DeviceMathFn::Exp2 => "exp2",
            DeviceMathFn::Ln => "log",
            DeviceMathFn::Log2 => "log2",
            DeviceMathFn::Log10 => "log10",
            DeviceMathFn::Sin => "sin",
            DeviceMathFn::Cos => "cos",
            DeviceMathFn::Tan => "tan",
            DeviceMathFn::Floor => "floor",
            DeviceMathFn::Ceil => "ceil",
            DeviceMathFn::Trunc => "trunc",
            DeviceMathFn::Round => "round",
            DeviceMathFn::Abs => "fabs",
            DeviceMathFn::Powf => "pow",
            DeviceMathFn::Min => "fmin",
            DeviceMathFn::Max => "fmax",
            DeviceMathFn::Fma => "fma",
        }
    }

    /// libdevice ABI 符号名(RXS-0081;f32 → `__nv_<base>f`,f64 → `__nv_<base>`)。
    pub fn nv_symbol(self, is_f32: bool) -> String {
        format!("__nv_{}{}", self.nv_base(), if is_f32 { "f" } else { "" })
    }
}

/// device views 算子(M5.1,RXS-0078;`View`/`ViewMut` 族子 view 划分方法)。
/// typeck 在接收者为 `View`/`ViewMut` lang item 时识别返回类型,views 不相交
/// device 借用扩展 pass([`crate::views_check`])消费同一识别面判定不相交性。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ViewOp {
    /// `split_at(mid)` → 产 [0, mid) 与 [mid, len) 两个结构性不相交子 view。
    SplitAt,
    /// `chunks(n)` → 块大小 n 的不重叠子 view 序列(尾块容许 < n)。
    Chunks,
    /// `windows(n)` → 大小 n、步长 1 的滑动窗口(相邻窗口重叠)。
    Windows,
}

impl ViewOp {
    /// `View`/`ViewMut` 方法名 → views 算子(RXS-0078;非划分算子返回 None)。
    pub fn from_method(name: &str) -> Option<Self> {
        Some(match name {
            "split_at" => ViewOp::SplitAt,
            "chunks" => ViewOp::Chunks,
            "windows" => ViewOp::Windows,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ViewOp::SplitAt => "split_at",
            ViewOp::Chunks => "chunks",
            ViewOp::Windows => "windows",
        }
    }
}

/// 宿主 GPU 编排已知操作(MS1.2,RXS-0189~0191;MS1.2b,RXS-0197~0199;
/// RFC-0009 §4.1/§4.2/§4.6/§4.7)。typeck 在接收者为 std::gpu / present lang item
/// 句柄(`Context`/`Stream`/`Buffer`/`PinnedBuffer`/`Present`/`Ready`/`Acquired`/
/// `Presentable`)且方法名命中编译器已知签名时识别(用户同名 impl 优先遮蔽);
/// tbir/mir_build 消费,降级为 `rxrt_*`/`rxp_*`/`rxio_*` 字面符号调用(RXS-0194)。
/// 着色合法性:仅 host 上下文合法,kernel/device 内出现 → RX3015(coloring 层,
/// RXS-0189;present 系与 `write_ppm` 同识别面,RXS-0197/0199)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GpuHostOp {
    /// `Context::create()` → `rxrt_ctx_create(@__rx_gpu_artifacts)`(RXS-0192)。
    CtxCreate,
    /// `ctx.stream()` → `rxrt_stream_create`。
    CtxStream,
    /// `ctx.alloc(n)` → `rxrt_buf_alloc(ctx, n * sizeof(T))`。
    CtxAlloc,
    /// `ctx.alloc_pinned(n)` → `rxrt_pinned_alloc(ctx, n * sizeof(T))`。
    CtxAllocPinned,
    /// `ctx.sync()` → `rxrt_ctx_sync`。
    CtxSync,
    /// `buf.upload(&pinned)` → `rxrt_pinned_ptr`/`rxrt_pinned_len` + `rxrt_buf_upload`。
    BufUpload,
    /// `buf.download(&mut pinned)` → 同上 + `rxrt_buf_download`。
    BufDownload,
    /// `buf.len()` → `rxrt_buf_len / sizeof(T)`。
    BufLen,
    /// `pinned.get(i)` → `rxrt_pinned_ptr` + 越界检查 + 元素读(RXS-0191)。
    PinnedGet,
    /// `pinned.set(i, v)` → 同上 + 元素写。
    PinnedSet,
    /// `pinned.len()` → `rxrt_pinned_len / sizeof(T)`。
    PinnedLen,
    /// `stream.sync()` → `rxrt_stream_sync`。
    StreamSync,
    /// `stream.launch(kernel, GridDim, BlockDim, (args..))` → `rxrt_launch`
    /// (🔒 slot+kinds marshalling,RXS-0191)。
    Launch,
    /// `Present::create(&ctx, rw, rh, ww, wh)` → `rxp_create`(MS1.2b,RXS-0197)。
    PresentCreate,
    /// `sess.ready()` → 纯类型面转移 `Present → Ready`(消费 self,不落运行时
    /// 符号,RXS-0197)。
    PresentReady,
    /// `ready.wait()` → `rxp_wait`(消费 self,`Ready → Acquired`;fence acquire
    /// 步引用 RXS-0142,RXS-0197)。
    PresentWait,
    /// `acq.backbuffer()` → `rxp_backbuffer`(借用句柄 `Buffer<C, f32>`,
    /// RXS-0198)。
    PresentBackbuffer,
    /// `acq.signal()` → `rxp_signal`(消费 self,`Acquired → Presentable`,
    /// RXS-0197)。
    PresentSignal,
    /// `pres.pump()` → `rxp_pump`(非消费;负值 → 终止,0/1 → bool 关闭请求,
    /// RXS-0197)。
    PresentPump,
    /// `pres.present()` → `rxp_present`(消费 self,`Presentable → Ready`,
    /// RXS-0197)。
    PresentPresent,
    /// `write_ppm(path, w, h, &pinned)` → `rxio_write_ppm`(宿主图像落盘桥,
    /// RXS-0114~0117 语义 0-byte 复用,RXS-0199)。
    WritePpm,
}

/// scoped atomics 原子读改写算子(M5.2,RXS-0080;`Atomic`/`AtomicView` 族方法)。
/// typeck 在接收者为 `Atomic`/`AtomicView` lang item 时识别,裁决 scope 类型契约
/// (RX3010);PTX `atom.{order}.{scope}` 映射为 D-406 / RD-008 高敏面(deferred),
/// agent 可落笔、agent 自主落地(本枚举仅服务类型契约识别面,不承载映射语义)。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AtomicOp {
    FetchAdd,
    FetchMax,
    FetchMin,
    FetchAnd,
    FetchOr,
    Exchange,
    CompareExchange,
}

impl AtomicOp {
    /// `Atomic`/`AtomicView` 方法名 → 原子算子(RXS-0080;非原子算子返回 None)。
    pub fn from_method(name: &str) -> Option<Self> {
        Some(match name {
            "fetch_add" => AtomicOp::FetchAdd,
            "fetch_max" => AtomicOp::FetchMax,
            "fetch_min" => AtomicOp::FetchMin,
            "fetch_and" => AtomicOp::FetchAnd,
            "fetch_or" => AtomicOp::FetchOr,
            "exchange" => AtomicOp::Exchange,
            "compare_exchange" => AtomicOp::CompareExchange,
            _ => return None,
        })
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
    /// `#[derive(Copy)]` 标注的 ADT → 属性 span(RXS-0053;合法性由
    /// typeck 定义处检查裁决,RX2008)。
    pub copy_derives: std::collections::HashMap<DefId, Span>,
    /// 识别出的 `impl Drop for T`(RXS-0055;trait 路径绑定到内建 Drop 的
    /// impl;形状合法性由 typeck 定义处检查裁决,RX2009)。
    pub drop_impls: Vec<DropImpl>,
}

/// `impl Drop for T` 登记(RXS-0055 最小识别面)。
#[derive(Debug)]
pub struct DropImpl {
    /// impl 目标(self_res 为 struct/enum Def 时 Some;其余形态留 None,
    /// 由定义处检查报 RX2009)。
    pub adt: Option<DefId>,
    /// impl 合成 item 的 DefId。
    pub impl_def: DefId,
    pub span: Span,
}

impl Crate {
    pub fn item(&self, id: DefId) -> &Item {
        &self.items[id.0 as usize]
    }

    pub fn body(&self, id: BodyId) -> &Body {
        &self.bodies[id.0 as usize]
    }

    /// ADT 是否携带 `#[derive(Copy)]`(RXS-0053)。
    pub fn has_copy_derive(&self, def: DefId) -> bool {
        self.copy_derives.contains_key(&def)
    }

    /// ADT 的 Drop impl 登记(首个;重复登记由 RX2009 拒绝)。
    pub fn drop_impl_of(&self, adt: DefId) -> Option<&DropImpl> {
        self.drop_impls.iter().find(|di| di.adt == Some(adt))
    }

    /// ADT 的 `Drop::drop` 关联函数 DefId(RXS-0055;形状违例时可能缺失)。
    pub fn drop_fn_of(&self, adt: DefId) -> Option<DefId> {
        let di = self.drop_impl_of(adt)?;
        let ItemKind::Impl { items, .. } = &self.item(di.impl_def).kind else {
            return None;
        };
        items.iter().copied().find(|d| self.item(*d).name == "drop")
    }

    /// 是否为 Drop impl 的 `drop` 关联函数(方法查找排除面,RXS-0055
    /// "不可显式调用")。
    pub fn is_drop_fn(&self, def: DefId) -> bool {
        if self.item(def).name != "drop" {
            return false;
        }
        self.drop_impls.iter().any(|di| {
            matches!(
                &self.item(di.impl_def).kind,
                ItemKind::Impl { items, .. } if items.contains(&def)
            )
        })
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
        /// `impl Trait for Type` 的 trait 路径解析结果(inherent impl 为 None;
        /// RXS-0055 Drop 识别面消费)。
        trait_res: Option<Res>,
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

/// `self` 接收者形态(RXS-0046;TBIR 方法糖显式化的 autoref/autoderef 依据)。
#[derive(Clone, Copy, Debug)]
pub struct SelfKind {
    pub by_ref: bool,
    pub mutable: bool,
}

#[derive(Debug)]
pub struct FnDecl {
    pub color: FnColor,
    /// 着色阶段标记(RXS-0153);`None` = 普通函数。着色阶段函数 `color` 取
    /// [`FnColor::Kernel`],`stage` 记录阶段类别——device codegen 收集排除着色阶段
    /// 根(本 PR 仅类型面),着色阶段类型面检查在 AST 层(crate::shader_stages)。
    pub stage: Option<crate::ast::ShaderStage>,
    /// 泛型参数名(序号即 `Res::GenericParam` 索引)。
    pub generic_params: Vec<String>,
    pub params: Vec<Param>,
    /// `self` 接收者形态(params[0] 为 self 时 Some)。
    pub self_kind: Option<SelfKind>,
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
        /// 静态数组长度字面量的 span(M5.3:整数字面量长度;非字面量/const 泛型
        /// 长度为 `None`,落 RD-007)。`Ty::Array` 不携长度(语义层),长度仅供
        /// device shared/array codegen 定 `[N x T]` 形状(RXS-0071/0079);取值在
        /// MIR lowering 经源文本解析(那里有 `QueryCtx::src`)。
        len: Option<crate::span::Span>,
    },
    Slice(Box<Ty>),
    FnPtr {
        params: Vec<Ty>,
        ret: Option<Box<Ty>>,
    },
    Infer,
    /// 类型位置整数字面量 const 实参(M5.3 review fix:保留 `ThreadCtx<DIM>` 等
    /// 的 DIM 字面量 span;语义层经 typeck 解析为 [`Ty::Const`])。
    ConstLit {
        span: crate::span::Span,
    },
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
    /// 字面量模式(载荷保留供 MIR 模式测试取值,M3.1)。
    Lit {
        negated: bool,
        lit: crate::ast::Lit,
    },
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
