//! OIT 档位纪律面（G9.5 M120；RFC-0025 §4.K；spec/display_pipeline.md
//! RXS-0371 L2/L3/L4）。
//!
//! - **仅测量不定档**（D4 D15）：本门只产 benchmark 数据；[`select_default_tier`]
//!   一律 fail-closed typed `Err(NotMeasuredYet)`——默认档选型由后续门的
//!   benchmark 数据裁决,不由论文偏好裁决。
//! - **无数据选型提交判 RED**：[`validate_selection_commit`]——无 benchmark
//!   数据引用的默认档选型提交 → typed `Err(SelectionWithoutBenchmarkData)`；
//!   引用数据缺档/缺算法 → `Err(BenchmarkDataIncomplete)`（RED 臂独立有效）。
//! - **排序 fallback 永保留**：[`OitTier::SortedFallback`] 恒可选
//!   （[`sorted_fallback_reachable`]),最低端档与正确性对照。
//! - **精确档内存有界契约**：linked-list 精确档池界必须显式声明
//!   （[`ExactTierMemoryPolicy::Bounded`]);请求无界增长 →
//!   `Err(ExactTierUnboundedMemory)`（RED 锚）;运行记录申报超界 →
//!   [`check_exact_tier_memory`] fail-closed。
//! - **精确档作用域**：linked-list 精确档仅毛发 strand 启用、场景级不开放
//!   （[`OitTier::ExactLinkedList`] 的 `scope` 恒 `HairStrandOnly`;场景级请求 →
//!   typed Err）。

use super::algorithms::OitAlgorithm;

/// OIT 档位闭集(RFC-0025 §4.K 三档 + 排序 fallback)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OitTier {
    /// ①默认档:TAA 半透明合成路径(现状延伸)。
    DefaultTaaComposite,
    /// ②有界近似档:WBOIT 起步(AVBOIT 评估项不承诺,D4 D16)。
    BoundedApprox,
    /// ③精确档:linked-list per-pixel fragment list(仅毛发 strand,场景级不开放)。
    ExactLinkedList,
    /// 排序 fallback(depth-sorted alpha):永保留最低端档与正确性对照。
    SortedFallback,
}

impl OitTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            OitTier::DefaultTaaComposite => "default_taa_composite",
            OitTier::BoundedApprox => "bounded_approx",
            OitTier::ExactLinkedList => "exact_linked_list",
            OitTier::SortedFallback => "sorted_fallback",
        }
    }
}

/// 精确档作用域闭集。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactTierScope {
    /// 仅毛发 strand(RXS-0371 L4)。
    HairStrandOnly,
}

/// OIT 失败类别(typed Err,fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum OitError {
    /// 本门仅测量:选型逻辑一律 fail-closed(未裁决)。
    NotMeasuredYet,
    /// 无 benchmark 数据的默认档选型提交(RXS-0371 L3 RED 锚)。
    SelectionWithoutBenchmarkData,
    /// 选型引用的 benchmark 数据不完整(缺算法/缺档位)。
    BenchmarkDataIncomplete { detail: String },
    /// 精确档内存无界增长请求(RXS-0371 L4 RED 锚)。
    ExactTierUnboundedMemory,
    /// 精确档内存实测超声明界(无界增长检出)。
    ExactTierMemoryExceeded { declared_cap: u64, observed: u64 },
    /// 精确档场景级请求(仅毛发 strand 作用域外开放 = 拒绝)。
    ExactTierSceneScopeRejected,
}

