#![forbid(unsafe_code)]
//! RXS-0281/0282 重排执行模型 + I11 拦截项(G4.3 PR-E,RD-035 执行面②③,RFC-0014 §4.B9/B10)。
//!
//! **定位**:sealed 图建依赖 DAG(RAW/WAW/WAR 边)→ 拓扑分层;同层独立 pass 可换序、可批级
//! 提交(单 queue 一次提交多 pass,层间屏障)。**纯 host safe 码**(`#![forbid(unsafe_code)]`
//! 编译期封口),零后端调用、零 GPU 依赖,可 golden 锚。
//!
//! **I11 拦截项**(RXS-0282):调度器 [`derive_exec_plan`] 与核验器 [`verify_exec_plan`] 为
//! **两独立纯函数**(同模块但独立函数,禁共享推导辅助函数的内部状态,只读 sealed 图 +
//! ExecPlan 入参;D6 互证先例)。核验器**自 sealed 图独立重建**依赖闭包,逐边核 ExecPlan
//! 是否保持(丢边即 Err)。[`red_self_test_scheduler_drops_edge`] /
//! [`red_self_test_verifier_dropped`] 双向断言两独立纯函数互证。
//!
//! **零新 RX 码、零新 lang item、零新借用码**;纯库层状态值,不占编译器段位。

use crate::rhi::{AccessKind, PassSpec, Result, RhiError, RhiGraph};

// ── 调度计划 ────────────────────────────────────────────────────────────────────────

/// 同层独立 pass 集合(可换序 / 批级提交;层间屏障裁定全序 happens-before,RXS-0239 既有承诺)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// 同层 pass 索引集合(声明序下标;任意排列均保持依赖闭包)。
    pub pass_indices: Vec<usize>,
}

/// 调度计划(纯 host 产物;sealed 图 → 拓扑分层)。同图 → 逐字节相同计划(golden 可锚)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecPlan {
    /// 拓扑分层(层 0 = 无前驱 pass;层间序 = 全序,层内序可换)。
    pub layers: Vec<Layer>,
    /// 批级提交标志(单 queue 一次提交同层多 pass;G4.3 兑现面,多 queue out-of-scope)。
    pub batch_submit: bool,
}

impl ExecPlan {
    /// 计划内 pass 总数(须 = 图 pass 数;核验器逐边核前置)。
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.layers.iter().map(|l| l.pass_indices.len()).sum()
    }

    /// pass 在计划中的层号(0-based;未出现 → `None`)。
    #[must_use]
    pub fn layer_of(&self, pass_idx: usize) -> Option<usize> {
        for (li, layer) in self.layers.iter().enumerate() {
            if layer.pass_indices.contains(&pass_idx) {
                return Some(li);
            }
        }
        None
    }

    /// 计划的执行序(逐层、层内声明序)。seal → 调度 → 着色 → 回放四序闭合的「回放序」基准。
    #[must_use]
    pub fn execution_order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.pass_count());
        for layer in &self.layers {
            // 层内按声明序稳定(确定性;层内换序合法但默认取声明序)。
            let mut idxs = layer.pass_indices.clone();
            idxs.sort_unstable();
            order.extend(idxs);
        }
        order
    }
}

// ── 依赖边(RAW/WAW/WAR)──────────────────────────────────────────────────────────────

/// 依赖边(声明序下 pass i → pass j;i < j)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DepEdge {
    from: usize,
    to: usize,
}

/// **调度器侧**独立边集重建(自 sealed 图 compute pass 切片)。逐资源收集声明序访问,
/// 连续访问对按 RAW/WAW/WAR 建边(read-read 无依赖不建边)。与 [`verify_edges`] 代码独立
/// (D6 互证:两函数各自重建,禁共享辅助函数内部状态)。
fn scheduler_edges(passes: &[PassSpec]) -> Vec<DepEdge> {
    // 逐资源收集 (pass_idx, kind) 声明序对。
    let n_res = passes
        .iter()
        .flat_map(|p| p.accesses.iter().map(|a| a.resource.0))
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);
    let mut timeline: Vec<Vec<(usize, AccessKind)>> = vec![Vec::new(); n_res];
    for (pidx, pass) in passes.iter().enumerate() {
        for a in &pass.accesses {
            let r = a.resource.0 as usize;
            if r < n_res {
                timeline[r].push((pidx, a.kind));
            }
        }
    }
    let mut edges = Vec::new();
    for accesses in &timeline {
        for win in accesses.windows(2) {
            let (prev_pass, prev_kind) = win[0];
            let (cur_pass, cur_kind) = win[1];
            if prev_pass == cur_pass {
                continue; // 同 pass 内访问不构成跨 pass 依赖边。
            }
            let is_hazard = !matches!((prev_kind, cur_kind), (AccessKind::Read, AccessKind::Read));
            if is_hazard {
                edges.push(DepEdge {
                    from: prev_pass,
                    to: cur_pass,
                });
            }
        }
    }
    edges
}

