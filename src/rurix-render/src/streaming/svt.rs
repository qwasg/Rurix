//! SVT 稀疏虚拟纹理 host 面（G31+ 波 C Task C13，SVT-1 虚拟纹理页表 + SVT-3
//! 瓦片边界过滤的宿主管理面；RD-041 分项，milestones/g22/g22_svt_gap.json
//! SVT-1/3 行立项窗兑现；TODO #33/#35）。
//!
//! - **虚拟地址空间**（SVT-1）：128K² = [`SVT_VIRTUAL_DIM`]² texel 虚拟纹理，
//!   128² texel 虚拟瓦片 ⇒ 页表 1024×1024（[`SVT_PAGE_TABLE_DIM`]）u32 项
//!   （0 = 未驻留；驻留 = 物理槽号+1）。活动区 = bistro 图集（8192×6144 ⇒
//!   64×48 = 3072 页），页表满尺寸分配、活动区外恒未驻留（采样域限定图集
//!   面，登记面如实标注）。
//! - **瓦片集构建**（SVT-3）：物理瓦片 = 128² + 全边 1 texel border 复制
//!   （130² u32）——border 环按**页所属槽的 REPEAT wrap 律**填（槽内坐标
//!   rem_euclid 回绕；槽 = 图集 2048 网格格，128 整除 2048 ⇒ 页不跨槽）。
//!   双线性 2×2 footprint 页内坐标 ∈ [0,127] ⇒ phys 坐标 ∈ [1,129] 恰被
//!   border 覆盖 ⇒ 单瓦片自足，跨瓦片读取零需求；全驻留时逐 texel 与整图
//!   直采（同 wrap 律）位级同值——边界正确性 = 构建律结构性事实。
//! - **驻留池/容量预算**：固定槽数物理池 + 确定性 LRU（单调触帧时钟全序，
//!   并列取低槽号——驱逐序确定性，streaming/pool.rs 同律）。
//! - **请求-驻留闭环**（SVT-2 宿主半）：device 侧请求缓冲（1 f32/px：0 =
//!   无 miss，page_id+1 = miss 页）→ [`SvtStreaming::consume`] 去重排序
//!   （BTreeSet，确定性）→ 驻留命中触帧/未驻留入池（LRU 驱逐）→ 页表写段
//!   + 瓦片上传段产出（次帧 `FrameUpdate.buffer_uploads` 消费面）。
//!
//! 纪律：host 纯 safe 确定性（全库 `forbid(unsafe_code)`）；禁 HashMap 迭代
//! 序依赖（索引面 BTreeMap/BTreeSet）；IO 量/miss 率只进 evidence 不进硬门。

use std::collections::{BTreeMap, BTreeSet};

use rurix_pkg::sha256;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// SVT 虚拟地址空间边长（texel；128K² = 131072² 虚拟纹理——RD-041 SVT-1 行字面）。
pub const SVT_VIRTUAL_DIM: u32 = 131_072;
/// 虚拟瓦片边长（texel）。
pub const SVT_TILE_DIM: u32 = 128;
/// 物理瓦片 border 环宽（texel；双线性 2×2 footprint 跨页 1 texel 恰覆盖）。
pub const SVT_BORDER: u32 = 1;
/// 物理瓦片边长（= 128 + 2×border）。
pub const SVT_PHYS_DIM: u32 = SVT_TILE_DIM + 2 * SVT_BORDER;
/// 页表边长（页；= 131072 / 128）。
pub const SVT_PAGE_TABLE_DIM: u32 = SVT_VIRTUAL_DIM / SVT_TILE_DIM;
/// 页表项数（1024²）。
pub const SVT_PAGE_COUNT: usize = (SVT_PAGE_TABLE_DIM * SVT_PAGE_TABLE_DIM) as usize;
/// 页表项编码：0 = 未驻留；驻留 = 物理槽号 + 1（小整数域——RX kernel 侧
/// `entry > 0` 判定 + `entry−1` 取槽,禁大整数字面量面）。
pub const SVT_ENTRY_NOT_RESIDENT: u32 = 0;
/// 物理瓦片 texel 数（130²）。
pub const SVT_PHYS_TEXELS: usize = (SVT_PHYS_DIM * SVT_PHYS_DIM) as usize;
/// 物理瓦片字节数（u32 打包 RGBA8）。
pub const SVT_PHYS_TILE_BYTES: usize = SVT_PHYS_TEXELS * 4;

// ---------------------------------------------------------------------------
// 错误面（typed Err，fail-closed）
// ---------------------------------------------------------------------------

