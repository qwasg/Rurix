//! 单层 principled 材质闭合的 pack/unpack(报告6 §2.1;RFC-0016 章 G 前半)。
//!
//! 32B 布局冻结在 [`crate::graph::types::MaterialClosure`],本模块只拥有
//! "参数 ↔ 打包位"的编解码口径(冻结文件零改动):
//! - `albedo_rgba8`:albedo RGB + 不透明度 A,各 8 位 unorm(round + clamp),
//!   字节序 R|G|B|A 由低到高;
//! - `f0_rgba8`:F0 RGB 各 8 位 unorm,A 保留写 0;
//! - `rough_metal_ao_flags`:roughness|metalness|ao 各 8 位 unorm,高 8 位为
//!   flags 原样位(语义见 `MATERIAL_FLAG_*`);
//! - `normal_oct16`:法线八面体编码,每轴 16 位 unorm(标准 oct 映射,含负半球
//!   折叠;Meyer 2010 / ryg 口径);
//! - `emissive_rgbe`:自发光 RGBE 共享指数(Radiance .hdr 口径,偏置 128)。
//!
//! 编解码对非物理输入(负值/NaN/超界)确定性收敛:unorm 通道 clamp,NaN 折叠为
//! 0;RGBE 超幅值饱和。往返误差界见模块单测(unorm ≤1/255,法线角误差 ≤0.02 rad,
//! RGBE 最大通道相对误差 ≤1/256)。

use crate::graph::types::MaterialClosure;

/// flags 位:alpha 混合(预测器据此生成 [`super::pso_cache::BlendMode::AlphaBlend`]
/// 变体;报告6 §2.2 特性开关显式列表)。
pub const MATERIAL_FLAG_ALPHA_BLEND: u8 = 1 << 0;
/// flags 位:双面渲染(预测器据此关背面剔除)。
pub const MATERIAL_FLAG_DOUBLE_SIDED: u8 = 1 << 1;

/// RGBE 指数偏置(Radiance .hdr 口径 128):解码比例 f = 2^(E-136)。
///
/// 选择依据:可表上界 ≈ 255·2^119 ≈ 1.7e38,既满足 HDR 范围 ≥ 2^10 的纪律又
/// 不溢出 f32(E=255 时仍 < f32::MAX ≈ 3.4e38);下界 2^-135 落入 f32 次正规,
/// 对自发光无实际影响。相对量化误差 ≤ 1/256(最大通道),远优于 2% 验收。
pub const EMISSIVE_RGBE_EXP_BIAS: i32 = 128;

/// 单层 principled 材质参数(报告6 §2.1:Substrate Slab 输入子集——albedo/F0/
/// roughness/normal/emissive,全部有明确定义域;多层混合归 P3+)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialParams {
    /// 反照率 RGB(定义域 [0,1])。
    pub albedo: [f32; 3],
    /// 不透明度(定义域 [0,1];打包进 albedo 的 A 通道)。
    pub opacity: f32,
    /// 0 度菲涅尔反射率 F0 RGB(电介质典型 0.04;定义域 [0,1])。
    pub f0: [f32; 3],
    /// 粗糙度(定义域 [0,1])。
    pub roughness: f32,
    /// 金属度(定义域 [0,1])。
    pub metalness: f32,
    /// 环境光遮蔽(定义域 [0,1])。
    pub ao: f32,
    /// 法线(任意非零向量,内部归一化;零向量收敛为单位 +Z)。
    pub normal: [f32; 3],
    /// 自发光 RGB(HDR,≥0;RGBE 共享指数编码,上界见 [`EMISSIVE_RGBE_EXP_BIAS`])。
    pub emissive: [f32; 3],
    /// 特性开关位(见 `MATERIAL_FLAG_*`;原样入 `rough_metal_ao_flags` 高 8 位)。
    pub flags: u8,
}

