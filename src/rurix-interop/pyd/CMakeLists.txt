# Rurix UC-01 互操作 PYD 构建(M8.1,D-M8-1):nanobind 模块链接 rurix-interop
# staticlib(C ABI)+ Rust native-static-libs。由 rx build --emit=pyd / scikit-build-core
# 驱动(spec/interop.md RXS-0122)。
cmake_minimum_required(VERSION 3.15...3.30)
project(rurix_uc01 LANGUAGES CXX)

if(NOT DEFINED RURIX_INTEROP_LIB)
  message(FATAL_ERROR "RURIX_INTEROP_LIB 未设置(rurix-interop staticlib 路径;由 rx build --emit=pyd 注入)")
endif()

find_package(Python 3.10 COMPONENTS Interpreter Development.Module REQUIRED)

# nanobind CMake 包(经已安装的 nanobind 定位,scikit-build-core 环境)。
execute_process(
  COMMAND "${Python_EXECUTABLE}" -c "import nanobind; print(nanobind.cmake_dir())"
  OUTPUT_STRIP_TRAILING_WHITESPACE OUTPUT_VARIABLE nanobind_ROOT)
find_package(nanobind CONFIG REQUIRED)

nanobind_add_module(rurix_uc01 STABLE_ABI src/binding.cpp)

# 链接 Rurix 互操作 staticlib(C ABI)+ Rust 标准库依赖的 Windows native libs
# (cargo rustc --print native-static-libs:kernel32 ntdll userenv ws2_32 dbghelp + msvcrt)。
target_link_libraries(rurix_uc01 PRIVATE
  "${RURIX_INTEROP_LIB}"
  kernel32 ntdll userenv ws2_32 dbghelp)

install(TARGETS rurix_uc01 LIBRARY DESTINATION .)
