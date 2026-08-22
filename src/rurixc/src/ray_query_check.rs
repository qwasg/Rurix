//! RayQuery 状态机诊断 — MIR 层 device 数据流检查 pass(spec 条款 RXS-0299,
//! spec/shader_stages.md;RFC-0018 §3.A5,G7.2 W3a)。
//!
//! 实现裁决:与 [`crate::shared_check`] 同类(device 扩展检查模块),但作用层为
//! **MIR**——遍历 [`crate::query::QueryCtx::device_mir_crate`] 收集的 **device
//! MIR body**(`kernel fn` 为根的 device 调用图),仅对含 `RayQuery` 局部的 body
//! 实施;以**管线顺序**满足 spec 时点([`crate::query::QueryCtx::check_ray_query`]
//! 在 `check_shared_barrier` 之后、device codegen 之前接入)。状态机本体
//! (`Initialized` → `Terminated`,RXS-0298 三态)的两个编译期结构约束:
//!
//! - **S2 terminate 后使用 / 二次 terminate**(前向 **may-terminated** 数据流,
//!   复用 [`crate::dataflow`] 骨架):`terminate` 置位、`ray_query_initialize` /
//!   `ray_query_initialize_first_hit`(RFC-0030 §4.6;两构造内建同属"初始化"
//!   集合,同一 `Rvalue::RayQueryInitialize` MIR 节点)重初始化清位;置位态下
//!   任何方法族调用(含二次 `terminate`)→ RX3018。
//!   may 汇合(按位或)= 分支任一路径可能 terminated 即保守拒(07 §4)。
//! - **S3 committed_* 支配域约束**(新建支配集计算 + 守卫边识别):committed
//!   查询族(`committed_t` 等五查询)须被 `proceed`/`has_committed` 真值分支
//!   或 `while proceed` 循环体**支配**;严格三形态纪律(守卫布尔必须经合成
//!   临时 `Use` 链在同块内唯一定义于 `proceed`/`has_committed`),其余形态
//!   (else 分支 / 循环外 / 无守卫 / 布尔中转)保守拒 → RX3018。
//!
//! 保守上界(07 §4):证不出合法形态即拒(误判方向恒为拒,strict-only,P-01);
//! S1 未初始化使用 by-construction 不可达(RXS-0297 所有权形态),S4 initialize
//! 重入同理,均不在本 pass 作用面。

use crate::dataflow::{Analysis, BitSet, Location, iterate_to_fixpoint, successors};
use crate::diag::ErrorCode;
use crate::hir::{DefId, RayQueryOp};
use crate::mir::{
    Body, LocalIdx, Operand, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::ty::Ty;

pub const E_RAY_QUERY_STATE: ErrorCode = ErrorCode(3018); // RX3018(RXS-0299)

/// 全 crate RayQuery 状态机诊断入口(provider:[`QueryCtx::check_ray_query`])。
pub fn check_crate(cx: &QueryCtx<'_>) {
    let Some(rq_def) = cx.resolutions().lang_items.ray_query else {
        return;
    };
    for body in &cx.device_mir_crate() {
        // 无 RayQuery 局部的 body 无状态机义务(跳过,省数据流造价)。
        if !has_ray_query_local(body, rq_def) {
            continue;
        }
        check_terminated_use(cx, body);
        check_committed_guards(cx, body);
    }
}

/// body 含 `RayQuery` 类型的局部(RayQuery lang item 直判)。
fn has_ray_query_local(body: &Body, rq_def: DefId) -> bool {
    body.locals
        .iter()
        .any(|l| matches!(&l.ty, Ty::Adt(d, _) if *d == rq_def))
}

/// RX3018 统一发射(detail 占位经 message key `shader.ray_query_state_invalid`)。
fn emit_state_diag(cx: &QueryCtx<'_>, detail: String, span: Span) {
    cx.diag()
        .struct_error(E_RAY_QUERY_STATE, "shader.ray_query_state_invalid")
        .arg("detail", detail)
        .span_label(span, "invalid RayQuery traversal state")
        .emit();
}

// ---------------------------------------------------------------------------
// S2:前向 may-terminated 数据流(RXS-0299 S2)
// ---------------------------------------------------------------------------

/// 前向 may 分析:状态位 = 各 local 的「可能已 terminated」事实(格底全 0,
/// 汇合默认按位或)。
struct MayTerminated;

impl Analysis for MayTerminated {
    fn bits(&self, body: &Body) -> usize {
        body.locals.len()
    }

    /// 入口边界:全清空(无 local 可能 terminated)。
    fn boundary(&self, _body: &Body, _state: &mut BitSet) {}

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, _loc: Location) {
        let StatementKind::Assign(place, rv) = &stmt.kind;
        match rv {
            Rvalue::RayQueryMethod {
                op: RayQueryOp::Terminate,
                rq_local,
            } => {
                state.insert(rq_local.0 as usize);
            }
            // 重初始化重置状态(仅整体写;投影写不重置,保守)。first-hit 变体
            // 同节点(`first_hit` 位不改三态协议,RFC-0030 §4.6)故自然同覆盖。
            Rvalue::RayQueryInitialize { .. } if place.proj.is_empty() => {
                state.remove(place.local.0 as usize);
            }
            _ => {}
        }
    }

    fn term_effect(&self, _state: &mut BitSet, _term: &Terminator, _loc: Location) {}
}

