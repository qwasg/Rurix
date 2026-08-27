//! G35-5 碰撞与力场 device probe harness(门 g35.wave5.collision;
//! RFC-0049 §4.7;G35_CONTRACT §4 契约;g35_particle_core_device 三态/RED
//! 臂同模)。
//!
//! ## 集成路径
//!
//! - **ray_query 臂**(生产档):`kernels/g35_sim_collide.rx`(AccelStruct
//!   首形参)经 `rurix_rt::vk::run_ray_query_effects` 逐帧 dispatch——
//!   **同帧语义**:第 k 帧以当帧障碍变换合成三角汤重建场景(单 BLAS ×
//!   单 identity 实例,tris SSBO 与 BLAS 逐字节同源 = 命中面镜像),粒子
//!   第 k 帧即响应当帧位移(对照注释:Niagara GPU RT 碰撞读上一帧末加速
//!   结构,异步一帧延迟)。
//! - **depth_buffer 臂**(对照教育臂):`kernels/g35_sim_collide_depth.rx`
//!   经 `vk::run_compute` 原位派发;深度图 = host 逐帧对当帧场景
//!   [`synth_topdown_depth`] 合成上传(同帧语义同律)。
//! - **off 臂**:同 depth kernel,params res = 0(kernel 头注冻结的显式
//!   off 档:纯力场 + 积分)。
//! - host 平行金标准 = `particles/collision.rs`(apply_fields +
//!   collide_step〔TriBvh〕/ depth_collide_step)逐帧推进对拍。
//!
//! ## 显式降级链(F12;fail-closed 禁静默换臂)
//!
//! `--collision ray_query|depth_buffer|off` 三档 CLI 闭集:
//! - 闭集外取值 → typed 错误 `E_G35_COLLISION_UNKNOWN_ARM` 退 2;
//! - ray_query 档 TLAS 能力不可用(run_ray_query_effects 能力链 Err,或
//!   `--force-no-tlas` 注入演示)→ typed 错误
//!   `E_G35_COLLISION_NO_TLAS_CAPABILITY` 退 3——**绝不静默降级到
//!   depth_buffer/off**;
//! - 全局 Vulkan loader 缺失 → `skipped_dev_env` JSON 退 0(三态之 SKIP,
//!   非 fake pass;RURIX_REQUIRE_REAL=1 下 SKIP→硬红由 smoke 层裁决)。
//!
//! ## 确定性脚本(冻结夹具)
//!
//! 场景 = 地板两三角(y=0,±8)+ 45° 斜面两三角(x∈[−3,−1])+ 可动方块
//! 8 三角(顶/底/±x 面;半宽 (0.75,0.5,0.75));方块中心第 [`MOVE_FRAME`]
//! 帧自 (5,0.55,0) 突移至 (0,0.55,0)(= 同帧见证);dt = 1/60;力场
//! gravity −9.8 / wind (0.3,0,0.1) / drag 0.05;e/mu_t = collision.rs 冻结
//! 缺省。粒子:前 [`SENTINEL_COUNT`] 枚哨兵粒子(初高解析定值,使其恰在
//! 突移帧线段跨越方块顶面 y=1.05——见证非空转的机器保证),其余随机带
//! 初始化(`rand_table(seed)` 消费律同 mod.rs)。
//!
//! ## 判据面
//!
//! ① 7 f32 流(pos_x/y/z、vel_x/y/z、age)device↔host 逐帧 max abs diff
//! 聚合全帧 p100——probe 只输出 measured,阈值判读归 smoke
//! (milestones/g35/g35_budget.json `g35.collision.parity_p100` 标定腿,
//! threshold = measured×2.0 程序产;ray query t 值 RT core vs host 有 ULP
//! 级差,g34 先例,容差协议正为此设);② flags 整数流 memcmp 位级 +
//! hit 流失配计数诚实登记(hit 命中判在 t 边界对 ULP 敏感,失配计入
//! 诊断,宏观失配必然放大进 ①);③ 同帧见证(host 三腿:gold vs
//! static 首异帧 == MOVE_FRAME;late〔k 帧查 k−1 帧场景 = Niagara 延迟
//! 模型〕在突移帧与 static 位级一致;device 侧方块顶命中登记
//! `1 + committed_primitive_index ∈ 方块三角段` 于突移帧非零);④ 力场
//! 语义(host 解析:wind_x/z > 0 ⇒ 漂移为正;drag ⇒ 速率低于无阻尼对照);
//! ⑤ device 双跑位级(digest = 7 f32 流 ‖ flags ‖ hit 逐帧 sha256 链式);
//! ⑥ frame_ms 登记(逐帧 device 派发墙钟均值;run_ray_query_effects /
//! run_compute 每帧重建 instance/device/AS,会话开销如实计入,
//! measured_local 登记语义非帧率对标)。
//!
//! ## NoContraction(g35_particle_core_device 同律 bin-local 复制;SPV
//! 文件 0-byte 不动)
//!
//! 两 kernel 均含 f32 乘加链,装载期注入 NoContraction 禁驱动 FMA 收缩
//! ⇒ 与 host 逐 op IEEE 对齐(容差收敛手段非判据替代)。
//!
//! ## RED 臂
//!
//! `--red-arm tamper-e`:device 绿链(冻结 e)+ device 红链(e×1.5 篡改
//! 注入 params,host 金标准仍冻结 e)——digest 必异 + 红链对拍 measured
//! 必须溢出容差(smoke 层复核 red_f32 > threshold)。
//!
//! ## 用法
//!
//! ```text
//! g35_collision_device --spv-collide <p> --spv-collide-depth <p>
//!     [--collision ray_query|depth_buffer|off] [--frames 64] [--cap 512]
//!     [--seed 42] [--evidence-out <path>] [--report-max-diff|--calibrate]
//! g35_collision_device --red-arm tamper-e --spv-collide <p> --spv-collide-depth <p>
//! g35_collision_device --collision ray_query --force-no-tlas ...   # typed 退 3
//! g35_collision_device --host-only [--frames N] [--cap N] [--seed N]
//! ```