impl Default for MaterialParams {
    /// 默认 = 不透明非金属灰(报告6 §2.1:参数全部有默认值)。
    fn default() -> Self {
        Self {
            albedo: [1.0, 1.0, 1.0],
            opacity: 1.0,
            f0: [0.04, 0.04, 0.04],
            roughness: 0.5,
            metalness: 0.0,
            ao: 1.0,
            normal: [0.0, 0.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            flags: 0,
        }
    }
}

impl MaterialParams {
    /// 打包为冻结布局的 32B 闭合(`material_id` 置 0,由
    /// [`super::table::MaterialTable`] 注册时回填)。
    pub fn pack(&self) -> MaterialClosure {
        MaterialClosure {
            albedo_rgba8: pack_rgba8(self.albedo, self.opacity),
            f0_rgba8: pack_rgba8(self.f0, 0.0),
            rough_metal_ao_flags: quantize_unorm8(self.roughness)
                | (quantize_unorm8(self.metalness) << 8)
                | (quantize_unorm8(self.ao) << 16)
                | (u32::from(self.flags) << 24),
            normal_oct16: pack_normal_oct16(self.normal),
            emissive_rgbe: pack_emissive_rgbe(self.emissive),
            material_id: 0,
            reserved: [0, 0],
        }
    }
}

/// 解包(往返配套;`material_id`/`reserved` 不属于参数面,不回读)。
pub fn unpack(c: &MaterialClosure) -> MaterialParams {
    let (albedo, opacity) = unpack_rgba8(c.albedo_rgba8);
    let (f0, _) = unpack_rgba8(c.f0_rgba8);
    let rmaf = c.rough_metal_ao_flags;
    MaterialParams {
        albedo,
        opacity,
        f0,
        roughness: unorm8(rmaf & 0xFF),
        metalness: unorm8((rmaf >> 8) & 0xFF),
        ao: unorm8((rmaf >> 16) & 0xFF),
        flags: (rmaf >> 24) as u8,
        normal: unpack_normal_oct16(c.normal_oct16),
        emissive: unpack_emissive_rgbe(c.emissive_rgbe),
    }
}

// ---------------------------------------------------------------------------
// unorm8(RGBA8 口径:round + clamp;NaN 折叠为 0——f32::clamp(NaN)=NaN,
// NaN as u32 饱和为 0,确定性收敛)
// ---------------------------------------------------------------------------

fn quantize_unorm8(x: f32) -> u32 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u32
}

fn unorm8(q: u32) -> f32 {
    q as f32 / 255.0
}

fn pack_rgba8(rgb: [f32; 3], a: f32) -> u32 {
    quantize_unorm8(rgb[0])
        | (quantize_unorm8(rgb[1]) << 8)
        | (quantize_unorm8(rgb[2]) << 16)
        | (quantize_unorm8(a) << 24)
}

fn unpack_rgba8(p: u32) -> ([f32; 3], f32) {
    (
        [
            unorm8(p & 0xFF),
            unorm8((p >> 8) & 0xFF),
            unorm8((p >> 16) & 0xFF),
        ],
        unorm8((p >> 24) & 0xFF),
    )
}

// ---------------------------------------------------------------------------
// 法线八面体编码(标准 oct 映射:z<0 负半球折叠到菱形边界;每轴 16 位 unorm,
// 角误差界 ~1e-4 rad 量级,单测以 0.02 rad 验收)
// ---------------------------------------------------------------------------

/// sign(0) 取 +1(避免折叠边界 0·sign 出 NaN/方向翻转)。
fn sign01(v: f32) -> f32 {
    if v >= 0.0 { 1.0 } else { -1.0 }
}

fn quantize_unorm16(x: f32) -> u32 {
    ((x * 0.5 + 0.5).clamp(0.0, 1.0) * 65535.0).round() as u32
}

