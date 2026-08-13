//! HLOD 运行时互斥切换面(G9.5 M111;RFC-0025 §4.B;spec/world_partition.md
//! RXS-0364 L3/L4 逐条对齐)。
//!
//! //@ spec: RXS-0364
//!
//! 本模块承载 M111 运行时腿(M110 已落离线 Builder `rurix-asset::hlod` +
//! `rxhlod` 工具;本面只消费其产物):
//!
//! - **screen-size 阈值互斥切换**([`HlodRuntime::select`]):逐 resident cell 按
//!   屏幕尺寸(cell 包围球投影面积占比,闭式解析)对切换阈值表
//!   ([`ScreenSizeThresholds`],层数为烘焙属性)选择 HLOD 层 vs 原始 cell 全量
//!   内容——二者**互斥**(同帧同 cell 只出一种内容,结构性断言全真)。
//! - **运行时零合并断言(RED 锚)**:HLOD 资产全部来自离线烘焙,运行时不得合并/
//!   重建几何——运行时合并/简化调用尝试一律 fail-closed typed
//!   `Err(HlodError::RuntimeMergeForbidden)`(RED 臂独立有效)。
//! - **M110 cell 事件总线接线**:层级状态随 cell 驻留状态切换
//!   ([`HlodRuntime::apply_cell_events`] 消费 `CellLoadBegin/CellResident/
//!   CellUnloadBegin/CellEvicted` 四事件闭集,事件总线只出不反向查询);
//!   运行时对 cell 元数据 HLOD 引用做产物 digest 核验臂(双构建 hash 相等的
//!   运行时消费面复用——引用 digest ≠ 实载产物 digest 即 typed Err)。
//! - **层级序列 golden**:同一视距序列产出确定性层级序列(digest 冻结对照;
//!   层级序列扰动即分叉,RED 臂)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M111 运行时语义面 = 选择/切换/零合并断言,GPU 非必需;
//! `RURIX_REQUIRE_REAL=1` 下以 host 确定性为准。G8/G9 底座(M04 页/M110 事件
//! 总线)只消费不重定,字面 0-byte。

use rurix_pkg::sha256;

use super::partition::{
    CellEvent, CellEventKind, CellHlodRef, PartitionError, PersistentWorld,
};

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed;本文件严禁 UB)
// ---------------------------------------------------------------------------

/// HLOD 运行时失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum HlodError {
    /// 运行时合并/重建几何调用尝试(RXS-0364 L3 RED 锚:HLOD 资产来自离线烘焙,
    /// 运行时零合并——合并注入即 RED)。
    RuntimeMergeForbidden { op: &'static str },
    /// cell 无 HLOD 层级引用却请求 HLOD 层(资产缺引,fail-closed)。
    MissingHlodRef { cell: u32 },
    /// 请求层级越界(≥ 烘焙层数)。
    LevelOutOfRange { cell: u32, level: u32, levels: u32 },
    /// 运行时产物 digest 与 cell 元数据引用不符(双构建 hash 相等运行时核验臂)。
    DigestMismatch { cell: u32 },
    /// 切换阈值表非法(非有限/非严格单调降/覆盖不全)。
    BadThresholds(&'static str),
    /// 层级状态机乱序(对未驻留 cell 选择/重复驻留等,消费面误用)。
    LevelStateOutOfOrder { cell: u32, why: &'static str },
    /// 事件流本身非法(透传 M110 状态机校验面)。
    EventLog(PartitionError),
}

impl std::fmt::Display for HlodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HlodError::RuntimeMergeForbidden { op } => {
                write!(f, "运行时零合并断言: {op} 调用尝试即拒绝(RED)")
            }
            HlodError::MissingHlodRef { cell } => write!(f, "cell {cell} 无 HLOD 层级引用"),
            HlodError::LevelOutOfRange { cell, level, levels } => {
                write!(f, "cell {cell} 请求 level {level} 越界(烘焙层数 {levels})")
            }
            HlodError::DigestMismatch { cell } => {
                write!(f, "cell {cell} HLOD 产物 digest 与元数据引用不符")
            }
            HlodError::BadThresholds(why) => write!(f, "切换阈值表非法: {why}"),
            HlodError::LevelStateOutOfOrder { cell, why } => {
                write!(f, "cell {cell} 层级状态机乱序: {why}")
            }
            HlodError::EventLog(e) => write!(f, "cell 事件流非法: {e}"),
        }
    }
}

