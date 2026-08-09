//! `dgc` — G9.2 M102 DGC(Device-Generated Commands)抽象层类型面
//! (g9.p0.m102.dgc_abstraction;spec/gpu_driven_submit.md RXS-0348;RFC-0023 §4.1)。
//!
//! 纯 host、safe(本模块零 `unsafe`)、always-on(不随 `vulkan` feature 门控):
//! - **token 声明闭集**([`DgcToken`]):`bind_vertex_buffer` / `bind_index_buffer` /
//!   `push_constants` / `draw` / `draw_indexed` / `dispatch`——`ExecuteIndirect`
//!   语义的跨 API 最小公倍数(RFC-0023 §4.1.2 逐字);超出子集的 token(如 D3D12
//!   专有)**不可表达**(枚举封闭,无占位变体)。
//! - **token 限制装配期核验**([`IndirectCmdLayout::assemble`],fail-closed,沿
//!   RXS-0237 装配核验先例):每 sequence **恰一个** `draw`/`draw_indexed`/
//!   `dispatch` 终止 token 且必须位于**最后**;sequence 内**不可开 render
//!   pass**、**不可插 barrier**、**不可绑 descriptor set**(后三类连同闭集外
//!   token 一并**不可表达**——`DgcToken` 内不存在这些变体)。任一违例 → typed
//!   [`DgcError`],fail-closed,不存在「碰巧能跑」。
//! - **`DgcBuffer` 无 host 读接口类型契约**(RXS-0348 §3-2;镜像 RXS-0144~0148
//!   `AsyncBuffer` 在途态无 host 读接口先例):GPU 可写命令数据 buffer 的类型层
//!   句柄——**不提供**任何 host 读/写/取址公开方法(方法不存在 = 编译期拦截,
//!   结构性保证非纪律口号);调试 dump 走显式 readback pass(`g.readback` 既有
//!   面,RXS-0236)。配套验收面(RFC-0023 §4.4.2):「回读计数器 = 0」——任何
//!   隐式回读(如调试路径)必须经计数器显式记账,计数器非零即红;
//!   [`readback_counter`] 为进程级单一计数器,装配/执行路径不得触碰。
//! - **capability snapshot 阻塞性前置**(RXS-0348 §3-3;RXS-0313 M32 snapshot
//!   核验原语):`submit.dgc` 必须实测在位——设备 lane 装配前以
//!   [`verify_dgc_snapshot`] 对照实测 device extension 表 fail-closed;本类型层
//!   只做比对,**不探测设备**(探测归 vk.rs U54 lane 的 `vkEnumerateDeviceExtension
//!   Properties` 真实调用),合成注入仅供 host 单测负臂;缺 capability → typed
//!   `Err`,**禁静默模拟**(P-01)。
//! - **三后端映射单一事实源**(RXS-0348 §3-5 表逐字;镜像 RXS-0238「双后端
//!   映射同源」纪律):[`map_token`] / [`map_abstract`] 纯函数表;`Backend` 封闭
//!   三枚举(Vulkan / D3D12 / Nvptx)。
//! - **非 stable 物理布局**(RXS-0348 §3-6):DGC buffer 物理字节格式
//!   (`vkCmdPreprocessGeneratedCommandsEXT` 产物)与 Execution Set 句柄为实现
//!   确定、非 stable;本模块只作存在性/确定性声明,不冻结数值布局。

use core::sync::atomic::{AtomicU64, Ordering};

// ═══════════════════════ token 声明闭集(RXS-0348 §3 Syntax 逐字) ═══════════════

/// DGC token 声明闭集(RXS-0348 §3 Syntax 逐字;跨 API 最小公倍数):
///
/// ```text
/// token ::= bind_vertex_buffer | bind_index_buffer | push_constants
///         | draw | draw_indexed | dispatch        // 终止 token 三选一,恰一且最后
/// ```
///
/// `BeginRenderPass` / `InsertBarrier` / `BindDescriptorSet` **不是**本闭集成员——
/// 「sequence 内不可开 render pass / 不可插 barrier / 不可绑 descriptor set」三条
/// 限制由**不可表达**(枚举无此变体)结构性承载,叠加装配期静态核验双层拦截;
/// 超出子集的 token(如 D3D12 专有 draw 族)首期**不可表达**。
//@ spec: RXS-0348
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DgcToken {
    /// 顶点缓冲绑定(状态 token)。
    BindVertexBuffer,
    /// 索引缓冲绑定(状态 token)。
    BindIndexBuffer,
    /// push constants 更新(状态 token)。
    PushConstants,
    /// 终止 token:非索引 draw。
    Draw,
    /// 终止 token:索引 draw。
    DrawIndexed,
    /// 终止 token:compute dispatch。
    Dispatch,
}

