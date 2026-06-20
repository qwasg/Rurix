# 03 · Resource lifetimes

[English](03_resources.en.md) · [简体中文](03_resources.md)

> API-convergence notice (RD-008): the syntax may change between versions. This is a **walkthrough chapter**: the runtime resource types need a package context and a real device, so the code is shown as reference snippets, linked to existing conformance corpora (already constrained by the parse gate) and to the real `src/uc02-demo` demo.

Rurix turns GPU resource-lifetime errors from **runtime crashes** into **compile errors**. This is its most fundamental safety gain over CUDA C++.

## Affine resource types

`Context`, `Stream`, `Event`, and `Buffer` are all **affine types** (move-only, RAII): once moved or destroyed, they cannot be used again. Combined with **context-brand lifetimes**, the compiler can statically reject an entire class of lifetime bugs.

Reference snippet (a complete, parseable sample is in [`conformance/syntax/buffers_context.rx`](../conformance/syntax/buffers_context.rx)):

```rurix
fn setup() -> Result<(), Error> {
    let dev: Device = Device::enumerate()?.first()?;
    let ctx: Context = dev.create_context()?;
    let stream: Stream<'_> = ctx.create_stream()?;
    let buf: DeviceBuffer<f32> = ctx.alloc::<f32>(1 << 20)?;
    let pinned: PinnedBuffer<f32> = ctx.alloc_pinned::<f32>(1 << 20)?;
    stream.copy_to_device(&pinned, &buf)?;
    stream.synchronize_and_destroy()?;
    Ok(())
}
```

- `Device::enumerate()` is driven by real device probing (strict-only: capability bits come from measurement, with no silent degradation).
- The `Stream<'_>` produced by `ctx.create_stream()` carries a brand lifetime that binds the stream to that context; the buffer from `alloc` carries the same brand.
- `?` propagates the `Result`; resources are reclaimed by affine rules at the end of scope or at an explicit `synchronize_and_destroy()`.

## Four error classes intercepted at compile time

UC-02 (the three-stream overlapped pipeline) turns the following four resource-lifetime error classes **all into compile errors**:

| Error class | Status quo (CUDA C++) | Rurix |
|---|---|---|
| use-after-free (released too early after a stream-ordered allocation, then used) | blows up at runtime | compile error (affine, already moved) |
| double-free | undefined at runtime | compile error |
| cross-thread misuse (e.g. cross-thread `cuCtxDestroy`) | blows up at runtime | compile error (brand + `Send` constraint) |
| cross-stream unsynchronized (reading a not-yet-ready result) | data race | compile error (event / typestate) |

## A real demo: the UC-02 three-stream pipeline

The end-to-end implementation of flagship use case UC-02 is in [`src/uc02-demo`](../src/uc02-demo). It demonstrates:

- **Single-threaded three-stream overlap**: the three `Stream`s for H2D upload / compute / D2H download overlap via events (`record_event` / `wait_event`).
- **Typestate pipelining**: `InFlight` → `acquire` (wait on the event, re-brand) → use → `commit` (record the event), which keeps "read before synchronized" out at compile time.
- **Cross-thread ownership transfer**: a device buffer moves to a worker thread along with a `Send` context, and the cross-thread lifetime is guaranteed by the type system.

> To see the "violate it and get a compile error" counterexamples, look at the reject corpora such as `conformance/launch/reject/`: deliberately wrong samples come with the expected diagnostic code.

---

Advanced chapters (the details of views/address spaces, shared memory/stdlib, a line-by-line walkthrough of the UC-02 pipeline, and UC-03 SPH + soft rasterizer) will be filled in by later PRs.

In-depth reference (Chinese-only): `spec/pipeline.md` (stream/event/ownership semantics), [`05_LANGUAGE_ARCHITECTURE.md`](../05_LANGUAGE_ARCHITECTURE.md) (affine resources & brand lifetimes), [`08_RUNTIME_AND_TOOLING.md`](../08_RUNTIME_AND_TOOLING.md) (execution resources).
