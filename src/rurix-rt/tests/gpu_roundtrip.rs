//! rurix-rt 运行时全链路真跑(M4.3,契约 D-M4-4 出口判据;08 §2)。
//!
//! **子进程隔离**(14 §6):GPU 操作在重入自身的子进程中执行——device 崩溃
//! (非法访存 / assert)不连坐 harness。父进程 spawn `--exact <test>` 子进程并
//! 断言其 exit 0;子进程经 `RURIX_RT_GPU_CHILD` 环境旗标进入 GPU 工作体。
//!
//! **无 GPU 降级 SKIP**:`Context::device_count()==0` / 驱动不可用时跳过(对齐
//! ptxas 关卡 SKIP 纪律,真实红绿在带 GPU 的 self-hosted runner;本机 RTX 4070 Ti
//! 真跑)。
//!
//! 锚定:装载协商(RXS-0076)+ 经典内存路径 + launch + 拷回逐元素核对。

use core::ffi::c_void;

use rurix_rt::{Context, CudaError};

const CHILD_ENV: &str = "RURIX_RT_GPU_CHILD";
const REL_TOL: f64 = 1e-5;

// M4.4 端到端(契约 D-M4-5):build.rs 经 rurixc 全管线把 kernels/saxpy.rx 产 PTX
// 嵌入(`include_str!`)。空 = 构建期无 clang/rurixc(降级 SKIP)。
const RURIX_SAXPY_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/saxpy.ptx"));
include!(concat!(env!("OUT_DIR"), "/saxpy_meta.rs")); // pub const SAXPY_KERNEL

// M5.3 gpu 并行基元(契约 D-M5-5):build.rs 经 rurixc 全管线(含 libdevice 链接)
// 产 PTX 嵌入。空 = 构建期无 clang/CUDA(降级 SKIP)。
const RURIX_REDUCE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/reduce.ptx"));
include!(concat!(env!("OUT_DIR"), "/reduce_meta.rs")); // REDUCE_KERNEL
const RURIX_SCAN_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/scan.ptx"));
include!(concat!(env!("OUT_DIR"), "/scan_meta.rs")); // SCAN_KERNEL
const RURIX_TRANSPOSE_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/transpose.ptx"));
include!(concat!(env!("OUT_DIR"), "/transpose_meta.rs")); // TRANSPOSE_KERNEL
const RURIX_GEMM_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/gemm_tile.ptx"));
include!(concat!(env!("OUT_DIR"), "/gemm_tile_meta.rs")); // GEMM_TILE_KERNEL

/// 手写 SAXPY PTX(`y[i] = a*x[i] + y[i]`,mul.rn+add.rn 与 host f32 两步舍入逐位
/// 一致;.version 8.0 为协商起点,驱动不支持时 rurix-rt 自动降版,08 §2.4)。
const SAXPY_PTX: &str = r#".version 8.0
.target sm_89
.address_size 64

.visible .entry saxpy(
    .param .u64 p_x,
    .param .u64 p_y,
    .param .f32 p_a,
    .param .u32 p_n
)
{
    .reg .pred  %p1;
    .reg .b32   %r<6>;
    .reg .f32   %f<5>;
    .reg .b64   %rd<8>;

    ld.param.u64    %rd1, [p_x];
    ld.param.u64    %rd2, [p_y];
    ld.param.f32    %f1,  [p_a];
    ld.param.u32    %r1,  [p_n];

    mov.u32         %r2, %ctaid.x;
    mov.u32         %r3, %ntid.x;
    mov.u32         %r4, %tid.x;
    mad.lo.s32      %r5, %r2, %r3, %r4;

    setp.ge.u32     %p1, %r5, %r1;
    @%p1 bra        DONE;

    cvta.to.global.u64  %rd3, %rd1;
    cvta.to.global.u64  %rd4, %rd2;
    mul.wide.u32        %rd5, %r5, 4;
    add.s64             %rd6, %rd3, %rd5;
    add.s64             %rd7, %rd4, %rd5;

    ld.global.f32   %f2, [%rd6];
    ld.global.f32   %f3, [%rd7];
    mul.rn.f32      %f4, %f1, %f2;
    add.rn.f32      %f4, %f4, %f3;
    st.global.f32   [%rd7], %f4;

DONE:
    ret;
}
"#;