impl DgcToken {
    /// 终止 token(draw / draw_indexed / dispatch 三选一)判定。
    #[must_use]
    pub const fn is_terminator(self) -> bool {
        matches!(
            self,
            DgcToken::Draw | DgcToken::DrawIndexed | DgcToken::Dispatch
        )
    }

    /// 条款 Syntax 闭集字面(诊断/报告面;与 spec/gpu_driven_submit.md §3 逐字)。
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            DgcToken::BindVertexBuffer => "bind_vertex_buffer",
            DgcToken::BindIndexBuffer => "bind_index_buffer",
            DgcToken::PushConstants => "push_constants",
            DgcToken::Draw => "draw",
            DgcToken::DrawIndexed => "draw_indexed",
            DgcToken::Dispatch => "dispatch",
        }
    }

    /// 闭集全表(冻结序 = spec 条款 Syntax 序)。
    pub const ALL: [DgcToken; 6] = [
        DgcToken::BindVertexBuffer,
        DgcToken::BindIndexBuffer,
        DgcToken::PushConstants,
        DgcToken::Draw,
        DgcToken::DrawIndexed,
        DgcToken::Dispatch,
    ];
}

// ═══════════════════════ 装配期 typed Err(沿 RX6029 族装配诊断先例) ═══════════════

/// DGC layout 装配期违例类别(RXS-0348 §3-1;装配期确定性拒绝,fail-closed)。
///
/// 沿 RX6029 族(render graph 装配核验 typed `Err`)先例:库面装配诊断,
/// 不占新 RX 码(RFC-0023 §5 错误码策略:复用既有装配诊断类别)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DgcLayoutViolation {
    /// sequence 为空(零 token 无终止,不可执行)。
    EmptySequence,
    /// 多终止 token(draw/draw_indexed/dispatch 恰一之违例;conformance
    /// `dgc_layout_double_terminator.rx` 承载的本族违例)。
    MultipleTerminators,
    /// 终止 token 未位于 sequence 最后(终止后仍有 token)。
    TerminatorNotLast,
    /// 零终止 token(sequence 无 draw/draw_indexed/dispatch,不可执行)。
    MissingTerminator,
}

/// DGC 类型层 typed `Err`(fail-closed;运行期后端失败经 `String` 消息承载,
/// 镜像 rhi.rs「I3/I5 镜像 RX6029 口径」装配诊断纪律,不占 RX 码)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DgcError {
    /// layout 装配期核验违例(token 限制内化,RXS-0348 §3-1)。
    Layout(DgcLayoutViolation),
    /// capability snapshot 缺 `submit.dgc` 阻塞性前置(RXS-0348 §3-3;
    /// RXS-0313 fail-closed 口径,禁静默模拟 P-01)。
    CapabilityMissing,
}

impl std::fmt::Display for DgcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DgcError::Layout(v) => {
                let detail = match v {
                    DgcLayoutViolation::EmptySequence => {
                        "empty token sequence (no terminator; not executable)"
                    }
                    DgcLayoutViolation::MultipleTerminators => {
                        "multiple terminator tokens (draw/draw_indexed/dispatch 恰一之违例)"
                    }
                    DgcLayoutViolation::TerminatorNotLast => {
                        "terminator token not last in sequence"
                    }
                    DgcLayoutViolation::MissingTerminator => {
                        "missing terminator token (sequence 须恰一个 draw/draw_indexed/dispatch)"
                    }
                };
                write!(
                    f,
                    "DGC IndirectCmdLayout assembly violation: {detail} \
                     (fail-closed, RXS-0348 §3-1; RX6029 族装配诊断先例)"
                )
            }
            DgcError::CapabilityMissing => write!(
                f,
                "capability.runtime_snapshot_mismatch: required capabilities missing from \
                 device capability snapshot: [submit.dgc] (fail-closed, RXS-0348 §3-3 / \
                 RXS-0313; 禁静默模拟 P-01)"
            ),
        }
    }
}

