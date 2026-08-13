//! OIT benchmark harness 测量面（G9.5 M120；RFC-0025 D4 伞形 §4.K；
//! spec/display_pipeline.md RXS-0371 逐条对齐）。
//!
//! - [`scene`] = canonical 场景（同场景同 overdraw 分布;整数 hash 闭式确定）;
//! - [`algorithms`] = nvpro `vk_order_independent_transparency` 七算法确定性
//!   参照（统一合成约定;排序 fallback 永保留 = 真值本体;linked-list 精确档
//!   与真值 diff=0）;
//! - [`measure`] = 七算法 × overdraw 档位阶梯的帧时/内存/质量误差测量
//!   （evidence 非空面;仅测量）;
//! - [`selection`] = 档位纪律（不定档 fail-closed NotMeasuredYet;无数据选型
//!   提交判 RED;精确档内存无界增长注入判 RED;精确档仅毛发 strand 作用域）。
//!
//! 纪律:host 纯 safe 确定性;零新 FFI;**本门只产 benchmark 数据,不做选型
//! 判定**(D4 D15);测量数据供 M114 毛发精确档裁决消费（strand 档承接锚:
//! M120 精确档 benchmark 裁决数据落地后重判,兜底 G9.7 穷举）。

pub mod algorithms;
pub mod measure;
pub mod scene;
pub mod selection;
