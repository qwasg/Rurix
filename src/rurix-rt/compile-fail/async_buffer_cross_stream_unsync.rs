//@ error: E0599
// 流序分配生命周期错误类别 3 / 3:跨 stream 未同步访问(规则③,06 §5.4 / RXS-0147 / RXS-0148）。
// AsyncBuffer<'stream, T> 携不变 'stream brand;跨 stream 使用**必须** buf.share_with(other, event)
// 显式建立时序边(cuEventRecord + cuStreamWaitEvent)重 brand 到 'other 的可读 AsyncReady——跳过
// share_with 直接读 AsyncBuffer → copy_to_host 不存在 → rustc E0599(方法不存在)。本样例**应编译失败**。
// 需 rurix-rt 默认构建(AsyncBuffer 随 rurix-rt 始终编译,RXS-0144)。
use rurix_rt::AsyncBuffer;

fn boom(buf: AsyncBuffer<'_, f32>, dst: &mut [f32]) {
    // ← 跨 stream 未同步:缺 buf.share_with(other_stream, event) 重 brand,无可读句柄 → E0599
    buf.copy_to_host(dst);
}

fn main() {
    let _ = boom;
}
