//! device MIR → DXIL DirectX 三元组 LLVM IR 文本(G2.2 PR-C2 分片1,RXS-0157;
//! RFC-0003 §4.1/§4.2,D-131=A)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate);未启用时整模块
//! 不编入 rurixc,PTX 路径(D-207)不受影响。target 分发在 MIR 之后分叉:DXIL 后端与
//! NVPTX 后端(`device_codegen`)并列、各自从 MIR 独立降级,不共享后端 lowering
//! (RFC-0003 §4.5)。
//!
//! **最小 compute 子集(分片1)**:仅支持 compute 着色入口(`kernel fn`,RXS-0153
//! compute-via-kernel 着色)的最小子集——无 ABI 形参、平凡(空)体 → DXIL `void` 入口
//! (`dxil-unknown-shadermodel6.0-compute` 三元组 + `hlsl.shader`="compute" /
//! `hlsl.numthreads` 入口属性,对齐 LLVM DirectX 后端 emit 形态)。子集外构造
//! (View/资源句柄形参、非平凡体——需绑定布局推导 G2.3 / FFI ABI 禁区)→ `RX6007`。
//!
//! 下游(IR → patched llc -filetype=obj → DXIL 容器 → dxc validator)见
//! [`crate::toolchain::ir_to_dxil`];golden 取文本反汇编经 validator 验证(RFC-0003
//! §9 Q-Golden)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ FFI ABI 二进制布局
//! (RFC-0003 §4.6)/ 绑定布局推导(G2.3,P-11)。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::ast::{FnColor, ShaderStage};
use crate::binding_layout;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::dxil_sig_gate::signature_gate;
use crate::dxil_spirv::{self, DxilError};
use crate::mir::{
    Body, Const, IoDir, IoSigElem, IoSigKind, Operand, ResourceBinding, Rvalue, StatementKind,
    TerminatorKind,
};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::toolchain::{self, DxilSignatures};

/// DXIL codegen 失败(RX6007;目标不可用 / 子集外构造 / 降级失败,RXS-0157 L1~L3)。
#[derive(Debug, Clone)]
pub struct DxilCodegenError {
    pub span: Span,
    pub detail: String,
}

impl DxilCodegenError {
    fn unsupported(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
        }
    }
}

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` 为根)+ DXIL 最小 compute codegen。
/// 无 kernel → `None`(无 device 产物);子集外 / 降级失败 → 经 `cx.diag()` 落
/// `RX6007` 结构化诊断并返回 `None`;成功 → `Some(DirectX 三元组 LLVM IR 文本)`。
/// patched llc → DXIL 容器 + dxc validator 由驱动在产 IR 后另行实施(RXS-0157 IR2)。
pub fn build_and_emit_dxil(cx: &QueryCtx<'_>, module_name: &str) -> Option<String> {
    let bodies = cx.device_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    // device MIR 构建已报错 → 不级联 codegen(防一错多报,对齐 device_codegen)。
    if cx.diag().has_errors() {
        return None;
    }
    // compute 入口 = kernel 着色 body(RXS-0153 compute-via-kernel);取首个为最小入口。
    let entry = bodies.iter().find(|b| b.color == FnColor::Kernel)?;
    match emit_dxil_ir(entry, module_name) {
        Ok(ir) => Some(ir),
        Err(e) => {
            cx.diag()
                .struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                .arg("detail", e.detail.clone())
                .span_label(e.span, "in DXIL compute entry")
                .emit();
            None
        }
    }
}

/// 单个 compute kernel body → DXIL DirectX 三元组 LLVM IR 文本(最小子集)。
/// 子集校验(RXS-0157 L2):无 ABI 形参 + 平凡体(块内零语句,终结子仅 Goto/Return/
/// Unreachable);违例 → `DxilCodegenError`(上层映射 RX6007)。
pub fn emit_dxil_ir(body: &Body, module_name: &str) -> Result<String, DxilCodegenError> {
    if body.arg_count != 0 {
        return Err(DxilCodegenError::unsupported(
            body.span,
            "DXIL 最小 compute 子集暂不支持带形参的 compute 入口(View/资源句柄绑定布局推导属 G2.3,FFI ABI 属禁区)",
        ));
    }
    for bb in &body.blocks {
        for st in &bb.stmts {
            // 最小子集仅容忍隐式 unit 返回赋值(`_0 = ()`,空体语义);其余语句
            // (真实计算 / 内存写 / 调用)需 codegen 降级 + 可能绑定布局,属后续分片。
            let StatementKind::Assign(_, Rvalue::Use(Operand::Const(Const::Unit))) = &st.kind
            else {
                return Err(DxilCodegenError::unsupported(
                    st.span,
                    "DXIL 最小 compute 子集暂不支持非平凡 compute 体(分片1 仅空体入口,语句降级随后续分片)",
                ));
            };
        }
        match bb.terminator.kind {
            TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
            _ => {
                return Err(DxilCodegenError::unsupported(
                    bb.terminator.span,
                    "DXIL 最小 compute 子集暂不支持该控制流终结子(分片1 仅空体入口)",
                ));
            }
        }
    }
    Ok(render_dxil_module(&body.symbol, module_name))
}

/// DirectX 三元组 LLVM IR 文本(最小空体 compute 入口)。形态对齐 LLVM DirectX 后端
/// emit 期望(shadermodel6.0-compute 三元组 + DXIL 数据布局 + `hlsl.shader`/
/// `hlsl.numthreads` 入口属性);经 patched llc -filetype=obj 产 DXIL 容器、dxc
/// validator 接受(round-8 recipe 验证)。numthreads 取最小 `1,1,1`(分片1 无 launch
/// bounds 降级)。确定性:给定符号名输出字节确定。
fn render_dxil_module(entry_symbol: &str, module_name: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "; ModuleID = '{module_name}'");
    let _ = writeln!(out, "source_filename = \"{module_name}\"");
    let _ = writeln!(
        out,
        "target datalayout = \"e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64\""
    );
    let _ = writeln!(
        out,
        "target triple = \"dxil-unknown-shadermodel6.0-compute\""
    );
    out.push('\n');
    let _ = writeln!(out, "define void @{entry_symbol}() #0 {{");
    out.push_str("entry:\n");
    out.push_str("  ret void\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str(
        "attributes #0 = { noinline nounwind \"hlsl.numthreads\"=\"1,1,1\" \"hlsl.shader\"=\"compute\" }\n",
    );
    out
}

// ===========================================================================
// 图形=B 路:stage 分发 + B 链接线(G2.2 PR-D2 分片 2/3,RXS-0161/0162;任务4)。
//
// 分发规则(按 `body.stage`,RFC-0004 §4.1):
//   None(host / compute via kernel) → A 路 [`emit_dxil_ir`](RXS-0157,完全不改)。
//   Some(Vertex|Fragment)           → B 路 [`emit_dxil_b`](本任务新增)。
//   Some(Mesh|Task|RayGen|...)       → STUB(RD-012)「暂不支持」显式 6xxx 停手。
//
// B 链(本任务到 `parse_dxil_signatures` 产出 [`DxilSignatures`] 为止):
//   dxil_spirv::emit_spirv(stage,&io_sig) -> Vec<u32>          (任务2)
//     └─ 写临时 .spv(u32 小端字节,纯 safe)
//        └─ toolchain::spirv_cross_to_hlsl(..) -> HLSL          (分片1)
//           └─ toolchain::dxc_hlsl_to_dxil(..) -> DXIL 容器      (分片1)
//              └─ toolchain::dxc_disasm(..) -> 反汇编文本         (分片1)
//                 └─ toolchain::parse_dxil_signatures(text) -> DxilSignatures
//                    └─ // TODO(task 5): signature_gate::check(..)(校验门接缝)
//
// strict-only(R6.1):B 链任一**语言层**失败(编码器不可映射 / 工具运行后拒绝)
//   → 6xxx,禁止静默 fallback/降级。**工具链缺失**(定位失败 / spawn 失败)→ SKIP
//   (非 6xxx,环境降级,对齐 RXS-0073 ptxas 干验证 / RXS-0157 validator SKIP)。
//
// 🔒 禁区(R1.10 / R6.3):B 路输入 `io_sig`(`MirIoType` 仅标量/向量)结构上无法
//   表达资源句柄/描述符/采样器,故纹理访问语义(描述符编码 / 采样 opcode / 缓存 /
//   LOD / 导数 / 越界)在本层不可达;一旦未来类型面扩展触及,`emit_spirv` 将在映射
//   处发 [`DxilError::Unmappable`] 并标「需升档」,本层只透传、不发明 lowering /
//   ABI 二进制布局 / UB 契约(RFC-0004 §4.6)。
// ===========================================================================

/// stage 分发路由(任务4分发点的判定结果)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageRoute {
    /// `None`(host / compute via kernel)→ A 路 [`emit_dxil_ir`](RXS-0157,不改)。
    PathA,
    /// `Some(Vertex|Fragment)`→ B 路 [`emit_dxil_b`]。
    PathB(ShaderStage),
    /// `Some(Mesh|Task|RayGen|ClosestHit|AnyHit|Miss)`→ STUB(RD-012)「暂不支持」。
    Stub(ShaderStage),
}

/// 按 `stage` 分发 codegen 路由(RFC-0004 §4.1;R6.7 A 路零漂移)。
fn classify_stage(stage: Option<ShaderStage>) -> StageRoute {
    match stage {
        // 非着色阶段(host / compute via kernel,kernel 入口 stage 常为 None)→ A 路。
        None => StageRoute::PathA,
        // compute 着色阶段亦走 A 路(D-131 compute=A);防御性归 A,保 A 路零漂移。
        Some(ShaderStage::Compute) => StageRoute::PathA,
        // 图形着色阶段 → B 路。
        Some(s @ (ShaderStage::Vertex | ShaderStage::Fragment)) => StageRoute::PathB(s),
        // mesh/task/RT 等 → STUB(RD-012)(registry 落条目归任务15/owner;本层 stub 接缝)。
        Some(s) => StageRoute::Stub(s),
    }
}

/// B 路产出(B 链译后签名 + host 侧推导的 RTS0 root signature 容器字节)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxilBOutcome {
    /// B 链跑通,得译后签名(任务5 `signature_gate::check` 的意图比对输入)+ host 侧
    /// 绑定布局推导序列化出的 RTS0 root signature 容器字节(RXS-0165;PR-E2b 生产
    /// 接线,供 device PSO 创建消费,G-G2-3)。
    Produced {
        /// B 链译后签名(ISG1/OSG1)。
        sigs: DxilSignatures,
        /// host 侧推导序列化的 RTS0 root signature 容器(确定性;非 stable ABI)。
        root_signature: Vec<u8>,
    },
    /// 工具链不可用(定位失败 / spawn 失败 / 临时文件失败)→ SKIP(非 6xxx,
    /// 环境降级,对齐 RXS-0073);携带 SKIP 原因供 note 展示。
    Skipped(String),
}

/// B 链跑链体内部结果(签名 / SKIP;RTS0 由 [`emit_dxil_b`] 在 host 侧另行推导组装)。
enum BChainResult {
    /// B 链跑通,得译后签名 + 经校验门接受的 dumpbin 反汇编文本。
    ///
    /// `disasm` 是 `signature_gate` 实际取数、实际验过的**同一份** dumpbin 产物
    /// (步骤6),不另起手搓链——golden 据此即「校验门所验产物」单一真相源
    /// ([`emit_dxil_b_disasm`])。生产签名/RTS0 出口忽略它,行为零漂移。
    Sigs {
        sigs: DxilSignatures,
        disasm: String,
        /// 经校验门接受的 DXIL 容器字节(`stage.dxil`,dxc 产);供 D3D12 PSO 创建消费
        /// (G2.4 device 真跑,[`emit_dxil_b_container`])。生产签名/RTS0/golden 出口忽略它。
        dxil: Vec<u8>,
    },
    /// 工具链不可用 → SKIP(携带原因)。
    Skipped(String),
}

/// B 路 strict-only 失败(任务7 落码 RX6010~RX6013;G2.3 PR-E2b-2 续接绑定布局推导
/// 失败 RX6013/6015/6016/6017;emit 点见 [`emit_b_error`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxilBError {
    /// MIR→SPIR-V 编码器不可映射(透传任务2 [`DxilError::Unmappable`];含未来纹理访问
    /// 语义触及的 🔒 升档点)→ `RX6013` `codegen.dxil_unmappable`。
    Spirv(DxilError),
    /// B 链外部工具阶段运行后拒绝(spirv-cross / dxc / dumpbin exit != 0)→
    /// `RX6010` `codegen.dxil_b_transpile_failed`。`step` 为失败阶段,`reason` 为工具
    /// 错误串。(工具缺失/spawn 失败为 SKIP,非 6xxx。)
    Toolchain {
        /// 失败的 B 链阶段名(诊断用)。
        step: String,
        /// 工具错误串(诊断用)。
        reason: String,
    },
    /// 强制签名一致性校验门拒绝 → `RX6011` `codegen.dxil_sig_mismatch`(输出未保真)/
    /// `RX6012` `codegen.dxil_sig_dropped_input`(声明输入被消除)。honor deferred.json
    /// RX6009=RD-013 故不复用 RX6009。不可裁剪、无旁路(R2.5 / Property 5):校验失败
    /// 的入口绝不返回 `Produced`、绝不产 golden。
    SigGate(signature_gate::SigGateError),
    /// 绑定布局推导失败(RXS-0163~0166;G2.3 PR-E2b-2 按变体专属落码,
    /// [`emit_b_error`] 分派):`Unmappable` → `RX6013` `codegen.dxil_unmappable`
    /// (bindless / unbounded descriptor array RD-018 defer,复用既有不可映射码,owner
    /// 已裁不新开);`RegisterConflict` → `RX6015` `codegen.dxil_register_conflict`;
    /// `RootSignatureTooLarge` → `RX6016` `codegen.dxil_root_signature_too_large`;
    /// `Psv0Mismatch` → `RX6017` `codegen.dxil_psv0_mismatch`。strict-only,无运行期
    /// fallback。🔒 诊断只描述失败类别,不落 register/space/packing 物理布局值。
    Binding(binding_layout::BindingInferError),
}

impl std::fmt::Display for DxilBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxilBError::Spirv(e) => write!(f, "MIR→SPIR-V 不可映射: {e}"),
            DxilBError::Toolchain { step, reason } => {
                write!(f, "B 链 {step} 转译失败: {reason}")
            }
            DxilBError::SigGate(e) => write!(f, "{e}"),
            DxilBError::Binding(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DxilBError {}

/// 创建一个唯一的临时工作目录(进程 id + 纳秒戳;清理由调用方 `remove_dir_all`)。
fn scratch_dir() -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "rurix_dxil_b_codegen_{}_{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 区分 B 链工具失败语义(strict-only 的 SKIP↔6xxx 判定边界):
/// - **spawn 失败**(工具实际不存在 / 不可执行)= SKIP(环境问题,非 6xxx,对齐
///   RXS-0073 ptxas 干验证纪律)。分片1 驱动以 `cannot spawn` 前缀标记 spawn 失败。
/// - **工具运行后拒绝**(exit != 0)= B 链转译失败 → 6xxx(strict-only,R6.1)。
///
/// (分片1 工具链驱动只读复用、勿改,故据其错误串前缀判别 spawn↔exit。)
fn classify_tool_failure(step: &str, reason: String) -> Result<BChainResult, DxilBError> {
    if reason.contains("cannot spawn") {
        Ok(BChainResult::Skipped(format!(
            "{step} 不可执行(spawn 失败): {reason}"
        )))
    } else {
        Err(DxilBError::Toolchain {
            step: step.to_owned(),
            reason,
        })
    }
}

/// 从 vertex 阶段 I/O 意图签名导出 spirv-cross **顶点输入**语义保名旗标
/// (`--set-hlsl-vertex-input-semantic <location> <semantic>`,RFC-0004 §4.4 机制①)。
///
/// **机制(实测,贴 evidence/dxil_b_strict_only_report.md §3 + 本任务报告)**:spirv-cross
/// HLSL 后端默认把顶点输入语义按 location 写为通用 `TEXCOORD#`;`--set-hlsl-vertex-input-
/// semantic <location> <semantic>` 按 **location** 覆盖回用户语义名。[`dxil_spirv::emit_spirv`]
/// 对 Input 方向 varying/interpolate **按 io_sig 顺序递增分配 `Location`**(builtin 取
/// `BuiltIn` 装饰、**不**占 location),故此处按同一顺序复算 `location → field_name` 映射,
/// **经 io_sig 导出、非硬编码**(与 `emit_io_elem` 的 `next_in_location` 严格对齐)。
///
/// 实测要点:spirv-cross **不**消费 SPIR-V `UserSemantic` 装饰为 HLSL 语义(机制是
/// **location**,非 UserSemantic);`--set-hlsl-named-vertex-input-semantic` 按变量
/// `OpName` 匹配,而 `emit_spirv` 不 emit `OpName`,故按 location 覆盖是 Rust-emit SPIR-V
/// 路径下可复现的保名通道(本机 dxc 1.8.0.4739 / spirv-cross vulkan-sdk 实测 ISG1
/// `POSITION`/`NORMAL` 存活、不退化)。
///
/// **边界(实测)**:本机制仅覆盖 **vertex 阶段输入**用户语义名(机制①,按 location
/// 覆盖旗标)。**输出 varying** 与 **fragment 输入 varying** 的保名经 **RXS-0172**
/// (选项① HLSL 边界改写,[`restore_varying_semantics`])在 spirv-cross 产 HLSL 与 dxc
/// 之间复原(spirv-cross HLSL 后端无输出/片元语义旗标、不消费 UserSemantic);保名失败
/// 仍经 strict-only 校验门 **RX6011** 拒(不放宽门,P-01 / Property 5)。
fn vertex_input_semantic_flags(stage: ShaderStage, io_sig: &[IoSigElem]) -> Vec<String> {
    if stage != ShaderStage::Vertex {
        // fragment 输入 varying 不经顶点输入旗标(spirv-cross 无片元输入语义旗标);其保名
        // 由 RXS-0172 `restore_varying_semantics` 在 HLSL 边界复原(RD-017)。
        return Vec::new();
    }
    let mut flags = Vec::new();
    let mut location: u32 = 0;
    for elem in io_sig {
        if !matches!(elem.dir, IoDir::In) {
            continue;
        }
        match &elem.kind {
            // builtin 输入取 BuiltIn 装饰、**不**占 location(对齐 emit_spirv::emit_io_elem)。
            IoSigKind::Builtin(_) => {}
            // 非 builtin 输入按 io_sig 顺序占 location;有用户语义名 → emit 保名旗标。
            IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                if !elem.field_name.is_empty() {
                    flags.push("--set-hlsl-vertex-input-semantic".to_owned());
                    flags.push(location.to_string());
                    flags.push(elem.field_name.clone());
                }
                location += 1;
            }
        }
    }
    flags
}

