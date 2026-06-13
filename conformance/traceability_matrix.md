# spec 条款 ↔ 测试锚定矩阵(生成物,勿手改)

> 生成:`py -3 ci/trace_matrix.py`(G-M1-4;每条款 ≥1 测试锚定,10 §4)。

| 条款 | spec 文件 | 锚定测试数 | 锚定 |
|---|---|---|---|
| RXS-0001 | spec/lexical.md | 1 | `src/rurixc/src/lexer.rs` |
| RXS-0002 | spec/lexical.md | 1 | `conformance/syntax/comments.rx` |
| RXS-0003 | spec/lexical.md | 2 | `conformance/syntax/comments.rx`, `conformance/syntax/comments_between_items.rx` |
| RXS-0004 | spec/lexical.md | 3 | `conformance/syntax/fn_basic.rx`, `conformance/syntax/hello_world.rx`, `conformance/syntax/idents_keywords.rx` |
| RXS-0005 | spec/lexical.md | 26 | `conformance/syntax/atomics_sync.rx`, `conformance/syntax/buffers_context.rx`, `conformance/syntax/closures_and_calls.rx` …(+23) |
| RXS-0006 | spec/lexical.md | 4 | `conformance/syntax/buffers_context.rx`, `conformance/syntax/const_generics.rx`, `conformance/syntax/literals_int.rx` …(+1) |
| RXS-0007 | spec/lexical.md | 2 | `conformance/syntax/literals_float.rx`, `conformance/syntax/vec_mat_swizzle.rx` |
| RXS-0008 | spec/lexical.md | 8 | `conformance/syntax/buffers_context.rx`, `conformance/syntax/export_c.rx`, `conformance/syntax/ffi_extern.rx` …(+5) |
| RXS-0009 | spec/lexical.md | 12 | `conformance/syntax/atomics_sync.rx`, `conformance/syntax/closures_and_calls.rx`, `conformance/syntax/control_flow.rx` …(+9) |
| RXS-0010 | spec/lexical.md | 1 | `src/rurixc/src/lexer.rs` |
| RXS-0011 | spec/syntax.md | 5 | `conformance/syntax/comments_between_items.rx`, `conformance/syntax/items_mix.rx`, `src/rurixc/src/parser.rs` …(+2) |
| RXS-0012 | spec/syntax.md | 6 | `conformance/syntax/attrs_meta.rx`, `conformance/syntax/attrs_on_items.rx`, `conformance/syntax/export_handles.rx` …(+3) |
| RXS-0013 | spec/syntax.md | 5 | `conformance/syntax/paths_expr.rx`, `conformance/syntax/turbofish_nested.rx`, `conformance/syntax/visibility_levels.rx` …(+2) |
| RXS-0014 | spec/syntax.md | 9 | `conformance/syntax/const_fn_eval.rx`, `conformance/syntax/device_math_chain.rx`, `conformance/syntax/fn_colors.rx` …(+6) |
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
| RXS-0030 | spec/syntax.md | 3 | `src/rurixc/src/parser.rs`, `tests/ui/parse/missing_semi.rx`, `tests/ui/parse/unclosed_brace.rx` |
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
| RXS-0052 | spec/borrow.md | 3 | `conformance/desugar/desugar_run_smoke.rx`, `conformance/desugar/drop_scope_blocks.rx`, `src/rurixc/src/tbir_build.rs` |
| RXS-0053 | spec/borrow.md | 1 | `conformance/borrowck/accept/copy_types.rx` |
| RXS-0054 | spec/borrow.md | 1 | `conformance/borrowck/accept/move_reinit.rx` |
| RXS-0055 | spec/borrow.md | 1 | `conformance/borrowck/accept/drop_order_run.rx` |
| RXS-0056 | spec/borrow.md | 1 | `conformance/borrowck/accept/temp_drop_stmt.rx` |
