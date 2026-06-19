//@ error: E0382
// 流序分配生命周期错误类别 2 / 3:释放后访问(规则②,06 §5.4 / RXS-0146 / RXS-0148)。
// AsyncBuffer 为 affine move-only 资源(非 Copy / 非 Clone,单一所有权);share_with 消费(move)它。
// move 后再用 → rustc E0382(use-after-free 类别;Drop = cuMemFreeAsync 流序释放单点,不双重释放)。
// 本样例**应编译失败**。需 rurix-rt 默认构建(AsyncBuffer 随 rurix-rt 始终编译,RXS-0144)。
use rurix_rt::{AsyncBuffer, SharedEvent, SharedStream};

fn boom(buf: AsyncBuffer<'_, f32>, other: &SharedStream, ev: &SharedEvent) {
    let _ready = buf.share_with(other, ev); // 所有权 move 入 share_with(资源在此被消费/转移)
    let _ = buf.len(); // ← E0382: use-after-free(move 后再用)
}

fn main() {
    let _ = boom;
}