impl std::fmt::Display for OitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OitError::NotMeasuredYet => write!(f, "本门仅测量不定档(NotMeasuredYet)"),
            OitError::SelectionWithoutBenchmarkData => {
                write!(f, "默认档选型提交无 benchmark 数据引用(RED)")
            }
            OitError::BenchmarkDataIncomplete { detail } => {
                write!(f, "benchmark 数据不完整: {detail}")
            }
            OitError::ExactTierUnboundedMemory => {
                write!(f, "精确档内存无界增长请求(RED)")
            }
            OitError::ExactTierMemoryExceeded { declared_cap, observed } => {
                write!(f, "精确档内存超界: 声明 {declared_cap} 观测 {observed}")
            }
            OitError::ExactTierSceneScopeRejected => {
                write!(f, "精确档仅毛发 strand 作用域,场景级不开放")
            }
        }
    }
}

impl std::error::Error for OitError {}

/// **默认档选型入口——本门 fail-closed**(仅测量不定档;返回 NotMeasuredYet,
/// 任何调用路径不得在此门产出选定档)。
pub fn select_default_tier() -> Result<OitTier, OitError> {
    Err(OitError::NotMeasuredYet)
}

/// benchmark 数据引用面(选型提交须携带;由 harness 产出 digest 锚定)。
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkDataRef {
    /// 测量文件 digest(measured 冻结带)。
    pub measurements_digest: [u8; 32],
    /// 覆盖算法集合(七算法全量要求)。
    pub algorithms: Vec<OitAlgorithm>,
    /// 覆盖档位集合。
    pub overdraw_levels: Vec<u32>,
    /// 测量记录数(非空面)。
    pub record_count: usize,
}

/// 默认档选型提交(外部消费面;harness 自身不产出)。
#[derive(Debug, Clone)]
pub struct SelectionCommit {
    pub tier: OitTier,
    pub algorithm: OitAlgorithm,
    /// benchmark 数据引用(None = 无数据提交 ⇒ RED)。
    pub benchmark: Option<BenchmarkDataRef>,
}

/// 选型提交核验(RXS-0371 L3):无数据 ⇒ RED;数据不全(缺算法/缺档/零记录)
/// ⇒ RED;齐备 ⇒ Ok(仅表示「引用合规」,不表示本门做了选型)。
pub fn validate_selection_commit(commit: &SelectionCommit) -> Result<(), OitError> {
    let data = commit
        .benchmark
        .as_ref()
        .ok_or(OitError::SelectionWithoutBenchmarkData)?;
    if data.record_count == 0 {
        return Err(OitError::BenchmarkDataIncomplete {
            detail: "零测量记录".into(),
        });
    }
    let missing: Vec<&str> = OitAlgorithm::ALL
        .iter()
        .filter(|a| !data.algorithms.contains(a))
        .map(|a| a.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(OitError::BenchmarkDataIncomplete {
            detail: format!("缺算法测量: {missing:?}"),
        });
    }
    if data.overdraw_levels.is_empty() {
        return Err(OitError::BenchmarkDataIncomplete {
            detail: "零 overdraw 档位".into(),
        });
    }
    Ok(())
}

/// 排序 fallback 可达断言(RXS-0371 L4:永保留最低端档与正确性对照)。
pub fn sorted_fallback_reachable() -> OitTier {
    OitTier::SortedFallback
}

/// 精确档内存策略(有界契约为唯一合法形)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactTierMemoryPolicy {
    /// 显式有界(节点池 cap,bytes)。
    Bounded { cap_bytes: u64 },
}

/// 精确档内存策略请求面:任何「无界增长」表达在本类型系统外——请求入口按
/// 调用侧意图参数化,`unbounded = true` ⇒ typed Err(RED 锚)。
pub fn request_exact_tier_memory(unbounded: bool, cap_bytes: u64) -> Result<ExactTierMemoryPolicy, OitError> {
    if unbounded {
        return Err(OitError::ExactTierUnboundedMemory);
    }
    Ok(ExactTierMemoryPolicy::Bounded { cap_bytes })
}