#![forbid(unsafe_code)]

use std::time::Instant;

use rurix_render::particles::collision::{
    apply_fields, collide_step, depth_collide_step, synth_topdown_depth, CollisionParams,
    DepthGrid, FieldParams,
};
use rurix_render::particles::core::ParticlePools;
use rurix_render::particles::{rand_table, RAND_K, RAND_TABLE_LEN, SEG};
use rurix_render::rt::bvh::TriBvh;
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "[g35_collision_device]";
const DEFAULT_FRAMES: usize = 64;
const DEFAULT_CAP: usize = 512;
const DEFAULT_SEED: u64 = 42;
/// dt = 1/60(冻结确定性脚本)。
const DT: f32 = 1.0 / 60.0;
/// 方块突移帧(同帧见证锚;frames > MOVE_FRAME 时见证适用)。
const MOVE_FRAME: usize = 32;
/// 哨兵粒子数(槽位 0..SENTINEL_COUNT;初高解析定值保证突移帧方块顶命中)。
const SENTINEL_COUNT: usize = 4;
/// 方块三角在场景三角汤中的下标段 [4, 12)(地板 0/1 + 斜面 2/3 + 方块 4..12)。
const BOX_TRI_LO: u32 = 4;
const BOX_TRI_HI: u32 = 12;
/// typed 错误码(降级链 fail-closed 面;smoke fallback_chain_explicit 消费)。
const E_NO_TLAS: &str = "E_G35_COLLISION_NO_TLAS_CAPABILITY";
const E_UNKNOWN_ARM: &str = "E_G35_COLLISION_UNKNOWN_ARM";
/// RED 臂篡改系数(e_red = e × 1.5 = 0.75;冻结注入形)。
const TAMPER_E_FACTOR: f32 = 1.5;
/// f32 对拍流名(pos/vel/age;life 为只读常量输入不入对拍,flags/hit 整数面)。
const F32_STREAMS: [&str; 7] = ["pos_x", "pos_y", "pos_z", "vel_x", "vel_y", "vel_z", "age"];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 力场冻结夹具(头注「确定性脚本」;方向语义 = collision.rs 头注对齐面)。
fn fields() -> FieldParams {
    FieldParams {
        gravity_y: -9.8,
        wind: [0.3, 0.0, 0.1],
        drag: 0.05,
    }
}

/// 碰撞响应冻结缺省(e = 0.5,mu_t = 0.8;collision.rs 冻结常量)。
fn collision_params() -> CollisionParams {
    CollisionParams::default()
}

/// 深度对照臂网格冻结夹具(覆盖 [−8,8]²,128×128 @ 0.125)。
fn depth_grid() -> DepthGrid {
    DepthGrid {
        x0: -8.0,
        z0: -8.0,
        cell: 0.125,
        res: 128,
    }
}

/// 方块中心脚本(第 MOVE_FRAME 帧突移 = 同帧见证)。
fn box_center(frame: usize) -> [f32; 3] {
    if frame < MOVE_FRAME {
        [5.0, 0.55, 0.0]
    } else {
        [0.0, 0.55, 0.0]
    }
}

/// 场景三角汤(12 三角 × 9 f32;顺序冻结:地板 t0/t1 + 斜面 t2/t3 + 方块
/// t4..t11〔顶/底/±x 面〕——BOX_TRI_LO/HI 段判据与本序绑定;tris SSBO 与
/// BLAS 输入逐字节同源 = 命中面镜像)。
fn scene_tris(bc: [f32; 3]) -> Vec<f32> {
    let mut v: Vec<[f32; 3]> = vec![
        // 地板 t0/t1(y = 0,±8)。
        [-8.0, 0.0, -8.0],
        [8.0, 0.0, -8.0],
        [8.0, 0.0, 8.0],
        [-8.0, 0.0, -8.0],
        [8.0, 0.0, 8.0],
        [-8.0, 0.0, 8.0],
        // 斜面 t2/t3(A(−3,2,−2) B(−1,0,−2) C(−1,0,2) D(−3,2,2))。
        [-3.0, 2.0, -2.0],
        [-1.0, 0.0, -2.0],
        [-1.0, 0.0, 2.0],
        [-3.0, 2.0, -2.0],
        [-1.0, 0.0, 2.0],
        [-3.0, 2.0, 2.0],
    ];
    let (hx, hy, hz) = (0.75f32, 0.5f32, 0.75f32);
    let (x0, x1) = (bc[0] - hx, bc[0] + hx);
    let (y0, y1) = (bc[1] - hy, bc[1] + hy);
    let (z0, z1) = (bc[2] - hz, bc[2] + hz);
    // 方块 8 三角:顶(t4/t5)/底(t6/t7)/+x(t8/t9)/−x(t10/t11)。
    v.extend_from_slice(&[
        [x0, y1, z0], [x1, y1, z0], [x1, y1, z1],
        [x0, y1, z0], [x1, y1, z1], [x0, y1, z1],
        [x0, y0, z0], [x1, y0, z1], [x1, y0, z0],
        [x0, y0, z0], [x0, y0, z1], [x1, y0, z1],
        [x1, y0, z0], [x1, y1, z1], [x1, y1, z0],
        [x1, y0, z0], [x1, y0, z1], [x1, y1, z1],
        [x0, y0, z0], [x0, y1, z0], [x0, y1, z1],
        [x0, y0, z0], [x0, y1, z1], [x0, y0, z1],
    ]);
    v.into_iter().flatten().collect()
}

/// 三角汤 → host TriBvh(索引 = 平铺序;当帧重建 = 同帧语义 host 承载)。
fn build_bvh(tris: &[f32]) -> TriBvh {
    let verts: Vec<[f32; 3]> = tris
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    let indices: Vec<[u32; 3]> = (0..verts.len() as u32 / 3)
        .map(|t| [t * 3, t * 3 + 1, t * 3 + 2])
        .collect();
    TriBvh::build(&verts, &indices)
}