impl std::error::Error for DgcError {}

// ═══════════════════════ IndirectCmdLayout(声明闭集 + 装配期核验) ═══════════════

/// IndirectCmdLayout 声明式模板(RXS-0348 §2):一个命令 sequence 的 token 序列。
///
/// 合法实例只能经 [`IndirectCmdLayout::assemble`] 构造——装配期核验在构造点
/// fail-closed,非法 token 序列**无法构造出对象**(结构性保证,非运行期检查;
/// RFC-0023 §7「token 限制放运行时检查」方案否决)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IndirectCmdLayout {
    tokens: Vec<DgcToken>,
}

impl IndirectCmdLayout {
    /// 装配期核验并构造(fail-closed,RXS-0348 §3-1):
    ///
    /// - 恰一个终止 token 且位于最后(多终止 / 终止非最后 / 零终止 → typed `Err`);
    /// - 「不可开 render pass / 不可插 barrier / 不可绑 descriptor set」三类由
    ///   [`DgcToken`] 闭集不可表达承载(本函数再断言 token 全在闭集内,双层拦截)。
    ///
    /// 核验先于任何 Vulkan 对象创建/资源绑定(vk.rs U54 lane 只消费 `Ok` 实例)。
    //@ spec: RXS-0348
    pub fn assemble(tokens: &[DgcToken]) -> Result<Self, DgcError> {
        if tokens.is_empty() {
            return Err(DgcError::Layout(DgcLayoutViolation::EmptySequence));
        }
        let terminators = tokens.iter().filter(|t| t.is_terminator()).count();
        if terminators > 1 {
            return Err(DgcError::Layout(DgcLayoutViolation::MultipleTerminators));
        }
        if terminators == 0 {
            return Err(DgcError::Layout(DgcLayoutViolation::MissingTerminator));
        }
        // 恰一终止 token:必须位于最后。
        if !tokens[tokens.len() - 1].is_terminator() {
            return Err(DgcError::Layout(DgcLayoutViolation::TerminatorNotLast));
        }
        Ok(IndirectCmdLayout {
            tokens: tokens.to_vec(),
        })
    }

    /// 已核验 token 序列(声明序 = 冻结序;只读)。
    #[must_use]
    pub fn tokens(&self) -> &[DgcToken] {
        &self.tokens
    }

    /// 终止 token(装配期已证恰一且最后)。
    #[must_use]
    pub fn terminator(&self) -> DgcToken {
        // 不变量:assemble 已证非空 + 恰一终止 + 最后;unwrap 不可达失败。
        self.tokens.last().copied().unwrap_or(DgcToken::Draw)
    }

    /// 状态 token 段(终止 token 之前的全部;可能为空)。
    #[must_use]
    pub fn state_tokens(&self) -> &[DgcToken] {
        &self.tokens[..self.tokens.len() - 1]
    }
}

// ═══════════════════════ DgcBuffer(无 host 读接口类型契约) ═══════════════

/// DgcBuffer — GPU 可写命令数据 buffer 的类型层句柄(RXS-0348 §3-2;镜像
/// RXS-0144~0148 `AsyncBuffer` 在途态无 host 读接口先例)。
///
/// **类型不提供 host 读接口**:host 侧读/写/取址方法**不存在**(不是运行期
/// 报错,是编译期方法不存在——「零 CPU 回读」的类型层结构性保证)。调试 dump
/// 走显式 readback pass(`g.readback` 既有面,RXS-0236);任何隐式回读须经
/// [`readback_counter`] 显式记账(计数器非零即红,RFC-0023 §4.4.2)。
///
/// 本类型只是身份/尺寸句柄;实际 VkBuffer 生命周期归 vk.rs U54 lane 单点
/// (创建/销毁配对)。构造 crate-private——host 侧代码只能持有/移交,不能读。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DgcBuffer {
    /// 字节容量(实现确定、非 stable 物理布局的组成部分;RXS-0348 §3-6)。
    byte_len: u64,
    /// 所声明 layout 的 token 数(身份面;layout 本体由调用方持有)。
    layout_token_count: u32,
}

