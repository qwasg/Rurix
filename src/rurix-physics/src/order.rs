//! 查询结果规范序(纯函数,后端无关;§4.A4 C-2 评审修订):Jolt
//! `NarrowPhaseQuery` 结果一致但返回顺序可变——cast 结果返回前按
//! `(t, BodyId)` 规范序排序,排序后序列 = 确定性面;overlap 无扫掠参数,
//! 规范序 = `BodyId` 升序。

use crate::types::{OverlapHit, QueryHit};

/// cast 命中按 `(t, BodyId)` 规范序排序(`t` 位级 total order,NaN 也确定)。
pub(crate) fn sort_query_hits(hits: &mut [QueryHit]) {
    hits.sort_by(|a, b| a.t.total_cmp(&b.t).then_with(|| a.body.cmp(&b.body)));
}

/// overlap 命中按 `BodyId` 升序排序。
pub(crate) fn sort_overlap_hits(hits: &mut [OverlapHit]) {
    hits.sort_by(|a, b| a.body.cmp(&b.body));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{BodyId, ShapeId};

    fn hit(body_idx: u32, t: f32) -> QueryHit {
        QueryHit {
            body: BodyId::new(body_idx, 1),
            t,
            position: [0.0; 3],
            normal: [0.0; 3],
            shape: ShapeId::new(body_idx, 1),
        }
    }

    #[test]
    fn query_hits_sorted_by_t_then_body() {
        let mut hits = vec![hit(2, 1.0), hit(1, 1.0), hit(9, 0.5), hit(3, 2.0)];
        sort_query_hits(&mut hits);
        let keys: Vec<_> = hits
            .iter()
            .map(|h| (h.t.to_bits(), h.body.index()))
            .collect();
        assert_eq!(
            keys,
            vec![
                (0.5f32.to_bits(), 9),
                (1.0f32.to_bits(), 1),
                (1.0f32.to_bits(), 2), // 同 t → BodyId 决胜
                (2.0f32.to_bits(), 3),
            ]
        );
    }

    #[test]
    fn query_hits_nan_t_deterministic() {
        let mut hits = vec![hit(1, f32::NAN), hit(2, 1.0), hit(3, -1.0)];
        sort_query_hits(&mut hits);
        // total_cmp:NaN 恒排最后,与输入序无关。
        assert_eq!(hits[2].body.index(), 1);
        assert_eq!(hits[0].body.index(), 3);
    }

    #[test]
    fn overlap_hits_sorted_by_body() {
        // BodyId 序 = u64 位序(generation 高 32b 主导),确定性即可,语义无涉。
        let mut hits = vec![
            OverlapHit {
                body: BodyId::new(5, 1),
                shape: ShapeId::new(5, 1),
            },
            OverlapHit {
                body: BodyId::new(1, 2),
                shape: ShapeId::new(1, 2),
            },
        ];
        sort_overlap_hits(&mut hits);
        assert_eq!(hits[0].body, BodyId::new(5, 1));
        assert_eq!(hits[1].body, BodyId::new(1, 2));
    }
}