/// **核验器侧**独立边集重建(自 sealed 图 compute pass 切片;与 [`scheduler_edges`] 代码独立,
/// D6 互证)。逻辑等价但代码独立副本——禁共享推导辅助函数内部状态。
fn verify_edges(passes: &[PassSpec]) -> Vec<DepEdge> {
    // 独立重建:逐资源访问时间线,连续访问对建 RAW/WAW/WAR 边。
    let max_res = passes
        .iter()
        .flat_map(|p| p.accesses.iter().map(|a| a.resource.0))
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);
    let mut per_resource: Vec<Vec<(usize, AccessKind)>> = vec![Vec::new(); max_res];
    for (i, pass) in passes.iter().enumerate() {
        for acc in &pass.accesses {
            let rid = acc.resource.0 as usize;
            if rid < max_res {
                per_resource[rid].push((i, acc.kind));
            }
        }
    }
    let mut out = Vec::new();
    for seq in &per_resource {
        for pair in seq.windows(2) {
            let (a_pass, a_kind) = pair[0];
            let (b_pass, b_kind) = pair[1];
            if a_pass == b_pass {
                continue;
            }
            // read-read 无依赖;其余 RAW/WAW/WAR 建边。
            let hazard = !(a_kind == AccessKind::Read && b_kind == AccessKind::Read);
            if hazard {
                out.push(DepEdge {
                    from: a_pass,
                    to: b_pass,
                });
            }
        }
    }
    out
}

// ── 调度器:derive_exec_plan(纯函数)──────────────────────────────────────────────────

/// sealed 图 → 拓扑分层调度计划(RXS-0281,纯函数)。建依赖 DAG(RAW/WAW/WAR 边)→
/// 最长路径分层(层 0 = 无前驱;层 k = 1 + max(前驱层))。同层 pass 互独立(无跨 pass 资源
/// 依赖),可换序 / 批级提交。同图 → 逐字节相同计划(golden 可锚;确定性,镜像 derive_syncs)。
///
/// **单 queue 批级提交**(G4.3 兑现面,多 queue out-of-scope):`batch_submit = true`。
/// 声明序为合法拓扑序(所有边 i→j 满足 i<j),故前向扫描一次即可定层。
///
/// # Panics
/// 仅在 `u32`/`usize` 转换溢出时 panic(图 pass 数远超该界不可能)。
#[must_use]
pub fn derive_exec_plan(graph: &RhiGraph) -> ExecPlan {
    let passes = graph.passes();
    let n = passes.len();
    let edges = scheduler_edges(passes);

    // 最长路径分层:layer[i] = 0 若无前驱,否则 1 + max(layer[前驱])。
    // 声明序为合法拓扑序(边 i→j 满足 i<j),故按 pass 索引升序处理即可。
    let mut layer = vec![0usize; n];
    for edge in &edges {
        // edge.to 的层 ≥ edge.from 的层 + 1。
        let candidate = layer[edge.from].saturating_add(1);
        if candidate > layer[edge.to] {
            layer[edge.to] = candidate;
        }
    }

    // 按层聚集(层内按声明序稳定)。
    let max_layer = layer.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Layer> = (0..=max_layer)
        .map(|_| Layer {
            pass_indices: Vec::new(),
        })
        .collect();
    for (pidx, &l) in layer.iter().enumerate() {
        layers[l].pass_indices.push(pidx);
    }
    // 移除空层(理论上无,防御)。
    layers.retain(|l| !l.pass_indices.is_empty());

    ExecPlan {
        layers,
        batch_submit: true,
    }
}

// ── 核验器:verify_exec_plan(独立纯函数,I11 拦截项)──────────────────────────────────

