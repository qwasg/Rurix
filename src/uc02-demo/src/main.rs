//! UC-02 旗舰 demo(M8.3,D-M8-3 / G-M8-3;02 §U2 / 01 §6):**三 stream 重叠流水线 +
//! 跨线程所有权转移**,复用 `rurix-rt` affine 运行时对象(spec/pipeline.md RXS-0130~0134)。
//!
//! - **Part A**:单线程三 stream(H2D / compute / D2H)重叠流水线——pinned staging 异步
//!   上传 → `InFlight` 在途(流序分配类型化,RXS-0132)→ compute stream `acquire`(`wait`
//!   h2d 事件)重 brand → launch → `commit`(record compute 事件)→ D2H `download`(`wait`
//!   compute 事件 + 异步回拷)。event 流序依赖编排(RXS-0131),N chunk 重叠。
//! - **Part B**:跨线程所有权转移(RXS-0133)——main 线程上传 + launch + record 事件,把
//!   `DeviceBox`(Send)+ `SharedEvent`(Send)`move` 给 worker 线程;worker `bind` 重绑
//!   primary context 后 `wait` 事件 + 读回。
//!
//! 资源生命周期错误类别(use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移)
//! **100% 编译期拦截**(reject 样例见 `compile-fail/*.rs`,冒烟步骤 36 核对,RXS-0134)。
//! 全 safe(`unsafe_code=deny`):仅用 `rurix-rt` safe API,FFI unsafe 归 rurix-rt。

use std::ffi::c_void;
use std::process::ExitCode;

use rurix_rt::{Context, CudaError, SharedContext};

/// 手写 scale PTX(`y[i] = a*y[i] + b`,单缓冲原地仿射;mul.rn+add.rn 与 host f32 两步舍入
/// 逐位一致;`.version 8.0` 协商起点,驱动不支持时 rurix-rt 自动降版,08 §2.4)。
const SCALE_PTX: &str = r#".version 8.0
.target sm_89
.address_size 64

.visible .entry scale(
    .param .u64 p_y,
    .param .f32 p_a,
    .param .f32 p_b,
    .param .u32 p_n
)
{
    .reg .pred  %p1;
    .reg .b32   %r<6>;
    .reg .f32   %f<5>;
    .reg .b64   %rd<5>;

    ld.param.u64    %rd1, [p_y];
    ld.param.f32    %f1,  [p_a];
    ld.param.f32    %f2,  [p_b];
    ld.param.u32    %r1,  [p_n];

    mov.u32         %r2, %ctaid.x;
    mov.u32         %r3, %ntid.x;
    mov.u32         %r4, %tid.x;
    mad.lo.s32      %r5, %r2, %r3, %r4;

    setp.ge.u32     %p1, %r5, %r1;
    @%p1 bra        DONE;

    cvta.to.global.u64  %rd2, %rd1;
    mul.wide.u32        %rd3, %r5, 4;
    add.s64             %rd4, %rd2, %rd3;

    ld.global.f32   %f3, [%rd4];
    mul.rn.f32      %f4, %f1, %f3;
    add.rn.f32      %f4, %f4, %f2;
    st.global.f32   [%rd4], %f4;

DONE:
    ret;
}
"#;

const N: usize = 4096; // 每 chunk 元素数
const CHUNKS: usize = 4; // 流水线 chunk 数(重叠深度)
const A: f32 = 2.5;
const B: f32 = -1.0;
const TOL: f32 = 1e-4;

/// host 参考(两步舍入,与 PTX mul.rn+add.rn 逐位一致)。
fn host_expect(v: f32) -> f32 {
    A * v + B
}

/// GPU 是否可用(无驱动 / 无设备 → SKIP)。
fn gpu_available() -> bool {
    matches!(Context::device_count(), Ok(n) if n > 0)
}

/// scale kernel launch 实参(按 kernel 形参顺序:p_y:u64, p_a:f32, p_b:f32, p_n:u32)。
fn scale_params(py: &mut u64, a: &mut f32, b: &mut f32, n: &mut u32) -> [*mut c_void; 4] {
    [
        (&raw mut *py).cast::<c_void>(),
        (&raw mut *a).cast::<c_void>(),
        (&raw mut *b).cast::<c_void>(),
        (&raw mut *n).cast::<c_void>(),
    ]
}