/// S2 重放裁决:每个 RayQuery 方法族调用点(转移**前**状态)若其 receiver
/// 局部可能已 terminated → RX3018。
fn check_terminated_use(cx: &QueryCtx<'_>, body: &Body) {
    let analysis = MayTerminated;
    let results = iterate_to_fixpoint(body, &analysis);
    for b in 0..body.blocks.len() {
        results.visit_block(body, &analysis, b, |state, loc: Location| {
            let bb = &body.blocks[loc.block];
            if loc.stmt >= bb.stmts.len() {
                return; // 终结子点无方法族调用
            }
            let stmt = &bb.stmts[loc.stmt];
            let StatementKind::Assign(_, Rvalue::RayQueryMethod { op, rq_local }) = &stmt.kind
            else {
                return;
            };
            if !state.contains(rq_local.0 as usize) {
                return;
            }
            let detail = match op {
                RayQueryOp::Terminate => {
                    "`terminate` called on an already-terminated `RayQuery` (double terminate, RXS-0299 S2)".to_owned()
                }
                _ => format!(
                    "`{}` called after `terminate` (use of terminated `RayQuery`, RXS-0299 S2)",
                    op.name()
                ),
            };
            emit_state_diag(cx, detail, stmt.span);
        });
    }
}

// ---------------------------------------------------------------------------
// S3:committed_* 守卫支配检查(RXS-0299 S3)
// ---------------------------------------------------------------------------

/// 守卫边:`guard` 块以 `SwitchBool` 终结,其判别经合成临时 `Use` 链在同块内
/// 唯一定义为 `proceed`/`has_committed`;`true_target` 为其真值分支目标。
struct GuardEdge {
    guard: usize,
    true_target: usize,
    rq_local: LocalIdx,
}

/// S3 重放裁决:每条 committed_* 调用须存在同 receiver 的守卫边 (G, T) 使
/// **T 支配调用块且 T 的唯一前驱是 G**(唯一前驱 + 支配 ⇒ 边支配,声音性充分);
/// 否则保守拒 → RX3018。
fn check_committed_guards(cx: &QueryCtx<'_>, body: &Body) {
    // 收集 committed_* 调用点(块序确定性遍历)。
    let mut calls: Vec<(usize, &Statement, RayQueryOp, LocalIdx)> = Vec::new();
    for (b, bb) in body.blocks.iter().enumerate() {
        for stmt in &bb.stmts {
            let StatementKind::Assign(_, Rvalue::RayQueryMethod { op, rq_local }) = &stmt.kind
            else {
                continue;
            };
            if op.is_committed_query() {
                calls.push((b, stmt, *op, *rq_local));
            }
        }
    }
    if calls.is_empty() {
        return;
    }
    let edges = guard_edges(body);
    let preds = predecessors(body);
    let dom = dominators(body, &preds);
    for (b, stmt, op, rq_local) in calls {
        let admitted = edges.iter().any(|e| {
            e.rq_local == rq_local
                && preds[e.true_target].as_slice() == [e.guard]
                && dom[b].contains(e.true_target)
        });
        if !admitted {
            emit_state_diag(
                cx,
                format!(
                    "`{}` requires domination by the true-branch of `proceed`/`has_committed` or a `while proceed` loop body (unguarded committed query, RXS-0299 S3)",
                    op.name()
                ),
                stmt.span,
            );
        }
    }
}

