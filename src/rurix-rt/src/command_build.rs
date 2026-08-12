//! `command_build` — G9.3 M105 command build node(g9.p1.m105.command_build_node;
//! spec/gpu_driven_submit.md RXS-0354;RFC-0023 §4.4)。
//!
//! 纯 host、safe(本模块零 `unsafe`)、always-on(不随 `vulkan` feature 门控):
//!
//! - **图节点语义**(RXS-0354 L1;RFC-0023 §4.4.1 逐字):command build node =
//!   render graph 内 compute pre-pass,GPU 上产出命令数据,后续 indirect-draw pass
//!   消费;「GPU 端生成 → GPU 端消费」是 RXS-0239 单 queue 全序内的数据流,不引入
//!   pass 内重排语义。图编排面**消费** `graph.rs` RXS-0346 既有冻结面
//!   ([`Graph::dgc_buffer`] / `reads_writes_uav` / `reads_indirect` /
//!   [`Graph::declare_indirect_dispatch`],字面 0-byte 复用不重定):producer pass
//!   以 `UavReadWrite` 写 DgcBuffer(command build node 唯一合法写形态),consumer
//!   pass 以 `IndirectCommandRead` 读,依赖边 `StorageWrite→IndirectCommandRead`
//!   经 `derive_barriers` 自然成边。
//! - **零 CPU 回读结构性强约束**(RXS-0354 L2/L4;RFC-0023 §4.1.1/§4.4.2 逐字):
//!   host 侧对 DgcBuffer 命令数据的读接口**不存在**(dgc.rs RXS-0348 L2 类型层
//!   契约,本模块不新增任何内容读路径);**回读计数器恒 0**——任何隐式回读(含
//!   调试路径)必须经 `dgc::readback_counter_record` 显式记账,非零即红;
//!   [`assert_zero_readback_since`] = 机器核验面(结构性断言 + 计数器双承担)。
//! - **构建产物确定性**(RXS-0354 L3;RFC-0023 §4.0-3/§4.4 逐字):同一输入 +
//!   同一构建器版本下,command build node 产出的命令数据内容流与 host 参照
//!   **逐字节一致**;同输入双构建 digest 相等。[`build_reference`] = host 参照
//!   构建器(确定性纯函数)。DGC buffer 物理字节格式(preprocess 产物)为实现
//!   确定、非 stable(RXS-0348 §3-6 同口径)——本模块只冻结内容流一致性与
//!   确定性,不冻结数值布局。
//!
//! device 接线点(留 CI 门代理,`ci/g9_command_build_node_smoke.py`,symbolic key
//! `g9.p1.m105.command_build_node`):device 侧经 vk.rs U54 lane 执行构建 kernel
//! (如 `kernels/dgc_prepass.rx` 产物)直写 DgcBuffer,产物字节与
//! [`build_reference`] 输出逐字节比对;本 host 面不触 device,不存在任何回读
//! 调用点(by construction)。

use crate::dgc::{self, DgcToken, IndirectCmdLayout};
use crate::graph::{AccessKind, Graph, GraphError, PassSpec, ResourceId};

// ═══════════════════════ 参数页与 host 参照构建器(RXS-0354 L3) ═══════════════════════

/// 参数页(command build node 的构建输入;host 侧声明面)。
///
/// device 侧对应 GPU 只读页:构建 kernel 经只读视图消费参数页、直写 DgcBuffer,
/// 全程零 CPU 回读(RXS-0354 L2)。首期内容流只承载**终止 token 参数**(状态
/// token 零载荷,与 vk.rs U54 段「vb/ib/push 不携数据载荷进 DgcBuffer」同一
/// 口径),故参数页字宽必须**恰等于**终止 token 参数元数(见
/// [`terminator_arity`]),不符 = 装配期确定性拒绝(fail-closed)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParameterPage {
    words: Vec<u32>,
}

impl ParameterPage {
    /// 自 u32 字列构造(声明序 = 参数序)。
    #[must_use]
    pub fn from_words(words: &[u32]) -> Self {
        ParameterPage {
            words: words.to_vec(),
        }
    }

