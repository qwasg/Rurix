//! 材质参数侧表扩展通道(G9.5 M115 皮肤/M114 毛发共享;RFC-0025 §4.L 🔒 显式
//! 修订行;spec/display_pipeline.md RXS-0373 L5 / RXS-0372 L5)。
//!
//! //@ spec: RXS-0373
//! //@ spec: RXS-0372
//!
//! 修订行语义(RFC-0025 §4.L 逐字):
//!
//! - **`MaterialClosure` 32B 定长布局/字段含义/flags 位段分配 0-byte 保持**:
//!   本模块不向 32B 内联面添加任何字节;[`closure_32b_layout_digest`] 产出冻结
//!   面 digest(既有打包面逐字段 LE 序列化 SHA-256),[`check_closure_face_untouched`]
//!   机核 flags 未分配位段与 `reserved` 预留拓扑字段位**不消费**(任一消费即
//!   typed `Err(FieldOverreach)`,禁静默扩 RED 锚)。
//! - **资产化侧表扩展通道**:Burley 扩散 profile(RGB 三通道 falloff)与
//!   Marschner 参数集(R/TT/TRT 瓣、基调色、高光偏移、medulla)作为
//!   [`MaterialSideTable`] 资产**按材质槽 ID 索引**接入单层 closure 求值;
//!   canonical 二进制编解码逐字节往返 + digest 签名(M01/M85 资产通道口径,
//!   烘焙/打包/manifest 入 DDC 的资产形态)。
//! - **侧表缺省 ≡ 无专项 lobe,既有材质输出逐位不变**:
//!   [`assert_default_table_invariant`] 机核——缺省(空/无)侧表路径输出
//!   digest 与无侧表基线 digest 必须逐位相等,不等即 RED(修订行零漂移证明)。
//! - **越权拒录**:材质槽 ID 越界(≥ 材质表长度)即 `Err(UnknownMaterialSlot)`;
//!   扩展闭集外字段/预留位消费即 `Err(FieldOverreach)`(侧表越权 RED 锚)。
//!
//! 纪律:host 纯 safe 确定性;零新 FFI;32B 冻结面只消费不重定——本文件不
//! 修改 `graph::types::MaterialClosure` 任何字节。

use std::collections::BTreeMap;

use rurix_pkg::sha256;

use crate::graph::types::MaterialClosure;

use super::closure::MaterialParams;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// 侧表资产 canonical 二进制 magic("RXST")。
pub const SIDE_TABLE_MAGIC: [u8; 4] = *b"RXST";
/// 侧表资产格式版本。
pub const SIDE_TABLE_VERSION: u16 = 1;
/// flags 已分配位段掩码(G6 冻结:bit0 alpha blend / bit1 双面;其余位段未分配,
/// 不消费)。
pub const FLAGS_ASSIGNED_MASK: u8 = 0b11;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 侧表失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum SideTableError {
    /// 字节流截断。
    Truncated { at: usize, need: usize },
    /// 解码后残余字节。
    TrailingBytes { extra: usize },
    /// magic 不符。
    BadMagic,
    /// 不支持的资产版本。
    UnsupportedVersion(u16),
    /// 非 canonical 构造。
    NotCanonical(&'static str),
    /// 输入含非有限值。
    NonFiniteValue { field: &'static str },
    /// 资产签名/内容篡改。
    AssetTampered { why: &'static str },
    /// 材质槽 ID 越界(侧表越权 RED 锚)。
    UnknownMaterialSlot { slot: u32, table_len: u32 },
    /// 扩展闭集外字段/32B 预留位消费(禁静默扩 RED 锚)。
    FieldOverreach { field: &'static str },
    /// 缺省侧表输出 ≠ 无侧表基线输出(修订行零漂移证明违反,RED 锚)。
    DefaultSideTableAltersOutput,
}

impl std::fmt::Display for SideTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SideTableError::Truncated { at, need } => write!(f, "truncated: offset {at} 需 {need}"),
            SideTableError::TrailingBytes { extra } => write!(f, "trailing bytes: 残余 {extra}"),
            SideTableError::BadMagic => write!(f, "bad magic(非 RXST)"),
            SideTableError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            SideTableError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            SideTableError::NonFiniteValue { field } => write!(f, "{field} 含非有限值"),
            SideTableError::AssetTampered { why } => write!(f, "侧表资产篡改: {why}(RED)"),
            SideTableError::UnknownMaterialSlot { slot, table_len } => {
                write!(f, "材质槽 {slot} 越界(表长 {table_len};侧表越权,RED)")
            }
            SideTableError::FieldOverreach { field } => {
                write!(f, "扩展闭集外/预留位消费: {field}(禁静默扩,RED)")
            }
            SideTableError::DefaultSideTableAltersOutput => {
                write!(f, "缺省侧表 ≡ 既有输出逐位不变违反(RED)")
            }
        }
    }
}

impl std::error::Error for SideTableError {}

pub type Result<T> = std::result::Result<T, SideTableError>;

// ---------------------------------------------------------------------------
// 扩展参数载荷(资产属性闭集;Burley 扩散 profile / Marschner 参数集)
// ---------------------------------------------------------------------------

/// Burley 扩散 profile(RGB 三通道 falloff 参数;per-material 资产;全零 = 无
/// 扩散衰减 ⇒ 退化纯漫反射)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurleyProfile {
    /// RGB 三通道 falloff 长度(≥0;0 = 该通道无扩散)。
    pub falloff_rgb: [f32; 3],
}

