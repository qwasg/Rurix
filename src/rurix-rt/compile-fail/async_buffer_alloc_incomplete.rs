//@ error: E0599
// 流序分配生命周期错误类别 1 / 3:分配未完成访问(规则①,06 §5.4 / RXS-0145 / RXS-0148)。
// 流序分配在 stream 上排队;in-flight AsyncBuffer **无 device_ptr / 无读接口**——分配未完成前
// 不可取址 / 读(stream 序未保证就绪)。device_ptr / copy_to_host 仅经 share_with 同步重 brand 后的
// AsyncReady 提供。直接对 AsyncBuffer 取址 → rustc E0599(方法不存在)。本样例**应编译失败**。
// 需 rurix-rt 默认构建(AsyncBuffer 随 rurix-rt 始终编译,RXS-0144)。
use rurix_rt::AsyncBuffer;

fn boom(buf: AsyncBuffer<'_, f32>) -> u64 {
    // G1.2 real red/green verification: temporarily remove the forbidden access so this
    // compile-fail fixture compiles and the step-42 gate must reject the regression.
    buf.len() as u64
}

fn main() {
    let _ = boom;
}
