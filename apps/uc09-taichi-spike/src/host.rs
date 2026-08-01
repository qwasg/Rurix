//! host 腿(恒跑,无 GPU/DLL 也绿):AOT 资产核验 + RenderGraph import/copy 计划。
//!
//! 两段(spec `run_g6_5_taichi_vulkan_aot_spike`「AOT 资产与生成脚本」+
//! RFC-0017 §4.E3 graph external import 消费接线的 host 规划面):
//! 1. **资产核验**:读 `assets/particles.tcm` + `particles.tcm.sha256`,实测 sha256
//!    与登记一致;生成脚本 `gen_particles_aot.py` 在树。资产缺失 = 硬 Err(仓内
//!    资产缺失即仓损坏,直接红);hash 不一致 = 断言红(出 JSON,exit 1)。
//! 2. **graph 计划**:`import` 外部资源 `taichi_particles`(Buffer,256B = 64×f32;
//!    device 腿 = TiRT 导出的 VkBuffer,只读引用、不入 transient 池)→ `create`
//!    transient `particles_copy` → `add_pass_with` 录 `cmd.copy(imported, copy,
//!    256)` → compile → CommandLog。剔除关闭:spike 单 pass 计划面,剔除根(写
//!    imported / Present)均不命中,开剔除会把唯一 pass 删掉、CommandLog 丢 copy
//!    记录(device 腿按 §4.E3 逐字消费该计划)。

use std::path::PathBuf;

use rurix_render::graph::types::{
    AccessKind, PassDesc, QueueClass, ResAccess, ResourceDesc, ResourceKind,
};
use rurix_render::graph::{CommandKind, CompileOptions, RenderGraph};

use crate::sha256::sha256_hex;

/// 粒子数(AOT 资产契约:kernel `fill_particles`,f32 × 64,p[i] = i*1.5+1.0)。
pub const PARTICLE_COUNT: u32 = 64;
/// 粒子 buffer 字节数(64 × f32)。
pub const PARTICLE_BYTES: u64 = PARTICLE_COUNT as u64 * 4;

/// host 腿汇总(JSON/exit 判定源;`tcm` 字节供 device 腿直接消费)。
pub struct HostSummary {
    /// particles.tcm 原始字节。
    pub tcm: Vec<u8>,
    /// 实测 sha256(小写 hex)。
    pub tcm_sha256: String,
    /// 登记 sha256(`.sha256` 文件首字段,小写化)。
    pub registered_sha256: String,
    /// 生成脚本在树。
    pub gen_script_present: bool,
    /// host 断言面(字段名冻结,smoke 消费)。
    pub asserts: Vec<(String, bool)>,
    /// 幸存 pass 数(剔除关闭 = 1)。
    pub graph_pass_count: usize,
    /// 编译产物资源数(= 2:imported + transient)。
    pub graph_resource_count: usize,
    /// 计划 copy 字节数(= 256;device 腿 §4.E3 第三段对拍基准)。
    pub copy_byte_size: u64,
}

impl HostSummary {
    /// host 断言全过(device 腿结论不入本判定,由调用方合流)。
    pub fn host_asserts_pass(&self) -> bool {
        self.asserts.iter().all(|(_, ok)| *ok)
    }

    /// 非 JSON 模式单行摘要。
    pub fn one_line(&self) -> String {
        format!(
            "tcm={}B sha256={} graph_passes={} copy={}B asserts_pass={}",
            self.tcm.len(),
            self.tcm_sha256,
            self.graph_pass_count,
            self.copy_byte_size,
            self.host_asserts_pass()
        )
    }
}

/// 资产目录(编译期锚定 crate 根;与 cwd 无关)。
fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// graph 计划核验结果(建图/编译/回放一次,host 腿与单测共用)。
struct GraphPlan {
    imported_marked: bool,
    imported_unpooled: bool,
    transient_pooled: bool,
    copy_records: u32,
    copy_byte_size: u64,
    total_commands: usize,
    pass_count: usize,
    resource_count: usize,
}

/// 建 spike 图并真编译:import `taichi_particles` → create `particles_copy` →
/// 单 pass 录 copy(imported → copy, [`PARTICLE_BYTES`])。
fn build_graph_plan() -> Result<GraphPlan, String> {
    let mut g = RenderGraph::new();
    // import 外部资源(desc.imported 字段由 import() 强制覆写 true;语义 = device
    // 腿 TiRT 导出的 VkBuffer,图只推导状态转换,不入 transient 池、不参与别名)。
    let imported = g.import(ResourceDesc {
        name: "taichi_particles".to_owned(),
        kind: ResourceKind::Buffer {
            size: PARTICLE_BYTES,
        },
        imported: false,
    });
    let copy = g.create(ResourceDesc {
        name: "particles_copy".to_owned(),
        kind: ResourceKind::Buffer {
            size: PARTICLE_BYTES,
        },
        imported: false,
    });
    g.add_pass_with(
        PassDesc {
            name: "tirt_particles_copy".to_owned(),
            queue: QueueClass::Graphics,
            reads: vec![ResAccess {
                res: imported,
                access: AccessKind::CopySrc,
            }],
            writes: vec![ResAccess {
                res: copy,
                access: AccessKind::CopyDst,
            }],
        },
        move |cmd| {
            cmd.copy(imported, copy, PARTICLE_BYTES);
        },
    );
    // 剔除/异步关闭(模块头注释:单 pass 计划面,开剔除必删唯一 pass)。
    let mut compiled = g
        .compile(CompileOptions {
            enable_culling: false,
            enable_async: false,
        })
        .map_err(|e| format!("spike 图编译失败(确定性拒): {e}"))?;
    let log = compiled.execute();

    let imp = compiled
        .resource(imported)
        .ok_or("imported 资源未进编译产物")?;
    let imported_marked = imp.imported();
    let imported_unpooled = imp.slot().is_none() && imp.lifetime().is_none();
    let cpy = compiled
        .resource(copy)
        .ok_or("transient copy 资源未进编译产物")?;
    let transient_pooled = !cpy.imported() && cpy.slot().is_some();

    let copy_sizes: Vec<u64> = log
        .commands()
        .iter()
        .filter_map(|c| match c.kind {
            CommandKind::Copy {
                src,
                dst,
                byte_size,
            } if src == imported && dst == copy => Some(byte_size),
            _ => None,
        })
        .collect();
    let copy_records = copy_sizes.len() as u32;
    let copy_byte_size = copy_sizes.first().copied().unwrap_or(0);
    Ok(GraphPlan {
        imported_marked,
        imported_unpooled,
        transient_pooled,
        copy_records,
        copy_byte_size,
        total_commands: log.len(),
        pass_count: compiled.passes().len(),
        resource_count: compiled.resources().len(),
    })
}

