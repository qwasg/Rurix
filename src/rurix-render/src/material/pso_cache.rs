//! PSO precache 与运行时编译告警(报告6 §2.2;RFC-0016 章 G 前半)。
//!
//! 预测式预编译纪律(UE 5.2+ PSO precaching 口径):
//! - **加载期**:变体预测器枚举 材质×pass 组合([`predict_precache_list`]),
//!   [`PsoCache::precache`] 批量预编译,不计告警;
//! - **运行期**:[`PsoCache::get_or_compile`] 命中返回缓存;未命中现场编译并使
//!   `runtime_compile_warnings` +1——**运行时编译即告警,验收归零**(Stray Spark
//!   手册目标;G5 时域门 G-G5-7 同口径)。
//!
//! 变体键 = [`PsoDesc::stable_hash`](手写 FNV-1a 64;不引外部依赖,不向冻结的
//! `graph::types` 反向添加派生)。

use std::collections::HashMap;

use crate::graph::types::{MaterialClosure, TextureFormat};

use super::closure::{MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_DOUBLE_SIDED};

/// 混合模式(管线状态;材质 flags → 变体的映射目标之一)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
    Additive,
    PremultipliedAlpha,
}

/// 剔除模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CullMode {
    None,
    Back,
    Front,
}

/// PSO 描述(变体键的全部字段;`TextureFormat` 复用冻结契约
/// [`crate::graph::types::TextureFormat`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsoDesc {
    pub vs_entry: String,
    pub fs_entry: String,
    pub color_formats: Vec<TextureFormat>,
    pub depth_format: Option<TextureFormat>,
    pub blend: BlendMode,
    pub cull: CullMode,
}

// ---------------------------------------------------------------------------
// 稳定哈希:FNV-1a 64(offset basis / prime 为公开常数);只依赖字段值,与
// 构造路径、字段写入顺序无关。字符串以 NUL 分隔防 ("ab","c") ≡ ("a","bc") 歧义;
// 枚举经本模块显式 tag 映射(不依赖冻结枚举的判别值序,types.rs 字段序冻结
// 不破此处稳定性)。
// ---------------------------------------------------------------------------

const FNV1A64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV1A64_PRIME);
    }
    h
}

fn fnv1a_str(h: u64, s: &str) -> u64 {
    fnv1a(fnv1a(h, s.as_bytes()), &[0])
}

fn texture_format_tag(f: TextureFormat) -> u8 {
    match f {
        TextureFormat::Rgba8Unorm => 1,
        TextureFormat::Rgba16Float => 2,
        TextureFormat::R11G11B10Float => 3,
        TextureFormat::Rg16Float => 4,
        TextureFormat::R32Uint => 5,
        TextureFormat::Rg32Uint => 6,
        TextureFormat::R32Float => 7,
        TextureFormat::Depth32Float => 8,
    }
}

fn blend_tag(b: BlendMode) -> u8 {
    match b {
        BlendMode::Opaque => 1,
        BlendMode::AlphaBlend => 2,
        BlendMode::Additive => 3,
        BlendMode::PremultipliedAlpha => 4,
    }
}

fn cull_tag(c: CullMode) -> u8 {
    match c {
        CullMode::None => 1,
        CullMode::Back => 2,
        CullMode::Front => 3,
    }
}

impl PsoDesc {
    /// 稳定哈希(FNV-1a 64):同一逻辑描述任意构造路径同值;任一字段不同则
    /// 值不同(颜色附件表含长度与顺序——附件序是管线语义的一部分)。
    pub fn stable_hash(&self) -> u64 {
        let mut h = FNV1A64_OFFSET;
        h = fnv1a_str(h, &self.vs_entry);
        h = fnv1a_str(h, &self.fs_entry);
        h = fnv1a(h, &(self.color_formats.len() as u32).to_le_bytes());
        for &f in &self.color_formats {
            h = fnv1a(h, &[texture_format_tag(f)]);
        }
        match self.depth_format {
            None => h = fnv1a(h, &[0xFF]),
            Some(f) => h = fnv1a(h, &[0xFE, texture_format_tag(f)]),
        }
        h = fnv1a(h, &[blend_tag(self.blend)]);
        fnv1a(h, &[cull_tag(self.cull)])
    }
}

/// PSO 缓存(`P` = 后端编译产物句柄,host 侧抽象;device 侧异步化由调用方在
/// compile_fn 内包装)。
#[derive(Debug)]
pub struct PsoCache<P> {
    map: HashMap<u64, P>,
    runtime_compile_warnings: u64,
}

impl<P> Default for PsoCache<P> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            runtime_compile_warnings: 0,
        }
    }
}

