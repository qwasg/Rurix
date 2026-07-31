//! 流送 ↔ body 桥(§4.B4;R-G6-4「先卸 body 再放页」类型/所有权纪律,R-3 评审修订)。
//!
//! 页驻留(`PageRequest` 满足)→ [`StreamingBridge::insert_page`] 批插;页卸载 →
//! [`StreamingBridge::remove_page`] 批移除并产出 [`RemovalReceipt`]——receipt 与页
//! id 绑定、移动语义单次消耗:字段私有、无 pub 构造器、不实现 `Clone`/`Copy`,
//! 唯一产出口 = `remove_page`,流送层放页路径按值消耗本类型(无 receipt 的放页
//! 路径编译期不可构造 + 放页侧运行时断言双保险)。形状数据所有权:静态 mesh
//! 形状引用流送页驻留几何,卸载前 body 必已移除。物理只订阅「页驻留/卸载」
//! 通知,不重新实现 `StreamingBudget` 计量(§4.B1-3 流送同构)。

use std::collections::HashMap;

use crate::error::PhysicsError;
use crate::id::BodyId;
use crate::types::BodyDesc;
use crate::world::PhysicsWorld;

/// 几何页键(§4.B4;对齐 G5 流送的资源/页二元组,字段声明序 = 规范序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageKey {
    /// 资源 id(流送层分配)。
    pub resource: u32,
    /// 资源内页 id。
    pub page: u32,
}

/// 移除完成凭据(§4.B4,与页 id 绑定;**先卸 body 再放页**)。字段私有、无 pub
/// 构造器、不实现 `Clone`/`Copy`——移动语义单次消耗,编译期不可伪造;唯一
/// 产出口 = [`StreamingBridge::remove_page`]。
#[derive(Debug)]
pub struct RemovalReceipt {
    page: PageKey,
    bodies: Vec<BodyId>,
}

impl RemovalReceipt {
    /// 凭据绑定的页。
    pub fn page(&self) -> PageKey {
        self.page
    }

    /// 本次移除的 body 集(插入序,与 `insert_page` 的 `descs` 序一一对应)。
    pub fn removed_bodies(&self) -> &[BodyId] {
        &self.bodies
    }
}

/// 页 ↔ body 双向映射桥(§4.B4):页驻留批插、页卸载批移除 + receipt 产出。
#[derive(Debug, Default)]
pub struct StreamingBridge {
    page_bodies: HashMap<PageKey, Vec<BodyId>>,
    body_page: HashMap<BodyId, PageKey>,
}

impl StreamingBridge {
    /// 构造空桥(等价 `Default::default()`)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 页驻留 → 批插(§4.B4):page 已 watched → 确定性 `Err(InvalidDesc)`
    /// (重复驻留须先卸载);否则 `world.add_bodies_batch(descs)`(all-or-nothing,
    /// 失败不登记映射)后登记双向映射。返回 id 序与 `descs` 一一对应。
    pub fn insert_page(
        &mut self,
        world: &mut PhysicsWorld,
        page: PageKey,
        descs: &[BodyDesc],
    ) -> Result<Vec<BodyId>, PhysicsError> {
        if self.page_bodies.contains_key(&page) {
            return Err(PhysicsError::InvalidDesc(format!(
                "页 {page:?} 已 watched(重复驻留须先卸载)"
            )));
        }
        let ids = world.add_bodies_batch(descs)?;
        self.page_bodies.insert(page, ids.clone());
        for &id in &ids {
            self.body_page.insert(id, page);
        }
        Ok(ids)
    }

    /// 页卸载 → 批移除(§4.B4 **先卸 body**):未知页 → 确定性
    /// `Err(InvalidDesc)`;否则先 `world.remove_bodies_batch`(all-or-nothing,
    /// 失败保留双向映射),成功后摘除映射并返回 [`RemovalReceipt`]
    /// (**再放页**:receipt 由调用方按值消耗,见类型文档)。
    pub fn remove_page(
        &mut self,
        world: &mut PhysicsWorld,
        page: PageKey,
    ) -> Result<RemovalReceipt, PhysicsError> {
        let Some(bodies) = self.page_bodies.get(&page).cloned() else {
            return Err(PhysicsError::InvalidDesc(format!("页 {page:?} 未 watched")));
        };
        world.remove_bodies_batch(&bodies)?;
        self.page_bodies.remove(&page);
        for id in &bodies {
            self.body_page.remove(id);
        }
        Ok(RemovalReceipt { page, bodies })
    }

    /// 页 → body 集反查(插入序;未 watched → `None`)。
    pub fn bodies_of(&self, page: PageKey) -> Option<&[BodyId]> {
        self.page_bodies.get(&page).map(Vec::as_slice)
    }

    /// body → 页反查(未 watched → `None`)。
    pub fn page_of(&self, body: BodyId) -> Option<PageKey> {
        self.body_page.get(&body).copied()
    }

    /// 当前 watched 页数。
    pub fn watched_count(&self) -> usize {
        self.page_bodies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_key_canonical_order() {
        // 规范序 = 字段声明序(resource 先,page 后)。
        let mut keys = [
            PageKey {
                resource: 2,
                page: 0,
            },
            PageKey {
                resource: 1,
                page: 3,
            },
            PageKey {
                resource: 1,
                page: 2,
            },
            PageKey {
                resource: 0,
                page: 9,
            },
        ];
        keys.sort();
        assert_eq!(
            keys,
            [
                PageKey {
                    resource: 0,
                    page: 9
                },
                PageKey {
                    resource: 1,
                    page: 2
                },
                PageKey {
                    resource: 1,
                    page: 3
                },
                PageKey {
                    resource: 2,
                    page: 0
                },
            ]
        );
    }

    #[test]
    fn receipt_accessors_roundtrip() {
        // receipt 仅模块内可构造(编译期不可伪造的测试侧镜像)。
        let page = PageKey {
            resource: 42,
            page: 7,
        };
        let bodies = vec![BodyId::new(3, 1), BodyId::new(5, 1)];
        let receipt = RemovalReceipt {
            page,
            bodies: bodies.clone(),
        };
        assert_eq!(receipt.page(), page);
        assert_eq!(receipt.removed_bodies(), bodies.as_slice());
        // 移动语义单次消耗(无 Clone/Copy):按值 drop。
        drop(receipt);
    }

    #[test]
    fn streaming_bridge_new_empty() {
        let bridge = StreamingBridge::new();
        let page = PageKey {
            resource: 1,
            page: 1,
        };
        assert_eq!(bridge.watched_count(), 0);
        assert!(bridge.bodies_of(page).is_none());
        assert!(bridge.page_of(BodyId::new(0, 1)).is_none());
        let defaulted = StreamingBridge::default();
        assert_eq!(defaulted.watched_count(), 0);
    }
}
