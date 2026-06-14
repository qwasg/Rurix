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

// M4.4 端到端(契约 D-M4-5):build.rs 经 rurixc 全管线把 kernels/saxpy.rx 产 PTX
// 嵌入(`include_str!`)。空 = 构建期无 clang/rurixc(降级 SKIP)。
const RURIX_SAXPY_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/saxpy.ptx"));
include!(concat!(env!("OUT_DIR"), "/saxpy_meta.rs")); // pub const SAXPY_KERNEL

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
