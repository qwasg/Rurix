//! 声明面(Frostbite 式,报告5 §2.1;RFC-0016 章 A)。
//!
//! 应用按线性序注册 pass 并逐 pass 显式声明读写;execute 闭包与声明分离(报告5 §3
//! render/pass 改造要求,vkguide setup/execute 双 lambda 先例)。[`CmdRecorder`] 本波次
//! 为**记录桩**:只记录 draw/dispatch/copy 意图,供 dump 与后续 device 腿逐字消费
//! (禁二次推导——rurix-rt graph.rs「执行器逐字重放」先例)。

use crate::graph::compile::{CompileOptions, CompiledGraph, GraphError};
use crate::graph::resources::ResourceNode;
use crate::graph::types::{PassDesc, PassId, ResourceDesc, ResourceId};

// ---------------------------------------------------------------------------
// 命令记录(执行意图;device 腿消费面)
// ---------------------------------------------------------------------------

/// 命令意图(封闭枚举;扩面只追加)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// 非索引绘制。
    Draw {
        /// 顶点数。
        vertex_count: u32,
        /// 实例数。
        instance_count: u32,
    },
    /// 索引绘制。
    DrawIndexed {
        /// 索引数。
        index_count: u32,
        /// 实例数。
        instance_count: u32,
    },
    /// compute 派发。
    Dispatch {
        /// 工作组 x。
        group_count_x: u32,
        /// 工作组 y。
        group_count_y: u32,
        /// 工作组 z。
        group_count_z: u32,
    },
    /// 资源间拷贝。
    Copy {
        /// 源资源。
        src: ResourceId,
        /// 目的资源。
        dst: ResourceId,
        /// 拷贝字节数。
        byte_size: u64,
    },
}

/// 一条已记录命令(携带所属 pass,执行序 = 图线性序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedCommand {
    /// 所属 pass。
    pub pass: PassId,
    /// 命令意图。
    pub kind: CommandKind,
}

/// pass 执行期命令记录器(纯记录桩:不触碰任何 GPU 面,host 可单测)。
pub struct CmdRecorder {
    pass: PassId,
    commands: Vec<RecordedCommand>,
}

impl CmdRecorder {
    pub(crate) fn new(pass: PassId) -> CmdRecorder {
        CmdRecorder {
            pass,
            commands: Vec::new(),
        }
    }

    /// 当前录制所属 pass。
    pub fn pass(&self) -> PassId {
        self.pass
    }

    /// 记录一次非索引绘制。
    pub fn draw(&mut self, vertex_count: u32, instance_count: u32) {
        self.push(CommandKind::Draw {
            vertex_count,
            instance_count,
        });
    }

    /// 记录一次索引绘制。
    pub fn draw_indexed(&mut self, index_count: u32, instance_count: u32) {
        self.push(CommandKind::DrawIndexed {
            index_count,
            instance_count,
        });
    }

    /// 记录一次 compute 派发。
    pub fn dispatch(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        self.push(CommandKind::Dispatch {
            group_count_x,
            group_count_y,
            group_count_z,
        });
    }

    /// 记录一次资源间拷贝。
    pub fn copy(&mut self, src: ResourceId, dst: ResourceId, byte_size: u64) {
        self.push(CommandKind::Copy {
            src,
            dst,
            byte_size,
        });
    }

    fn push(&mut self, kind: CommandKind) {
        self.commands.push(RecordedCommand {
            pass: self.pass,
            kind,
        });
    }

    pub(crate) fn finish(self) -> Vec<RecordedCommand> {
        self.commands
    }
}

/// 一次图执行的完整命令记录(pass 线性序拼接)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLog {
    pub(crate) commands: Vec<RecordedCommand>,
}

impl CommandLog {
    /// 全部已记录命令。
    pub fn commands(&self) -> &[RecordedCommand] {
        &self.commands
    }

    /// 命令条数。
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

// ---------------------------------------------------------------------------
// 图声明面
// ---------------------------------------------------------------------------

/// pass execute 闭包载体(声明与执行分离的托管形态;随编译移交产物)。
pub type PassExecute = Box<dyn FnMut(&mut CmdRecorder)>;

/// pass 节点(声明 + execute 闭包;闭包随编译进入 [`CompiledGraph`],被剔除即丢弃)。
pub(crate) struct PassNode {
    pub(crate) id: PassId,
    pub(crate) desc: PassDesc,
    pub(crate) execute: Option<PassExecute>,
}

/// Frostbite 式声明式 render graph(声明面)。
///
/// 生命周期:`create` / `import` 注册资源 → `add_pass` / `add_pass_with` 按线性序
/// 注册 pass(声明序 = 依赖语义序,报告5 §2.1)→ [`RenderGraph::compile`] 消耗图
/// 产出 [`CompiledGraph`]。pass 注册强制携带 name(`PassDesc` 契约面;报告5 §6
/// 调试上下文缓解:强制名字自 P0 保留)。
pub struct RenderGraph {
    pub(crate) resources: Vec<ResourceNode>,
    pub(crate) passes: Vec<PassNode>,
}

impl RenderGraph {
    /// 新建空图。
    #[must_use]
    pub fn new() -> RenderGraph {
        RenderGraph {
            resources: Vec::new(),
            passes: Vec::new(),
        }
    }

    /// 创建 transient 资源(帧内即生即灭;`desc.imported` 强制覆写为 false)。
    pub fn create(&mut self, desc: ResourceDesc) -> ResourceId {
        self.add_resource(desc, false)
    }