impl<P> PsoCache<P> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 加载期批量预编译(报告6 §2.2:PostLoad 枚举预测变体集后台编译;此路径
    /// 不触发告警)。同哈希重复描述幂等跳过。
    pub fn precache<'a>(
        &mut self,
        descs: impl IntoIterator<Item = &'a PsoDesc>,
        mut compile_fn: impl FnMut(&PsoDesc) -> P,
    ) {
        for d in descs {
            if let std::collections::hash_map::Entry::Vacant(e) = self.map.entry(d.stable_hash()) {
                let compiled = compile_fn(d);
                e.insert(compiled);
            }
        }
    }

    /// 运行期获取:命中返回缓存;未命中调 `compile_fn` 现场编译并使
    /// `runtime_compile_warnings` +1(运行时编译即告警,验收归零)。
    pub fn get_or_compile(&mut self, desc: &PsoDesc, compile_fn: impl FnOnce(&PsoDesc) -> P) -> &P {
        match self.map.entry(desc.stable_hash()) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                self.runtime_compile_warnings += 1;
                e.insert(compile_fn(desc))
            }
        }
    }

    /// 运行时编译告警计数(G5 门验收:归零)。
    pub fn warnings(&self) -> u64 {
        self.runtime_compile_warnings
    }

    pub fn contains(&self, desc: &PsoDesc) -> bool {
        self.map.contains_key(&desc.stable_hash())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// 变体预测器(报告6 §2.2 + §5:PassSet × MaterialFlags → ShaderKey 映射;
// 单层闭合使材质不引入 shader 排列,只引入管线状态 blend/cull——预测集规模
// = |材质| × |pass|,Fortnite 30k/百万级同构)
// ---------------------------------------------------------------------------

/// pass 的 shader/状态模板(预测器输入;mesh pass 为有限集合——报告1 的
/// depth/shadow/velocity/base 等)。
#[derive(Debug, Clone)]
pub struct PassShaderTemplate {
    pub vs_entry: String,
    pub fs_entry: String,
    pub color_formats: Vec<TextureFormat>,
    pub depth_format: Option<TextureFormat>,
}

/// 从闭合包内 flags 字段解码特性位(pack 口径:`rough_metal_ao_flags` 高 8 位)。
fn closure_flags(c: &MaterialClosure) -> u8 {
    (c.rough_metal_ao_flags >> 24) as u8
}

/// 材质×pass 笛卡尔积生成 precache 清单:材质 flags 决定 blend/cull
/// (`MATERIAL_FLAG_ALPHA_BLEND` → AlphaBlend,`MATERIAL_FLAG_DOUBLE_SIDED` →
/// 不剔除),其余字段取自 pass 模板。重复变体(不同材质同 flags 同 pass)由
/// [`PsoCache::precache`] 幂等吸收。
pub fn predict_precache_list(
    closures: &[MaterialClosure],
    passes: &[PassShaderTemplate],
) -> Vec<PsoDesc> {
    let mut out = Vec::with_capacity(closures.len() * passes.len());
    for c in closures {
        let flags = closure_flags(c);
        let blend = if flags & MATERIAL_FLAG_ALPHA_BLEND != 0 {
            BlendMode::AlphaBlend
        } else {
            BlendMode::Opaque
        };
        let cull = if flags & MATERIAL_FLAG_DOUBLE_SIDED != 0 {
            CullMode::None
        } else {
            CullMode::Back
        };
        for p in passes {
            out.push(PsoDesc {
                vs_entry: p.vs_entry.clone(),
                fs_entry: p.fs_entry.clone(),
                color_formats: p.color_formats.clone(),
                depth_format: p.depth_format,
                blend,
                cull,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::closure::MaterialParams;
    use crate::material::table::MaterialTable;
    use std::cell::Cell;

    fn desc(vs: &str, fs: &str, blend: BlendMode) -> PsoDesc {
        PsoDesc {
            vs_entry: vs.to_string(),
            fs_entry: fs.to_string(),
            color_formats: vec![TextureFormat::Rgba8Unorm, TextureFormat::Rg32Uint],
            depth_format: Some(TextureFormat::Depth32Float),
            blend,
            cull: CullMode::Back,
        }
    }

    #[test]
    fn hash_stability() {
        let d1 = desc("mesh_vs", "mat_fs", BlendMode::Opaque);
        // 另一构造路径:逐字段赋值(字段写入顺序不同)→ 同值同哈希。
        let mut d2 = desc("mat_fs", "mesh_vs", BlendMode::Opaque);
        d2.vs_entry = String::from("mesh_vs");
        d2.fs_entry = String::from("mat_fs");
        d2.color_formats = d1.color_formats.to_vec();
        d2.depth_format = d1.depth_format;
        d2.blend = d1.blend;
        d2.cull = d1.cull;
        assert_eq!(d1, d2);
        assert_eq!(d1.stable_hash(), d2.stable_hash());
        // 重算确定性。
        assert_eq!(d1.stable_hash(), d1.stable_hash());
        // 任一字段不同 → 哈希不同。
        assert_ne!(
            d1.stable_hash(),
            desc("mesh_vs2", "mat_fs", BlendMode::Opaque).stable_hash()
        );
        assert_ne!(
            d1.stable_hash(),
            desc("mesh_vs", "mat_fs", BlendMode::AlphaBlend).stable_hash()
        );
        let mut d3 = d1.clone();
        d3.depth_format = None;
        assert_ne!(d1.stable_hash(), d3.stable_hash());
        let mut d4 = d1.clone();
        d4.color_formats.pop();
        assert_ne!(d1.stable_hash(), d4.stable_hash());
        let mut d5 = d1.clone();
        d5.cull = CullMode::None;
        assert_ne!(d1.stable_hash(), d5.stable_hash());
        // 字符串分隔防歧义:("ab","c") ≠ ("a","bc")。
        let da = desc("ab", "c", BlendMode::Opaque);
        let db = desc("a", "bc", BlendMode::Opaque);
        assert_ne!(da.stable_hash(), db.stable_hash());
    }

    #[test]
    fn precache_then_get_zero_warnings() {
        // 全清单 precache 后,运行期 get 全命中:compile_fn 不被调,告警 0。
        let descs = [
            desc("mesh_vs", "mat_a", BlendMode::Opaque),
            desc("mesh_vs", "mat_b", BlendMode::AlphaBlend),
        ];
        let mut cache: PsoCache<u64> = PsoCache::new();
        let mut next_handle = 1u64;
        cache.precache(descs.iter(), |_| {
            let h = next_handle;
            next_handle += 1;
            h
        });
        assert_eq!(cache.len(), 2);
        for (i, d) in descs.iter().enumerate() {
            let got = cache.get_or_compile(d, |_| panic!("precache 后不得现场编译"));
            assert_eq!(*got, (i + 1) as u64);
        }
        assert_eq!(cache.warnings(), 0);
    }

    #[test]
    fn runtime_compile_warns() {
        // 未 precache:首次 get 现场编译 + 告警 1;二次同描述命中,告警不涨。
        let d = desc("mesh_vs", "mat_fs", BlendMode::Opaque);
        let mut cache: PsoCache<u64> = PsoCache::new();
        let compile_count = Cell::new(0u32);
        let h = *cache.get_or_compile(&d, |_| {
            compile_count.set(compile_count.get() + 1);
            42
        });
        assert_eq!(h, 42);
        assert_eq!(compile_count.get(), 1);
        assert_eq!(cache.warnings(), 1);
        let h2 = *cache.get_or_compile(&d, |_| {
            compile_count.set(compile_count.get() + 1);
            99
        });
        assert_eq!(h2, 42);
        assert_eq!(compile_count.get(), 1);
        assert_eq!(cache.warnings(), 1);
        // 新描述 → 第二次告警。
        let d2 = desc("mesh_vs", "other_fs", BlendMode::Opaque);
        cache.get_or_compile(&d2, |_| 7);
        assert_eq!(cache.warnings(), 2);
        assert!(cache.contains(&d));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn predictor_enumerates_material_cross_pass() {
        // 2 材质(不透明单面 / alpha 混合双面)× 2 pass → 4 变体;状态映射正确。
        let mut table = MaterialTable::new();
        table.register(&MaterialParams {
            flags: 0,
            ..Default::default()
        });
        table.register(&MaterialParams {
            flags: MATERIAL_FLAG_ALPHA_BLEND | MATERIAL_FLAG_DOUBLE_SIDED,
            ..Default::default()
        });
        let passes = [
            PassShaderTemplate {
                vs_entry: "depth_vs".into(),
                fs_entry: "depth_fs".into(),
                color_formats: vec![],
                depth_format: Some(TextureFormat::Depth32Float),
            },
            PassShaderTemplate {
                vs_entry: "base_vs".into(),
                fs_entry: "base_fs".into(),
                color_formats: vec![TextureFormat::Rgba8Unorm, TextureFormat::Rg32Uint],
                depth_format: Some(TextureFormat::Depth32Float),
            },
        ];
        let list = predict_precache_list(table.closures(), &passes);
        assert_eq!(list.len(), 4);
        assert_eq!(list[0].blend, BlendMode::Opaque);
        assert_eq!(list[0].cull, CullMode::Back);
        assert_eq!(list[2].blend, BlendMode::AlphaBlend);
        assert_eq!(list[2].cull, CullMode::None);
        assert_eq!(list[1].vs_entry, "base_vs");
        assert_eq!(list[3].fs_entry, "base_fs");
        // 预测清单全量 precache → 运行期按清单取全命中零告警。
        let mut cache: PsoCache<u64> = PsoCache::new();
        cache.precache(list.iter(), |_| 1);
        assert_eq!(cache.len(), 4);
        for d in &list {
            cache.get_or_compile(d, |_| panic!("预测清单未覆盖"));
        }
        assert_eq!(cache.warnings(), 0);
    }
}
