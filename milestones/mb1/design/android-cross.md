I have everything needed. Environment probe is conclusive and I've read all the load-bearing files. Here is the implementation design.

---

# Phase 4 Android 交叉编译 — 实现设计 (RXS-0211)

## 0. 环境探测结论 (dev-env degrade)

只读探测,均**缺位**:
- `ANDROID_NDK_HOME` / `ANDROID_NDK_ROOT` / `ANDROID_HOME`:全空。
- 常见路径 `%LOCALAPPDATA%\Android\Sdk\ndk`、`C:\Android\Sdk\ndk`、`C:\Program Files\Android`:全无。
- `rustup target list --installed` → 仅 `x86_64-pc-windows-msvc`;**无** `aarch64-linux-android`。

⟹ 本机进入 **dev-env degrade**:交叉构建门(下述 `ci/vulkan_android_build_smoke.py`)在本机必 **SKIP**(非 fake pass);真交叉 build 绿归 owner 侧的 NDK-在位 runner 或本地装 NDK 后验证。达标定义见 §4。

**关键单点阻塞确认**:`vk.rs:475-478` 的 `unsafe extern "system" { fn LoadLibraryA; fn GetProcAddress; }` 是全模块**唯一**链接期 OS 符号(其余 ~35 Vulkan 命令全经 `vkGetInstanceProcAddr` 动态解析,零链接期 Vulkan 符号)。故当前 `cargo build -p rurix-rt --features vulkan --target aarch64-linux-android` 会在链接 `vk_saxpy` bin 时因 `LoadLibraryA`/`GetProcAddress` 无定义而 **fail** —— 这正是 Phase 4 要消除的唯一缝。`extern "system"` 在 aarch64-android == AAPCS64 == `extern "C"`,故 ~35 个 `Fn*` 函数指针类型**零改动**即在 Android ABI 正确。

---

## 1. OS 加载缝抽象 (vk.rs)

**现状**(`src/rurix-rt/src/vk.rs:473-507`):Win32 `LoadLibraryA(c"vulkan-1.dll")` + `GetProcAddress`,无条件编译。

**改动**:把 OS 加载原语抽进 cfg 分支的 `loader` 子模块,`load_vulkan_loader` 只调抽象 `open`/`sym` + per-OS 库名常量。Windows 路径逻辑**逐字节等价**(零漂移)。

### 1a. 替换 `vk.rs:475-478` 的 extern 块 → cfg 分叉子模块

```rust
// ── OS 动态加载缝(跨端;镜像 sys.rs 无外部依赖纪律) ───────────────────────────
// Windows:      vulkan-1.dll  / LoadLibraryA + GetProcAddress(Win32 kernel32)。
// Android+Linux: libvulkan.so / dlopen(RTLD_NOW) + dlsym(libc;Android 由 libc 直接
//                提供 dlopen/dlsym,NDK 默认链接;现代 glibc 亦并入 libc,无需 -ldl)。
#[cfg(windows)]
mod loader {
    use core::ffi::{c_char, c_void, CStr};
    unsafe extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    pub(super) const VULKAN_LIB: &CStr = c"vulkan-1.dll";
    /// # Safety: `name` 为 NUL 结尾字面量。
    pub(super) unsafe fn open(name: *const c_char) -> *mut c_void {
        LoadLibraryA(name)
    }
    /// # Safety: `lib` 为 `open` 返回的有效模块句柄或 null;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        GetProcAddress(lib, name)
    }
}

#[cfg(not(windows))]
mod loader {
    use core::ffi::{c_char, c_void, CStr};
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: i32 = 2; // 立即绑定全部符号(POSIX 通用值,Android/glibc/musl 一致)。
    pub(super) const VULKAN_LIB: &CStr = c"libvulkan.so";
    /// # Safety: `name` 为 NUL 结尾字面量。
    pub(super) unsafe fn open(name: *const c_char) -> *mut c_void {
        dlopen(name, RTLD_NOW)
    }
    /// # Safety: `handle` 为 `open` 返回的有效句柄或 null;`name` NUL 结尾。
    pub(super) unsafe fn sym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
        dlsym(handle, name)
    }
}
```