fn pack_normal_oct16(n: [f32; 3]) -> u32 {
    // L1 归一化把向量投到八面体表面;零向量收敛为 (0,0) → 解码单位 +Z。
    let l1 = (n[0].abs() + n[1].abs() + n[2].abs()).max(f32::MIN_POSITIVE);
    let mut x = n[0] / l1;
    let mut y = n[1] / l1;
    if n[2] < 0.0 {
        let ox = x;
        x = (1.0 - y.abs()) * sign01(ox);
        y = (1.0 - ox.abs()) * sign01(y);
    }
    quantize_unorm16(x) | (quantize_unorm16(y) << 16)
}

fn unpack_normal_oct16(p: u32) -> [f32; 3] {
    let mut x = (p & 0xFFFF) as f32 / 65535.0 * 2.0 - 1.0;
    let mut y = (p >> 16) as f32 / 65535.0 * 2.0 - 1.0;
    let z = 1.0 - x.abs() - y.abs();
    if z < 0.0 {
        let ox = x;
        x = (1.0 - y.abs()) * sign01(ox);
        y = (1.0 - ox.abs()) * sign01(y);
    }
    let len = (x * x + y * y + z * z).sqrt();
    if len < f32::MIN_POSITIVE {
        return [0.0, 0.0, 1.0];
    }
    [x / len, y / len, z / len]
}

// ---------------------------------------------------------------------------
// 自发光 RGBE 共享指数(Radiance .hdr 口径:max 通道对齐到 [0.5,1) 尾数,
// 共享 8 位指数 E = e + 128,解码 f = 2^(E-136))
// ---------------------------------------------------------------------------

/// 2^e 位构造(精确;e 超 f32 范围时饱和,编码侧已钳位不会到达)。
fn exp2i(e: i32) -> f32 {
    if e > 127 {
        f32::INFINITY
    } else if e >= -126 {
        f32::from_bits(((e + 127) as u32) << 23)
    } else if e >= -149 {
        f32::from_bits(1u32 << (e + 149))
    } else {
        0.0
    }
}

fn pack_emissive_rgbe(rgb: [f32; 3]) -> u32 {
    let v = rgb[0].max(rgb[1]).max(rgb[2]);
    // 全 0 / 负 / NaN(f32::max 丢弃 NaN;> 不成立即收敛:≤ 阈值或 NaN)→ 全零编码。
    if v <= 1e-32 || v.is_nan() {
        return 0;
    }
    // ±∞ 显式拦截:INFINITY 经 to_bits() 得 raw_exp=0xFF → e_channel 饱和 255,
    // 逐通道字节钳 255(隐式正确);此处显式短路避免依赖隐式路径,确定性编码。
    if v.is_infinite() {
        return 0xFFFFFFFF; // E=255, R=G=B=255(极端 HDR 饱和)
    }
    // v 必为正规格化浮点(> 1e-32 ≫ 最小正规格化 1.18e-38):
    // v = m·2^(raw-127),m∈[1,2) → 尾数 f=m/2∈[0.5,1),指数 e=raw-126。
    let raw_exp = ((v.to_bits() >> 23) & 0xFF) as i32;
    let e = raw_exp - 126;
    // E 通道饱和于 255(v ≥ 2^127 的极端 HDR;此时逐通道字节随之饱和,确定性)。
    let e_channel = (e + EMISSIVE_RGBE_EXP_BIAS).min(255);
    // 未饱和:scale = 256·f/v = 2^(134-raw_exp)(raw≥21 由 1e-32 门槛保证,不溢出);
    // 饱和:取解码比例 2^(E-136) 的倒数,通道字节钳 255。
    let scale = if e_channel == e + EMISSIVE_RGBE_EXP_BIAS {
        exp2i(134 - raw_exp)
    } else {
        1.0 / exp2i(e_channel - 136)
    };
    let q = |c: f32| -> u32 {
        // max(0.0) 同时钳负值与 NaN(NaN·max 语义取非 NaN 操作数 = 0)。
        (c.max(0.0) * scale).round().min(255.0) as u32
    };
    q(rgb[0]) | (q(rgb[1]) << 8) | (q(rgb[2]) << 16) | ((e_channel as u32) << 24)
}

