//! `descriptor_table` — M103 descriptor buffer 全局表分配/回收律(G9.2 硬门
//! `g9.p0.m103.descriptor_global_table`;RFC-0023 §4.3;spec/rendering_platform.md
//! RXS-0347)。
//!
//! **定位**:「资源 → 全局 descriptor 索引」映射的 host 侧**单一事实源**——纯 host
//! safe、零 unsafe、零后端调用(本 crate `graph.rs` 同纪律;`vk.rs` 的
//! VK_EXT_descriptor_buffer 物理写入面只**消费**本模块产物)。全局表 =
//! 单一 descriptor buffer 内按表容量编号的全局索引空间;shader 侧以全局索引
//! 寻址(`push ConstantIndex`),与 reflection v1 尾随可选字段记录面
//! (`global_descriptor_indices`,rurixc `reflection.rs`)双向精确相等。
//!
//! **索引分配律/回收(RXS-0347 §3 逐字)**:
//! - **确定性**:同输入同映射逐字节等值——分配序 = 资源注册声明序;回收空位
//!   按**升序**复用(BTreeSet 迭代序,与输入次序无关);`HashMap` 迭代序不进产物。
//! - **fail-closed**:索引越界(≥ capacity)/ 悬空索引(未分配或已回收读)/
//!   双重释放 → typed `Err`(确定性拒绝,不静默回退、不最近邻回退);
//!   回收重用前读取旧资源索引 = 悬空,拒绝(不产生悬空索引消费)。
//! - **泄漏计数器**:`live_count()` = 分配 − 回收;`assert_no_leak()` 非零即红
//!   (计数器断言,装配期确定性)。
//! - **索引空间预算**:`capacity` 由 capability profile 事实承载
//!   (`bindless.descriptor_buffer`,RXS-0349);超预算分配 = 装配期确定性拒绝。
//!
//! **0-byte 纪律**:本模块为加性新面;既有 set/binding 路径(v1/v2 descriptor
//! set)不经本模块,回归 digest 不变(M31/M85 digest 链 0-byte)。

use std::collections::{BTreeSet, HashMap};

/// 全局 descriptor 索引表(M103;容量 = capability profile 索引空间预算)。
///
/// 分配 = 声明序单调递增;回收 = 空位升序复用(确定性)。同一 `(capacity,
/// register/release 调用序)` 输入 → 索引映射逐字节等值。
#[derive(Debug)]
pub struct GlobalDescriptorTable {
    /// 索引空间上限(索引域 `0..capacity`;capability profile 事实,RXS-0347 §4)。
    capacity: u32,
    /// 下一个未用过的索引(无空位时的分配点)。
    next_fresh: u32,
    /// 已回收空位(升序复用;`BTreeSet` 保确定性)。
    free: BTreeSet<u32>,
    /// 资源名 → 全局索引(声明序注册;同名重注册 = fail-closed)。
    slots: HashMap<String, u32>,
    /// 在册索引集(悬空判定:不在册 = 未分配或已回收)。
    live: BTreeSet<u32>,
    /// 累计分配计数(分配律审计;回收不减)。
    alloc_total: u64,
    /// 累计回收计数。
    free_total: u64,
}

/// 全局表装配错误(fail-closed;typed `Err`,不占 RX 码——host 装配面沿
/// `binding_layout::BindingInferError` / graph.rs `GraphError` 先例)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// 索引空间预算超限(分配超出 capacity;capability profile 事实)。
    BudgetExceeded {
        /// 请求序次(第 N 次分配)与预算。
        detail: String,
    },
    /// 索引越界(≥ capacity)。
    IndexOutOfBounds {
        /// 诊断上下文。
        detail: String,
    },
    /// 悬空索引(读未分配/已回收索引,或读已回收资源)。
    DanglingIndex {
        /// 诊断上下文。
        detail: String,
    },
    /// 双重回收(索引不在册)。
    DoubleFree {
        /// 诊断上下文。
        detail: String,
    },
    /// 同名资源重复注册(映射歧义,fail-closed 不覆盖)。
    DuplicateResource {
        /// 诊断上下文。
        detail: String,
    },
    /// 索引泄漏(live ≠ 0;计数器断言)。
    Leak {
        /// 诊断上下文。
        detail: String,
    },
}