/// RXS-0172:输出 varying / fragment 输入 varying 用户语义名保名(选项①,HLSL 边界改写)。
///
/// 在 spirv-cross 产 HLSL 与 dxc 之间施加:把退化为通用 `TEXCOORD<location>` 的 varying
/// 语义 token 按 `io_sig` 的 **location→用户语义名** provenance 改回用户名。provenance
/// 与 [`vertex_input_semantic_flags`] / [`dxil_spirv::emit_spirv`] **同源**(varying 按
/// 方向各自递增 `Location`,`#[builtin]` 不占 location)。`dir == In` 映射 spirv-cross
/// 输入 struct(entry 形参类型),`dir == Out` 映射输出 struct(entry 返回类型)。
///
/// **边界(RXS-0172 L3,ABI 中立)**:只替换 HLSL struct field 的 semantic token,不动
/// 类型 / 字段名 / 行结构 / 寄存器 packing,不冻结 register/mask/packing/byte layout/
/// 稳定 `Location`。
///
/// **fail-closed(RXS-0172 L4)**:仅在 provenance 明确(目标 struct 内存在对应
/// `TEXCOORD<loc>` field)时改写;不匹配则保留退化名,经末端 `signature_gate` RX6011
/// 拒(不放宽门,Property 5;RXS-0172 L2)。vertex 阶段输入经机制①(顶点输入保名旗标)
/// 已非 `TEXCOORD#`,本改写对其 In 方向自然 no-op。
fn restore_varying_semantics(io_sig: &[IoSigElem], hlsl: &str) -> String {
    let structs = collect_struct_names(hlsl);
    let (in_struct, out_struct) = find_entry_io_structs(hlsl, &structs);
    let mut text = hlsl.to_string();
    if let Some(s) = in_struct {
        text = rewrite_struct_varyings(&text, &s, &varying_provenance(io_sig, true));
    }
    if let Some(s) = out_struct {
        text = rewrite_struct_varyings(&text, &s, &varying_provenance(io_sig, false));
    }
    text
}

/// 按方向(`want_in`:In/Out)导出 varying 的 location→用户语义名 provenance。
/// 与 [`emit_io_elem`](dxil_spirv) 同源:builtin 不占 location,varying/interpolate
/// 按方向各自从 0 递增;空 `field_name` 不参与(无可恢复名)。
fn varying_provenance(io_sig: &[IoSigElem], want_in: bool) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut location: u32 = 0;
    for elem in io_sig {
        if matches!(elem.dir, IoDir::In) != want_in {
            continue;
        }
        match &elem.kind {
            IoSigKind::Builtin(_) => {}
            IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                if !elem.field_name.is_empty() {
                    out.push((location, elem.field_name.clone()));
                }
                location += 1;
            }
        }
    }
    out
}