/// operand 的裸局部引用(无投影;Const 无)。
fn operand_local(op: &Operand) -> Option<LocalIdx> {
    match op {
        Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() => Some(p.local),
        _ => None,
    }
}

/// 守卫边识别:严格三形态纪律(布尔中转/跨块定义/用户命名局部一律保守拒)。
fn guard_edges(body: &Body) -> Vec<GuardEdge> {
    let mut edges = Vec::new();
    for (gi, bb) in body.blocks.iter().enumerate() {
        let TerminatorKind::SwitchBool { discr, then, .. } = &bb.terminator.kind else {
            continue;
        };
        let Some(mut cur) = operand_local(discr) else {
            continue;
        };
        // 沿合成临时 `Use` 拷贝链回溯(每环节必须在块 G 内唯一定义)。
        let rq_local = loop {
            // 合成临时纪律:用户命名局部(如 `let advanced = rq.proceed()` 的
            // 布尔中转)保守拒;编译器合成临时 `Local.name` 为 `None`。
            if body.locals[cur.0 as usize].name.is_some() {
                break None;
            }
            // 定义语句必须同在块 G 内且唯一(零定义 = 更早块定义,保守拒;
            // 多定义 = 歧义,保守拒)。
            let mut defs = bb.stmts.iter().filter(|s| {
                let StatementKind::Assign(p, _) = &s.kind;
                p.proj.is_empty() && p.local == cur
            });
            let Some(def) = defs.next() else {
                break None;
            };
            if defs.next().is_some() {
                break None;
            }
            let StatementKind::Assign(_, rv) = &def.kind;
            match rv {
                Rvalue::RayQueryMethod {
                    op: RayQueryOp::Proceed | RayQueryOp::HasCommitted,
                    rq_local,
                } => break Some(*rq_local),
                Rvalue::Use(op) => {
                    let Some(next) = operand_local(op) else {
                        break None;
                    };
                    cur = next;
                }
                _ => break None,
            }
        };
        if let Some(rq_local) = rq_local {
            edges.push(GuardEdge {
                guard: gi,
                true_target: then.0 as usize,
                rq_local,
            });
        }
    }
    edges
}

/// CFG 前驱表(终结子后继的逆向)。
fn predecessors(body: &Body) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); body.blocks.len()];
    for (i, bb) in body.blocks.iter().enumerate() {
        for s in successors(&bb.terminator.kind) {
            preds[s].push(i);
        }
    }
    preds
}

