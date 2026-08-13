//! OIT benchmark 测量面（G9.5 M120；RFC-0025 §4.K；spec/display_pipeline.md
//! RXS-0371 L1）。
//!
//! 对七算法 × overdraw 档位阶梯（[`super::scene::BENCHMARK_LAYERS`]）在同场景
//! 同 overdraw 分布下测量：**帧时**（host wall-clock，min-of-reps 口径，抗噪）、
//! **内存**（存储模型 bytes，公式化确定性）、**质量误差**（对排序真值
//! max/mean/超阈像素计数）。**只产测量数据，不做选型判定**（选型面在
//! [`super::selection`],fail-closed NotMeasuredYet）。
//!
//! 确定性面：内存/质量/图像 digest/计数为确定性量（双跑位级一致,measured
//! 冻结）；帧时为 wall-clock measured（冻结带记参考值,判据 = 非零非空,
//! 不作位级比较——wall time 不可位冻,如实登记）。

use super::algorithms::{
    image_digest, quality_error, run_algorithm, sorted_fallback, AlgoOutput, OitAlgorithm,
};
use super::scene::{canonical_scene, OitScene, BENCHMARK_EXTENT, BENCHMARK_LAYERS};

/// 每算法每档位测量重复次数(min 取值,冻结)。
pub const BENCHMARK_REPS: u32 = 5;
/// 质量误差超阈阈值(f32;冻结)。
pub const QUALITY_EPS: f32 = 1e-4;

/// 单算法单档位测量记录。
#[derive(Debug, Clone)]
pub struct Measurement {
    pub algorithm: OitAlgorithm,
    pub overdraw_layers: u32,
    /// 帧时 min-of-reps(ns,wall-clock measured)。
    pub frame_ns_min: u64,
    /// 全部 reps 样本(ns)。
    pub frame_ns_samples: Vec<u64>,
    /// 存储模型 bytes。
    pub storage_bytes: u64,
    /// 辅助面 bytes。
    pub aux_bytes: u64,
    /// 质量误差(对排序真值)。
    pub quality_max_abs: f32,
    pub quality_mean_abs: f64,
    pub quality_pixels_over_eps: u32,
    /// fragment 计数面。
    pub fragments_total: u64,
    pub fragments_kept: u64,
    pub fragments_tail: u64,
    pub fragments_dropped: u64,
    /// 输出图像 digest。
    pub image_digest: [u8; 32],
}

/// benchmark 全量结果(七算法 × 档位)。
#[derive(Debug, Clone)]
pub struct BenchmarkRun {
    pub measurements: Vec<Measurement>,
    /// 每档位排序真值 digest(正确性对照锚)。
    pub truth_digest_per_level: Vec<(u32, [u8; 32])>,
    /// 每档位场景 digest(同场景锚)。
    pub scene_digest_per_level: Vec<(u32, [u8; 32])>,
}

impl BenchmarkRun {
    /// evidence 非空判据面:七算法 × 全档位、每记录帧时非零且内存/质量面在位。
    pub fn is_nonempty(&self) -> bool {
        let expect = OitAlgorithm::ALL.len() * BENCHMARK_LAYERS.len();
        if self.measurements.len() != expect {
            return false;
        }
        self.measurements.iter().all(|m| {
            m.frame_ns_min > 0
                && !m.frame_ns_samples.is_empty()
                && m.storage_bytes > 0
                && m.fragments_total > 0
        }) && OitAlgorithm::ALL.iter().all(|a| {
            BENCHMARK_LAYERS
                .iter()
                .all(|&l| self.find(a, l).is_some())
        })
    }

    pub fn find(&self, algo: &OitAlgorithm, layers: u32) -> Option<&Measurement> {
        self.measurements
            .iter()
            .find(|m| m.algorithm == *algo && m.overdraw_layers == layers)
    }
}

fn measure_one(algo: OitAlgorithm, scene: &OitScene, truth_rgb: &[[f32; 3]]) -> Measurement {
    let mut best: Option<AlgoOutput> = None;
    let mut best_ns = u64::MAX;
    let mut samples = Vec::with_capacity(BENCHMARK_REPS as usize);
    for _ in 0..BENCHMARK_REPS {
        let t0 = std::time::Instant::now();
        let out = run_algorithm(algo, scene);
        let ns = t0.elapsed().as_nanos() as u64;
        samples.push(ns);
        if ns < best_ns {
            best_ns = ns;
            best = Some(out);
        }
    }
    let out = best.expect("reps ≥ 1");
    let (qmax, qmean, qcount) = quality_error(&out.rgb, truth_rgb, QUALITY_EPS);
    Measurement {
        algorithm: algo,
        overdraw_layers: scene.layers,
        frame_ns_min: best_ns,
        frame_ns_samples: samples,
        storage_bytes: out.storage_bytes,
        aux_bytes: out.aux_bytes,
        quality_max_abs: qmax,
        quality_mean_abs: qmean,
        quality_pixels_over_eps: qcount,
        fragments_total: scene.stream.len() as u64,
        fragments_kept: out.fragments_kept,
        fragments_tail: out.fragments_tail,
        fragments_dropped: out.fragments_dropped,
        image_digest: image_digest(&out.rgb),
    }
}

/// 跑全量 benchmark(同场景同 overdraw 分布;真值一次算出,七算法共用)。
pub fn run_benchmark() -> BenchmarkRun {
    let mut measurements = Vec::new();
    let mut truth_digests = Vec::new();
    let mut scene_digests = Vec::new();
    for &layers in BENCHMARK_LAYERS.iter() {
        let scene = canonical_scene(BENCHMARK_EXTENT.0, BENCHMARK_EXTENT.1, layers);
        scene_digests.push((layers, scene.digest()));
        let truth = sorted_fallback(&scene);
        truth_digests.push((layers, image_digest(&truth.rgb)));
        for algo in OitAlgorithm::ALL {
            measurements.push(measure_one(algo, &scene, &truth.rgb));
        }
    }
    BenchmarkRun {
        measurements,
        truth_digest_per_level: truth_digests,
        scene_digest_per_level: scene_digests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0371
    #[test]
    fn benchmark_evidence_nonempty() {
        // 小档快测(单测用 16×16 替代 canonical 128×128 以省时;判据面同构)。
        let scene = canonical_scene(16, 16, 16);
        let truth = sorted_fallback(&scene);
        for algo in OitAlgorithm::ALL {
            let m = measure_one(algo, &scene, &truth.rgb);
            assert!(m.frame_ns_min > 0, "{:?} 帧时非空", algo);
            assert!(m.storage_bytes > 0, "{:?} 内存非空", algo);
            assert!(m.fragments_total > 0);
            assert_eq!(m.frame_ns_samples.len(), BENCHMARK_REPS as usize);
        }
    }
}
