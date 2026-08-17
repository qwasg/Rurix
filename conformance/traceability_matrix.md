# spec 条款 ↔ 测试锚定矩阵(生成物,勿手改)

> 生成:`py -3 ci/trace_matrix.py`(G-M1-4;每条款 ≥1 测试锚定,10 §4)。

| 条款 | spec 文件 | 锚定测试数 | 锚定 |
|---|---|---|---|
| RXS-0001 | spec/lexical.md | 1 | `src/rurixc/src/lexer.rs` |
| RXS-0002 | spec/lexical.md | 1 | `conformance/syntax/comments.rx` |
| RXS-0003 | spec/lexical.md | 2 | `conformance/syntax/comments.rx`, `conformance/syntax/comments_between_items.rx` |
| RXS-0004 | spec/lexical.md | 4 | `conformance/syntax/fn_basic.rx`, `conformance/syntax/hello_world.rx`, `conformance/syntax/idents_keywords.rx` …(+1) |
| RXS-0005 | spec/lexical.md | 29 | `conformance/syntax/atomics_sync.rx`, `conformance/syntax/buffers_context.rx`, `conformance/syntax/closures_and_calls.rx` …(+26) |
| RXS-0006 | spec/lexical.md | 5 | `conformance/syntax/buffers_context.rx`, `conformance/syntax/const_generics.rx`, `conformance/syntax/literals_int.rx` …(+2) |
| RXS-0007 | spec/lexical.md | 2 | `conformance/syntax/literals_float.rx`, `conformance/syntax/vec_mat_swizzle.rx` |
| RXS-0008 | spec/lexical.md | 9 | `conformance/syntax/buffers_context.rx`, `conformance/syntax/export_c.rx`, `conformance/syntax/ffi_extern.rx` …(+6) |
| RXS-0009 | spec/lexical.md | 12 | `conformance/syntax/atomics_sync.rx`, `conformance/syntax/closures_and_calls.rx`, `conformance/syntax/control_flow.rx` …(+9) |
| RXS-0010 | spec/lexical.md | 1 | `src/rurixc/src/lexer.rs` |
| RXS-0011 | spec/syntax.md | 5 | `conformance/syntax/comments_between_items.rx`, `conformance/syntax/items_mix.rx`, `src/rurixc/src/parser.rs` …(+2) |
| RXS-0012 | spec/syntax.md | 6 | `conformance/syntax/attrs_meta.rx`, `conformance/syntax/attrs_on_items.rx`, `conformance/syntax/export_handles.rx` …(+3) |
| RXS-0013 | spec/syntax.md | 5 | `conformance/syntax/paths_expr.rx`, `conformance/syntax/turbofish_nested.rx`, `conformance/syntax/visibility_levels.rx` …(+2) |
| RXS-0014 | spec/syntax.md | 12 | `conformance/syntax/const_fn_eval.rx`, `conformance/syntax/device_math_chain.rx`, `conformance/syntax/fn_colors.rx` …(+9) |
| RXS-0015 | spec/syntax.md | 4 | `conformance/syntax/enum_payloads.rx`, `conformance/syntax/struct_generic_where.rx`, `conformance/syntax/struct_tuple_unit.rx` …(+1) |
| RXS-0016 | spec/syntax.md | 7 | `conformance/syntax/impl_inherent_methods.rx`, `conformance/syntax/lifetimes_in_impls.rx`, `conformance/syntax/result_chain_host.rx` …(+4) |
| RXS-0017 | spec/syntax.md | 3 | `conformance/syntax/mod_nested.rx`, `conformance/syntax/use_alias.rx`, `src/rurixc/src/parser.rs` |
| RXS-0018 | spec/syntax.md | 4 | `conformance/syntax/const_fn_eval.rx`, `conformance/syntax/static_mut.rx`, `conformance/syntax/type_alias_generic.rx` …(+1) |
| RXS-0019 | spec/syntax.md | 4 | `conformance/syntax/export_handles.rx`, `conformance/syntax/extern_pub_fn.rx`, `src/rurixc/src/parser.rs` …(+1) |
| RXS-0020 | spec/syntax.md | 9 | `conformance/syntax/fn_where_ret.rx`, `conformance/syntax/generics_const_params.rx`, `conformance/syntax/generics_defaults.rx` …(+6) |
| RXS-0021 | spec/syntax.md | 8 | `conformance/syntax/const_args_forms.rx`, `conformance/syntax/generics_const_params.rx`, `conformance/syntax/generics_shr_split.rx` …(+5) |
| RXS-0022 | spec/syntax.md | 9 | `conformance/syntax/kernel_views_generic.rx`, `conformance/syntax/shape_tuples.rx`, `conformance/syntax/types_addrspace_contextual.rx` …(+6) |
| RXS-0023 | spec/syntax.md | 6 | `conformance/syntax/patterns_at_bindings.rx`, `conformance/syntax/patterns_literals_ranges.rx`, `conformance/syntax/patterns_refs_slices.rx` …(+3) |
| RXS-0024 | spec/syntax.md | 6 | `conformance/syntax/blocks_as_values.rx`, `conformance/syntax/fn_nested_items.rx`, `conformance/syntax/let_without_init.rx` …(+3) |
| RXS-0025 | spec/syntax.md | 8 | `conformance/syntax/expr_assign_compound.rx`, `conformance/syntax/expr_precedence.rx`, `conformance/syntax/expr_ranges.rx` …(+5) |
| RXS-0026 | spec/syntax.md | 11 | `conformance/syntax/blocks_as_values.rx`, `conformance/syntax/expr_arrays_repeat.rx`, `conformance/syntax/expr_attr_prefixed.rx` …(+8) |
| RXS-0027 | spec/syntax.md | 9 | `conformance/syntax/calls_methods_chained.rx`, `conformance/syntax/device_math_chain.rx`, `conformance/syntax/index_field_tuple.rx` …(+6) |
| RXS-0028 | spec/syntax.md | 4 | `conformance/syntax/expr_return_break_values.rx`, `conformance/syntax/if_else_chains.rx`, `conformance/syntax/loops_while_for.rx` …(+1) |
| RXS-0029 | spec/syntax.md | 6 | `conformance/syntax/match_block_arms.rx`, `conformance/syntax/match_empty_and_nested.rx`, `conformance/syntax/match_guards.rx` …(+3) |
| RXS-0030 | spec/syntax.md | 4 | `src/rurixc/src/lossless.rs`, `src/rurixc/src/parser.rs`, `tests/ui/parse/missing_semi.rx` …(+1) |
| RXS-0031 | spec/syntax.md | 5 | `conformance/syntax/feature_gate_closures.rx`, `src/rurixc/src/feature_gate.rs`, `src/rurixc/src/parser.rs` …(+2) |
| RXS-0032 | spec/names.md | 7 | `conformance/resolve/block_items.rx`, `conformance/resolve/modules_basic.rx`, `conformance/resolve/nested_modules.rx` …(+4) |
| RXS-0033 | spec/names.md | 5 | `conformance/resolve/shadowing_blocks.rx`, `conformance/resolve/statics_consts_patterns.rx`, `conformance/syntax/names_module_scope.rx` …(+2) |
| RXS-0034 | spec/names.md | 9 | `conformance/resolve/enum_variants_assoc.rx`, `conformance/resolve/generics_params_refs.rx`, `conformance/resolve/modules_basic.rx` …(+6) |
| RXS-0035 | spec/names.md | 6 | `conformance/resolve/use_alias_chain.rx`, `conformance/resolve/use_simple.rx`, `conformance/syntax/names_use_visibility.rx` …(+3) |
| RXS-0036 | spec/names.md | 6 | `conformance/resolve/nested_modules.rx`, `conformance/resolve/private_descendants.rx`, `conformance/resolve/visibility_pub_package.rx` …(+3) |
| RXS-0037 | spec/names.md | 3 | `conformance/syntax/names_duplicates.rx`, `src/rurixc/src/resolve.rs`, `tests/ui/resolve/duplicate_definition.rx` |
| RXS-0038 | spec/names.md | 7 | `conformance/syntax/names_duplicates.rx`, `conformance/syntax/names_use_visibility.rx`, `src/rurixc/src/resolve.rs` …(+4) |
| RXS-0039 | spec/types.md | 3 | `conformance/typeck/literals_defaults.rx`, `conformance/typeck/tuples_arrays_typed.rx`, `src/rurixc/src/typeck.rs` |
| RXS-0040 | spec/types.md | 2 | `conformance/typeck/signatures.rx`, `src/rurixc/src/typeck.rs` |
| RXS-0041 | spec/types.md | 4 | `conformance/typeck/inference_locals.rx`, `conformance/typeck/shadow_rebind_typed.rx`, `src/rurixc/src/typeck.rs` …(+1) |
| RXS-0042 | spec/types.md | 7 | `conformance/typeck/calls.rx`, `conformance/typeck/references_params.rx`, `src/rurixc/src/typeck.rs` …(+4) |
| RXS-0043 | spec/types.md | 7 | `conformance/desugar/for_range_desugar.rx`, `conformance/typeck/control_flow_typed.rx`, `conformance/typeck/operators_typed.rx` …(+4) |
| RXS-0044 | spec/types.md | 9 | `conformance/desugar/option_result_prelude.rx`, `conformance/typeck/adt_construct.rx`, `conformance/typeck/control_flow_typed.rx` …(+6) |
| RXS-0045 | spec/types.md | 2 | `conformance/typeck/generics_mono.rx`, `src/rurixc/src/typeck.rs` |
| RXS-0046 | spec/types.md | 4 | `conformance/typeck/methods_casts.rx`, `src/rurixc/src/tbir_build.rs`, `src/rurixc/src/typeck.rs` …(+1) |
| RXS-0047 | spec/types.md | 13 | `src/rurixc/src/typeck.rs`, `tests/ui/typeck/arg_count.rx`, `tests/ui/typeck/arg_type_mismatch.rx` …(+10) |
| RXS-0048 | spec/borrow.md | 9 | `conformance/desugar/desugar_run_smoke.rx`, `conformance/desugar/iterator_protocol.rx`, `conformance/desugar/option_result_prelude.rx` …(+6) |
| RXS-0049 | spec/borrow.md | 6 | `conformance/desugar/desugar_run_smoke.rx`, `conformance/desugar/for_range_desugar.rx`, `conformance/desugar/iterator_protocol.rx` …(+3) |
| RXS-0050 | spec/borrow.md | 5 | `conformance/desugar/desugar_run_smoke.rx`, `conformance/desugar/question_mark_result.rx`, `src/rurixc/src/lower.rs` …(+2) |
| RXS-0051 | spec/borrow.md | 5 | `conformance/desugar/match_exhaustive.rx`, `src/rurixc/src/mir_build.rs`, `src/rurixc/src/tbir_build.rs` …(+2) |
| RXS-0052 | spec/borrow.md | 4 | `conformance/desugar/desugar_run_smoke.rx`, `conformance/desugar/drop_scope_blocks.rx`, `src/rurixc/src/drop_elab.rs` …(+1) |
| RXS-0053 | spec/borrow.md | 4 | `conformance/borrowck/accept/copy_types.rx`, `src/rurixc/src/move_check.rs`, `src/rurixc/src/typeck.rs` …(+1) |
| RXS-0054 | spec/borrow.md | 12 | `conformance/borrowck/accept/move_reinit.rx`, `conformance/borrowck/reject/use_after_move/basic.rx`, `conformance/borrowck/reject/use_after_move/conditional_move.rx` …(+9) |
| RXS-0055 | spec/borrow.md | 4 | `conformance/borrowck/accept/drop_order_run.rx`, `src/rurixc/src/drop_elab.rs`, `src/rurixc/src/mir_build.rs` …(+1) |
| RXS-0056 | spec/borrow.md | 1 | `conformance/borrowck/accept/temp_drop_stmt.rx` |
| RXS-0057 | spec/borrow.md | 4 | `conformance/borrowck/reject/double_mut_borrow/basic.rx`, `conformance/borrowck/reject/shared_mut_conflict/basic.rx`, `tests/ui/borrowck/double_mut_borrow.rx` …(+1) |
| RXS-0058 | spec/borrow.md | 7 | `conformance/borrowck/accept/shared_borrows.rx`, `conformance/borrowck/reject/double_mut_borrow/basic.rx`, `conformance/borrowck/reject/shared_mut_conflict/basic.rx` …(+4) |
| RXS-0059 | spec/borrow.md | 2 | `conformance/borrowck/accept/nll_released_reborrow.rx`, `src/rurixc/src/borrow_check.rs` |
| RXS-0060 | spec/borrow.md | 5 | `conformance/borrowck/reject/assign_while_borrowed/basic.rx`, `conformance/borrowck/reject/move_while_borrowed/basic.rx`, `src/rurixc/src/borrow_check.rs` …(+2) |
| RXS-0061 | spec/borrow.md | 4 | `conformance/borrowck/accept/reference_to_param.rx`, `conformance/borrowck/reject/dangling_reference/basic.rx`, `src/rurixc/src/borrow_check.rs` …(+1) |
| RXS-0062 | spec/consteval.md | 7 | `conformance/consteval/const_arith_run.rx`, `conformance/consteval/const_eval_run.rx`, `conformance/consteval/const_fn_eval.rx` …(+4) |
| RXS-0063 | spec/consteval.md | 5 | `conformance/consteval/const_arith_run.rx`, `conformance/consteval/const_eval_run.rx`, `conformance/consteval/const_fn_eval.rx` …(+2) |
| RXS-0064 | spec/consteval.md | 1 | `src/rurixc/src/const_eval.rs` |
| RXS-0065 | spec/consteval.md | 2 | `src/rurixc/src/const_eval.rs`, `tests/ui/consteval/overflow_add.rx` |
| RXS-0066 | spec/device.md | 7 | `conformance/coloring/accept/host_calls_device.rx`, `conformance/coloring/accept/kernel_calls_device.rx`, `conformance/coloring/reject/direct_kernel_call/basic.rx` …(+4) |
| RXS-0067 | spec/device.md | 8 | `conformance/addrspace/accept/constant_view_match.rx`, `conformance/addrspace/accept/matching_space.rx`, `conformance/addrspace/accept/mut_global_match.rx` …(+5) |
| RXS-0068 | spec/device.md | 4 | `conformance/coloring/accept/uniform_barrier.rx`, `conformance/coloring/reject/barrier_non_uniform/basic.rx`, `src/rurixc/src/coloring.rs` …(+1) |
| RXS-0069 | spec/device.md | 2 | `src/rurixc/src/coloring.rs`, `src/rurixc/src/typeck.rs` |
| RXS-0070 | spec/device.md | 3 | `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs`, `tests/ui/codegen/kernel_array_index.rx` |
| RXS-0071 | spec/device.md | 4 | `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs`, `tests/ui/codegen/host_addrspace_view.rx` …(+1) |
| RXS-0072 | spec/device.md | 4 | `conformance/device/reject/threadctx_dim/basic.rx`, `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs` …(+1) |
| RXS-0073 | spec/device.md | 4 | `src/rurixc/src/device_codegen.rs`, `src/rurixc/src/ptxas.rs`, `src/rurixc/tests/ptxas_gate.rs` …(+1) |
| RXS-0074 | spec/device.md | 10 | `conformance/launch/accept/saxpy_launch.rx`, `conformance/launch/reject/arg_type_mismatch/basic.rx`, `conformance/launch/reject/context_brand_mismatch/basic.rx` …(+7) |
| RXS-0075 | spec/device.md | 10 | `conformance/launch/accept/saxpy_launch.rx`, `conformance/launch/reject/arg_type_mismatch/basic.rx`, `conformance/launch/reject/context_brand_mismatch/basic.rx` …(+7) |
| RXS-0076 | spec/device.md | 2 | `src/rurix-rt/src/lib.rs`, `src/rurix-rt/tests/gpu_roundtrip.rs` |
| RXS-0077 | spec/device.md | 1 | `src/rurix-rt/src/lib.rs` |
| RXS-0078 | spec/device.md | 14 | `conformance/views/accept/chunks_disjoint.rx`, `conformance/views/accept/split_at_disjoint.rx`, `conformance/views/reject/alias_mut_write/basic.rx` …(+11) |
| RXS-0079 | spec/device.md | 10 | `conformance/shared/accept/shared_barrier_consistent.rx`, `conformance/shared/reject/barrier_too_late/basic.rx`, `conformance/shared/reject/unsynced_cross_lane_read/basic.rx` …(+7) |
| RXS-0080 | spec/device.md | 11 | `conformance/atomics/accept/narrower_scope_ok.rx`, `conformance/atomics/accept/scoped_atomics_ok.rx`, `conformance/atomics/reject/scope_addrspace_incompat/basic.rx` …(+8) |
| RXS-0081 | spec/device.md | 8 | `conformance/libdevice/accept/device_math_intrinsics.rx`, `conformance/libdevice/accept/f64_intrinsics.rx`, `conformance/libdevice/accept/log_exp_intrinsics.rx` …(+5) |
| RXS-0082 | spec/device.md | 6 | `conformance/libdevice/accept/device_math_intrinsics.rx`, `conformance/libdevice/accept/f64_intrinsics.rx`, `conformance/libdevice/accept/log_exp_intrinsics.rx` …(+3) |
| RXS-0083 | spec/toolchain.md | 3 | `src/rurixc/tests/toolchain_corpus.rs`, `src/rx/src/doc.rs`, `src/rx/tests/cli.rs` |
| RXS-0084 | spec/toolchain.md | 2 | `conformance/toolchain/hello.rx`, `src/rurixc/tests/toolchain_corpus.rs` |
| RXS-0085 | spec/toolchain.md | 1 | `conformance/toolchain/exit_code.rx` |
| RXS-0086 | spec/toolchain.md | 2 | `conformance/toolchain/check_ok.rx`, `src/rurixc/tests/toolchain_corpus.rs` |
| RXS-0087 | spec/toolchain.md | 3 | `src/rurixc/tests/fmt_corpus.rs`, `src/rurixc/tests/toolchain_corpus.rs`, `src/rx/tests/cli.rs` |
| RXS-0088 | spec/toolchain.md | 1 | `src/rurixc/tests/toolchain_corpus.rs` |
| RXS-0089 | spec/toolchain.md | 2 | `src/rurix-pkg/src/manifest.rs`, `src/rurix-pkg/src/toml.rs` |
| RXS-0090 | spec/toolchain.md | 2 | `src/rurix-pkg/src/manifest.rs`, `src/rurix-pkg/src/vendor.rs` |
| RXS-0091 | spec/toolchain.md | 1 | `src/rurix-pkg/src/resolve.rs` |
| RXS-0092 | spec/toolchain.md | 3 | `src/rurix-pkg/src/lock.rs`, `src/rurix-pkg/src/toml.rs`, `src/rurix-pkg/src/vendor.rs` |
| RXS-0093 | spec/toolchain.md | 3 | `src/rurix-pkg/src/content_tree.rs`, `src/rurix-pkg/src/sha256.rs`, `src/rurix-pkg/src/vendor.rs` |
| RXS-0094 | spec/toolchain.md | 2 | `src/rurix-pkg/src/vendor.rs`, `src/rx/tests/cli.rs` |
| RXS-0095 | spec/toolchain.md | 4 | `conformance/toolchain/rx_test_basic.rx`, `conformance/toolchain/rx_test_gpu.rx`, `src/rurixc/src/test_harness.rs` …(+1) |
| RXS-0096 | spec/toolchain.md | 2 | `conformance/workspace/repro/src/main.rx`, `src/rurix-pkg/src/vendor.rs` |
| RXS-0097 | spec/toolchain.md | 1 | `conformance/workspace/repro/src/main.rx` |
| RXS-0098 | spec/toolchain.md | 3 | `src/rurixc/src/query.rs`, `src/rurixc/src/tooling/lsp.rs`, `src/rurixc/src/tooling/session.rs` |
| RXS-0099 | spec/toolchain.md | 2 | `conformance/toolchain/lsp_mvp/sample.rx`, `src/rurixc/src/tooling/diag_json.rs` |
| RXS-0100 | spec/toolchain.md | 3 | `conformance/toolchain/lsp_mvp/sample.rx`, `src/rurixc/src/tooling/ide_query.rs`, `src/rurixc/src/tooling/lsp.rs` |
| RXS-0101 | spec/toolchain.md | 2 | `conformance/toolchain/lsp_mvp/sample.rx`, `src/rurixc/src/tooling/ide_query.rs` |
| RXS-0102 | spec/toolchain.md | 2 | `conformance/toolchain/lsp_mvp/sample.rx`, `src/rurixc/src/tooling/ide_query.rs` |
| RXS-0103 | spec/toolchain.md | 3 | `conformance/toolchain/lsp_mvp/sample.rx`, `src/rurixc/src/tooling/ide_query.rs`, `src/rurixc/src/tooling/lsp.rs` |
| RXS-0104 | spec/stdlib.md | 2 | `conformance/stdlib/device/vec_scalar.rx`, `conformance/stdlib/host/vec_ops.rx` |
| RXS-0105 | spec/stdlib.md | 3 | `conformance/stdlib/device/vec_scalar.rx`, `conformance/stdlib/host/vec_ops.rx`, `conformance/stdlib/reject/illegal_swizzle/basic.rx` |
| RXS-0106 | spec/stdlib.md | 3 | `conformance/stdlib/device/vec_scalar.rx`, `conformance/stdlib/host/vec_ops.rx`, `conformance/stdlib/reject/dim_mismatch/basic.rx` |
| RXS-0107 | spec/stdlib.md | 2 | `conformance/stdlib/device/vec_scalar.rx`, `conformance/stdlib/host/vec_ops.rx` |
| RXS-0108 | spec/stdlib.md | 2 | `conformance/stdlib/device/mat_scalar.rx`, `conformance/stdlib/host/mat_ops.rx` |
| RXS-0109 | spec/stdlib.md | 2 | `conformance/stdlib/device/mat_scalar.rx`, `conformance/stdlib/host/mat_ops.rx` |
| RXS-0110 | spec/stdlib.md | 4 | `conformance/stdlib/device/geom_scalar.rx`, `conformance/stdlib/host/geom_ops.rx`, `conformance/stdlib/reject/geom_type_confusion/basic.rx` …(+1) |
| RXS-0111 | spec/stdlib.md | 3 | `conformance/stdlib/device/geom_scalar.rx`, `conformance/stdlib/host/geom_ops.rx`, `src/rurix-geometry/src/lib.rs` |
| RXS-0112 | spec/stdlib.md | 3 | `conformance/stdlib/device/geom_scalar.rx`, `conformance/stdlib/host/geom_ops.rx`, `src/rurix-geometry/src/lib.rs` |
| RXS-0113 | spec/stdlib.md | 3 | `conformance/stdlib/device/geom_scalar.rx`, `conformance/stdlib/host/geom_ops.rx`, `src/rurix-geometry/src/lib.rs` |
| RXS-0114 | spec/imageio.md | 1 | `src/image-io/src/lib.rs` |
| RXS-0115 | spec/imageio.md | 1 | `src/image-io/src/lib.rs` |
| RXS-0116 | spec/imageio.md | 1 | `src/image-io/src/lib.rs` |
| RXS-0117 | spec/imageio.md | 1 | `src/image-io/src/lib.rs` |
| RXS-0118 | spec/softraster.md | 2 | `conformance/soft_raster/device/sr_binning.rx`, `src/soft-raster/src/lib.rs` |
| RXS-0119 | spec/softraster.md | 2 | `conformance/soft_raster/device/sr_raster_tile.rx`, `src/soft-raster/src/lib.rs` |
| RXS-0120 | spec/softraster.md | 2 | `conformance/soft_raster/device/sr_depth.rx`, `src/soft-raster/src/lib.rs` |
| RXS-0121 | spec/softraster.md | 2 | `conformance/soft_raster/device/sr_tonemap.rx`, `src/soft-raster/src/lib.rs` |
| RXS-0122 | spec/interop.md | 1 | `src/rurix-interop/src/lib.rs` |
| RXS-0123 | spec/interop.md | 1 | `src/rurix-interop/src/lib.rs` |
| RXS-0124 | spec/interop.md | 1 | `src/rurix-interop/src/lib.rs` |
| RXS-0125 | spec/interop.md | 1 | `src/rurix-interop/src/lib.rs` |
| RXS-0126 | spec/cublas.md | 1 | `src/rurix-cublas/src/lib.rs` |
| RXS-0127 | spec/cublas.md | 1 | `src/rurix-cublas/src/lib.rs` |
| RXS-0128 | spec/cublas.md | 1 | `src/rurix-cublas/src/lib.rs` |
| RXS-0129 | spec/cublas.md | 1 | `src/rurix-cublas/src/lib.rs` |
| RXS-0130 | spec/pipeline.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0131 | spec/pipeline.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0132 | spec/pipeline.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0133 | spec/pipeline.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0134 | spec/pipeline.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0135 | spec/release.md | 2 | `src/rurixup/src/bundle.rs`, `src/rurixup/src/install.rs` |
| RXS-0136 | spec/release.md | 1 | `src/rurixup/src/bundle.rs` |
| RXS-0137 | spec/release.md | 1 | `src/rurixup/src/signing.rs` |
| RXS-0138 | spec/release.md | 1 | `src/rurixup/src/sbom.rs` |
| RXS-0139 | spec/release.md | 2 | `src/rurixup/src/gate.rs`, `src/rurixup/src/lib.rs` |
| RXS-0140 | spec/interop_d3d12.md | 1 | `src/rurix-rt/src/interop.rs` |
| RXS-0141 | spec/interop_d3d12.md | 1 | `src/rurix-rt/src/interop.rs` |
| RXS-0142 | spec/interop_d3d12.md | 1 | `src/rurix-rt/src/interop.rs` |
| RXS-0143 | spec/interop_d3d12.md | 1 | `src/rurix-rt/src/interop.rs` |
| RXS-0144 | spec/async_buffer.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0145 | spec/async_buffer.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0146 | spec/async_buffer.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0147 | spec/async_buffer.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0148 | spec/async_buffer.md | 1 | `src/rurix-rt/src/pipeline.rs` |
| RXS-0149 | spec/engine_integration.md | 1 | `src/rurix-engine/src/lib.rs` |
| RXS-0150 | spec/release.md | 1 | `src/rurix-rt/src/fatbin.rs` |
| RXS-0151 | spec/release.md | 1 | `src/rurix-rt/src/fatbin.rs` |
| RXS-0152 | spec/release.md | 1 | `src/rurix-pkg/src/lock.rs` |
| RXS-0153 | spec/shader_stages.md | 4 | `conformance/shader/accept/basic_stages.rx`, `conformance/shader/reject/stage_misuse/direct_call.rx`, `src/rurixc/src/shader_stages.rs` …(+1) |
| RXS-0154 | spec/shader_stages.md | 4 | `conformance/shader/accept/basic_stages.rx`, `conformance/shader/reject/io_annotation/unannotated_field.rx`, `src/rurixc/src/shader_stages.rs` …(+1) |
| RXS-0155 | spec/shader_stages.md | 4 | `conformance/shader/accept/basic_stages.rx`, `conformance/shader/reject/interface_mismatch/vs_fs_mismatch.rx`, `src/rurixc/src/shader_stages.rs` …(+1) |
| RXS-0156 | spec/shader_stages.md | 4 | `conformance/shader/accept/basic_stages.rx`, `conformance/shader/reject/resource_handle/handle_return.rx`, `src/rurixc/src/shader_stages.rs` …(+1) |
| RXS-0157 | spec/dxil_backend.md | 4 | `conformance/dxil/accept/cs_noop.rx`, `conformance/dxil/reject/nontrivial_body.rx`, `conformance/dxil/reject/view_param.rx` …(+1) |
| RXS-0158 | spec/dxil_backend.md | 5 | `conformance/dxil/graphics/accept/fs_passthrough.rx`, `conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_vs.rx` …(+2) |
| RXS-0159 | spec/dxil_backend.md | 4 | `conformance/dxil/graphics/accept/fs_passthrough.rx`, `conformance/dxil/graphics/reject/vs_builtin_scalar_position.rx`, `src/rurixc/src/dxil_codegen.rs` …(+1) |
| RXS-0160 | spec/dxil_backend.md | 3 | `conformance/dxil/graphics/accept/vs_fs_link.rx`, `src/rurixc/src/dxil_codegen.rs`, `src/rurixc/src/dxil_sig_gate.rs` |
| RXS-0161 | spec/dxil_backend.md | 7 | `conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_fs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_vs.rx` …(+4) |
| RXS-0162 | spec/dxil_backend.md | 5 | `conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_vs.rx`, `conformance/dxil/graphics/accept/vs_varying.rx` …(+2) |
| RXS-0163 | spec/binding_layout.md | 3 | `src/rurixc/src/binding_layout.rs`, `src/rurixc/src/dxil_codegen.rs`, `src/rurixc/src/mir_build.rs` |
| RXS-0164 | spec/binding_layout.md | 2 | `src/rurixc/src/binding_layout.rs`, `src/rurixc/src/dxil_codegen.rs` |
| RXS-0165 | spec/binding_layout.md | 4 | `src/rurixc/src/binding_layout.rs`, `src/rurixc/src/dxil_codegen.rs`, `src/rurixc/src/mir_build.rs` …(+1) |
| RXS-0166 | spec/binding_layout.md | 3 | `src/rurixc/src/binding_layout.rs`, `src/rurixc/src/dxil_codegen.rs`, `src/rurixc/tests/dxil_golden.rs` |
| RXS-0167 | spec/d3d12_runtime.md | 2 | `src/uc04-demo/src/error.rs`, `src/uc04-demo/src/pso.rs` |
| RXS-0168 | spec/d3d12_runtime.md | 3 | `conformance/dxil/graphics/accept/uc04_gbuffer_fs.rx`, `src/rurixc/tests/dxil_golden.rs`, `src/uc04-demo/src/deferred.rs` |
| RXS-0169 | spec/d3d12_runtime.md | 1 | `src/uc04-demo/src/barrier.rs` |
| RXS-0170 | spec/d3d12_runtime.md | 2 | `src/uc04-demo/src/device.rs`, `src/uc04-demo/src/readback.rs` |
| RXS-0171 | spec/dxil_backend.md | 8 | `conformance/dxil/graphics/accept/fs_body_arith.rx`, `conformance/dxil/graphics/accept/uc04_gbuffer_fs.rx`, `conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx` …(+5) |
| RXS-0172 | spec/dxil_backend.md | 3 | `conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_vs.rx`, `src/rurixc/src/dxil_codegen.rs` |
| RXS-0173 | spec/dxil_backend.md | 3 | `conformance/dxil/graphics/accept/uc04_gbuffer_fs.rx`, `conformance/dxil/graphics/accept/uc04_lighting_fs.rx`, `src/rurixc/src/dxil_sig_gate.rs` |
| RXS-0174 | spec/shader_stages.md | 1 | `conformance/dxil/graphics/accept/uc04_lighting_fs.rx` |
| RXS-0175 | spec/dxil_backend.md | 1 | `conformance/dxil/graphics/accept/uc04_lighting_fs.rx` |
| RXS-0176 | spec/dxil_backend.md | 1 | `conformance/dxil/graphics/accept/uc04_lighting_fs.rx` |
| RXS-0177 | spec/edition.md | 2 | `src/rurix-pkg/src/manifest.rs`, `src/rurix-pkg/tests/edition_corpus.rs` |
| RXS-0178 | spec/edition.md | 2 | `src/rurix-pkg/src/manifest.rs`, `src/rurix-pkg/tests/edition_corpus.rs` |
| RXS-0179 | spec/edition.md | 2 | `src/rurix-pkg/src/manifest.rs`, `src/rurix-pkg/tests/edition_corpus.rs` |
| RXS-0180 | spec/edition.md | 1 | `src/rurix-pkg/src/manifest.rs` |
| RXS-0185 | spec/release.md | 1 | `src/rurixup/src/channel.rs` |
| RXS-0186 | spec/release.md | 3 | `src/rurixup/src/channel.rs`, `src/rurixup/src/gate.rs`, `src/rurixup/src/lib.rs` |
| RXS-0187 | spec/release.md | 1 | `src/rurixup/src/toolchain.rs` |
| RXS-0188 | spec/release.md | 1 | `src/rurixup/src/toolchain.rs` |
| RXS-0189 | spec/host_orchestration.md | 3 | `conformance/host_orch/accept/saxpy_single_source/main.rx`, `conformance/host_orch/reject/buffer_move/main.rx`, `conformance/host_orch/reject/gpu_in_kernel/main.rx` |
| RXS-0190 | spec/host_orchestration.md | 2 | `conformance/host_orch/accept/saxpy_single_source/main.rx`, `conformance/host_orch/reject/elem_infer/main.rx` |
| RXS-0191 | spec/host_orchestration.md | 2 | `conformance/host_orch/accept/saxpy_single_source/main.rx`, `conformance/host_orch/reject/launch_arg_subset/main.rx` |
| RXS-0192 | spec/host_orchestration.md | 1 | `conformance/host_orch/accept/saxpy_single_source/main.rx` |
| RXS-0193 | spec/host_orchestration.md | 3 | `src/rurix-rt-cabi/src/imageio.rs`, `src/rurix-rt-cabi/src/lib.rs`, `src/rurix-rt-cabi/src/present.rs` |
| RXS-0194 | spec/host_orchestration.md | 1 | `src/rurix-rt-cabi/src/lib.rs` |
| RXS-0195 | spec/host_orchestration.md | 3 | `conformance/host_orch/accept/extern_link/main.rx`, `src/rurixc/src/driver.rs`, `src/rurixc/src/mir_build.rs` |
| RXS-0196 | spec/host_orchestration.md | 8 | `conformance/host_orch/accept/mod_file/main.rx`, `conformance/host_orch/accept/mod_file/util.rx`, `conformance/host_orch/reject/mod_cycle/main.rx` …(+5) |
| RXS-0197 | spec/host_orchestration.md | 5 | `conformance/host_orch/accept/present_loop/main.rx`, `conformance/host_orch/reject/present_in_kernel/main.rx`, `conformance/host_orch/reject/present_out_of_order/main.rx` …(+2) |
| RXS-0198 | spec/host_orchestration.md | 4 | `conformance/host_orch/accept/present_loop/main.rx`, `src/rurix-rt-cabi/src/lib.rs`, `src/rurix-rt-cabi/src/present.rs` …(+1) |
| RXS-0199 | spec/host_orchestration.md | 2 | `conformance/host_orch/accept/imageio_write/main.rx`, `src/rurix-rt-cabi/src/imageio.rs` |
| RXS-0200 | spec/vulkan_backend.md | 1 | `conformance/vulkan/accept/vk_noop.rx` |
| RXS-0201 | spec/vulkan_backend.md | 2 | `conformance/vulkan/accept/vk_noop.rx`, `src/rurixc/src/vulkan_codegen.rs` |
| RXS-0202 | spec/vulkan_backend.md | 1 | `conformance/vulkan/accept/vk_fill.rx` |
| RXS-0203 | spec/vulkan_backend.md | 2 | `conformance/vulkan/accept/vk_fill.rx`, `conformance/vulkan/accept/vk_saxpy.rx` |
| RXS-0204 | spec/vulkan_backend.md | 2 | `conformance/vulkan/accept/vk_fragment.rx`, `conformance/vulkan/accept/vk_vertex.rx` |
| RXS-0205 | spec/vulkan_backend.md | 1 | `conformance/vulkan/accept/vk_math.rx` |
| RXS-0206 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/backend.rs` |
| RXS-0207 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/vk.rs` |
| RXS-0208 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/vk.rs` |
| RXS-0209 | spec/vulkan_backend.md | 2 | `src/rurix-pkg/src/lock.rs`, `src/rurix-rt/src/fatbin.rs` |
| RXS-0210 | spec/vulkan_backend.md | 4 | `conformance/vulkan/accept/vk_tri_fs.rx`, `conformance/vulkan/accept/vk_tri_vs.rx`, `src/rurix-rt/src/vk.rs` …(+1) |
| RXS-0211 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/vk.rs` |
| RXS-0212 | spec/vulkan_backend.md | 1 | `src/rurixc/src/toolchain.rs` |
| RXS-0213 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/vk.rs` |
| RXS-0214 | spec/release.md | 2 | `src/rurixup/src/install.rs`, `src/rurixup/src/toolchain.rs` |
| RXS-0215 | spec/release.md | 1 | `src/rurixup/src/shim.rs` |
| RXS-0216 | spec/release.md | 1 | `src/rurixup/src/fetch.rs` |
| RXS-0217 | spec/release.md | 1 | `src/rurixup/src/fetch.rs` |
| RXS-0218 | spec/release.md | 1 | `src/rurixup/src/bundle.rs` |
| RXS-0219 | spec/release.md | 1 | `src/rurixup/src/e2e.rs` |
| RXS-0220 | spec/d3d12_runtime.md | 1 | `src/uc04-demo/src/present.rs` |
| RXS-0221 | spec/d3d12_runtime.md | 2 | `src/rurix-rt/src/vk.rs`, `src/uc04-demo/src/present.rs` |
| RXS-0222 | spec/d3d12_runtime.md | 2 | `src/uc04-demo/src/device.rs`, `src/uc04-demo/src/present.rs` |
| RXS-0223 | spec/shader_stages.md | 29 | `conformance/dxil/graphics/accept/sample_superset_fs.rx`, `conformance/dxil/graphics/accept/sample_superset_rw_fs.rx`, `conformance/dxil/graphics/accept/sampling_cmp_fs.rx` …(+26) |
| RXS-0224 | spec/shader_stages.md | 2 | `conformance/dxil/graphics/accept/sampling_sample_lod_fs.rx`, `src/rurixc/src/binding_layout.rs` |
| RXS-0225 | spec/host_orchestration.md | 2 | `conformance/dxil/graphics/accept/sampling_sample_lod_fs.rx`, `src/rurix-rt/src/sampler.rs` |
| RXS-0226 | spec/dxil_backend.md | 10 | `conformance/dxil/graphics/accept/sample_superset_fs.rx`, `conformance/dxil/graphics/accept/sample_superset_rw_fs.rx`, `conformance/dxil/graphics/accept/sampling_cmp_fs.rx` …(+7) |
| RXS-0227 | spec/dxil_backend.md | 1 | `src/rurixc/src/dxil_spirv.rs` |
| RXS-0228 | spec/dxil_backend.md | 4 | `conformance/dxil/graphics/accept/sample_superset_fs.rx`, `conformance/dxil/graphics/accept/sampling_fetch_vs.rx`, `conformance/dxil/graphics/accept/sampling_load_fs.rx` …(+1) |
| RXS-0229 | spec/dxil_backend.md | 4 | `conformance/dxil/graphics/accept/sample_superset_rw_fs.rx`, `conformance/dxil/graphics/accept/sampling_fetch_vs.rx`, `conformance/dxil/graphics/accept/sampling_storage_fs.rx` …(+1) |
| RXS-0230 | spec/vulkan_backend.md | 4 | `conformance/dxil/graphics/accept/sampling_fetch_vs.rx`, `conformance/dxil/graphics/accept/sampling_fullscreen_vs.rx`, `src/rurix-rt/src/vk.rs` …(+1) |
| RXS-0231 | spec/shader_stages.md | 4 | `conformance/dxil/graphics/accept/bindless_quadrant_vs.rx`, `conformance/dxil/graphics/accept/bindless_sample_fs.rx`, `conformance/shader/accept/bindless_dynamic_index.rx` …(+1) |
| RXS-0232 | spec/shader_stages.md | 7 | `conformance/dxil/graphics/accept/bindless_quadrant_vs.rx`, `conformance/dxil/graphics/accept/bindless_sample_fs.rx`, `conformance/shader/accept/bindless_dynamic_index.rx` …(+4) |
| RXS-0233 | spec/binding_layout.md | 3 | `conformance/dxil/graphics/accept/bindless_sample_fs.rx`, `src/rurix-rt/src/vk.rs`, `src/rurixc/src/binding_layout.rs` |
| RXS-0234 | spec/dxil_backend.md | 4 | `conformance/dxil/graphics/accept/bindless_sample_fs.rx`, `src/rurix-rt/src/vk.rs`, `src/rurixc/src/dxil_spirv.rs` …(+1) |
| RXS-0235 | spec/host_orchestration.md | 5 | `conformance/dxil/graphics/accept/bindless_quadrant_vs.rx`, `conformance/host_orch/accept/bindless_table/main.rx`, `conformance/host_orch/reject/table_in_kernel/main.rx` …(+2) |
| RXS-0236 | spec/render_graph.md | 2 | `conformance/host_orch/accept/graph_deferred_three_pass/main.rx`, `conformance/host_orch/reject/graph_in_kernel/main.rx` |
| RXS-0237 | spec/render_graph.md | 1 | `src/rurix-rt/src/graph.rs` |
| RXS-0238 | spec/render_graph.md | 1 | `src/rurix-rt/src/graph.rs` |
| RXS-0239 | spec/render_graph.md | 1 | `src/uc04-demo/tests/d6_crosscheck.rs` |
| RXS-0240 | spec/render_graph.md | 2 | `src/rurix-rt/src/graph.rs`, `src/rurix-rt/src/vk.rs` |
| RXS-0241 | spec/render_graph.md | 2 | `src/rurix-rt-cabi/src/lib.rs`, `src/uc04-demo/tests/d6_crosscheck.rs` |
| RXS-0242 | spec/shader_stages.md | 1 | `src/rurixc/src/shader_stages.rs` |
| RXS-0243 | spec/shader_stages.md | 2 | `conformance/reflection/accept/mesh_reflection.rx`, `src/rurixc/src/shader_stages.rs` |
| RXS-0244 | spec/shader_stages.md | 2 | `conformance/rt_pipeline/accept/multi_miss.rx`, `src/rurixc/src/shader_stages.rs` |
| RXS-0245 | spec/shader_stages.md | 2 | `conformance/rt_pipeline/reject/trace_dynamic_sbt_offset.rx`, `src/rurixc/src/shader_stages.rs` |
| RXS-0246 | spec/vulkan_backend.md | 1 | `src/rurixc/src/vulkan_codegen.rs` |
| RXS-0247 | spec/vulkan_backend.md | 1 | `src/rurixc/src/vulkan_codegen.rs` |
| RXS-0248 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/vk.rs` |
| RXS-0249 | spec/dxil_backend.md | 1 | `src/rurixc/src/dxil_codegen.rs` |
| RXS-0250 | spec/export_c.md | 6 | `conformance/export_c/accept/add.rx`, `conformance/export_c/accept/name_override.rx`, `conformance/export_c/reject/attr_non_pub.rx` …(+3) |
| RXS-0251 | spec/export_c.md | 7 | `conformance/export_c/accept/add.rx`, `conformance/export_c/accept/ptr_store.rx`, `conformance/export_c/accept/unit_return.rx` …(+4) |
| RXS-0252 | spec/export_c.md | 2 | `conformance/export_c/accept/multi_export.rx`, `src/rurixc/src/export_c.rs` |
| RXS-0253 | spec/export_c.md | 2 | `conformance/export_c/accept/multi_export.rx`, `src/rurixc/src/export_c.rs` |
| RXS-0254 | spec/export_c.md | 2 | `conformance/export_c/accept/multi_export.rx`, `src/rurixc/src/export_c.rs` |
| RXS-0255 | spec/export_c.md | 8 | `conformance/export_c/accept/arith_no_panic.rx`, `conformance/export_c/accept/unit_return.rx`, `conformance/export_c/reject/index_body_panic.rx` …(+5) |
| RXS-0256 | spec/rhi.md | 5 | `conformance/uc05/accept/rhi_min.rx`, `conformance/uc05/reject/rhi_cross_brand.rx`, `conformance/uc05/reject/rhi_in_kernel.rx` …(+2) |
| RXS-0257 | spec/rhi.md | 5 | `conformance/uc05/accept/pass_declared.rx`, `conformance/uc05/assembly/pass_undeclared_read.rx`, `src/rurix-rt-cabi/src/lib.rs` …(+2) |
| RXS-0258 | spec/rhi.md | 7 | `conformance/uc05/accept/graph_three_pass.rx`, `conformance/uc05/assembly/graph_cycle.rx`, `conformance/uc05/assembly/graph_empty.rx` …(+4) |
| RXS-0259 | spec/rhi.md | 5 | `conformance/uc05/accept/graph_three_pass.rx`, `conformance/uc05/reject/res_double_move.rx`, `conformance/uc05/reject/res_use_after_move.rx` …(+2) |
| RXS-0260 | spec/rhi.md | 4 | `conformance/uc05/accept/single_submit.rx`, `conformance/uc05/reject/rhi_double_submit.rx`, `src/rurix-rt-cabi/src/lib.rs` …(+1) |
| RXS-0261 | spec/rhi.md | 1 | `src/rurix-rt-cabi/src/lib.rs` |
| RXS-0262 | spec/rhi.md | 1 | `src/rurix-rt/src/rhi.rs` |
| RXS-0263 | spec/rhi.md | 1 | `src/rurixc/tests/uc05_corpus.rs` |
| RXS-0264 | spec/rhi.md | 1 | `src/rurixc/tests/uc05_corpus.rs` |
| RXS-0265 | spec/rhi.md | 1 | `src/rurixc/tests/uc05_corpus.rs` |
| RXS-0270 | spec/rhi.md | 3 | `conformance/uc05/accept/gfx_pass.rx`, `conformance/uc05/reject/cross_brand_gfx.rx`, `src/rurix-rt-cabi/src/lib.rs` |
| RXS-0271 | spec/rhi.md | 4 | `conformance/uc05/accept/gfx_resources.rx`, `conformance/uc05/reject/cross_brand_gfx.rx`, `conformance/uc05/reject/rhi_gfx_in_kernel.rx` …(+1) |
| RXS-0272 | spec/rhi.md | 8 | `conformance/uc05/accept/gfx_bindless.rx`, `conformance/uc05/accept/gfx_pass.rx`, `conformance/uc05/accept/gfx_resources.rx` …(+5) |
| RXS-0273 | spec/rhi.md | 2 | `conformance/uc05/accept/gfx_bindless.rx`, `conformance/uc05/accept/gfx_resources.rx` |
| RXS-0274 | spec/rhi.md | 5 | `conformance/uc05/reject/gfx_present_not_last.rx`, `conformance/uc05/reject/gfx_present_twice.rx`, `src/rurix-rt-cabi/src/lib.rs` …(+2) |
| RXS-0275 | spec/vulkan_backend.md | 1 | `src/rurixc/src/vulkan_codegen.rs` |
| RXS-0276 | spec/rhi.md | 2 | `conformance/uc05/accept/gfx_bindless.rx`, `src/rurix-rt/src/rhi.rs` |
| RXS-0277 | spec/rhi.md | 1 | `src/rurixc/src/mir_build.rs` |
| RXS-0280 | spec/rhi.md | 2 | `src/rurix-rt/src/alias_alloc.rs`, `src/rurix-rt/src/rhi.rs` |
| RXS-0281 | spec/rhi.md | 2 | `src/rurix-rt/src/rhi.rs`, `src/rurix-rt/src/scheduler.rs` |
| RXS-0282 | spec/rhi.md | 2 | `src/rurix-rt/src/rhi.rs`, `src/rurix-rt/src/scheduler.rs` |
| RXS-0283 | spec/rhi.md | 5 | `conformance/uc05/accept/const_capacity_graph.rx`, `conformance/uc05/reject/nonstatic_graph_construction.rx`, `conformance/uc05/reject/transient_capacity_overflow.rx` …(+2) |
| RXS-0290 | spec/vulkan_backend.md | 2 | `src/rurix-rt-cabi/src/artifacts.rs`, `src/rurixc/src/codegen.rs` |
| RXS-0291 | spec/vulkan_backend.md | 2 | `src/rurixc/src/codegen.rs`, `src/rurixc/src/driver.rs` |
| RXS-0292 | spec/vulkan_backend.md | 2 | `src/rurix-rt-cabi/src/artifacts.rs`, `src/rurix-rt/src/fatbin.rs` |
| RXS-0293 | spec/vulkan_backend.md | 3 | `conformance/uc05/accept/rhi_create_vk.rx`, `src/rurix-rt-cabi/src/lib.rs`, `src/rurixc/tests/uc05_corpus.rs` |
| RXS-0294 | spec/vulkan_backend.md | 1 | `src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs` |
| RXS-0297 | spec/shader_stages.md | 4 | `conformance/rayquery/accept/ray_query_basic.rx`, `conformance/rayquery/accept/ray_query_hit_miss.rx`, `conformance/rayquery/reject/ray_query_escape.rx` …(+1) |
| RXS-0298 | spec/shader_stages.md | 3 | `conformance/rayquery/accept/ray_query_basic.rx`, `conformance/rayquery/accept/ray_query_hit_miss.rx`, `src/rurixc/src/vulkan_codegen.rs` |
| RXS-0299 | spec/shader_stages.md | 5 | `conformance/rayquery/accept/ray_query_basic.rx`, `conformance/rayquery/accept/ray_query_hit_miss.rx`, `conformance/rayquery/reject/committed_unguarded.rx` …(+2) |
| RXS-0300 | spec/vulkan_backend.md | 5 | `conformance/rayquery/accept/ray_query_basic.rx`, `conformance/rayquery/accept/ray_query_hit_miss.rx`, `conformance/vulkan/accept/vk_vec_component.rx` …(+2) |
| RXS-0301 | spec/vulkan_backend.md | 7 | `conformance/vulkan/accept/vk_hw_raster_visbuffer_fs.rx`, `conformance/vulkan/accept/vk_hw_raster_visbuffer_vs.rx`, `conformance/vulkan/reject/vk_hw_raster_cta_atomic_reject.rx` …(+4) |
| RXS-0302 | spec/vulkan_backend.md | 4 | `conformance/vulkan/accept/vk_hw_raster_visbuffer_fs.rx`, `conformance/vulkan/accept/vk_hw_raster_visbuffer_vs.rx`, `conformance/vulkan/reject/vk_hw_raster_cta_atomic_reject.rx` …(+1) |
| RXS-0303 | spec/vulkan_backend.md | 4 | `conformance/vulkan/accept/vk_hw_raster_visbuffer_fs.rx`, `conformance/vulkan/accept/vk_hw_raster_visbuffer_vs.rx`, `conformance/vulkan/reject/vk_hw_raster_f64_reject.rx` …(+1) |
| RXS-0304 | spec/rendering_platform.md | 8 | `conformance/reflection/accept/basic_reflection.rx`, `conformance/reflection/accept/compute_only.rx`, `conformance/reflection/accept/empty_entries.rx` …(+5) |
| RXS-0305 | spec/rendering_platform.md | 3 | `conformance/reflection/accept/basic_reflection.rx`, `conformance/reflection/reject/unbounded_sampler_table.rx`, `src/rurixc/src/reflection.rs` |
| RXS-0306 | spec/rendering_platform.md | 2 | `conformance/reflection/accept/basic_reflection.rx`, `src/rurixc/src/reflection.rs` |
| RXS-0307 | spec/rendering_platform.md | 1 | `src/rurixc/src/reflection.rs` |
| RXS-0308 | spec/rendering_platform.md | 7 | `conformance/permutation/accept/basic_domain.rx`, `conformance/permutation/accept/empty_domain_entry.rx`, `conformance/permutation/accept/int_axis.rx` …(+4) |
| RXS-0309 | spec/rendering_platform.md | 5 | `conformance/permutation/accept/axis_order_permuted.rx`, `conformance/permutation/accept/basic_domain.rx`, `conformance/permutation/accept/empty_domain_entry.rx` …(+2) |
| RXS-0310 | spec/rendering_platform.md | 5 | `conformance/permutation/accept/basic_domain.rx`, `conformance/permutation/accept/empty_domain_entry.rx`, `conformance/permutation/accept/int_axis.rx` …(+2) |
| RXS-0311 | spec/shader_stages.md | 6 | `conformance/capability/accept/fallback_low_profile.rx`, `conformance/capability/accept/implicit_propagation.rx`, `conformance/capability/accept/requires_supported.rx` …(+3) |
| RXS-0312 | spec/rendering_platform.md | 7 | `conformance/capability/accept/fallback_low_profile.rx`, `conformance/capability/accept/implicit_propagation.rx`, `conformance/capability/accept/requires_supported.rx` …(+4) |
| RXS-0313 | spec/rendering_platform.md | 1 | `src/rurixc/src/capability_check.rs` |
| RXS-0314 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/pso_cache.rs` |
| RXS-0315 | spec/vulkan_backend.md | 1 | `src/rurix-rt/src/pso_cache.rs` |
| RXS-0316 | spec/vulkan_backend.md | 2 | `src/rurix-rt/src/pso_cache.rs`, `src/rurix-rt/src/vk.rs` |
| RXS-0317 | spec/rendering_platform.md | 1 | `src/rurixc/src/manifest.rs` |
| RXS-0318 | spec/rendering_platform.md | 1 | `src/rurixc/src/manifest.rs` |
| RXS-0319 | spec/rhi.md | 3 | `conformance/gfx_submit/accept/m89_two_tri_quad.rx`, `conformance/gfx_submit/accept/vb_only_draw.rx`, `src/rurix-rt-cabi/src/lib.rs` |
| RXS-0320 | spec/rhi.md | 6 | `conformance/gfx_submit/accept/m89_two_tri_quad.rx`, `conformance/gfx_submit/reject/draw_ib_oob.rx`, `conformance/gfx_submit/reject/draw_vb_range_oob.rx` …(+3) |
| RXS-0321 | spec/rhi.md | 4 | `conformance/gfx_submit/accept/m89_two_tri_quad.rx`, `conformance/gfx_submit/accept/vb_only_draw.rx`, `src/rurix-rt-cabi/src/lib.rs` …(+1) |
| RXS-0322 | spec/shader_stages.md | 5 | `conformance/rt_pipeline/accept/two_hit_groups.rx`, `conformance/rt_pipeline/reject/record_outside_rt_stage.rx`, `conformance/rt_pipeline/reject/record_recursive.rx` …(+2) |
| RXS-0323 | spec/shader_stages.md | 6 | `conformance/rt_pipeline/accept/multi_miss.rx`, `conformance/rt_pipeline/accept/procedural_group.rx`, `conformance/rt_pipeline/accept/two_hit_groups.rx` …(+3) |
| RXS-0324 | spec/shader_stages.md | 7 | `conformance/rt_pipeline/accept/anyhit_ignore.rx`, `conformance/rt_pipeline/accept/callable_basic.rx`, `conformance/rt_pipeline/accept/procedural_group.rx` …(+4) |
| RXS-0325 | spec/vulkan_backend.md | 6 | `conformance/rt_pipeline/accept/anyhit_ignore.rx`, `conformance/rt_pipeline/accept/callable_basic.rx`, `conformance/rt_pipeline/accept/procedural_group.rx` …(+3) |
| RXS-0326 | spec/vulkan_backend.md | 3 | `conformance/rt_pipeline/accept/callable_basic.rx`, `conformance/rt_pipeline/accept/two_hit_groups.rx`, `src/rurix-rt/src/rt_incremental.rs` |
| RXS-0327 | spec/vulkan_backend.md | 3 | `conformance/rt_pipeline/accept/two_hit_groups.rx`, `src/rurix-rt/src/rt_incremental.rs`, `src/rurix-rt/src/vk_m50_rt_body.rs` |
| RXS-0328 | spec/geometry_pages.md | 2 | `src/rurix-asset/src/geom_build.rs`, `src/rurix-geom-pages/src/logical.rs` |
| RXS-0329 | spec/geometry_pages.md | 1 | `src/rurix-asset/src/geom_build.rs` |
| RXS-0330 | spec/geometry_pages.md | 1 | `src/rurix-asset/src/geom_build.rs` |
| RXS-0331 | spec/geometry_pages.md | 2 | `src/rurix-asset/src/geom_build.rs`, `src/rurix-geom-pages/src/logical.rs` |
| RXS-0332 | spec/asset_pipeline.md | 1 | `src/rurix-asset/src/schema.rs` |
| RXS-0333 | spec/asset_pipeline.md | 5 | `src/rurix-asset/src/gltf/canonical.rs`, `src/rurix-asset/src/gltf/glb.rs`, `src/rurix-asset/src/gltf/json.rs` …(+2) |
| RXS-0334 | spec/asset_pipeline.md | 2 | `src/rurix-asset/src/bcdec.rs`, `src/rurix-asset/src/texture.rs` |
| RXS-0335 | spec/asset_pipeline.md | 1 | `src/rurix-asset/src/canon.rs` |
| RXS-0336 | spec/asset_pipeline.md | 2 | `src/rurix-asset/src/cook.rs`, `src/rurix-asset/src/graph.rs` |
| RXS-0337 | spec/asset_pipeline.md | 1 | `src/rurix-asset/src/verify.rs` |
| RXS-0338 | spec/geometry_pages.md | 1 | `src/rurix-geom-pages/src/memory.rs` |
| RXS-0339 | spec/geometry_pages.md | 1 | `src/rurix-geom-pages/src/disk.rs` |
| RXS-0340 | spec/geometry_pages.md | 1 | `src/rurix-geom-pages/src/codec.rs` |
| RXS-0341 | spec/geometry_pages.md | 1 | `src/rurix-geom-pages/src/disk.rs` |
| RXS-0342 | spec/geometry_pages.md | 2 | `src/rurix-geom-pages/src/expand.rs`, `src/rurix-geom-pages/src/expand_v2.rs` |
| RXS-0343 | spec/asset_pipeline.md | 1 | `src/rurix-asset/src/ddc.rs` |
| RXS-0344 | spec/geometry_pages.md | 4 | `conformance/geom_pages/reject/rxpl_v2_unknown_major.rx`, `src/rurix-asset/src/geom_build_v2.rs`, `src/rurix-geom-pages/src/expand_v2.rs` …(+1) |
| RXS-0345 | spec/virtual_geometry.md | 2 | `conformance/virtual_geometry/reject/dag_error_nonmonotonic.rx`, `src/rurix-geom-build/src/dag.rs` |
| RXS-0346 | spec/render_graph.md | 2 | `conformance/render_graph/reject/missing_reads_indirect.rx`, `src/rurix-rt/src/graph.rs` |
| RXS-0347 | spec/rendering_platform.md | 4 | `conformance/reflection/reject/global_descriptor_index_dangling.rx`, `src/rurix-rt/src/descriptor_table.rs`, `src/rurix-rt/src/vk.rs` …(+1) |
| RXS-0348 | spec/gpu_driven_submit.md | 3 | `conformance/gpu_driven_submit/reject/dgc_layout_double_terminator.rx`, `src/rurix-rt/src/dgc.rs`, `src/rurix-rt/src/vk.rs` |
| RXS-0349 | spec/shader_stages.md | 2 | `conformance/capability/reject/unknown_capability_id_g92.rx`, `src/rurixc/src/capability_check.rs` |
| RXS-0350 | spec/virtual_geometry.md | 3 | `conformance/virtual_geometry/accept/visible_cluster_set_valid_cut.rx`, `conformance/virtual_geometry/reject/selection_cut_hole_injected.rx`, `src/rurix-geom-build/src/cull_ref.rs` |
| RXS-0351 | spec/virtual_geometry.md | 4 | `conformance/virtual_geometry/accept/clas_blas_matched.rx`, `conformance/virtual_geometry/reject/clas_blas_cluster_mismatch.rx`, `src/rurix-rt/src/rt_clas.rs` …(+1) |
| RXS-0352 | spec/virtual_geometry.md | 2 | `conformance/virtual_geometry/accept/single_source_three_consumers.rx`, `conformance/virtual_geometry/reject/bypass_single_source_variant.rx` |
| RXS-0353 | spec/virtual_geometry.md | 2 | `conformance/virtual_geometry/accept/visible_cluster_set_valid_cut.rx`, `src/rurix-geom-build/src/dag.rs` |
| RXS-0354 | spec/gpu_driven_submit.md | 2 | `conformance/gpu_driven_submit/reject/command_build_host_readback.rx`, `src/rurix-rt/src/command_build.rs` |
| RXS-0355 | spec/gpu_driven_submit.md | 4 | `src/rurix-rt/src/execution_set.rs`, `src/rurix-rt/src/pso_cache.rs`, `src/rurix-rt/src/vk.rs` …(+1) |
| RXS-0356 | spec/gpu_driven_submit.md | 2 | `conformance/gpu_driven_submit/reject/variant_budget_exceeded.rx`, `src/rurixc/src/shader_library.rs` |
| RXS-0357 | spec/global_illumination.md | 2 | `conformance/gi/accept/pt_reference_fixed_seed_minimal.rx`, `conformance/gi/reject/pt_seed_changed_nondeterministic.rx` |
| RXS-0358 | spec/global_illumination.md | 1 | `conformance/gi/reject/surface_cache_card_hole_leak.rx` |
| RXS-0359 | spec/global_illumination.md | 1 | `conformance/gi/reject/tracing_fallback_silent_demotion.rx` |
| RXS-0360 | spec/global_illumination.md | 2 | `conformance/gi/accept/spg_radiance_cache_screen_level_minimal.rx`, `conformance/gi/reject/radiance_cache_product_is_disabled.rx` |
| RXS-0361 | spec/global_illumination.md | 1 | `conformance/gi/reject/multi_light_restir_tier_unproven.rx` |
| RXS-0362 | spec/global_illumination.md | 3 | `conformance/gi/accept/if_tier_ladder_shared_kernel_minimal.rx`, `conformance/gi/reject/if_as_budget_exceeded_no_demote.rx`, `conformance/gi/reject/if_octahedral_srgb_encoding.rx` |
| RXS-0363 | spec/world_partition.md | 3 | `conformance/world_partition/accept/cell_event_sequence_minimal.rx`, `conformance/world_partition/reject/cell_event_sequence_out_of_order.rx`, `conformance/world_partition/reject/partition_budget_overrun_no_demote.rx` |
| RXS-0364 | spec/world_partition.md | 3 | `conformance/world_partition/accept/hlod_baking_double_build_minimal.rx`, `conformance/world_partition/reject/hlod_runtime_merge_forbidden.rx`, `src/rurix-asset/src/hlod.rs` |
| RXS-0365 | spec/world_partition.md | 2 | `conformance/world_partition/accept/atmosphere_froxel_fog_minimal.rx`, `conformance/world_partition/reject/atmosphere_weather_map_signature_tampered.rx` |
| RXS-0366 | spec/world_partition.md | 2 | `conformance/world_partition/accept/water_dual_pipeline_minimal.rx`, `conformance/world_partition/reject/water_spectrum_param_invalid.rx` |
| RXS-0367 | spec/world_partition.md | 2 | `conformance/world_partition/accept/terrain_chunk_cell_aligned_minimal.rx`, `conformance/world_partition/reject/terrain_lod_gap_crack.rx` |
| RXS-0368 | spec/world_partition.md | 2 | `conformance/world_partition/accept/decal_dbuffer_placeholder_minimal.rx`, `conformance/world_partition/reject/decal_overdraw_budget_exceeded.rx` |
| RXS-0369 | spec/display_pipeline.md | 2 | `conformance/display_pipeline/accept/view_transform_four_plugins_minimal.rx`, `conformance/display_pipeline/reject/non_hdr_swapchain_pq_output.rx` |
| RXS-0370 | spec/display_pipeline.md | 2 | `conformance/display_pipeline/accept/post_stack_explicit_order_minimal.rx`, `conformance/display_pipeline/reject/post_stack_implicit_sdr_clamp.rx` |
| RXS-0371 | spec/display_pipeline.md | 2 | `conformance/display_pipeline/accept/oit_benchmark_harness_minimal.rx`, `conformance/display_pipeline/reject/oit_default_tier_without_benchmark_data.rx` |
| RXS-0372 | spec/display_pipeline.md | 2 | `conformance/display_pipeline/accept/hair_marschner_lobes_minimal.rx`, `conformance/display_pipeline/reject/hair_lobe_tt_zeroed_no_diff.rx` |
| RXS-0373 | spec/display_pipeline.md | 2 | `conformance/display_pipeline/accept/skin_diffusion_profile_minimal.rx`, `conformance/display_pipeline/reject/skin_profile_zero_falloff_no_diffuse.rx` |
| RXS-0374 | spec/physics.md | 3 | `conformance/physics/accept/field_solver_coupling_minimal.rx`, `conformance/physics/reject/field_journal_capture_roundtrip_break.rx`, `conformance/physics/reject/world_field_render_writeback.rx` |
| RXS-0375 | spec/physics.md | 1 | `conformance/physics/accept/gameplay_field_full_phase_minimal.rx` |
| RXS-0376 | spec/physics.md | 2 | `conformance/physics/accept/buoyancy_field_channel_minimal.rx`, `conformance/physics/reject/buoyancy_bypass_api_injection.rx` |
| RXS-0377 | spec/physics.md | 2 | `conformance/physics/accept/jolt_ab_seven_step_minimal.rx`, `conformance/physics/reject/jolt_56_vendor_overwrite_baseline.rx` |
| RXS-0378 | spec/physics.md | 2 | `conformance/physics/accept/rapier_benchmark_ab_fixture_minimal.rx`, `conformance/physics/reject/rapier_benchmark_as_replay_oracle.rx` |
| RXS-0379 | spec/physics.md | 1 | `conformance/physics/reject/async_decorative_channel_without_verdict.rx` |
| RXS-0380 | spec/external_reference.md | 4 | `conformance/external_reference/accept/harness_command_face_minimal.rx`, `conformance/external_reference/reject/command_face_switch_outside_closed_set.rx`, `conformance/external_reference/reject/execcmds_template_injection.rx` …(+1) |
| RXS-0381 | spec/external_reference.md | 5 | `conformance/external_reference/accept/license_registry_minimal.rx`, `conformance/external_reference/reject/class_masquerade.rx`, `conformance/external_reference/reject/license_outside_whitelist.rx` …(+2) |
| RXS-0382 | spec/external_reference.md | 3 | `conformance/external_reference/accept/cache_layout_minimal.rx`, `conformance/external_reference/reject/cache_digest_tamper.rx`, `conformance/external_reference/reject/git_binary_guard_hit.rx` |
| RXS-0383 | spec/external_reference.md | 2 | `conformance/external_reference/accept/manifest_freeze_minimal.rx`, `conformance/external_reference/reject/manifest_in_place_edit.rx` |
| RXS-0384 | spec/visual_comparison.md | 4 | `conformance/visual_comparison/accept/determinism_contract_minimal.rx`, `conformance/visual_comparison/reject/non_unit_quat_injection.rx`, `conformance/visual_comparison/reject/schema_extra_field_injection.rx` …(+1) |
| RXS-0385 | spec/imageio.md | 4 | `conformance/imageio/accept/exr_hdr_container_minimal.rx`, `conformance/imageio/reject/bit_depth_truncation_8bit_clamp.rx`, `conformance/imageio/reject/srgb_linear_mislabel.rx` …(+1) |
| RXS-0386 | spec/visual_comparison.md | 2 | `conformance/visual_comparison/accept/metric_domain_contract_minimal.rx`, `conformance/visual_comparison/reject/domain_label_mismatch.rx` |
| RXS-0387 | spec/visual_comparison.md | 2 | `conformance/visual_comparison/accept/ssim_psnr_caliber_minimal.rx`, `conformance/visual_comparison/reject/hdr_direct_ssim_psnr.rx` |
| RXS-0388 | spec/visual_comparison.md | 2 | `conformance/visual_comparison/accept/pixel_diff_report_minimal.rx`, `conformance/visual_comparison/reject/diff_scalar_inconsistency.rx` |
| RXS-0389 | spec/visual_comparison.md | 3 | `conformance/visual_comparison/accept/flip_caliber_minimal.rx`, `conformance/visual_comparison/reject/flip_caliber_drift.rx`, `conformance/visual_comparison/reject/flip_reference_perturbation.rx` |
| RXS-0390 | spec/visual_comparison.md | 1 | `conformance/visual_comparison/accept/application_probe_minimal.rx` |
| RXS-0391 | spec/visual_comparison.md | 3 | `conformance/visual_comparison/accept/gap_registry_minimal.rx`, `conformance/visual_comparison/reject/gap_registry_missing_attribution.rx`, `conformance/visual_comparison/reject/gap_registry_unmeasured_narrative.rx` |
| RXS-0392 | spec/visual_comparison.md | 2 | `conformance/visual_comparison/accept/caliber_alignment_minimal.rx`, `conformance/visual_comparison/reject/caliber_fitting_masquerade.rx` |
| RXS-0393 | spec/visual_comparison.md | 2 | `conformance/visual_comparison/accept/fix_closure_criterion_minimal.rx`, `conformance/visual_comparison/reject/closure_handwritten_threshold.rx` |
| RXS-0394 | spec/global_illumination.md | 2 | `conformance/gi/accept/light_seed_set_minimal.rx`, `conformance/gi/reject/light_seed_gltf_direct_bypass.rx` |
| RXS-0395 | spec/global_illumination.md | 2 | `conformance/gi/accept/gi_multibounce_two_level_minimal.rx`, `conformance/gi/reject/gi_single_bounce_masquerade.rx` |
| RXS-0396 | spec/global_illumination.md | 2 | `conformance/gi/accept/world_radiance_cache_minimal.rx`, `conformance/gi/reject/world_cache_farfield_zero_energy.rx` |
| RXS-0397 | spec/global_illumination.md | 2 | `conformance/gi/accept/sky_ibl_direct_diffuse_minimal.rx`, `conformance/gi/reject/sky_ibl_gi_double_count.rx` |
| RXS-0398 | spec/global_illumination.md | 3 | `conformance/gi/accept/mis_full_surface_minimal.rx`, `conformance/gi/reject/mis_energy_bias_inject.rx`, `conformance/gi/reject/mis_weight_missing.rx` |
| RXS-0399 | spec/global_illumination.md | 3 | `conformance/gi/accept/rr_throughput_adaptive_minimal.rx`, `conformance/gi/reject/rr_compensation_missing.rx`, `conformance/gi/reject/rr_early_kill_bias.rx` |
| RXS-0400 | spec/global_illumination.md | 2 | `conformance/gi/accept/lds_deterministic_minimal.rx`, `conformance/gi/reject/lds_nondeterministic_inject.rx` |
| RXS-0401 | spec/global_illumination.md | 3 | `conformance/gi/accept/adaptive_convergence_minimal.rx`, `conformance/gi/reject/early_stop_masquerade.rx`, `conformance/gi/reject/unconverged_pixel_underreport.rx` |