/// I11 核验器(独立纯函数,RXS-0282)。**自 sealed 图独立重建依赖闭包**(不调
/// [`derive_exec_plan`] / [`scheduler_edges`] 的内部函数),逐边核 ExecPlan 是否保持:
/// 每条 RAW/WAW/WAR 边 i→j 须满足 `layer[i] < layer[j]`(层间全序裁定);丢边即 Err。
///
/// 核验在 `execute()` 派发前**严格先于**调用(pre-dispatch fail-closed):失败则一个 kernel
/// 也不派发(镜像 seal 严格先于派发的纪律)。
///
/// # Errors
/// 丢边 / pass 数不符 / pass 越界 → [`RhiError::Structure`](库层状态值,零新码)。
pub fn verify_exec_plan(graph: &RhiGraph, plan: &ExecPlan) -> Result<()> {
    let passes = graph.passes();
    let n = passes.len();

    // 前置:计划 pass 总数须 = 图 pass 数(防丢/多 pass)。
    if plan.pass_count() != n {
        return Err(RhiError::Structure {
            detail: format!(
                "ExecPlan pass 数 {} ≠ 图 pass 数 {n}(I11 核验:pass 集失配)",
                plan.pass_count()
            ),
        });
    }

    // pass 索引须 ∈ [0, n)(防越界)。
    for layer in &plan.layers {
        for &p in &layer.pass_indices {
            if p >= n {
                return Err(RhiError::Structure {
                    detail: format!("ExecPlan 含越界 pass 索引 {p}(I11 核验:越界)"),
                });
            }
        }
    }

    // 独立重建依赖闭包(D6:不调 scheduler_edges,自 verify_edges 独立重建)。
    let edges = verify_edges(passes);

    // 逐边核:每条边 i→j 须 layer[i] < layer[j]。
    for edge in &edges {
        let li = plan.layer_of(edge.from);
        let lj = plan.layer_of(edge.to);
        match (li, lj) {
            (Some(li), Some(lj)) => {
                if li >= lj {
                    return Err(RhiError::Structure {
                        detail: format!(
                            "I11 拦截:依赖边 pass{}→pass{}(RAW/WAW/WAR)在 ExecPlan 中层序 {li}≥{lj} \
                             未保持(丢边,层间全序须裁定)",
                            edge.from, edge.to
                        ),
                    });
                }
            }
            _ => {
                return Err(RhiError::Structure {
                    detail: format!(
                        "I11 拦截:依赖边 pass{}→pass{} 在 ExecPlan 中缺失 pass(I11 核验:pass 丢失)",
                        edge.from, edge.to
                    ),
                });
            }
        }
    }

    Ok(())
}

// ── red_self_test 双向(I11 拦截项互证,纯 host 库单测)──────────────────────────────

/// 桩化核验器(不核边;恒返回 Ok)。用于 [`red_self_test_verifier_dropped`] 证明「若核验器
/// 丢失逐边核,丢边计划会被放行」——测试门检红(双向互证)。
fn verify_exec_plan_faulty(_graph: &RhiGraph, _plan: &ExecPlan) -> Result<()> {
    // 故意不核边:恒 Ok(模拟核验器丢失逐边核逻辑)。
    Ok(())
}

/// red_self_test 双向①:桩化调度器丢边被核验器检出(RXS-0282)。
/// 构造一个违反 RAW 边的计划(把有依赖的两 pass 放同层),真核验器须 Err;桩核验器放行 →
/// 测试门检红(证明核验器逐边核是必要拦截)。
///
/// 返回 `true` = 互证成立(真核验器拦 + 桩核验器放行)。
//@ spec: RXS-0282
#[must_use]
pub fn red_self_test_scheduler_drops_edge() -> bool {
    // 构造 sealed 图:p0 写 a,p1 读 a(RAW 边 p0→p1)。
    let mut g = RhiGraph::new();
    let a = g.resource("a");
    let _ = g.add_pass(PassSpec::new("p0").writes(a));
    let _ = g.add_pass(PassSpec::new("p1").reads(a));
    // 安全:demo 构图已知合法;seal 不会失败。
    let _ = g.seal();

    // 桩化「丢边」计划:把 p0、p1 放同层(违反 RAW 边 p0→p1)。
    let faulty_plan = ExecPlan {
        layers: vec![Layer {
            pass_indices: vec![0, 1],
        }],
        batch_submit: true,
    };

    // 真核验器须检出(Err);桩核验器放行(Ok)。
    let real_catches = verify_exec_plan(&g, &faulty_plan).is_err();
    let faulty_passes = verify_exec_plan_faulty(&g, &faulty_plan).is_ok();
    real_catches && faulty_passes
}

