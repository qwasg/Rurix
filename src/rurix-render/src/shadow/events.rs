//! VSM 页缓存跨帧事件模型(G8.5a M19;`g8.p0.m19.vsm_page_cache`)。
//!
//! 失效原因闭集与事件 kind 进 side-band 日志,**不**占用页表项保留位
//! `[26..32)`(G5 冻结面 0-byte)。canonical 序列化 = 确定性 JSON 行
//! (帧升序 → 灯 → 级 → 槽位行主序 → kind 名)。

use std::fmt::Write as _;

/// 冻结失效原因闭集(五值;MAP M19 / 设计 §2.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidationReason {
    CasterMoved,
    LightChanged,
    ClipmapScroll,
    NonVirtualCaster,
    Evicted,
}

impl InvalidationReason {
    pub const ALL: [InvalidationReason; 5] = [
        Self::CasterMoved,
        Self::LightChanged,
        Self::ClipmapScroll,
        Self::NonVirtualCaster,
        Self::Evicted,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CasterMoved => "CasterMoved",
            Self::LightChanged => "LightChanged",
            Self::ClipmapScroll => "ClipmapScroll",
            Self::NonVirtualCaster => "NonVirtualCaster",
            Self::Evicted => "Evicted",
        }
    }
}

/// 灯标识(方向光 / local spot 索引)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LightId {
    Directional,
    Local(u8),
}

impl LightId {
    pub fn as_str(self) -> String {
        match self {
            Self::Directional => "directional".to_owned(),
            Self::Local(i) => format!("local:{i}"),
        }
    }
}

/// 页事件 kind。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    MarkHit,
    MarkMiss,
    Alloc,
    Evict,
    Deny,
    Invalidate(InvalidationReason),
    Raster,
    Sample,
}

impl EventKind {
    pub fn as_str(self) -> String {
        match self {
            Self::MarkHit => "MarkHit".to_owned(),
            Self::MarkMiss => "MarkMiss".to_owned(),
            Self::Alloc => "Alloc".to_owned(),
            Self::Evict => "Evict".to_owned(),
            Self::Deny => "Deny".to_owned(),
            Self::Invalidate(r) => format!("Invalidate:{}", r.as_str()),
            Self::Raster => "Raster".to_owned(),
            Self::Sample => "Sample".to_owned(),
        }
    }
}

/// 单条页缓存事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageEvent {
    pub frame: u32,
    pub light: LightId,
    pub level: u8,
    pub slot: (u8, u8),
    pub kind: EventKind,
    pub phys: u16,
}

/// 事件日志(跨帧累积;canonical 导出前排序)。
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    events: Vec<PageEvent>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, e: PageEvent) {
        self.events.push(e);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PageEvent> {
        self.events.iter()
    }

    pub fn events(&self) -> &[PageEvent] {
        &self.events
    }

    /// 确定性排序:帧 → 灯 → 级 → 槽 y → 槽 x → kind 名 → phys。
    pub fn sorted_clone(&self) -> Vec<PageEvent> {
        let mut v = self.events.clone();
        v.sort_by(|a, b| {
            (
                a.frame,
                a.light,
                a.level,
                a.slot.1,
                a.slot.0,
                a.kind.as_str(),
                a.phys,
            )
                .cmp(&(
                    b.frame,
                    b.light,
                    b.level,
                    b.slot.1,
                    b.slot.0,
                    b.kind.as_str(),
                    b.phys,
                ))
        });
        v
    }

    /// canonical JSON 行文本(末行换行;UTF-8)。
    pub fn canonical_json(&self) -> String {
        let mut out = String::new();
        for e in self.sorted_clone() {
            let _ = writeln!(
                out,
                "{{\"frame\":{},\"light\":\"{}\",\"level\":{},\"slot\":[{},{}],\"kind\":\"{}\",\"phys\":{}}}",
                e.frame,
                e.light.as_str(),
                e.level,
                e.slot.0,
                e.slot.1,
                e.kind.as_str(),
                e.phys
            );
        }
        out
    }

    pub fn reasons_present(&self) -> Vec<InvalidationReason> {
        let mut rs = Vec::new();
        for e in &self.events {
            if let EventKind::Invalidate(r) = e.kind {
                if !rs.contains(&r) {
                    rs.push(r);
                }
            }
        }
        rs.sort();
        rs
    }

    pub fn count_kind_on_frame(&self, frame: u32, pred: impl Fn(&EventKind) -> bool) -> usize {
        self.events
            .iter()
            .filter(|e| e.frame == frame && pred(&e.kind))
            .count()
    }

    pub fn count_reason(&self, reason: InvalidationReason) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Invalidate(r) if r == reason))
            .count()
    }

    pub fn has_local_light_kinds(&self) -> bool {
        let mut alloc = false;
        let mut raster = false;
        let mut sample = false;
        for e in &self.events {
            if !matches!(e.light, LightId::Local(_)) {
                continue;
            }
            match e.kind {
                EventKind::Alloc => alloc = true,
                EventKind::Raster => raster = true,
                EventKind::Sample => sample = true,
                _ => {}
            }
        }
        alloc && raster && sample
    }
}

/// 极简 SHA-256(公有域压缩实现;避免为 host 金标准拉第三方依赖)。
pub fn sha256_hex(data: &[u8]) -> String {
    let d = sha256(data);
    let mut s = String::with_capacity(64);
    for b in d {
        let _ = write!(s, "{b:02x}");
    }
    s
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    // FIPS 180-4 压缩实现(常量/轮函数固定)。
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bits = (data.len() as u64).saturating_mul(8);
    let mut buf = data.to_vec();
    buf.push(0x80);
    while (buf.len() % 64) != 56 {
        buf.push(0);
    }
    buf.extend_from_slice(&bits.to_be_bytes());
    debug_assert_eq!(buf.len() % 64, 0);
    let data = buf.as_slice();
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn canonical_sort_stable() {
        let mut log = EventLog::new();
        log.push(PageEvent {
            frame: 1,
            light: LightId::Directional,
            level: 0,
            slot: (2, 0),
            kind: EventKind::Alloc,
            phys: 1,
        });
        log.push(PageEvent {
            frame: 1,
            light: LightId::Directional,
            level: 0,
            slot: (1, 0),
            kind: EventKind::Alloc,
            phys: 0,
        });
        let c = log.canonical_json();
        let lines: Vec<_> = c.lines().collect();
        assert!(lines[0].contains("\"slot\":[1,0]"));
        assert!(lines[1].contains("\"slot\":[2,0]"));
    }
}
