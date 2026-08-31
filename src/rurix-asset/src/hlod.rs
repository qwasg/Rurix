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
/// 产物资产格式版本(v1 = 无属性面;无 UV 输入恒走 v1,字节面冻结)。
pub const HLOD_ASSET_VERSION: u16 = 1;
/// 产物资产格式版本(G31+ #96 属性臂:每三角 9×f32 位置追加 6×f32 corner
/// UV;仅全量 UV 输入产出——v1 编码路径 0-byte 保留)。
pub const HLOD_ASSET_VERSION_ATTRS: u16 = 2;

/// 输入:cell 几何资产的单个 Component(命名 + 三角面集合,每三角 9×f32)。
/// G31+ #96:`uv` = 与 `triangles` 平行的逐三角 corner UV(6 f32/tri,
/// 顶点序同源;None = 无属性输入,既有路径 0-byte)。跨 Component 须齐次
/// (全 Some 或全 None,validate_input fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentGeometry {
    pub name: String,
    pub triangles: Vec<[f32; 9]>,
    pub uv: Option<Vec<[f32; 6]>>,
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
/// G31+ #96:`uv` = 与 `proxy_triangles` 平行的逐三角 corner UV(仅属性臂
/// 产出;None = v1 形态,编码字节面不变)。
#[derive(Debug, Clone, PartialEq)]
pub struct HlodComponentProxy {
    pub component: String,
    pub source_triangles: u32,
    pub proxy_triangles: Vec<[f32; 9]>,
    pub uv: Option<Vec<[f32; 6]>>,
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
    // #96:UV 齐次性(全 Some 或全 None——bake 产物版本单一确定,禁混合)。
    let with_uv = input.components[0].uv.is_some();
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
        // #96 UV 面校验(fail-closed:齐次 + 平行等长 + 有限)。
        if c.uv.is_some() != with_uv {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                format!("hlod Component {} UV 在场性与首 Component 不齐(混合输入拒)", c.name),
            ));
        }
        if let Some(uv) = &c.uv {
            if uv.len() != c.triangles.len() {
                return Err(AssetError::new(
                    ErrorKind::Invalid,
                    format!(
                        "hlod Component {} UV 行数 {} ≠ 三角数 {}",
                        c.name,
                        uv.len(),
                        c.triangles.len()
                    ),
                ));
            }
            for u in uv {
                if !u.iter().all(|v| v.is_finite()) {
                    return Err(AssetError::new(
                        ErrorKind::Invalid,
                        format!("hlod Component {} 含非有限 UV", c.name),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 离线烘焙:逐 Component canonical 排序 → 逐层确定性抽取简化(stride = 2^level,
/// 至少保留 1 三角)→ 层级资产。纯函数,同输入同输出。
/// #96 边界:stride 抽面烘焙器不承载属性臂(产物 uv 恒 None,编码恒 v1
/// ——M111 golden 锚 0-byte;属性臂 = [`bake_hlod_merged`] 独占)。
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
                uv: None,
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
/// #96 版本分派:全 proxy 带 UV → v2(每三角 9×f32 位置后追加 6×f32
/// corner UV);全无 → v1 字节面不变。混合 = bake 构造不变量破坏,
/// assert 拒(两烘焙器输出恒齐次,validate_input 输入面已 fail-closed)。
pub fn encode_hlod_asset(asset: &HlodAsset) -> Vec<u8> {
    let n_total: usize = asset.levels.iter().map(|l| l.proxies.len()).sum();
    let n_with_uv: usize = asset
        .levels
        .iter()
        .flat_map(|l| &l.proxies)
        .filter(|p| p.uv.is_some())
        .count();
    assert!(
        n_with_uv == 0 || n_with_uv == n_total,
        "HlodAsset UV 非齐次({n_with_uv}/{n_total})——bake 构造不变量破坏"
    );
    let with_uv = n_total > 0 && n_with_uv == n_total;
    let version = if with_uv {
        HLOD_ASSET_VERSION_ATTRS
    } else {
        HLOD_ASSET_VERSION
    };
    let mut buf = Vec::new();
    buf.extend_from_slice(&HLOD_ASSET_MAGIC);
    buf.extend_from_slice(&version.to_le_bytes());
    write_name(&mut buf, &asset.cell_name);
    buf.extend_from_slice(&(asset.levels.len() as u32).to_le_bytes());
    for l in &asset.levels {
        buf.extend_from_slice(&l.level.to_le_bytes());
        buf.extend_from_slice(&(l.proxies.len() as u32).to_le_bytes());
        for p in &l.proxies {
            write_name(&mut buf, &p.component);
            buf.extend_from_slice(&p.source_triangles.to_le_bytes());
            buf.extend_from_slice(&(p.proxy_triangles.len() as u32).to_le_bytes());
            if with_uv {
                let uv = p.uv.as_ref().expect("齐次已断言");
                assert_eq!(
                    uv.len(),
                    p.proxy_triangles.len(),
                    "proxy UV 行数与三角数不齐——bake 构造不变量破坏"
                );
                for (t, u) in p.proxy_triangles.iter().zip(uv) {
                    for v in t {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                    for v in u {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                }
            } else {
                for t in &p.proxy_triangles {
                    for v in t {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
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

// ---------------------------------------------------------------------------
// G31+ #67/#97 HLOD 质量烘焙(跨 Component 合并 + QEM 简化——UE5 WP HLOD
// Merged/Simplified 型对齐;既有 [`bake_hlod`] stride 抽面 0-byte 不动,
// M111 golden 锚维持,本函数为加性第二烘焙器)
// ---------------------------------------------------------------------------

/// 合并层 proxy 的 Component 名(RXHL v1 结构兼容——合并层 proxies 长度 1,
/// 运行时选层面零改动;`source_triangles` = 合并前总三角数)。
pub const HLOD_MERGED_COMPONENT: &str = "__merged__";

/// 质量烘焙(G31+ #67/#97):
/// - **L0 = 全量几何**(逐 Component canonical 序,与 [`bake_hlod`] L0 逐位
///   同值——运行时 Full/HLOD 互斥切换协议(RXS-0364 三态)零改动);
/// - **L ≥ 1 = 跨 Component 合并 → 位置 bits 精确焊接 → QEM 简化到
///   `总三角 / 2^level`**(rurix-geom-build `qem::simplify_free_mesh` 事实源
///   直调:最优位置收缩 + fold-over 拒绝,替代 stride 抽面的无误差控制欠采样
///   ——「不要用 stride 抽面冒充远处降复杂度」调研结论字面兑现);
/// - 产物结构 = 既有 RXHL v1(合并层单 proxy `__merged__`);双构建 hash
///   相等/声明序扰动免疫/几何扰动分叉三判据与既有烘焙器同锚(单测)。
/// - **G31+ #96 属性臂**(输入 UV 齐次在场时):canonical 排序键扩 UV bits
///   (位置重复三角的平行 UV 序与声明序解耦),焊接键 = (位置, UV) bits
///   (接缝顶点不误并),逐层直调 `qem::simplify_free_mesh_attrs`,代理
///   三角带 corner UV(编码 = RXHL v2);无 UV 输入路径产物逐位不变(v1)。
pub fn bake_hlod_merged(input: &HlodBakeInput) -> Result<HlodAsset> {
    validate_input(input)?;
    // UV 齐次性已由 validate_input 保证(首 Component 即全体)。
    let with_uv = input.components[0].uv.is_some();
    // Component 分发序 canonical 化(声明序扰动免疫,与 bake_hlod 同律)。
    let mut comps: Vec<&ComponentGeometry> = input.components.iter().collect();
    comps.sort_by(|a, b| a.name.cmp(&b.name));
    // 逐 Component canonical 序:经索引置换排序(稳定序,产物元素序列与
    // 直接 sort_by_cached_key(tri_sort_key) 逐位一致);UV 在场时键尾扩
    // UV bits,缺席补常量零(比较结果与既有键完全同序 ⇒ v1 路径 0-漂移)。
    let mut sorted_tris: Vec<Vec<[f32; 9]>> = Vec::with_capacity(comps.len());
    let mut sorted_uv: Vec<Option<Vec<[f32; 6]>>> = Vec::with_capacity(comps.len());
    for c in &comps {
        let mut order: Vec<u32> = (0..c.triangles.len() as u32).collect();
        order.sort_by_cached_key(|&i| {
            (
                tri_sort_key(&c.triangles[i as usize]),
                c.uv
                    .as_ref()
                    .map_or([0u32; 6], |uv| uv[i as usize].map(f32::to_bits)),
            )
        });
        sorted_tris.push(order.iter().map(|&i| c.triangles[i as usize]).collect());
        sorted_uv.push(
            c.uv.as_ref()
                .map(|uv| order.iter().map(|&i| uv[i as usize]).collect()),
        );
    }
    // L0:全量(逐 Component,bake_hlod L0 同形;#96 UV 平行透传)。
    let mut levels = Vec::with_capacity(input.levels as usize);
    let l0_proxies: Vec<HlodComponentProxy> = comps
        .iter()
        .zip(sorted_tris.iter().zip(&sorted_uv))
        .map(|(c, (tris, uv))| HlodComponentProxy {
            component: c.name.clone(),
            source_triangles: tris.len() as u32,
            proxy_triangles: tris.clone(),
            uv: uv.clone(),
        })
        .collect();
    levels.push(HlodLevel {
        level: 0,
        proxies: l0_proxies,
    });
    if input.levels > 1 && !with_uv {
        // ── 无属性臂(既有路径逐字:位置 bits 焊接 + simplify_free_mesh)──
        // 跨 Component 合并(canonical 序拼接)→ 位置 bits 精确焊接。
        let total: usize = sorted_tris.iter().map(Vec::len).sum();
        let mut weld: std::collections::HashMap<[u32; 3], u32> =
            std::collections::HashMap::with_capacity(total * 3);
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for tris in &sorted_tris {
            for t in tris {
                for k in 0..3 {
                    let p = [t[k * 3], t[k * 3 + 1], t[k * 3 + 2]];
                    let key = p.map(f32::to_bits);
                    let next = positions.len() as u32;
                    let id = *weld.entry(key).or_insert_with(|| {
                        positions.push(p);
                        next
                    });
                    indices.push(id);
                }
            }
        }
        // 逐层 QEM 简化(自上一层产物继续减半——层间累进,总代价 O(n)级)。
        let mut cur_pos = positions;
        let mut cur_idx = indices;
        for level in 1..input.levels {
            let target = (total >> level).max(1);
            let (np, ni, _err) =
                rurix_geom_build::qem::simplify_free_mesh(&cur_pos, &cur_idx, target);
            cur_pos = np;
            cur_idx = ni;
            let proxy_triangles: Vec<[f32; 9]> = cur_idx
                .chunks_exact(3)
                .map(|t| {
                    let mut out = [0.0f32; 9];
                    for k in 0..3 {
                        let p = cur_pos[t[k] as usize];
                        out[k * 3..k * 3 + 3].copy_from_slice(&p);
                    }
                    out
                })
                .collect();
            levels.push(HlodLevel {
                level,
                proxies: vec![HlodComponentProxy {
                    component: HLOD_MERGED_COMPONENT.to_string(),
                    source_triangles: total as u32,
                    proxy_triangles,
                    uv: None,
                }],
            });
        }
    } else if input.levels > 1 {
        // ── #96 属性臂:焊接键 = (位置, UV) bits(同位置不同 UV = 接缝拷贝,
        //    不误并;简化链按接缝顶点保守锁定,rurix-geom-build 同律)──
        let total: usize = sorted_tris.iter().map(Vec::len).sum();
        let mut weld: std::collections::HashMap<[u32; 5], u32> =
            std::collections::HashMap::with_capacity(total * 3);
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut vertex_uv: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (tris, uvs) in sorted_tris.iter().zip(&sorted_uv) {
            let uvs = uvs.as_ref().expect("UV 齐次(validate_input)");
            for (t, u) in tris.iter().zip(uvs) {
                for k in 0..3 {
                    let p = [t[k * 3], t[k * 3 + 1], t[k * 3 + 2]];
                    let uvk = [u[k * 2], u[k * 2 + 1]];
                    let key = [
                        p[0].to_bits(),
                        p[1].to_bits(),
                        p[2].to_bits(),
                        uvk[0].to_bits(),
                        uvk[1].to_bits(),
                    ];
                    let next = positions.len() as u32;
                    let id = *weld.entry(key).or_insert_with(|| {
                        positions.push(p);
                        vertex_uv.push(uvk);
                        next
                    });
                    indices.push(id);
                }
            }
        }
        // 逐层属性保持 QEM 简化(层间累进,与无属性臂同律;简化产物顶点
        // UV 平行表 → 代理三角 corner UV 经 uv[indices[k]] 取用)。
        let mut cur_pos = positions;
        let mut cur_idx = indices;
        let mut cur_uv = vertex_uv;
        for level in 1..input.levels {
            let target = (total >> level).max(1);
            let out = rurix_geom_build::qem::simplify_free_mesh_attrs(
                &cur_pos, &cur_idx, &cur_uv, None, target,
            )
            .map_err(|e| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("hlod 属性简化 L{level}: {e}"),
                )
            })?;
            cur_pos = out.positions;
            cur_idx = out.indices;
            cur_uv = out.uv;
            let n_out = cur_idx.len() / 3;
            let mut proxy_triangles: Vec<[f32; 9]> = Vec::with_capacity(n_out);
            let mut proxy_uv: Vec<[f32; 6]> = Vec::with_capacity(n_out);
            for t in cur_idx.chunks_exact(3) {
                let mut tp = [0.0f32; 9];
                let mut tu = [0.0f32; 6];
                for k in 0..3 {
                    tp[k * 3..k * 3 + 3].copy_from_slice(&cur_pos[t[k] as usize]);
                    tu[k * 2..k * 2 + 2].copy_from_slice(&cur_uv[t[k] as usize]);
                }
                proxy_triangles.push(tp);
                proxy_uv.push(tu);
            }
            levels.push(HlodLevel {
                level,
                proxies: vec![HlodComponentProxy {
                    component: HLOD_MERGED_COMPONENT.to_string(),
                    source_triangles: total as u32,
                    proxy_triangles,
                    uv: Some(proxy_uv),
                }],
            });
        }
    }
    Ok(HlodAsset {
        cell_name: input.cell_name.clone(),
        levels,
    })
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
            uv: None,
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

    /// 连贯网格 demo 输入(合并简化质量面需要共享边拓扑——demo_bake_input
    /// 的随机三角汤无共享边,QEM 无可收缩边;本 fixture = 球面四象限分片
    /// 4 Component,跨 Component 共享边界,合并后可充分简化)。
    fn merged_demo_input(levels: u32) -> HlodBakeInput {
        let mesh = rurix_geom_build::TriMesh::uv_sphere(1.0, 24, 24);
        let mut comps: Vec<Vec<[f32; 9]>> = vec![Vec::new(); 4];
        for f in 0..mesh.triangle_count() {
            let t = mesh.triangle(f);
            let mut flat = [0.0f32; 9];
            let mut cx = 0.0f32;
            let mut cz = 0.0f32;
            for k in 0..3 {
                let p = mesh.positions[t[k] as usize];
                flat[k * 3..k * 3 + 3].copy_from_slice(&p);
                cx += p[0];
                cz += p[2];
            }
            let q = usize::from(cx >= 0.0) + 2 * usize::from(cz >= 0.0);
            comps[q].push(flat);
        }
        HlodBakeInput {
            cell_name: "cell_sphere".to_string(),
            levels,
            components: comps
                .into_iter()
                .enumerate()
                .map(|(i, triangles)| ComponentGeometry {
                    name: format!("quad_{i}"),
                    triangles,
                    uv: None,
                })
                .collect(),
        }
    }

    /// #96 属性臂 fixture:球面四象限分片 + 平面投影 corner UV(与
    /// merged_demo_input 同几何,UV = (x,z) 仿射投影——简化插值有解析对照)。
    fn merged_demo_input_uv(levels: u32) -> HlodBakeInput {
        let mut input = merged_demo_input(levels);
        for c in input.components.iter_mut() {
            let uv: Vec<[f32; 6]> = c
                .triangles
                .iter()
                .map(|t| {
                    let mut u = [0.0f32; 6];
                    for k in 0..3 {
                        u[k * 2] = (t[k * 3] + 1.0) * 0.5;
                        u[k * 2 + 1] = (t[k * 3 + 2] + 1.0) * 0.5;
                    }
                    u
                })
                .collect();
            c.uv = Some(uv);
        }
        input
    }

    /// G31+ #67/#97:质量烘焙三判据(双构建/声明序免疫/几何扰动分叉)与
    /// 合并简化性质(L0 全量逐位 == 既有烘焙器 L0;L≥1 单 proxy 合并 +
    /// 三角数按 2^level 递减;stride 抽面对照登记)。
    #[test]
    fn merged_bake_invariants_and_quality() {
        let input = merged_demo_input(4);
        let a = bake_hlod_merged(&input).expect("bake 1");
        let b = bake_hlod_merged(&input).expect("bake 2");
        assert_eq!(hlod_asset_digest(&a), hlod_asset_digest(&b), "双构建漂移");
        // 声明序扰动免疫。
        let mut perturbed = input.clone();
        perturbed.components.reverse();
        for c in perturbed.components.iter_mut() {
            c.triangles.reverse();
        }
        assert_eq!(
            hlod_asset_digest(&bake_hlod_merged(&perturbed).unwrap()),
            hlod_asset_digest(&a),
            "声明序扰动必须免疫"
        );
        // 几何扰动分叉。
        let mut geo = input.clone();
        geo.components[0].triangles[0][0] += 0.25;
        assert_ne!(
            hlod_asset_digest(&bake_hlod_merged(&geo).unwrap()),
            hlod_asset_digest(&a),
            "几何扰动必须分叉"
        );
        // L0 = 全量,与既有 stride 烘焙器 L0 逐位同值(互斥切换协议 0 改动)。
        let stride = bake_hlod(&input).expect("stride 对照");
        assert_eq!(a.levels[0], stride.levels[0], "L0 全量面与既有烘焙器不一致");
        // L≥1:单 __merged__ proxy + 三角数按 2^level 目标递减(±簇化余量)。
        let total: usize = input.components.iter().map(|c| c.triangles.len()).sum();
        for l in 1..4usize {
            let lv = &a.levels[l];
            assert_eq!(lv.proxies.len(), 1, "合并层须单 proxy");
            assert_eq!(lv.proxies[0].component, HLOD_MERGED_COMPONENT);
            let n = lv.proxies[0].proxy_triangles.len();
            let target = total >> l;
            assert!(
                n <= target + target / 4 + 8,
                "L{l} 三角数 {n} 未接近目标 {target}"
            );
            assert!(n >= 1);
            // 质量粗判:简化产物顶点仍在单位球邻域(QEM 不飞点;stride 抽面
            // 无此保证面——它只是欠采样)。
            for t in &lv.proxies[0].proxy_triangles {
                for k in 0..3 {
                    let r = (t[k * 3] * t[k * 3] + t[k * 3 + 1] * t[k * 3 + 1]
                        + t[k * 3 + 2] * t[k * 3 + 2])
                        .sqrt();
                    assert!((r - 1.0).abs() < 0.25, "L{l} 顶点飞出球面邻域 r={r}");
                }
            }
        }
        // 对照登记(#67 数据面:stride 抽面同层三角数相近但为无误差控制
        // 欠采样——空间连贯性无保证;打印如实)。
        println!(
            "[hlod_merged_vs_stride] total={total} merged L1..3 = {:?} ; stride L1..3 = {:?}",
            (1..4)
                .map(|l| a.levels[l].proxies[0].proxy_triangles.len())
                .collect::<Vec<_>>(),
            (1..4)
                .map(|l| stride.levels[l]
                    .proxies
                    .iter()
                    .map(|p| p.proxy_triangles.len())
                    .sum::<usize>())
                .collect::<Vec<_>>(),
        );
    }

    /// G31+ #96:属性臂三判据(双构建/声明序免疫/UV 扰动分叉)+ RXHL v2
    /// 编码 + 位置面与无属性臂逐位一致(本 fixture UV = 位置仿射投影 ⇒
    /// 同位置 bits 同 UV bits ⇒ 无接缝 ⇒ crate 契约「位置/拓扑产物逐位
    /// 一致」可机核)+ 无 UV 输入编码恒 v1(字节面冻结)。
    #[test]
    fn merged_bake_uv_arm_invariants() {
        let input = merged_demo_input_uv(4);
        let a = bake_hlod_merged(&input).expect("uv bake 1");
        let b = bake_hlod_merged(&input).expect("uv bake 2");
        assert_eq!(hlod_asset_digest(&a), hlod_asset_digest(&b), "uv 臂双构建漂移");
        // 声明序扰动免疫(UV 平行表随三角同置换)。
        let mut perturbed = input.clone();
        perturbed.components.reverse();
        for c in perturbed.components.iter_mut() {
            c.triangles.reverse();
            if let Some(uv) = c.uv.as_mut() {
                uv.reverse();
            }
        }
        assert_eq!(
            hlod_asset_digest(&bake_hlod_merged(&perturbed).unwrap()),
            hlod_asset_digest(&a),
            "uv 臂声明序扰动必须免疫"
        );
        // UV 内容扰动必须分叉(UV 进编码字节)。
        let mut uvmoved = input.clone();
        uvmoved.components[0].uv.as_mut().unwrap()[0][0] += 0.25;
        assert_ne!(
            hlod_asset_digest(&bake_hlod_merged(&uvmoved).unwrap()),
            hlod_asset_digest(&a),
            "UV 扰动必须分叉"
        );
        // 编码版本:uv 臂 = v2;无 uv = v1(字节面冻结)。
        let bytes = encode_hlod_asset(&a);
        assert_eq!(&bytes[..4], &HLOD_ASSET_MAGIC);
        assert_eq!(bytes[4..6], HLOD_ASSET_VERSION_ATTRS.to_le_bytes());
        let plain_input = merged_demo_input(4);
        let plain = bake_hlod_merged(&plain_input).expect("无 uv 对照");
        let plain_bytes = encode_hlod_asset(&plain);
        assert_eq!(plain_bytes[4..6], HLOD_ASSET_VERSION.to_le_bytes());
        // 位置面对拍:本 fixture UV = f(位置) ⇒ (位置,UV) 焊接 ≡ 位置焊接
        // (无接缝)⇒ 每层位置/拓扑与无属性臂逐位一致(crate 契约锚)。
        assert_eq!(a.levels.len(), plain.levels.len());
        for (la, lp) in a.levels.iter().zip(&plain.levels) {
            assert_eq!(la.proxies.len(), lp.proxies.len());
            for (pa, pp) in la.proxies.iter().zip(&lp.proxies) {
                assert_eq!(
                    pa.proxy_triangles
                        .iter()
                        .map(|t| t.map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    pp.proxy_triangles
                        .iter()
                        .map(|t| t.map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    "L{} 位置面与无属性臂漂移",
                    la.level
                );
                // UV 平行表:行数齐 + 有限 + L0 = 输入投影逐位。
                let uv = pa.uv.as_ref().expect("uv 臂每 proxy 带 UV");
                assert_eq!(uv.len(), pa.proxy_triangles.len());
                for (t, u) in pa.proxy_triangles.iter().zip(uv) {
                    assert!(u.iter().all(|v| v.is_finite()));
                    if la.level == 0 {
                        for k in 0..3 {
                            assert_eq!(
                                u[k * 2].to_bits(),
                                ((t[k * 3] + 1.0) * 0.5).to_bits(),
                                "L0 UV 须逐位等于输入投影"
                            );
                            assert_eq!(
                                u[k * 2 + 1].to_bits(),
                                ((t[k * 3 + 2] + 1.0) * 0.5).to_bits(),
                            );
                        }
                    }
                }
            }
        }
        // 混合 UV 在场性输入 fail-closed。
        let mut mixed = input.clone();
        mixed.components[1].uv = None;
        assert!(bake_hlod_merged(&mixed).is_err(), "混合 UV 输入必须拒");
        // UV 行数不齐 fail-closed。
        let mut ragged = input;
        ragged.components[0].uv.as_mut().unwrap().pop();
        assert!(bake_hlod_merged(&ragged).is_err(), "UV 行数不齐必须拒");
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