    /// 参数字列(只读)。
    #[must_use]
    pub fn words(&self) -> &[u32] {
        &self.words
    }
}

/// 终止 token 的 indirect 参数 u32 字宽(内容流语义布局,与 vk.rs U54 段
/// 「draw = VkDrawIndirectCommand 16B / dispatch = VkDispatchIndirectCommand
/// 12B」同一事实源;draw_indexed = VkDrawIndexedIndirectCommand 20B 同族)。
/// 状态 token 返回 `None`(零载荷)。实现确定、非 stable 物理布局的**语义内容
/// 流**面(RXS-0348 §3-6:只作存在性/确定性声明,不冻结数值布局)。
#[must_use]
pub const fn terminator_arity(token: DgcToken) -> Option<u32> {
    match token {
        DgcToken::Draw => Some(4),
        DgcToken::DrawIndexed => Some(5),
        DgcToken::Dispatch => Some(3),
        DgcToken::BindVertexBuffer | DgcToken::BindIndexBuffer | DgcToken::PushConstants => None,
    }
}

/// command build 装配/断言失败的 typed `Err`(fail-closed;库面装配诊断沿
/// RX6029 族/dgc.rs `DgcError` 先例,不占 RX 码)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CommandBuildError {
    /// 参数页字宽与终止 token 参数元数不符(内容流无法确定性物化)。
    ParamPageMismatch {
        /// 终止 token 要求的参数 u32 字宽。
        expected: u32,
        /// 参数页实际字宽。
        actual: u32,
    },
    /// 全链路零 CPU 回读违例:readback_counter 增量非零(RFC-0023 §4.4.2
    /// 「回读计数器 = 0」断言;非零即红)。
    ReadbackDetected {
        /// 自基线起的回读记账增量。
        delta: u64,
    },
}

impl std::fmt::Display for CommandBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandBuildError::ParamPageMismatch { expected, actual } => write!(
                f,
                "command build 参数页字宽 {actual} 与终止 token 参数元数 {expected} 不符\
                 (fail-closed,RXS-0354;状态 token 首期零载荷,参数页仅承载终止参数)"
            ),
            CommandBuildError::ReadbackDetected { delta } => write!(
                f,
                "command build node 全链路零 CPU 回读违例:readback_counter 增量 = {delta} \
                 (非零即红,RFC-0023 §4.4.2 / RXS-0354 L2;调试 dump 唯一通道 = 显式 \
                 readback pass `g.readback`,RXS-0236)"
            ),
        }
    }
}

impl std::error::Error for CommandBuildError {}

/// host 参照构建器(RXS-0354 L3;**确定性纯函数**):已装配核验的 layout + 参数页
/// → 命令数据内容流字节。
///
/// 内容流 = 终止 token 参数的 u32 小端字列(字宽 = [`terminator_arity`];与
/// vk.rs U54 段 device 面 DgcBuffer 首期数据载荷逐字节同构——draw 16B /
/// draw_indexed 20B / dispatch 12B;状态 token 零载荷)。同输入双构建输出
/// 逐字节相等;device 产物与本输出的逐字节比对接线点留 CI 门代理
/// (`ci/g9_command_build_node_smoke.py`)。
///
/// # Errors
/// 参数页字宽 ≠ 终止 token 参数元数 → [`CommandBuildError::ParamPageMismatch`]。
//@ spec: RXS-0354
pub fn build_reference(
    layout: &IndirectCmdLayout,
    params: &ParameterPage,
) -> Result<Vec<u8>, CommandBuildError> {
    // layout 已经 IndirectCmdLayout::assemble 核验(恰一终止且最后),terminator
    // 必为 draw/draw_indexed/dispatch 之一 → arity 恒 Some。
    let arity = terminator_arity(layout.terminator()).unwrap_or(0);
    let actual = u32::try_from(params.words().len()).unwrap_or(u32::MAX);
    if actual != arity {
        return Err(CommandBuildError::ParamPageMismatch {
            expected: arity,
            actual,
        });
    }
    let mut out = Vec::with_capacity(arity as usize * 4);
    for w in params.words() {
        out.extend_from_slice(&w.to_le_bytes());
    }
    Ok(out)
}

