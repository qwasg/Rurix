//! VSM 虚拟阴影(报告3 P0–P1;RFC-0016 章 D)。
//!
//! 方向光 clipmap 栈(16K 虚拟 / 128×128 页);页标记(屏幕反馈)→ 页分配
//! (共享物理页池,非 sparse binding)→ 失效(灯/级联/图元)→ 多视图
//! shadow_depth_raster → 投影采样。页表为跨帧外部资源(不入 transient)。
//!
//! 本波落地(G5.3-D host 波,纯 Rust 零外部依赖,`forbid(unsafe_code)`):
//! - [`clipmap`][]:clipmap 栈与虚拟地址空间——级联配置/选级公式/灯正交基/
//!   toroidal 槽位换算(原点页粒度 snap 的环形更新语义);
//! - [`page_table`][]:32 位页表项位打包(物理页索引/驻留/脏/帧龄,P0 冻结
//!   阶段不变量,单测锁定)+ 单级 128×128 页表;
//! - [`pool`][]:共享物理页池(固定预算扁平深度存储,跨全部级);
//! - [`vsm`][]:host 系统——page_mark/page_alloc/invalidate 三 pass、多视图
//!   CPU 深度光栅、投影采样、帧流程与增量语义;device 侧 W3 统一接线,
//!   本模块为对拍金标准。

pub mod clipmap;
pub mod page_table;
pub mod pool;
pub mod vsm;