impl std::error::Error for HlodError {}

pub type Result<T> = std::result::Result<T, HlodError>;

// ---------------------------------------------------------------------------
// screen-size 阈值表与选择语义
// ---------------------------------------------------------------------------

/// screen-size 切换阈值表(RFC-0025 §4.B「按 screen-size 阈值互斥切换;层数为烘
/// 焙属性」):`thresholds[i]` = 选择 level i 的最小屏幕尺寸占比(包围球投影
/// 面积占比,见 [`screen_size_fraction`]);**有限非负、严格单调降**(近大远小:
/// level 0 阈值最大 = 全量内容仅近距可达,level 升则阈值降)。
///
/// 选择律(互斥闭式):取**首个**满足 `screen_size >= thresholds[i]` 的 i——
/// i=0 ⇒ [`SelectedContent::Full`](原始 cell 全量内容);i>0 ⇒
/// [`SelectedContent::Hlod`] 代理层;无一满足(尺寸小于全部阈值)⇒ 距相机过远
/// 不绘制 ⇒ [`SelectedContent::Culled`]。无 HLOD 引用的 cell 恒
/// [`SelectedContent::Full`]。
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenSizeThresholds {
    /// 逐层阈值(长度 = 烘焙层数;有限非负严格降)。
    pub thresholds: Vec<f64>,
}

impl ScreenSizeThresholds {
    /// 构造并核验(非空、≤8 层、有限非负、严格单调降)。
    pub fn new(thresholds: Vec<f64>) -> Result<Self> {
        if thresholds.is_empty() {
            return Err(HlodError::BadThresholds("空阈值表"));
        }
        if thresholds.len() > 8 {
            return Err(HlodError::BadThresholds("阈值数超 HLOD 层数上界 8"));
        }
        for (i, &t) in thresholds.iter().enumerate() {
            if !t.is_finite() || t <= 0.0 {
                return Err(HlodError::BadThresholds("阈值必须有限正"));
            }
            if i > 0 && thresholds[i - 1].partial_cmp(&t) != Some(std::cmp::Ordering::Greater) {
                return Err(HlodError::BadThresholds("阈值必须严格单调降"));
            }
        }
        Ok(Self { thresholds })
    }

    /// canonical 切换距离表(cell 包围球半径 `radius_m` ⇒ 逐层切换距离 golden
    /// 事实源):screen_size(d) = (r/d)² ⇒ d = r/√s;阈值严格降 ⇒ 距离严格升
    /// (近界→远界)。
    pub fn switch_distances_m(&self, bound_radius_m: f64) -> Vec<f64> {
        self.thresholds
            .iter()
            .map(|s| bound_radius_m / s.sqrt())
            .collect()
    }
}

/// 逐 cell 选择结果(互斥三态闭集)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedContent {
    /// 原始 cell 全量内容(无 HLOD 引用,或 screen-size 达 level 0 阈值)。
    Full,
    /// HLOD 代理层(离线烘焙产物;level ∈ 1..levels)。
    Hlod { level: u32 },
    /// 距相机过远不绘制(屏幕尺寸低于全部有限阈值)。
    Culled,
}

impl SelectedContent {
    pub fn as_str(&self) -> String {
        match self {
            SelectedContent::Full => "full".to_string(),
            SelectedContent::Hlod { level } => format!("hlod_l{level}"),
            SelectedContent::Culled => "culled".to_string(),
        }
    }