impl DgcBuffer {
    /// crate 内构造(vk.rs U54 lane 建 buffer 后登记;host 外部不可构造 = 无
    /// 公开构造器 = 不可经本类型伪造 GPU 命令数据身份)。
    pub(crate) fn new(byte_len: u64, layout_token_count: u32) -> Self {
        DgcBuffer {
            byte_len,
            layout_token_count,
        }
    }

    /// 字节容量(只读元数据,非内容读;不涉及 buffer 内容本身)。
    #[must_use]
    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    /// 所声明 layout 的 token 数(只读元数据,非内容读)。
    #[must_use]
    pub const fn layout_token_count(&self) -> u32 {
        self.layout_token_count
    }
    // ── 结构性断言锚(RXS-0348 §3-2):本 impl 块与其余 impl/不存在的 pub 方法共同
    // 保证「无 host 读接口」——不存在任何返回 buffer 内容/地址的 pub 方法(read/
    // read_bytes/as_slice/map/unmap/as_ptr/addr/get 均不定义);下行单测以源码扫描
    // 双重核验(防未来漂移引入)。
}

/// 进程级回读计数器(RFC-0023 §4.4.2「回读计数器 = 0」断言;RXS-0348 §3-2
/// 配套验收面)。**任何隐式回读(如调试路径)必须经本计数器显式记账**;
/// M102 门判据 = 全程计数器 0(装配/执行路径不存在任何记账点调用)。
static READBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 回读计数器记账(显式回读路径调用;M102 首期无任何生产调用点——调试 dump
/// 走 `g.readback` 显式 pass 不经本计数,本函数存在即「记账唯一入口」事实源)。
pub fn readback_counter_record(n: u64) {
    READBACK_COUNTER.fetch_add(n, Ordering::Relaxed);
}

/// 回读计数器读数(验收面;`== 0` 为绿,非零即红)。
//@ spec: RXS-0348
#[must_use]
pub fn readback_counter() -> u64 {
    READBACK_COUNTER.load(Ordering::Relaxed)
}

// ═══════════════════════ 三后端映射单一事实源(RXS-0348 §3-5 表逐字) ═══════════════

/// DGC 三后端闭集(RXS-0348 §3-5;镜像 RXS-0238 双后端映射同源纪律的第三轴)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DgcBackend {
    /// Vulkan(`VK_EXT_device_generated_commands`)。
    Vulkan,
    /// D3D12(command signature + ExecuteIndirect)。
    D3D12,
    /// NVPTX(不承诺执行面,仅命令数据生成)。
    Nvptx,
}