/// 经典迭代 must 支配集:`dom[entry] = {entry}`,其余初始全集,
/// `dom[b] = {b} ∪ ⋂_{p∈preds(b)} dom[p]` 收敛。不可达块冻结为 `{b}`(其
/// committed_* 调用证不出守卫支配即拒 —— 误判方向恒为拒),且不参与可达块
/// 的汇合(其贡献等价全集,剔除不改变收敛结果)。
fn dominators(body: &Body, preds: &[Vec<usize>]) -> Vec<BitSet> {
    let n = body.blocks.len();
    // 入口可达集(BFS)。
    let mut reachable = vec![false; n];
    let mut stack = Vec::new();
    if n > 0 {
        reachable[0] = true;
        stack.push(0);
    }
    while let Some(b) = stack.pop() {
        for s in successors(&body.blocks[b].terminator.kind) {
            if !reachable[s] {
                reachable[s] = true;
                stack.push(s);
            }
        }
    }
    let mut dom: Vec<BitSet> = (0..n)
        .map(|b| {
            if b == 0 || !reachable[b] {
                let mut s = BitSet::new(n);
                s.insert(b);
                s
            } else {
                BitSet::filled(n)
            }
        })
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for b in 1..n {
            if !reachable[b] {
                continue;
            }
            let mut new = BitSet::filled(n);
            for &p in &preds[b] {
                if reachable[p] {
                    new.intersect(&dom[p]);
                }
            }
            new.insert(b);
            if new != dom[b] {
                dom[b] = new;
                changed = true;
            }
        }
    }
    dom
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// driver.rs 同序检查链(shader-stages AST 层 → typeck → 着色 → views →
    /// shared+barrier → RayQuery 状态机;前置阶段有错即停防级联),返回排序后
    /// 诊断码序列。
    fn check(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_shader_stages();
        if !diag.has_errors() {
            cx.check_crate();
        }
        if !diag.has_errors() {
            cx.check_coloring();
        }
        if !diag.has_errors() {
            cx.check_views();
        }
        if !diag.has_errors() {
            cx.check_shared_barrier();
        }
        if !diag.has_errors() {
            cx.check_ray_query();
        }
        let mut codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        codes.sort_unstable();
        codes
    }

    /// kernel 壳 + initialize 五实参(tlas, origin, t_min, dir, t_max)形态;
    /// 向量实参用实现已接的结构性元组形态(RXS-0223 容忍口径:`vec3(...)`
    /// 构造调用在 resolve 值位置无兜底,`vec3<f32>` 非真实 typeck 类型)。
    const HEAD: &str = r#"kernel fn k(tlas: AccelStruct, t: ThreadCtx<1>) {
    let mut rq = ray_query_initialize(tlas, (0.0, 0.0, 0.0), 0.0, (0.0, 0.0, 1.0), 100.0);
"#;

    /// first-hit 变体壳(RFC-0030 §4.6):构造经 `ray_query_initialize_first_hit`,
    /// 三态协议(S2/S3)与基线内建完全同形——两构造同属"初始化"集合。
    const HEAD_FIRST_HIT: &str = r#"kernel fn k(tlas: AccelStruct, t: ThreadCtx<1>) {
    let mut rq = ray_query_initialize_first_hit(tlas, (0.0, 0.0, 0.0), 0.0, (0.0, 0.0, 1.0), 100.0);
