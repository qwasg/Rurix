# repro log — 桌面 VVL control 三腿实验(逐字命令 + 输出)

**Status: DRAFT — do NOT file.**(备包证据文件;上游提报由 owner 亲自执行,AI 不对外提交)

- 日期:2026-07-17(同日两次独立执行,结果逐字一致;下为第二次执行的完整逐字记录)
- 机器:Windows 11 x86_64(**无 MTE**),NVIDIA RTX 4070 Ti
- VVL:VK_LAYER_KHRONOS_validation **1.3.296**(注册于 `HKLM\SOFTWARE\Khronos\Vulkan\ExplicitLayers` → `C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin`)
- shell:git-bash(exit 139 = 128+SIGSEGV,对应 Windows 原生 `0xC0000005` STATUS_ACCESS_VIOLATION;PowerShell 下同崩为 `$LASTEXITCODE=-1073741819`)
- 环境画像其余项见 [PROVENANCE.md](PROVENANCE.md)「EA1 支线 B 回填」节

## 工件与再生配方(工件 .spv 本体不入库)

| 工件 | 来源 |
|---|---|
| `vs.spv`(376B,合法) | `rurixc --target vulkan` 编 `conformance/vulkan/accept/vk_tri_vs.rx`(与 `ci/vulkan_graphics_smoke.py` 构建路径一致) |
| `fs.spv`(合法) | 同上,fragment 侧 |
| `vs_tampered.spv` | `vs.spv` 施仓内既有确定性损坏配方 `ci/vulkan_codegen_smoke.py:121-125`:`spv[20] ^= 0xFF`(byte20 `0x11→0xEE`) |
| harness | `cargo build -p rurix-rt --features vulkan --bin vk_triangle` → `target/debug/vk_triangle.exe <vs.spv> <fs.spv>`(offscreen 三角形,`RURIX_VK_VALIDATION=1` 时启 debug messenger) |

## Leg A — 合法模块,VVL 开(基线)

```
$ RURIX_VK_VALIDATION=1 VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation \
    target/debug/vk_triangle.exe vs.spv fs.spv
VK_TRIANGLE: ok W=64 H=64 covered=968 center=(130,59,65)
exit=0
```

## Leg B — 损坏模块,VVL 开(3/3 确定性崩溃)

三次重复,stderr 与退出码逐字一致(仅示一次;rep 2/3、rep 3/3 输出与此逐字节相同):

```
$ RURIX_VK_VALIDATION=1 VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation \
    target/debug/vk_triangle.exe vs_tampered.spv fs.spv
[vk-validation] Validation Error: [ VUID-VkShaderModuleCreateInfo-pCode-08737 ] | MessageID = 0xa5625282 | vkCreateShaderModule(): pCreateInfo->pCode (spirv-val produced an error):
End of input reached while decoding OpAtomicSMax starting at word 5: expected more operands after 2 words.
The Vulkan spec states: If pCode is a pointer to SPIR-V code, pCode must adhere to the validation rules described by the Validation Rules within a Module section of the SPIR-V Environment appendix (https://vulkan.lunarg.com/doc/view/1.3.296.0/windows/1.3-extensions/vkspec.html#VUID-VkShaderModuleCreateInfo-pCode-08737)
/usr/bin/bash: line 17:  1738 Segmentation fault      RURIX_VK_VALIDATION=1 VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation target/debug/vk_triangle.exe "$S/vs_tampered.spv" "$S/fs.spv"
exit=139
```

- stdout 为空;VUID 吐出**之后**才 access-violate——与 Android(SEGV_ACCERR,VUID 未吐即崩)时序不同。
- 崩溃点在 `vkCreateShaderModule` 的 VVL 处理路径内(spirv-val 已识别损坏并产出诊断,随后 VVL 自身访问违例)。

## Leg C — 同一损坏模块,VVL 关(对照的对照)

```
$ target/debug/vk_triangle.exe vs_tampered.spv fs.spv
VK_TRIANGLE: ok W=64 H=64 covered=968 center=(130,59,65)
exit=0
```

NVIDIA 驱动容忍该损坏模块并正常出图——崩溃归因 **VVL 自身**,非驱动/harness 用坏句柄。

## 结论与诚实边界

1. **桌面(无 MTE)也崩** → 崩溃非 Adreno/MTE 独有;bug 表观范围超出「Adreno / Android 16」,标题/范围提报时或需上修(已在 ISSUE_DRAFT Control experiments 末尾 flag 给 owner)。
2. 版本差:桌面 VVL **1.3.296** vs 设备 **1.4.350.1**;VUID 差:桌面 `pCode-08737` vs Android 设计目标 `pCode-08742`。
3. 配方为「同类确定性配方」(`spv[20]^=0xFF`)**非** Android round-1 的逐字节重放(round-1 精确偏移未存档,须随独立 MRP 钉死)。
