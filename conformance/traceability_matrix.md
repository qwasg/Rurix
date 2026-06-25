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
| RXS-0062 | spec/consteval.md | 5 | `conformance/consteval/const_eval_run.rx`, `src/rurixc/src/const_eval.rs`, `src/rurixc/src/mir_build.rs` …(+2) |
| RXS-0063 | spec/consteval.md | 3 | `conformance/consteval/const_eval_run.rx`, `src/rurixc/src/const_eval.rs`, `tests/ui/consteval/overflow_mul.rx` |
| RXS-0064 | spec/consteval.md | 1 | `src/rurixc/src/const_eval.rs` |
| RXS-0065 | spec/consteval.md | 2 | `src/rurixc/src/const_eval.rs`, `tests/ui/consteval/overflow_add.rx` |
| RXS-0066 | spec/device.md | 7 | `conformance/coloring/accept/host_calls_device.rx`, `conformance/coloring/accept/kernel_calls_device.rx`, `conformance/coloring/reject/direct_kernel_call/basic.rx` …(+4) |
| RXS-0067 | spec/device.md | 4 | `conformance/addrspace/accept/matching_space.rx`, `conformance/addrspace/reject/space_mismatch/basic.rx`, `src/rurixc/src/typeck.rs` …(+1) |
| RXS-0068 | spec/device.md | 4 | `conformance/coloring/accept/uniform_barrier.rx`, `conformance/coloring/reject/barrier_non_uniform/basic.rx`, `src/rurixc/src/coloring.rs` …(+1) |
| RXS-0069 | spec/device.md | 2 | `src/rurixc/src/coloring.rs`, `src/rurixc/src/typeck.rs` |
| RXS-0070 | spec/device.md | 3 | `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs`, `tests/ui/codegen/kernel_array_index.rx` |
| RXS-0071 | spec/device.md | 4 | `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs`, `tests/ui/codegen/host_addrspace_view.rx` …(+1) |
| RXS-0072 | spec/device.md | 4 | `conformance/device/reject/threadctx_dim/basic.rx`, `src/rurix-rt/tests/gpu_roundtrip.rs`, `src/rurixc/src/device_codegen.rs` …(+1) |
| RXS-0073 | spec/device.md | 3 | `src/rurixc/src/device_codegen.rs`, `src/rurixc/tests/ptxas_gate.rs`, `tests/ui/codegen/device_string_literal.rx` |
| RXS-0074 | spec/device.md | 10 | `conformance/launch/accept/saxpy_launch.rx`, `conformance/launch/reject/arg_type_mismatch/basic.rx`, `conformance/launch/reject/context_brand_mismatch/basic.rx` …(+7) |
| RXS-0075 | spec/device.md | 10 | `conformance/launch/accept/saxpy_launch.rx`, `conformance/launch/reject/arg_type_mismatch/basic.rx`, `conformance/launch/reject/context_brand_mismatch/basic.rx` …(+7) |
| RXS-0076 | spec/device.md | 2 | `src/rurix-rt/src/lib.rs`, `src/rurix-rt/tests/gpu_roundtrip.rs` |
| RXS-0077 | spec/device.md | 1 | `src/rurix-rt/src/lib.rs` |
| RXS-0078 | spec/device.md | 14 | `conformance/views/accept/chunks_disjoint.rx`, `conformance/views/accept/split_at_disjoint.rx`, `conformance/views/reject/alias_mut_write/basic.rx` …(+11) |
| RXS-0079 | spec/device.md | 10 | `conformance/shared/accept/shared_barrier_consistent.rx`, `conformance/shared/reject/barrier_too_late/basic.rx`, `conformance/shared/reject/unsynced_cross_lane_read/basic.rx` …(+7) |
| RXS-0080 | spec/device.md | 11 | `conformance/atomics/accept/narrower_scope_ok.rx`, `conformance/atomics/accept/scoped_atomics_ok.rx`, `conformance/atomics/reject/scope_addrspace_incompat/basic.rx` …(+8) |
| RXS-0081 | spec/device.md | 4 | `conformance/libdevice/accept/device_math_intrinsics.rx`, `conformance/libdevice/reject/host_math/basic.rx`, `src/rurixc/tests/libdevice_link_mapping.rs` …(+1) |
| RXS-0082 | spec/device.md | 2 | `conformance/libdevice/accept/device_math_intrinsics.rx`, `src/rurixc/tests/libdevice_link_mapping.rs` |
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
| RXS-0158 | spec/dxil_backend.md | 7 | `conformance/dxil/accept/compute_fn_noop.rx`, `conformance/dxil/accept/fragment_noop.rx`, `conformance/dxil/accept/vertex_noop.rx` …(+4) |
| RXS-0159 | spec/dxil_backend.md | 5 | `conformance/dxil/accept/fragment_io.rx`, `conformance/dxil/accept/vertex_io.rx`, `conformance/dxil/reject/builtin_unmappable.rx` …(+2) |