/// 收集 HLSL 顶层 `struct <Name>` 声明名(供 entry I/O struct 识别)。
fn collect_struct_names(hlsl: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in hlsl.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("struct ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names
}

/// 解析 entry(` main(`)签名,识别输入 struct(形参类型)与输出 struct(返回类型)。
/// 返回 `(输入 struct, 输出 struct)`;非 struct(如 PS 直返 `float4 : SV_Target`)为 `None`。
fn find_entry_io_structs(hlsl: &str, structs: &[String]) -> (Option<String>, Option<String>) {
    let known = |t: &str| structs.iter().any(|s| s == t);
    for line in hlsl.lines() {
        let Some(mpos) = line.find(" main(") else {
            continue;
        };
        let out_struct = line[..mpos]
            .split_whitespace()
            .last()
            .map(str::to_owned)
            .filter(|t| known(t));
        let mut in_struct = None;
        if let Some(op) = line[mpos..].find('(') {
            let start = mpos + op + 1;
            if let Some(cp) = line[start..].find(')') {
                let params = &line[start..start + cp];
                for tok in params.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                    if !tok.is_empty() && known(tok) {
                        in_struct = Some(tok.to_owned());
                        break;
                    }
                }
            }
        }
        return (in_struct, out_struct);
    }
    (None, None)
}