/// Marschner 参数集(纵向/方位角分离参数化;每缕基调色、高光偏移、medulla
/// 配置为资产属性)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarschnerParams {
    /// 每缕基调色 RGB。
    pub base_color: [f32; 3],
    /// R 瓣纵向高光偏移(弧度)。
    pub shift_r: f32,
    /// TT 瓣纵向高光偏移。
    pub shift_tt: f32,
    /// TRT 瓣纵向高光偏移。
    pub shift_trt: f32,
    /// R 瓣纵向宽度。
    pub width_r: f32,
    /// TT 瓣纵向宽度。
    pub width_tt: f32,
    /// TRT 瓣纵向宽度。
    pub width_trt: f32,
    /// medulla 配置指数(≥0;TT 瓣髓质衰减)。
    pub medulla: f32,
}

fn check_finite3(tag: &'static str, v: &[f32; 3]) -> Result<()> {
    if v.iter().all(|x| x.is_finite()) && v.iter().all(|x| *x >= 0.0) {
        Ok(())
    } else {
        Err(SideTableError::NonFiniteValue { field: tag })
    }
}

impl MarschnerParams {
    /// 域校验(宽度/medulla 非负有限;偏移有限)。
    pub fn validate(&self) -> Result<()> {
        check_finite3("base_color", &self.base_color)?;
        for (tag, v) in [
            ("shift_r", self.shift_r),
            ("shift_tt", self.shift_tt),
            ("shift_trt", self.shift_trt),
        ] {
            if !v.is_finite() {
                return Err(SideTableError::NonFiniteValue { field: tag });
            }
        }
        for (tag, v) in [
            ("width_r", self.width_r),
            ("width_tt", self.width_tt),
            ("width_trt", self.width_trt),
            ("medulla", self.medulla),
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(SideTableError::NonFiniteValue { field: tag });
            }
        }
        Ok(())
    }
}

/// 扩展 lobe 闭集(侧表条目载荷;tag 0=Burley 1=Marschner,闭集外 tag 即
/// [`SideTableError::FieldOverreach`])。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LobeExtension {
    Burley(BurleyProfile),
    Marschner(MarschnerParams),
}

// ---------------------------------------------------------------------------
// 侧表(按材质槽 ID 索引;canonical 编解码 + 签名)
// ---------------------------------------------------------------------------

/// 材质参数侧表(条目按材质槽 ID 升序 canonical;BTreeMap 即确定性序)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialSideTable {
    entries: BTreeMap<u32, LobeExtension>,
}

