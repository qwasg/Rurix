/*
 * rurix_engine.h — Rurix 引擎集成 C ABI 随附头文件（G1.3，spec/engine_integration.md RXS-0149 /
 * MR-0002）。
 *
 * 与 rurix_engine.dll 导出 ABI **逐一对应**（单一事实源由 crate `EXPORTED_C_ABI` 持有，
 * 一致性由 `cargo test -p rurix-engine c_abi_header_matches_exports` + CI 步骤 43 host 段守卫，
 * 漂移即红）。本头由人工维护以 1:1 兑现 D-113「编译器内建头文件生成」方向；`#[export(c)]`
 * 编译器 codegen + 内建头文件自动生成 defer（RD-009）。
 *
 * 用法（自建最小 C++/D3D12 渲染 harness）：
 *   #include "rurix_engine.h"   // 链接 rurix_engine.dll.lib
 *   设备指针（uint64_t）由宿主在与 CUDA device 同 adapter（LUID 匹配）的 device primary
 *   context 内分配（复用 G1.1 interop 路径，对齐 UC-01 零拷贝设备指针约定）。
 *
 * 返回码（int32_t，07 §5，复用 RXS-0125 既有互操作诊断段位，含义冻结）：
 *   0      = 成功
 *   7013   = 互操作协议不支持（RX7013）
 *   7014   = 设备指针非法（空指针 / 非设备地址，RX7014）
 *   7015   = 形状不匹配（维度为 0 / 算子维度不相容，RX7015）
 *   < 0    = 运行时 / 驱动失败（PTX 装载 / launch / 无 GPU / 无嵌入 PTX）
 *
 * 范围：仅承担 compute pass，不进图形着色阶段 / DXIL（G2，D-131）；永不 Python 原生嵌入
 * （红线 1，SG-008，仅 C ABI 通道）。
 */
#ifndef RURIX_ENGINE_H
#define RURIX_ENGINE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* C ABI 版本（与 crate RX_ENGINE_ABI_VERSION 一致；宿主链接前可经 rurix_engine_abi_version 校核）。 */
#define RURIX_ENGINE_ABI_VERSION 1u

/* 返回引擎集成 C ABI 版本（= RURIX_ENGINE_ABI_VERSION）。 */
uint32_t rurix_engine_abi_version(void);

/* SAXPY compute pass：out[i] = a * x[i] + y[i]（n 个 f32 设备指针；复用 RXS-0125 saxpy）。 */
int32_t rurix_engine_compute_saxpy(uint64_t out, uint64_t x, uint64_t y, float a, uint64_t n);

/* Reduction compute pass：out[0] = Σ x[i]（x 为 n 个 f32 输入，out 为 1 元素标量；复用 RXS-0125 reduce）。 */
int32_t rurix_engine_compute_reduce(uint64_t out, uint64_t x, uint64_t n);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RURIX_ENGINE_H */
