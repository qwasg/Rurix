// Rurix UC-01 互操作 PYD 绑定(M8.1,D-M8-1;spec/interop.md RXS-0122~0125)。
//
// nanobind 经 scikit-build-core 产 PYD,链接 rurix-interop staticlib(C ABI)。
// PyTorch CUDA 张量经 `__cuda_array_interface__` v3 / DLPack 双协议**零拷贝**抽取
// 设备指针(nb::ndarray<device::cuda> 经 DLPack 导入;或 Python 侧经 CAI 取 data
// 指针传 *_ptr 入口),前向 M5 自研 kernel(SAXPY/Reduction/GEMM)launch。
//
// C ABI(rurix-interop staticlib 导出,RXS-0125;返回码 0=成功 / 7013~7015 互操作
// 诊断段位 / 负=运行时失败)。

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include <nanobind/nanobind.h>
#include <nanobind/ndarray.h>
#include <nanobind/stl/vector.h>
#include <nanobind/stl/string.h>

namespace nb = nanobind;

// rurix-interop staticlib 的 C ABI 导出(RXS-0125)。
extern "C" {
int rurix_uc01_saxpy(uint64_t out, uint64_t x, uint64_t y, float a, uint64_t n);
int rurix_uc01_reduce(uint64_t out, uint64_t x, uint64_t n);
int rurix_uc01_gemm(uint64_t c, uint64_t a, uint64_t b, uint64_t m, uint64_t n,
                    uint64_t k);
}

// 互操作诊断段位(07 §5;含义冻结,与 registry/error_codes.json RX7013~7015 对齐)。
static const char *code_message(int code) {
  switch (code) {
  case 0:
    return "ok";
  case 7013:
    return "RX7013: 互操作协议不支持(对象未暴露 __cuda_array_interface__ v3 / DLPack)";
  case 7014:
    return "RX7014: 设备指针非法(空指针 / 非设备地址)";
  case 7015:
    return "RX7015: 形状不匹配(维度为 0 / 算子维度不相容)";
  default:
    return "互操作运行时/驱动失败";
  }
}

static void check(int code) {
  if (code != 0) {
    throw std::runtime_error(std::string(code_message(code)) +
                             " (code=" + std::to_string(code) + ")");
  }
}

// CUDA f32 设备张量(经 DLPack / CAI 零拷贝导入;.data() = 设备指针)。
using CudaF32 = nb::ndarray<float, nb::device::cuda>;

static uint64_t dev_ptr(const CudaF32 &t) {
  return reinterpret_cast<uint64_t>(t.data());
}

NB_MODULE(rurix_uc01, m) {
  m.doc() = "Rurix UC-01 互操作:rx build --emit=pyd 产 PYD,经 CAI v3 / DLPack "
            "双协议零拷贝接入 PyTorch,复用 M5 自研 kernel 算子替换(RXS-0122~0125)";

  // —— DLPack 协议路径(nb::ndarray 经 torch __dlpack__ 零拷贝导入)——
  m.def(
      "saxpy",
      [](CudaF32 out, CudaF32 x, CudaF32 y, float a) {
        uint64_t n = x.shape(0);
        check(rurix_uc01_saxpy(dev_ptr(out), dev_ptr(x), dev_ptr(y), a, n));
      },
      nb::arg("out"), nb::arg("x"), nb::arg("y"), nb::arg("a"),
      "SAXPY: out = a*x + y(DLPack 零拷贝,复用 M5 saxpy kernel)");

  m.def(
      "reduce",
      [](CudaF32 out, CudaF32 x) {
        uint64_t n = x.shape(0);
        check(rurix_uc01_reduce(dev_ptr(out), dev_ptr(x), n));
      },
      nb::arg("out"), nb::arg("x"),
      "Reduction: out[0] = sum(x)(DLPack 零拷贝,复用 M5 reduce kernel)");

  m.def(
      "gemm",
      [](CudaF32 c, CudaF32 a, CudaF32 b) {
        uint64_t mm = a.shape(0);
        uint64_t kk = a.shape(1);
        uint64_t nn = b.shape(1);
        check(rurix_uc01_gemm(dev_ptr(c), dev_ptr(a), dev_ptr(b), mm, nn, kk));
      },
      nb::arg("c"), nb::arg("a"), nb::arg("b"),
      "GEMM: C[M,N] = A[M,K]·B[K,N] 行主序(DLPack 零拷贝,复用 M5 gemm_tile kernel)");

  // —— __cuda_array_interface__ v3 协议路径(Python 侧取 data 指针传裸地址)——
  m.def("saxpy_ptr", &rurix_uc01_saxpy, nb::arg("out"), nb::arg("x"),
        nb::arg("y"), nb::arg("a"), nb::arg("n"),
        "SAXPY(CAI v3 路径:Python 经 __cuda_array_interface__ 取设备指针)");
  m.def("reduce_ptr", &rurix_uc01_reduce, nb::arg("out"), nb::arg("x"),
        nb::arg("n"), "Reduction(CAI v3 路径)");
  m.def("gemm_ptr", &rurix_uc01_gemm, nb::arg("c"), nb::arg("a"), nb::arg("b"),
        nb::arg("m"), nb::arg("n"), nb::arg("k"), "GEMM(CAI v3 路径)");

  // 内省:支持的算子集 + 双协议名(供 UC-01 冒烟 operators_passed 与协议核对)。
  m.def("operators", []() {
    return std::vector<std::string>{"saxpy", "reduce", "gemm"};
  });
  m.def("protocols", []() {
    return std::vector<std::string>{"__cuda_array_interface__", "dlpack"};
  });
}
