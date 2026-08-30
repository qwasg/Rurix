# rurixc「if 包 while」codegen 缺陷修复报告(G37 W1)

> 缺陷出处:`artifacts/day_0828/e_final/HANDOVER.md` §A.3 / `artifacts/day_0828/CAMPAIGN_LOG.md` Round 1(A1 灯光提取臂:「首版『if 包 while+if』嵌套形态被 spirv-val 拒……改 branchless gate_lc 绕行,缺陷已登记」,`a1_lamp_lights/ACCEPTANCE_SUMMARY.json` 留痕「块 144→非头 121 分支」)。
> 本轮 = 编译器面修复 + 回归样例登记。**kernel 里的 branchless 绕行不摘除,生产 SPV 冻结字节零触碰**(纪律执行,见 §5 零漂移证明)。

## 1. 根因(一句话 + 展开)

**一句话**:`vulkan_codegen::structured_merge` 求 if 的 selection merge 用「两臂最近共同前向可达块」,但可达性遍历不裁剪循环回边——if 落在 while 循环体内时 CFG 有环,else 臂可经「join→latch→循环头→then 臂」绕整圈把 then 臂内部块(含 then 目标本身)算进共同可达集;当 then 臂内的 while 之后还有语句(真 join 的臂内距离被拉长)时,绕环假候选以更小 max 距离胜出,`OpSelectionMerge` 被指向 then 臂内块,产出非法 SPIR-V。

展开(MIR 块图,触发形态 `while li<n { if gate { while j<4 {..} if d {..} } li+=1 }`):

```
bb0 entry ─→ bb1 外层循环头 ─┬→ bb2 if 头(gate)─┬→ bb4 then:内层 while+尾 if … → bb6
                             │                    └→ bb5 else(空)────────────────→ bb6
                             └→ bb3 外层出口       bb6 if join = 外层 latch ──回边──→ bb1
```

- `structured_merge(bb4, bb5)` 的 BFS 里,bb5 经 **bb6→bb1(回边)→bb2→bb4** 4 步「够到」bb4,则候选 bb4 的 max(then=0, else=4)=4;
- 真 join bb6 的 then 侧距离 = 穿过内层 while(头→出口)再过尾 if(两臂→join)= 5 步,max(5,1)=5 > 4 → **假候选 bb4 胜出**;
- 发射 `OpSelectionMerge %bb4` + `OpBranchConditional %c %bb4 %bb5`(merge == then 目标)→ spirv-val 拒:`block <ID> 24 branches to the selection construct, but not to the selection header <ID> 14`。

**为何最小「if 包 while」不触发**(day_0828 未见即红的原因):内层 while 之后没有其它语句时,真 join 的 then 侧距离(≈3)仍小于绕环路径(≈4),真解恰好胜出;A1 kernel 的 gate 内是「阴影射线 while + 命中判定 if」,尾随 if 把真 join 距离 +2,越过临界点。形态矩阵见 §2。失败方式为 rurixc 内部 spirv-val 门 fail-closed(RX6026 编译期拒),非静默错误语义。

## 2. 复现(修复前编译器,形态矩阵)

复现件:`repro/`(`if_while_min.rx` + `shapes/s1..s8`,pre_fix/post_fix 双态 SPV 与 spirv-val 输出存档)。

| 形态 | 结构 | pre-fix | post-fix |
|---|---|---|---|
| min | if 包 while(极小) | 绿 | 绿(字节全同) |
| s1 | if 包 (while+尾 if) | 绿 | 绿(字节全同) |
| s2 | while 包 if 包 while | 绿(距离恰好未越界) | 绿(字节全同) |
| s3 | if-else 双臂各含 while | 绿 | 绿(字节全同) |
| s4 | 两层 if 包 while | 绿 | 绿(字节全同) |
| s5 | if 包 while(体内含 if) | 绿 | 绿(字节全同) |
| s6 | if 包两顺序 while | 绿 | 绿(字节全同) |
| **s7** | **while 包 if 包 (while+尾 if)**(A1 字面形态) | **红:RX6026,spirv-val `block 24 branches to the selection construct, but not to the selection header 14`** | **绿,spirv-val rc=0** |
| s8 | if 包 while+尾直线语句 | 绿 | 绿(字节全同) |