// ═══════════════════════ 零 CPU 回读机器核验(RXS-0354 L2/L4) ═══════════════════════

/// 回读基线快照(全链路零回读断言的起点;= dgc.rs 进程级单一计数器读数,
/// RFC-0023 §4.4.2 配套验收面)。
//@ spec: RXS-0354
#[must_use]
pub fn readback_baseline() -> u64 {
    dgc::readback_counter()
}

/// 全链路零 CPU 回读机器核验(RXS-0354 L4:结构性断言 + readback_counter=0
/// 双承担):自 `baseline` 起 readback_counter 增量必须 == 0——任何隐式回读
/// (含调试路径)须经计数器显式记账,**非零即红**;调试 dump 唯一通道 = 显式
/// readback pass(`g.readback` 既有面,RXS-0236)。本模块生产路径不存在任何
/// 记账调用点(by construction;`dgc::readback_counter_record` 的唯一入口地位
/// 见 dgc.rs)。
///
/// # Errors
/// 增量非零 → [`CommandBuildError::ReadbackDetected`]。
//@ spec: RXS-0354
pub fn assert_zero_readback_since(baseline: u64) -> Result<(), CommandBuildError> {
    let delta = dgc::readback_counter().saturating_sub(baseline);
    if delta == 0 {
        Ok(())
    } else {
        Err(CommandBuildError::ReadbackDetected { delta })
    }
}

// ═══════════════════════ command build node 图节点(RXS-0354 L1) ═══════════════════════

/// command build node(render graph 内 compute pre-pass 图节点声明面,RXS-0354 L1):
/// 输入 = 参数页(经 [`ParameterPage`])+ DgcBuffer 输出声明;构建 kernel 派生 =
/// 节点身份(kernel 名 + artifact digest,device 执行面归 vk.rs U54 lane /
/// CI 门代理接线);输出 indirect buffer 供 indirect pass 消费(graph.rs
/// `AccessKind::IndirectCommandRead` 边,RXS-0346 既有面)。
///
/// 合法实例的 layout 只能经 `dgc::IndirectCmdLayout::assemble` 取得(token 限制
/// 装配期核验 fail-closed,RXS-0348);本节点不复制该核验(单一事实源)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandBuildNode {
    /// 节点诊断名(同时作 DgcBuffer 资源名与 pass 名前缀)。
    name: String,
    /// 已装配核验的命令 layout(token 闭集 + 终止 token 纪律,RXS-0348)。
    layout: IndirectCmdLayout,
    /// 构建 kernel 名(compute pre-pass kernel 身份;device 派生面归 CI 门代理)。
    kernel_name: String,
    /// 构建 kernel artifact digest(身份面;构建器版本敏感性的确定性锚)。
    kernel_digest: [u8; 32],
}

/// 图节点挂接产物(资源/ pass 下标句柄;供编排核验与测试锚定)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CommandBuildHandles {
    /// 分配的 DgcBuffer 资源(graph.rs `Graph::dgc_buffer` 产物)。
    pub dgc_buffer: ResourceId,
    /// producer(command build node)pass 下标(声明序)。
    pub producer_pass: usize,
    /// consumer(indirect)pass 下标(声明序)。
    pub consumer_pass: usize,
}

impl CommandBuildNode {
    /// 新建节点声明面(layout 须先经 `IndirectCmdLayout::assemble` 核验;
    /// `kernel_name`/`kernel_digest` = 构建 kernel 身份,device 派生面归 CI 门代理)。
    #[must_use]
    pub fn new(
        name: &str,
        layout: IndirectCmdLayout,
        kernel_name: &str,
        kernel_digest: [u8; 32],
    ) -> Self {
        CommandBuildNode {
            name: name.to_owned(),
            layout,
            kernel_name: kernel_name.to_owned(),
            kernel_digest,
        }
    }