/// red_self_test 双向②:桩化核验器被门检出(RXS-0282)。
/// 注入丢边计划,真核验器拦;桩核验器(不核边)放行 → 证明「核验器侧逐边核」是必要门,
/// 若核验器被桩化则丢边计划会被放行(测试门检红)。
///
/// 返回 `true` = 互证成立(真核验器拦丢边 + 桩核验器漏拦)。
//@ spec: RXS-0282
#[must_use]
pub fn red_self_test_verifier_dropped() -> bool {
    // 构造 sealed 图:p0 写 a,p1 写 a,p2 读 a(WAW p0→p1, RAW p1→p2)。
    let mut g = RhiGraph::new();
    let a = g.resource("a");
    let _ = g.add_pass(PassSpec::new("p0").writes(a));
    let _ = g.add_pass(PassSpec::new("p1").writes(a));
    let _ = g.add_pass(PassSpec::new("p2").reads(a));
    let _ = g.seal();

    // 注入丢边计划:p0→p1→p2 但 p1 被放到 p2 之后层(违反 WAW p0→p1 与 RAW p1→p2)。
    let dropped_plan = ExecPlan {
        layers: vec![
            Layer {
                pass_indices: vec![0],
            },
            Layer {
                pass_indices: vec![2],
            }, // p2 在 p1 之前 → 违反 RAW p1→p2
            Layer {
                pass_indices: vec![1],
            },
        ],
        batch_submit: true,
    };

    // 真核验器须检出(Err);桩核验器放行(Ok)→ 证明桩核验器漏拦。
    let real_catches = verify_exec_plan(&g, &dropped_plan).is_err();
    let faulty_misses = verify_exec_plan_faulty(&g, &dropped_plan).is_ok();
    real_catches && faulty_misses
}