### 1b. 重写 `load_vulkan_loader`(`vk.rs:494-507`)

```rust
fn load_vulkan_loader() -> Option<FnGetInstanceProcAddr> {
    // SAFETY: open/sym 为各 OS 稳定 ABI 加载原语(Win32 LoadLibraryA / POSIX dlopen);
    // 入参 NUL 结尾字面量;返回地址经 null 校验后 transmute 为已知 ABI 的函数指针。
    // loader 不 close/FreeLibrary —— 进程生命周期常驻(镜像 sys.rs U1 nvcuda.dll 纪律)。
    unsafe {
        let lib = loader::open(loader::VULKAN_LIB.as_ptr());
        if lib.is_null() {
            return None;
        }
        let p = loader::sym(lib, c"vkGetInstanceProcAddr".as_ptr());
        if p.is_null() {
            return None;
        }
        Some(std::mem::transmute::<*mut c_void, FnGetInstanceProcAddr>(p))
    }
}
```

**零漂移保证**:Windows 分支 `open`=`LoadLibraryA`、`sym`=`GetProcAddress`、`VULKAN_LIB`=`vulkan-1.dll` —— 与现行控制流逐调用等价,NVIDIA/桌面路径行为字节不变。`vk.rs` 顶部 `use core::ffi::{c_char, c_void};`(`vk.rs:21`)保留;新子模块各自 `use`。

> `run_compute` 内 `vulkan-1.dll`/`libvulkan` 缺失 → 现有 `.ok_or("vulkan-1.dll / vkGetInstanceProcAddr 不可用")`(`vk.rs:558`)建议改文案为 `"vulkan loader (vulkan-1.dll/libvulkan.so) 不可用"`,使 device_smoke 的 `no_device` 关键字表(已含 `"libvulkan"`)在两 OS 都命中。

---

## 2. NDK 交叉编译接线

本机无 NDK/target → 标 **dev-env degrade SKIP**;下方为 NDK-在位时的接线(committed 到仓,不含机器绝对路径)。

### 2a. 新建 `.cargo/config.toml`(仓根,LF 新文件)

```toml
# Android aarch64 交叉编译(mb1 Phase 4,RXS-0211)。
# NDK clang 包装器作 linker + llvm-ar 作 ar;仅 `--target aarch64-linux-android` 命中时读取,
# 桌面/NVIDIA 构建零影响(其他 target 不触此段)。
# 包装器名依赖 NDK 的 .../toolchains/llvm/prebuilt/<host>/bin 在 PATH;CI 经环境变量
# CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER 注入绝对路径覆盖本项(见 ci/vulkan_android_build_smoke.py),
# 保 hermetic;本文件为本地开发者便利默认。API 26(≥ Vulkan/ANativeWindow 最低 24)。
[target.aarch64-linux-android]
linker = "aarch64-linux-android26-clang"
ar = "llvm-ar"
```

Windows host 上 NDK 的 clang 包装器名为 `aarch64-linux-android26-clang.cmd`;CI 脚本按 host 选后缀并以**绝对路径 env 覆盖**(env > config.toml),故 committed 文件用无后缀名即可,不阻塞跨 host。

### 2b. 前置步骤(文档 + CI 脚本自动化)

1. `rustup target add aarch64-linux-android`(装 std 预编)。
2. NDK r23+(纯 LLVM;r25/r26 优)。
3. 环境暴露 `ANDROID_NDK_HOME`(或 `ANDROID_NDK_ROOT`)。
4. 构建:`cargo build -p rurix-rt --features vulkan --target aarch64-linux-android`(lib + `vk_saxpy` bin 全链接绿)。

CI 侧不依赖 committed config.toml 的 PATH 假设,改用绝对路径 env(§5b)。

---

## 3. ANativeWindow present 代码就位 (cfg(target_os="android"))

`present` 与 `run_compute` **正交**:compute 路径不需 surface,故交叉 build 的 compute+bin 不被 present 复杂度拖累;present 仅在 android target 编译,on-device 验证归 G-MB1-7。在 `vk.rs` 末尾(`tests` 模块前)追加:

