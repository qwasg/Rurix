# Round-1 HALT 摘录 — 字节损坏 SPIR-V → validation layer 在 Adreno/MTE SIGSEGV

> 日期:2026-07-16(17:00 场,round-1)· 设备:HONOR BKQ-AN10(SM8850/Adreno,Android 16/SDK 36,`user/release-keys`)
> 状态:**HALT**（RED 未按设计变红——layer 在处理故意错误时自身崩溃,VUID 未吐出即 SIGSEGV）。此机制已弃用,round-2 改「合法 SPIR-V + 假入口名」(见主报告 §3)。
> 本文件为原始 `device-run\round1\logcat_red.txt`（123,208 B）中崩溃段的逐字摘录 + `round1\transcript.md` §3 诊断,供审计对照;round-1 全量 buffer(`logcat_red_full.txt`,1,299,363 B)与 transcript 全文留 scratch,不入库。

## Round-1 RED 机制(已弃用)

vertex `.spv` **喂入损坏字节**（有意破坏 SPIR-V），期望 `vkCreateShaderModule` / pipeline 建立时 layer 报 `VUID-VkShaderModuleCreateInfo-pCode-08742`。实际结果:layer 在解析非法 SPIR-V、格式化错误消息的路径内**踩到已释放/错标指针**,被设备的 MTE(tagged-pointer)抓死 → 硬 SIGSEGV,VUID 从未落 logcat。

## 崩溃逐字（`round1\logcat_red.txt`）

```
--------- beginning of crash
07-16 17:00:50.866 10987 11030 F libc    : Fatal signal 11 (SIGSEGV), code 2 (SEGV_ACCERR), fault addr 0xb400007063e834d4 in tid 11030 (com.rurix.vk), pid 10987 (com.rurix.vk)
07-16 17:00:51.104 11045 11045 F DEBUG   : Build fingerprint: 'HONOR/BKQ-AN10/HNBKQ:16/HONORBKQ-ANXX/10DLDLD160SP1C00E160:user/release-keys'
07-16 17:00:51.104 11045 11045 F DEBUG   : ABI: 'arm64'
07-16 17:00:51.104 11045 11045 F DEBUG   : Cmdline: com.rurix.vk
07-16 17:00:51.104 11045 11045 F DEBUG   : pid: 10987, tid: 11030, name: com.rurix.vk  >>> com.rurix.vk <<<
07-16 17:00:51.104 11045 11045 F DEBUG   : tagged_addr_ctrl: 0000000000000001 (PR_TAGGED_ADDR_ENABLE)
07-16 17:00:51.104 11045 11045 F DEBUG   : signal 11 (SIGSEGV), code 2 (SEGV_ACCERR), fault addr 0xb400007063e834d4
07-16 17:00:51.104 11045 11045 F DEBUG   : 16 total frames
07-16 17:00:51.104 11045 11045 F DEBUG   : backtrace:
07-16 17:00:51.104 11045 11045 F DEBUG   :       #00 pc 0000000001283494  .../lib/arm64/libVkLayer_khronos_validation.so (BuildId: 13204c6e71811fabb9fd173b89b19c786d8337b4)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #01 pc 000000000128a064  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #02 pc 00000000012a291c  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #03 pc 000000000129a0e8  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #04 pc 00000000012df32c  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #05 pc 0000000000b72f08  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #06 pc 00000000000253d4  .../lib/arm64/librurix_vk.so (rurix_rt::vk::present_body::{{closure}}::h8635107f2d791f91+68)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #07 pc 0000000000024848  .../lib/arm64/librurix_vk.so (rurix_rt::vk::present_body::hef55352a118f8926+3840)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #08 pc 0000000000027578  .../lib/arm64/librurix_vk.so (rurix_rt::vk::run_graphics_present_android::h17848797eabd6f4c+2532)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #09 pc 00000000000277fc  .../lib/arm64/librurix_vk.so (rurix_rt::vk::run_graphics_present_android_safe::h0317a6c8e2fc9a64+204)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #10 pc 000000000001f734  .../lib/arm64/librurix_vk.so (rurix_vk::render_thread::h4f66fdfa6903d8f1+1576)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #14 pc 0000000000082858  /apex/com.android.runtime/lib64/bionic/libc.so (__pthread_start(void*)+232)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #15 pc 0000000000075730  /apex/com.android.runtime/lib64/bionic/libc.so (__start_thread+64)
```

## 诊断（`round1\transcript.md` §3,原文）

- validation layer **确已加载且在执行**——非「layer 未加载」失败模式:`libVkLayer_khronos_validation.so` 占崩溃栈顶 **6 帧**（#00–#05),由 rurix 自身的 `present_body::{{closure}}`（#06）**同步进入** `run_graphics_present_android`,layer 在调用路径里真活,非仅被映射。
- 失败模式:RED 路把 layer 逼进 **SIGSEGV（signal 11,SEGV_ACCERR）**——发生在任何 VUID/validation 消息写入 logcat **之前**、app 写 `present_result.json` **之前**。硬崩,**非**受控 validation-abort（干净 abort-on-error 应是 SIGABRT/signal 6 带 `Abort message` 承载 VUID——二者皆无）。
- 故障地址 `0xb400007063e834d4` 落在 `0xb4000070…` tagged-pointer / scudo 堆区;此类指针上的 `SEGV_ACCERR` 是 **use-after-free / MTE 标签错配**的签名——即 layer 处理 RED 情形时解引用了已释放或错标指针。layer PC 簇 #05 `0xb72f08`（低区,dispatch/intercept 入口）→ #00–#04 `0x128–0x12d`（26 MB layer 高区,core-validation / 消息上报机构）——intercept→core-validation→崩溃链,最一致于 layer **在检测/格式化故意错误时崩溃**,故预期的干净 VUID 从未浮现。
- 全量 buffer 扫（`logcat_red_full.txt`）:`RurixVK` / `RurixVK-VVL` app 行 = **0**（app 在吐出自己任何日志前即崩）；`VUID` / `Validation Error` 文本 = **0**。

## 判据表（round-1,缺省即 FAIL）

| criterion | expected | observed | verdict |
|---|---|---|---|
| present_result.json 存在 | yes | **absent（NO_RESULT_FILE）** | FAIL |
| validation_errors > 0 | yes | 字段不可得（无结果） | FAIL |
| logcat 含 VUID / "Validation Error" | yes | **none** | FAIL |
| present 非全绿 | (n/a) | present 前进程即 SIGSEGV | — |

## 收束

**RED 未按设计变红 → 按协议在 step 3 HALT,未进 GREEN。** 这一崩溃本身是**独立的上游证据**:MTE 在 Adreno/Android 16 上抓到 Khronos validation layer 在非法-SPIR-V 错误格式化路径的真实内存伤(use-after-free / tag 错配)——不是本项目缺陷,是 layer 上游鲁棒性 bug 被 MTE 硬抓。为消除该 spec-UB 依赖,round-2 改用**合法 SPIR-V + 模块内不存在的假入口名**(`rurix_red_bogus_entry`)驱动 `VUID-VkPipelineShaderStageCreateInfo-pName-00707`——不向 layer/驱动喂非法字节,天然规避该崩溃路径(见 `src/rurix-rt/src/vk.rs` red_selftest 注释 + 主报告 §3)。