// ── 单测(RXS-0281/0282 测试锚定)──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rhi::{PassSpec, RhiGraph};

    /// 线性 RAW 链 → 每层单 pass(无独立 pass 可批级;demo 图手算 golden)。
    /// demo.rx 三 pass:p0 写 a,p1 读 a 写 b,p2 读 a 读 b 写 c。
    /// 边:p0→p1(RAW a),p1→p2(RAW b)。层:[[0],[1],[2]]。
    //@ spec: RXS-0281
    #[test]
    fn linear_raw_chain_one_pass_per_layer() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        let c = g.resource("c");
        let _ = g.add_pass(PassSpec::new("p0").writes(a));
        let _ = g.add_pass(PassSpec::new("p1").reads(a).writes(b));
        let _ = g.add_pass(PassSpec::new("p2").reads(a).reads(b).writes(c));
        let _ = g.seal();

        let plan = derive_exec_plan(&g);
        // 手算 golden:三 pass RAW 链 → 三层,每层单 pass(无独立 pass)。
        assert_eq!(
            plan.layers,
            vec![
                Layer {
                    pass_indices: vec![0]
                },
                Layer {
                    pass_indices: vec![1]
                },
                Layer {
                    pass_indices: vec![2]
                },
            ],
            "线性 RAW 链 → 每层单 pass(golden)"
        );
        assert!(plan.batch_submit, "单 queue 批级提交标志");
        assert_eq!(plan.execution_order(), vec![0, 1, 2]);
        // 核验通过(依赖保持)。
        assert!(verify_exec_plan(&g, &plan).is_ok(), "依赖保持 → 核验通过");
    }

    /// 菱形依赖:独立 pass 同层(p0 写 a/p0 写 b → p1 读 a / p2 读 b → p3 读 a 写 c...)。
    /// 简化:p0 写 a 写 b,p1 读 a,p2 读 b → p1/p2 同层(互独立,无跨资源依赖)。
    //@ spec: RXS-0281
    #[test]
    fn diamond_independent_passes_share_layer() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        let _ = g.add_pass(PassSpec::new("p0").writes(a).writes(b));
        let _ = g.add_pass(PassSpec::new("p1").reads(a));
        let _ = g.add_pass(PassSpec::new("p2").reads(b));
        let _ = g.seal();

        let plan = derive_exec_plan(&g);
        // 边:p0→p1(RAW a),p0→p2(RAW b)。p1/p2 无边 → 同层。
        // 层 0 = [0],层 1 = [1, 2](p1/p2 独立同层)。
        assert_eq!(plan.layers.len(), 2, "菱形 → 两层");
        assert_eq!(plan.layers[0].pass_indices, vec![0]);
        assert_eq!(plan.layers[1].pass_indices, vec![1, 2], "p1/p2 独立同层");
        assert!(verify_exec_plan(&g, &plan).is_ok());
    }

    /// WAW + WAR 边混合建序。
    //@ spec: RXS-0281
    #[test]
    fn waw_war_edges_layering() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let _ = g.add_pass(PassSpec::new("p0").writes(a)); // W
        let _ = g.add_pass(PassSpec::new("p1").reads(a)); // R → WAR with p2
        let _ = g.add_pass(PassSpec::new("p2").writes(a)); // W → WAW with p0, WAR with p1
        let _ = g.seal();
        let plan = derive_exec_plan(&g);
        // 边:p0→p1(RAW),p1→p2(WAR),p0→p2(WAW)。
        // 层:p0=0,p1=1(前驱 p0 层 0),p2=2(前驱 p1 层 1)。
        assert_eq!(
            plan.layers,
            vec![
                Layer {
                    pass_indices: vec![0]
                },
                Layer {
                    pass_indices: vec![1]
                },
                Layer {
                    pass_indices: vec![2]
                },
            ]
        );
        assert!(verify_exec_plan(&g, &plan).is_ok());
    }

    /// 确定性:同图 → 逐字节相同计划(golden 可锚)。
    //@ spec: RXS-0281
    #[test]
    fn deterministic_same_plan() {
        let mk = || {
            let mut g = RhiGraph::new();
            let a = g.resource("a");
            let b = g.resource("b");
            let _ = g.add_pass(PassSpec::new("p0").writes(a));
            let _ = g.add_pass(PassSpec::new("p1").reads(a).writes(b));
            let _ = g.add_pass(PassSpec::new("p2").reads(b));
            let _ = g.seal();
            derive_exec_plan(&g)
        };
        assert_eq!(mk(), mk(), "同图 → 逐字节相同计划");
    }

    // ── I11 核验器 + red_self_test 双向(RXS-0282)────────────────────────────────── //

    /// 核验器拒丢边:把 RAW 依赖的两 pass 放同层 → Err。
    //@ spec: RXS-0282
    #[test]
    fn verifier_rejects_dropped_edge() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let _ = g.add_pass(PassSpec::new("p0").writes(a));
        let _ = g.add_pass(PassSpec::new("p1").reads(a));
        let _ = g.seal();
        // 丢边计划:p0、p1 同层(违反 RAW p0→p1)。
        let bad = ExecPlan {
            layers: vec![Layer {
                pass_indices: vec![0, 1],
            }],
            batch_submit: true,
        };
        assert!(
            matches!(verify_exec_plan(&g, &bad), Err(RhiError::Structure { .. })),
            "丢边 → Structure Err(I11)"
        );
    }

    /// 核验器拒 pass 数失配。
    //@ spec: RXS-0282
    #[test]
    fn verifier_rejects_pass_count_mismatch() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let _ = g.add_pass(PassSpec::new("p0").writes(a));
        let _ = g.seal();
        // 计划多一个 pass。
        let bad = ExecPlan {
            layers: vec![Layer {
                pass_indices: vec![0, 1],
            }],
            batch_submit: true,
        };
        assert!(verify_exec_plan(&g, &bad).is_err(), "pass 数失配 → Err");
    }

    /// red_self_test 双向①:桩化调度器丢边被核验器检出。
    //@ spec: RXS-0282
    #[test]
    fn red_self_test_scheduler_drops_edge_passes() {
        assert!(
            red_self_test_scheduler_drops_edge(),
            "red_self_test 双向①:真核验器拦丢边 + 桩核验器放行"
        );
    }

    /// red_self_test 双向②:桩化核验器被门检出(漏拦丢边计划)。
    //@ spec: RXS-0282
    #[test]
    fn red_self_test_verifier_dropped_passes() {
        assert!(
            red_self_test_verifier_dropped(),
            "red_self_test 双向②:真核验器拦丢边 + 桩核验器漏拦"
        );
    }

    /// derive_exec_plan 产计划自洽(核验器对调度器产物恒 Ok;自证无丢边)。
    //@ spec: RXS-0282
    #[test]
    fn scheduler_output_self_consistent() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        let c = g.resource("c");
        let _ = g.add_pass(PassSpec::new("p0").writes(a));
        let _ = g.add_pass(PassSpec::new("p1").writes(b));
        let _ = g.add_pass(PassSpec::new("p2").reads(a).reads(b).writes(c));
        let _ = g.seal();
        let plan = derive_exec_plan(&g);
        // 调度器产物须自洽(核验器恒 Ok)。
        assert!(verify_exec_plan(&g, &plan).is_ok(), "调度器产物自洽");
        // p0/p1 互独立(写不同资源)→ 同层;p2 依赖两者 → 后层。
        assert_eq!(plan.layers.len(), 2);
        assert_eq!(plan.layers[0].pass_indices, vec![0, 1], "p0/p1 独立同层");
        assert_eq!(plan.layers[1].pass_indices, vec![2]);
    }
}
