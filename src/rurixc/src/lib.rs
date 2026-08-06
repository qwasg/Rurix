//! rurixc — Rurix 编译器(D-201,07 号文档总体管线)。
//!
//! M1.1 范围:诊断地基(契约 D-M1-1)——`Span`/`SourceMap`/`DiagCtxt` 与
//! message-key 骨架,先于 lexer 落地(r1 顺序,07 §5)。
//! M1.2 范围:lexer + 词法条款(契约 D-M1-2,RXS-0001 ~ RXS-0010)。
//! M1.3 范围:parser/AST/feature gate(契约 D-M1-3,RXS-0011 ~ RXS-0031)。
//! M1.4 范围:诊断渲染/UI golden 通道/rx fmt 雏形(契约 D-M1-4 / D-M1-5)。

pub mod ast;
// G8.2 M31(RXS-0304):绑定推导律上提为恒编入——reflection v1(`--emit=reflection`,
// 默认构建)与 dxil/vulkan 编码路复用同一推导事实源(`infer_spirv_bindings_vk_native`);
// 默认 codegen 路径对本模块的调用面零漂移(仅 dxil/vulkan/ reflection 消费)。
pub mod binding_layout;
// G8.2 M32(RXS-0311~0313):capability ID 闭集/`#[requires]`/调用图并集/profile
// 选择律与 fallback(`--profile` + `--emit=capabilities`)+ `verify_profile_snapshot`。
pub mod borrow_check;
#[cfg(feature = "shader-stages")]
pub mod capability_check;
pub mod codegen;
pub mod coloring;
pub mod const_eval;
pub mod dataflow;
pub mod device_codegen;
pub mod diag;
pub mod driver;
pub mod drop_elab;
#[cfg(feature = "dxil-backend")]
pub mod dxil_codegen;
#[cfg(feature = "dxil-backend")]
pub mod dxil_sig_gate;
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
pub mod dxil_spirv;
pub mod export_c;
pub mod feature_gate;
pub mod fmt;
pub mod hir;
// G8.2 M31(RXS-0304):AST 签名面无损提取层(自 mir_build::dxil_io 机械搬迁),
// 供 device MIR 附着与 reflection v1 复用同一提取律。
#[cfg(feature = "shader-stages")]
pub mod iface_extract;
pub mod launch_check;
pub mod lexer;
pub mod lossless;
pub mod lower;
// G8.2 M85(RXS-0317~0318):shader/PSO manifest v1 canonical merge/dedup/coverage
// (`--merge-manifests` / `--assemble-manifest`;`--phase g8.2` host 门)。
#[cfg(feature = "shader-stages")]
pub mod manifest;
pub mod messages;
pub mod mir;
pub mod mir_build;
pub mod mod_assembly;
pub mod move_check;
pub mod parser;
// G8.2 M29(RXS-0308~0310):permutation 域/canonical key/裁剪预算报告
// (`--emit=permutations` + `--permutation-budget`/`--permutation-select`)。
#[cfg(feature = "shader-stages")]
pub mod permutation;
pub mod profile;
pub mod ptxas;
pub mod query;
pub mod ray_query_check;
// G8.2 M31(RXS-0304~0307):reflection v1 与 interface hash(`--emit=reflection`)。
#[cfg(feature = "shader-stages")]
pub mod reflection;
pub mod render;
pub mod resolve;
#[cfg(feature = "shader-stages")]
pub mod shader_stages;
pub mod shared_check;
pub mod source_map;
pub mod span;
pub mod tbir;
pub mod tbir_build;
pub mod test_harness;
pub mod toolchain;
pub mod tooling;
pub mod ty;
pub mod typeck;
pub mod views_check;
#[cfg(feature = "vulkan-backend")]
pub mod vulkan_codegen;