/// 哨兵初高解析解:力场下悬停粒子(初速 0)的逐帧跌落量与位置无关
/// (力场 = 常量加速度 + 线性阻尼),故 y0 = 方块顶面 1.05 + 第
/// MOVE_FRAME 帧跌落区间中点 ⇒ 该帧线段必跨越顶面(见证非空转机器保证)。
/// 返回 (y0, x_drift, z_drift):漂移量用于把哨兵终点钉在方块足印内。
fn sentinel_solution() -> (f32, f32, f32) {
    let f = fields();
    let (mut vx, mut vy, mut vz) = (0.0f32, 0.0f32, 0.0f32);
    let (mut dx, mut dy, mut dz) = (0.0f32, 0.0f32, 0.0f32);
    let mut drop_before = 0.0f32;
    let k = 1.0 - f.drag * DT;
    for frame in 0..=MOVE_FRAME {
        if frame == MOVE_FRAME {
            drop_before = dy;
        }
        vx = (vx + f.wind[0] * DT) * k;
        vy = (vy + (f.gravity_y + f.wind[1]) * DT) * k;
        vz = (vz + f.wind[2] * DT) * k;
        dx += vx * DT;
        dy += vy * DT;
        dz += vz * DT;
    }
    let drop_after = dy;
    // dy 为负(向下);y0 使 y(MOVE_FRAME−1) > 1.05 > y(MOVE_FRAME)。
    let y0 = 1.05 - (drop_before + drop_after) * 0.5;
    (y0, dx, dz)
}

/// 冻结粒子池初始化(哨兵 + 随机带;probe 夹具,与 collision.rs 单测夹具
/// 互不镜像)。
fn init_pool(seed: u64, cap: usize) -> ParticlePools {
    let table = rand_table(seed);
    let mut p = ParticlePools::with_capacity(cap);
    let (sy, sdx, sdz) = sentinel_solution();
    for j in 0..cap {
        if j < SENTINEL_COUNT {
            // 哨兵:x 錯位于方块足印内(扣除风漂移),y 解析定高,零初速。
            p.pos_x[j] = -0.45 + 0.3 * j as f32 - sdx;
            p.pos_y[j] = sy;
            p.pos_z[j] = 0.0 - sdz;
            p.vel_x[j] = 0.0;
            p.vel_y[j] = 0.0;
            p.vel_z[j] = 0.0;
            p.life[j] = 100.0;
        } else {
            let r = |k: usize| table[(j * RAND_K + k) % RAND_TABLE_LEN];
            p.pos_x[j] = (r(0) * 2.0 - 1.0) * 2.5;
            p.pos_y[j] = 3.0 + (r(1) * 2.0 - 1.0) * 0.4;
            p.pos_z[j] = (r(2) * 2.0 - 1.0) * 1.5;
            p.vel_x[j] = (r(3) * 2.0 - 1.0) * 0.5;
            p.vel_y[j] = -2.0 + (r(4) * 2.0 - 1.0) * 0.5;
            p.vel_z[j] = (r(5) * 2.0 - 1.0) * 0.5;
            p.life[j] = 5.0 + 5.0 * r(6);
        }
        p.age[j] = 0.0;
        p.pid[j] = j as u32;
    }
    p.n = cap;
    p
}

// ---------------------------------------------------------------------------
// 字节工具 + NoContraction(g35_particle_core_device 先例字面)
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// SPIR-V NoContraction 后处理(bin-local 同律复制自
/// g35_particle_core_device.rs / g14_3_lane_body.rs;SPV 文件 0-byte 不动):
/// 对全部 OpFAdd/OpFSub/OpFMul 结果 id 注入 `OpDecorate %id NoContraction`,
/// 禁驱动 mul+add FMA 收缩——GPU 浮点序列与 host 严格 IEEE 逐 op 对齐。
fn spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize; // SPIR-V header 5 字
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界(NoContraction 注入)");
        }
        match op {
            71 if first_decorate.is_none() => first_decorate = Some(i),
            19..=39 if first_type.is_none() => first_type = Some(i),
            129 | 131 | 133 => result_ids.push(spv[i + 2]),
            _ => {}
        }
        i += wc;
    }
    let at = first_decorate
        .or(first_type)
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚(NoContraction 注入)"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push(71u32 | (3 << 16));
        out.push(*id);
        out.push(42);
    }
    out.extend_from_slice(&spv[at..]);
    out
}

// ---------------------------------------------------------------------------
// JSON 出报(手写零新依赖;g35_particle_core_device 同模)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn strs_json(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", inner.join(","))
}

fn base_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// 出报(stdout 恒打;--evidence-out 同步落盘)。
fn emit_evidence(line: &str, out: &Option<String>) {
    println!("{line}");
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --evidence-out {path}: {e}")));
    }
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

/// 降级链三档(F12 CLI 闭集;闭集外 typed 退 2)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    RayQuery,
    DepthBuffer,
    Off,
}

impl Arm {
    fn name(self) -> &'static str {
        match self {
            Arm::RayQuery => "ray_query",
            Arm::DepthBuffer => "depth_buffer",
            Arm::Off => "off",
        }
    }
}

