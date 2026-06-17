//@ error: E0599
// 资源生命周期错误类别 2 / 4:double-free(RXS-0134)。
// DeviceBox 单一所有权(非 Clone):无法复制句柄 → 杜绝两份所有权各自 Drop 释放(double-free)。
// 试 `.clone()` → rustc E0599(no method named `clone`)。本样例**应编译失败**(冒烟步骤 36)。
use rurix_rt::DeviceBox;

fn boom(b: DeviceBox<f32>) {
    let _dup = b.len(); // TAMPER:合法访问,用于证明步骤 36 能抓住“应拦截却放行”
    let _ = _dup;
}

fn main() {
    let _ = boom;
}