    /// 已核验 layout(只读)。
    #[must_use]
    pub fn layout(&self) -> &IndirectCmdLayout {
        &self.layout
    }

    /// 构建 kernel 名(只读)。
    #[must_use]
    pub fn kernel_name(&self) -> &str {
        &self.kernel_name
    }

    /// 构建 kernel artifact digest(只读)。
    #[must_use]
    pub fn kernel_digest(&self) -> &[u8; 32] {
        &self.kernel_digest
    }

    /// producer pass 声明(command build node = compute pre-pass,`reads_writes_uav`
    /// 写 DgcBuffer——DgcBuffer 的唯一合法写形态,RXS-0346)。
    #[must_use]
    pub fn producer_spec(&self, dgc_buffer: ResourceId) -> PassSpec {
        PassSpec::new(&format!("{}:build", self.name)).reads_writes_uav(dgc_buffer)
    }

    /// consumer pass 声明(indirect pass,`reads_indirect` 消费 DgcBuffer;
    /// RXS-0354 L1「声明 `reads_indirect(dgc_buf)`」逐字面)。
    #[must_use]
    pub fn consumer_spec(&self, dgc_buffer: ResourceId) -> PassSpec {
        PassSpec::new(&format!("{}:consume", self.name)).reads_indirect(dgc_buffer)
    }

    /// 图节点挂接(RXS-0354 L1;依赖推导接入 = graph.rs RXS-0346 既有面,字面
    /// 0-byte 复用):分配 DgcBuffer 资源 → 追加 producer pass → 追加 consumer
    /// pass(`reads_indirect` + `consumer_extras` 附加访问,如 indirect-draw 的
    /// RT 写)→ 登记 `IndirectDispatch` 编排边(seal() 逐边 strict 核验:
    /// 漏声明/类别违例 → RX6029,既有判据不降格)。
    ///
    /// `consumer_extras` 元素 = `(资源, AccessKind)`(经 `PassSpec::with` 追加;
    /// 同资源重复声明等图结构违例由 `Graph::seal` 裁定)。
    ///
    /// # Errors
    /// seal 后建面调用 → [`GraphError::Structure`](RX6029);装配期核验在 `seal()`。
    pub fn mount(
        &self,
        g: &mut Graph,
        consumer_extras: &[(ResourceId, AccessKind)],
    ) -> Result<CommandBuildHandles, GraphError> {
        let dgc_buffer = g.dgc_buffer(&self.name);
        let producer_pass = g.pass_count();
        g.add_pass(self.producer_spec(dgc_buffer))?;
        let consumer_pass = g.pass_count();
        let mut consumer = self.consumer_spec(dgc_buffer);
        for &(resource, kind) in consumer_extras {
            consumer = consumer.with(resource, kind);
        }
        g.add_pass(consumer)?;
        g.declare_indirect_dispatch(producer_pass, consumer_pass, dgc_buffer)?;
        Ok(CommandBuildHandles {
            dgc_buffer,
            producer_pass,
            consumer_pass,
        })
    }
}

