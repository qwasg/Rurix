//! scoped atomics 的 PTX `atom.{order}.{scope}` 映射真跑测试骨架(M5.2,RXS-0080)。
//!
//! **D-406 / RD-008 高敏面(deferred)**:`atom.{order}.{scope}` 的内存序/作用域映射
//! 语义为高敏面,**agent 可起草/实现,agent 自主落地**(AI 已条款化 RXS-0080
//! 类型契约 + 挂本骨架;映射本体随 RD-008 承接里程碑落地,M5 契约 D-M5-3 /
//! M5_PLAN §2 任务 3)。本测试以 `#[ignore]` 占位,供解禁:
//!
//! 解禁条件(落地时,agent 自主批准):
//! 1. `device_codegen` 落地 `Atomic`/`AtomicView` 原子操作 → satisfy morally strong
//!    的 `atom.{order}.{scope}` 指令降级(同 proxy / scope 双向包含 / 完全重叠,
//!    RXS-0080 Dynamic Semantics);
//! 2. 去除下方 `#[ignore]`,补「源 → NVPTX IR / PTX → `atom.*` 指令」真跑断言;
//! 3. Compute Sanitizer `racecheck` 运行期背书(M5 契约 G-M5-4),run URL 归档;
//! 4. 映射 PR 引用本测试 + RXS-0080 条款号。
//!
//! 在此之前,本骨架运行(`cargo test -- --ignored`)会 panic,提示映射尚未实现——
//! 防止 D-406 / RD-008 高敏面被误标为"已完成"。

//@ spec: RXS-0080
#[test]
#[ignore = "D-406 / RD-008 高敏面(deferred):scoped atomics 的 PTX atom.{order}.{scope} 映射经 owner 批准后落地(RXS-0080 / 契约 G-M5-4)"]
fn scoped_atomics_ptx_atom_mapping_is_deferred() {
    panic!(
        "D-406 / RD-008:scoped atomics 的 PTX atom.{{order}}.{{scope}} 映射尚未实现(RXS-0080)。\
         AI 仅完成 scope 类型契约(RX3010)+ 本骨架;映射语义实现 + Compute Sanitizer \
         racecheck 背书(G-M5-4)为 deferred 高敏面(agent 可落笔,经 owner 批准),\
         落地时去除 #[ignore] 并补真跑断言。"
    );
}
