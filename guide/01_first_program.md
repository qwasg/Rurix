# 01 · 第一个程序

> API 收敛期(RD-008):语法可能随版本变化。本章三个示例均经 CI 门 `rx check` + `rx run` 真跑。

本章在 host 侧把语言基础走一遍:程序入口、绑定、控制流、结构体、函数着色。GPU kernel 留到[下一章](02_first_kernel.md)。

## 1. Hello, Rurix

唯一事实源:[`conformance/tutorial/01_hello.rx`](../conformance/tutorial/01_hello.rx)

```rurix
fn main() {
    let greeting = "你好,Rurix";
    println(greeting);
}
```

`fn main()` 是可执行目标的入口(缺 `main` 的文件 `rx check` 会报 `RX6002: no main function found for executable target`)。`let` 引入不可变绑定,字符串字面量与 `println` 即可输出。

```sh
cargo run -p rx -- check conformance/tutorial/01_hello.rx       # 仅静态检查,退出 0
cargo run -p rx -- run   conformance/tutorial/01_hello.rx -o build/hello.exe   # 构建并执行
```

`rx check` 走完整前端(借用/资源/类型),但不产 codegen——这是你写代码时最快的反馈回路。`rx run` 透传产物退出码(RXS-0085)。

## 2. 绑定、控制流、结构体

唯一事实源:[`conformance/tutorial/02_host_basics.rx`](../conformance/tutorial/02_host_basics.rx)

```rurix
struct Point {
    x: f32,
    y: f32,
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let origin = Point { x: 0.0, y: 0.0 };
    let mut total = 0;
    for i in 0..5 {
        total = add(total, i);
    }
    if total > 3 {
        println("total 偏大");
    } else {
        println("total 偏小");
    }
}
```

要点(对 Rust 用户应当眼熟):

- `struct` 定义聚合类型,字面量用 `Point { x: .., y: .. }`。
- 绑定默认不可变;要重新赋值需 `let mut`。
- `for i in 0..5` 遍历区间;`if/else` 是表达式式控制流。
- 函数体最后一个表达式(无分号)即返回值(`a + b`)。

## 3. 函数着色:host / device / const

唯一事实源:[`conformance/tutorial/03_fn_colors.rx`](../conformance/tutorial/03_fn_colors.rx)

Rurix 用**函数着色**(RXS-0014)区分代码在哪一侧执行,但共享同一套类型系统与模块系统——不像 CUDA C++ 那样 host/device 各一套:

```rurix
device fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

const fn inc(n: usize) -> usize {
    n + 1
}

fn main() {
    let mid = lerp(0.0, 10.0, 0.5);
    println("device fn 可从 host 调用");
}
```

- **`fn`**(host):普通宿主函数。
- **`device fn`**:设备侧函数,**单向**可被 host 与 kernel 调用(避免 CUDA `__host__ __device__` 的组合爆炸)。
- **`const fn`**:编译期可求值。
- **`kernel fn`**:GPU 入口,见[下一章](02_first_kernel.md)。

四种着色并排的完整样例见 [`conformance/syntax/fn_colors.rx`](../conformance/syntax/fn_colors.rx)。

---

下一步 → [02 第一个 kernel](02_first_kernel.md)

深入参考:`spec/lexical.md`(标识符/关键字/字面量,RXS-0004~)、`spec/names.md`(函数项与着色,RXS-0014)。
