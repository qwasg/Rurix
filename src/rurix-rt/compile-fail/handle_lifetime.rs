//@ compile-fail — RXS-0140 句柄生命周期:frame/external 资源 affine（move-only），
//   move 后再用 → rustc E0382（use-after-move）。应被编译期拦截;若编译通过即红。
//   需 rurix-rt 以 --features d3d12-interop 构建（interop 模块）。
use rurix_rt::interop::scope;

fn main() {
    let _ = scope(0, [2, 2], [2, 2], |_cx, ready| {
        let _acquired = ready.wait()?; // 消费 ready（move 进 wait）
        let _again = ready.frame_index(); // ← use-after-move:E0382
        Ok(())
    });
}