impl MaterialSideTable {
    /// 缺省侧表(空;≡ 无专项 lobe)。
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 注册扩展条目(装配面):**材质槽 ID 越界即拒录**(越权 RED 锚);载荷
    /// 域校验一体。
    pub fn insert(&mut self, slot: u32, ext: LobeExtension, material_table_len: u32) -> Result<()> {
        if slot >= material_table_len {
            return Err(SideTableError::UnknownMaterialSlot { slot, table_len: material_table_len });
        }
        match &ext {
            LobeExtension::Burley(p) => check_finite3("falloff_rgb", &p.falloff_rgb)?,
            LobeExtension::Marschner(p) => p.validate()?,
        }
        self.entries.insert(slot, ext);
        Ok(())
    }

    /// 按材质槽 ID 查询(缺省 = None ≡ 无专项 lobe)。
    pub fn lookup(&self, slot: u32) -> Option<&LobeExtension> {
        self.entries.get(&slot)
    }

    /// canonical 序迭代(编码/hashing 事实源)。
    pub fn iter(&self) -> impl Iterator<Item = (u32, LobeExtension)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, *v))
    }
}

fn write_ext(w: &mut Vec<u8>, ext: &LobeExtension) {
    match ext {
        LobeExtension::Burley(p) => {
            w.push(0u8);
            for v in p.falloff_rgb {
                w.extend_from_slice(&v.to_le_bytes());
            }
        }
        LobeExtension::Marschner(p) => {
            w.push(1u8);
            for v in p.base_color {
                w.extend_from_slice(&v.to_le_bytes());
            }
            for v in [p.shift_r, p.shift_tt, p.shift_trt, p.width_r, p.width_tt, p.width_trt, p.medulla] {
                w.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
}

/// canonical 二进制编码(magic + version + 条目数 + 条目(槽 ID 升序),LE)。
pub fn encode_side_table(t: &MaterialSideTable) -> Vec<u8> {
    let mut w = Vec::new();
    w.extend_from_slice(&SIDE_TABLE_MAGIC);
    w.extend_from_slice(&SIDE_TABLE_VERSION.to_le_bytes());
    w.extend_from_slice(&(t.entries.len() as u32).to_le_bytes());
    for (slot, ext) in t.iter() {
        w.extend_from_slice(&slot.to_le_bytes());
        write_ext(&mut w, &ext);
    }
    w
}

/// canonical 二进制解码(逐位核验 + 域校验;闭集外 tag / 越界槽即拒录)。
pub fn decode_side_table(bytes: &[u8], material_table_len: u32) -> Result<MaterialSideTable> {
    let take = |pos: &mut usize, n: usize| -> Result<&[u8]> {
        if bytes.len() - *pos < n {
            return Err(SideTableError::Truncated { at: *pos, need: n });
        }
        let s = &bytes[*pos..*pos + n];
        *pos += n;
        Ok(s)
    };
    let mut pos = 0usize;
    if take(&mut pos, 4)? != SIDE_TABLE_MAGIC {
        return Err(SideTableError::BadMagic);
    }
    let ver = u16::from_le_bytes(take(&mut pos, 2)?.try_into().expect("u16"));
    if ver != SIDE_TABLE_VERSION {
        return Err(SideTableError::UnsupportedVersion(ver));
    }
    let count = u32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("u32"));
    let mut t = MaterialSideTable::new();
    let mut prev: Option<u32> = None;
    for _ in 0..count {
        let slot = u32::from_le_bytes(take(&mut pos, 4)?.try_into().expect("u32"));
        if prev.is_some_and(|p| slot <= p) {
            return Err(SideTableError::NotCanonical("槽 ID 非严格升序"));
        }
        prev = Some(slot);
        let tag = take(&mut pos, 1)?[0];
        let f32s = |pos: &mut usize, n: usize, out: &mut [f32]| -> Result<()> {
            for v in out.iter_mut().take(n) {
                *v = f32::from_le_bytes(take(pos, 4)?.try_into().expect("f32"));
            }
            Ok(())
        };
        let ext = match tag {
            0 => {
                let mut rgb = [0.0f32; 3];
                f32s(&mut pos, 3, &mut rgb)?;
                LobeExtension::Burley(BurleyProfile { falloff_rgb: rgb })
            }
            1 => {
                let mut base = [0.0f32; 3];
                f32s(&mut pos, 3, &mut base)?;
                let mut rest = [0.0f32; 7];
                f32s(&mut pos, 7, &mut rest)?;
                LobeExtension::Marschner(MarschnerParams {
                    base_color: base,
                    shift_r: rest[0],
                    shift_tt: rest[1],
                    shift_trt: rest[2],
                    width_r: rest[3],
                    width_tt: rest[4],
                    width_trt: rest[5],
                    medulla: rest[6],
                })
            }
            _ => return Err(SideTableError::FieldOverreach { field: "extension_tag" }),
        };
        t.insert(slot, ext, material_table_len)?;
    }
    if pos != bytes.len() {
        return Err(SideTableError::TrailingBytes { extra: bytes.len() - pos });
    }
    Ok(t)
}

/// 侧表签名(digest 即完整性;M01/M85 通道口径)。
pub fn side_table_signature(t: &MaterialSideTable) -> [u8; 32] {
    sha256::digest(&encode_side_table(t))
}

/// 侧表完整性核验(篡改即拒录)。
pub fn verify_side_table(t: &MaterialSideTable, expected_sig: &[u8; 32]) -> Result<()> {
    if &side_table_signature(t) != expected_sig {
        return Err(SideTableError::AssetTampered { why: "digest 不符" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 32B 冻结面核验(0-byte 保持 + 预留位不消费 + 缺省侧表零漂移)
// ---------------------------------------------------------------------------

/// `MaterialClosure` 32B 二进制布局 digest:默认参数打包面逐字段 LE 序列化
/// (8×u32 = 32B 逐字节)SHA-256。**本函数只消费既有冻结面,不重定**;布局/
/// 字段含义/flags 位段任何字节漂移 ⇒ digest 漂移 ⇒ golden 判 RED。
pub fn closure_32b_layout_digest() -> [u8; 32] {
    let c = MaterialParams::default().pack();
    let mut buf = Vec::with_capacity(32);
    buf.extend_from_slice(&c.albedo_rgba8.to_le_bytes());
    buf.extend_from_slice(&c.f0_rgba8.to_le_bytes());
    buf.extend_from_slice(&c.rough_metal_ao_flags.to_le_bytes());
    buf.extend_from_slice(&c.normal_oct16.to_le_bytes());
    buf.extend_from_slice(&c.emissive_rgbe.to_le_bytes());
    buf.extend_from_slice(&c.material_id.to_le_bytes());
    buf.extend_from_slice(&c.reserved[0].to_le_bytes());
    buf.extend_from_slice(&c.reserved[1].to_le_bytes());
    debug_assert_eq!(buf.len(), 32);
    sha256::digest(&buf)
}

/// 32B 冻结面未触核验(逐 closure):`reserved` 预留拓扑字段位必须为零、f0 保留
/// A 通道必须为零、flags 未分配位段必须为零——任一消费即 `Err(FieldOverreach)`
/// (禁静默扩 RED 锚)。
pub fn check_closure_face_untouched(c: &MaterialClosure) -> Result<()> {
    if c.reserved != [0, 0] {
        return Err(SideTableError::FieldOverreach { field: "reserved" });
    }
    if (c.f0_rgba8 >> 24) != 0 {
        return Err(SideTableError::FieldOverreach { field: "f0_reserved_a" });
    }
    if ((c.rough_metal_ao_flags >> 24) as u8) & !FLAGS_ASSIGNED_MASK != 0 {
        return Err(SideTableError::FieldOverreach { field: "flags_unassigned_bits" });
    }
    Ok(())
}

/// 缺省侧表零漂移机核(修订行「缺省侧表 ≡ 无专项 lobe,既有材质输出逐位
/// 不变」):无侧表基线输出 digest 与缺省侧表路径输出 digest 必须逐位相等;
/// **注入缺省侧表输出仍变即 RED**。
pub fn assert_default_table_invariant(baseline_digest: &[u8; 32], default_table_digest: &[u8; 32]) -> Result<()> {
    if baseline_digest != default_table_digest {
        return Err(SideTableError::DefaultSideTableAltersOutput);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::closure::MATERIAL_FLAG_ALPHA_BLEND;

    fn sample_marschner() -> MarschnerParams {
        MarschnerParams {
            base_color: [0.35, 0.22, 0.12],
            shift_r: -0.1,
            shift_tt: -0.05,
            shift_trt: -0.15,
            width_r: 0.1,
            width_tt: 0.08,
            width_trt: 0.14,
            medulla: 0.3,
        }
    }

    //@ spec: RXS-0373
    #[test]
    fn side_table_roundtrip_signature_and_slot_overreach_red() {
        let mut t = MaterialSideTable::new();
        t.insert(0, LobeExtension::Burley(BurleyProfile { falloff_rgb: [0.8, 0.5, 0.3] }), 4).unwrap();
        t.insert(2, LobeExtension::Marschner(sample_marschner()), 4).unwrap();
        let bytes = encode_side_table(&t);
        let back = decode_side_table(&bytes, 4).unwrap();
        assert_eq!(t, back);
        assert_eq!(encode_side_table(&back), bytes);
        let sig = side_table_signature(&t);
        verify_side_table(&t, &sig).unwrap();
        // 槽越权 ⇒ 拒录(RED)。
        assert!(matches!(
            t.insert(4, LobeExtension::Burley(BurleyProfile { falloff_rgb: [0.1, 0.1, 0.1] }), 4),
            Err(SideTableError::UnknownMaterialSlot { slot: 4, table_len: 4 })
        ));
        // 篡改 ⇒ 签名拒录。
        let mut bad = t.clone();
        bad.insert(1, LobeExtension::Burley(BurleyProfile { falloff_rgb: [0.9, 0.9, 0.9] }), 4).unwrap();
        assert!(matches!(verify_side_table(&bad, &sig), Err(SideTableError::AssetTampered { .. })));
    }

    //@ spec: RXS-0372
    #[test]
    fn closure_32b_face_untouched_and_overreach_red() {
        // 冻结面:默认打包 closure reserved/f0 A/flags 未分配位全零 ⇒ 通过。
        let c = MaterialParams::default().pack();
        check_closure_face_untouched(&c).unwrap();
        // 合法 flags 位(alpha blend)⇒ 通过。
        let mut legal = MaterialParams { flags: MATERIAL_FLAG_ALPHA_BLEND, ..Default::default() }.pack();
        check_closure_face_untouched(&legal).unwrap();
        // 预留拓扑字段位消费 ⇒ RED。
        legal.reserved = [1, 0];
        assert!(matches!(
            check_closure_face_untouched(&legal),
            Err(SideTableError::FieldOverreach { field: "reserved" })
        ));
        // flags 未分配位段消费 ⇒ RED。
        let mut bad = MaterialParams::default().pack();
        bad.rough_metal_ao_flags |= 0x0400_0000; // bit2(未分配)置位段
        assert!(matches!(
            check_closure_face_untouched(&bad),
            Err(SideTableError::FieldOverreach { field: "flags_unassigned_bits" })
        ));
        // 32B 尺寸断言(结构性;布局 digest 稳定性)。
        assert_eq!(core::mem::size_of::<MaterialClosure>(), 32);
        assert_eq!(closure_32b_layout_digest(), closure_32b_layout_digest());
    }

    //@ spec: RXS-0373
    #[test]
    fn default_table_invariant_machine_check() {
        let d = sha256::digest(b"baseline");
        assert_default_table_invariant(&d, &d).unwrap();
        let other = sha256::digest(b"altered");
        assert!(matches!(
            assert_default_table_invariant(&d, &other),
            Err(SideTableError::DefaultSideTableAltersOutput)
        ));
    }
}
