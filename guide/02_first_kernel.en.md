# 02 · Your first kernel

[English](02_first_kernel.en.md) · [简体中文](02_first_kernel.md)

> API-convergence notice (RD-008): the syntax may change between versions. The kernel-definition example in this chapter is exercised live by the `rx check` CI gate; the launch and runtime parts are reference snippets (they need a package context / runtime, and link to existing conformance corpora).

The classic introductory kernel: **SAXPY**, i.e. `y = a·x + y`.

## Kernel definition

Single source of truth: [`conformance/tutorial/04_first_kernel.rx`](../conformance/tutorial/04_first_kernel.rx)

```rurix
kernel fn saxpy<const N: usize>(
    grid: Grid<(N,)>,
    a: f32,
    x: View<global, f32, (N,)>,
    y: ViewMut<global, f32, (N,)>,
) {
    let i = grid.thread_index();
    y[i] = a * x[i] + y[i];
}
```

A line-by-line breakdown — these three pieces are the core of a Rurix kernel:

- **`kernel fn`**: the GPU-entry coloring (RXS-0014).
- **`Grid<(N,)>`**: a **typed execution shape**. `(N,)` is a one-dimensional grid, and `grid.thread_index()` gives the current thread's index. The execution hierarchy (grid/block/thread) is a first-class citizen of the type system, not a runtime convention.
- **`View<global, f32, (N,)>` / `ViewMut<global, f32, (N,)>`**: views into device memory. The three type parameters are, in order, the **address space** (`global`), the **element type** (`f32`), and the **shape** (`(N,)`). The address space is a **type parameter**, not a runtime value — misusing memory across address spaces becomes a compile error. `View` is read-only; `ViewMut` is writable.
- `<const N: usize>`: a const generic, making the kernel generic over the grid size.

Run it through the static check (the file contains a minimal `main`, so it also works with `rx check` / `rx run`):

```sh
cargo run -p rx -- check conformance/tutorial/04_first_kernel.rx   # exit 0
```

## Launch (reference snippet)

Actually dispatching the kernel onto a stream needs a **runtime context** (`Stream` / `Buffer`, etc.); that part needs a package context to resolve, so it is shown as a reference snippet. A complete, parseable sample is in [`conformance/launch/accept/saxpy_launch.rx`](../conformance/launch/accept/saxpy_launch.rx):

```rurix
kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>, n: usize, t: ThreadCtx<1>) {
    let i = t.global_id();
    if i < n {
        out[i] = x[i];
    }
}

fn run<C>(stream: Stream<C>, out: Buffer<C, f32>, x: Buffer<C, f32>, n: usize) {
    stream.launch(saxpy, GridDim(n), BlockDim(n), (out, x, n));
}
```

> Note: in the conformance corpus a kernel has two equivalent ways to spell its execution shape — `Grid<(N,)>` + `grid.thread_index()` (the tutorial example above) and `ThreadCtx<1>` + `t.global_id()` (this launch sample). Pick one while learning; **do not mix them**.

The launch type contract (grid/block dimension consistency, argument element-type matching, a single context brand) is checked statically by the compiler (RXS-0074 launch type contract / RXS-0075 launch diagnostic requirements). The `C` in `Stream<C>` / `Buffer<C, f32>` is a **context brand** that binds the stream and the buffer to the same context — and that is exactly the foundation for the resource-lifetime safety of the next chapter.

---

Next → [03 Resource lifetimes](03_resources.en.md)

In-depth reference (Chinese-only): `spec/device.md` (kernel coloring / address spaces / execution shapes), `spec/types.md` (the View family); the launch contract is RXS-0074/0075 in `spec`.
