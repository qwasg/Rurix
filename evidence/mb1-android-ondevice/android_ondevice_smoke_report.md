# mb1 Android 真机 on-device smoke — G-MB1-7 进度证据(compute + offscreen graphics)

> 日期:2026-07-16 · 执行:agent(owner 白栀 在场提供设备并逐步授权)· 证据等级:**measured**(真机真跑,退出码判定)
> 建树 commit:`5c1b8193`(main,PR #141 mb1 成果包合入点)· 本报告为 **G-MB1-7 部分进度**,尾门**维持 open**(见 §5 诚实边界)。

## 1. 环境

| 项 | 值 |
|---|---|
| 设备 | HONOR BKQ-AN10(arm64-v8a) |
| SoC / GPU | Qualcomm **SM8850**(QTI;Adreno 系)——继 NVIDIA(桌面系统 ICD)与 lavapipe(Mesa CPU)后**第三家厂商驱动** |
| OS | Android 16(SDK 36),production build(`ro.debuggable=0`,release-keys) |
| `libvulkan.so` | `/system/lib64/libvulkan.so` 在位 |
| 主机工具链 | NDK r27d(27.3.13750724,API 26 clang)+ `rustup target aarch64-linux-android`;release 构建 |
| 传输 | 无线调试(`adb connect 192.168.3.194:39729`,mDNS 自动发现;USB 链路在持续传输下反复掉线弃用) |
| 部署路径 | `/data/local/tmp/rurix/`(shell 直跑,非 APK) |

产物 provenance(sha256 前 16 位;`.spv` 均为主机 `rurixc --target vulkan` 产出且 spirv-val accepted,与桌面消费**同一字节流**,承 RXS-0207/0211 语义中性):

| 产物 | sha256[:16] |
|---|---|
| `saxpy.spv`(`conformance/vulkan/accept/vk_saxpy.rx`) | `f3be0dbdb2aa6fd5` |
| `tri_vs.spv` / `tri_fs.spv`(`vk_tri_vs.rx` / `vk_tri_fs.rx`) | `4a1077a1332feb27` / `be68099179067f62` |
| `vk_saxpy`(aarch64 release bin) | `15e2c79a79d2273e` |
| `vk_triangle`(aarch64 release bin) | `4820c0b208137843` |

## 2. compute 腿(RXS-0207)— **PASS(数值逐位精确)**

```
$ ./vk_saxpy saxpy.spv
VK_SAXPY: ok entry=rx_saxpy_8 n=1024 a=2 out[0]=0 out[1]=2.5 out[1023]=2557.5 max_err=0.00e0
EXIT_CODE=0
```

复跑第二次结果逐字节一致。**跨厂商数值对照**:`out[0]/out[1]/out[1023]/max_err` 与桌面 NVIDIA(系统 ICD)及 lavapipe(CPU)先前归档值(§8 W1~W8 记录)**全等**——同一 `.spv` 三家驱动(NVIDIA / Mesa-lavapipe / Qualcomm-Adreno)数值逐位一致,saxpy 确定性跨端成立。

## 3. offscreen graphics 腿(RXS-0210 L1~L3 面)— **PASS(像素校验与桌面全等)**

```
$ ./vk_triangle tri_vs.spv tri_fs.spv
VK_TRIANGLE: ok W=64 H=64 covered=968 center=(130,59,65)
EXIT_CODE=0
```

复跑第二次一致。**covered=968 与桌面 NVIDIA 归档值(MB1_CONTRACT §8:offscreen covered=968)完全一致**,中心像素插值 `(130,59,65)` 通过 demo 内建断言(背景/中心/插值三段校验)。

## 4. validation layer 尝试 — **BLOCKED(OS 策略,非本项目缺陷;诚实不充绿)**

- 推送 Khronos 官方 `libVkLayer_khronos_validation.so`(vulkan-sdk-1.4.350.1,arm64-v8a,sha256 与主机核对一致)→ `RURIX_VK_VALIDATION=1` + `VK_LAYER_PATH` 运行 → `vkCreateInstance` = **-6(VK_ERROR_LAYER_NOT_PRESENT)**。
- 根因:Android production build(`ro.debuggable=0`)的 Vulkan loader **不为 shell 进程加载外部 layer**;官方旁路 `/data/local/debug/vulkan/` 亦 `Permission denied`。
- **结论:on-device validation 零报错证据须经 debuggable APK + `enable_gpu_debug_layers` 官方通道(Phase B),本轮不伪造、不充绿。** 桌面双 ICD 的 validation 零报错证据(§8 既档)不受影响。
- logcat 附注:layer 尝试运行时见厂商 loader 行 `E vulkan: No find igraphics vulkan lib`(荣耀定制 loader 探测日志,良性);两条 PASS 腿运行期间 logcat 无 Vulkan 相关报错。

## 5. 诚实边界(G-MB1-7 维持 open)

DoD 四要素对照:

| DoD 要素 | 状态 |
|---|---|
| arm64 真机装载交叉产物 + compute 数值对照 | ✅ 本报告 §2(measured) |
| ANativeWindow present 真跑 N 帧 | ❌ **未跑**——shell 进程无 ANativeWindow,需最小 NativeActivity APK 壳(Phase B 新工件) |
| VK_LAYER_KHRONOS_validation 零报错 | ❌ **BLOCKED**(§4,OS 策略;归 Phase B APK 通道) |
| logcat + run 证据归档 | ✅ 本报告 + 原始运行样本(见 §6) |

**G-MB1-7 不签**:present 与 on-device validation 两要素未达。本报告将尾门从「缺硬件 pending-hardware」推进为「有硬件,余 APK 壳工件」(offscreen graphics 为 DoD 外的附加跨厂商证据)。

## 6. 运维备注(复现要点)

- 该机 USB 链路持续传输下反复 offline;**无线调试稳定**(`adb mdns services` 自动发现,USB 首次授权后免配对直连,实测 20~100MB/s)。
- **荣耀定制 adbd 怪癖**:`adb push <file> <目录>/` 形式会截断远端文件名尾部 2 字符(`tri_vs.spv`→`tri_vs.s`);**推送须显式指定远端全名** `adb push <file> <目录>/<全名>`。
- 复跑命令序列:主机 `rurixc --target vulkan <src.rx> -o <out.spv>` → `adb push`(显式全名)→ `adb shell "cd /data/local/tmp/rurix && ./vk_saxpy saxpy.spv"`,退出码判定。