    fn code(&self) -> u8 {
        match self {
            SelectedContent::Full => 0,
            SelectedContent::Hlod { .. } => 1,
            SelectedContent::Culled => 2,
        }
    }
}

/// cell 包围球屏幕尺寸占比(闭式解析:正交等效投影,球投影圆盘面积 /
/// 视口面积;`distance_m` 为相机到 cell 包围球心距离;返回 ∈ [0, ∞),
/// 调用方以阈值表裁 [0,1] 语义)。d ≤ r ⇒ 占比 +∞(球含相机,恒选 level 0)。
pub fn screen_size_fraction(bound_radius_m: f64, distance_m: f64) -> f64 {
    if distance_m <= bound_radius_m {
        return f64::INFINITY;
    }
    (bound_radius_m * bound_radius_m) / (distance_m * distance_m)
}

// ---------------------------------------------------------------------------
// 层级序列 golden 编码
// ---------------------------------------------------------------------------

/// 单帧单 cell 选择记录(帧号 + cell + 结果;层级序列 golden 最小单元)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRecord {
    pub frame: u32,
    pub cell: u32,
    pub content: SelectedContent,
}

/// 层级序列 canonical 编码(frame u32 ‖ cell u32 ‖ kind u8 ‖ level u32,LE)。
pub fn encode_selection_log(records: &[SelectionRecord]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(records.len() * 13);
    for r in records {
        buf.extend_from_slice(&r.frame.to_le_bytes());
        buf.extend_from_slice(&r.cell.to_le_bytes());
        buf.push(r.content.code());
        let level = match r.content {
            SelectedContent::Hlod { level } => level,
            _ => 0,
        };
        buf.extend_from_slice(&level.to_le_bytes());
    }
    buf
}

/// 层级序列 digest(同一视距序列产出确定性层级序列的 golden 对照事实源)。
pub fn selection_log_digest(records: &[SelectionRecord]) -> [u8; 32] {
    sha256::digest(&encode_selection_log(records))
}

// ---------------------------------------------------------------------------
// HLOD 运行时(消费 M110 事件总线;零合并断言;digest 核验臂)
// ---------------------------------------------------------------------------

/// HLOD 运行时:逐 resident cell 的层级选择状态机。
///
/// - 驻留集由 M110 cell 事件总线驱动([`Self::apply_cell_events`];增量 drain
///   消费——本结构内置持久化逐 cell 生命周期状态机(与
///   `partition::validate_event_log` 同一转移闭集),逐批校验乱序即 typed Err,
///   帧号跨批单调性持续追踪);
/// - 选择只作用于驻留 cell(未驻留选择 ⇒ typed Err,消费面误用 fail-closed);
/// - **零合并**:本结构没有任何合并/重建入口;[`Self::request_runtime_merge`]
///   为唯一显式探测口,任何调用一律 `Err(RuntimeMergeForbidden)`。
pub struct HlodRuntime {
    resident: std::collections::BTreeSet<u32>,
    /// 逐 cell 生命周期相位(增量事件校验持久态:0=Absent 1=Loading 2=Active
    /// 3=Unloading;与 M110 `validate_event_log` 同一闭集)。
    cell_phase: std::collections::HashMap<u32, u8>,
    last_event_frame: Option<u32>,
    /// 逐 cell 实载 HLOD 产物 digest(装载登记面;与元数据引用核验用)。
    loaded_digests: std::collections::BTreeMap<u32, [u8; 32]>,
    records: Vec<SelectionRecord>,
}

impl HlodRuntime {
    pub fn new() -> Self {
        Self {
            resident: std::collections::BTreeSet::new(),
            cell_phase: std::collections::HashMap::new(),
            last_event_frame: None,
            loaded_digests: std::collections::BTreeMap::new(),
            records: Vec::new(),
        }
    }

    pub fn resident(&self) -> &std::collections::BTreeSet<u32> {
        &self.resident
    }

