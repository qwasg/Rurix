//@ error: E0599
// 资源生命周期错误类别 3 / 4:跨 stream 未同步访问(RXS-0132 流序分配类型化 / RXS-0134)。
// InFlight<T>(某 stream 上排队异步操作的在途缓冲)**无任何读接口**;读/跨 stream 操作的
// `copy_to_host` 仅 DeviceBox 提供。必经 SharedStream::acquire(内插 cuStreamWaitEvent)重 brand
// 回 DeviceBox 方可读——跳过同步直接读 InFlight → rustc E0599(方法不存在)。本样例**应编译失败**。
use rurix_rt::InFlight;

fn boom(inflight: InFlight<f32>, dst: &mut [f32]) {
    // ← 跨 stream 未同步读:InFlight 无 copy_to_host(须先 SharedStream::acquire)→ E0599
    inflight.copy_to_host(dst);
}

fn main() {
    let _ = boom;
}