```rust
// ── Android present 缝(VK_KHR_android_surface;on-device 尾门 G-MB1-7) ────────
// run_compute 语义与本模块无关(compute 不需 surface);此处仅就位 surface 创建 FFI,
// 使 android target 编译绿。完整 swapchain acquire→submit→present 循环为 on-device 尾门。
#[cfg(target_os = "android")]
pub mod android_present {
    use core::ffi::c_void;

    /// 由 Android app(NativeActivity / GameActivity)经 JNI/native glue 提供的不透明窗口。
    #[repr(C)]
    pub struct ANativeWindow {
        _private: [u8; 0],
    }

    type VkInstance = *mut c_void;
    type VkSurfaceKHR = u64;
    const ST_ANDROID_SURFACE_CREATE_INFO_KHR: u32 = 1_000_008_000;

    #[repr(C)]
    struct AndroidSurfaceCreateInfoKHR {
        s_type: u32,
        p_next: *const c_void,
        flags: u32,
        window: *mut ANativeWindow,
    }

    type FnCreateAndroidSurfaceKHR = unsafe extern "system" fn(
        VkInstance,
        *const AndroidSurfaceCreateInfoKHR,
        *const c_void,
        *mut VkSurfaceKHR,
    ) -> i32;

    /// 从 ANativeWindow* 建 VkSurfaceKHR。要求 instance 已启用扩展
    /// `VK_KHR_surface` + `VK_KHR_android_surface`(present 路径 vkCreateInstance 时启用;
    /// compute 路径不启用,故 run_compute 的 InstanceCreateInfo 保持 enabled_extension_count=0)。
    ///
    /// # Safety
    /// `instance` 为有效 VkInstance;`window` 为 Android app 存活期内的有效 ANativeWindow*;
    /// `create_fn` 为 vkGetInstanceProcAddr(instance,"vkCreateAndroidSurfaceKHR") 解析所得。
    pub unsafe fn create_android_surface(
        instance: VkInstance,
        window: *mut ANativeWindow,
        create_fn: FnCreateAndroidSurfaceKHR,
    ) -> Result<VkSurfaceKHR, String> {
        let ci = AndroidSurfaceCreateInfoKHR {
            s_type: ST_ANDROID_SURFACE_CREATE_INFO_KHR,
            p_next: core::ptr::null(),
            flags: 0,
            window,
        };
        let mut surface: VkSurfaceKHR = 0;
        // SAFETY: ci 布局与 VkAndroidSurfaceCreateInfoKHR 逐字节对齐;window 由调用方担保有效。
        let r = create_fn(instance, &ci, core::ptr::null(), &mut surface);
        if r != 0 {
            return Err(format!("vkCreateAndroidSurfaceKHR 失败: {r}"));
        }
        Ok(surface)
    }
}
```

**平台无关逻辑可单测**:`entry_point_name`(`vk.rs:512`,已 RXS-0207 host 单测)+ push-constant/buffer 字节编排(`bin/vk_saxpy.rs` 的 `to_bytes`/`to_f32`)本就 device-free。新增 RXS-0211 锚点单测(§5c)覆盖加载缝库名选择,不触设备。

---

## 4. 构建绿 vs 设备 pending-hardware 边界

**达标(RXS-0211 DoD;"构建绿即达标")**:
1. `cargo build -p rurix-rt --features vulkan --target aarch64-linux-android` **链接绿**(lib + `vk_saxpy` bin;`LoadLibraryA` 缝消除后无未定义符号)。
2. `#[cfg(target_os="android")]` present 缝随 android target **编译绿**。
3. 平台无关单测 host **绿**:`entry_point_name_parses`(现存)+ 新 `loader_seam_selects_platform_lib`(RXS-0211 锚)。
4. **NVIDIA(CUDA)零回归**:default features 不变、`cargo build/test -p rurix-rt`(无 `--features vulkan`)零改动;Windows `vk.rs` 加载路径逐字节等价。
5. `ci/vulkan_android_build_smoke.py`:NDK+target 在位 → 交叉 build 绿;缺 → SKIP。