/// 在指定 struct 块内,把 field 的 `TEXCOORD<loc>` 语义按 provenance 改回用户名。
/// 只动 semantic token(`:` 后的标识符),前缀(类型/字段名/冒号)与后缀(`;`/packing)
/// 逐字节保留(RXS-0172 L3 ABI 中立)。
fn rewrite_struct_varyings(hlsl: &str, struct_name: &str, prov: &[(u32, String)]) -> String {
    if prov.is_empty() {
        return hlsl.to_owned();
    }
    let header = format!("struct {struct_name}");
    let mut in_block = false;
    let mut out_lines: Vec<String> = Vec::new();
    for line in hlsl.lines() {
        let trimmed = line.trim_start();
        if !in_block {
            let is_header = trimmed.strip_prefix(&header).is_some_and(|tail| {
                tail.chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
            });
            if is_header {
                in_block = true;
            }
            out_lines.push(line.to_owned());
            continue;
        }
        if trimmed.starts_with('}') {
            in_block = false;
            out_lines.push(line.to_owned());
            continue;
        }
        out_lines.push(rewrite_field_semantic(line, prov));
    }
    let mut s = out_lines.join("\n");
    if hlsl.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// 改写单个 field 行的 semantic token:若 `:` 后语义为 `TEXCOORD<loc>` 且 provenance
/// 命中,替换为用户名;否则原样返回(fail-closed,RXS-0172 L4)。
fn rewrite_field_semantic(line: &str, prov: &[(u32, String)]) -> String {
    let Some(colon) = line.rfind(':') else {
        return line.to_owned();
    };
    let after = &line[colon + 1..];
    let lead = after.len() - after.trim_start().len();
    let sem_start = colon + 1 + lead;
    let rest = &line[sem_start..];
    let sem_len = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .count();
    if sem_len == 0 {
        return line.to_owned();
    }
    let sem = &rest[..sem_len];
    let digits = match sem
        .strip_prefix("TEXCOORD")
        .or_else(|| sem.strip_prefix("texcoord"))
    {
        Some(d) => d,
        None => return line.to_owned(),
    };
    let Ok(loc) = digits.parse::<u32>() else {
        return line.to_owned();
    };
    for (l, field) in prov {
        if *l == loc {
            let mut s = String::with_capacity(line.len());
            s.push_str(&line[..sem_start]);
            s.push_str(field);
            s.push_str(&line[sem_start + sem_len..]);
            return s;
        }
    }
    line.to_owned()
}

/// B 链跑链体(步骤 3~7):写临时 `.spv` → spirv-cross → dxc → dumpbin →
/// `parse_dxil_signatures`。临时目录由调用方 [`emit_dxil_b`] 创建并统一清理。
#[allow(clippy::too_many_arguments)] // B 链参数面(spv/工具/profile/io_sig/extra/stage)各为不同关注点,聚合为 struct 反损可读性。
fn run_b_chain(
    spv: &[u32],
    spvx: &Path,
    dxc: &Path,
    profile: &str,
    dir: &Path,
    io_sig: &[IoSigElem],
    extra: &[String],
    stage: ShaderStage,
) -> Result<BChainResult, DxilBError> {
    // 3) 写临时 `.spv`:`&[u32]` 小端 → `&[u8]`(纯 safe,`u32::to_le_bytes`,R1.11)。
    let spv_path = dir.join("stage.spv");
    let mut bytes = Vec::with_capacity(spv.len() * 4);
    for w in spv {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    if let Err(e) = std::fs::write(&spv_path, &bytes) {
        return Ok(BChainResult::Skipped(format!("写临时 .spv 失败: {e}")));
    }

    // 4) spirv-cross:SPIR-V → HLSL(SM 6.0)。`extra` = 顶点输入语义保名旗标
    //    (`--set-hlsl-vertex-input-semantic <loc> <semantic>`,经 io_sig 导出,
    //    [`vertex_input_semantic_flags`];RFC-0004 §4.4 机制①,实测顶点输入名存活)。
    let hlsl_path = dir.join("stage.hlsl");
    if let Err(e) = toolchain::spirv_cross_to_hlsl(spvx, &spv_path, &hlsl_path, 60, extra) {
        return classify_tool_failure("spirv-cross", e);
    }

    // 4.5) RXS-0172 输出 varying / fragment 输入 varying 用户语义名保名(选项①):在
    //     spirv-cross 产 HLSL 与 dxc 之间,按 io_sig location provenance 把退化的
    //     `TEXCOORD#` 改回用户语义名(`restore_varying_semantics`,与机制① 同源 provenance)。
    //     只动 semantic token(ABI 中立,RXS-0172 L3);fail-closed(provenance 不明确不改写,
    //     RXS-0172 L4);保名失败由步骤 8 signature_gate RX6011 闭合、不放宽门(Property 5)。
    match std::fs::read_to_string(&hlsl_path) {
        Ok(src) => {
            let restored = restore_varying_semantics(io_sig, &src);
            if restored != src
                && let Err(e) = std::fs::write(&hlsl_path, restored.as_bytes())
            {
                return Ok(BChainResult::Skipped(format!("写回保名 HLSL 失败: {e}")));
            }
        }
        Err(e) => return Ok(BChainResult::Skipped(format!("读回译 HLSL 失败: {e}"))),
    }

    // 5) dxc:HLSL → DXIL 容器(profile vs_6_0/ps_6_0,entry "main")。
    let dxil_path = dir.join("stage.dxil");
    if let Err(e) = toolchain::dxc_hlsl_to_dxil(dxc, &hlsl_path, profile, "main", &dxil_path) {
        return classify_tool_failure("dxc", e);
    }

    // 6) dxc -dumpbin:DXIL → 反汇编文本(`dxc_disasm` 吃 dxc **所在目录**,内部
    //    join dxc.exe;故由 dxc 可执行本体取 `.parent()`)。
    let dxc_dir = dxc.parent().map(Path::to_path_buf).unwrap_or_default();
    let disasm = match toolchain::dxc_disasm(&dxc_dir, &dxil_path) {
        Ok(d) => d,
        Err(e) => return classify_tool_failure("dxc -dumpbin", e),
    };

    // 7) 解析 DXIL ISG1/OSG1 签名 part(校验门取数基础)。
    let sigs = toolchain::parse_dxil_signatures(&disasm);

    // 8) 强制签名一致性校验门(任务5,不可裁剪 / 无旁路,R2.5 / Property 5):
    //    比对译后签名与 MIR 意图签名(用户语义名 / 系统值 / 被用输入 / 阶段间
    //    location 链接键),缺失 / 改名 / 错配 / 「声明但未用输入被消除」→ strict-only
    //    失败 → 6xxx,**终止该入口产物**(不返回 Produced、不产 golden)。
    //    fragment 输出 varying 按 RXS-0173 以 SV_Target# 渲染目标系统值忠实核对(阶段
    //    上下文 `stage`),vertex 输出维持 RXS-0172 用户语义名保名。
    signature_gate::check_with_stage(&sigs, io_sig, stage).map_err(DxilBError::SigGate)?;

    // 9) 回读经校验门接受的 DXIL 容器字节(步骤5 dxc 产 `stage.dxil`),供 device
    //    PSO 创建消费(G2.4)。读失败按环境降级 SKIP(非 6xxx;校验门已过故文件应在)。
    let dxil = match std::fs::read(&dxil_path) {
        Ok(b) => b,
        Err(e) => {
            return Ok(BChainResult::Skipped(format!(
                "读回 DXIL 容器字节失败: {e}"
            )));
        }
    };

    Ok(BChainResult::Sigs { sigs, disasm, dxil })
}

/// B 路 codegen:着色阶段(`stage` ∈ {Vertex, Fragment})+ I/O 意图签名(`io_sig`)
/// 与资源句柄绑定(`resources`)→ MIR→SPIR-V(含资源 `DescriptorSet`/`Binding` 装饰)
/// →spirv-cross→dxc→dumpbin→`parse_dxil_signatures`→`signature_gate::check`
/// → [`DxilSignatures`] 与 host 侧推导序列化的 RTS0 root signature 容器
/// ([`DxilBOutcome::Produced`])。
///
/// 强制签名一致性校验门(`signature_gate::check`,任务5)在 B 链末尾(步骤8)运行,
/// 不可裁剪、无旁路:译后签名与 `io_sig` 不一致 → strict-only 失败,绝不返回
/// [`DxilBOutcome::Produced`]。
///
/// RTS0 推导(RXS-0165;PR-E2b 生产接线):`binding_layout::infer_root_signature`
/// → `serialize_rts0`,纯 host 推导(工具链无关);`emit_spirv` 已先拒 bindless/
/// unbounded(RD-018),故生产侧资源(`Texture2D<F>`/`Sampler`)恒可推导。
///
/// # Errors
/// - 编码器不可映射构造(非 vertex·fragment 阶段 / 不可映射类型 / 未建模 builtin /
///   builtin 类型不符 / 越界向量宽度 / bindless 资源)→ [`DxilBError::Spirv`]
///   (strict-only,6xxx)。
/// - B 链外部工具运行后拒绝 → [`DxilBError::Toolchain`](6xxx)。
/// - 签名一致性校验门拒绝(语义名 / 系统值未保真 / 声明输入被消除)→
///   [`DxilBError::SigGate`](6xxx,任务5)。
///
/// 工具链不可用(定位失败 / spawn 失败 / 临时文件失败)→ `Ok(`[`DxilBOutcome::Skipped`]`)`
/// (非 6xxx,环境降级,真实红绿在带工具链的 dev/owner 环境)。
pub fn emit_dxil_b(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
) -> Result<DxilBOutcome, DxilBError> {
    // 1) MIR→SPIR-V(任务2 编码器 + RXS-0163 资源绑定装饰);不可映射 → 透传 6xxx
    //    (strict-only,不静默降级)。资源句柄绑定的 `DescriptorSet`/`Binding` 由
    //    host 侧 `binding_layout::infer_spirv_bindings` 确定性推导(见 emit_spirv)。
    let spv = dxil_spirv::emit_spirv(stage, io_sig, resources).map_err(DxilBError::Spirv)?;
    emit_dxil_b_from_spv(stage, io_sig, resources, spv)
}

/// B 路生产入口:消费完整 MIR [`Body`] 并按 RXS-0171 降级图形 body I/O 数据流。
///
/// 旧 [`emit_dxil_b`] 保留为签名-only 测试/工具入口;生产分发使用本函数,避免
/// RD-013 所述的 void stub 路径继续旁路 `body.blocks` / `locals`。
pub fn emit_dxil_b_body(body: &Body) -> Result<DxilBOutcome, DxilBError> {
    let Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) = body.stage else {
        return Err(DxilBError::Spirv(DxilError::Unmappable {
            what: "stage".to_owned(),
            detail: format!("Body stage {:?} 不在图形=B body lowering 范围", body.stage),
        }));
    };
    let spv = dxil_spirv::emit_spirv_body(stage, body).map_err(DxilBError::Spirv)?;
    emit_dxil_b_from_spv(stage, &body.io_sig, &body.resources, spv)
}

fn emit_dxil_b_from_spv(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
    spv: Vec<u32>,
) -> Result<DxilBOutcome, DxilBError> {
    // emit_spirv 成功即保证 stage ∈ {Vertex, Fragment};据此取 dxc profile。
    let profile = match stage {
        ShaderStage::Vertex => "vs_6_0",
        ShaderStage::Fragment => "ps_6_0",
        // 不可达(非图形阶段已在 emit_spirv 被拒);防御性 SKIP,不 panic。
        _ => return Ok(DxilBOutcome::Skipped("非图形阶段(不可达)".to_owned())),
    };

    // 1b) root signature 形态推导 + RTS0 容器序列化(RXS-0165;纯 host,工具链无关)。
    //     emit_spirv 已先拒 bindless/unbounded → 生产侧资源恒可推导。
    let root_signature = serialize_root_signature(resources)?;

    // 2) 工具链定位:缺失 → SKIP(非 6xxx,环境降级)。
    let Some(spvx) = toolchain::locate_spirv_cross() else {
        return Ok(DxilBOutcome::Skipped("spirv-cross 不可定位".to_owned()));
    };
    let Some(dxc) = toolchain::locate_dxc() else {
        return Ok(DxilBOutcome::Skipped("dxc 不可定位".to_owned()));
    };

    // 顶点输入语义保名旗标(经 io_sig 导出,非硬编码;RFC-0004 §4.4 机制①,实测)。
    // fragment / 无命名输入 → 空(behavior 不变)。
    let extra = vertex_input_semantic_flags(stage, io_sig);

    // 3~7) 临时工作目录内跑链;无论成败统一清理。
    let dir = match scratch_dir() {
        Ok(d) => d,
        Err(e) => return Ok(DxilBOutcome::Skipped(format!("临时目录创建失败: {e}"))),
    };
    let result = run_b_chain(&spv, &spvx, &dxc, profile, &dir, io_sig, &extra, stage);
    let _ = std::fs::remove_dir_all(&dir);
    match result {
        Ok(BChainResult::Sigs { sigs, .. }) => Ok(DxilBOutcome::Produced {
            sigs,
            root_signature,
        }),
        Ok(BChainResult::Skipped(why)) => Ok(DxilBOutcome::Skipped(why)),
        Err(e) => Err(e),
    }
}

/// 生产忠实 B 链反汇编(golden 单一真相源;RXS-0162 golden + RXS-0171 body 降级 +
/// RXS-0172 varying 保名)。
///
/// 驱动与 [`emit_dxil_b_body`] **同一条**生产链:`emit_spirv_body`(RXS-0171 入口
/// body I/O 数据流降级)→ 顶点输入保名旗标([`vertex_input_semantic_flags`])→
/// RXS-0172 输出/片元 varying 用户语义名保名([`restore_varying_semantics`],在
/// `run_b_chain` 内 HLSL 边界)→ dxc → dumpbin → `parse_dxil_signatures` → 强制
/// `signature_gate::check`。返回**经校验门接受**的规范化反汇编文本。
///
/// golden 比对此返回值,故 golden 字节 = 校验门所验产物本身——不再有第二条手搓
/// 链(签名-only `emit_spirv` + 空旗标 + 跳过保名)与生产链漂移。规范化仅抹平
/// 工具版本噪声行(shader hash / dxc ident),不动语言相关结构。
///
/// # Returns
/// - `Ok(Some(disasm))`:工具链可用、链跑通且校验门通过。
/// - `Ok(None)`:工具链不可用(spirv-cross / dxc 定位或 spawn 失败 / 临时目录失败)
///   → 环境降级(非 6xxx;真实红绿在带 pin B 工具链的 dev/owner 环境)。
///
/// # Errors
/// 同 [`emit_dxil_b_body`]:不可映射构造 / 工具运行后拒绝 / 校验门拒绝 → 6xxx,
/// 绝不静默成功。
pub fn emit_dxil_b_disasm(body: &Body) -> Result<Option<String>, DxilBError> {
    let Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) = body.stage else {
        return Err(DxilBError::Spirv(DxilError::Unmappable {
            what: "stage".to_owned(),
            detail: format!("Body stage {:?} 不在图形=B body lowering 范围", body.stage),
        }));
    };
    // 1) MIR→SPIR-V:消费完整 Body(RXS-0171 body 降级),与 emit_dxil_b_body 同源。
    let spv = dxil_spirv::emit_spirv_body(stage, body).map_err(DxilBError::Spirv)?;
    let profile = match stage {
        ShaderStage::Vertex => "vs_6_0",
        ShaderStage::Fragment => "ps_6_0",
        _ => return Ok(None),
    };
    // 2) 工具链定位:缺失 → SKIP(环境降级,非 6xxx)。
    let (Some(spvx), Some(dxc)) = (toolchain::locate_spirv_cross(), toolchain::locate_dxc()) else {
        return Ok(None);
    };
    // 顶点输入语义保名旗标(经 io_sig 导出;RFC-0004 §4.4 机制①)。
    let extra = vertex_input_semantic_flags(stage, &body.io_sig);
    let dir = match scratch_dir() {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };
    // 3~8) 与生产出口共用 run_b_chain(含 RXS-0172 保名 + 强制 signature_gate)。
    let result = run_b_chain(
        &spv,
        &spvx,
        &dxc,
        profile,
        &dir,
        &body.io_sig,
        &extra,
        stage,
    );
    let _ = std::fs::remove_dir_all(&dir);
    match result {
        Ok(BChainResult::Sigs { disasm, .. }) => Ok(Some(normalize_b_disasm(&disasm))),
        Ok(BChainResult::Skipped(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// B 路生产入口:消费完整 MIR [`Body`] 产出**经校验门接受的 DXIL 容器字节**(供 D3D12
/// PSO 创建消费;G2.4 UC-04 device 真跑)。与 [`emit_dxil_b_body`] / [`emit_dxil_b_disasm`]
/// **同一条**生产链:`emit_spirv_body`(RXS-0171 body I/O 数据流降级)→ 顶点输入保名旗标
/// → RXS-0172 输出/片元输入 varying 用户语义名保名 → dxc(产 DXIL 容器)→ dumpbin →
/// `parse_dxil_signatures` → 强制 `signature_gate::check_with_stage`(RXS-0173 fragment
/// 输出 SV_Target# 忠实核对)。返回的字节 = 校验门所验产物本身(无第二条手搓链)。
///
/// **G-G2-4 防降级**:device PSO 消费的 DXIL 来自本入口(Rurix 源经图形=B 链),非手写
/// HLSL/DXIL;校验门失败的入口绝不返回字节(`?` 终止落 6xxx)。
///
/// # Returns
/// - `Ok(Some(dxil))`:工具链可用、链跑通且校验门通过 → DXIL 容器字节。
/// - `Ok(None)`:工具链不可用(spirv-cross / dxc 定位或 spawn 失败 / 临时目录失败)→
///   环境降级(非 6xxx;真实产出在带 pin B 工具链的 dev/CI 环境)。
///
/// # Errors
/// 同 [`emit_dxil_b_body`]:不可映射构造 / 工具运行后拒绝 / 校验门拒绝 → 6xxx,绝不
/// 静默成功。
pub fn emit_dxil_b_container(body: &Body) -> Result<Option<Vec<u8>>, DxilBError> {
    let Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) = body.stage else {
        return Err(DxilBError::Spirv(DxilError::Unmappable {
            what: "stage".to_owned(),
            detail: format!("Body stage {:?} 不在图形=B body lowering 范围", body.stage),
        }));
    };
    // 1) MIR→SPIR-V:消费完整 Body(RXS-0171 body 降级),与 emit_dxil_b_body 同源。
    let spv = dxil_spirv::emit_spirv_body(stage, body).map_err(DxilBError::Spirv)?;
    let profile = match stage {
        ShaderStage::Vertex => "vs_6_0",
        ShaderStage::Fragment => "ps_6_0",
        _ => return Ok(None),
    };
    // 2) 工具链定位:缺失 → SKIP(环境降级,非 6xxx)。
    let (Some(spvx), Some(dxc)) = (toolchain::locate_spirv_cross(), toolchain::locate_dxc()) else {
        return Ok(None);
    };
    let extra = vertex_input_semantic_flags(stage, &body.io_sig);
    let dir = match scratch_dir() {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };
    // 3~9) 与生产出口共用 run_b_chain(含 RXS-0172/0173 + 强制 signature_gate + 回读 DXIL)。
    let result = run_b_chain(
        &spv,
        &spvx,
        &dxc,
        profile,
        &dir,
        &body.io_sig,
        &extra,
        stage,
    );
    let _ = std::fs::remove_dir_all(&dir);
    match result {
        Ok(BChainResult::Sigs { dxil, .. }) => Ok(Some(dxil)),
        Ok(BChainResult::Skipped(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 规范化 dxc 反汇编中的版本噪声行(shader hash 内容/版本派生 + dxc ident 构建串),
/// 使 golden 聚焦语言相关结构(签名表 / 入口 / 着色器类型),不写死工具版本。
fn normalize_b_disasm(s: &str) -> String {
    let mut lines = Vec::new();
    for raw in s.replace("\r\n", "\n").lines() {
        let t = raw.trim_start();
        if t.starts_with("; shader hash:") {
            lines.push("; shader hash: <OWNER-BLESSED-NORMALIZED>".to_owned());
        } else if raw.contains("dxc(private)") || raw.contains("dxcoob ") {
            // 保留 metadata id 前缀(如 `!0 = `),仅规范化版本串。
            let id = raw.split('=').next().unwrap_or("").trim_end();
            lines.push(format!("{id} = !{{!\"dxc <OWNER-BLESSED-NORMALIZED>\"}}"));
        } else {
            lines.push(raw.to_owned());
        }
    }
    lines.join("\n")
}

/// root signature 形态推导 + RTS0 容器序列化(RXS-0165;PR-E2b 生产接线)。
///
/// **G2.3 PR-E2b-2(本片落码)**:绑定推导失败按失败类别经 [`DxilBError::Binding`]
/// 携带真实 [`binding_layout::BindingInferError`] 变体上抛,[`emit_b_error`] 分派专属
/// 码——`Unmappable` → `RX6013`(复用)/ `RegisterConflict` → `RX6015` / `RootSignature
/// TooLarge` → `RX6016` / `Psv0Mismatch` → `RX6017`(替换 E2b-1 经 `RX6013` 一律透传的
/// interim)。`emit_spirv` 在本函数前已先拒 bindless/unbounded(RD-018),生产侧资源
/// (`Texture2D<F>`/`Sampler`,基数 One)恒可推导,故 `Err` 分支当前仍主要作 strict-only
/// 防御(绝不静默产出空 root signature),失败类别的真实红绿由 [`binding_layout`] host
/// 单测 + [`emit_b_error`] 分派单测保证。
fn serialize_root_signature(resources: &[ResourceBinding]) -> Result<Vec<u8>, DxilBError> {
    match binding_layout::infer_root_signature(resources) {
        Ok(rs) => Ok(binding_layout::serialize_rts0(&rs)),
        Err(e) => Err(DxilBError::Binding(e)),
    }
}

/// 单个 device [`Body`] 的 DXIL codegen 分发产出(任务4分发点的整体结果)。
#[derive(Debug)]
pub enum DispatchOutcome {
    /// `None`(compute/kernel)→ A 路 DirectX 三元组 LLVM IR 文本(RXS-0157)。
    PathAIr(String),
    /// Vertex/Fragment → B 路译后签名(任务5校验门接缝输入)+ RTS0 root signature。
    PathBSignatures {
        /// B 链译后签名(ISG1/OSG1)。
        sigs: DxilSignatures,
        /// host 侧推导序列化的 RTS0 root signature 容器(RXS-0165;PR-E2b)。
        root_signature: Vec<u8>,
    },
    /// Vertex/Fragment → B 路工具链 SKIP(非 6xxx;携带原因)。
    SkippedB(String),
    /// 已发诊断(A 路 RX6007 子集外 / B 路 strict-only 6xxx / mesh·task·RT stub 6xxx);
    /// 无产物。
    Diagnosed,
}

/// B 路 strict-only 失败 → 按真实可达类别落 6xxx 结构化诊断(任务7 只追加新码)。
///
/// 编号映射(honor `registry/deferred.json`:RX6008=mesh/task/RT RD-012、
/// RX6009=阶段 I/O body 数据流降级 RD-013,均留给既有引用不改派;本片真实可达类别
/// 自 RX6010 起):
/// - [`DxilBError::Toolchain`] → `RX6010` `codegen.dxil_b_transpile_failed`
///   (spirv-cross / dxc / dumpbin 运行后 exit≠0;工具缺失/spawn 失败为 SKIP 非本码);
/// - [`SigGateError::SigMismatch`] → `RX6011` `codegen.dxil_sig_mismatch`
///   (输出方向用户语义名 / 系统值未保真);
/// - [`SigGateError::SigDroppedInput`] → `RX6012` `codegen.dxil_sig_dropped_input`
///   (声明的外部输入被消除且不可等价保留);
/// - [`DxilBError::Spirv`](`DxilError::Unmappable`)→ `RX6013` `codegen.dxil_unmappable`
///   (MIR→SPIR-V 编码器最小子集外构造);
/// - [`DxilBError::Binding`] → 按 [`binding_layout::BindingInferError`] 变体分派
///   (G2.3 PR-E2b-2):`Unmappable` → `RX6013`(复用,bindless/unbounded RD-018)/
///   `RegisterConflict` → `RX6015` / `RootSignatureTooLarge` → `RX6016` /
///   `Psv0Mismatch` → `RX6017`。
///
/// 🔒 纹理访问语义结构上不可达(`io_sig` 仅标量/向量),**不造码**(R3.6 不预造)。
/// 🔒 绑定布局诊断只描述失败类别,**不**落 register/space/packing 物理布局值。
fn emit_b_error(diag: &DiagCtxt, span: Span, err: &DxilBError) {
    use crate::binding_layout::BindingInferError;
    use crate::dxil_sig_gate::signature_gate::SigGateError;
    match err {
        DxilBError::Toolchain { step, reason } => {
            diag.struct_error(ErrorCode(6010), "codegen.dxil_b_transpile_failed")
                .arg("step", step.clone())
                .arg("reason", reason.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::SigGate(SigGateError::SigMismatch { detail }) => {
            diag.struct_error(ErrorCode(6011), "codegen.dxil_sig_mismatch")
                .arg("detail", detail.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::SigGate(SigGateError::SigDroppedInput { detail }) => {
            diag.struct_error(ErrorCode(6012), "codegen.dxil_sig_dropped_input")
                .arg("detail", detail.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::Spirv(e) => {
            // 采样首期收敛子集外 → RX6023(RXS-0175);其余 MIR→SPIR-V 不可映射 → RX6013。
            match e {
                DxilError::SampleUnsupported { .. } => {
                    diag.struct_error(ErrorCode(6023), "codegen.dxil_sample_unsupported")
                        .arg("detail", e.to_string())
                        .span_label(span, "in DXIL graphics entry")
                        .emit();
                }
                DxilError::Unmappable { .. } => {
                    diag.struct_error(ErrorCode(6013), "codegen.dxil_unmappable")
                        .arg("detail", e.to_string())
                        .span_label(span, "in DXIL graphics entry")
                        .emit();
                }
            }
        }
        DxilBError::Binding(e) => {
            // 绑定布局推导失败按类别分派专属码(RXS-0163~0166;owner 已裁:
            // Unmappable 复用 RX6013,其余新开 RX6015/6016/6017)。诊断载荷只取失败
            // 类别描述(BindingInferError::Display),🔒 不含 register/space 物理值。
            let (code, key) = match e {
                BindingInferError::Unmappable { .. } => (6013, "codegen.dxil_unmappable"),
                BindingInferError::RegisterConflict { .. } => {
                    (6015, "codegen.dxil_register_conflict")
                }
                BindingInferError::RootSignatureTooLarge { .. } => {
                    (6016, "codegen.dxil_root_signature_too_large")
                }
                BindingInferError::Psv0Mismatch { .. } => (6017, "codegen.dxil_psv0_mismatch"),
            };
            diag.struct_error(ErrorCode(code), key)
                .arg("detail", e.to_string())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
    }
}

/// 按 `body.stage` 分发 codegen 并落诊断(任务4分发点)。
///
/// - `None`(compute/kernel)→ A 路 [`emit_dxil_ir`](完全不改);成功 →
///   [`DispatchOutcome::PathAIr`],子集外 → RX6007 + [`DispatchOutcome::Diagnosed`]。
/// - `Some(Vertex|Fragment)`→ B 路 [`emit_dxil_b`];产出 →
///   [`DispatchOutcome::PathBSignatures`],SKIP → note +
///   [`DispatchOutcome::SkippedB`],strict-only 失败 → 6xxx +
///   [`DispatchOutcome::Diagnosed`]。
/// - mesh/task/RT 等 → STUB(RD-012)「暂不支持」6xxx + [`DispatchOutcome::Diagnosed`]。
pub fn dispatch_and_emit(diag: &DiagCtxt, body: &Body, module_name: &str) -> DispatchOutcome {
    match classify_stage(body.stage) {
        // ── A 路(compute/kernel):完全复用既有 emit_dxil_ir,零漂移(R6.7)。 ──
        StageRoute::PathA => match emit_dxil_ir(body, module_name) {
            Ok(ir) => DispatchOutcome::PathAIr(ir),
            Err(e) => {
                diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                    .arg("detail", e.detail.clone())
                    .span_label(e.span, "in DXIL compute entry")
                    .emit();
                DispatchOutcome::Diagnosed
            }
        },
        // ── B 路(vertex/fragment):MIR→SPIR-V→…→parse_dxil_signatures。 ──
        StageRoute::PathB(_stage) => match emit_dxil_b_body(body) {
            Ok(DxilBOutcome::Produced {
                sigs,
                root_signature,
            }) => {
                // 校验门已在 B 链内部(run_b_chain 步骤8)强制通过:能到此即译后签名
                // 与 MIR 意图签名一致(用户语义名/系统值/被用输入/链接键保真)。校验
                // 失败的入口绝不到此(已转 Err 分支落 6xxx),Property 5 不旁路由此保证。
                DispatchOutcome::PathBSignatures {
                    sigs,
                    root_signature,
                }
            }
            Ok(DxilBOutcome::Skipped(why)) => {
                eprintln!(
                    "rurixc: note: [SKIP] DXIL B 链工具链不可用({why});转译 + 签名校验 \
                     SKIPPED(开发环境降级,非 6xxx,对齐 RXS-0073;真实红绿在带工具链环境)"
                );
                DispatchOutcome::SkippedB(why)
            }
            Err(e) => {
                emit_b_error(diag, body.span, &e);
                DispatchOutcome::Diagnosed
            }
        },
        // ── STUB(RD-012):mesh/task/RT 着色器类型 DXIL 阶段降级不可用 → 显式 RX6008 停手。 ──
        StageRoute::Stub(stage) => {
            // RX6008 改接(RFC-0013 §4.E9,RD-012 预留码正式落 registry,introduced_in
            // G3.6,语义「DXIL 阶段降级不可用」)。拒绝通道自 RX6007 迁至专用码 RX6008
            // (只加类别不改既有语义,Q-M-RX6008Scope):probe 绿的阶段落地真降级后从拒绝
            // 集移除(mesh/task 全量落地 = PR-Me/后续接线);blocked 阶段(RT 六模型,双上游
            // 钳制 spirv-cross 无 SPV_KHR_ray_tracing 消费 + RD-015)维持在拒绝集(strict-only,
            // 无静默降级,R6.1)。
            diag.struct_error(ErrorCode(6008), "codegen.dxil_stage_deferred")
                .arg(
                    "detail",
                    format!(
                        "着色器类型 {stage:?} 的 DXIL 阶段降级不可用(RD-012;RT 六模型上游 \
                         blocked → RD-034 尾门,步骤 69 blocked 探针;mesh/task 全量落地随 B 链接线)"
                    ),
                )
                .span_label(body.span, "in DXIL graphics entry")
                .emit();
            DispatchOutcome::Diagnosed
        }
    }
}

/// vertex+fragment 多阶段联编点的链接核对结果(RXS-0160 IR2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageLinkOutcome {
    /// 无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ 无链接核对(behavior 不变,
    /// 单阶段 / A 路零漂移,RXS-0157 R6.7)。
    NoPair,
    /// vertex out ↔ fragment in 链接一致(语义名 / 类型 / 插值全配对)。
    Linked,
    /// 错链(strict-only;经 [`emit_stage_link_error`] 落 `RX6014`
    /// `codegen.dxil_stage_link_mismatch`,agent 裁定方案 B 新开码、不复用 RX6011,
    /// 见 [`signature_gate::StageLinkError`])。
    LinkError(signature_gate::StageLinkError),
}

/// vertex+fragment 多阶段联编点接缝(RXS-0160 IR2):从 device MIR body 集合中收集
/// vertex / fragment 两阶段的 `io_sig`,汇集到链接核对入口
/// [`signature_gate::check_stage_link`]。
///
/// 由单着色阶段编译([`dispatch_and_emit`] 逐 body)扩到 **vertex+fragment 配对**的
/// 链接核对:取首个 vertex 阶段 body 与首个 fragment 阶段 body,以 vertex 输出方向 +
/// fragment 输入方向的 `io_sig` 核实跨阶段 varying 链接键(语义名 / 类型 / 插值)。
/// 无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ [`StageLinkOutcome::NoPair`]
/// (behavior 不变,零漂移)。
///
/// **错误码(G2.3 PR-E2b-2 已落,agent 裁定方案 B)**:错链返回
/// [`StageLinkOutcome::LinkError`],经 [`emit_stage_link_error`] 落 `RX6014`
/// `codegen.dxil_stage_link_mismatch`——agent 裁定**新开 RX6014**(不复用 RX6011 单阶段
/// 签名不一致语义;spec §2 RXS-0160 IR3)。strict-only 语义由 `check_stage_link` 保证
/// (错链必 Err,绝不静默通过)。
pub fn link_graphics_stages(bodies: &[Body]) -> StageLinkOutcome {
    let vs = bodies
        .iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex)));
    let fs = bodies
        .iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Fragment)));
    match (vs, fs) {
        (Some(v), Some(f)) => match signature_gate::check_stage_link(&v.io_sig, &f.io_sig) {
            Ok(()) => StageLinkOutcome::Linked,
            Err(e) => StageLinkOutcome::LinkError(e),
        },
        _ => StageLinkOutcome::NoPair,
    }
}

/// 阶段间接口错链 → `RX6014` `codegen.dxil_stage_link_mismatch` 结构化诊断(RXS-0160;
/// G2.3 PR-E2b-2,agent 裁定方案 B 新开码)。
///
/// 两类 [`signature_gate::StageLinkError`](`Unlinked` 缺链接键 / `LinkMismatch`
/// 类型·插值失配)均落同一 `RX6014`(同属阶段间接口错链失败类别,RXS-0160 L2/L3);
/// strict-only,无运行期 fallback。🔒 诊断只描述错链失败类别,**不**落 location /
/// register / mask 物理布局值(ABI 中立,RFC-0004 §4.6(a))。
pub fn emit_stage_link_error(diag: &DiagCtxt, span: Span, err: &signature_gate::StageLinkError) {
    diag.struct_error(ErrorCode(6014), "codegen.dxil_stage_link_mismatch")
        .arg("detail", err.to_string())
        .span_label(span, "in DXIL graphics stage link")
        .emit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::UnOp;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// RXS-0157:空体 compute kernel(`kernel fn` 无形参)→ DirectX 三元组 DXIL IR。
    //@ spec: RXS-0157
    #[test]
    fn empty_compute_kernel_emits_dxil_directx_triple() {
        let src = "kernel fn cs_noop() {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(!diag.has_errors(), "空体 compute kernel 应 0 诊断");
        let ir = build_and_emit_dxil(&cx, "cs_noop").expect("应产出 DXIL IR");
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""));
        assert!(ir.contains("\"hlsl.shader\"=\"compute\""));
        assert!(ir.contains("\"hlsl.numthreads\"=\"1,1,1\""));
        assert!(ir.contains("ret void"));
    }

    /// RXS-0157 L2:带 ABI 形参的 kernel(View 形参)→ 子集外 → RX6007。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_view_param_is_rx6007() {
        let src = "kernel fn k(out: ViewMut<global, f32>) {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        let ir = build_and_emit_dxil(&cx, "k");
        assert!(ir.is_none(), "带形参 compute 入口应被拒(子集外)");
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(codes.contains(&6007), "应发 RX6007,实得 {codes:?}");
    }

    // ───────────────── 任务4:stage 分发 + B 链 单测 ─────────────────

    use crate::hir::{DefId, PrimTy};
    use crate::mir::{
        BasicBlock, IoDir, IoSigKind, Local, LocalIdx, MirIoType, Place, Statement, Terminator,
    };
    use crate::ty::Ty;

    fn dummy_span() -> Span {
        Span::new(SourceId(0), 0, 0, Edition::Rx0)
    }

    /// 便捷构造一个图形阶段 [`IoSigElem`]。
    fn io(name: &str, kind: IoSigKind, ty: MirIoType, dir: IoDir) -> IoSigElem {
        IoSigElem {
            field_name: name.to_owned(),
            kind,
            ty,
            dir,
        }
    }

    /// 最小图形阶段 vertex I/O:builtin position(out) + 一个 varying(out)+
    /// builtin vertex_index(in)。
    fn vertex_io() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
        ]
    }

    /// 最小图形阶段 fragment I/O:varying(in)+ builtin frag_coord(in)+
    /// varying(out)。
    fn fragment_io() -> Vec<IoSigElem> {
        vec![
            io(
                "in_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ]
    }

    /// RXS-0172 provenance:varying 按方向各自从 0 递增 location,builtin 不占,空名跳过
    /// (与 `vertex_input_semantic_flags` / `emit_io_elem` 同源)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_varying_provenance_matches_location_assignment() {
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
        ];
        // Out 方向:builtin position 不占 location,normal→0,uv→1。
        assert_eq!(
            varying_provenance(&io_sig, false),
            vec![(0, "normal".to_owned()), (1, "uv".to_owned())]
        );
        // In 方向:仅 builtin,无 varying → 空。
        assert!(varying_provenance(&io_sig, true).is_empty());
    }

    /// 模拟 spirv-cross 回译 HLSL(vertex 输出 struct 的 varying 退化为 TEXCOORD#)。
    fn vs_degraded_hlsl() -> &'static str {
        "struct SPIRV_Cross_Input\n\
         {\n\
         \x20   float3 in_var_POSITION : POSITION;\n\
         };\n\
         \n\
         struct SPIRV_Cross_Output\n\
         {\n\
         \x20   float3 out_var_NORMAL : TEXCOORD0;\n\
         \x20   float2 out_var_UV : TEXCOORD1;\n\
         \x20   float4 gl_Position : SV_Position;\n\
         };\n\
         \n\
         SPIRV_Cross_Output main(SPIRV_Cross_Input stage_input)\n\
         {\n\
         \x20   SPIRV_Cross_Output stage_output;\n\
         \x20   return stage_output;\n\
         }\n"
    }

    /// RXS-0172 L1/L3:vertex 输出 varying 退化名按 location provenance 复原为用户语义名,
    /// 只动 semantic token(类型/字段名/`;`/行数不变),SV_Position 与输入 struct 不动。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_output_varying_semantics_restored() {
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ];
        let src = vs_degraded_hlsl();
        let out = restore_varying_semantics(&io_sig, src);
        // 保名:输出 struct 的 TEXCOORD0/1 → normal/uv。
        assert!(out.contains("float3 out_var_NORMAL : normal;"), "{out}");
        assert!(out.contains("float2 out_var_UV : uv;"), "{out}");
        // SV_Position 不动;退化 TEXCOORD# 已消失(输出 struct)。
        assert!(out.contains("float4 gl_Position : SV_Position;"));
        assert!(!out.contains(": TEXCOORD"));
        // ABI 中立:类型/字段名保留;行数不变。
        assert!(out.contains("float3 out_var_NORMAL :"));
        assert!(out.contains("float2 out_var_UV :"));
        assert_eq!(src.lines().count(), out.lines().count());
    }

    /// RXS-0172 L4:fail-closed —— provenance 不覆盖的 location 退化名保留(经末端门拒)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_unmapped_location_is_left_degraded_fail_closed() {
        // 仅给 location 0 的 provenance;location 1(uv)无对应 → 保留 TEXCOORD1。
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
        ];
        let out = restore_varying_semantics(&io_sig, vs_degraded_hlsl());
        assert!(out.contains("float3 out_var_NORMAL : normal;"), "{out}");
        // 无 provenance 的 TEXCOORD1 不被改写(fail-closed,留给 RX6011)。
        assert!(out.contains("float2 out_var_UV : TEXCOORD1;"), "{out}");
    }

    /// RXS-0172 L1:fragment 输入 varying 退化名按 location provenance 复原。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_fragment_input_varying_restored() {
        let src = "struct SPIRV_Cross_Input\n\
                   {\n\
                   \x20   float3 in_var_NORMAL : TEXCOORD0;\n\
                   \x20   float2 in_var_UV : TEXCOORD1;\n\
                   };\n\
                   \n\
                   float4 main(SPIRV_Cross_Input stage_input) : SV_Target\n\
                   {\n\
                   \x20   return 0.0.xxxx;\n\
                   }\n";
        let io_sig = vec![
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
        ];
        let out = restore_varying_semantics(&io_sig, src);
        assert!(out.contains("float3 in_var_NORMAL : normal;"), "{out}");
        assert!(out.contains("float2 in_var_UV : uv;"), "{out}");
        assert!(!out.contains(": TEXCOORD"));
    }

    /// RXS-0172 L2:保名标准不放宽 —— 退化名经强制校验门 RX6011 拒,复原后等价名过。
    /// 用生产 `signature_gate::check`(不放宽)对模拟的 dxc 译后签名核验。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_gate_not_relaxed_degraded_rejected_restored_accepted() {
        use crate::toolchain::{DxilSignatures, SigElement};
        let intent = vec![
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ];
        let mk = |name: &str, idx: u32| SigElement {
            name: name.to_owned(),
            index: idx,
            register: idx.to_string(),
            sysvalue: "NONE".to_owned(),
            used: true,
        };
        // 退化(TEXCOORD#):门拒(RX6011 SigMismatch)。
        let degraded = DxilSignatures {
            input: Vec::new(),
            output: vec![mk("TEXCOORD", 0), mk("TEXCOORD", 1)],
        };
        assert!(
            signature_gate::check(&degraded, &intent).is_err(),
            "退化 TEXCOORD# 必须被 RX6011 拒(门不放宽)"
        );
        // 复原(用户名):门过,无需放宽。
        let restored = DxilSignatures {
            input: Vec::new(),
            output: vec![mk("normal", 0), mk("uv", 0)],
        };
        assert!(
            signature_gate::check(&restored, &intent).is_ok(),
            "复原用户语义名后校验门应过(不放宽)"
        );
    }

    /// RXS-0172:entry I/O struct 识别(返回类型=输出 struct;形参类型=输入 struct)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_entry_io_struct_identification() {
        let structs = collect_struct_names(vs_degraded_hlsl());
        let (in_s, out_s) = find_entry_io_structs(vs_degraded_hlsl(), &structs);
        assert_eq!(in_s.as_deref(), Some("SPIRV_Cross_Input"));
        assert_eq!(out_s.as_deref(), Some("SPIRV_Cross_Output"));
    }

    /// 构造一个最小平凡 [`Body`](空体 + 单 Return 块);`stage`/`io_sig` 由调用方设。
    fn make_body(stage: Option<ShaderStage>, io_sig: Vec<IoSigElem>) -> Body {
        let sp = dummy_span();
        Body {
            def: DefId(0),
            symbol: "main".to_owned(),
            color: FnColor::Kernel,
            generic_args: Vec::new(),
            locals: vec![Local {
                ty: Ty::unit(),
                name: None,
                span: sp,
                shared: false,
                array_len: None,
            }],
            arg_count: 0,
            blocks: vec![BasicBlock {
                stmts: Vec::new(),
                terminator: Terminator {
                    kind: TerminatorKind::Return,
                    span: sp,
                },
            }],
            span: sp,
            stage,
            io_sig,
            resources: Vec::new(),
        }
    }

    fn emitted_codes(diag: &DiagCtxt) -> Vec<u16> {
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    /// 分发恒跑(工具无关):None→A 路;Vertex/Fragment→B 路;mesh/task/RT→stub。
    /// 阶段→着色器类型/路由分类(含 mesh/task/RT deferred→stub→RX6007)即 RXS-0158 主旨。
    //@ spec: RXS-0158, RXS-0161
    #[test]
    fn classify_stage_routes_by_category() {
        // None(host / compute via kernel)→ A 路。
        assert_eq!(classify_stage(None), StageRoute::PathA);
        // compute 着色阶段亦归 A(D-131 compute=A)。
        assert_eq!(
            classify_stage(Some(ShaderStage::Compute)),
            StageRoute::PathA
        );
        // 图形阶段 → B 路。
        assert_eq!(
            classify_stage(Some(ShaderStage::Vertex)),
            StageRoute::PathB(ShaderStage::Vertex)
        );
        assert_eq!(
            classify_stage(Some(ShaderStage::Fragment)),
            StageRoute::PathB(ShaderStage::Fragment)
        );
        // mesh/task/RT 等 → STUB(RD-012)。
        for s in [
            ShaderStage::Mesh,
            ShaderStage::Task,
            ShaderStage::RayGen,
            ShaderStage::ClosestHit,
            ShaderStage::AnyHit,
            ShaderStage::Miss,
        ] {
            assert_eq!(
                classify_stage(Some(s)),
                StageRoute::Stub(s),
                "{s:?} 应 stub"
            );
        }
    }

    /// 分发:compute/kernel body(stage None,空体)→ A 路,产 DirectX 三元组 IR,
    /// 不进 B 路(A 路用例不回归)。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_compute_body_goes_path_a() {
        let diag = DiagCtxt::new();
        let body = make_body(None, Vec::new());
        match dispatch_and_emit(&diag, &body, "cs_noop") {
            DispatchOutcome::PathAIr(ir) => {
                assert!(
                    ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""),
                    "A 路应产 compute 三元组 IR"
                );
            }
            other => panic!("compute body 应走 A 路,实得 {other:?}"),
        }
        assert!(!diag.has_errors(), "空体 compute A 路应 0 诊断");
    }

    /// 分发:vertex/fragment body → B 路分支(非 A 路)。任务5 校验门接入后,带工具链
    /// 真跑时 trivial passthrough 被 DCE → 校验门如期拒绝 → `Diagnosed`(6xxx,经
    /// 既有 DXIL 诊断通道);工具链缺失 → `SkippedB`;均**不**得 `PathAIr`(零漂移)。
    /// 关键不变式:图形阶段恒走 B 路,绝不误入 A 路。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_graphics_body_goes_path_b_not_a() {
        for (stage, io_sig) in [
            (ShaderStage::Vertex, vertex_io()),
            (ShaderStage::Fragment, fragment_io()),
        ] {
            let diag = DiagCtxt::new();
            let body = make_body(Some(stage), io_sig);
            match dispatch_and_emit(&diag, &body, "gfx") {
                // 校验门通过(签名保真)。
                DispatchOutcome::PathBSignatures { .. } => {}
                // 工具链不可用 → SKIP(非 6xxx,环境降级)。
                DispatchOutcome::SkippedB(_) => {}
                // 带工具链真跑:trivial passthrough DCE 消除声明输入 → 校验门
                // strict-only 如期拒绝 → 新 6xxx(RX6012 声明输入被消除 / RX6011 签名
                // 不一致;设计决策1 红例域,非误入 A 路,绝不复用 A 路 RX6007)。
                DispatchOutcome::Diagnosed => {
                    let codes = emitted_codes(&diag);
                    assert!(
                        codes.iter().any(|c| (6010..=6013).contains(c)),
                        "{stage:?} B 路校验门拒绝应经新 B 路 6xxx 码(RX6010~6013),实得 {codes:?}"
                    );
                    assert!(
                        !codes.contains(&6007),
                        "{stage:?} B 路失败绝不复用 A 路 RX6007(零漂移),实得 {codes:?}"
                    );
                }
                DispatchOutcome::PathAIr(_) => panic!("{stage:?} 误入 A 路"),
            }
        }
    }

    /// mesh/task/RT stub:发「DXIL 阶段降级不可用」RX6008 诊断、不产物(RFC-0013 §4.E9
    /// RX6008 改接;RD-012 预留码正式落 registry,拒绝通道自 RX6007 迁至专用码 RX6008)。
    //@ spec: RXS-0161
    //@ spec: RXS-0249
    #[test]
    fn dispatch_mesh_task_rt_stub_diagnoses_no_artifact() {
        for s in [
            ShaderStage::Mesh,
            ShaderStage::Task,
            ShaderStage::RayGen,
            ShaderStage::ClosestHit,
            ShaderStage::AnyHit,
            ShaderStage::Miss,
            ShaderStage::Intersection,
            ShaderStage::Callable,
        ] {
            let diag = DiagCtxt::new();
            let body = make_body(Some(s), Vec::new());
            let outcome = dispatch_and_emit(&diag, &body, "gfx");
            assert!(
                matches!(outcome, DispatchOutcome::Diagnosed),
                "{s:?} 应 stub 诊断不产物,实得 {outcome:?}"
            );
            assert!(
                emitted_codes(&diag).contains(&6008),
                "{s:?} stub 应发 RX6008(阶段降级不可用,RX6008 改接),实得 {:?}",
                emitted_codes(&diag)
            );
            assert!(
                !emitted_codes(&diag).contains(&6007),
                "{s:?} stub 已自 RX6007 迁至 RX6008,不得再发 RX6007,实得 {:?}",
                emitted_codes(&diag)
            );
        }
    }

    /// strict-only:不可映射构造(f64 标量)→ emit_dxil_b 返回 [`DxilBError::Spirv`]
    /// (透传任务2 编码器),绝不静默成功。工具无关恒跑。
    //@ spec: RXS-0161
    #[test]
    fn emit_dxil_b_unmappable_is_error_not_silent() {
        let io = vec![io(
            "weird",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F64),
            IoDir::Out,
        )];
        let r = emit_dxil_b(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilBError::Spirv(DxilError::Unmappable { .. }))),
            "f64 应透传不可映射 6xxx,实得 {r:?}"
        );
    }

    /// strict-only:不可映射构造经分发 → 6xxx 诊断、不产物(走既有 RX6007 通道)。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_unmappable_graphics_body_diagnoses() {
        let io = vec![io(
            "weird",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F64),
            IoDir::Out,
        )];
        let diag = DiagCtxt::new();
        let body = make_body(Some(ShaderStage::Vertex), io);
        let outcome = dispatch_and_emit(&diag, &body, "gfx");
        assert!(
            matches!(outcome, DispatchOutcome::Diagnosed),
            "不可映射应诊断不产物,实得 {outcome:?}"
        );
        assert!(
            emitted_codes(&diag).contains(&6013),
            "应发 RX6013 不可映射构造"
        );
    }

    /// RXS-0171 strict-only:白名单外 body rvalue 经生产分发映射为 RX6013。
    //@ spec: RXS-0171
    #[test]
    fn dispatch_unsupported_body_rvalue_diagnoses_rx6013() {
        let io = vec![io(
            "out_luma",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F32),
            IoDir::Out,
        )];
        let mut body = make_body(Some(ShaderStage::Fragment), io);
        let sp = dummy_span();
        body.locals = vec![Local {
            ty: Ty::Adt(DefId(9017), Vec::new()),
            name: None,
            span: sp,
            shared: false,
            array_len: None,
        }];
        body.blocks[0].stmts.push(Statement {
            kind: StatementKind::Assign(
                Place::local(LocalIdx(0)),
                Rvalue::UnaryOp(UnOp::Neg, Operand::Const(Const::Float(1.0, PrimTy::F32))),
            ),
            span: sp,
        });

        let diag = DiagCtxt::new();
        let outcome = dispatch_and_emit(&diag, &body, "gfx");
        assert!(
            matches!(outcome, DispatchOutcome::Diagnosed),
            "unsupported body rvalue 应诊断不产物,实得 {outcome:?}"
        );
        assert!(
            emitted_codes(&diag).contains(&6013),
            "unsupported body rvalue 应发 RX6013,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    // 🔒 禁区说明(纹理访问语义 → 6xxx):IoSigElem/MirIoType 仅可表达已建模标量/
    // 向量,**结构上无法**表达资源句柄/描述符/采样器,故纹理访问语义(描述符编码/
    // 采样 opcode/缓存/LOD/导数/越界)在本层不可构造、不可达(任务2 即如此);该路径
    // 由后续绑定布局分片(G2.3,P-11)覆盖,本层保留 emit_dxil_b 的 DxilBError::Spirv
    // 透传接缝 + 模块顶注「需升档」标注。故本任务无纹理 6xxx 单测(输入不可达)。

    /// B 链端到端(带工具链 → 真跑直到 `signature_gate::check`;缺失 → SKIP 不 fail)。
    /// vertex + fragment 各一例。
    ///
    /// 任务5 接缝接入后的真实行为(设计决策1):任务2 最小子集 emit 的是 trivial
    /// passthrough `main`,**不读写 I/O**,dxc 会把未用的 builtin/varying DCE 消除
    /// (B 链 vertex 例实测得 `input:[]`)→ 校验门按 strict-only **如期拒绝**
    /// (`SigDroppedInput`:声明输入被消除)。这是 R2.4 预期红例域,**不是 bug**,
    /// 更**不**为让测试通过而旁路校验门(Property 5)。故接受的真跑结局为:
    /// - `Skipped`(工具链不可用)→ SKIP;
    /// - `Err(SigGate(SigDroppedInput))`(DCE 消除声明输入)→ 校验门如期红;
    /// - `Err(SigGate(SigMismatch))`(语义名/系统值未保真)→ 校验门如期红;
    /// - `Produced`(若译后签名恰好保真)→ 校验门绿。
    ///
    /// 编码器不可映射 / 工具转译失败仍判为测试失败(最小子集不应触发)。
    //@ spec: RXS-0162
    #[test]
    fn emit_dxil_b_end_to_end_or_skip() {
        for (tag, stage, io_sig) in [
            ("vertex", ShaderStage::Vertex, vertex_io()),
            ("fragment", ShaderStage::Fragment, fragment_io()),
        ] {
            match emit_dxil_b(stage, &io_sig, &[]) {
                Ok(DxilBOutcome::Produced { sigs, .. }) => {
                    // 校验门已强制通过:译后签名与意图签名保真。
                    eprintln!("[OK] {tag} B 链产签名且校验门通过: {sigs:?}");
                }
                Ok(DxilBOutcome::Skipped(why)) => {
                    eprintln!("[SKIP] {tag} B 链工具链不可用: {why}");
                }
                Err(DxilBError::SigGate(e)) => {
                    // strict-only 如期拒绝(trivial passthrough DCE 消除声明输入/
                    // 未保真),非 bug、非旁路。
                    eprintln!("[GATE-REJECT] {tag} 校验门如期拒绝 DCE/未保真产物: {e}");
                }
                Err(e) => panic!(
                    "[{tag}] B 链最小子集不应因编码器/工具失败而红(校验门拒绝走 SigGate): {e}"
                ),
            }
        }
    }

    /// **Property 5(校验门不旁路)**:校验门失败是 B 路 strict-only 失败的一种,经
    /// **唯一**出口 [`emit_b_error`] 落 6xxx 结构化诊断,**绝不**静默通过、绝不产物。
    /// 两类 [`SigGateError`] 分别落 `RX6011`(SigMismatch)/ `RX6012`(SigDroppedInput)。
    ///
    /// 代码层佐证(无需运行):`run_b_chain` 步骤8 以 `signature_gate::check(..)
    /// .map_err(DxilBError::SigGate)?` 在返回 [`DxilBOutcome::Produced`] **之前**以 `?`
    /// 终止——校验失败的入口不可能到达 `Produced` 分支;且 `check` 签名仅 `(actual,
    /// intent)`,无任何 skip / 开关 / env 参数(类型层即无旁路面)。
    //@ spec: RXS-0162
    #[test]
    fn property5_siggate_failure_routes_to_6xxx_never_silent() {
        use crate::dxil_sig_gate::signature_gate::SigGateError;
        let cases = [
            (
                DxilBError::SigGate(SigGateError::SigMismatch {
                    detail: "语义名未保真".to_owned(),
                }),
                6011u16,
            ),
            (
                DxilBError::SigGate(SigGateError::SigDroppedInput {
                    detail: "声明输入被消除".to_owned(),
                }),
                6012u16,
            ),
        ];
        for (err, expected) in cases {
            let diag = DiagCtxt::new();
            emit_b_error(&diag, dummy_span(), &err);
            assert!(diag.has_errors(), "校验门失败必落诊断(strict-only,不静默)");
            assert!(
                emitted_codes(&diag).contains(&expected),
                "校验门失败必经新 6xxx 码 RX{expected}(不旁路、不复用 RX6007),实得 {:?}",
                emitted_codes(&diag)
            );
            assert!(
                !emitted_codes(&diag).contains(&6007),
                "校验门失败绝不再落 A 路 RX6007(零漂移),实得 {:?}",
                emitted_codes(&diag)
            );
        }
    }

    /// B 链转译失败(`DxilBError::Toolchain`,spirv-cross/dxc/dumpbin exit≠0)经
    /// [`emit_b_error`] 落 `RX6010` `codegen.dxil_b_transpile_failed`,strict-only 不静默。
    /// (SKIP——工具缺失/spawn 失败——在 `classify_tool_failure` 即转 `Skipped`,不到此。)
    //@ spec: RXS-0157
    #[test]
    fn emit_b_error_toolchain_routes_to_rx6010() {
        let err = DxilBError::Toolchain {
            step: "dxc".to_owned(),
            reason: "exit 1: validation error".to_owned(),
        };
        let diag = DiagCtxt::new();
        emit_b_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6010),
            "B 链转译失败应发 RX6010,实得 {:?}",
            emitted_codes(&diag)
        );
        assert!(
            !emitted_codes(&diag).contains(&6007),
            "B 链转译失败绝不复用 A 路 RX6007(零漂移),实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 不可映射构造(`DxilBError::Spirv(Unmappable)`)经 [`emit_b_error`] 落 `RX6013`
    /// `codegen.dxil_unmappable`,strict-only 不静默。
    //@ spec: RXS-0157
    #[test]
    fn emit_b_error_unmappable_routes_to_rx6013() {
        let err = DxilBError::Spirv(DxilError::Unmappable {
            what: "scalar-type".to_owned(),
            detail: "f64 不在已建模标量子集".to_owned(),
        });
        let diag = DiagCtxt::new();
        emit_b_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6013),
            "不可映射构造应发 RX6013,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 绑定布局推导失败经 [`emit_b_error`] 按变体分派专属码(RXS-0163~0166;
    /// G2.3 PR-E2b-2,owner 已裁:Unmappable 复用 RX6013,其余新开 RX6015/6016/6017)。
    //@ spec: RXS-0163, RXS-0164, RXS-0165, RXS-0166
    #[test]
    fn emit_b_error_binding_routes_to_dedicated_codes() {
        use crate::binding_layout::BindingInferError;
        let cases: &[(DxilBError, u16)] = &[
            (
                DxilBError::Binding(BindingInferError::Unmappable {
                    detail: "bindless 资源(RD-018 defer)".to_owned(),
                }),
                6013,
            ),
            (
                DxilBError::Binding(BindingInferError::RegisterConflict {
                    detail: "t0 区间重叠".to_owned(),
                }),
                6015,
            ),
            (
                DxilBError::Binding(BindingInferError::RootSignatureTooLarge {
                    dwords: 66,
                    limit: 64,
                }),
                6016,
            ),
            (
                DxilBError::Binding(BindingInferError::Psv0Mismatch {
                    detail: "反射资源数与意图不一致".to_owned(),
                }),
                6017,
            ),
        ];
        for (err, expected) in cases {
            let diag = DiagCtxt::new();
            emit_b_error(&diag, dummy_span(), err);
            let codes = emitted_codes(&diag);
            assert!(
                codes.contains(expected),
                "{err:?} 应发 RX{expected},实得 {codes:?}"
            );
            // 零漂移:绑定布局失败绝不复用 RX6007(A 路)/ RX6011~6012(签名校验门)。
            assert!(
                !codes.contains(&6007),
                "绑定布局失败绝不复用 A 路 RX6007,实得 {codes:?}"
            );
        }
    }

    /// 阶段间接口错链经 [`emit_stage_link_error`] 落 `RX6014`
    /// `codegen.dxil_stage_link_mismatch`(RXS-0160;agent 裁定方案 B 新开码,
    /// 不复用 RX6011),两类错链(`Unlinked` / `LinkMismatch`)同落 RX6014。
    //@ spec: RXS-0160
    #[test]
    fn emit_stage_link_error_routes_to_rx6014() {
        let errs = [
            signature_gate::StageLinkError::Unlinked {
                detail: "fragment 输入 `extra` 无上游链接键".to_owned(),
            },
            signature_gate::StageLinkError::LinkMismatch {
                detail: "链接键 `color` 两端类型失配".to_owned(),
            },
        ];
        for err in &errs {
            let diag = DiagCtxt::new();
            emit_stage_link_error(&diag, dummy_span(), err);
            let codes = emitted_codes(&diag);
            assert!(
                codes.contains(&6014),
                "阶段间接口错链应发 RX6014,实得 {codes:?}"
            );
            // 零漂移:错链新开码 RX6014,绝不复用单阶段签名不一致 RX6011。
            assert!(
                !codes.contains(&6011),
                "错链 owner 裁定新开 RX6014,绝不复用 RX6011,实得 {codes:?}"
            );
        }
    }

    /// 顶点输入语义保名旗标导出(工具无关,恒跑):[`vertex_input_semantic_flags`] 按
    /// io_sig 顺序复算 location → field_name(与 emit_spirv 的 next_in_location 对齐),
    /// 经 io_sig 导出、**非硬编码**(RFC-0004 §4.4 机制①,实测顶点输入名存活)。
    //@ spec: RXS-0159
    #[test]
    fn vertex_input_semantic_flags_derive_from_io_sig() {
        // vertex:命名输入 POSITION(loc0)/ NORMAL(loc1)+ builtin vertex_index(不占
        // location)+ 命名输出(不取输入旗标)。
        let io_sig = vec![
            io(
                "POSITION",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
            io(
                "NORMAL",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ];
        let flags = vertex_input_semantic_flags(ShaderStage::Vertex, &io_sig);
        // POSITION→loc0(builtin 不占 location)、NORMAL→loc1;输出 color 不取旗标。
        assert_eq!(
            flags,
            vec![
                "--set-hlsl-vertex-input-semantic".to_owned(),
                "0".to_owned(),
                "POSITION".to_owned(),
                "--set-hlsl-vertex-input-semantic".to_owned(),
                "1".to_owned(),
                "NORMAL".to_owned(),
            ],
            "顶点输入保名旗标应按 io_sig 顺序复算 location(builtin 不占位),非硬编码"
        );

        // fragment:顶点输入保名旗标机制不适用(无顶点输入语义旗标)→ 空(边界;fragment
        // 输入 varying 保名经 RXS-0172 `restore_varying_semantics`,见该函数单测)。
        assert!(
            vertex_input_semantic_flags(ShaderStage::Fragment, &fragment_io()).is_empty(),
            "fragment 阶段不导出顶点输入保名旗标(spirv-cross 无片元输入语义旗标;保名走 RXS-0172)"
        );

        // vertex 无命名输入(仅 builtin 输入 / 命名输出)→ 空(行为不变)。
        assert!(
            vertex_input_semantic_flags(ShaderStage::Vertex, &vertex_io()).is_empty(),
            "无命名顶点输入 → 无保名旗标(行为不变)"
        );
    }

    // ───────────────── RXS-0160:vertex+fragment 多阶段联编点接缝 ─────────────────

    /// 链接一致的 vertex 输出(position builtin out + uv interpolate out)。
    fn vs_link_io() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ]
    }

    /// 与 [`vs_link_io`] 链接一致的 fragment 输入(frag_coord builtin in + uv
    /// interpolate in + out_color varying out)。
    fn fs_link_io() -> Vec<IoSigElem> {
        vec![
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
            io(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ]
    }

    /// accept:vertex+fragment 配对 + 链接一致 → `Linked`(多阶段联编点核对通过)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_consistent_pair_is_linked() {
        let bodies = vec![
            make_body(Some(ShaderStage::Vertex), vs_link_io()),
            make_body(Some(ShaderStage::Fragment), fs_link_io()),
        ];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::Linked,
            "vertex+fragment 链接一致应 Linked"
        );
    }

    /// reject:fragment 输入 varying(`extra`)在 vertex 输出无链接键 → `LinkError`
    /// (错链;strict-only)→ 经 [`emit_stage_link_error`] 落 `RX6014`(agent 裁定方案 B
    /// 新开码,G2.3 PR-E2b-2)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_mismatched_pair_is_link_error() {
        let fs = vec![io(
            "extra",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::In,
        )];
        let bodies = vec![
            make_body(Some(ShaderStage::Vertex), vs_link_io()),
            make_body(Some(ShaderStage::Fragment), fs),
        ];
        let outcome = link_graphics_stages(&bodies);
        let StageLinkOutcome::LinkError(err) = outcome else {
            panic!("错链应 LinkError,实得 {outcome:?}");
        };
        // 错链经生产 emit 接缝落真实码 RX6014(替换 agent 裁码前的占位「6xxx」)。
        let diag = DiagCtxt::new();
        emit_stage_link_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6014),
            "错链应发 RX6014,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 单阶段编译(仅 vertex,缺 fragment)→ `NoPair`(无链接核对,零漂移)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_single_stage_is_no_pair() {
        let bodies = vec![make_body(Some(ShaderStage::Vertex), vs_link_io())];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::NoPair,
            "缺 fragment 阶段应 NoPair(单阶段编译零漂移)"
        );
    }

    /// 无图形阶段(compute/kernel,stage None)→ `NoPair`(A 路 / 单阶段零漂移)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_no_graphics_is_no_pair() {
        let bodies = vec![make_body(None, Vec::new())];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::NoPair,
            "无图形阶段(compute)应 NoPair(零漂移)"
        );
    }
}
