//@ compile-fail — RXS-0142 信号时序:消费式 typestate 下未 signal 即试 present。
//   `present` 仅 PresentableFrame 提供;AcquiredFrame 无 `present` 方法 → rustc E0599。
//   跳过 signal 即无可 present 句柄,信号时序违例编译期拦截;若编译通过即红。
//   需 rurix-rt 以 --features d3d12-interop 构建。
use rurix_rt::interop::scope;

fn main() {
    let _ = scope(0, [2, 2], [2, 2], |_cx, ready| {
        let acquired = ready.wait()?; // Ready → Acquired（已 wait 2n）
        let _ = acquired.present(); // ← 未 signal 即 present:AcquiredFrame 无 present,E0599
        Ok(())
    });
}
