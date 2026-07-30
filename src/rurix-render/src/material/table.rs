//! 材质表(报告6 §5:`MaterialTable` GPU 定长数组 + 材质 ID 索引;RFC-0016
//! 章 G 前半)。
//!
//! 注册 [`MaterialParams`] → `material_id`(u32,注册序);**打包位型去重**——
//! 相同 pack 结果(量化后逐位相同)返回同一 id,使低于量化分辨率的参数抖动不
//! 产生新材质(GPU 侧 classify/resolve 以 id 为分类键,表保持最小)。
//! [`MaterialTable::closures`] 导出定长数组,即 GPU material buffer 上传源。

use std::collections::HashMap;

use crate::graph::types::MaterialClosure;

use super::closure::MaterialParams;

/// 去重键:pack 结果的全部参数位(material_id 置零后;含 reserved 段,冻结布局
/// 逐字段对齐,不向 [`crate::graph::types`] 反向添加 Hash 派生)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClosureKey {
    albedo_rgba8: u32,
    f0_rgba8: u32,
    rough_metal_ao_flags: u32,
    normal_oct16: u32,
    emissive_rgbe: u32,
    reserved: [u32; 2],
}

impl ClosureKey {
    fn of(c: &MaterialClosure) -> Self {
        Self {
            albedo_rgba8: c.albedo_rgba8,
            f0_rgba8: c.f0_rgba8,
            rough_metal_ao_flags: c.rough_metal_ao_flags,
            normal_oct16: c.normal_oct16,
            emissive_rgbe: c.emissive_rgbe,
            reserved: c.reserved,
        }
    }
}

/// 材质表(单一事实来源;id 即 [`MaterialClosure::material_id`] 与导出下标)。
#[derive(Debug, Default)]
pub struct MaterialTable {
    entries: Vec<MaterialClosure>,
    index: HashMap<ClosureKey, u32>,
}

impl MaterialTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册材质:命中去重(打包位型相同)返回既有 id;否则追加并返回新 id。
    ///
    /// id 稳定性:同一 [`MaterialTable`] 内,相同参数任意次注册同 id;导出顺序
    /// 即首次注册顺序,`closures()[id].material_id == id`。
    pub fn register(&mut self, params: &MaterialParams) -> u32 {
        let packed = params.pack();
        let key = ClosureKey::of(&packed);
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = u32::try_from(self.entries.len()).expect("材质表条目数超 u32");
        let mut entry = packed;
        entry.material_id = id;
        self.entries.push(entry);
        self.index.insert(key, id);
        id
    }

    /// 查询(不注册):参数已在表则返回 id。
    pub fn get(&self, params: &MaterialParams) -> Option<u32> {
        self.index.get(&ClosureKey::of(&params.pack())).copied()
    }

    /// 导出 GPU 定长数组(材质 buffer 上传源;下标 = material_id)。
    pub fn closures(&self) -> &[MaterialClosure] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::closure::{MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_DOUBLE_SIDED};

    fn params(albedo: [f32; 3], flags: u8) -> MaterialParams {
        MaterialParams {
            albedo,
            flags,
            ..Default::default()
        }
    }

    #[test]
    fn id_stability_and_dedup() {
        let mut t = MaterialTable::new();
        // roughness 取量化格中值 0.37(0.37·255 = 94.35,远离格界;0.5 恰在
        // 127.5 格界,任何抖动都会跨格,不能做抖动用例)。
        let a = MaterialParams {
            albedo: [0.8, 0.2, 0.05],
            roughness: 0.37,
            ..Default::default()
        };
        let b = params([0.1, 0.9, 0.3], MATERIAL_FLAG_ALPHA_BLEND);
        let id_a0 = t.register(&a);
        let id_b = t.register(&b);
        let id_a1 = t.register(&a); // 相同参数重复注册 → 同 id
        assert_eq!(id_a0, 0);
        assert_eq!(id_b, 1);
        assert_eq!(id_a1, id_a0);
        assert_eq!(t.len(), 2);
        // 低于量化分辨率的抖动(1e-4 ≪ 1/255,格中值两侧不跨格)打包同位型 → 同 id。
        let mut a_jitter = a;
        a_jitter.albedo[0] += 1e-4;
        a_jitter.roughness += 1e-4;
        assert_eq!(t.register(&a_jitter), id_a0);
        assert_eq!(t.len(), 2);
        // flags 参与打包 → 不同 flags 不同 id。
        let a_flagged = params(a.albedo, MATERIAL_FLAG_DOUBLE_SIDED);
        let id_c = t.register(&a_flagged);
        assert_eq!(id_c, 2);
        assert_ne!(id_c, id_a0);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn export_gpu_source() {
        let mut t = MaterialTable::new();
        let p0 = params([1.0, 0.0, 0.0], 0);
        let p1 = params([0.0, 1.0, 0.0], MATERIAL_FLAG_ALPHA_BLEND);
        let id0 = t.register(&p0);
        let id1 = t.register(&p1);
        let snapshot: Vec<MaterialClosure> = t.closures().to_vec();
        // 导出顺序 = 注册顺序;条目内 material_id 回填 = 下标。
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[id0 as usize].material_id, id0);
        assert_eq!(snapshot[id1 as usize].material_id, id1);
        // 内容与直接 pack 一致(仅 material_id 回填)。
        let mut expect0 = p0.pack();
        expect0.material_id = id0;
        assert_eq!(snapshot[0], expect0);
        // get 查询不注册、不增表。
        assert_eq!(t.get(&p1), Some(id1));
        assert_eq!(t.get(&params([0.3, 0.3, 0.9], 0)), None);
        assert_eq!(t.len(), 2);
        // 导出布局:32B 定长(GPU buffer 元素)。
        assert_eq!(core::mem::size_of_val(&snapshot[0]), 32);
    }
}