/// 子进程隔离执行器:父进程 spawn 自身 `--exact <test>` 子进程并断言 exit 0;
/// 子进程(`RURIX_RT_GPU_CHILD` 置位)运行 `body`(GPU 工作体)。
fn isolated(test_name: &str, body: fn()) {
    if std::env::var(CHILD_ENV).is_ok() {
        body();
        return;
    }
    let exe = std::env::current_exe().expect("定位测试可执行文件");
    let status = std::process::Command::new(exe)
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .status()
        .expect("spawn 子进程失败");
    assert!(
        status.success(),
        "子进程 GPU 真跑失败(exit={:?}):{test_name}",
        status.code()
    );
}

/// GPU 是否可用(无驱动 / 无设备 → SKIP)。
fn gpu_available() -> bool {
    match Context::device_count() {
        Ok(n) => n > 0,
        Err(CudaError::DriverUnavailable) => false,
        // 其余错误(驱动在但初始化异常):视为不可用,SKIP(不误判失败)
        Err(_) => false,
    }
}

/// 冒烟:cuInit + Context 创建/销毁(FFI 加载器 + RAII 通路真跑)。
#[test]
fn context_smoke_isolated() {
    isolated("context_smoke_isolated", || {
        if !gpu_available() {
            eprintln!("[rurix-rt] SKIP context_smoke: 无可用 GPU/驱动(降级 SKIP)");
            return;
        }
        let ctx = Context::new().expect("创建 Context");
        assert!(!ctx.is_poisoned());
        ctx.synchronize().expect("cuCtxSynchronize");
        // Drop 自动 cuCtxSynchronize + cuCtxDestroy(D-231)
    });
}

/// 全链路真跑:装载协商 → alloc → H2D → launch → D2H → 逐元素 f32 精确核对。
#[test]
fn saxpy_roundtrip_isolated() {
    isolated("saxpy_roundtrip_isolated", || {
        if !gpu_available() {
            eprintln!("[rurix-rt] SKIP saxpy_roundtrip: 无可用 GPU/驱动(降级 SKIP)");
            return;
        }
        let n: usize = 4096;
        let a: f32 = 2.5;
        let x: Vec<f32> = (0..n).map(|i| (i as f32) * 0.5).collect();
        let y: Vec<f32> = (0..n).map(|i| (i as f32) * -1.25 + 3.0).collect();
        // host 参考:两步舍入(mul 后 add),与 PTX mul.rn+add.rn 逐位一致
        let expect: Vec<f32> = (0..n).map(|i| a * x[i] + y[i]).collect();

        let ctx = Context::new().expect("创建 Context");

        let mut dx = ctx.alloc::<f32>(n).expect("alloc dx");
        let mut dy = ctx.alloc::<f32>(n).expect("alloc dy");
        dx.copy_from_host(&x).expect("H2D x");
        dy.copy_from_host(&y).expect("H2D y");

        let module = ctx
            .load_module(SAXPY_PTX)
            .expect("装载协商 + cuModuleLoadDataEx");
        eprintln!(
            "[rurix-rt] 装载协商通过,.version = {}",
            module.negotiated_version()
        );
        let kernel = module.function("saxpy").expect("cuModuleGetFunction saxpy");
        let stream = ctx.create_stream().expect("create_stream");

        // launch 实参(按 kernel 形参顺序:p_x:u64, p_y:u64, p_a:f32, p_n:u32)
        let mut px = dx.device_ptr();
        let mut py = dy.device_ptr();
        let mut aa = a;
        let mut nn = n as u32;
        let mut params: [*mut c_void; 4] = [
            (&raw mut px).cast::<c_void>(),
            (&raw mut py).cast::<c_void>(),
            (&raw mut aa).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        let block = 256u32;
        let grid = (n as u32).div_ceil(block);
        stream
            .launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)
            .expect("cuLaunchKernel");
        stream.synchronize().expect("cuStreamSynchronize");

        let mut got = vec![0f32; n];
        dy.copy_to_host(&mut got).expect("D2H result");

        for i in 0..n {
            assert_eq!(
                got[i], expect[i],
                "SAXPY 逐元素核对失败 @ {i}: got {} expect {}",
                got[i], expect[i]
            );
        }
        eprintln!("[rurix-rt] SAXPY 全链路真跑通过:{n} 元素 f32 精确相等");
    });
}