    pub fn records(&self) -> &[SelectionRecord] {
        &self.records
    }

    /// 清空选择记录(harness 双跑位级一致对照用)。
    pub fn clear_records(&mut self) {
        self.records.clear();
    }

    /// M110 事件总线消费面(增量 drain):逐事件经持久化状态机校验(四事件闭集
    /// 转移 + 帧号跨批单调不减),乱序/回退即 typed Err;校验通过后迁移驻留集
    /// (只出不反向查询分区状态)。
    pub fn apply_cell_events(&mut self, events: &[CellEvent]) -> Result<()> {
        for e in events {
            if let Some(prev) = self.last_event_frame
                && e.frame < prev
            {
                return Err(HlodError::EventLog(PartitionError::FrameNonMonotonic {
                    prev,
                    got: e.frame,
                }));
            }
            self.last_event_frame = Some(e.frame);
            let p = self.cell_phase.entry(e.cell).or_insert(0);
            let ok = match (*p, e.kind) {
                (0, CellEventKind::CellLoadBegin) => Some(1),
                (1, CellEventKind::CellResident) => Some(2),
                (2, CellEventKind::CellUnloadBegin) => Some(3),
                (3, CellEventKind::CellEvicted) => Some(0),
                _ => None,
            };
            match ok {
                Some(next) => *p = next,
                None => {
                    let expected = match *p {
                        0 => "CellLoadBegin",
                        1 => "CellResident",
                        2 => "CellUnloadBegin",
                        _ => "CellEvicted",
                    };
                    return Err(HlodError::EventLog(PartitionError::EventOutOfOrder {
                        cell: e.cell,
                        expected,
                        got: e.kind,
                    }));
                }
            }
            match e.kind {
                CellEventKind::CellResident => {
                    self.resident.insert(e.cell);
                }
                CellEventKind::CellEvicted => {
                    self.resident.remove(&e.cell);
                    self.loaded_digests.remove(&e.cell);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 产物装载登记(运行时核验臂事实源):cell 驻留后登记其实载 HLOD 产物
    /// digest;与元数据 `CellHlodRef.digest` 不符即 `DigestMismatch`(M110 烘焙
    /// 工具产物双构建 hash 相等的运行时消费面——实载产物必须等于资产声明的
    /// 那一构建)。
    pub fn register_loaded_asset(
        &mut self,
        cell: u32,
        meta: &CellHlodRef,
        actual_digest: [u8; 32],
    ) -> Result<()> {
        if !self.resident.contains(&cell) {
            return Err(HlodError::LevelStateOutOfOrder {
                cell,
                why: "对未驻留 cell 登记产物",
            });
        }
        if actual_digest != meta.digest {
            return Err(HlodError::DigestMismatch { cell });
        }
        self.loaded_digests.insert(cell, actual_digest);
        Ok(())
    }

    /// screen-size 阈值互斥选择(逐 cell):无 HLOD 引用 ⇒ 恒 Full;有引用 ⇒
    /// 按阈值表取可达最大层(互斥:同帧同 cell 只出一种内容);请求结果与阈值
    /// 表/烘焙层数不一致即 typed Err。
    pub fn select(
        &mut self,
        world: &PersistentWorld,
        cell: u32,
        distance_m: f64,
        thresholds: &ScreenSizeThresholds,
        frame: u32,
    ) -> Result<SelectedContent> {
        if !self.resident.contains(&cell) {
            return Err(HlodError::LevelStateOutOfOrder {
                cell,
                why: "对未驻留 cell 选择层级",
            });
        }
        if !distance_m.is_finite() || distance_m < 0.0 {
            return Err(HlodError::BadThresholds("视距非有限或为负"));
        }
        let meta = world
            .cells
            .get(cell as usize)
            .ok_or(HlodError::LevelStateOutOfOrder {
                cell,
                why: "cell 下标越界",
            })?;
        let content = match &meta.hlod {
            None => SelectedContent::Full,
            Some(h) => {
                if thresholds.thresholds.len() != h.levels as usize {
                    return Err(HlodError::LevelOutOfRange {
                        cell,
                        level: thresholds.thresholds.len() as u32,
                        levels: h.levels,
                    });
                }
                // 包围球半径 = 包围盒对角线之半(2D cell,z 为资产属性)。
                let dx = (meta.bounds_max[0] - meta.bounds_min[0]) as f64;
                let dy = (meta.bounds_max[1] - meta.bounds_min[1]) as f64;
                let dz = (meta.bounds_max[2] - meta.bounds_min[2]) as f64;
                let radius = 0.5 * (dx * dx + dy * dy + dz * dz).sqrt();
                let s = screen_size_fraction(radius, distance_m);
                let mut chosen: Option<u32> = None;
                for (i, t) in thresholds.thresholds.iter().enumerate() {
                    if s >= *t {
                        chosen = Some(i as u32);
                        break;
                    }
                }
                match chosen {
                    Some(0) => SelectedContent::Full,
                    Some(level) => SelectedContent::Hlod { level },
                    None => SelectedContent::Culled,
                }
            }
        };
        self.records.push(SelectionRecord {
            frame,
            cell,
            content,
        });
        Ok(content)
    }

    /// 互斥结构性断言(机器核验面):同一帧同一 cell 在记录中只出现一次,且
    /// Full/Hlod 互斥(同帧同 cell 同时出现两种内容即违反——本断言全真是
    /// 「互斥切换」判据的机核臂)。
    pub fn assert_mutually_exclusive(&self) -> Result<()> {
        let mut seen: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
        for r in &self.records {
            if !seen.insert((r.frame, r.cell)) {
                return Err(HlodError::LevelStateOutOfOrder {
                    cell: r.cell,
                    why: "同帧同 cell 重复选择(互斥违反)",
                });
            }
        }
        Ok(())
    }

    /// **运行时零合并断言(RXS-0364 L3 RED 锚)**:HLOD 资产全部来自离线烘焙,
    /// 运行时不得合并/重建/简化几何——本探测口为唯一入口,任何调用一律
    /// fail-closed typed Err;返回值永为 Err(函数存在的意义 = RED 臂可调用、
    /// 必被拒)。
    pub fn request_runtime_merge(
        &self,
        op: &'static str,
        _cells: &[u32],
    ) -> Result<()> {
        Err(HlodError::RuntimeMergeForbidden { op })
    }
}

impl Default for HlodRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// canonical 场景(harness 与单测同一事实源;measured 冻结,禁手写 golden)
// ---------------------------------------------------------------------------

/// canonical golden 切换阈值表(2 层:[0.05, 0.005];对应 M110 canonical_world
/// HLOD 引用 levels=2——level 0 = 全量,level 1 = 代理;层数为烘焙属性)。
/// 冻结常量——golden/单测同一口径。
pub fn canonical_thresholds() -> ScreenSizeThresholds {
    ScreenSizeThresholds::new(vec![0.05, 0.005]).expect("canonical 阈值表合法")
}

/// canonical 视距序列(32 帧:相机由近及远匀速拉远再推近,闭式整数推进,
/// 位级确定;序列对同一 resident cell 集合逐帧选择 ⇒ 层级序列 golden)。
pub fn canonical_distance_path(frames: u32) -> Vec<f64> {
    (0..frames)
        .map(|f| {
            let t = f as f64;
            if t < 16.0 {
                40.0 + t * 55.0
            } else {
                40.0 + (31.0 - t) * 55.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::partition::{
        CellEvent, CellEventKind, PartitionError, PartitionRuntime, canonical_budget,
        canonical_camera_path, canonical_world,
    };

    fn resident_world_and_runtime(frames: u32) -> (PersistentWorld, HlodRuntime) {
        let world = canonical_world();
        let mut prt = PartitionRuntime::new(world.clone(), canonical_budget()).unwrap();
        let mut rt = HlodRuntime::new();
        let path = canonical_camera_path(frames);
        for (f, s) in path.iter().enumerate() {
            prt.tick(f as u32, std::slice::from_ref(s)).unwrap();
            rt.apply_cell_events(&prt.drain_events()).unwrap();
        }
        (world, rt)
    }

    /// RXS-0364 L3:screen-size 互斥切换——同一视距序列产出确定性层级序列
    /// (双跑逐位一致);互斥断言全真;层级随距离单调变化(远 ⇒ 更高 level 或
    /// Culled)。
    #[test]
    //@ spec: RXS-0364
    fn screen_size_switch_deterministic_and_monotonic() {
        let (world, mut rt) = resident_world_and_runtime(8);
        let thresholds = canonical_thresholds();
        let hlod_cell = (0..world.cells.len() as u32)
            .find(|&c| rt.resident().contains(&c) && world.cells[c as usize].hlod.is_some())
            .expect("canonical 场景有驻留 HLOD cell");
        let run = |rt: &mut HlodRuntime| -> Vec<SelectionRecord> {
            for (f, d) in canonical_distance_path(32).iter().enumerate() {
                rt.select(&world, hlod_cell, *d, &thresholds, f as u32).unwrap();
            }
            rt.records().to_vec()
        };
        let a = run(&mut rt);
        rt.records.clear();
        let b = run(&mut rt);
        assert_eq!(a, b, "同一视距序列产出确定性层级序列");
        assert_eq!(selection_log_digest(&a), selection_log_digest(&b));
        assert!(rt.assert_mutually_exclusive().is_ok());
        // 单调性:距离升则层级不降(Full→Hlod→Culled 序)。
        let rank = |c: &SelectedContent| match c {
            SelectedContent::Full => 0u8,
            SelectedContent::Hlod { level } => 1 + (*level as u8).min(6),
            SelectedContent::Culled => 9,
        };
        let first_half: Vec<u8> = a[..16].iter().map(|r| rank(&r.content)).collect();
        for w in first_half.windows(2) {
            assert!(w[1] >= w[0], "拉远段层级单调不降: {first_half:?}");
        }
        // 序列必须真的发生切换(近 Full、远 Hlod/Culled)。
        assert!(a.iter().any(|r| r.content == SelectedContent::Full));
        assert!(a
            .iter()
            .any(|r| matches!(r.content, SelectedContent::Hlod { .. } | SelectedContent::Culled)));
    }

    /// RXS-0364 L3(RED 锚):运行时合并调用尝试即断言拒绝——简化/合批/重建
    /// 三探测臂全部 typed Err;sabotage 探针(选择/事件面)照常工作(能红证明:
    /// 拒绝键于合并调用本身而非运行时全拒)。
    #[test]
    //@ spec: RXS-0364
    fn runtime_merge_forbidden_red() {
        let rt = HlodRuntime::new();
        for op in ["merge", "simplify", "rebuild"] {
            match rt.request_runtime_merge(op, &[0, 1]) {
                Err(HlodError::RuntimeMergeForbidden { op: got }) => assert_eq!(got, op),
                other => panic!("运行时合并未被拒绝: {other:?}"),
            }
        }
        // sabotage:正常事件流 + 选择面不被误拒。
        let (world, mut ok_rt) = resident_world_and_runtime(4);
        let cell = *ok_rt.resident().iter().next().unwrap();
        ok_rt
            .select(&world, cell, 100.0, &canonical_thresholds(), 0)
            .expect("正常选择不得被零合并断言误伤");
    }

    /// RXS-0364 L3:cell 事件总线接线——驻留状态随事件切换;未驻留 cell 选择
    /// fail-closed;乱序事件流注入必拒;帧号回退必拒。
    #[test]
    //@ spec: RXS-0364
    fn cell_event_bus_wiring() {
        let mut rt = HlodRuntime::new();
        let ev = |f, c, k| CellEvent {
            frame: f,
            cell: c,
            kind: k,
        };
        rt.apply_cell_events(&[
            ev(0, 7, CellEventKind::CellLoadBegin),
            ev(0, 7, CellEventKind::CellResident),
        ])
        .unwrap();
        assert!(rt.resident().contains(&7));
        rt.apply_cell_events(&[
            ev(1, 7, CellEventKind::CellUnloadBegin),
            ev(1, 7, CellEventKind::CellEvicted),
        ])
        .unwrap();
        assert!(!rt.resident().contains(&7));
        // 未驻留选择 ⇒ typed Err。
        let world = canonical_world();
        assert!(matches!(
            rt.select(&world, 7, 100.0, &canonical_thresholds(), 2),
            Err(HlodError::LevelStateOutOfOrder { cell: 7, .. })
        ));
        // 乱序事件流 ⇒ typed Err(Resident 先于 LoadBegin)。
        assert!(matches!(
            rt.apply_cell_events(&[ev(2, 9, CellEventKind::CellResident)]),
            Err(HlodError::EventLog(_))
        ));
        // 帧号回退 ⇒ typed Err(跨批单调性持续追踪)。
        let mut rt2 = HlodRuntime::new();
        rt2.apply_cell_events(&[ev(5, 1, CellEventKind::CellLoadBegin)])
            .unwrap();
        assert!(matches!(
            rt2.apply_cell_events(&[ev(3, 2, CellEventKind::CellLoadBegin)]),
            Err(HlodError::EventLog(PartitionError::FrameNonMonotonic { .. }))
        ));
    }

    /// RXS-0364 L2 运行时核验臂:实载产物 digest 与元数据引用一致 ⇒ 登记成功;
    /// 篡改 digest(≠ 双构建产物)⇒ DigestMismatch fail-closed。
    #[test]
    //@ spec: RXS-0364
    fn double_build_digest_runtime_verification() {
        let (world, mut rt) = resident_world_and_runtime(4);
        let cell = rt
            .resident()
            .iter()
            .copied()
            .find(|&c| world.cells[c as usize].hlod.is_some())
            .expect("驻留 HLOD cell");
        let meta = world.cells[cell as usize].hlod.unwrap();
        rt.register_loaded_asset(cell, &meta, meta.digest).unwrap();
        let mut forged = meta.digest;
        forged[0] ^= 0x5a;
        assert!(matches!(
            rt.register_loaded_asset(cell, &meta, forged),
            Err(HlodError::DigestMismatch { .. })
        ));
    }

    /// RXS-0364 L3/L4:阈值表合法性 fail-closed + 切换距离表闭式换算
    /// (阈值严格降 ⇒ 距离严格升)。
    #[test]
    //@ spec: RXS-0364
    fn thresholds_fail_closed_and_switch_distance_table() {
        assert!(ScreenSizeThresholds::new(vec![]).is_err());
        assert!(ScreenSizeThresholds::new(vec![0.5, 0.5]).is_err()); // 非严格降
        assert!(ScreenSizeThresholds::new(vec![0.25, 0.5]).is_err()); // 逆序
        assert!(ScreenSizeThresholds::new(vec![f64::INFINITY, 0.5]).is_err()); // 非有限
        assert!(ScreenSizeThresholds::new(vec![f64::NAN]).is_err());
        assert!(ScreenSizeThresholds::new(vec![0.5, 0.0]).is_err()); // 非正
        assert!(ScreenSizeThresholds::new(vec![0.5, 0.25]).is_ok());
        let t = canonical_thresholds();
        let d = t.switch_distances_m(45.2548);
        assert_eq!(d.len(), 2);
        assert!(d[0] > 0.0 && d[1] > d[0], "切换距离随层级严格升: {d:?}");
    }
}