    /// import 外部资源(跨帧历史/backbuffer/流送产物;`desc.imported` 强制覆写为
    /// true——图只推导状态转换,不入 transient 池、不参与别名,报告5 §2.3 约束一)。
    pub fn import(&mut self, desc: ResourceDesc) -> ResourceId {
        self.add_resource(desc, true)
    }

    fn add_resource(&mut self, mut desc: ResourceDesc, imported: bool) -> ResourceId {
        desc.imported = imported;
        let id =
            ResourceId(u32::try_from(self.resources.len()).expect("resource count overflow u32"));
        self.resources.push(ResourceNode { id, desc });
        id
    }

    /// 注册无执行闭包的 pass(纯声明;线性序追加)。
    pub fn add_pass(&mut self, desc: PassDesc) -> PassId {
        let id = PassId(u32::try_from(self.passes.len()).expect("pass count overflow u32"));
        self.passes.push(PassNode {
            id,
            desc,
            execute: None,
        });
        id
    }

    /// 注册携带 execute 闭包的 pass(声明与执行分离;闭包随编译移交产物)。
    pub fn add_pass_with<F>(&mut self, desc: PassDesc, execute: F) -> PassId
    where
        F: FnMut(&mut CmdRecorder) + 'static,
    {
        let id = PassId(u32::try_from(self.passes.len()).expect("pass count overflow u32"));
        self.passes.push(PassNode {
            id,
            desc,
            execute: Some(Box::new(execute)),
        });
        id
    }

    /// pass 数。
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }

    /// 资源数。
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// 编译(消耗图):四趟编译 + 编译期校验,产出 [`CompiledGraph`];
    /// 违例确定性拒,见 [`GraphError`]。
    ///
    /// # Errors
    /// 见 [`GraphError`]:读未写 / 同 pass 冲突 / 越期句柄 / 重复 Present / 异步依赖环。
    pub fn compile(self, options: CompileOptions) -> Result<CompiledGraph, GraphError> {
        crate::graph::compile::compile(self, options)
    }
}

impl Default for RenderGraph {
    fn default() -> RenderGraph {
        RenderGraph::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{AccessKind, QueueClass, ResAccess, ResourceKind, TextureFormat};

    fn rd(name: &str) -> ResourceDesc {
        ResourceDesc {
            name: name.to_owned(),
            kind: ResourceKind::Texture2d {
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                mip_levels: 1,
            },
            imported: false,
        }
    }

    fn pd(name: &str, reads: Vec<ResAccess>, writes: Vec<ResAccess>) -> PassDesc {
        PassDesc {
            name: name.to_owned(),
            queue: QueueClass::Graphics,
            reads,
            writes,
        }
    }

    fn ra(res: ResourceId, access: AccessKind) -> ResAccess {
        ResAccess { res, access }
    }

    /// 声明面:id 线性递增;create/import 强制覆写 imported 标志。
    #[test]
    fn declaration_surface_ids_and_imported_flags() {
        let mut g = RenderGraph::new();
        let t = g.create(rd("t"));
        let e = g.import(rd("e")); // desc.imported=false,import() 强制覆写为 true
        assert_eq!(t, ResourceId(0));
        assert_eq!(e, ResourceId(1));
        assert_eq!(g.resource_count(), 2);
        let p0 = g.add_pass(pd("mk", vec![], vec![ra(t, AccessKind::ColorTarget)]));
        let p1 = g.add_pass(pd(
            "use",
            vec![ra(t, AccessKind::ShaderRead)],
            vec![ra(e, AccessKind::ColorTarget)],
        ));
        assert_eq!((p0, p1), (PassId(0), PassId(1)));
        assert_eq!(g.pass_count(), 2);
        let c = g.compile(CompileOptions::default()).expect("合法图");
        assert!(!c.resource(t).expect("t 幸存").imported());
        assert!(c.resource(e).expect("e 幸存").imported());
    }

    /// execute 闭包随编译移交:按线性序记录意图;被剔除 pass 的闭包不执行。
    #[test]
    fn execute_records_intents_and_skips_culled() {
        let mut g = RenderGraph::new();
        let keep = g.create(rd("keep"));
        let dead = g.create(rd("dead"));
        let out = g.import(rd("out"));
        g.add_pass_with(
            pd("produce", vec![], vec![ra(keep, AccessKind::ColorTarget)]),
            |cmd| {
                cmd.draw(3, 1);
            },
        );
        // 无贡献 pass:写无人消费的 transient → 剔除,闭包不得执行。
        g.add_pass_with(
            pd("dead_pass", vec![], vec![ra(dead, AccessKind::ColorTarget)]),
            |cmd| {
                cmd.draw(99, 1);
            },
        );
        g.add_pass_with(
            pd(
                "consume",
                vec![ra(keep, AccessKind::ShaderRead)],
                vec![ra(out, AccessKind::ColorTarget)],
            ),
            move |cmd| {
                cmd.dispatch(8, 8, 1);
                cmd.copy(keep, out, 64);
            },
        );
        let mut c = g.compile(CompileOptions::default()).expect("合法图");
        let log = c.execute();
        assert_eq!(
            log.commands(),
            &[
                RecordedCommand {
                    pass: PassId(0),
                    kind: CommandKind::Draw {
                        vertex_count: 3,
                        instance_count: 1
                    },
                },
                RecordedCommand {
                    pass: PassId(2),
                    kind: CommandKind::Dispatch {
                        group_count_x: 8,
                        group_count_y: 8,
                        group_count_z: 1
                    },
                },
                RecordedCommand {
                    pass: PassId(2),
                    kind: CommandKind::Copy {
                        src: keep,
                        dst: out,
                        byte_size: 64
                    },
                },
            ]
        );
        // FnMut 闭包可重入:二次执行记录相同。
        assert_eq!(c.execute(), log);
    }
}