/// SVT host 面失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum SvtError {
    /// 非 canonical 构造（附静态原因串）。
    NotCanonical(&'static str),
    /// device 请求缓冲含越界页号（结构性缺陷，fail-closed 不静默）。
    BadRequestPage { page_id: u32 },
    /// 池容量不足（全槽钉住——本面不钉住，防御性保留）。
    PoolFull,
}

impl std::fmt::Display for SvtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvtError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            SvtError::BadRequestPage { page_id } => {
                write!(f, "请求缓冲页号 {page_id} 越界（活动瓦片集外,结构性缺陷）")
            }
            SvtError::PoolFull => write!(f, "物理池满且无未钉住槽可驱逐"),
        }
    }
}

impl std::error::Error for SvtError {}

pub type Result<T> = std::result::Result<T, SvtError>;

// ---------------------------------------------------------------------------
// 槽描述 + 瓦片集（SVT-3 border 复制构建律）
// ---------------------------------------------------------------------------

/// 图集槽（一纹理一槽；origin = 图集 texel 坐标,w/h = 纹理真实尺寸 pow2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvtSlotDesc {
    pub origin_x: u32,
    pub origin_y: u32,
    pub width: u32,
    pub height: u32,
}

/// SVT 瓦片集（活动区全部虚拟页的物理 payload 宿主"盘"面——device 侧驻留
/// 池的读取源；payload 序 = 页号序（py × pages_x + px））。
#[derive(Debug, Clone, PartialEq)]
pub struct SvtTileSet {
    /// 活动区页宽（页）。
    pub pages_x: u32,
    /// 活动区页高（页）。
    pub pages_y: u32,
    /// 全部页 payload（页数 × [`SVT_PHYS_TEXELS`] u32,130² 含 border）。
    pub payloads: Vec<u32>,
    /// payload 字节面 digest（sha256: 前缀；evidence 互核面）。
    pub digest: String,
}

impl SvtTileSet {
    /// 活动区页数。
    pub fn page_total(&self) -> u32 {
        self.pages_x * self.pages_y
    }

    /// 页 payload 切片（130² u32）。
    pub fn page_payload(&self, page_id: u32) -> Result<&[u32]> {
        if page_id >= self.page_total() {
            return Err(SvtError::NotCanonical("page_id 越出活动瓦片集"));
        }
        let b = page_id as usize * SVT_PHYS_TEXELS;
        Ok(&self.payloads[b..b + SVT_PHYS_TEXELS])
    }