**pending-hardware(G-MB1-7,open 尾门)**:
- 真 Android 设备/模拟器上 `libvulkan.so` 存在、saxpy compute 数值精确回读。
- ANativeWindow surface + swapchain 真出图可见。
- 无 android runner,故 **不由 `RURIX_REQUIRE_REAL` 强制**(那是桌面 device 门);另设 `RURIX_REQUIRE_ANDROID=1` 仅在未来专用 android-build runner 上把"缺 NDK"翻硬红(仍不覆盖 on-device,on-device 恒 G-MB1-7 open)。

---

## 5. RXS-0211 条款体 + CI 交叉构建门

### 5a. 条款体(`spec/vulkan_backend.md`,§2,插在 `### RXS-0207` 后,即 `vk.rs` 修订记录 `## 3` 之前 —— 数字序,补上区间已声明但未落体的 0211)

`### RXS-0211 Android 移植缝与交叉构建`,FLS 分节,**严禁 UB 节**:
- **Syntax**:无新语法(运行时/工具链面)。
- **Legality**:
  - L1 OS 加载缝:`cfg(windows)` → `vulkan-1.dll`/`LoadLibraryA`;`cfg(not(windows))` → `libvulkan.so`/`dlopen(RTLD_NOW)`。库名与加载原语 per-OS 唯一确定。
  - L2 present:`cfg(target_os="android")` 经 `VK_KHR_android_surface` 从 `ANativeWindow*` 建 `VkSurfaceKHR`;compute 路径不启用 surface 扩展。
  - L3 构建降级:缺 NDK/`aarch64-linux-android` target → 交叉构建门 **SKIP**(dev-env 降级,非 fake pass)。
- **Dynamic Semantics**:加载缝对 `run_compute` **语义中性** —— 同一 Phase 1 `.spv` 在桌面与 Android 消费,SPIR-V 字流与 compute 结果不因 OS 改变(承 RXS-0207 marshalling 单一事实源)。Android present 为 on-device 语义,属尾门 G-MB1-7。
- **Implementation Requirements**:
  - IR1 cfg-gated loader,Windows 路径**零漂移**(逐调用等价现行 `load_vulkan_loader`)。
  - IR2 交叉 build 绿:`-p rurix-rt --features vulkan --target aarch64-linux-android` lib+bin 链接无未定义符号。
  - IR3 平台无关单测(`entry_point_name` + 加载缝库名)host 绿;present 缝 android 编译绿。
  - IR4 ≥1 `//@ spec: RXS-0211` 测试锚定(`ci/trace_matrix.py --check` 全锚定 +1)。
  - IR5 NVIDIA(CUDA)零回归:default 构建/测试字节不变。

修订记录追加 `| v1.6 | 2026-07-15 | MB1.3 Android 移植缝:落 RXS-0211 …` 一行(体例照 v1.5)。

### 5b. 新建 `ci/vulkan_android_build_smoke.py`(步骤 56)