struct Args {
    spv_collide: Option<String>,
    spv_collide_depth: Option<String>,
    collision: Arm,
    frames: usize,
    cap: usize,
    seed: u64,
    evidence_out: Option<String>,
    red_arm: Option<String>,
    host_only: bool,
    report_max_diff: bool,
    force_no_tlas: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_collide: None,
        spv_collide_depth: None,
        collision: Arm::RayQuery,
        frames: DEFAULT_FRAMES,
        cap: DEFAULT_CAP,
        seed: DEFAULT_SEED,
        evidence_out: None,
        red_arm: None,
        host_only: false,
        report_max_diff: false,
        force_no_tlas: false,
    };
    let mut it = std::env::args().skip(1);
    let next_or = |it: &mut dyn Iterator<Item = String>, k: &str| {
        it.next().unwrap_or_else(|| fail(&format!("{k} 缺值")))
    };
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-collide" => a.spv_collide = it.next(),
            "--spv-collide-depth" => a.spv_collide_depth = it.next(),
            "--collision" => {
                let v = next_or(&mut it, "--collision");
                a.collision = match v.as_str() {
                    "ray_query" => Arm::RayQuery,
                    "depth_buffer" => Arm::DepthBuffer,
                    "off" => Arm::Off,
                    other => {
                        // F12 CLI 闭集:闭集外 typed 错误退 2(fail-closed)。
                        eprintln!(
                            "{TAG}: {E_UNKNOWN_ARM} --collision {other} 不在闭集 \
                             ray_query|depth_buffer|off(显式降级链禁静默换臂)"
                        );
                        std::process::exit(2)
                    }
                };
            }
            "--frames" => {
                a.frames = next_or(&mut it, "--frames")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--frames 非法: {e}")));
            }
            "--cap" => {
                a.cap = next_or(&mut it, "--cap")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--cap 非法: {e}")));
            }
            "--seed" => {
                a.seed = next_or(&mut it, "--seed")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--seed 非法: {e}")));
            }
            "--evidence-out" => a.evidence_out = it.next(),
            "--red-arm" => a.red_arm = it.next(),
            "--host-only" => a.host_only = true,
            "--report-max-diff" | "--calibrate" => a.report_max_diff = true,
            "--force-no-tlas" => a.force_no_tlas = true,
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.frames == 0 {
        fail("--frames 必须 ≥ 1");
    }
    if a.cap == 0 || a.cap % SEG != 0 {
        fail(&format!("--cap 必须为 SEG={SEG} 正整倍数(得 {})", a.cap));
    }
    a
}

/// typed fail-closed 退出(降级链:ray_query 档无 TLAS 能力,禁静默换臂)。
fn typed_no_tlas_exit(reason: &str, evidence_out: &Option<String>) -> ! {
    let line = format!(
        "{{\"schema\":\"rurix.g35.collision_probe.v1\",\"state\":\"typed_error\",\
         \"typed_error\":{},\"collision\":\"ray_query\",\"reason\":{},\
         \"note\":\"fail-closed:ray_query 档 TLAS 能力不可用即 typed 退 3,禁静默降级 depth_buffer/off\"}}",
        jstr(E_NO_TLAS),
        jstr(reason),
    );
    emit_evidence(&line, evidence_out);
    eprintln!("{TAG}: {E_NO_TLAS} {reason}");
    std::process::exit(3)
}

// ---------------------------------------------------------------------------
// host 侧腿(见证三腿 + 力场语义解析判)
// ---------------------------------------------------------------------------

/// 7 f32 流位级快照(见证判用;to_bits——−0/NaN 面弱于 ==,g27 harness 同律)。
fn pool_bits(p: &ParticlePools) -> Vec<u32> {
    let mut out = Vec::with_capacity(p.n * 7 + 1);
    out.push(p.n as u32);
    for i in 0..p.n {
        for v in [
            p.pos_x[i], p.pos_y[i], p.pos_z[i], p.vel_x[i], p.vel_y[i], p.vel_z[i], p.age[i],
        ] {
            out.push(v.to_bits());
        }
    }
    out
}

/// host ray query 链单跑(box_of 定每帧障碍;返回逐帧快照 + 逐帧方块命中数)。
fn host_witness_run(
    box_of: &dyn Fn(usize) -> [f32; 3],
    seed: u64,
    frames: usize,
    cap: usize,
) -> (Vec<Vec<u32>>, Vec<u32>) {
    let f = fields();
    let cp = collision_params();
    let mut p = init_pool(seed, cap);
    let mut snaps = Vec::with_capacity(frames);
    let mut box_hits = Vec::with_capacity(frames);
    for k in 0..frames {
        let bvh = build_bvh(&scene_tris(box_of(k)));
        apply_fields(&mut p, &f, DT);
        let out = collide_step(&mut p, &bvh, &cp, DT);
        let hits = out
            .hit
            .iter()
            .filter(|&&h| h >= 1 + BOX_TRI_LO && h < 1 + BOX_TRI_HI)
            .count() as u32;
        snaps.push(pool_bits(&p));
        box_hits.push(hits);
    }
    (snaps, box_hits)
}

/// 见证三腿判(gold = 当帧场景;static = 障碍恒帧 0 位;late = k 帧查
/// k−1 帧场景 = Niagara 一帧延迟模型)。
struct Witness {
    applicable: bool,
    host_div_static_frame: i64,
    late_same_at_move_frame: bool,
    gold_late_differ_at_move_frame: bool,
    box_hits_move_frame_host: u32,
}

fn run_witness(seed: u64, frames: usize, cap: usize) -> Witness {
    if frames <= MOVE_FRAME {
        return Witness {
            applicable: false,
            host_div_static_frame: -1,
            late_same_at_move_frame: false,
            gold_late_differ_at_move_frame: false,
            box_hits_move_frame_host: 0,
        };
    }
    let (gold, gold_hits) = host_witness_run(&box_center, seed, frames, cap);
    let (stat, _) = host_witness_run(&|_| box_center(0), seed, frames, cap);
    let (late, _) = host_witness_run(&|k| box_center(k.saturating_sub(1)), seed, frames, cap);
    let div = (0..frames).find(|&k| gold[k] != stat[k]).map_or(-1, |k| k as i64);
    Witness {
        applicable: true,
        host_div_static_frame: div,
        late_same_at_move_frame: late[MOVE_FRAME] == stat[MOVE_FRAME],
        gold_late_differ_at_move_frame: gold[MOVE_FRAME] != late[MOVE_FRAME],
        box_hits_move_frame_host: gold_hits[MOVE_FRAME],
    }
}

