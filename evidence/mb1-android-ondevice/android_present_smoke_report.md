# mb1 Android 真机 on-device present smoke — G-MB1-7 全四要素 measured(RED/GREEN 门控)

> 日期:2026-07-16(晚场,round-2)· 执行:on-device forensics agent(Opus)+ 独立 counter-agent 三路反证 · owner 白栀 提供设备并逐步授权工具下载 · 证据等级:**measured**(真机真跑,门控 RED→GREEN 判定)
> 建树 commit:`5c1b8193`(main,PR #141 mb1 成果包合入点)· 本报告承接同目录 [`android_ondevice_smoke_report.md`](android_ondevice_smoke_report.md)(shell 直跑 compute/offscreen)——本轮经**最小 NativeActivity APK 壳**补齐 present + on-device validation 两要素。**G-MB1-7 尾门维持 open**,不自签(见 §7)。

## 1. 环境

| 项 | 值 |
|---|---|
| 设备 | HONOR BKQ-AN10(arm64-v8a) |
| SoC / GPU | Qualcomm **SM8850**(QTI;Adreno 系)——继 NVIDIA(桌面系统 ICD)与 lavapipe(Mesa CPU)后**第三家厂商驱动** |
| OS | Android 16(SDK 36),`HONOR/BKQ-AN10/HNBKQ:16/HONORBKQ-ANXX/10DLDLD160SP1C00E160:user/release-keys` |
| 物理屏 | WindowManager `Display{#0 state=ON size=1256x2808}`(present ext 独立佐证) |
| 主机工具链 | NDK r27d(27.3.13750724,API 26 clang)+ `rustup target aarch64-linux-android`;`aapt2`/`zipalign -P 16`/`apksigner`(build-tools 36.0.0);JDK 22 |
| 传输 | 无线调试(`adb -s 192.168.3.194:39813`,mDNS 自动发现;USB 链路在持续传输下反复掉线弃用) |
| 部署 | **APK**(`com.rurix.vk` / `android.app.NativeActivity`)——非 shell 直跑;present 需 ANativeWindow |
| 模式协议 | `run-as com.rurix.vk` 写 `files/rurix_mode`(`red`\|`green`)→ 结果 `files/present_result.json` |

### 1.1 provenance(sha256,主机 `certutil` / py -3 重算 == transcript 断言)

| 产物 | sha256 | 大小 |
|---|---|---|
| `app-aligned.apk`(装机包) | `b1b2c99a2b90fa6798a1bae9d9e26d14815ad49729325498eb1994be50938192`(前32 `b1b2c99a2b90fa6798a1bae9d9e26d14` **== 预期 ✓**) | 27,001,350 B |
| `librurix_vk.so`(stage & APK 内嵌逐字节一致 ✓) | `626bc917f84fbb8e1a19cfb24dc0f7d71edd6e52471c2819e2a06372bc91fe30` | 614,416 B |
| `libVkLayer_khronos_validation.so`(vulkan-sdk android-binaries 1.4.350.1) | `34a741d51cb6e9111ec52cda20eee812bcfbcd197348c1404232aacb60e89ef3` | 26,345,704 B |

设备 `pm path com.rurix.vk` = `package:/data/app/~~CFzY2lYo4RoETz_Gdi_e1g==/com.rurix.vk-bSXi3k5dQ0IAhRgK-I7bFA==/base.apk`(与 transcript 逐字一致)。

**库 provenance 回指工作树代码**:RED logcat 逐字吐出的假入口名 `"rurix_red_bogus_entry"` 与 `src/rurix-rt/src/vk.rs` `red_selftest` 分支的源码常量 `c"rurix_red_bogus_entry"` 逐字相同——运行期行为回指该源码,佐证装机库即 pName-00707 机制代码(counter-agent provenance 反证已核:git 工作树仅 MB1/Phase B 文件改动,无越界)。

### 1.2 打包命令序列概要(工具件不入库;逐字见 `scratch\mb1-apk\build_apk.ps1`)

1. 桌面零回归门:`cargo build --workspace` + `ci\vulkan_present_smoke.py`(win32 present 数值零回归)。
2. 交叉 cdylib:`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=<ndk clang>`;`cargo build -p rurix-android-present --target aarch64-linux-android --release` → `librurix_vk.so`。
3. `aapt2 link -o base.apk -I android.jar --manifest AndroidManifest.xml --min-sdk-version 26 --target-sdk-version 36`(无 dex,`hasCode=false`)。
4. `jar uf0 base.apk -C stage lib`——**STORED**(不压缩)加入两个 `.so`(`librurix_vk.so` + layer),满足 16KB 对齐前置。
5. `zipalign -f -P 16 -v 4 base.apk app-aligned.apk` + `-c` 校验(16KB 页对齐)。
6. `apksigner sign`(debug keystore)+ `verify -v`。
7. 装机 `adb install -r -t -g`;RED 先跑(证 layer 真活捕获故意 bug)→ GREEN(零错才有意义)。

### 1.3 AndroidManifest.xml(逐字附录 — scratch,不入库)

```xml
<?xml version="1.0" encoding="utf-8"?>
<!-- mb1 W7 G-MB1-7 Phase B:零-Java NativeActivity 壳 manifest(scratch,不入库)。
     hasCode=false(无 dex);debuggable=true → user-build 亦解锁 in-APK VK_LAYER_KHRONOS_validation;
     extractNativeLibs=true → loader 无条件抽取 lib/arm64-v8a/*.so 为真实文件可发现(layer + cdylib)。
     android.app.lib_name=rurix_vk → 装载 lib/arm64-v8a/librurix_vk.so 并调 ANativeActivity_onCreate。 -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.rurix.vk"
    android:versionCode="1"
    android:versionName="0.0-mb1-w7">

    <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="36" />

    <application
        android:label="RurixVK"
        android:hasCode="false"
        android:debuggable="true"
        android:extractNativeLibs="true">

        <!-- in-APK GPU debug layer 注入解锁(配合 debuggable;主通道 = APK 内 layer .so)。 -->
        <meta-data
            android:name="com.android.graphics.injectLayers.enable"
            android:value="true" />

        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:configChanges="orientation|keyboardHidden|screenSize">
            <meta-data
                android:name="android.app.lib_name"
                android:value="rurix_vk" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

**为何 APK 壳解锁 validation**:前轮(同目录 shell 报告 §4)在 production build(`ro.debuggable=0`)下 shell 进程 `vkCreateInstance` = `-6`(VK_ERROR_LAYER_NOT_PRESENT),`/data/local/debug/vulkan` `Permission denied`。本轮 `android:debuggable="true"` + `com.android.graphics.injectLayers.enable` + APK 内嵌 layer `.so` 是官方旁路——loader 为 debuggable APK 加载 in-APK layer,无需改系统属性。round-2 RED 全量 buffer 佐证 layer 真加载:`D vulkan: added global layer 'VK_LAYER_KHRONOS_validation' from library '.../lib/arm64/libVkLayer_khronos_validation.so'`。

## 2. RED 机制迭代(round-1 → round-2)— 诚实留痕

**round-1(已弃用):字节损坏 SPIR-V → layer 在 Adreno/MTE SIGSEGV。**
vertex `.spv` 喂损坏字节,期望 `VUID-...-pCode-08742`。实际:validation layer 在解析非法 SPIR-V、格式化错误消息的路径上踩已释放/错标指针,被设备 MTE 抓死 → 硬 `SIGSEGV`(signal 11,`SEGV_ACCERR`),故障地址 `0xb400007063e834d4` 落 tagged-pointer/scudo 堆区(use-after-free / MTE-tag 错配签名),**栈顶 6 帧全在 `libVkLayer_khronos_validation.so`**(#00–#05),由 rurix `present_body::{{closure}}`(#06)同步进入。VUID 未吐出即崩,app 未写结果,进程消失 → RED 未按设计变红 → **按协议 HALT**。

> 关键判读:这一崩溃**本身是独立的上游证据**——MTE 在真机上抓到 Khronos validation layer 在非法-SPIR-V 错误格式化路径的真实内存伤,是 layer 上游鲁棒性 bug 被 MTE 硬抓,**非本项目缺陷**。逐字崩溃栈 + 诊断见 [`round1_halt_excerpt.md`](round1_halt_excerpt.md)。

**round-2(采用):合法 SPIR-V + 假入口名 → pName-00707。**
`src/rurix-rt/src/vk.rs` `red_selftest` 分支令 graphics pipeline 的 vertex stage `pName` 用**模块内不存在的假入口名** `c"rurix_red_bogus_entry"`(**SPIR-V 二进制完全合法,不喂非法字节**)。这样 layer 只需比对入口名表即干净报 `VUID-VkPipelineShaderStageCreateInfo-pName-00707`,**天然消除 round-1 的 spec-UB/崩溃路径**——不依赖 layer 解析非法字节的鲁棒性。`green` = 真入口名 `c"main"`。

## 3. RED 结果(pName-00707)— 全 PASS(零报错可信的根基)

`present_result_red.json`(逐字):
```json
{"mode":"red","present_ok":false,"frames":0,"ext_w":0,"ext_h":0,"covered":0,"corner_bg":false,"center_covered":false,"validation_errors":1,"max_err":null}
```

关键 logcat 行(`logcat_red.txt`,逐字):
```
E RurixVK-VVL: vkCreateGraphicsPipelines(): pCreateInfos[0].pStages[0].pName "rurix_red_bogus_entry" entry point not found for stage VK_SHADER_STAGE_VERTEX_BIT. (The only entry point found was "main" for VK_SHADER_STAGE_VERTEX_BIT)
E RurixVK-VVL: The Vulkan spec states: pName must be the name of an OpEntryPoint in module with an execution model that matches stage (https://docs.vulkan.org/spec/latest/chapters/pipelines.html#VUID-VkPipelineShaderStageCreateInfo-pName-00707)
E RurixVK : present err: VK_LAYER_KHRONOS_validation 报 1 条 ERROR 级校验错误(见 logcat RurixVK-VVL;fail-closed)
I RurixVK : RESULT {"mode":"red","present_ok":false,...,"validation_errors":1,...}
```

| criterion | expected | observed | verdict |
|---|---|---|---|
| `present_result.json` 存在 | yes | FOUND_AT=0(≤1s 出现) | PASS |
| `validation_errors > 0` | yes | **1** | PASS |
| logcat 含 VUID-…-pName-00707 | yes | **命中**(RurixVK-VVL,精确 VUID + 假入口名 `"rurix_red_bogus_entry"` + 运行期 module-inspect 细节「The only entry point found was main」) | PASS |
| present fail-closed | yes | `present_ok=false` + "fail-closed" 行 | PASS |
| 进程**不** SIGSEGV | yes | **pid 18300 存活**,全量 buffer(4955 行)`SIGSEGV\|signal 11\|SEGV_\|native_crash` 命中 = **0** | PASS |

**RED 干净变红:validation layer 捕获故意 bug(受控 fail-closed,零崩溃)——round-1 的 layer-crash 失败模式已彻底消除。这是 GREEN「零校验错」有意义的根基:layer 确实会抓 bug,故 GREEN 的 0 不是「layer 没在看」。**

## 4. GREEN 结果 — 全 PASS(两遍逐字段一致)

`present_result_green.json` 与 `present_result_green2.json`(两遍,**字节完全相同**):
```json
{"mode":"green","present_ok":true,"frames":3,"ext_w":1256,"ext_h":2808,"covered":864256,"corner_bg":true,"center_covered":true,"validation_errors":0,"max_err":null}
```
`logcat_green.txt`:RurixVK-VVL 行 = **0**;`pName-00707\|VUID\|Validation Error` 命中 = **0**;`RESULT` 行与 JSON 逐字节相符;pidof 18571 存活。

### present 结构性断言核对表(GREEN 判据)

| criterion | expected | observed | verdict |
|---|---|---|---|
| `present_ok` | true | true | PASS |
| `frames` | 3 | 3 | PASS |
| `covered` | >0 | 864256 | PASS |
| `corner_bg` | true | true | PASS |
| `center_covered` | true | true | PASS |
| `validation_errors` | 0 | 0(两遍) | PASS |
| `ext_w × ext_h` | ~1256×2808 | 1256×2808(== WM `Display{#0 size=1256x2808}`) | PASS |

### 截屏(PNG 不入库,记 scratch 路径 + sha256/尺寸)

| 文件(`scratch\mb1-apk\device-run\`) | sha256 | 尺寸 | 说明 |
|---|---|---|---|
| `present_frame.png` | `07da5e3c7ef81ffd58a2064540e2f57233fa31bf1f1b000e3c23c95f367dba6b` | 1256×2808,2,054,373 B | `am start` 返回瞬截——截到 HONOR 桌面 app-open 过渡动画白卡 + 默认 NativeActivity 图标占位,**非 Vulkan 画面**(am start 早于合成返回,present ~60ms 已完成)。**如实保留,不充证据。** |
| `present_frame_settled.png` | `54cdf0ab7622a6433249d0e7da0f50f0cd68d37dda9d1510b3c53e693ff511a9` | 1256×2808,174,297 B | 设备端 sleep 2s 后截——**真实渲染帧**:插值三角形(左上绿→右上蓝→底红顶点)黑底满屏。四角纯黑(corner_bg)+ 中心彩色覆盖(center_covered)肉眼可证;counter-agent 像素反演覆盖率 ≈0.2506 与进程内 `covered=864256`(0.2451)高度吻合,覆盖 bbox 中心 ≈ 图像中心 → 覆盖块居中,系真实渲染非纯色填充。 |

## 5. 三路独立反证(counter-agent,全「无法反驳」)

| claim | refuted | confidence | 核验要点 |
|---|---|---|---|
| **layer-loaded** | false | high | 全量 buffer `added global layer 'VK_LAYER_KHRONOS_validation'` 真加载;VUID 为 layer 生成(module-inspect 细节 + spec URL),非 app 伪造的 RESULT 行(app RESULT 是独立 RurixVK tag 行);`Fatal signal` 计数=0,pid 存活。GREEN 层加载据同二进制推断(RED/GREEN 同 APK/.so,layer 使能 mode-无关,仅 vertex pName 异)——无「RED 载 layer 而 GREEN 不载」的可信路径。 |
| **present-happened** | false | high | 两遍 result JSON 字节相同;logcat RESULT 逐字匹配;WM `Display size=1256x2808` 独立确认屏分辨率 == ext;截屏 IHDR 1256×2808 + 像素几何/覆盖率反演与 covered 吻合。四路独立证据互印,无矛盾。 |
| **provenance** | false | high | 主机 certutil 重算 APK sha256 前32 == 预期;设备 pm path == transcript;`vk.rs:4671` 命中 `c"rurix_red_bogus_entry"`(red_selftest 分支)运行期回指;getprop `ro.product.model` == BKQ-AN10;git 工作树仅 MB1/Phase B 文件,无越界;round-1/round-2 产物分目录未混淆。唯一未独立核验点(不降级结论):未重建 `.so` 逐字节比对源码新鲜构建——但 RED 运行期精确吐源码同名 VUID 已强佐证装机库=pName-00707 代码。 |

## 6. DoD 四要素终态表(G-MB1-7)

| DoD 要素 | 状态 | 证据 |
|---|---|---|
| ANativeWindow **present** 真跑 N 帧 | ✅ **measured** | §4:frames=3,covered=864256,ext=1256×2808,corner_bg+center_covered,两遍一致 + 截屏实证 |
| `VK_LAYER_KHRONOS_validation` **零报错** | ✅ **measured** | §3 RED 干净捕获故意 bug(validation_errors=1,不崩)证 layer 真活 → §4 GREEN validation_errors=0(两遍)才有意义 |
| **compute** 数值对照(arm64 真机) | ✅ **measured**(上午已档) | 同目录 [`android_ondevice_smoke_report.md`](android_ondevice_smoke_report.md) §2:`vk_saxpy` max_err=0.00e0,与 NVIDIA/lavapipe 逐位一致(三厂商) |
| **logcat + run 证据归档** | ✅ **measured** | 本报告 + `logcat_{red,green}.txt` + `present_result_{red,green,green2}.json` + `round1_halt_excerpt.md`(均入库,LF 归一) |

四要素**全 measured**;RED(真红:layer 干净抓 bug 不崩)与 GREEN(真绿:3 帧零校验错 + 画面实证)证据链内部自洽。

## 7. 签署姿态(G-MB1-7 维持 open,不自签)

- **G-MB1-7 `acceptance_gates` 维持 open**:present + validation + compute + 证据归档四要素虽已全 measured,**签署权归 owner**。agent 不自签硬件尾门——本报告将 G-MB1-7 从「有硬件,余 APK 壳工件」(上午 shell 轮)推进为「**四要素全 measured,待 owner 裁签**」,但 gate 状态由 owner 翻转,非 close-out 自动触发。
- 与技术门 G-MB1-2~5(agent 完全自主签署,evidence-based)区分:两道硬件尾门(G-MB1-6 AMD / G-MB1-7 Android)是本里程碑显式声明的 owner-签署门,agent 只归档 measured 证据、不代签、不伪造。
- 工具件(NDK / build-tools / adb / validation layer `.so` / debug keystore / APK)**不入库**(体积 + 非源);打包脚本 `build_apk.ps1` 逐字记入本 evidence 目录保可复现。CI runner 需同等 provisioning + 真机方可复现真绿。

## 8. 运维备注(荣耀 run-as / adbd 怪癖,复现要点)

- **HONOR run-as + SELinux**:`run-as com.rurix.vk` 的 `runas_app` 上下文**不能走绝对路径** `/data/user/0/com.rurix.vk/...` 或 `/data/data/...`(Permission denied)——一律用 **run-as 家目录的相对路径**(cwd = app data dir),如 `files/rurix_mode`。
- **嵌套 `sh -c` 丢参**:`adb shell` + 本地 shell 会剥掉单引号,`run-as com.rurix.vk sh -c 'ls files'` 变成 `sh -c ls files`(`files` 成 `$0` 非 `ls` 参数)。**整条远程命令须外层双引号包住**:`adb -s <dev> shell "run-as com.rurix.vk sh -c 'cat files/present_result.json'"`。
- **`files/` 免建**:post-install 已存在,`mkdir -p files` 多余。
- **传输**:该机 USB 链路持续传输下反复 offline;**无线调试稳定**(`adb mdns services` 自动发现,首次授权后免配对直连)。掉线 → 重发现;失败如实报。
- **截屏时序**:`am start` 返回早于 surface 合成(present ~60ms 已完成),即截会捕到桌面过渡动画——须**设备端 sleep 2s settle** 后 `exec-out screencap -p` 才得真渲染帧(主机侧禁 sleep,等待用 device 端 sleep 循环)。
- **teardown**:每 run 后 `am force-stop`;`svc power stayon false` 已恢复;未 uninstall。