```python
#!/usr/bin/env python3
"""mb1 Android 交叉构建冒烟(RXS-0211;RFC-0011 §4)。

NDK + aarch64-linux-android target 在位 → cross-build `rurix-rt --features vulkan`(lib+bin)
链接绿;缺 → SKIP(dev-env 降级,非 fake)。RURIX_REQUIRE_ANDROID=1(专用 android-build
runner)时缺 NDK/target 翻硬红。**不触设备**(on-device saxpy/present = G-MB1-7 open,
不在本门)。退出码判定(cargo build returncode)。
"""
import os, subprocess, sys, glob
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = "aarch64-linux-android"
API = "26"

def find_ndk():
    for k in ("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"):
        v = os.environ.get(k)
        if v and Path(v).is_dir():
            return Path(v)
    sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if sdk:
        cands = sorted(glob.glob(str(Path(sdk) / "ndk" / "*")))
        if cands:
            return Path(cands[-1])
    return None

def has_target():
    r = subprocess.run(["rustup", "target", "list", "--installed"],
                       capture_output=True, text=True)
    return TARGET in r.stdout

def clang_bin(ndk):
    host = "windows-x86_64" if sys.platform == "win32" else \
           ("darwin-x86_64" if sys.platform == "darwin" else "linux-x86_64")
    bindir = ndk / "toolchains" / "llvm" / "prebuilt" / host / "bin"
    ext = ".cmd" if sys.platform == "win32" else ""
    linker = bindir / f"{TARGET}{API}-clang{ext}"
    ar_ext = ".exe" if sys.platform == "win32" else ""
    ar = bindir / f"llvm-ar{ar_ext}"
    return bindir, linker, ar

def main():
    require = os.environ.get("RURIX_REQUIRE_ANDROID") == "1"
    ndk, tgt = find_ndk(), has_target()
    if not ndk or not tgt:
        msg = f"NDK={'ok' if ndk else 'missing'} target={'ok' if tgt else 'missing'}"
        if require:
            print(f"[vk_android] FAIL 要求 android 构建但环境缺:{msg}", file=sys.stderr)
            return 1
        print(f"[vk_android] SKIP dev-env 降级(非 fake):{msg}")
        return 0
    bindir, linker, ar = clang_bin(ndk)
    if not linker.is_file():
        print(f"[vk_android] {'FAIL' if require else 'SKIP'} NDK clang 缺: {linker}",
              file=sys.stderr)
        return 1 if require else 0
    env = dict(os.environ)
    env["PATH"] = str(bindir) + os.pathsep + env.get("PATH", "")
    env["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"] = str(linker)
    env["CARGO_TARGET_AARCH64_LINUX_ANDROID_AR"] = str(ar)
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan",
         "--target", TARGET, "--quiet"],
        cwd=ROOT, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        print(f"[vk_android] FAIL 交叉 build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        return 1
    print(f"[vk_android] PASS {TARGET} 交叉 build 绿(lib+vk_saxpy;on-device=G-MB1-7 open)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

### 5c. 平台无关锚点单测(`vk.rs` 的 `#[cfg(test)] mod tests`,追加)

```rust
//@ spec: RXS-0211
#[test]
fn loader_seam_selects_platform_lib() {
    // OS 加载缝库名 per-OS 唯一(cfg 选择正确);不触设备,纯 host。
    let expected = if cfg!(windows) { "vulkan-1.dll" } else { "libvulkan.so" };
    assert_eq!(loader::VULKAN_LIB.to_str().unwrap(), expected);
    // 平台无关的 entry-name 编排(桌面/Android 共用同一 .spv 消费路径)在两 OS 一致。
    let spv = [0x0723_0203u32, 0x0001_0000, 0, 5, 0];
    assert_eq!(super::entry_point_name(&spv), None); // 无 OpEntryPoint → None,确定性。
}
```

### 5d. `pr-smoke.yml` 步骤 56(插在步骤 55 `vulkan_device_smoke` 后,`budget_eval` 前)

```yaml
      # 步骤 56（mb1,契约 G-MB1-7,RFC-0011 / RXS-0211）:Android 交叉构建冒烟。
      # `ci/vulkan_android_build_smoke.py`:NDK + aarch64-linux-android target 在位 →
      # cross-build rurix-rt --features vulkan(lib+vk_saxpy)链接绿;缺 → SKIP(dev-env
      # 降级,非 fake)。**不触设备**——on-device saxpy/present = G-MB1-7 open 尾门,
      # 无 android runner 故不设 RURIX_REQUIRE_REAL;专用 android-build runner 经
      # RURIX_REQUIRE_ANDROID=1 把缺 NDK 翻硬红。NVIDIA runner 无 NDK → 干净 SKIP。
      - name: vulkan android cross-build smoke (MB1 CI_GATES §2.56, G-MB1-7, RFC-0011)
        shell: powershell
        run: py -3 ci/vulkan_android_build_smoke.py
```

注意:**不加** `RURIX_REQUIRE_REAL: "1"`,否则现 NVIDIA runner(无 NDK)会误红。SKIP 是该 runner 的正确态。

---

## 6. 精确改动清单