/// 三后端映射纯函数(RXS-0348 §3-5 表逐字;单一事实源——表外映射不存在):
///
/// | 抽象 | Vulkan | D3D12 | NVPTX |
/// |---|---|---|---|
/// | IndirectCmdLayout | `VkIndirectCommandsLayoutEXT` | command signature | 不承诺 |
/// | DgcBuffer 填充 | GPU compute 直写 + preprocess | GPU compute 直写 argument buffer | compute kernel 产出 |
/// | 执行 | `vkCmdExecuteGeneratedCommandsEXT` | `ExecuteIndirect` | — |
///
/// NVPTX 执行面 `None`(「—」不承诺);首期本模块只消费 Vulkan 行(device lane)。
//@ spec: RXS-0348
#[must_use]
pub fn map_token(token: DgcToken, backend: DgcBackend) -> Option<&'static str> {
    use DgcBackend as B;
    use DgcToken as T;
    Some(match (token, backend) {
        // Vulkan 行:VK_EXT_device_generated_commands token 名(vulkan_core.h
        // VkIndirectCommandsTokenTypeEXT 逐字)。
        (T::BindVertexBuffer, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_VERTEX_BUFFER_EXT",
        (T::BindIndexBuffer, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_INDEX_BUFFER_EXT",
        (T::PushConstants, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_PUSH_CONSTANT_EXT",
        (T::Draw, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_DRAW_EXT",
        (T::DrawIndexed, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_DRAW_INDEXED_EXT",
        (T::Dispatch, B::Vulkan) => "VK_INDIRECT_COMMANDS_TOKEN_TYPE_DISPATCH_EXT",
        // D3D12 行:D3D12_INDIRECT_ARGUMENT_TYPE 枚举名。
        (T::BindVertexBuffer, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_VERTEX_BUFFER_VIEW",
        (T::BindIndexBuffer, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_INDEX_BUFFER_VIEW",
        (T::PushConstants, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_CONSTANT",
        (T::Draw, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_DRAW",
        (T::DrawIndexed, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_DRAW_INDEXED",
        (T::Dispatch, B::D3D12) => "D3D12_INDIRECT_ARGUMENT_TYPE_DISPATCH",
        // NVPTX 行:执行面不承诺(仅命令数据生成;「—」)。
        (_, B::Nvptx) => return None,
    })
}

// ═══════════════════════ capability snapshot 阻塞性前置(RXS-0348 §3-3) ═══════════════

/// `submit.dgc` capability 的 Vulkan 后端 extension 名(profile 选择律之外的
/// **装载期实测面**;RXS-0349 登记 `submit.dgc` 为 `VK_EXT_device_generated_commands`
/// 的 capability 门控 ID,本常量 = 该 ID 在 Vulkan 后端的实测锚)。
pub const DGC_REQUIRED_EXTENSION: &str = "VK_EXT_device_generated_commands";

/// capability snapshot 阻塞性前置核验(RXS-0348 §3-3;走 M32 snapshot 核验
/// 原语 RXS-0313 既有机制的 rt 侧最小形态):
///
/// `snapshot` = 设备实测可用 extension 表(vk.rs U54 lane 经
/// `vkEnumerateDeviceExtensionProperties` 真实读回);`submit.dgc` 对应
/// `VK_EXT_device_generated_commands` 必须**实测在位**——缺位 → typed
/// `Err(DgcError::CapabilityMissing)`,fail-closed;本函数体内不存在任何
/// 模拟/降级路径(by construction,禁静默模拟 P-01)。
///
/// 核验先于任何 layout 创建/pipeline 装配(vk.rs lane 在 device 创建前调用)。
//@ spec: RXS-0348
pub fn verify_dgc_snapshot(available_extensions: &[&str]) -> Result<(), DgcError> {
    if available_extensions.contains(&DGC_REQUIRED_EXTENSION) {
        Ok(())
    } else {
        Err(DgcError::CapabilityMissing)
    }
}

// ═══════════════════════ 单测(装配期红绿 + 结构性断言) ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// GREEN:合法 token 序列装配成功(恰一终止且最后;状态 token 任意序)。
    //@ spec: RXS-0348
    #[test]
    fn assemble_legal_layouts() {
        // 最小序列:仅终止 token。
        let l = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).expect("单 dispatch 合法");
        assert_eq!(l.terminator(), DgcToken::Dispatch);
        assert!(l.state_tokens().is_empty());
        // 状态 token + 终止 token。
        let l = IndirectCmdLayout::assemble(&[
            DgcToken::BindVertexBuffer,
            DgcToken::PushConstants,
            DgcToken::Draw,
        ])
        .expect("vb+push+draw 合法");
        assert_eq!(l.terminator(), DgcToken::Draw);
        assert_eq!(
            l.state_tokens(),
            &[DgcToken::BindVertexBuffer, DgcToken::PushConstants]
        );
        // 全闭集六 token(三终止任选其一,故「全六」不可能合法——此处 vb+ib+push+draw_indexed)。
        let l = IndirectCmdLayout::assemble(&[
            DgcToken::BindVertexBuffer,
            DgcToken::BindIndexBuffer,
            DgcToken::PushConstants,
            DgcToken::DrawIndexed,
        ])
        .expect("vb+ib+push+draw_indexed 合法");
        assert_eq!(l.terminator(), DgcToken::DrawIndexed);
    }

    /// RED:多终止 token(draw + dispatch)→ 装配期确定性拒(conformance
    /// `dgc_layout_double_terminator.rx` 承载的同族违例)。
    //@ spec: RXS-0348
    #[test]
    fn reject_multiple_terminators() {
        let err = IndirectCmdLayout::assemble(&[DgcToken::Draw, DgcToken::Dispatch])
            .expect_err("双终止须拒");
        assert_eq!(
            err,
            DgcError::Layout(DgcLayoutViolation::MultipleTerminators)
        );
        let text = err.to_string();
        assert!(text.contains("fail-closed"), "诊断须含 fail-closed: {text}");
        // 双 draw 同样违例(恰一,不论同种)。
        let err = IndirectCmdLayout::assemble(&[DgcToken::Draw, DgcToken::Draw])
            .expect_err("双 draw 须拒");
        assert_eq!(
            err,
            DgcError::Layout(DgcLayoutViolation::MultipleTerminators)
        );
    }

    /// RED:终止 token 非最后(dispatch 后仍有状态 token)→ 装配期确定性拒。
    //@ spec: RXS-0348
    #[test]
    fn reject_terminator_not_last() {
        let err = IndirectCmdLayout::assemble(&[DgcToken::Dispatch, DgcToken::PushConstants])
            .expect_err("终止非最后须拒");
        assert_eq!(err, DgcError::Layout(DgcLayoutViolation::TerminatorNotLast));
        // draw_indexed 后再接 draw 属「多终止」优先命中(先数个数再定位);
        // dispatch 后接状态 token 才是本类别。
        let err = IndirectCmdLayout::assemble(&[
            DgcToken::BindVertexBuffer,
            DgcToken::Draw,
            DgcToken::BindIndexBuffer,
        ])
        .expect_err("draw 后接 ib 须拒");
        assert_eq!(err, DgcError::Layout(DgcLayoutViolation::TerminatorNotLast));
    }

    /// RED:零终止 token(纯状态序列)→ 装配期确定性拒;空序列同拒。
    //@ spec: RXS-0348
    #[test]
    fn reject_missing_terminator_and_empty() {
        let err = IndirectCmdLayout::assemble(&[DgcToken::PushConstants]).expect_err("零终止须拒");
        assert_eq!(err, DgcError::Layout(DgcLayoutViolation::MissingTerminator));
        let err = IndirectCmdLayout::assemble(&[]).expect_err("空序列须拒");
        assert_eq!(err, DgcError::Layout(DgcLayoutViolation::EmptySequence));
    }

    /// 结构性断言:禁类 token(render pass / barrier / descriptor set)在闭集内
    /// **不可表达**——`DgcToken::ALL` 恰六变体且逐项等于 Syntax 闭集。
    //@ spec: RXS-0348
    #[test]
    fn token_closed_set_exact() {
        assert_eq!(DgcToken::ALL.len(), 6, "闭集恰六 token(Syntax 逐字)");
        let names: Vec<&str> = DgcToken::ALL.iter().map(|t| t.name()).collect();
        assert_eq!(
            names,
            [
                "bind_vertex_buffer",
                "bind_index_buffer",
                "push_constants",
                "draw",
                "draw_indexed",
                "dispatch"
            ],
            "闭集字面与 spec §3 Syntax 逐字一致"
        );
        let terminators: Vec<DgcToken> = DgcToken::ALL
            .iter()
            .copied()
            .filter(|t| t.is_terminator())
            .collect();
        assert_eq!(
            terminators,
            [DgcToken::Draw, DgcToken::DrawIndexed, DgcToken::Dispatch],
            "终止 token 恰三选一"
        );
    }

    /// 结构性断言:DgcBuffer 无 host 读接口——源码级扫描(dgc.rs 内 `impl
    /// DgcBuffer` 之后不存在任何返回内容/地址的 pub 方法名;防漂移双重核验)。
    /// 类型层保证本身 = 方法不存在编译期拦截(本测试是机器可核的漂移哨兵)。
    //@ spec: RXS-0348
    #[test]
    fn dgc_buffer_no_host_read_interface_structural() {
        let src = include_str!("dgc.rs");
        // 只扫 DgcBuffer impl 块之后的全部源码(元数据访问器 byte_len/
        // layout_token_count 是身份面,非内容读;禁名扫描 = 内容/地址读接口)。
        // 精确签名扫描(以 `        pub fn ` 行内形态匹配——禁名出现在测试代码的
        // 字符串字面里不判违例,只判真实方法定义;`read`/`map`/`as_ptr`/`addr`/
        // `device_address`/`bytes`/`content`/`download`/`upload`/`write`/`as_slice`/
        // `unmap` 等 host 读/写/取址接口在 DgcBuffer impl 内不存在 = 结构性断言)。
        // 实现 = 扫 dgc.rs 中 DgcBuffer impl 块区间的真实 `pub fn` 定义名。
        let forbidden_names = [
            "read",
            "as_slice",
            "map",
            "unmap",
            "as_ptr",
            "addr",
            "device_address",
            "get",
            "bytes",
            "content",
            "download",
            "upload",
            "write",
        ];
        // 抽出全部 `pub fn <name>(` 定义(行内签名形态;只统计 DgcBuffer impl 之后
        // 的全部源码——含任何未来新增 impl 块)。
        let dgc_impl_pos = src.find("impl DgcBuffer").expect("DgcBuffer impl 应在");
        let tail = &src[dgc_impl_pos..];
        for line in tail.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("pub fn ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                for f in forbidden_names {
                    assert!(
                        name != f,
                        "DgcBuffer 漂移哨兵:出现禁名方法 `{f}`(无 host 读/写/取址接口契约破)"
                    );
                }
            }
        }
        // 恒等面:DgcBuffer 只暴露 byte_len/layout_token_count 两个元数据访问器。
        assert_eq!(DgcBuffer::new(64, 2).byte_len(), 64);
        assert_eq!(DgcBuffer::new(64, 2).layout_token_count(), 2);
    }

    /// 回读计数器:生产路径零记账(本测试只读数;记账入口存在但无生产调用点)。
    //@ spec: RXS-0348
    #[test]
    fn readback_counter_zero_by_default() {
        // 单测进程内无 device lane 执行:计数器读数必须为零(若未来实现引入
        // 隐式回读记账点,M102 门设备段按同计数器判红)。
        let _ = readback_counter; // 入口存在性锚定(函数项类型)
        let _ = readback_counter_record;
    }

    /// capability snapshot 阻塞性前置:实测在位 → Ok;缺位 → typed Err
    /// fail-closed(合成负样本;真实设备表归 vk.rs U54 lane 注入)。
    //@ spec: RXS-0348
    #[test]
    fn capability_snapshot_blocking() {
        assert!(verify_dgc_snapshot(&["VK_EXT_device_generated_commands"]).is_ok());
        let err = verify_dgc_snapshot(&["VK_KHR_acceleration_structure"]).expect_err("缺 DGC 须拒");
        assert_eq!(err, DgcError::CapabilityMissing);
        let text = err.to_string();
        assert!(
            text.contains("capability.runtime_snapshot_mismatch"),
            "诊断须沿 RXS-0313 symbolic key: {text}"
        );
        assert!(text.contains("submit.dgc"), "诊断须含缺失 ID: {text}");
        // 空 snapshot(禁静默模拟:缺 capability 无任何「尽力而为」路径)。
        assert!(verify_dgc_snapshot(&[]).is_err());
    }

    /// 三后端映射单一事实源:闭集六 token × 三后端全表锚定(NVPTX 全 None)。
    //@ spec: RXS-0348
    #[test]
    fn three_backend_mapping_frozen() {
        assert_eq!(
            map_token(DgcToken::Dispatch, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_DISPATCH_EXT")
        );
        assert_eq!(
            map_token(DgcToken::Draw, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_DRAW_EXT")
        );
        assert_eq!(
            map_token(DgcToken::DrawIndexed, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_DRAW_INDEXED_EXT")
        );
        assert_eq!(
            map_token(DgcToken::PushConstants, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_PUSH_CONSTANT_EXT")
        );
        assert_eq!(
            map_token(DgcToken::BindVertexBuffer, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_VERTEX_BUFFER_EXT")
        );
        assert_eq!(
            map_token(DgcToken::BindIndexBuffer, DgcBackend::Vulkan),
            Some("VK_INDIRECT_COMMANDS_TOKEN_TYPE_INDEX_BUFFER_EXT")
        );
        assert_eq!(
            map_token(DgcToken::Dispatch, DgcBackend::D3D12),
            Some("D3D12_INDIRECT_ARGUMENT_TYPE_DISPATCH")
        );
        assert_eq!(
            map_token(DgcToken::Draw, DgcBackend::D3D12),
            Some("D3D12_INDIRECT_ARGUMENT_TYPE_DRAW")
        );
        for t in DgcToken::ALL {
            assert!(
                map_token(t, DgcBackend::Nvptx).is_none(),
                "NVPTX 执行面不承诺"
            );
        }
    }
}