// ═══════════════════════ 单测(图节点绿/红 + 零回读 + 产物确定性) ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dgc::DgcToken;
    use crate::graph::{BarrierForm, D3d12State};

    /// 基准节点:dispatch 终止 token 的 command build node(镜像
    /// `kernels/dgc_prepass.rx` 的消费范式)。
    fn dispatch_node() -> CommandBuildNode {
        let layout = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).expect("合法 layout");
        CommandBuildNode::new("cull_build", layout, "dgc_prepass", [0xD6; 32])
    }

    /// GREEN(RXS-0354 L1):节点挂接 → 图装配/推导通过;`StorageWrite→
    /// IndirectCommandRead` 边在 barrier 计划中自然成边(BufferSync,
    /// UnorderedAccess→IndirectArgument,录于 consumer pass 边界);句柄下标正确。
    //@ spec: RXS-0354
    #[test]
    fn mounts_indirect_edge_and_derives_barrier_green() {
        let node = dispatch_node();
        let mut g = Graph::new();
        let out = g.color_target("draw_out");
        let handles = node
            .mount(&mut g, &[(out, AccessKind::ColorAttachmentWrite)])
            .expect("挂接须成功");
        assert_eq!(handles.producer_pass, 0);
        assert_eq!(handles.consumer_pass, 1);
        let plan = g.execute().expect("完整声明图 execute 通过");
        // dgc: Common→UnorderedAccess(producer)+ UnorderedAccess→IndirectArgument(新边)。
        let edge = plan
            .iter()
            .find(|b| b.d3d12_after == D3d12State::IndirectArgument)
            .expect("IndirectCommandRead 边 barrier 缺失");
        assert_eq!(edge.form, BarrierForm::BufferSync, "buffer 无 layout");
        assert_eq!(edge.d3d12_before, D3d12State::UnorderedAccess);
        assert_eq!(edge.at_pass, handles.consumer_pass);
        assert_eq!(edge.resource, handles.dgc_buffer);
        // 同图双跑逐字节等值(推导确定性,RXS-0346 既有判据同源)。
        let mut g2 = Graph::new();
        let out2 = g2.color_target("draw_out");
        node.mount(&mut g2, &[(out2, AccessKind::ColorAttachmentWrite)])
            .unwrap();
        g2.seal().unwrap();
        let mut g3 = Graph::new();
        let out3 = g3.color_target("draw_out");
        node.mount(&mut g3, &[(out3, AccessKind::ColorAttachmentWrite)])
            .unwrap();
        g3.seal().unwrap();
        assert_eq!(g2.derive_barriers(), g3.derive_barriers());
    }

    /// RED(RXS-0354 L1 配套 strict;RXS-0346 判据不降格):indirect pass 消费
    /// DgcBuffer(编排边登记)但 consumer 声明缺 `reads_indirect` → 装配期
    /// RX6029 strict 拒。
    //@ spec: RXS-0354
    #[test]
    fn broken_consumer_declaration_red() {
        let node = dispatch_node();
        let mut g = Graph::new();
        let dgc = g.dgc_buffer(node.kernel_name());
        let out = g.color_target("draw_out");
        // producer 经节点声明面(唯一合法写形态);consumer 人为漏 reads_indirect。
        g.add_pass(node.producer_spec(dgc)).unwrap();
        g.add_pass(PassSpec::new("indirect_draw").writes_rt(out))
            .unwrap();
        g.declare_indirect_dispatch(0, 1, dgc).unwrap();
        match g.seal() {
            Err(e @ GraphError::Structure { .. }) => {
                assert_eq!(e.rx_code(), "RX6029");
                assert!(
                    e.to_string().contains("reads_indirect"),
                    "诊断须点名漏声明: {e}"
                );
            }
            other => panic!("漏声明 indirect 读边应 RX6029,实得 {other:?}"),
        }
    }

    /// 内容流 golden(RXS-0354 L3):dispatch(1,1,1) = 12 字节 `01 00 00 00`×3
    /// (与 `kernels/dgc_prepass.rx` 直写面逐字节同构;哨兵字属 harness 见证,
    /// 不在命令数据内容流);draw/draw_indexed 字宽锚定。
    //@ spec: RXS-0354
    #[test]
    fn reference_content_stream_golden() {
        let layout = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).unwrap();
        let bytes = build_reference(&layout, &ParameterPage::from_words(&[1, 1, 1]))
            .expect("dispatch(1,1,1) 合法");
        assert_eq!(
            bytes,
            vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0],
            "VkDispatchIndirectCommand{{x=1,y=1,z=1}} 12B 内容流"
        );
        let layout = IndirectCmdLayout::assemble(&[DgcToken::Draw]).unwrap();
        let bytes = build_reference(&layout, &ParameterPage::from_words(&[3, 1, 0, 0]))
            .expect("draw(3,1,0,0) 合法");
        assert_eq!(bytes.len(), 16, "VkDrawIndirectCommand 16B");
        assert_eq!(
            bytes,
            vec![3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let layout = IndirectCmdLayout::assemble(&[
            DgcToken::BindVertexBuffer,
            DgcToken::BindIndexBuffer,
            DgcToken::DrawIndexed,
        ])
        .unwrap();
        let bytes =
            build_reference(&layout, &ParameterPage::from_words(&[6, 1, 0, 0, 0])).unwrap();
        assert_eq!(bytes.len(), 20, "VkDrawIndexedIndirectCommand 20B(状态 token 零载荷)");
    }

    /// 构建产物确定性(RXS-0354 L3):同输入双构建逐字节相等;参数微扰必改字节;
    /// 参数页字宽不符 → 装配期 typed `Err`(fail-closed,非 panic)。
    //@ spec: RXS-0354
    #[test]
    fn reference_build_deterministic_and_fail_closed() {
        let layout = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).unwrap();
        let params = ParameterPage::from_words(&[8, 4, 2]);
        let a = build_reference(&layout, &params).unwrap();
        let b = build_reference(&layout, &params).unwrap();
        assert_eq!(a, b, "同输入双构建逐字节相等");
        let perturbed = build_reference(&layout, &ParameterPage::from_words(&[8, 4, 3])).unwrap();
        assert_ne!(a, perturbed, "参数微扰必改内容流");
        // 字宽不符(少/多)→ ParamPageMismatch,typed Err。
        for bad in [&[1u32, 1][..], &[1, 1, 1, 0][..]] {
            let err = build_reference(&layout, &ParameterPage::from_words(bad))
                .expect_err("字宽不符须拒");
            assert_eq!(
                err,
                CommandBuildError::ParamPageMismatch {
                    expected: 3,
                    actual: bad.len() as u32,
                }
            );
        }
    }

    /// 零 CPU 回读(RXS-0354 L2/L4):全链路(layout 装配 → 参照构建 → 图节点
    /// 挂接 → seal/execute)readback_counter 增量 == 0 机器核验 GREEN;**RED
    /// 自测**:显式注入一次回读记账后同一断言必红(能红反证)。单测试函数
    /// 内串行(进程级计数器,避免并行用例互染)。
    //@ spec: RXS-0354
    #[test]
    fn zero_readback_full_chain_green_and_injected_red() {
        let baseline = readback_baseline();
        // 全链路:host 侧不存在任何 readback 记账调用点(by construction)。
        let node = dispatch_node();
        let stream = build_reference(
            node.layout(),
            &ParameterPage::from_words(&[1, 1, 1]),
        )
        .unwrap();
        assert_eq!(stream.len(), 12);
        let mut g = Graph::new();
        let out = g.color_target("draw_out");
        node.mount(&mut g, &[(out, AccessKind::ColorAttachmentWrite)])
            .unwrap();
        g.execute().unwrap();
        assert_eq!(
            assert_zero_readback_since(baseline),
            Ok(()),
            "全链路零回读:计数器增量须为 0"
        );
        // RED 自测:注入一次显式回读记账 → 同一断言必须红(非零即红)。
        dgc::readback_counter_record(1);
        let err = assert_zero_readback_since(baseline).expect_err("注入回读后断言必红");
        assert_eq!(err, CommandBuildError::ReadbackDetected { delta: 1 });
        assert!(
            err.to_string().contains("零 CPU 回读"),
            "诊断须点名零回读违例: {err}"
        );
    }

    /// conformance 锚定语料消费(RXS-0354;`command_build_host_readback.rx`,
    /// 可消费不可改):锚文件存在且携带本条款号(RED 语义面 = 本模块
    /// `ReadbackDetected` / dgc.rs 类型层无 host 读接口)。
    //@ spec: RXS-0354
    #[test]
    fn conformance_anchor_consumed() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/gpu_driven_submit/reject/command_build_host_readback.rx");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读锚定语料 {}: {e}", path.display()));
        assert!(text.contains("RXS-0354"), "锚定语料须携条款号");
        assert!(text.contains("g9.p1.m105.command_build_node"));
    }
}