| # | 文件 | 位置 | 动作 |
|---|------|------|------|
| 1 | `src/rurix-rt/src/vk.rs` | `:475-478` | 删 `unsafe extern "system" { LoadLibraryA; GetProcAddress; }`,替为 §1a 的 `#[cfg(windows)] mod loader` + `#[cfg(not(windows))] mod loader`(dlopen/dlsym)。 |
| 2 | `src/rurix-rt/src/vk.rs` | `:494-507` | 重写 `load_vulkan_loader`(§1b):调 `loader::open/sym/VULKAN_LIB`。 |
| 3 | `src/rurix-rt/src/vk.rs` | `:558` | Err 文案改 `"vulkan loader (vulkan-1.dll/libvulkan.so) 不可用"`(可选,利 no_device 命中)。 |
| 4 | `src/rurix-rt/src/vk.rs` | tests 前 | 追加 §3 的 `#[cfg(target_os="android")] pub mod android_present`。 |
| 5 | `src/rurix-rt/src/vk.rs` | tests 模块内 | 追加 §5c 的 `//@ spec: RXS-0211` 单测。 |
| 6 | `.cargo/config.toml` | 仓根(**新建 LF**) | §2a `[target.aarch64-linux-android]` linker/ar。 |
| 7 | `ci/vulkan_android_build_smoke.py` | **新建 LF** | §5b 交叉构建门。 |
| 8 | `.github/workflows/pr-smoke.yml` | `:430` 后(步骤 55↔budget 之间) | §5d 步骤 56(**无** `RURIX_REQUIRE_REAL`)。 |
| 9 | `spec/vulkan_backend.md` | `## 3` 修订记录前(RXS-0207 后) | §5a `### RXS-0211` 条款体 + 修订记录 v1.6 一行。 |
| 10 | `spec/README.md` | §4 CI_GATES / 文件清单 | 登记步骤 56 `vulkan_android_build_smoke.py` + G-MB1-7(照步骤 54/55 体例)。 |
| 11 | `unsafe-audit/rurix-rt.md` | U26 条目 | 扩注:加载缝 cfg 分叉(Win32 `LoadLibraryA` / POSIX `dlopen(RTLD_NOW)`)+ android_present `vkCreateAndroidSurfaceKHR` FFI,均在 U26 Vulkan FFI 边界内(无需新 U 号,同一 feature `vulkan` 边界)。 |

**纪律核对**:
- LF 新文件(#6/#7);既有文件(#1-5/#8-11)Edit 工具改,CRLF/LF 保形。
- NVIDIA 零回归:`vk.rs` gate 于 feature `vulkan`(默认关),default `cargo build/test -p rurix-rt` 零触;Windows 加载路径逐字节等价。
- `ci/trace_matrix.py` 全锚定 **191→192**(RXS-0211 新增 ≥1 `//@ spec`,无悬空锚点、无裸条款头)。
- 编号:RXS-0211 属已声明预留区间(`spec/vulkan_backend.md:35` 明列"0211 Android 移植缝"),本轮落体,非新造。
- 无新错误码(6xxx 段不预造);Full RFC(RFC-0011)档位维持;gated on 红线 3 解除 + RFC-0011 批准,合入归 owner。

**验证命令**:
```
# host(本机可跑,验零回归 + 平台无关单测):
cargo test -p rurix-rt --features vulkan --lib          # entry_point_name + loader_seam 绿
cargo build -p rurix-rt                                 # default,NVIDIA 路零改动绿
cargo clippy -p rurix-rt --features vulkan -- -D warnings
py -3 ci/vulkan_android_build_smoke.py                  # 本机 → SKIP(NDK 缺)
py -3 ci/trace_matrix.py --check                        # 全锚定 192

# NDK-在位环境(owner 侧 / 本地装 NDK 后,验交叉 build 绿 = 达标):
rustup target add aarch64-linux-android
$env:ANDROID_NDK_HOME="<ndk>"; py -3 ci/vulkan_android_build_smoke.py   # → PASS
cargo build -p rurix-rt --features vulkan --target aarch64-linux-android # lib+vk_saxpy 链接绿
```

on-device saxpy 数值精确回读 + ANativeWindow present 出图 = **G-MB1-7 open**(无 android runner,真机尾门,不纳入本轮达标)。