/// Part A:单线程三 stream 重叠流水线(H2D / compute / D2H + event 流序依赖 + 流序分配
/// 类型化 `InFlight`,RXS-0131/0132)。返回逐元素最大绝对误差。
fn part_a_overlap() -> Result<f32, CudaError> {
    let shared = SharedContext::from_primary(0)?;
    let bound = shared.bind()?;
    let h2d = bound.create_stream()?;
    let compute = bound.create_stream()?;
    let d2h = bound.create_stream()?;
    let module = bound.load_module(SCALE_PTX)?;
    let kernel = module.function("scale")?;

    let block = 256u32;
    let grid = (N as u32).div_ceil(block);
    let mut max_err = 0f32;

    for c in 0..CHUNKS {
        let input: Vec<f32> = (0..N).map(|i| (c * N + i) as f32 * 0.25 - 3.0).collect();

        // H2D:pinned staging → 异步上传 → 在途缓冲(InFlight,流序分配类型化)
        let dy = bound.alloc::<f32>(N)?;
        let mut pin_in = bound.alloc_pinned::<f32>(N)?;
        pin_in.as_mut_slice().copy_from_slice(&input);
        let evt_h2d = bound.create_event()?;
        let inflight = h2d.upload(dy, pin_in, evt_h2d)?;

        // compute:acquire(wait h2d 事件)→ 重 brand 回 DeviceBox → launch → commit(record)
        let (dy, staging) = compute.acquire(inflight)?;
        let mut py = dy.device_ptr();
        let (mut a, mut b, mut n) = (A, B, N as u32);
        let mut params = scale_params(&mut py, &mut a, &mut b, &mut n);
        compute.launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)?;
        let evt_compute = bound.create_event()?;
        let inflight = compute.commit(dy, staging, evt_compute)?;

        // D2H:download(wait compute 事件 + 异步回拷 + 同步)
        let pin_out = bound.alloc_pinned::<f32>(N)?;
        let pin_out = d2h.download(inflight, pin_out)?;

        for (i, &got) in pin_out.as_slice().iter().enumerate() {
            max_err = max_err.max((got - host_expect(input[i])).abs());
        }
    }
    bound.synchronize()?;
    Ok(max_err)
}

/// Part B:跨线程所有权转移(RXS-0133)——main 上传 + launch + record 事件,把 `DeviceBox`
/// + `SharedEvent` `move` 给 worker 线程;worker `bind` 重绑后 `wait` 事件 + 读回。返回最大误差。
fn part_b_cross_thread() -> Result<f32, CudaError> {
    let shared = SharedContext::from_primary(0)?;
    let input: Vec<f32> = (0..N).map(|i| i as f32 * 0.5 + 1.0).collect();

    let bound = shared.bind()?;
    let stream = bound.create_stream()?;
    let module = bound.load_module(SCALE_PTX)?;
    let kernel = module.function("scale")?;

    let mut dy = bound.alloc::<f32>(N)?;
    dy.copy_from_host(&input)?; // 同步 H2D
    let mut py = dy.device_ptr();
    let (mut a, mut b, mut n) = (A, B, N as u32);
    let mut params = scale_params(&mut py, &mut a, &mut b, &mut n);
    let block = 256u32;
    let grid = (N as u32).div_ceil(block);
    stream.launch(&kernel, [grid, 1, 1], [block, 1, 1], &mut params)?;
    let evt = bound.create_event()?;
    stream.record_event(&evt)?;

    // 跨线程转移:DeviceBox(Send)+ SharedEvent(Send)+ SharedContext(Clone, Send)move 入 worker。
    let shared_w = shared.clone();
    let out: Vec<f32> = std::thread::spawn(move || -> Result<Vec<f32>, CudaError> {
        let bound_w = shared_w.bind()?; // worker 重绑 current primary context
        let stream_w = bound_w.create_stream()?;
        stream_w.wait_event(&evt)?; // 跨 stream / 跨线程流序依赖
        stream_w.synchronize()?;
        let mut out = vec![0f32; N];
        dy.copy_to_host(&mut out)?; // dy 已 move 入本线程,读回
        Ok(out)
    })
    .join()
    .expect("worker 线程 join")?;

    let mut max_err = 0f32;
    for (i, &got) in out.iter().enumerate() {
        max_err = max_err.max((got - host_expect(input[i])).abs());
    }
    Ok(max_err)
}

fn main() -> ExitCode {
    if !gpu_available() {
        println!("UC02_RESULT: skip (no GPU/driver available)");
        return ExitCode::SUCCESS;
    }
    let a_err = match part_a_overlap() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("UC02_RESULT: error part_a: {e}");
            return ExitCode::from(2);
        }
    };
    let b_err = match part_b_cross_thread() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("UC02_RESULT: error part_b: {e}");
            return ExitCode::from(2);
        }
    };
    let ok = a_err <= TOL && b_err <= TOL;
    println!(
        "UC02_RESULT: {} chunks={CHUNKS} n={N} part_a_overlap_max_err={a_err:.6} \
         part_b_cross_thread_max_err={b_err:.6} tol={TOL:.6}",
        if ok { "ok" } else { "fail" }
    );
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}
