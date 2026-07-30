//! clipmap 栈与虚拟地址空间(报告3 §2.1 机制三「clipmap LOD」;RFC-0016 §4.D1)。
//!
//! 方向光不建 CSM,直接以 clipmap VSM 起步(报告3 §3 取舍一):以相机为中心、
//! 半径逐级 ×2 的正交投影级联;每级虚拟地址空间 16K×16K、页 128×128,页表
//! 128×128 项/级。级数可配(UE 默认 6–22 级参照;本 crate 默认 6,测试用 4)。
//!
//! 选级公式(P0 冻结的「阴影空间 LOD」选择规则,报告3 §4 阶段不变量):
//! 按着色点到相机距离 `d` 取最小满足 `d ≤ R_L` 的级 `L`;投影出窗再逐级向粗
//! 级回退(由 [`crate::shadow::vsm`] 的 mark/sample 回退环实现)。

/// 每级虚拟地址空间边长(纹素):16K。
pub const VIRTUAL_TEXELS: u32 = 16384;
/// 页边长(纹素):128×128 页(报告3 §4 阶段不变量,P0 冻结)。
pub const PAGE_TEXELS: u32 = 128;
/// 页表边长(项):16384/128 = 128,即每级 128×128 = 16384 项。
pub const PAGE_TABLE_DIM: u32 = VIRTUAL_TEXELS / PAGE_TEXELS;

/// clipmap 栈配置(报告3 §2.1;RFC-0016 §4.D1)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipmapConfig {
    /// 级数(默认 6;UE 参照 6–22,空级几乎免费)。
    pub levels: u8,
    /// 第 0 级半径(世界单位):第 L 级半径 = base_radius·2^L,窗口边长 2R。
    pub base_radius: f32,
    /// 沿灯方向的深度半范围(世界单位):每级深度区间 =
    /// 相机灯向坐标 ± depth_extent(方向光正交投影的深度裁剪,P0 简化)。
    pub depth_extent: f32,
}

impl Default for ClipmapConfig {
    fn default() -> Self {
        Self {
            levels: 6,
            base_radius: 8.0,
            depth_extent: 64.0,
        }
    }
}

impl ClipmapConfig {
    /// 参数合法性(级数 ≥1,半径/深度范围为正值且有限)。
    pub fn validate(&self) {
        assert!(self.levels >= 1, "clipmap 级数必须 ≥1");
        assert!(
            self.base_radius.is_finite() && self.base_radius > 0.0,
            "基准半径必须为正值"
        );
        assert!(
            self.depth_extent.is_finite() && self.depth_extent > 0.0,
            "深度半范围必须为正值"
        );
    }

    /// 第 `level` 级半径 R_L = R_0·2^L。
    pub fn radius(&self, level: u8) -> f32 {
        debug_assert!(level < self.levels);
        self.base_radius * 2.0f32.powi(i32::from(level))
    }

    /// 第 `level` 级单页世界尺寸 = 2R_L / 128(原点 snap 粒度)。
    pub fn page_world(&self, level: u8) -> f32 {
        2.0 * self.radius(level) / PAGE_TABLE_DIM as f32
    }

    /// 第 `level` 级单纹素世界尺寸 = 2R_L / 16384(级分辨率,误差换算用)。
    pub fn texel_world(&self, level: u8) -> f32 {
        2.0 * self.radius(level) / VIRTUAL_TEXELS as f32
    }

    /// 选级:最小满足 `dist ≤ R_L` 的级;超出最远级半径钳到末级。
    ///
    /// 即 `L = clamp(ceil(log2(dist / R_0)), 0, levels-1)`,`dist ≤ R_0` 取 0 级
    /// (报告3 §2.1「按屏幕像素投影尺寸选级」的 P0 距离近似;RFC-0016 §4.D1)。
    pub fn select_level(&self, dist: f32) -> u8 {
        if dist <= self.base_radius {
            return 0;
        }
        let l = (dist / self.base_radius).log2().ceil() as i32;
        l.clamp(0, i32::from(self.levels) - 1) as u8
    }
}

/// 方向光正交基(右手):`fwd` = 光线传播方向,`right`/`up` 张成阴影平面。
///
/// 灯空间坐标:`x_l = p·right`,`y_l = p·up`,`z_l = p·fwd`;沿灯方向越靠近
/// 光源 `z_l` 越小,深度图保存最小 `z_l`(最近遮挡)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightBasis {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub fwd: [f32; 3],
}