impl std::fmt::Display for TableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableError::BudgetExceeded { detail }
            | TableError::IndexOutOfBounds { detail }
            | TableError::DanglingIndex { detail }
            | TableError::DoubleFree { detail }
            | TableError::DuplicateResource { detail }
            | TableError::Leak { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for TableError {}

impl GlobalDescriptorTable {
    /// 新建容量 `capacity` 的全局表(capability profile 索引空间预算,RXS-0347 §4)。
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        GlobalDescriptorTable {
            capacity,
            next_fresh: 0,
            free: BTreeSet::new(),
            slots: HashMap::new(),
            live: BTreeSet::new(),
            alloc_total: 0,
            free_total: 0,
        }
    }

    /// 索引空间预算(profile 事实)。
    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// 在册条目数(泄漏计数器:分配 − 回收)。
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// 累计分配/回收计数(分配律审计)。
    #[must_use]
    pub fn counters(&self) -> (u64, u64) {
        (self.alloc_total, self.free_total)
    }

    /// 注册资源并分配全局索引(确定性:声明序;空位升序复用)。
    ///
    /// # Errors
    /// 超预算 → [`TableError::BudgetExceeded`];同名重注册 → [`TableError::DuplicateResource`]。
    pub fn register(&mut self, name: &str) -> Result<u32, TableError> {
        if self.slots.contains_key(name) {
            return Err(TableError::DuplicateResource {
                detail: format!("资源 `{name}` 重复注册(同名歧义,fail-closed 不覆盖)"),
            });
        }
        // 空位升序复用优先;无空位取 next_fresh(预算门)。
        let idx = if let Some(&i) = self.free.iter().next() {
            self.free.remove(&i);
            i
        } else {
            if self.next_fresh >= self.capacity {
                return Err(TableError::BudgetExceeded {
                    detail: format!(
                        "全局 descriptor 索引空间超限:第 {} 次分配超出 capacity {}(capability profile 预算,RXS-0347 §4)",
                        self.alloc_total + 1,
                        self.capacity
                    ),
                });
            }
            let i = self.next_fresh;
            self.next_fresh += 1;
            i
        };
        self.slots.insert(name.to_owned(), idx);
        self.live.insert(idx);
        self.alloc_total += 1;
        Ok(idx)
    }

    /// 回收资源索引(streaming 换出;回收后可复用)。回收后该索引/资源读 = 悬空。
    ///
    /// # Errors
    /// 未注册资源 / 双重回收 → [`TableError::DoubleFree`]。
    pub fn release(&mut self, name: &str) -> Result<u32, TableError> {
        let Some(idx) = self.slots.remove(name) else {
            return Err(TableError::DoubleFree {
                detail: format!("资源 `{name}` 未在册(双重回收/未注册,fail-closed)"),
            });
        };
        self.live.remove(&idx);
        self.free.insert(idx);
        self.free_total += 1;
        Ok(idx)
    }

    /// 读资源 → 全局索引(reflection 记录面 / shader ConstantIndex 同源)。
    ///
    /// # Errors
    /// 未注册或已回收(悬空)→ [`TableError::DanglingIndex`]。
    pub fn index_of(&self, name: &str) -> Result<u32, TableError> {
        match self.slots.get(name) {
            Some(&i) if self.live.contains(&i) => Ok(i),
            _ => Err(TableError::DanglingIndex {
                detail: format!(
                    "资源 `{name}` 的全局索引悬空(未注册或已回收;回收重用不产生悬空索引消费,RXS-0347 §3)"
                ),
            }),
        }
    }

    /// 全局索引消费核验(shader 实际消费索引对拍;双向精确相等的反射侧)。
    ///
    /// # Errors
    /// 越界(≥ capacity)→ [`TableError::IndexOutOfBounds`];未在册(悬空)→
    /// [`TableError::DanglingIndex`]。
    pub fn validate_index(&self, idx: u32) -> Result<(), TableError> {
        if idx >= self.capacity {
            return Err(TableError::IndexOutOfBounds {
                detail: format!(
                    "全局索引 {idx} 越界(≥ capacity {};fail-closed,RXS-0347 §3)",
                    self.capacity
                ),
            });
        }
        if !self.live.contains(&idx) {
            return Err(TableError::DanglingIndex {
                detail: format!("全局索引 {idx} 悬空(未分配或已回收;fail-closed)"),
            });
        }
        Ok(())
    }

    /// 泄漏计数器断言(装配期;非零即红)。
    ///
    /// # Errors
    /// `live_count() != 0` → [`TableError::Leak`]。
    pub fn assert_no_leak(&self) -> Result<(), TableError> {
        if self.live.is_empty() {
            Ok(())
        } else {
            Err(TableError::Leak {
                detail: format!(
                    "全局 descriptor 索引泄漏:live {}(分配 {} / 回收 {});泄漏计数器非零即红(RXS-0347 §3)",
                    self.live.len(),
                    self.alloc_total,
                    self.free_total
                ),
            })
        }
    }

    /// 当前映射的确定性快照(reflection 记录面;按索引升序,与输入序无关字节等值)。
    #[must_use]
    pub fn mapping_snapshot(&self) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> =
            self.slots.iter().map(|(k, &i)| (k.clone(), i)).collect();
        v.sort_by_key(|(_, i)| *i);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 分配确定性:同输入同映射逐字节等值(声明序 → 索引序)。
    //@ spec: RXS-0347
    #[test]
    fn allocation_is_deterministic() {
        let mut a = GlobalDescriptorTable::new(8);
        let mut b = GlobalDescriptorTable::new(8);
        for n in ["tex_a", "tex_b", "tex_c"] {
            assert_eq!(a.register(n).unwrap(), b.register(n).unwrap());
        }
        assert_eq!(a.mapping_snapshot(), b.mapping_snapshot());
        assert_eq!(
            a.mapping_snapshot(),
            vec![
                ("tex_a".to_owned(), 0),
                ("tex_b".to_owned(), 1),
                ("tex_c".to_owned(), 2)
            ]
        );
    }

    /// 回收重用:空位升序复用;回收后读旧索引 = 悬空(不产生悬空索引消费)。
    //@ spec: RXS-0347
    #[test]
    fn recycle_reuses_ascending_and_dangling_rejected() {
        let mut t = GlobalDescriptorTable::new(4);
        let _a = t.register("a").unwrap(); // 0
        let _b = t.register("b").unwrap(); // 1
        let _c = t.register("c").unwrap(); // 2
        t.release("b").unwrap(); // 1 回收
        t.release("a").unwrap(); // 0 回收
        // 空位 {0,1}:升序复用——先 0 后 1(确定性,与释放序无关)。
        assert_eq!(t.register("d").unwrap(), 0);
        assert_eq!(t.register("e").unwrap(), 1);
        // 旧名读 = 悬空(fail-closed)。
        assert!(matches!(
            t.index_of("a"),
            Err(TableError::DanglingIndex { .. })
        ));
        assert!(matches!(
            t.index_of("b"),
            Err(TableError::DanglingIndex { .. })
        ));
        // 新映射精确。
        assert_eq!(t.index_of("d").unwrap(), 0);
        assert_eq!(t.index_of("e").unwrap(), 1);
        assert_eq!(t.index_of("c").unwrap(), 2);
    }

    /// 越界/悬空/双释放 fail-closed;泄漏计数器断言。
    //@ spec: RXS-0347
    #[test]
    fn fail_closed_paths_and_leak_counter() {
        let mut t = GlobalDescriptorTable::new(2);
        t.register("x").unwrap();
        t.register("y").unwrap();
        // 预算超限。
        assert!(matches!(
            t.register("z"),
            Err(TableError::BudgetExceeded { .. })
        ));
        // 越界。
        assert!(matches!(
            t.validate_index(2),
            Err(TableError::IndexOutOfBounds { .. })
        ));
        // 悬空(未分配)。
        assert!(matches!(
            t.validate_index(0).map(|_| ()),
            Ok(()) // 0 在册
        ));
        t.release("x").unwrap();
        assert!(matches!(
            t.validate_index(0),
            Err(TableError::DanglingIndex { .. })
        ));
        // 双释放。
        assert!(matches!(
            t.release("x"),
            Err(TableError::DoubleFree { .. })
        ));
        // 泄漏计数器:y 未回收 → live=1 → 红。
        assert!(matches!(t.assert_no_leak(), Err(TableError::Leak { .. })));
        t.release("y").unwrap();
        assert!(t.assert_no_leak().is_ok());
        assert_eq!(t.counters(), (2, 2));
    }

    /// 同名重注册 fail-closed(映射歧义不覆盖)。
    //@ spec: RXS-0347
    #[test]
    fn duplicate_register_rejected() {
        let mut t = GlobalDescriptorTable::new(4);
        t.register("dup").unwrap();
        assert!(matches!(
            t.register("dup"),
            Err(TableError::DuplicateResource { .. })
        ));
    }
}