    /// payload 字节面（LE；上传/digest 同源）。
    pub fn payloads_bytes(&self) -> Vec<u8> {
        self.payloads
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

/// 瓦片集构建（SVT-3 border 复制律）。
///
/// `atlas` = u32 打包 RGBA8 图集（atlas_w × atlas_h,行主序）；`slots` = 槽表
/// （图集 2048 网格律法面由调用方以 `slot_of` 闭包供给——页内原点图集坐标
/// → 槽号；None = 该页无槽（payload 全 0,活动区外登记面））。
///
/// border 环填充律（SVT-3 机核）：页 (px,py) 的 payload phys (1+dx,1+dy)
/// （dx,dy ∈ [−1,128]）= 槽内坐标 (px·128 + dx − ox, py·128 + dy − oy) 经
/// `rem_euclid(w/h)` 回绕后的槽 texel——**页所属槽的 wrap 律**（非 border
/// 落点图集网格槽的律）⇒ 槽界处 border = 本槽回绕 texel,与生产采样 REPEAT
/// 语义位级同值（整图直采对拍锚）。
pub fn build_tile_set(
    atlas_w: u32,
    atlas_h: u32,
    atlas: &[u32],
    slots: &[SvtSlotDesc],
    slot_of: &dyn Fn(u32, u32) -> Option<usize>,
) -> Result<SvtTileSet> {
    if atlas_w == 0 || atlas_h == 0 || atlas_w % SVT_TILE_DIM != 0 || atlas_h % SVT_TILE_DIM != 0 {
        return Err(SvtError::NotCanonical("图集尺寸为零或非瓦片整倍"));
    }
    if atlas.len() != (atlas_w as usize) * (atlas_h as usize) {
        return Err(SvtError::NotCanonical("图集 texel 数与尺寸不符"));
    }
    for s in slots {
        if s.width == 0
            || s.height == 0
            || !s.width.is_power_of_two()
            || !s.height.is_power_of_two()
        {
            return Err(SvtError::NotCanonical("槽尺寸非 pow2 非零（wrap 精确域限定）"));
        }
        if s.origin_x + s.width > atlas_w || s.origin_y + s.height > atlas_h {
            return Err(SvtError::NotCanonical("槽矩形越出图集"));
        }
    }
    let pages_x = atlas_w / SVT_TILE_DIM;
    let pages_y = atlas_h / SVT_TILE_DIM;
    let mut payloads = vec![0u32; (pages_x * pages_y) as usize * SVT_PHYS_TEXELS];
    let phys = SVT_PHYS_DIM as usize;
    let tile = SVT_TILE_DIM as i64;
    for py in 0..pages_y {
        for px in 0..pages_x {
            let page_id = py * pages_x + px;
            let base = page_id as usize * SVT_PHYS_TEXELS;
            let Some(slot) = slot_of(px * SVT_TILE_DIM, py * SVT_TILE_DIM) else {
                continue; // 无槽页 = 全 0 payload（活动区外登记面）
            };
            let s = slots[slot];
            let (ox, oy) = (s.origin_x as i64, s.origin_y as i64);
            let (w, h) = (s.width as i64, s.height as i64);
            for dy in -1i64..=tile {
                for dx in -1i64..=tile {
                    // 槽内逻辑坐标（页锚定;越界经本槽 wrap 律回绕）。
                    let lx = (px as i64 * tile + dx - ox).rem_euclid(w);
                    let ly = (py as i64 * tile + dy - oy).rem_euclid(h);
                    let texel = atlas[((oy + ly) as usize) * atlas_w as usize + (ox + lx) as usize];
                    payloads[base + ((dy + 1) as usize) * phys + (dx + 1) as usize] = texel;
                }
            }
        }
    }
    let bytes: Vec<u8> = payloads.iter().flat_map(|v| v.to_le_bytes()).collect();
    let digest = format!("sha256:{}", hex_lower(&sha256::digest(&bytes)));
    Ok(SvtTileSet {
        pages_x,
        pages_y,
        payloads,
        digest,
    })
}

/// 槽 fallback 色（miss 合法面 = 低 mip 等效：槽全 texel srgb→linear 均值
/// ×mod——texture_mean_albedo 策略的 SVT miss 面同族;f64 累加确定性）。
/// `linlut` = 256 项 srgb→linear LUT（host 单一事实源预算面）;`mod_rgb` 逐槽
/// 调制（= 生产采样 mod 同值）。返回槽数 × 4 f32（rgb + 0）。
pub fn build_fallback_table(
    atlas_w: u32,
    atlas: &[u32],
    slots: &[SvtSlotDesc],
    mod_rgb: &[[f32; 3]],
    linlut: &[f32; 256],
) -> Result<Vec<f32>> {
    if slots.len() != mod_rgb.len() {
        return Err(SvtError::NotCanonical("槽表与 mod 表长度不符"));
    }
    if atlas.len() != (atlas_w as usize) * (slots.iter().map(|s| s.origin_y + s.height).max().unwrap_or(0) as usize)
    {
        return Err(SvtError::NotCanonical("图集 texel 数与槽面不符"));
    }
    let mut out = Vec::with_capacity(slots.len() * 4);
    for (k, s) in slots.iter().enumerate() {
        let mut acc = [0.0f64; 3];
        for y in 0..s.height {
            for x in 0..s.width {
                let p = atlas[((s.origin_y + y) as usize) * atlas_w as usize
                    + (s.origin_x + x) as usize] as usize;
                acc[0] += f64::from(linlut[p % 256]);
                acc[1] += f64::from(linlut[(p / 256) % 256]);
                acc[2] += f64::from(linlut[(p / 65536) % 256]);
            }
        }
        let n = (s.width as f64) * (s.height as f64);
        out.push((acc[0] / n) as f32 * mod_rgb[k][0]);
        out.push((acc[1] / n) as f32 * mod_rgb[k][1]);
        out.push((acc[2] / n) as f32 * mod_rgb[k][2]);
        out.push(0.0);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 物理驻留池（确定性 LRU）
// ---------------------------------------------------------------------------

/// 物理池槽。
#[derive(Debug, Clone)]
struct PhysSlot {
    /// 驻留页号。
    page_id: u32,
    /// 上次触帧序号（单调时钟全序;驱逐确定性来源）。
    touch_seq: u64,
}

/// 固定槽数物理瓦片池（容量预算语义;确定性 LRU——触帧 = 命中刷新/入池,
/// 驱逐 = 最久未触,并列取低槽号;streaming/pool.rs PagePool 同律形态）。
#[derive(Debug)]
pub struct SvtPool {
    slots: Vec<Option<PhysSlot>>,
    /// page_id → 物理槽号（驻留索引面）。
    index: BTreeMap<u32, usize>,
    clock: u64,
}

impl SvtPool {
    /// 固定槽数池（容量 ≥1）。
    pub fn new(capacity: u32) -> Self {
        assert!(capacity >= 1, "物理池容量至少 1 槽");
        Self {
            slots: vec![None; capacity as usize],
            index: BTreeMap::new(),
            clock: 0,
        }
    }

    pub fn capacity(&self) -> u32 {
        self.slots.len() as u32
    }

    pub fn resident_count(&self) -> u32 {
        self.index.len() as u32
    }

    /// 驻留查询（不触帧）。
    pub fn lookup(&self, page_id: u32) -> Option<u32> {
        self.index.get(&page_id).map(|&s| s as u32)
    }

    fn next_seq(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// 触帧刷新（命中）。
    pub fn touch(&mut self, page_id: u32) -> Option<u32> {
        let &slot = self.index.get(&page_id)?;
        let seq = self.next_seq();
        self.slots[slot].as_mut().expect("index 指向槽必占用").touch_seq = seq;
        Some(slot as u32)
    }

    /// 指定槽入池（页表外源重建面——探针/对拍 rig 的确定性初态构造;
    /// 槽越界/占用冲突/页已驻留即 typed Err,fail-closed）。
    pub fn insert_at(&mut self, page_id: u32, slot: u32) -> Result<()> {
        if slot as usize >= self.slots.len() {
            return Err(SvtError::NotCanonical("insert_at 物理槽越界"));
        }
        if self.index.contains_key(&page_id) {
            return Err(SvtError::NotCanonical("insert_at 页已驻留"));
        }
        if self.slots[slot as usize].is_some() {
            return Err(SvtError::NotCanonical("insert_at 物理槽占用冲突"));
        }
        let seq = self.next_seq();
        self.slots[slot as usize] = Some(PhysSlot {
            page_id,
            touch_seq: seq,
        });
        self.index.insert(page_id, slot as usize);
        Ok(())
    }

    /// 入池：已驻留 = 触帧刷新;否则取最低空闲槽,无空闲驱逐最久未触
    /// （并列取低槽号——驱逐序确定性）。返回 (槽号, 被驱逐页号)。
    pub fn insert(&mut self, page_id: u32) -> (u32, Option<u32>) {
        if let Some(slot) = self.touch(page_id) {
            return (slot, None);
        }
        let slot = match self.slots.iter().position(|s| s.is_none()) {
            Some(i) => i,
            None => {
                // LRU 驱逐：最久未触（touch_seq 最小;并列取低槽号——iter 顺序）。
                let (victim, _) = self
                    .slots
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (i, s.as_ref().expect("满池无空槽").touch_seq))
                    .min_by(|(ia, sa), (ib, sb)| sa.cmp(sb).then(ia.cmp(ib)))
                    .expect("容量 ≥1 必有槽");
                let evicted = self.slots[victim].as_ref().expect("占用").page_id;
                self.index.remove(&evicted);
                self.slots[victim] = None;
                let seq = self.next_seq();
                self.slots[victim] = Some(PhysSlot { page_id, touch_seq: seq });
                self.index.insert(page_id, victim);
                return (victim as u32, Some(evicted));
            }
        };
        let seq = self.next_seq();
        self.slots[slot] = Some(PhysSlot { page_id, touch_seq: seq });
        self.index.insert(page_id, slot);
        (slot as u32, None)
    }
}

// ---------------------------------------------------------------------------
// SVT 流送状态机（页表 + 池 + 请求-驻留闭环;SVT-2 宿主半）
// ---------------------------------------------------------------------------

/// 一帧请求消费产出（次帧 `FrameUpdate.buffer_uploads` 段面 + evidence 计数面）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SvtFramePlan {
    /// 请求缓冲非零项数（= 本帧 miss 像素数;fallback 像素数同值）。
    pub miss_pixels: u32,
    /// 去重后 miss 页数。
    pub unique_pages: u32,
    /// 其中已驻留（触帧刷新;零上传）。
    pub already_resident: u32,
    /// 本帧新入池页数（= 瓦片上传段数）。
    pub loaded: u32,
    /// 本帧驱逐页数。
    pub evicted: u32,
    /// 瓦片上传字节量（loaded × [`SVT_PHYS_TILE_BYTES`]）。
    pub io_bytes: u64,
    /// 页表写段（页表网格序下标, 新项）——含驱逐清 0 与入池驻留写入;
    /// device 页表 SSBO 字节偏移 = 下标×4。
    pub page_table_writes: Vec<(u32, u32)>,
    /// 瓦片上传段（物理槽号, 活动区紧凑序页号;payload 由瓦片集查）。
    pub tile_uploads: Vec<(u32, u32)>,
}

/// SVT 流送状态机（页表 host 影 + 物理池 + 瓦片集"盘"面）。
///
/// 索引空间（单一事实源）：页表项下标 = **128K² 页表网格序**（vpy·1024+vpx,
/// kernel 与 device 页表 SSBO 同序）;瓦片集 payload/池驻留键 = **活动区紧凑
/// 序**（tile = vpy·pages_x+vpx）。活动区 = 页表网格的 pages_x×pages_y 左下
/// 子格,两序经 [`SvtStreaming::table_index`]/[`SvtStreaming::tile_of_table`]
/// 互转（子格外页表项恒未驻留）。
#[derive(Debug)]
pub struct SvtStreaming {
    /// 页表 host 影（[`SVT_PAGE_COUNT`] u32;device 侧页表 SSBO 的同源事实源）。
    page_table: Vec<u32>,
    pool: SvtPool,
    tiles: SvtTileSet,
}

impl SvtStreaming {
    /// 冷启动臂（页表全空/池全空;pool_tiles < 活动页数 = 强制小池压力面）。
    pub fn new_cold(tiles: SvtTileSet, pool_tiles: u32) -> Result<Self> {
        if pool_tiles == 0 {
            return Err(SvtError::NotCanonical("pool_tiles=0"));
        }
        Ok(Self {
            page_table: vec![0u32; SVT_PAGE_COUNT],
            pool: SvtPool::new(pool_tiles),
            tiles,
        })
    }

    /// 全驻留臂（活动页全映射,物理槽号 = 页号;池容量 = 活动页数——
    /// 全驻留参考面,SVT 采样 vs 整图直采位级对拍的锚臂）。
    pub fn new_full(tiles: SvtTileSet) -> Result<Self> {
        let total = tiles.page_total();
        if total == 0 {
            return Err(SvtError::NotCanonical("活动瓦片集为空"));
        }
        let mut s = Self::new_cold(tiles, total)?;
        for tile in 0..total {
            let (slot, evicted) = s.pool.insert(tile);
            debug_assert_eq!(slot, tile, "全驻留初态槽号 = 页号（池全空顺入）");
            debug_assert!(evicted.is_none());
            let t = s.table_index(tile);
            s.page_table[t as usize] = slot + 1;
        }
        Ok(s)
    }

    /// 活动区紧凑序 → 页表网格序（vpy·1024+vpx）。
    pub fn table_index(&self, tile: u32) -> u32 {
        (tile / self.tiles.pages_x) * SVT_PAGE_TABLE_DIM + (tile % self.tiles.pages_x)
    }

    /// 页表网格序 → 活动区紧凑序（子格外 = None——采样域限定面,fail-closed
    /// 由 consume 判 BadRequestPage）。
    pub fn tile_of_table(&self, table_index: u32) -> Option<u32> {
        let vpx = table_index % SVT_PAGE_TABLE_DIM;
        let vpy = table_index / SVT_PAGE_TABLE_DIM;
        if vpx >= self.tiles.pages_x || vpy >= self.tiles.pages_y {
            return None;
        }
        Some(vpy * self.tiles.pages_x + vpx)
    }

    /// 外源页表重建（探针/对拍 rig 确定性初态;页表 → 池驻留一致性核验
    /// fail-closed——驻留项页号越活动区/物理槽越界/槽冲突即 Err）。池容量
    /// = 活动页数（rig 面零驱逐噪声;生产臂走 new_cold/new_full）。
    pub fn from_page_table(tiles: SvtTileSet, page_table: Vec<u32>) -> Result<Self> {
        if page_table.len() != SVT_PAGE_COUNT {
            return Err(SvtError::NotCanonical("页表长度 ≠ 1024²"));
        }
        let total = tiles.page_total();
        let mut pool = SvtPool::new(total.max(1));
        let pages_x = tiles.pages_x;
        let pages_y = tiles.pages_y;
        for (t, &e) in page_table.iter().enumerate() {
            if e == SVT_ENTRY_NOT_RESIDENT {
                continue;
            }
            let t = t as u32;
            let (vpx, vpy) = (t % SVT_PAGE_TABLE_DIM, t / SVT_PAGE_TABLE_DIM);
            if vpx >= pages_x || vpy >= pages_y {
                return Err(SvtError::NotCanonical("页表驻留项页号越出活动区"));
            }
            pool.insert_at(vpy * pages_x + vpx, e - 1)?;
        }
        let _ = total;
        Ok(Self {
            page_table,
            pool,
            tiles,
        })
    }

    pub fn pool_capacity(&self) -> u32 {
        self.pool.capacity()
    }

    pub fn resident_count(&self) -> u32 {
        self.pool.resident_count()
    }

    pub fn page_total(&self) -> u32 {
        self.tiles.page_total()
    }

    pub fn tiles(&self) -> &SvtTileSet {
        &self.tiles
    }

    /// 页表 host 影只读面（evidence digest/初态字节面）。
    pub fn page_table(&self) -> &[u32] {
        &self.page_table
    }

    /// 页表字节面（LE;descs 初态上传同源）。
    pub fn page_table_bytes(&self) -> Vec<u8> {
        self.page_table
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    /// 页表 digest（evidence 互核面）。
    pub fn page_table_digest(&self) -> String {
        format!("sha256:{}", hex_lower(&sha256::digest(&self.page_table_bytes())))
    }

    /// 驻留查询（页号 → 物理槽号）。
    pub fn resident_slot(&self, page_id: u32) -> Option<u32> {
        self.pool.lookup(page_id)
    }

    /// 请求-驻留闭环（SVT-2 宿主半）：device 请求缓冲（1 f32/px;0 = 无
    /// miss,page_id+1 = miss 页）→ 去重排序（BTreeSet 确定性）→ 命中触帧/
    /// 未驻留入池（LRU 驱逐）→ 页表写段 + 瓦片上传段。越界页号 = 结构性
    /// 缺陷 fail-closed（不静默）。
    pub fn consume(&mut self, req: &[f32]) -> Result<SvtFramePlan> {
        let mut plan = SvtFramePlan::default();
        let mut pages: BTreeSet<u32> = BTreeSet::new();
        for &v in req {
            if v == 0.0 {
                continue;
            }
            plan.miss_pixels += 1;
            // 请求编码 = 页表网格序页号+1（kernel 同律）→ 活动区紧凑序互转。
            let t = (v as u32).saturating_sub(1);
            let Some(tile) = self.tile_of_table(t) else {
                return Err(SvtError::BadRequestPage { page_id: t });
            };
            pages.insert(tile);
        }
        plan.unique_pages = pages.len() as u32;
        for tile in pages {
            if self.pool.lookup(tile).is_some() {
                self.pool.touch(tile);
                plan.already_resident += 1;
                continue;
            }
            let (slot, evicted) = self.pool.insert(tile);
            if let Some(ev) = evicted {
                let te = self.table_index(ev);
                self.page_table[te as usize] = SVT_ENTRY_NOT_RESIDENT;
                plan.page_table_writes.push((te, SVT_ENTRY_NOT_RESIDENT));
                plan.evicted += 1;
            }
            let t = self.table_index(tile);
            self.page_table[t as usize] = slot + 1;
            plan.page_table_writes.push((t, slot + 1));
            plan.tile_uploads.push((slot, tile));
            plan.loaded += 1;
            plan.io_bytes += SVT_PHYS_TILE_BYTES as u64;
        }
        Ok(plan)
    }
}

// ---------------------------------------------------------------------------
// 工具面
// ---------------------------------------------------------------------------

/// 小写 hex（digest 字面形态;与 bin 面 sha256_hex 同形态）。
fn hex_lower(d: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成图集:atlas 512×256,两槽 256×256 于 (0,0)/(256,0);texel 值 =
    /// 唯一可溯编码（x + y·4096 + slot·2^24）——border wrap 正确性逐点可判。
    fn synth_atlas() -> (Vec<u32>, Vec<SvtSlotDesc>) {
        let (aw, ah) = (512u32, 256u32);
        let slots = vec![
            SvtSlotDesc { origin_x: 0, origin_y: 0, width: 256, height: 256 },
            SvtSlotDesc { origin_x: 256, origin_y: 0, width: 256, height: 256 },
        ];
        let mut atlas = vec![0u32; (aw * ah) as usize];
        for y in 0..ah {
            for x in 0..aw {
                let slot = if x < 256 { 0u32 } else { 1 };
                atlas[(y * aw + x) as usize] = x + y * 4096 + slot * 16_777_216;
            }
        }
        (atlas, slots)
    }

    fn synth_slot_of(ax: u32, _ay: u32) -> Option<usize> {
        Some(if ax < 256 { 0 } else { 1 })
    }

    /// 瓦片集构建:页数/尺寸/digest 稳定 + border 环 wrap 律逐点（槽界不串扰——
    /// 右界 border = 本槽 x=0 回绕 texel,非邻槽 texel;SVT-3 机核锚）。
    #[test]
    fn tile_set_border_wrap_law() {
        let (atlas, slots) = synth_atlas();
        let ts = build_tile_set(512, 256, &atlas, &slots, &synth_slot_of).unwrap();
        assert_eq!((ts.pages_x, ts.pages_y), (4, 2));
        assert_eq!(ts.payloads.len(), 8 * SVT_PHYS_TEXELS);
        let phys = SVT_PHYS_DIM as usize;
        // 页 0（槽 0 首 128²):内部直拷;左 border（dx=−1）= 槽内 x=255 回绕。
        let p0 = ts.page_payload(0).unwrap();
        assert_eq!(p0[1 * phys + 1], atlas[0]); // (0,0)
        assert_eq!(p0[1 * phys + 0], atlas[255]); // 左 border wrap → x=255
        assert_eq!(p0[0 * phys + 1], atlas[255 * 512]); // 上 border wrap → y=255
        // 页 1（槽 0 次 128²,图集 x ∈ [128,256)):右 border（dx=128）=
        // 槽内 x=256 → 回绕 0（本槽 texel,非邻槽 x=256 的 texel——SVT-3 机核）。
        let p1 = ts.page_payload(1).unwrap();
        let slot0_wrap = atlas[0]; // 槽 0 (0,0)
        let neighbor = atlas[256]; // 图集 (256,0) = 槽 1 (0,0)
        assert_ne!(slot0_wrap, neighbor, "合成面两槽 texel 必异（判据有效前提）");
        assert_eq!(p1[1 * phys + 129], slot0_wrap, "右 border = 本槽 wrap texel");
        assert_eq!(p1[1 * phys + 128], atlas[255]); // 内部末列直拷
        // 页 2（槽 1 首 128²,图集 x ∈ [256,384)):左 border = 槽 1 x=255 回绕。
        let p2 = ts.page_payload(2).unwrap();
        assert_eq!(p2[1 * phys + 0], atlas[256 + 255]);
        assert_eq!(p2[1 * phys + 1], atlas[256]);
        // digest 确定性（同输入同 digest）。
        let ts2 = build_tile_set(512, 256, &atlas, &slots, &synth_slot_of).unwrap();
        assert_eq!(ts.digest, ts2.digest);
    }

    /// fallback 均值:合成槽均值手算互核 + ×mod 施加面。
    #[test]
    fn fallback_mean_linear_times_mod() {
        let (aw, ah) = (256u32, 256u32);
        let mut atlas = vec![0u32; (aw * ah) as usize];
        // 槽 texel:R=255（lut[255] 项）G/B=0 ⇒ 均值 = lut[255]。
        for p in atlas.iter_mut() {
            *p = 255;
        }
        let slots = vec![SvtSlotDesc { origin_x: 0, origin_y: 0, width: 256, height: 256 }];
        let mut lut = [0.0f32; 256];
        for (i, e) in lut.iter_mut().enumerate() {
            *e = i as f32 / 255.0;
        }
        let fb = build_fallback_table(aw, &atlas, &slots, &[[0.5, 1.0, 1.0]], &lut).unwrap();
        assert_eq!(fb.len(), 4);
        assert!((fb[0] - 0.5).abs() < 1e-6, "mean(lut[255])=1 ×mod 0.5");
        assert!(fb[1].abs() < 1e-6 && fb[2].abs() < 1e-6);
        assert_eq!(fb[3], 0.0);
    }

    /// 池 LRU:驱逐序确定性（最久未触,并列低槽号）+ 触帧刷新面。
    #[test]
    fn pool_lru_deterministic() {
        let mut pool = SvtPool::new(2);
        assert_eq!(pool.insert(10).0, 0);
        assert_eq!(pool.insert(20).0, 1);
        pool.touch(10); // 10 刷新 ⇒ 20 成最久未触
        let (slot, evicted) = pool.insert(30);
        assert_eq!((slot, evicted), (1, Some(20)), "LRU 驱逐 20 腾槽 1");
        assert_eq!(pool.lookup(20), None);
        assert_eq!(pool.lookup(30), Some(1));
        // 确定性:同操作序列重放同结果。
        let mut p2 = SvtPool::new(2);
        p2.insert(10);
        p2.insert(20);
        p2.touch(10);
        assert_eq!(p2.insert(30), (1, Some(20)));
    }

    /// 闭环（SVT-2 宿主半）:冷启动 → miss 请求 → 入池/页表写段/IO 账 →
    /// 同请求复消费 = 全命中零上传;小池驱逐面（页表清 0 写段在案）。
    #[test]
    fn streaming_consume_closed_loop() {
        let (atlas, slots) = synth_atlas();
        let ts = build_tile_set(512, 256, &atlas, &slots, &synth_slot_of).unwrap();
        let total = ts.page_total();
        assert_eq!(total, 8);
        // 冷臂 pool=3 < 8:请求 {0,1,2,3} ⇒ 入 0/1/2 后满,3 驱逐 0（插入序最早）。
        let mut s = SvtStreaming::new_cold(ts, 3).unwrap();
        let req: Vec<f32> = [1.0, 2.0, 3.0, 4.0, 1.0, 0.0].into_iter().collect();
        let plan = s.consume(&req).unwrap();
        assert_eq!(plan.miss_pixels, 5);
        assert_eq!(plan.unique_pages, 4);
        assert_eq!(plan.loaded, 4);
        assert_eq!(plan.evicted, 1);
        assert_eq!(plan.already_resident, 0);
        assert_eq!(plan.io_bytes, 4 * SVT_PHYS_TILE_BYTES as u64);
        assert_eq!(plan.tile_uploads.len(), 4);
        assert_eq!(plan.page_table_writes.len(), 5, "1 驱逐清 0 + 4 驻留写入");
        assert!(plan.page_table_writes.contains(&(0, 0)), "驱逐页表清 0 在案");
        assert!(s.resident_slot(0).is_none(), "页 0 被驱逐");
        assert_eq!(s.resident_slot(3), Some(0), "页 3 入驱逐腾出槽 0");
        // 复消费同请求（请求集 4 > 池 3）:升序处理级联驱逐——0 入驱 1,1 入
        // 驱 2,2 入驱 3,3 入驱 0（thrash 如实记账;全 4 页重载零命中）。
        let plan2 = s.consume(&req).unwrap();
        assert_eq!(plan2.already_resident, 0);
        assert_eq!(plan2.loaded, 4);
        assert_eq!(plan2.evicted, 4);
        for ev in 0..4u32 {
            assert!(plan2.page_table_writes.contains(&(ev, 0)), "驱逐页表清 0 在案");
        }
        assert!(s.resident_slot(0).is_none());
        assert!(s.resident_slot(3).is_some());
        // 请求集 ≤ 池容面（驻留 {1,2,3} 子集）:再消费 = 全命中零上传（稳态闭环）。
        let req_small: Vec<f32> = [2.0, 3.0, 4.0].into_iter().collect();
        let plan3 = s.consume(&req_small).unwrap();
        assert_eq!(plan3.already_resident, 3);
        assert_eq!(plan3.loaded, 0);
        assert!(plan3.page_table_writes.is_empty());
        // 越界页号 fail-closed。
        assert!(matches!(
            s.consume(&[99.0]),
            Err(SvtError::BadRequestPage { page_id: 98 })
        ));
        // 确定性双跑:全新同构状态机同请求序列 ⇒ 页表 digest 位级一致。
        let ts2 = build_tile_set(512, 256, &atlas, &slots, &synth_slot_of).unwrap();
        let mut s2 = SvtStreaming::new_cold(ts2, 3).unwrap();
        s2.consume(&req).unwrap();
        s2.consume(&req).unwrap();
        assert_eq!(s.page_table_digest(), s2.page_table_digest());
    }

    /// 全驻留臂:活动页全映射（槽号 = 页号,页表网格序下标 = 活动区紧凑序
    /// 经 table_index 互转——合成面 pages_x=4 ⇒ tile 4..7 映至 1024..1027）
    /// + 复消费零加载 + 页表 digest 稳定。
    #[test]
    fn streaming_full_residency_arm() {
        let (atlas, slots) = synth_atlas();
        let ts = build_tile_set(512, 256, &atlas, &slots, &synth_slot_of).unwrap();
        let mut s = SvtStreaming::new_full(ts).unwrap();
        assert_eq!(s.resident_count(), 8);
        assert_eq!(s.pool_capacity(), 8);
        for p in 0..8u32 {
            assert_eq!(s.resident_slot(p), Some(p));
            let t = s.table_index(p);
            let want = if p < 4 { p } else { 1024 + (p - 4) };
            assert_eq!(t, want, "table_index 网格序映射");
            assert_eq!(s.page_table()[t as usize], p + 1);
            assert_eq!(s.tile_of_table(t), Some(p));
        }
        // 全 8 页请求（网格序编码 = table_index+1）⇒ 全命中零上传。
        let req: Vec<f32> = (0..8u32).map(|p| (s.table_index(p) + 1) as f32).collect();
        let plan = s.consume(&req).unwrap();
        assert_eq!(plan.loaded, 0);
        assert_eq!(plan.already_resident, 8);
        assert!(plan.page_table_writes.is_empty());
        let d1 = s.page_table_digest();
        s.consume(&req).unwrap();
        assert_eq!(s.page_table_digest(), d1);
        // 子格外页号（vpx ≥ pages_x）fail-closed。
        assert!(matches!(
            s.consume(&[9.0]),
            Err(SvtError::BadRequestPage { page_id: 8 })
        ));
    }
}
