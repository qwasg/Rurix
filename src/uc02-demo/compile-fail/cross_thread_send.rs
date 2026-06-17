//@ error: E0277
// 资源生命周期错误类别 4 / 4:跨线程非法转移(RXS-0133 / RXS-0134)。
// SharedStream(及线程绑定守卫 Bound / 单线程 Context)为 !Send(裸句柄,current context 线程
// 绑定):送入另一线程 → rustc E0277(cannot be sent between threads safely)。可跨线程的仅
// SharedContext(Send+Sync)/ DeviceBox(Send)/ SharedEvent(Send)。本样例**应编译失败**。
use rurix_rt::SharedStream;

fn boom(stream: SharedStream) {
    std::thread::spawn(move || {
        // ← 跨线程非法转移:SharedStream 为 !Send,move 入 worker 线程后使用 → E0277
        let _ = stream.synchronize();
    });
}

fn main() {
    let _ = boom;
}