/// host 腿主流程(资产核验 + graph 计划;断言失败仍出 Ok 汇总,由 exit 判定红)。
pub fn run_host_leg() -> Result<HostSummary, String> {
    let dir = assets_dir();
    let tcm_path = dir.join("particles.tcm");
    let tcm = std::fs::read(&tcm_path)
        .map_err(|e| format!("资产缺失/不可读 {}: {e}", tcm_path.display()))?;
    let sha_path = dir.join("particles.tcm.sha256");
    let sha_text = std::fs::read_to_string(&sha_path)
        .map_err(|e| format!("sha256 登记缺失/不可读 {}: {e}", sha_path.display()))?;
    let registered_sha256 = sha_text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let gen_script_present = dir.join("gen_particles_aot.py").is_file();

    let tcm_sha256 = sha256_hex(&tcm);
    let plan = build_graph_plan()?;
    // ② 登记字段形态合法性(64 位小写 hex)先行计算,供断言面字面量消费。
    let sha256_form_ok = registered_sha256.len() == 64
        && registered_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());

    let asserts: Vec<(String, bool)> = vec![
        // ① .tcm 在位且非空(读成功已由上面 Err 兜底,此处锁非空)。
        ("asset_tcm_present".into(), !tcm.is_empty()),
        // ② 登记字段形态合法。
        ("asset_sha256_registered".into(), sha256_form_ok),
        // ③ 实测 sha256 与登记一致(资产本体核验;再生成物非逐位可复现,见侦查记录 §3)。
        ("asset_sha256_match".into(), tcm_sha256 == registered_sha256),
        // ④ 生成脚本在树(host 段 CI 核验三件套之一)。
        ("asset_gen_script_present".into(), gen_script_present),
        // ⑤ imported 资源标记正确(图只管理状态转换,不入 transient 池)。
        ("graph_import_marked".into(), plan.imported_marked),
        ("graph_import_not_pooled".into(), plan.imported_unpooled),
        // ⑥ transient copy 目标创建且入池(imported 的对偶面)。
        ("graph_transient_pooled".into(), plan.transient_pooled),
        // ⑦ CommandLog 恰含一条 copy 记录且 byte_size=256(device 腿逐字消费面)。
        (
            "graph_copy_recorded".into(),
            plan.copy_records == 1
                && plan.copy_byte_size == PARTICLE_BYTES
                && plan.total_commands == 1,
        ),
    ];

    Ok(HostSummary {
        tcm,
        tcm_sha256,
        registered_sha256,
        gen_script_present,
        asserts,
        graph_pass_count: plan.pass_count,
        graph_resource_count: plan.resource_count,
        copy_byte_size: plan.copy_byte_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: 「AOT 资产与生成脚本」Scenario 资产核验(host 段恒跑,不依赖 GPU)
    #[test]
    fn asset_hash_matches_registration() {
        let dir = assets_dir();
        let tcm = std::fs::read(dir.join("particles.tcm")).expect("particles.tcm 在位");
        let sha_text =
            std::fs::read_to_string(dir.join("particles.tcm.sha256")).expect(".sha256 在位");
        let registered = sha_text.split_whitespace().next().unwrap_or("");
        assert_eq!(registered.len(), 64, "登记须 64 位 hex: {registered}");
        assert_eq!(
            sha256_hex(&tcm),
            registered.to_ascii_lowercase(),
            "实测 sha256 与登记不一致"
        );
        assert!(dir.join("gen_particles_aot.py").is_file(), "生成脚本须在树");
    }

    //@ spec: RFC-0017 §4.E3 graph external import 消费接线(host 规划面)
    #[test]
    fn graph_import_marking_and_copy_record() {
        let plan = build_graph_plan().expect("spike 图编译");
        assert!(plan.imported_marked, "import() 须强制 imported=true");
        assert!(
            plan.imported_unpooled,
            "imported 不入 transient 池/无生命周期"
        );
        assert!(plan.transient_pooled, "transient copy 目标须入池");
        assert_eq!(plan.copy_records, 1, "CommandLog 恰一条匹配 copy 记录");
        assert_eq!(plan.copy_byte_size, PARTICLE_BYTES);
        assert_eq!(plan.total_commands, 1, "全图恰一条命令");
        assert_eq!((plan.pass_count, plan.resource_count), (1, 2));
    }

    //@ spec: host 腿全链(仓内资产 + 图计划)断言恒绿
    #[test]
    fn host_leg_asserts_all_pass() {
        let s = run_host_leg().expect("host 腿硬失败");
        for (name, ok) in &s.asserts {
            assert!(ok, "host 断言 {name} 红");
        }
    }
}
