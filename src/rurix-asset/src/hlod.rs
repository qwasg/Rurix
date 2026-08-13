//! HLOD 离线烘焙 Builder(G9.5 M110 波工具面;RXS-0364 语义锚最小实现)。
//!
//! //@ spec: RXS-0364
//!
//! 职责面(M110 实现任务范围;M111 完整门 = `g9.p1.m111.hlod_baking` 独立 assertion,
//! 本模块不冒充其全量判据):
//! - **离线按 Component 分发**:输入 cell 几何资产(逐 Component 三角面集合),
//!   逐 Component 生成代理几何(确定性抽取简化 + 逐层合批);
//! - **产物即资产**:产物经 canonical 二进制编码落成普通资产字节(走同一
//!   cook/DDC 通道的资产形态,不私定磁盘格式——本模块只产字节,digest 寻址);
//! - **双构建 hash 相等**:同输入两次独立烘焙产物 digest 逐位一致(沿 M79 判据
//!   形态);输入扰动(声明序/三角面顺序)不影响产物 hash(canonical 排序事实
//!   源);几何内容扰动必须分叉(RED 臂能红证明)。
//!
//! 纪律:纯 host 离线工具(GPU 非必需——烘焙 = 确定性几何抽取与合批,零 device
//! 依赖);零新 FFI;fail-closed typed Err。

use crate::canon;
use crate::error::{AssetError, ErrorKind, Result};

/// HLOD 层数上界(cell 级 HLOD 树,层数为烘焙属性,RFC-0025 §4.B)。
pub const MAX_HLOD_LEVELS: u32 = 8;
/// 产物资产 magic("RXHL")。
pub const HLOD_ASSET_MAGIC: [u8; 4] = *b"RXHL";
/// 产物资产格式版本。
pub const HLOD_ASSET_VERSION: u16 = 1;

/// 输入:cell 几何资产的单个 Component(命名 + 三角面集合,每三角 9×f32)。
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentGeometry {
    pub name: String,
    pub triangles: Vec<[f32; 9]>,
}

/// 输入:单个 cell 的 HLOD 烘焙请求(逐 Component 分发)。
#[derive(Debug, Clone, PartialEq)]
pub struct HlodBakeInput {
    pub cell_name: String,
    /// 层数为烘焙属性(1..=[`MAX_HLOD_LEVELS`];level 0 = 全量几何)。
    pub levels: u32,
    pub components: Vec<ComponentGeometry>,
}

/// 产物:逐 Component 代理几何(简化抽取后的三角面 + 源三角数留痕)。
#[derive(Debug, Clone, PartialEq)]
pub struct HlodComponentProxy {
    pub component: String,
    pub source_triangles: u32,
    pub proxy_triangles: Vec<[f32; 9]>,
}

/// 产物:单个 HLOD 层。
#[derive(Debug, Clone, PartialEq)]
pub struct HlodLevel {
    pub level: u32,
    pub proxies: Vec<HlodComponentProxy>,
}

/// 产物:HLOD 层级资产(产物即资产;canonical 编码 + digest 寻址)。
#[derive(Debug, Clone, PartialEq)]
pub struct HlodAsset {
    pub cell_name: String,
    pub levels: Vec<HlodLevel>,
}

fn check_name(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 256
        || !s.bytes().all(|b| (0x20..=0x7e).contains(&b))
    {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!("hlod 名称非法(可打印 ASCII,1..=256 字节): {s:?}"),
        ));
    }
    Ok(())
}

/// 三角面 canonical 排序键:量化质心(毫米栅格)→ 原始字节字典序(声明序扰动
/// 不影响产物 hash 的事实源)。
fn tri_sort_key(t: &[f32; 9]) -> (i64, i64, i64, [u8; 36]) {
    let cx = (t[0] as f64 + t[3] as f64 + t[6] as f64) / 3.0;
    let cy = (t[1] as f64 + t[4] as f64 + t[7] as f64) / 3.0;
    let cz = (t[2] as f64 + t[5] as f64 + t[8] as f64) / 3.0;
    let mut bytes = [0u8; 36];
    for (i, v) in t.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    (
        (cx * 1000.0).round() as i64,
        (cy * 1000.0).round() as i64,
        (cz * 1000.0).round() as i64,
        bytes,
    )
}

