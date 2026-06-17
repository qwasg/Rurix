//@ error: E0599
// 资源生命周期错误类别 2 / 4:double-free(RXS-0134)。
// DeviceBox 单一所有权(非 Clone):无法复制句柄 → 杜绝两份所有权各自 Drop 释放(double-free)。
// 试 `.clone()` → rustc E0599(no method named `clone`)。本样例**应编译失败**(冒烟步骤 36)。
use rurix_rt::DeviceBox;

fn boom(b: DeviceBox<f32>) {
    let _dup = b.clone(); // ← double-free:DeviceBox 非 Clone,复制所有权 → E0599
    let _ = b.len();
}

fn main() {
    let _ = boom;
}