/// M4.4 端到端真跑(契约 D-M4-5 / G-M4-1 真跑通道,CI 步骤 20):**Rurix 源**
/// `kernels/saxpy.rx` 经 rurixc 全管线(着色检查 → NVPTX codegen → ptxas 关卡 →
/// clang IR→PTX)产 PTX,嵌入后装载 → H2D → launch → D2H → 逐元素 f32 精确核对。
/// 与 [`saxpy_roundtrip_isolated`](手写 PTX)的区别:此处 PTX 来自 device codegen。
//@ spec: RXS-0070, RXS-0071, RXS-0072, RXS-0076
#[test]
fn rurix_saxpy_e2e_isolated() {
    isolated("rurix_saxpy_e2e_isolated", || {
        if RURIX_SAXPY_PTX.trim().is_empty() || SAXPY_KERNEL.is_empty() {
            eprintln!(
                "[rurix-rt] SKIP rurix_saxpy_e2e: 构建期无 clang/rurixc,未嵌入 device PTX(降级 SKIP)"
            );
            return;
        }
        if !gpu_available() {
            eprintln!("[rurix-rt] SKIP rurix_saxpy_e2e: 无可用 GPU/驱动(降级 SKIP)");
            return;
        }
        let n: usize = 1 << 20;
        let a: f32 = 2.5;
        let x: Vec<f32> = (0..n).map(|i| (i as f32) * 0.5).collect();
        let y: Vec<f32> = (0..n).map(|i| (i as f32) * -1.25 + 3.0).collect();
        let expect: Vec<f32> = (0..n).map(|i| a * x[i] + y[i]).collect();

        let ctx = Context::new().expect("创建 Context");
        let mut dx = ctx.alloc::<f32>(n).expect("alloc dx");
        let mut dy = ctx.alloc::<f32>(n).expect("alloc dy");
        let d_out = ctx.alloc::<f32>(n).expect("alloc out");
        dx.copy_from_host(&x).expect("H2D x");
        dy.copy_from_host(&y).expect("H2D y");

        let module = ctx
            .load_module(RURIX_SAXPY_PTX)
            .expect("装载协商 + cuModuleLoadDataEx(rurixc 生成 PTX)");
        eprintln!(
            "[rurix-rt] rurix_saxpy 装载协商通过,.version = {},entry = {}",
            module.negotiated_version(),
            SAXPY_KERNEL
        );
        let kernel = module.function(SAXPY_KERNEL).expect("cuModuleGetFunction");
        let stream = ctx.create_stream().expect("create_stream");

        // 形参顺序(device codegen):out:ptr, x:ptr, y:ptr, a:f32, n:usize/i64
        let mut p_out = d_out.device_ptr();
        let mut p_x = dx.device_ptr();
        let mut p_y = dy.device_ptr();
        let mut aa = a;
        let mut nn: u64 = n as u64;
        let mut params: [*mut c_void; 5] = [
            (&raw mut p_out).cast::<c_void>(),
            (&raw mut p_x).cast::<c_void>(),
            (&raw mut p_y).cast::<c_void>(),
            (&raw mut aa).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        let block = 256u32;
        let grid = (n as u32).div_ceil(block);
        stream
            .launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)
            .expect("cuLaunchKernel");
        stream.synchronize().expect("cuStreamSynchronize");

        let mut got = vec![0f32; n];
        d_out.copy_to_host(&mut got).expect("D2H result");
        for i in 0..n {
            assert_eq!(
                got[i], expect[i],
                "Rurix SAXPY 逐元素核对失败 @ {i}: got {} expect {}",
                got[i], expect[i]
            );
        }
        eprintln!(
            "[rurix-rt] Rurix SAXPY 端到端真跑通过:{n} 元素 f32 精确相等(device codegen PTX)"
        );
    });
}

/// 守卫:嵌入 PTX 空 / 无 GPU → SKIP(降级,返回 true 表示应跳过)。
fn skip_kernel(tag: &str, ptx: &str, entry: &str) -> bool {
    if ptx.trim().is_empty() || entry.is_empty() {
        eprintln!("[rurix-rt] SKIP {tag}: 构建期未嵌入 device PTX(降级 SKIP)");
        return true;
    }
    if !gpu_available() {
        eprintln!("[rurix-rt] SKIP {tag}: 无可用 GPU/驱动(降级 SKIP)");
        return true;
    }
    false
}