反汇编证据(`repro/s7_{pre,post}_fix.dis.txt`):gate if(头 %14)的 merge 声明 pre-fix 为 `OpSelectionMerge %16`(%16 = 自己的 then 目标,内层 while 初始化块),post-fix 为 `OpSelectionMerge %18`(真 join,自增 li 后接外层 continue)。其余 merge(`OpLoopMerge %15 %25` / `OpLoopMerge %21 %26` / 尾 if `OpSelectionMerge %24`)两态一致。

- pre-fix 编译失败原文:`repro/rurixc_s7_pre_fix_stderr.txt`;非法 SPV 外部校验:`repro/spirv_val_s7_pre_fix_external.txt`(rc=1)
- post-fix 校验:`repro/spirv_val_s7_post_fix.txt`(rc=0)

## 3. 修复

**方案**:`structured_merge` 的可达性遍历**排除已识别循环的回边(latch→header)**,使交汇计算在无环前向图上进行。回边表 = codegen 预扫描既有产物 `Builder::loop_info`(循环头→(merge, latch),G7.2 W3a 面),零新增分析。

安全性论证:
- 真 join 恒可经纯前向路径到达(`mir_build::lower_if` 两臂降级尾必 `Goto join`),裁剪只删绕环假候选、不删真解;
- 回边对「臂起点可达的前向块」的 BFS 距离无影响(经回边回到的头块距离必更大),故**既有合法编译的 merge 选择不变**——§5 生产 kernel 90/90 字节位级对拍佐证;
- dxil_spirv 图形扩展路(`structured_merge` 第二调用方)`has_cycle` 预拒循环、CFG 恒无环,传空表位级等义;
- 多 latch 循环(`continue` 产物)不在 `loop_info`(既有保守拒面),其形态修复前后同为 RX6026 fail-closed,无行为漂移。

## 4. 回归样例(登记)

语料(`conformance/vulkan/accept/`,自动进 `ci/vulkan_codegen_smoke.py` accept 段——CI 门位覆盖):

| 语料 | 形状 |
|---|---|
| `vk_if_while.rx` | if 包 while |
| `vk_if_else_both_while.rx` | if-else 双臂各含 while |
| `vk_if_if_while.rx` | 嵌套两层 if 包 while |
| `vk_loop_if_while_if.rx` | while 包 if 包 (while+尾 if)——**缺陷字面触发形态** |