/// 输入合法性 fail-closed 核验。
pub fn validate_input(input: &HlodBakeInput) -> Result<()> {
    check_name(&input.cell_name)?;
    if input.levels == 0 || input.levels > MAX_HLOD_LEVELS {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!(
                "hlod 层数 {} 越界(1..={MAX_HLOD_LEVELS})",
                input.levels
            ),
        ));
    }
    if input.components.is_empty() {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            "hlod 输入零 Component",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    for c in &input.components {
        check_name(&c.name)?;
        if !names.insert(c.name.as_str()) {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                format!("hlod Component 名重复: {}", c.name),
            ));
        }
        if c.triangles.is_empty() {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                format!("hlod Component {} 零三角面", c.name),
            ));
        }
        for t in &c.triangles {
            if !t.iter().all(|v| v.is_finite()) {
                return Err(AssetError::new(
                    ErrorKind::Invalid,
                    format!("hlod Component {} 含非有限坐标", c.name),
                ));
            }
        }
    }
    Ok(())
}

/// 离线烘焙:逐 Component canonical 排序 → 逐层确定性抽取简化(stride = 2^level,
/// 至少保留 1 三角)→ 层级资产。纯函数,同输入同输出。
pub fn bake_hlod(input: &HlodBakeInput) -> Result<HlodAsset> {
    validate_input(input)?;
    // Component 分发序 canonical 化(按名排序,声明序扰动免疫)。
    let mut comps: Vec<&ComponentGeometry> = input.components.iter().collect();
    comps.sort_by(|a, b| a.name.cmp(&b.name));
    // 逐 Component 三角面 canonical 序(与声明序无关)。
    let mut sorted_tris: Vec<Vec<[f32; 9]>> = Vec::with_capacity(comps.len());
    for c in &comps {
        let mut tris = c.triangles.clone();
        tris.sort_by_cached_key(tri_sort_key);
        sorted_tris.push(tris);
    }
    let mut levels = Vec::with_capacity(input.levels as usize);
    for level in 0..input.levels {
        let stride = 1usize << level;
        let mut proxies = Vec::with_capacity(comps.len());
        for (c, tris) in comps.iter().zip(&sorted_tris) {
            let proxy: Vec<[f32; 9]> = tris
                .iter()
                .step_by(stride)
                .copied()
                .collect::<Vec<_>>();
            let proxy = if proxy.is_empty() {
                vec![tris[0]]
            } else {
                proxy
            };
            proxies.push(HlodComponentProxy {
                component: c.name.clone(),
                source_triangles: tris.len() as u32,
                proxy_triangles: proxy,
            });
        }
        levels.push(HlodLevel { level, proxies });
    }
    Ok(HlodAsset {
        cell_name: input.cell_name.clone(),
        levels,
    })
}