fn unpack_emissive_rgbe(p: u32) -> [f32; 3] {
    let e = (p >> 24) & 0xFF;
    if e == 0 {
        return [0.0; 3];
    }
    let f = exp2i(e as i32 - 136);
    [
        (p & 0xFF) as f32 * f,
        ((p >> 8) & 0xFF) as f32 * f,
        ((p >> 16) & 0xFF) as f32 * f,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift64* 确定性伪随机(零外部依赖;单测可复现)。
    struct XorShift(u64);

    impl XorShift {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545F4914F6CDD1D)
        }

        /// [0,1) 均匀。
        fn next_f32(&mut self) -> f32 {
            (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
        }

        /// 单位球内拒绝采样得均匀方向。
        fn next_dir(&mut self) -> [f32; 3] {
            loop {
                let v = [
                    self.next_f32() * 2.0 - 1.0,
                    self.next_f32() * 2.0 - 1.0,
                    self.next_f32() * 2.0 - 1.0,
                ];
                let l2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
                if (1e-3..=1.0).contains(&l2) {
                    let l = l2.sqrt();
                    return [v[0] / l, v[1] / l, v[2] / l];
                }
            }
        }
    }

    fn angle(a: [f32; 3], b: [f32; 3]) -> f32 {
        let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        d.acos()
    }

    #[test]
    fn pack_layout_bit_fields() {
        // 位段锚定:round(0.5·255)=round(127.5)=128(f32::round 半进远离零)。
        let p = MaterialParams {
            albedo: [1.0, 0.5, 0.0],
            opacity: 1.0,
            f0: [0.0, 0.5, 1.0],
            roughness: 1.0,
            metalness: 0.5,
            ao: 0.0,
            normal: [0.0, 0.0, 1.0],
            emissive: [0.0; 3],
            flags: 0xA5,
        };
        let c = p.pack();
        assert_eq!(c.albedo_rgba8, 255 | (128 << 8) | (255 << 24));
        assert_eq!(c.f0_rgba8, (128 << 8) | (255 << 16));
        assert_eq!(c.rough_metal_ao_flags, 255 | (128 << 8) | (0xA5u32 << 24));
        assert_eq!(c.normal_oct16, 0x8000 | (0x8000 << 16)); // +Z → oct (0,0) → 32768
        assert_eq!(c.emissive_rgbe, 0);
        assert_eq!(c.material_id, 0);
        assert_eq!(c.reserved, [0, 0]);
        assert_eq!(core::mem::size_of::<MaterialClosure>(), 32);
    }

    #[test]
    fn rgba8_roundtrip_error_bound() {
        // unorm8 往返 ≤ 1/255 + eps(含 opacity/f0/rough/metal/ao 同口径)。
        let mut max_err = 0.0f32;
        let mut rng = XorShift(0x9E3779B97F4A7C15);
        for _ in 0..4096 {
            let p = MaterialParams {
                albedo: [rng.next_f32(), rng.next_f32(), rng.next_f32()],
                opacity: rng.next_f32(),
                f0: [rng.next_f32(), rng.next_f32(), rng.next_f32()],
                roughness: rng.next_f32(),
                metalness: rng.next_f32(),
                ao: rng.next_f32(),
                ..Default::default()
            };
            let u = unpack(&p.pack());
            for k in 0..3 {
                max_err = max_err.max((p.albedo[k] - u.albedo[k]).abs());
                max_err = max_err.max((p.f0[k] - u.f0[k]).abs());
            }
            max_err = max_err
                .max((p.opacity - u.opacity).abs())
                .max((p.roughness - u.roughness).abs())
                .max((p.metalness - u.metalness).abs())
                .max((p.ao - u.ao).abs());
        }
        let bound = 1.0 / 255.0 + 1e-6;
        assert!(max_err <= bound, "unorm8 往返误差 {max_err} 超界 {bound}");
        println!("rgba8_roundtrip_error_bound: max_err = {max_err:.8}");
    }

    #[test]
    fn normal_oct_roundtrip_angular_error() {
        // 1000 随机方向(含负半球)角误差 ≤ 0.02 rad;实测量级 1e-4。
        let mut rng = XorShift(0xD1B54A32D192ED03);
        let mut max_ang = 0.0f32;
        for _ in 0..1000 {
            let n = rng.next_dir();
            let d = unpack_normal_oct16(pack_normal_oct16(n));
            max_ang = max_ang.max(angle(n, d));
        }
        assert!(max_ang <= 0.02, "法线角误差 {max_ang} rad 超界 0.02");
        println!("normal_oct_roundtrip_angular_error: max_ang = {max_ang:.6} rad");
    }

    #[test]
    fn emissive_rgbe_roundtrip() {
        // 0 / 常规 / HDR 大值(≥2^10):最大通道相对误差 ≤1/256,全通道绝对
        // 误差 ≤ max/256;与 max 同量级通道相对误差 ≤2%。
        let cases: [[f32; 3]; 5] = [
            [0.0, 0.0, 0.0],
            [1.0, 0.5, 0.25],
            [1024.0, 512.0, 256.0],
            [5000.0, 4000.0, 3000.0],
            [65504.0, 1.0e-4, 3.25],
        ];
        let mut max_rel = 0.0f32;
        for rgb in cases {
            let d = unpack_emissive_rgbe(pack_emissive_rgbe(rgb));
            let v = rgb[0].max(rgb[1]).max(rgb[2]);
            if v == 0.0 {
                assert_eq!(d, [0.0; 3]);
                continue;
            }
            for k in 0..3 {
                let abs_err = (rgb[k] - d[k]).abs();
                assert!(abs_err <= v / 256.0 + 1e-6, "绝对误差 {abs_err} 超 max/256");
                if rgb[k] >= v / 4.0 {
                    let rel = abs_err / rgb[k];
                    max_rel = max_rel.max(rel);
                    assert!(rel <= 0.02, "相对误差 {rel} 超 2%");
                }
            }
        }
        // 随机 HDR 颜色(通道 ≥ max/4)。共享指数格式的逐通道相对误差界为
        // 1/(512·f·α)(α = 通道/max,f∈[0.5,1) 尾数):α ≥ 1/4 → ≤1/64 < 2%;
        // 远小于 max 的通道相对误差退化是 Radiance RGBE 固有特性,由上面的
        // 绝对误差界 ≤ max/256 约束。
        let mut rng = XorShift(0xB5297A4DB5297A4D);
        for _ in 0..512 {
            let v = rng.next_f32() * 10_000.0 + 1e-3;
            let rgb = [
                v * (0.25 + 0.75 * rng.next_f32()),
                v * (0.25 + 0.75 * rng.next_f32()),
                v * (0.25 + 0.75 * rng.next_f32()),
            ];
            let d = unpack_emissive_rgbe(pack_emissive_rgbe(rgb));
            for k in 0..3 {
                let rel = (rgb[k] - d[k]).abs() / rgb[k];
                max_rel = max_rel.max(rel);
                assert!(rel <= 0.02, "随机 HDR 相对误差 {rel} 超 2%");
            }
        }
        println!("emissive_rgbe_roundtrip: max_rel_err = {max_rel:.6}");
    }

    #[test]
    fn emissive_rgbe_hdr_range() {
        // HDR 范围验收 ≥ 2^10:E=255 可表上界 = 255·2^119 ≥ 1024。
        let max_representable = 255.0 * exp2i(255 - 136);
        assert!(max_representable >= 1024.0);
        assert!(max_representable.is_finite());
        // 2^10 整值精确往返(1024 = 2^10,尾数 128/256·2^3 编码无损)。
        let d = unpack_emissive_rgbe(pack_emissive_rgbe([1024.0; 3]));
        for c in d {
            assert!((c - 1024.0).abs() <= 1024.0 / 256.0);
        }
        println!("emissive_rgbe_hdr_range: max = {max_representable:.3e}");
    }

    #[test]
    fn boundary_values() {
        // 全 0:法线零向量收敛单位 +Z;自发光全零往返精确。
        let z = MaterialParams {
            albedo: [0.0; 3],
            opacity: 0.0,
            f0: [0.0; 3],
            roughness: 0.0,
            metalness: 0.0,
            ao: 0.0,
            normal: [0.0; 3],
            emissive: [0.0; 3],
            flags: 0,
        };
        let uz = unpack(&z.pack());
        assert_eq!(uz.albedo, [0.0; 3]);
        assert!((uz.normal[2] - 1.0).abs() < 1e-6);
        assert_eq!(uz.emissive, [0.0; 3]);
        // 全 1:量化端点精确(255/255 = 1.0 精确可表)。
        let o = MaterialParams {
            albedo: [1.0; 3],
            opacity: 1.0,
            f0: [1.0; 3],
            roughness: 1.0,
            metalness: 1.0,
            ao: 1.0,
            normal: [1.0, 1.0, 1.0],
            emissive: [1.0; 3],
            flags: 0xFF,
        };
        let uo = unpack(&o.pack());
        for k in 0..3 {
            assert_eq!(uo.albedo[k], 1.0);
            assert_eq!(uo.f0[k], 1.0);
        }
        assert_eq!(uo.opacity, 1.0);
        assert_eq!(uo.flags, 0xFF);
        // 负法线 z(负半球折叠):-Z 往返角误差 ≈ 0。
        let nz = unpack_normal_oct16(pack_normal_oct16([0.0, 0.0, -1.0]));
        assert!(angle([0.0, 0.0, -1.0], nz) < 1e-4);
        let nz2 = unpack_normal_oct16(pack_normal_oct16([0.3, -0.4, -0.5]));
        let l = 0.3f32.hypot(0.4).hypot(0.5);
        assert!(angle([0.3 / l, -0.4 / l, -0.5 / l], nz2) < 1e-4);
        // 非物理输入确定性收敛:负值/NaN → 0;超界 → 1。
        let bad = MaterialParams {
            albedo: [f32::NAN, -1.0, 2.0],
            ..Default::default()
        };
        let ub = unpack(&bad.pack());
        assert_eq!(ub.albedo[0], 0.0);
        assert_eq!(ub.albedo[1], 0.0);
        assert_eq!(ub.albedo[2], 1.0);
        assert_eq!(
            unpack_emissive_rgbe(pack_emissive_rgbe([f32::NAN; 3])),
            [0.0; 3]
        );
    }

    #[test]
    fn pack_unpack_roundtrip_params() {
        // 全参数往返:flags 精确,其余在上述各分项界内。
        let p = MaterialParams {
            albedo: [0.8, 0.2, 0.05],
            opacity: 0.75,
            f0: [0.04, 0.04, 0.04],
            roughness: 0.35,
            metalness: 0.9,
            ao: 1.0,
            normal: [0.0, 1.0, 0.0],
            emissive: [2.5, 0.0, 0.5],
            flags: MATERIAL_FLAG_ALPHA_BLEND | MATERIAL_FLAG_DOUBLE_SIDED,
        };
        let u = unpack(&p.pack());
        assert_eq!(u.flags, p.flags);
        for k in 0..3 {
            assert!((p.albedo[k] - u.albedo[k]).abs() <= 1.0 / 255.0 + 1e-6);
            assert!((p.f0[k] - u.f0[k]).abs() <= 1.0 / 255.0 + 1e-6);
        }
        assert!(angle(p.normal, u.normal) <= 0.02);
        assert!((p.emissive[0] - u.emissive[0]).abs() / p.emissive[0] <= 0.02);
        assert_eq!(u.emissive[1], 0.0);
    }
}