/// 力场语义解析判(碰撞关断的纯力场腿;collision.rs 头注方向约定):
/// wind_x/z > 0 ⇒ 末均值漂移为正;drag ⇒ 末均速低于无阻尼对照。
fn fields_sanity(seed: u64, cap: usize) -> (bool, bool, bool) {
    let f = fields();
    let run = |drag: f32| -> ParticlePools {
        let mut p = init_pool(seed, cap);
        let fp = FieldParams { drag, ..f };
        let grid = DepthGrid {
            x0: 0.0,
            z0: 0.0,
            cell: 1.0,
            res: 0,
        };
        let cp = collision_params();
        for _ in 0..60 {
            apply_fields(&mut p, &fp, DT);
            depth_collide_step(&mut p, &grid, &[], &cp, DT);
        }
        p
    };
    let with_drag = run(f.drag);
    let no_drag = run(0.0);
    let base = init_pool(seed, cap);
    let mean = |v: &[f32], n: usize| v[..n].iter().sum::<f32>() / n as f32;
    let speed = |p: &ParticlePools| {
        (0..p.n)
            .map(|i| {
                (p.vel_x[i] * p.vel_x[i] + p.vel_y[i] * p.vel_y[i] + p.vel_z[i] * p.vel_z[i])
                    .sqrt() as f64
            })
            .sum::<f64>()
            / p.n as f64
    };
    let wind_dx = mean(&with_drag.vel_x, with_drag.n) > mean(&base.vel_x, base.n);
    let wind_dz = mean(&with_drag.vel_z, with_drag.n) > mean(&base.vel_z, base.n);
    let drag_decay = speed(&with_drag) < speed(&no_drag);
    (wind_dx, wind_dz, drag_decay)
}

// ---------------------------------------------------------------------------
// device 臂
// ---------------------------------------------------------------------------

struct DevKernels {
    spv_collide: Vec<u32>,
    entry_collide: String,
    spv_depth: Vec<u32>,
    entry_depth: String,
}

impl DevKernels {
    fn create(args: &Args) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let need = |o: &Option<String>, k: &str| -> String {
            o.clone().unwrap_or_else(|| fail(&format!("缺 {k}")))
        };
        // 两 kernel 均为 f32 乘加链,装载期注入 NoContraction(头注)。
        let spv_collide =
            spv_inject_no_contraction(&load_spv(&need(&args.spv_collide, "--spv-collide")));
        let spv_depth = spv_inject_no_contraction(&load_spv(&need(
            &args.spv_collide_depth,
            "--spv-collide-depth",
        )));
        let entry = |spv: &[u32], k: &str| -> Result<String, String> {
            vk::entry_point_name(spv).ok_or(format!("{k} SPV 无 OpEntryPoint"))
        };
        Ok(Self {
            entry_collide: entry(&spv_collide, "sim_collide")?,
            entry_depth: entry(&spv_depth, "sim_collide_depth")?,
            spv_collide,
            spv_depth,
        })
    }
}

/// device 粒子流(7 f32 + life 常量 + flags/hit 回读;RQ 臂 in/out 拆分逐帧
/// 轮转,depth/off 臂原位)。
struct DevState {
    streams: Vec<Vec<u8>>, // 7:pos3/vel3/age
    life: Vec<u8>,
    flags: Vec<u8>,
    hit: Vec<u8>,
}

impl DevState {
    fn from_pool(p: &ParticlePools) -> Self {
        let n = p.n;
        Self {
            streams: vec![
                bytes_f32(&p.pos_x[..n]),
                bytes_f32(&p.pos_y[..n]),
                bytes_f32(&p.pos_z[..n]),
                bytes_f32(&p.vel_x[..n]),
                bytes_f32(&p.vel_y[..n]),
                bytes_f32(&p.vel_z[..n]),
                bytes_f32(&p.age[..n]),
            ],
            life: bytes_f32(&p.life[..n]),
            flags: vec![0u8; n * 4],
            hit: vec![0u8; n * 4],
        }
    }
}