测试(`src/rurixc/tests/compute_if_while_vulkan_spirv_val.rs`,Cargo.toml 登记 `[[test]] required-features = ["vulkan-backend"]`,#175 教训):

- `if_while_shapes_selection_merge_targets_are_join_blocks`:结构不变量**恒跑腿**(无外部工具)——每个 `OpSelectionMerge` 后必紧随 `OpBranchConditional` 且 merge ≠ then/else 目标;`OpLoopMerge` merge ≠ continue;含 while/if 语料必须出现两种 merge 指令。
- `if_while_shapes_pass_spirv_val`:spirv-val 严格校验腿(缺工具 SKIP,镜像 `compute_w1_vulkan_spirv_val` 口径)。

**红证**(测试真能抓缺陷,git stash 临时撤修复实测):两腿对 `vk_loop_if_while_if` 双双红——结构腿 `selection merge %16 不得等于分支目标 (then %16 / else %17)`;spirv-val 腿报错与 A1 登记同款。恢复修复后 2/2 绿。

## 5. 验证

| 验证面 | 命令 | 结果 |
|---|---|---|
| 新增回归测试 | `cargo test -p rurixc --features vulkan-backend --test compute_if_while_vulkan_spirv_val` | 2 passed / 0 failed |
| 全量(vulkan 特性套) | `cargo test -p rurixc --features vulkan-backend` | **526 单测 + 108 集成测试全绿,0 失败,1 ignored(既有)** |
| 全量(默认特性套) | `cargo test -p rurixc` | 31 测试目标全绿(rc=0) |
| 全特性编译 | `cargo check -p rurixc --all-features` | 绿 |
| CI 冒烟(语料集成) | `py -3 ci/vulkan_codegen_smoke.py` | PASS:17 accept(13 既有+4 新增)17/17 spirv-val;5 reject 确定性拒 |
| **生产 kernel 零漂移** | `drift_check.ps1`(pre/post 两态编译器各编 98 个生产 .rx 到 `.tmp`,SHA256 对拍;冻结 SPV 文件零触碰) | **90/90 可编 kernel 字节位级全同;8 个两态对称失败(rurix-rt PTX 面 + g31_rt_slab_hit,独立 CLI 编译面外,与修复无关);0 rc 漂移**(`drift_check/DRIFT_VERDICT.json` + 两 manifest) |

构建纪律:全程 dev profile、默认 `target/`;未跑任何 GPU 程序;未触 `g31_window_present.rs`/`g14_3_lane_body.rs`/kernel .rx/.spv/milestones/registry。

## 6. 修改文件清单

| 文件 | 改动 |
|---|---|
| `src/rurixc/src/vulkan_codegen.rs` | `structured_merge` 增 `loop_info` 参数 + 回边裁剪(+文档);`emit_terminator` 调用点传 `&b.loop_info` |
| `src/rurixc/src/dxil_spirv.rs` | `structured_merge` 调用点传空表(该路无环,位级等义;+注释) |
| `src/rurixc/Cargo.toml` | 登记 `[[test]] compute_if_while_vulkan_spirv_val`(required-features) |
| `src/rurixc/tests/compute_if_while_vulkan_spirv_val.rs` | 新增回归测试(两腿) |
| `conformance/vulkan/accept/vk_if_while.rx` | 新增语料:if 包 while |
| `conformance/vulkan/accept/vk_if_else_both_while.rx` | 新增语料:双臂 while |
| `conformance/vulkan/accept/vk_if_if_while.rx` | 新增语料:两层 if 包 while |
| `conformance/vulkan/accept/vk_loop_if_while_if.rx` | 新增语料:缺陷字面触发形态 |
| `artifacts/day_0830_delivery/w1_fixes/rurixc_if_while/**` | 本报告 + 复现件 + 对拍件(交付物) |

## 7. 遗留风险(如实登记)

1. **`continue`/多 latch 循环仍为保守拒面**:`while c { .. continue .. }` 产双回边,`loop_merge_targets` 恒 `None` → 无 `OpLoopMerge` → RX6026 fail-closed。既有限制、修复前后行为一致,非本缺陷面;生产 kernel 目录(`src/rurix-render/kernels` + `src/rurix-rt/kernels`)零 `continue` 语句(仅注释提及;conformance 的 `continue` 语料属 syntax/desugar 前端面,非 Vulkan compute 路)。留窗。
2. **未识别回边(如多 latch)不参与裁剪**:此时 `structured_merge` 理论仍可能被环污染,但该形态先于 merge 选择即被拒(同上 fail-closed),不产静默错误 SPV。
3. **A1 kernel 的 branchless gate 绕行按纪律不摘除**:摘除意味着重编生产 SPV(冻结字节面)。若未来质量战役择机摘除,须走冻结面协议(重收割锚 + 全门复跑),本修复已为其扫清编译器障碍。
4. **合成 continue 块的 label 分配顺序**:修复不触碰(loop 面零改动),`OpLoopMerge` 两态反汇编逐字一致佐证。
5. 形态覆盖为「while/if 任意两层嵌套 + 缺陷三层字面形态」;更深(≥4 层)嵌套无理论缺口(裁剪后图恒无环、交汇唯一),但未逐一枚举,CI accept 段可随需追加。
