//@ error: E0382
// 资源生命周期错误类别 1 / 4:use-after-free(RXS-0134)。
// DeviceBox 为 affine move-only 资源(非 Copy):move 后再用 → rustc E0382。
// 编译期拦截「释放后使用」——本样例**应编译失败**(冒烟步骤 36 断言;放行即红,反 YAML-only)。
use rurix_rt::DeviceBox;

fn sink(_b: DeviceBox<f32>) {}

fn boom(b: DeviceBox<f32>) {
    sink(b); // 所有权 move 入 sink(资源在此被消费/释放)
    let _ = b.len(); // ← use-after-free:b 已 move,此处再用 → E0382
}

fn main() {
    let _ = boom;
}