/// RQ 臂单帧(当帧三角汤重建场景 = 同帧语义;单 BLAS × 单 identity 实例;
/// buffers 序与 kernel 头注 SSBO 序严格一致)。
fn device_frame_rayquery(
    dev: &DevKernels,
    st: &mut DevState,
    tris: &[f32],
    n: usize,
    e_device: f32,
) -> Result<(), String> {
    let f = fields();
    let cp = collision_params();
    let nseg = n.div_ceil(SEG);
    let params = [
        n as f32,
        nseg as f32,
        DT,
        f.gravity_y,
        f.wind[0],
        f.wind[1],
        f.wind[2],
        f.drag,
        e_device,
        cp.mu_t,
        0.0,
        0.0,
    ];
    let params_b = bytes_f32(&params);
    let tris_b = bytes_f32(tris);
    let blas_refs: Vec<&[f32]> = vec![tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let scene = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let buffers = [
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&st.streams[0]),
        RayQueryBufferDesc::Input(&st.streams[1]),
        RayQueryBufferDesc::Input(&st.streams[2]),
        RayQueryBufferDesc::Input(&st.streams[3]),
        RayQueryBufferDesc::Input(&st.streams[4]),
        RayQueryBufferDesc::Input(&st.streams[5]),
        RayQueryBufferDesc::Input(&st.streams[6]),
        RayQueryBufferDesc::Input(&st.life),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
        RayQueryBufferDesc::Output(n * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene,
        &[RayQueryDispatchDesc {
            name: "g35_sim_collide",
            spv: &dev.spv_collide,
            entry: &dev.entry_collide,
            buffers: &buffers,
            push_constants: &[],
            groups: [nseg as u32, 1, 1],
        }],
    )?;
    let rb = out
        .readbacks
        .into_iter()
        .next()
        .ok_or("单 dispatch 缺回读")?;
    if rb.len() != 9 {
        return Err(format!("回读路数 {} ≠ 9", rb.len()));
    }
    let mut it = rb.into_iter();
    for k in 0..7 {
        st.streams[k] = it.next().unwrap();
    }
    st.flags = it.next().unwrap();
    st.hit = it.next().unwrap();
    Ok(())
}

/// depth/off 臂单帧(run_compute 原位;off = res 0 + 1 f32 哨兵深度缓冲
/// 〔Vulkan 禁 0-byte buffer,kernel res=0 不消费〕)。
fn device_frame_depth(
    dev: &DevKernels,
    st: &mut DevState,
    depth_map: &[f32],
    grid: &DepthGrid,
    n: usize,
    e_device: f32,
) -> Result<(), String> {
    let f = fields();
    let cp = collision_params();
    let nseg = n.div_ceil(SEG);
    let params = [
        n as f32,
        nseg as f32,
        DT,
        f.gravity_y,
        f.wind[0],
        f.wind[1],
        f.wind[2],
        f.drag,
        e_device,
        cp.mu_t,
        grid.x0,
        grid.z0,
        grid.cell,
        grid.res as f32,
        0.0,
        0.0,
    ];
    let take = std::mem::take::<Vec<u8>>;
    let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(12);
    bufs.push(bytes_f32(&params));
    bufs.push(if depth_map.is_empty() {
        bytes_f32(&[0.0])
    } else {
        bytes_f32(depth_map)
    });
    for k in 0..7 {
        bufs.push(take(&mut st.streams[k]));
    }
    bufs.push(take(&mut st.life));
    bufs.push(take(&mut st.flags));
    bufs.push(take(&mut st.hit));
    vk::run_compute(&dev.spv_depth, &dev.entry_depth, &mut bufs, &[], [
        nseg as u32,
        1,
        1,
    ])
    .map_err(|e| format!("sim_collide_depth dispatch: {e}"))?;
    for k in 0..7 {
        st.streams[k] = take(&mut bufs[2 + k]);
    }
    st.life = take(&mut bufs[9]);
    st.flags = take(&mut bufs[10]);
    st.hit = take(&mut bufs[11]);
    Ok(())
}

// ---------------------------------------------------------------------------
// 全链单跑(host 平行金标准逐帧对拍 + 链式 digest)
// ---------------------------------------------------------------------------

struct ChainReport {
    f32_stream_max: [f32; 7],
    flags_bitexact: bool,
    hit_mismatch_total: u64,
    hit_mismatch_frames: usize,
    hits_total_host: u64,
    box_hits_move_frame_device: u32,
    digest: String,
    frame_ms_mean: f64,
    problems: Vec<String>,
}

impl ChainReport {
    fn f32_p100(&self) -> f32 {
        self.f32_stream_max.iter().copied().fold(0.0f32, f32::max)
    }

    fn stream_max_json(&self) -> String {
        let inner: Vec<String> = F32_STREAMS
            .iter()
            .zip(self.f32_stream_max.iter())
            .map(|(name, v)| format!("{}:{:e}", jstr(name), v))
            .collect();
        format!("{{{}}}", inner.join(","))
    }
}

/// device 全链(arm 三档;host 平行金标准恒冻结 e;e_device 为 RED 臂篡改
/// 注入点,绿链 = 冻结 e)。TLAS 能力链 Err 在 ray_query 臂 → typed 退 3
/// (fail-closed 禁静默换臂;头注「显式降级链」)。
fn run_chain(
    dev: &DevKernels,
    arm: Arm,
    seed: u64,
    frames: usize,
    cap: usize,
    e_device: f32,
    evidence_out: &Option<String>,
) -> ChainReport {
    let f = fields();
    let cp = collision_params();
    let grid = depth_grid();
    let mut hp = init_pool(seed, cap);
    let mut st = DevState::from_pool(&hp);
    let n = cap;
    let mut r = ChainReport {
        f32_stream_max: [0.0; 7],
        flags_bitexact: true,
        hit_mismatch_total: 0,
        hit_mismatch_frames: 0,
        hits_total_host: 0,
        box_hits_move_frame_device: 0,
        digest: "0".repeat(64),
        frame_ms_mean: 0.0,
        problems: Vec::new(),
    };
    let mut ms_total = 0.0f64;
    let problem = |problems: &mut Vec<String>, msg: String| {
        if problems.len() < 16 {
            problems.push(msg);
        }
    };
    for k in 0..frames {
        let tris = scene_tris(box_center(k));
        // ── host 平行金标准(collision.rs;力场 → 碰撞,当帧场景)──
        apply_fields(&mut hp, &f, DT);
        let hout = match arm {
            Arm::RayQuery => {
                let bvh = build_bvh(&tris);
                collide_step(&mut hp, &bvh, &cp, DT)
            }
            Arm::DepthBuffer => {
                let bvh = build_bvh(&tris);
                let map = synth_topdown_depth(&bvh, &grid);
                depth_collide_step(&mut hp, &grid, &map, &cp, DT)
            }
            Arm::Off => {
                let off = DepthGrid { res: 0, ..grid };
                depth_collide_step(&mut hp, &off, &[], &cp, DT)
            }
        };
        r.hits_total_host += hout.hit.iter().filter(|&&h| h != 0).count() as u64;
        // ── device 派发(墙钟计时;同帧场景重建)──
        let t0 = Instant::now();
        let dres = match arm {
            Arm::RayQuery => device_frame_rayquery(dev, &mut st, &tris, n, e_device),
            Arm::DepthBuffer => {
                let bvh = build_bvh(&tris);
                let map = synth_topdown_depth(&bvh, &grid);
                device_frame_depth(dev, &mut st, &map, &grid, n, e_device)
            }
            Arm::Off => {
                let off = DepthGrid { res: 0, ..grid };
                device_frame_depth(dev, &mut st, &[], &off, n, e_device)
            }
        };
        if let Err(e) = dres {
            if arm == Arm::RayQuery {
                // 降级链 fail-closed:ray_query 档能力链失效 = typed 退 3。
                typed_no_tlas_exit(&format!("帧 {k}: {e}"), evidence_out);
            }
            fail(&format!("帧 {k}: {e}"));
        }
        ms_total += t0.elapsed().as_secs_f64() * 1000.0;
        // ── f32 7 流 max abs diff(全帧 p100 聚合;probe 只测不判)──
        for s in 0..7 {
            let dev_f = read_f32(&st.streams[s]);
            let host_f: &[f32] = match s {
                0 => &hp.pos_x,
                1 => &hp.pos_y,
                2 => &hp.pos_z,
                3 => &hp.vel_x,
                4 => &hp.vel_y,
                5 => &hp.vel_z,
                _ => &hp.age,
            };
            for i in 0..n {
                let mut d = (dev_f[i] - host_f[i]).abs();
                if !d.is_finite() {
                    d = f32::INFINITY;
                    problem(
                        &mut r.problems,
                        format!("帧 {k}: {} 流非有限差(i={i})", F32_STREAMS[s]),
                    );
                }
                if d > r.f32_stream_max[s] {
                    r.f32_stream_max[s] = d;
                }
            }
        }
        // ── flags 整数流零容差;hit 流失配计数诚实登记(头注判据面 ②)──
        let dev_flags = read_u32(&st.flags);
        if dev_flags[..n] != hout.flags[..] {
            r.flags_bitexact = false;
            problem(&mut r.problems, format!("帧 {k}: flags 非位级"));
        }
        let dev_hit = read_u32(&st.hit);
        let mism = (0..n).filter(|&i| dev_hit[i] != hout.hit[i]).count();
        if mism > 0 {
            r.hit_mismatch_total += mism as u64;
            r.hit_mismatch_frames += 1;
        }
        if k == MOVE_FRAME && arm == Arm::RayQuery {
            r.box_hits_move_frame_device = dev_hit[..n]
                .iter()
                .filter(|&&h| h >= 1 + BOX_TRI_LO && h < 1 + BOX_TRI_HI)
                .count() as u32;
        }
        // ── 链式 digest(7 f32 流 ‖ flags ‖ hit;sha256(prev_hex ‖ bytes))──
        let mut trace: Vec<u8> = Vec::with_capacity(64 + n * 36);
        trace.extend_from_slice(r.digest.as_bytes());
        for s in 0..7 {
            trace.extend_from_slice(&st.streams[s]);
        }
        trace.extend_from_slice(&st.flags);
        trace.extend_from_slice(&st.hit);
        r.digest = rurix_pkg::sha256::hex_digest(&trace);
    }
    r.frame_ms_mean = ms_total / frames as f64;
    r
}

// ---------------------------------------------------------------------------
// host-only 腿(host 金标准链恒可跑:见证 + 力场语义 + 双跑位级)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let w = run_witness(args.seed, args.frames, args.cap);
    let (wind_dx, wind_dz, drag_decay) = fields_sanity(args.seed, args.cap);
    let (a, ha) = host_witness_run(&box_center, args.seed, args.frames, args.cap);
    let (b, _) = host_witness_run(&box_center, args.seed, args.frames, args.cap);
    let double_ok = a == b;
    let hits_total: u64 = ha.iter().map(|&h| h as u64).sum();
    let ok = double_ok
        && wind_dx
        && wind_dz
        && drag_decay
        && (!w.applicable
            || (w.host_div_static_frame == MOVE_FRAME as i64
                && w.late_same_at_move_frame
                && w.gold_late_differ_at_move_frame
                && w.box_hits_move_frame_host >= 1));
    let line = format!(
        "{{\"schema\":\"rurix.g35.collision_host.v1\",\"mode\":\"host-only\",\"state\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"witness_applicable\":{},\
         \"host_div_static_frame\":{},\"late_same_at_move_frame\":{},\
         \"gold_late_differ_at_move_frame\":{},\"box_hits_frame32_host\":{},\
         \"box_hits_total\":{hits_total},\"wind_dx_positive\":{wind_dx},\
         \"wind_dz_positive\":{wind_dz},\"drag_speed_decay\":{drag_decay},\
         \"double_run_bitexact\":{double_ok},\"base_commit\":{}}}",
        jstr(if ok { "pass" } else { "fail" }),
        args.frames,
        args.cap,
        args.seed,
        w.applicable,
        w.host_div_static_frame,
        w.late_same_at_move_frame,
        w.gold_late_differ_at_move_frame,
        w.box_hits_move_frame_host,
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:双跑同 seed;--red-arm tamper-e = 绿/红双链)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.host_only {
        host_only_leg(&args);
    }

    // 显式降级链 fail-closed 演示注入:--force-no-tlas 在 ray_query 档模拟
    // TLAS 能力缺失 → typed 退 3(任何机器上机验「禁静默换臂」的机器事实;
    // loader 探测之前判——能力缺失语义与环境无关)。
    if args.force_no_tlas {
        if args.collision == Arm::RayQuery {
            typed_no_tlas_exit("--force-no-tlas 注入(能力缺失演示)", &args.evidence_out);
        }
        fail("--force-no-tlas 仅对 --collision ray_query 有意义(降级链档位语义)");
    }

    let dev = match DevKernels::create(&args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g35.collision_probe.v1\",\"state\":\"skipped_dev_env\",\
                 \"reason\":{}}}",
                jstr(&e)
            );
            emit_evidence(&line, &args.evidence_out);
            std::process::exit(0);
        }
    };

    let cp = collision_params();
    let f = fields();

    if let Some(arm_name) = &args.red_arm {
        if arm_name != "tamper-e" {
            fail(&format!("未知 RED 臂: {arm_name}(闭集 tamper-e)"));
        }
        // RED 臂:device 红链 e×1.5 篡改注入,host 金标准恒冻结 e ⇒ 红链
        // 对拍 measured 必溢出容差(smoke 复核 > threshold);绿/红 digest
        // 必异(digest 判据对响应系数敏感性证明)。
        let g = run_chain(
            &dev,
            args.collision,
            args.seed,
            args.frames,
            args.cap,
            cp.e,
            &args.evidence_out,
        );
        let e_red = cp.e * TAMPER_E_FACTOR;
        let r = run_chain(
            &dev,
            args.collision,
            args.seed,
            args.frames,
            args.cap,
            e_red,
            &args.evidence_out,
        );
        let detected = g.digest != r.digest;
        let line = format!(
            "{{\"schema\":\"rurix.g35.collision_red_arm.v1\",\"arm\":\"tamper-e\",\
             \"collision\":{},\"detected\":{detected},\"e_frozen\":{:e},\"e_tampered\":{:e},\
             \"green_f32_max_abs_diff\":{:e},\"red_f32_max_abs_diff\":{:e},\
             \"digest_green\":{},\"digest_red\":{},\"base_commit\":{}}}",
            jstr(args.collision.name()),
            cp.e,
            e_red,
            g.f32_p100(),
            r.f32_p100(),
            jstr(&format!("sha256:{}", g.digest)),
            jstr(&format!("sha256:{}", r.digest)),
            jstr(&base_commit()),
        );
        emit_evidence(&line, &args.evidence_out);
        if !detected {
            fail("red-arm tamper-e 失效(漏检):篡改 e 后 digest 未变");
        }
        eprintln!("{TAG}: red-arm tamper-e 检出 — digest 已异,red_f32={:e}", r.f32_p100());
        std::process::exit(0);
    }

    // ── 全档验证:双跑同 seed(⑤ device 双跑位级)+ 逐帧对拍(①②)+
    //    见证(③,ray_query 档)+ 力场语义(④)──
    let a = run_chain(
        &dev,
        args.collision,
        args.seed,
        args.frames,
        args.cap,
        cp.e,
        &args.evidence_out,
    );
    let b = run_chain(
        &dev,
        args.collision,
        args.seed,
        args.frames,
        args.cap,
        cp.e,
        &args.evidence_out,
    );
    let determinism = a.digest == b.digest;
    let w = if args.collision == Arm::RayQuery {
        run_witness(args.seed, args.frames, args.cap)
    } else {
        Witness {
            applicable: false,
            host_div_static_frame: -1,
            late_same_at_move_frame: false,
            gold_late_differ_at_move_frame: false,
            box_hits_move_frame_host: 0,
        }
    };
    let (wind_dx, wind_dz, drag_decay) = fields_sanity(args.seed, args.cap);
    let witness_ok = !w.applicable
        || (w.host_div_static_frame == MOVE_FRAME as i64
            && w.late_same_at_move_frame
            && w.gold_late_differ_at_move_frame
            && w.box_hits_move_frame_host >= 1
            && a.box_hits_move_frame_device >= 1);
    // 样本量门:碰撞臂命中必须非零(空转 = 判据镂空);off 档命中必须恒零。
    let sample_ok = match args.collision {
        Arm::Off => a.hits_total_host == 0,
        _ => a.hits_total_host >= 1,
    };
    let state = if a.flags_bitexact && determinism && witness_ok && wind_dx && wind_dz
        && drag_decay && sample_ok && a.problems.is_empty()
    {
        "pass"
    } else {
        "fail"
    };
    if args.report_max_diff {
        println!("f32_max_abs_diff={:e}", a.f32_p100());
    }
    eprintln!(
        "{TAG}: {} arm={} frames={} cap={} seed={} f32_p100={:e} flags_bitexact={} \
         hit_mism={} witness_ok={} hits_host={} double_run={} frame_ms={:.3}",
        state,
        args.collision.name(),
        args.frames,
        args.cap,
        args.seed,
        a.f32_p100(),
        a.flags_bitexact,
        a.hit_mismatch_total,
        witness_ok,
        a.hits_total_host,
        determinism,
        a.frame_ms_mean,
    );
    let mut problems = a.problems.clone();
    if !determinism {
        problems.push("device 双跑 digest 非位级一致".into());
    }
    if !sample_ok {
        problems.push(format!(
            "样本量门破:arm={} hits_total_host={}",
            args.collision.name(),
            a.hits_total_host
        ));
    }
    let line = format!(
        "{{\"schema\":\"rurix.g35.collision_probe.v1\",\"state\":{},\"arm\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"dt\":{:e},\
         \"e\":{:e},\"mu_t\":{:e},\"eps\":1e-3,\
         \"gravity_y\":{:e},\"wind\":[{:e},{:e},{:e}],\"drag\":{:e},\
         \"f32_max_abs_diff\":{:e},\"f32_stream_max\":{},\
         \"flags_bitexact\":{},\"hit_mismatch_total\":{},\"hit_mismatch_frames\":{},\
         \"hits_total_host\":{},\"box_move_frame\":{},\
         \"witness_applicable\":{},\"host_div_static_frame\":{},\
         \"late_same_at_move_frame\":{},\"gold_late_differ_at_move_frame\":{},\
         \"box_hits_frame32_host\":{},\"box_hits_frame32_device\":{},\
         \"wind_dx_positive\":{},\"wind_dz_positive\":{},\"drag_speed_decay\":{},\
         \"determinism_double_run\":{},\"digest_a\":{},\"digest_b\":{},\
         \"frame_ms_mean\":{:.6},\
         \"nocontraction_injected\":[\"g35_sim_collide\",\"g35_sim_collide_depth\"],\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        jstr(args.collision.name()),
        args.frames,
        args.cap,
        args.seed,
        DT,
        cp.e,
        cp.mu_t,
        f.gravity_y,
        f.wind[0],
        f.wind[1],
        f.wind[2],
        f.drag,
        a.f32_p100(),
        a.stream_max_json(),
        a.flags_bitexact,
        a.hit_mismatch_total,
        a.hit_mismatch_frames,
        a.hits_total_host,
        MOVE_FRAME,
        w.applicable,
        w.host_div_static_frame,
        w.late_same_at_move_frame,
        w.gold_late_differ_at_move_frame,
        w.box_hits_move_frame_host,
        a.box_hits_move_frame_device,
        wind_dx,
        wind_dz,
        drag_decay,
        determinism,
        jstr(&format!("sha256:{}", a.digest)),
        jstr(&format!("sha256:{}", b.digest)),
        a.frame_ms_mean,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    if state != "pass" {
        std::process::exit(1);
    }
}