impl LightBasis {
    /// 由光线传播方向构造正交基(确定性:参考上方向按方向选取,防平行退化)。
    pub fn from_direction(dir: [f32; 3]) -> Self {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        assert!(len > 1e-12, "灯方向不得为零向量");
        let fwd = [dir[0] / len, dir[1] / len, dir[2] / len];
        let up_ref = if fwd[2].abs() < 0.99 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let cross = |a: [f32; 3], b: [f32; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let r = cross(up_ref, fwd);
        let rl = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        let right = [r[0] / rl, r[1] / rl, r[2] / rl];
        let up = cross(fwd, right);
        Self { right, up, fwd }
    }

    /// 世界点 → 灯空间坐标 (x_l, y_l, z_l)。
    pub fn to_light(&self, p: [f32; 3]) -> [f32; 3] {
        let dot = |a: [f32; 3]| a[0] * p[0] + a[1] * p[1] + a[2] * p[2];
        [dot(self.right), dot(self.up), dot(self.fwd)]
    }
}

/// 原点 snap:灯平面坐标 → 所在世界页坐标(页粒度,`floor` 语义)。
///
/// 世界页坐标系以固定原点(世界 0 点)为基准,与相机无关——这是 toroidal
/// addressing 的前提:页表槽位 = 世界页坐标 mod 128,窗口平移时未离开窗口的
/// 页槽位与内容保持不变(报告3 §2.1「级联原点切换标脏环形更新带」)。
pub fn world_page_coord(light_plane_coord: f32, page_world: f32) -> i32 {
    (light_plane_coord / page_world).floor() as i32
}

/// 页表槽位:世界页坐标 → 槽位(toroidal wrap,`wp mod 128` ∈ [0,128) )。
pub fn slot_of(world_page: i32) -> u8 {
    world_page.rem_euclid(PAGE_TABLE_DIM as i32) as u8
}

/// 槽位反解:窗口内世界页坐标 = window_min + 槽位相对窗口起点槽位的偏移。
///
/// 窗口宽度恰为 128 页,每个剩余类在窗口内恰出现一次,故反解唯一。
pub fn world_page_of_slot(slot: u8, window_min: i32) -> i32 {
    let base = i32::from(slot) - i32::from(slot_of(window_min));
    window_min + base.rem_euclid(PAGE_TABLE_DIM as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_doubling_and_page_texel_world() {
        let cfg = ClipmapConfig {
            levels: 4,
            base_radius: 8.0,
            depth_extent: 16.0,
        };
        // 半径逐级 ×2:8/16/32/64
        assert!((cfg.radius(0) - 8.0).abs() < 1e-7);
        assert!((cfg.radius(1) - 16.0).abs() < 1e-7);
        assert!((cfg.radius(3) - 64.0).abs() < 1e-7);
        // 页世界尺寸 = 2R/128;纹素 = 2R/16384(级分辨率)
        assert!((cfg.page_world(0) - 0.125).abs() < 1e-7);
        assert!((cfg.texel_world(0) - 16.0 / 16384.0).abs() < 1e-7);
        assert!((cfg.page_world(2) - 0.5).abs() < 1e-7);
        assert_eq!(PAGE_TABLE_DIM, 128);
        assert_eq!(PAGE_TEXELS * PAGE_TABLE_DIM, VIRTUAL_TEXELS);
    }

    #[test]
    fn select_level_by_distance() {
        let cfg = ClipmapConfig {
            levels: 4,
            base_radius: 8.0,
            depth_extent: 16.0,
        };
        assert_eq!(cfg.select_level(0.0), 0);
        assert_eq!(cfg.select_level(8.0), 0);
        assert_eq!(cfg.select_level(8.001), 1);
        assert_eq!(cfg.select_level(16.0), 1);
        assert_eq!(cfg.select_level(17.0), 2);
        assert_eq!(cfg.select_level(33.0), 3);
        // 超出末级半径钳到末级
        assert_eq!(cfg.select_level(1000.0), 3);
    }

    #[test]
    fn basis_orthonormal_deterministic() {
        // 垂直向下的灯:fwd = (0,0,-1)
        let b = LightBasis::from_direction([0.0, 0.0, -1.0]);
        let dot = |a: [f32; 3], c: [f32; 3]| a[0] * c[0] + a[1] * c[1] + a[2] * c[2];
        assert!(dot(b.right, b.up).abs() < 1e-6);
        assert!(dot(b.right, b.fwd).abs() < 1e-6);
        assert!(dot(b.up, b.fwd).abs() < 1e-6);
        assert!((dot(b.fwd, b.fwd) - 1.0).abs() < 1e-6);
        // 灯空间坐标锚定:dir=(0,0,-1) 时 x_l = y_w,y_l = x_w,z_l = -z_w
        let l = b.to_light([2.0, 3.0, 4.0]);
        assert!((l[0] - 3.0).abs() < 1e-6);
        assert!((l[1] - 2.0).abs() < 1e-6);
        assert!((l[2] + 4.0).abs() < 1e-6);
        // 非单位方向自动归一化,同一基
        let b2 = LightBasis::from_direction([0.0, 0.0, -7.5]);
        assert!((b2.fwd[2] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn toroidal_slot_roundtrip() {
        // 槽位 = wp mod 128;窗口内反解唯一
        assert_eq!(slot_of(0), 0);
        assert_eq!(slot_of(127), 127);
        assert_eq!(slot_of(128), 0);
        assert_eq!(slot_of(-1), 127);
        assert_eq!(slot_of(-128), 0);
        // 窗口 [-64, 64):逐槽位反解还原
        for wp in -64i32..64 {
            assert_eq!(world_page_of_slot(slot_of(wp), -64), wp);
        }
        // 窗口平移一页 [-63, 65):离开窗口的 wp=-64 不再反解到自身
        for wp in -63i32..65 {
            assert_eq!(world_page_of_slot(slot_of(wp), -63), wp);
        }
        assert_ne!(world_page_of_slot(slot_of(-64), -63), -64);
    }
}