fn write_name(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

/// 产物 canonical 二进制编码(magic + version + 全字段,LE)。
pub fn encode_hlod_asset(asset: &HlodAsset) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&HLOD_ASSET_MAGIC);
    buf.extend_from_slice(&HLOD_ASSET_VERSION.to_le_bytes());
    write_name(&mut buf, &asset.cell_name);
    buf.extend_from_slice(&(asset.levels.len() as u32).to_le_bytes());
    for l in &asset.levels {
        buf.extend_from_slice(&l.level.to_le_bytes());
        buf.extend_from_slice(&(l.proxies.len() as u32).to_le_bytes());
        for p in &l.proxies {
            write_name(&mut buf, &p.component);
            buf.extend_from_slice(&p.source_triangles.to_le_bytes());
            buf.extend_from_slice(&(p.proxy_triangles.len() as u32).to_le_bytes());
            for t in &p.proxy_triangles {
                for v in t {
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
    }
    buf
}

/// 产物 digest(双构建 hash 相等判据事实源;SHA-256 与 M01/M79 同源)。
pub fn hlod_asset_digest(asset: &HlodAsset) -> [u8; 32] {
    canon::digest_bytes(&encode_hlod_asset(asset))
}

/// 确定性 demo 输入(cell 几何资产 fixture:harness/单测/工具三方正例同一事实
/// 源;4 Component × 384 三角,LCG 位级确定)。
pub fn demo_bake_input() -> HlodBakeInput {
    let mut s: u64 = 0x410d_5eed_aa55_1234u64.wrapping_add(7);
    let mut next = move || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 11) % 200_000) as f32 / 1000.0 - 100.0
    };
    let mut components = Vec::new();
    for ci in 0..4u32 {
        let mut triangles = Vec::with_capacity(384);
        for _ in 0..384 {
            let mut t = [0.0f32; 9];
            for v in t.iter_mut() {
                *v = next();
            }
            triangles.push(t);
        }
        components.push(ComponentGeometry {
            name: format!("comp_{ci}"),
            triangles,
        });
    }
    HlodBakeInput {
        cell_name: "cell_demo".to_string(),
        levels: 3,
        components,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: &[u8; 32]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// RXS-0364:双构建 hash 相等——同输入两次独立烘焙产物字节/digest 逐位一致。
    #[test]
    //@ spec: RXS-0364
    fn double_build_hash_equal() {
        let input = demo_bake_input();
        let a = bake_hlod(&input).expect("bake 1");
        let b = bake_hlod(&input).expect("bake 2");
        assert_eq!(a, b);
        assert_eq!(encode_hlod_asset(&a), encode_hlod_asset(&b));
        assert_eq!(hlod_asset_digest(&a), hlod_asset_digest(&b));
        // 产物即资产:magic/version 冻结面。
        let bytes = encode_hlod_asset(&a);
        assert_eq!(&bytes[..4], &HLOD_ASSET_MAGIC);
        assert_eq!(bytes[4..6], HLOD_ASSET_VERSION.to_le_bytes());
    }

    /// RXS-0364:输入扰动免疫——三角面声明序与 Component 声明序打乱,产物
    /// digest 逐位不变。
    #[test]
    //@ spec: RXS-0364
    fn declaration_order_perturbation_invariant() {
        let mut input = demo_bake_input();
        let base = hlod_asset_digest(&bake_hlod(&input).unwrap());
        input.components.reverse();
        for c in input.components.iter_mut() {
            c.triangles.reverse();
            c.triangles.rotate_left(7);
        }
        let perturbed = hlod_asset_digest(&bake_hlod(&input).unwrap());
        assert_eq!(base, perturbed, "声明序扰动不得影响产物 hash");
    }

    /// RXS-0364(RED 臂能红证明):几何内容扰动必须分叉。
    #[test]
    //@ spec: RXS-0364
    fn geometry_perturbation_diverges() {
        let input = demo_bake_input();
        let base = hlod_asset_digest(&bake_hlod(&input).unwrap());
        let mut moved = input.clone();
        moved.components[0].triangles[0][0] += 1.0;
        assert_ne!(base, hlod_asset_digest(&bake_hlod(&moved).unwrap()));
        let mut added = input;
        added.components[1].triangles.push([0.0f32; 9]);
        assert_ne!(base, hlod_asset_digest(&bake_hlod(&added).unwrap()));
        // 层数为烘焙属性:层数不同产物不同。
        let mut deeper = demo_bake_input();
        deeper.levels = 4;
        assert_ne!(base, hlod_asset_digest(&bake_hlod(&deeper).unwrap()));
    }

    /// RXS-0364:逐层简化语义——level 升则代理三角数单调不增;level 0 = 全量。
    #[test]
    //@ spec: RXS-0364
    fn levels_monotonic_decimation() {
        let asset = bake_hlod(&demo_bake_input()).unwrap();
        assert_eq!(asset.levels.len(), 3);
        for l in asset.levels.windows(2) {
            for (a, b) in l[0].proxies.iter().zip(l[1].proxies.iter()) {
                assert!(a.proxy_triangles.len() >= b.proxy_triangles.len());
                assert_eq!(a.source_triangles, b.source_triangles);
            }
        }
        for p in &asset.levels[0].proxies {
            assert_eq!(p.proxy_triangles.len() as u32, p.source_triangles);
        }
        let _ = hex(&hlod_asset_digest(&asset));
    }

    /// RXS-0364:非法输入 fail-closed(零 Component/零三角/层数越界/非有限
    /// 坐标/重名)。
    #[test]
    //@ spec: RXS-0364
    fn invalid_input_fail_closed() {
        let mut zero_comp = demo_bake_input();
        zero_comp.components.clear();
        assert!(bake_hlod(&zero_comp).is_err());
        let mut zero_tri = demo_bake_input();
        zero_tri.components[0].triangles.clear();
        assert!(bake_hlod(&zero_tri).is_err());
        let mut bad_levels = demo_bake_input();
        bad_levels.levels = 0;
        assert!(bake_hlod(&bad_levels).is_err());
        bad_levels.levels = MAX_HLOD_LEVELS + 1;
        assert!(bake_hlod(&bad_levels).is_err());
        let mut nan = demo_bake_input();
        nan.components[0].triangles[0][0] = f32::NAN;
        assert!(bake_hlod(&nan).is_err());
        let mut dup = demo_bake_input();
        dup.components[1].name = dup.components[0].name.clone();
        assert!(bake_hlod(&dup).is_err());
    }
}