/// 精确档内存实测核验(无界增长注入即 RED:观测超声明界 ⇒ typed Err)。
pub fn check_exact_tier_memory(
    policy: &ExactTierMemoryPolicy,
    observed_bytes: u64,
) -> Result<(), OitError> {
    match policy {
        ExactTierMemoryPolicy::Bounded { cap_bytes } => {
            if observed_bytes > *cap_bytes {
                return Err(OitError::ExactTierMemoryExceeded {
                    declared_cap: *cap_bytes,
                    observed: observed_bytes,
                });
            }
            Ok(())
        }
    }
}

/// 精确档作用域核验(仅毛发 strand;场景级请求 ⇒ typed Err)。
pub fn exact_tier_scope(scope: ExactTierScope) -> Result<ExactTierScope, OitError> {
    match scope {
        ExactTierScope::HairStrandOnly => Ok(scope),
    }
}

/// 场景级精确档请求面(RXS-0371 L4:场景级不开放)。
pub fn request_scene_scope_exact_tier() -> Result<OitTier, OitError> {
    Err(OitError::ExactTierSceneScopeRejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_data_ref() -> BenchmarkDataRef {
        BenchmarkDataRef {
            measurements_digest: [0xAB; 32],
            algorithms: OitAlgorithm::ALL.to_vec(),
            overdraw_levels: vec![4, 16, 64, 256],
            record_count: 28,
        }
    }

    //@ spec: RXS-0371
    #[test]
    fn selection_fail_closed_not_measured_yet() {
        // 仅测量不定档:选型入口一律 typed Err。
        assert!(matches!(select_default_tier(), Err(OitError::NotMeasuredYet)));
    }

    //@ spec: RXS-0371
    #[test]
    fn selection_commit_without_data_rejected() {
        // 无 benchmark 数据的选型提交判 RED。
        let commit = SelectionCommit {
            tier: OitTier::DefaultTaaComposite,
            algorithm: OitAlgorithm::WeightedBlended,
            benchmark: None,
        };
        assert!(matches!(
            validate_selection_commit(&commit),
            Err(OitError::SelectionWithoutBenchmarkData)
        ));
        // 数据不全(缺算法)⇒ RED。
        let mut partial = full_data_ref();
        partial.algorithms.retain(|a| *a != OitAlgorithm::Loop64);
        let commit2 = SelectionCommit {
            tier: OitTier::DefaultTaaComposite,
            algorithm: OitAlgorithm::Simple,
            benchmark: Some(partial),
        };
        assert!(matches!(
            validate_selection_commit(&commit2),
            Err(OitError::BenchmarkDataIncomplete { .. })
        ));
        // 齐备 ⇒ 引用合规(非选型)。
        let commit3 = SelectionCommit {
            tier: OitTier::DefaultTaaComposite,
            algorithm: OitAlgorithm::LinkedList,
            benchmark: Some(full_data_ref()),
        };
        assert!(validate_selection_commit(&commit3).is_ok());
    }

    //@ spec: RXS-0371
    #[test]
    fn sorted_fallback_reachable_and_exact_scope_closed() {
        assert_eq!(sorted_fallback_reachable(), OitTier::SortedFallback);
        assert!(exact_tier_scope(ExactTierScope::HairStrandOnly).is_ok());
        assert!(matches!(
            request_scene_scope_exact_tier(),
            Err(OitError::ExactTierSceneScopeRejected)
        ));
    }

    //@ spec: RXS-0371
    #[test]
    fn exact_tier_unbounded_memory_rejected() {
        // 无界增长请求 ⇒ RED;超界观测 ⇒ RED;界内 ⇒ Ok。
        assert!(matches!(
            request_exact_tier_memory(true, 1 << 20),
            Err(OitError::ExactTierUnboundedMemory)
        ));
        let policy = request_exact_tier_memory(false, 1 << 20).unwrap();
        assert!(check_exact_tier_memory(&policy, (1 << 20) - 1).is_ok());
        assert!(matches!(
            check_exact_tier_memory(&policy, (1 << 20) + 1),
            Err(OitError::ExactTierMemoryExceeded { .. })
        ));
    }
}