"#;

    fn kernel_src(body: &str) -> String {
        let mut s = String::from(HEAD);
        s.push_str(body);
        s.push_str("}\n");
        s
    }

    fn kernel_src_first_hit(body: &str) -> String {
        let mut s = String::from(HEAD_FIRST_HIT);
        s.push_str(body);
        s.push_str("}\n");
        s
    }

    //@ spec: RXS-0299
    #[test]
    fn accept_full_traversal_flow_is_clean() {
        // 语料同形态:initialize → while proceed → if has_committed → committed
        // 五查询 → terminate 合法全流程。
        let src = kernel_src(
            r#"    while rq.proceed() {
        if rq.has_committed() {
            let t_hit = rq.committed_t();
            let bary = rq.committed_barycentric();
            let inst = rq.committed_instance_index();
            let prim = rq.committed_primitive_index();
            let geom = rq.committed_geometry_index();
        }
    }
    rq.terminate();
"#,
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn use_after_terminate_is_rx3018() {
        // S2 直线:terminate 后 proceed(置位态方法族调用)。
        let src = kernel_src("    rq.terminate();\n    rq.proceed();\n");
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn double_terminate_is_rx3018() {
        // S2:二次 terminate(置位态再 terminate)。
        let src = kernel_src("    rq.terminate();\n    rq.terminate();\n");
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn branch_may_terminate_then_use_is_rx3018() {
        // S2 分支 may-terminate:单分支 terminate,汇合后 proceed;may 汇合
        // (按位或)= 任一路径可能 terminated 即保守拒。
        let src = kernel_src(
            "    if t.thread_index() == 0 {\n        rq.terminate();\n    }\n    rq.proceed();\n",
        );
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn terminate_inside_loop_before_break_is_clean() {
        // S2 循环内 terminate 合法:terminate 后仅达循环出口(break),proceed
        // 回边不经置位路径,头部汇合状态清空。
        let src = kernel_src(
            "    while rq.proceed() {\n        if t.thread_index() == 0 {\n            rq.terminate();\n            break;\n        }\n    }\n",
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_in_while_proceed_body_is_clean() {
        // S3 形态 ③:while proceed 循环体支配 committed_* 查询。
        let src =
            kernel_src("    while rq.proceed() {\n        let t_hit = rq.committed_t();\n    }\n");
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_without_guard_is_rx3018() {
        // S3 无守卫拒:initialize 后直接 committed_* 查询。
        let src = kernel_src("    let t_hit = rq.committed_t();\n");
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_in_if_proceed_true_branch_is_clean() {
        // S3 形态 ①:if proceed true 分支支配 committed_* 查询。
        let src =
            kernel_src("    if rq.proceed() {\n        let t_hit = rq.committed_t();\n    }\n");
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_in_if_has_committed_true_branch_is_clean() {
        // S3 形态 ②:if has_committed true 分支支配 committed_* 查询。
        let src = kernel_src(
            "    while rq.proceed() {\n        if rq.has_committed() {\n            let t_hit = rq.committed_t();\n        }\n    }\n",
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_behind_bool_intermediate_is_rx3018() {
        // S3 布尔中转拒:`let advanced = rq.proceed()` 用户命名局部中转,守卫
        // 边识别保守拒(严格三形态纪律)。
        let src = kernel_src(
            "    let advanced = rq.proceed();\n    if advanced {\n        let t_hit = rq.committed_t();\n    }\n",
        );
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_in_else_branch_is_rx3018() {
        // S3 else 分支拒:committed_* 在 proceed 的 false 分支,非 true 分支支配。
        let src = kernel_src(
            "    if rq.proceed() {\n    } else {\n        let t_hit = rq.committed_t();\n    }\n",
        );
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn committed_query_after_while_loop_is_rx3018() {
        // S3 循环后无守卫拒:while proceed 循环体不支配循环出口后的 committed_*。
        let src =
            kernel_src("    while rq.proceed() {\n    }\n    let t_hit = rq.committed_t();\n");
        assert_eq!(check(&src), vec![3018]);
    }

    // ---- first-hit 变体(RFC-0030 §4.6):三态协议与基线内建同形 ----

    //@ spec: RXS-0299
    #[test]
    fn first_hit_full_traversal_flow_is_clean() {
        // first_hit 构造同入"初始化"集合(Initialized 起点):initialize_first_hit
        // → while proceed → if has_committed → committed_t → terminate 全流程 0 诊断。
        let src = kernel_src_first_hit(
            "    while rq.proceed() {\n        if rq.has_committed() {\n            let t_hit = rq.committed_t();\n        }\n    }\n    rq.terminate();\n",
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0299
    #[test]
    fn first_hit_use_after_terminate_is_rx3018() {
        // S2 对 first_hit 构造的遍历器同律:terminate 置位后 proceed → RX3018。
        let src = kernel_src_first_hit("    rq.terminate();\n    rq.proceed();\n");
        assert_eq!(check(&src), vec![3018]);
    }

    //@ spec: RXS-0299
    #[test]
    fn first_hit_committed_query_without_guard_is_rx3018() {
        // S3 对 first_hit 构造的遍历器同律:无守卫 committed_t → RX3018
        // (与 conformance/rayquery/reject/first_hit_committed_unguarded.rx 同形)。
        let src = kernel_src_first_hit("    let t_hit = rq.committed_t();\n");
        assert_eq!(check(&src), vec![3018]);
    }
}
