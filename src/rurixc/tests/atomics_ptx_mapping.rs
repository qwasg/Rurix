//! scoped atomics 的 PTX `atom.{order}.{scope}` 映射真跑测试骨架(M5.2,RXS-0080)。
//!
//! **D-406 禁区**:`atom.{order}.{scope}` 的内存序/作用域映射语义**由人工落笔**
//! (AI 仅条款化 RXS-0080 类型契约 + 挂本骨架,不实现 PTX 映射代码,M5 契约
//! D-M5-3 / M5_PLAN §2 任务 3)。本测试以 `#[ignore]` 占位,供人工解禁:
//!
//! 解禁条件(人工):
//! 1. `device_codegen` 落地 `Atomic`/`AtomicView` 原子操作 → satisfy morally strong
//!    的 `atom.{order}.{scope}` 指令降级(同 proxy / scope 双向包含 / 完全重叠,
//!    RXS-0080 Dynamic Semantics);
//! 2. 去除下方 `#[ignore]`,补「源 → NVPTX IR / PTX → `atom.*` 指令」真跑断言;
//! 3. Compute Sanitizer `racecheck` 运行期背书(M5 契约 G-M5-4),run URL 归档;
//! 4. 映射 PR 引用本测试 + RXS-0080 条款号。
//!
//! 在此之前,本骨架运行(`cargo test -- --ignored`)会 panic,提示映射尚未实现——
//! 防止 D-406 禁区被 AI 误标为"已完成"。
//!
//! **追踪**:PTX 映射实现登记为 **RD-008**(`registry/deferred.json`,owner M7,
//! D-406 人工落笔);backfill 时人工实现映射 codegen + Compute Sanitizer racecheck
//! 背书后解开本 `#[ignore]` 并补真跑断言,关闭 RD-008。M5 仅交付类型契约 + RX3010
//! + 本骨架(M5_PLAN §2 口径已更正:映射 codegen M5 期未交付)。

//@ spec: RXS-0080
#[test]
#[ignore = "D-406 禁区:scoped atomics 的 PTX atom.{order}.{scope} 映射由人工落笔(RXS-0080 / 契约 G-M5-4;追踪 RD-008)"]
fn scoped_atomics_ptx_atom_mapping_is_human_authored() {
    panic!(
        "D-406:scoped atomics 的 PTX atom.{{order}}.{{scope}} 映射尚未由人工实现(RXS-0080)。\
         AI 仅完成 scope 类型契约(RX3010)+ 本骨架;映射语义实现 + Compute Sanitizer \
         racecheck 背书(G-M5-4)为人工落笔项,落地时去除 #[ignore] 并补真跑断言。"
    );
}