/// M5.3 reduce 端到端真跑(契约 D-M5-5):block 级 shared 树形归约 → 每 block partial,
/// host 合并;相对容差核对(浮点重排)。atomics-free。
//@ spec: RXS-0079
#[test]
fn rurix_reduce_e2e_isolated() {
    isolated("rurix_reduce_e2e_isolated", || {
        if skip_kernel("rurix_reduce_e2e", RURIX_REDUCE_PTX, REDUCE_KERNEL) {
            return;
        }
        let n: usize = 1 << 20;
        let block = 256u32;
        let src: Vec<f32> = (0..n).map(|i| ((i % 13) as f32) * 0.25).collect();
        let expect: f64 = src.iter().map(|&v| v as f64).sum();
        let grid = (n as u32).div_ceil(block);
        let nblocks = grid as usize;

        let ctx = Context::new().expect("Context");
        let mut dsrc = ctx.alloc::<f32>(n).expect("alloc src");
        let dpart = ctx.alloc::<f32>(nblocks).expect("alloc partials");
        dsrc.copy_from_host(&src).expect("H2D src");
        let module = ctx.load_module(RURIX_REDUCE_PTX).expect("load_module");
        let kernel = module.function(REDUCE_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_src = dsrc.device_ptr();
        let mut p_part = dpart.device_ptr();
        let mut nn: u64 = n as u64;
        let mut params: [*mut c_void; 3] = [
            (&raw mut p_src).cast::<c_void>(),
            (&raw mut p_part).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        stream
            .launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut partials = vec![0f32; nblocks];
        dpart.copy_to_host(&mut partials).expect("D2H partials");
        let got: f64 = partials.iter().map(|&v| v as f64).sum();
        let denom = expect.abs().max(1.0);
        assert!(
            (got - expect).abs() / denom <= REL_TOL,
            "reduce 偏差超容差:got {got} expect {expect}"
        );
        eprintln!("[rurix-rt] reduce 真跑通过:sum={got} 参考={expect}");
    });
}

/// M5.3 scan 端到端真跑(契约 D-M5-5):block 级 Hillis-Steele inclusive 前缀和,
/// shared+barrier,atomics-free;与 host 逐 block 参考相对容差核对。
//@ spec: RXS-0079
#[test]
fn rurix_scan_e2e_isolated() {
    isolated("rurix_scan_e2e_isolated", || {
        if skip_kernel("rurix_scan_e2e", RURIX_SCAN_PTX, SCAN_KERNEL) {
            return;
        }
        let n: usize = 1 << 20;
        let block = 256usize;
        let src: Vec<f32> = (0..n).map(|i| ((i % 11) as f32) * 0.5 + 0.25).collect();
        let mut expect = vec![0f32; n];
        for base in (0..n).step_by(block) {
            let end = (base + block).min(n);
            let mut acc = 0f64;
            for i in base..end {
                acc += src[i] as f64;
                expect[i] = acc as f32;
            }
        }
        let ctx = Context::new().expect("Context");
        let mut dsrc = ctx.alloc::<f32>(n).expect("alloc src");
        let ddst = ctx.alloc::<f32>(n).expect("alloc dst");
        dsrc.copy_from_host(&src).expect("H2D src");
        let module = ctx.load_module(RURIX_SCAN_PTX).expect("load_module");
        let kernel = module.function(SCAN_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_src = dsrc.device_ptr();
        let mut p_dst = ddst.device_ptr();
        let mut nn: u64 = n as u64;
        let mut params: [*mut c_void; 3] = [
            (&raw mut p_src).cast::<c_void>(),
            (&raw mut p_dst).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        let grid = (n as u32).div_ceil(block as u32);
        stream
            .launch(&kernel, [grid, 1, 1], [block as u32, 1, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut got = vec![0f32; n];
        ddst.copy_to_host(&mut got).expect("D2H dst");
        for i in 0..n {
            let denom = (expect[i] as f64).abs().max(1.0);
            assert!(
                (got[i] as f64 - expect[i] as f64).abs() / denom <= REL_TOL,
                "scan @ {i}: got {} expect {}",
                got[i],
                expect[i]
            );
        }
        eprintln!("[rurix-rt] scan 真跑通过:{n} 元素逐 block inclusive scan 核对一致");
    });
}

/// M5.3 transpose 端到端真跑(契约 D-M5-5):16x16 shared-tile 转置(2D ThreadCtx),
/// `dst[R*h+C]=src[C*w+R]` 逐元素精确核对。atomics-free。
//@ spec: RXS-0072, RXS-0079
#[test]
fn rurix_transpose_e2e_isolated() {
    isolated("rurix_transpose_e2e_isolated", || {
        if skip_kernel("rurix_transpose_e2e", RURIX_TRANSPOSE_PTX, TRANSPOSE_KERNEL) {
            return;
        }
        let (w, h) = (200usize, 150usize);
        let tile = 16u32;
        let src: Vec<f32> = (0..h * w).map(|i| i as f32 * 0.5).collect();
        let mut expect = vec![0f32; w * h];
        for r in 0..w {
            for c in 0..h {
                expect[r * h + c] = src[c * w + r];
            }
        }
        let ctx = Context::new().expect("Context");
        let mut dsrc = ctx.alloc::<f32>(h * w).expect("alloc src");
        let ddst = ctx.alloc::<f32>(w * h).expect("alloc dst");
        dsrc.copy_from_host(&src).expect("H2D src");
        let module = ctx.load_module(RURIX_TRANSPOSE_PTX).expect("load_module");
        let kernel = module.function(TRANSPOSE_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_src = dsrc.device_ptr();
        let mut p_dst = ddst.device_ptr();
        let mut ww: u64 = w as u64;
        let mut hh: u64 = h as u64;
        let mut params: [*mut c_void; 4] = [
            (&raw mut p_src).cast::<c_void>(),
            (&raw mut p_dst).cast::<c_void>(),
            (&raw mut ww).cast::<c_void>(),
            (&raw mut hh).cast::<c_void>(),
        ];
        let gx = (w as u32).div_ceil(tile);
        let gy = (h as u32).div_ceil(tile);
        stream
            .launch(&kernel, [gx, gy, 1], [tile, tile, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut got = vec![0f32; w * h];
        ddst.copy_to_host(&mut got).expect("D2H dst");
        for i in 0..w * h {
            assert_eq!(got[i], expect[i], "transpose @ {i}");
        }
        eprintln!("[rurix-rt] transpose 真跑通过:{h}x{w} → {w}x{h} 逐元素相等");
    });
}

/// M5.3 tiled GEMM 端到端真跑(契约 D-M5-5 / G-M5-1 通道):经典 16x16 shared tiling,
/// **不触 Tensor Core**(SG-002 维持);与 host f64 累加参考相对容差核对。atomics-free。
//@ spec: RXS-0072, RXS-0079
#[test]
fn rurix_gemm_tile_e2e_isolated() {
    isolated("rurix_gemm_tile_e2e_isolated", || {
        if skip_kernel("rurix_gemm_e2e", RURIX_GEMM_PTX, GEMM_TILE_KERNEL) {
            return;
        }
        let (m, n, k) = (100usize, 80usize, 70usize);
        let tile = 16u32;
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 7) as f32) * 0.1 + 0.05).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 5) as f32) * 0.2 + 0.1).collect();
        let mut expect = vec![0f32; m * n];
        for row in 0..m {
            for col in 0..n {
                let mut acc = 0f64;
                for kk in 0..k {
                    acc += a[row * k + kk] as f64 * b[kk * n + col] as f64;
                }
                expect[row * n + col] = acc as f32;
            }
        }
        let ctx = Context::new().expect("Context");
        let mut da = ctx.alloc::<f32>(m * k).expect("alloc a");
        let mut db = ctx.alloc::<f32>(k * n).expect("alloc b");
        let dc = ctx.alloc::<f32>(m * n).expect("alloc c");
        da.copy_from_host(&a).expect("H2D a");
        db.copy_from_host(&b).expect("H2D b");
        let module = ctx.load_module(RURIX_GEMM_PTX).expect("load_module");
        let kernel = module.function(GEMM_TILE_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_a = da.device_ptr();
        let mut p_b = db.device_ptr();
        let mut p_c = dc.device_ptr();
        let mut mm: u64 = m as u64;
        let mut nn: u64 = n as u64;
        let mut kk: u64 = k as u64;
        let mut params: [*mut c_void; 6] = [
            (&raw mut p_a).cast::<c_void>(),
            (&raw mut p_b).cast::<c_void>(),
            (&raw mut p_c).cast::<c_void>(),
            (&raw mut mm).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
            (&raw mut kk).cast::<c_void>(),
        ];
        let gx = (n as u32).div_ceil(tile);
        let gy = (m as u32).div_ceil(tile);
        stream
            .launch(&kernel, [gx, gy, 1], [tile, tile, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut got = vec![0f32; m * n];
        dc.copy_to_host(&mut got).expect("D2H c");
        for i in 0..m * n {
            let denom = (expect[i] as f64).abs().max(1.0);
            assert!(
                (got[i] as f64 - expect[i] as f64).abs() / denom <= 1e-3,
                "gemm @ {i}: got {} expect {}",
                got[i],
                expect[i]
            );
        }
        eprintln!("[rurix-rt] tiled GEMM 真跑通过:{m}x{k} * {k}x{n} 核对一致");
    });
}

const PARTIAL_N: usize = 256 * 1024 + 37;

/// M5.3 review fix:末 block 非满(n % 256 != 0)reduce 回归。
#[test]
fn rurix_reduce_partial_block_e2e_isolated() {
    isolated("rurix_reduce_partial_block_e2e_isolated", || {
        if skip_kernel("rurix_reduce_partial", RURIX_REDUCE_PTX, REDUCE_KERNEL) {
            return;
        }
        let n = PARTIAL_N;
        let block = 256u32;
        let src: Vec<f32> = (0..n).map(|i| ((i % 13) as f32) * 0.25).collect();
        let expect: f64 = src.iter().map(|&v| v as f64).sum();
        let grid = (n as u32).div_ceil(block);
        let nblocks = grid as usize;
        let ctx = Context::new().expect("Context");
        let mut dsrc = ctx.alloc::<f32>(n).expect("alloc src");
        let dpart = ctx.alloc::<f32>(nblocks).expect("alloc partials");
        dsrc.copy_from_host(&src).expect("H2D src");
        let module = ctx.load_module(RURIX_REDUCE_PTX).expect("load_module");
        let kernel = module.function(REDUCE_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_src = dsrc.device_ptr();
        let mut p_part = dpart.device_ptr();
        let mut nn: u64 = n as u64;
        let mut params: [*mut c_void; 3] = [
            (&raw mut p_src).cast::<c_void>(),
            (&raw mut p_part).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        stream
            .launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut partials = vec![0f32; nblocks];
        dpart.copy_to_host(&mut partials).expect("D2H partials");
        let got: f64 = partials.iter().map(|&v| v as f64).sum();
        let denom = expect.abs().max(1.0);
        assert!(
            (got - expect).abs() / denom <= REL_TOL,
            "partial-block reduce: got {got} expect {expect}"
        );
        eprintln!("[rurix-rt] reduce 末 block 非满真跑通过:n={n} sum={got}");
    });
}

/// M5.3 review fix:末 block 非满 scan 回归(block-local inclusive)。
#[test]
fn rurix_scan_partial_block_e2e_isolated() {
    isolated("rurix_scan_partial_block_e2e_isolated", || {
        if skip_kernel("rurix_scan_partial", RURIX_SCAN_PTX, SCAN_KERNEL) {
            return;
        }
        let n = PARTIAL_N;
        let block = 256usize;
        let src: Vec<f32> = (0..n).map(|i| ((i % 11) as f32) * 0.5 + 0.25).collect();
        let mut expect = vec![0f32; n];
        for base in (0..n).step_by(block) {
            let end = (base + block).min(n);
            let mut acc = 0f64;
            for i in base..end {
                acc += src[i] as f64;
                expect[i] = acc as f32;
            }
        }
        let ctx = Context::new().expect("Context");
        let mut dsrc = ctx.alloc::<f32>(n).expect("alloc src");
        let ddst = ctx.alloc::<f32>(n).expect("alloc dst");
        dsrc.copy_from_host(&src).expect("H2D src");
        let module = ctx.load_module(RURIX_SCAN_PTX).expect("load_module");
        let kernel = module.function(SCAN_KERNEL).expect("function");
        let stream = ctx.create_stream().expect("stream");
        let mut p_src = dsrc.device_ptr();
        let mut p_dst = ddst.device_ptr();
        let mut nn: u64 = n as u64;
        let mut params: [*mut c_void; 3] = [
            (&raw mut p_src).cast::<c_void>(),
            (&raw mut p_dst).cast::<c_void>(),
            (&raw mut nn).cast::<c_void>(),
        ];
        let grid = (n as u32).div_ceil(block as u32);
        stream
            .launch(&kernel, [grid, 1, 1], [block as u32, 1, 1], &mut params)
            .expect("launch");
        stream.synchronize().expect("sync");
        let mut got = vec![0f32; n];
        ddst.copy_to_host(&mut got).expect("D2H dst");
        for i in 0..n {
            let denom = (expect[i] as f64).abs().max(1.0);
            assert!(
                (got[i] as f64 - expect[i] as f64).abs() / denom <= REL_TOL,
                "partial scan @ {i}: got {} expect {}",
                got[i],
                expect[i]
            );
        }
        eprintln!("[rurix-rt] scan 末 block 非满真跑通过:n={n}");
    });
}
