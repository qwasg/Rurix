<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C 验收门 Task C18） -->
# G33_PLAN — 商业化期执行计划（波 C 批次范围）

> 事实源 = [G33_CONTRACT.md](G33_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 期定位

G33 = **商业化期**（G31+ 待办总表 §6 波次线 3）：把"游戏画面引擎"补成"外部可采纳的商用渲染器"——§5 商业化工程面八件（SDK/文档/兼容矩阵/健壮性/分发/许可/profiling/支持政策）+ §3 长线窗按锚兑现（NGX 分解/RD-027 守护/P4 四行/HLOD L4/SVT 三行/KTX2 三行/RT pipeline + 六窗重判 + 十二阻塞探针）+ 波 C 验收门。上游法定输入 = `milestones/g30/g30_campaign_handover_registry.json`（RFC-0047 §5.5 唯一法定输入面）+ `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` §5 #48~#56 + §3 #14/#16/#20~#32 + §6 波次线 3 + `registry/deferred.json`。上一期 G32 波 B 已验收（G32_CONTRACT §8 close-out 实测 facts 在案）。

## 2. 波次

| 波次 | 内容 | 门 | 状态 |
|---|---|---|---|
| 波 C（本波） | C1 SDK + C2 文档 + C3 降级链 + C4 健壮性 + C5 分发 + C6 许可 + C7 profiler + C8 政策 + C9 NGX 分解 + C10 RD-027 守护 + C11 P4 四行 + C12 HLOD L4 + C13 SVT + C14 KTX2 + C15 RT pipeline + C16 六窗重判 + C17 十二阻塞探针 + C18 验收门 | 十八门 + 验收六面 | 已交付并验收（G33_CONTRACT §8 close-out 实测 facts） |
| 后续（期内候选/期外） | #56 外部采纳判据（维持未宣称，外部生产项目选择为准）；RD-015 重判窗（llvm#57928 closed 信号在案，重判程序待启动）；GAP-01~03 许可义务闭合（发布形态前置）；RD-027 修复（上游 NVIDIA 本体，绕行在案）；#11 BistroExterior（G10-N6 锚挂起维持）；骨骼虚拟几何实施窗（C16 判档登记，实施归后续期） | 后续波立项程序 | 未立项 |

## 3. 波 C 交付面（C1~C17，全绿/如实在案）

- **C1 SDK 稳定 API 面**：9 C ABI 函数两层 DLL（export_c codegen 复用）+ 外部 C++ 宿主真跑 digest==Stage A 锚 + stable 快照 renderer_sdk_api 第五段；门 `g31.waveC.sdk` PASS。
- **C2 文档与示例**：docs/renderer/ 三件 + minimal_host 示例 + walkthrough 1.29s；门 `g31.waveC.docs` PASS。
- **C3 兼容矩阵与降级链**：capability report + 六链 fail-closed + 12 单测 + 三超分臂真机切换；AMD/Intel 格 DEV_ENV_DEGRADE；门 `g31.waveC.capability` PASS。
- **C4 运行时健壮性**：device-lost poisoned 锁存/TDR 超时/OOM budget/窗口风暴 121 resize/soak 故障臂；门 `g31.waveC.robustness` PASS。
- **C5 分发打包**：16 组件 bundle + 签名/SBOM + 离线可建 digest==锚 + 红臂四路 + EA1 回归；门 `g31.waveC.dist` 9/9 PASS；GAP-01~03 维持 open 登记。
- **C6 许可终审**：16 vendor 矩阵 cleared 15/conditional 1；门 `g31.waveC.license` PASS。
- **C7 profiling 工具面**：双 bin --profile-json + debug labels + 分解恒等式 + Stage A 锚 HIT；门 `g31.waveC.profiling` PASS。
- **C8 支持政策**：support_policy + release_checklist + SECURITY 双件增补段；门 `g31.waveC.support` PASS。
- **C9 NGX 分解**：四段分解 + 承接锚兑现（宿主差可分离 measured 证据）；门 `g31.waveC.ngx_decomp`（交付时 PASS ratio 0.980232；验收复跑 ratio 轨迹面诚实红登记，§8.1③）。
- **C10 RD-027 守护**：毒区全测绘（绿 7 毒 13）+ O0 护栏 + fail-closed 毒区拒绝；门 `g31.waveC.rd027` PASS；RD-027 open 维持 + 绕行登记。
- **C11 P4 四行**：RXPD v2 磁盘面 + GPU 请求反馈链 + LOD cut 驻留联动 + 优先级 IO；门 `g31.waveC.p4stream` PASS（交付时 ×3）。
- **C12 HLOD L4**：两半全齐 + 三入口解锁 + 改判四级链登记；门 `g31.waveC.hlodl4` PASS。
- **C13 SVT**：SVT-1/2/3 三行落地 + SVT-4 维持 defer；门 `g31.waveC.svt` PASS。
- **C14 KTX2**：KTX2-1/2/3 三行 + A/B（DDS 维持 Windows-first，ETC1S 6.96× 跨平台档）；门 `g31.waveC.ktx2` PASS。
- **C15 RT pipeline**：RFC-0048 Agent Approved + device 真跑（镜像语料臂位级 == RayQuery 臂）+ SER measured ratio 0.518 + .rx codegen 缺口 PR-2/3/4 open 登记；门 `g31.waveC.rtpipeline` PASS；M52 维持 defer。
- **C16 六窗重判**：M61 3/3 → maintain-no-go；RD-039 骨骼 triggered 余项维持；SMRT/世界缓存/NRD/RD-026 维持；门 `g31.waveC.rejudgment` + `g31.waveC.meshbench` PASS。
- **C17 十二阻塞探针**：零冒充全维持 open；门 `g31.waveC.blockedprobes` PASS；RD-015 锚信号（llvm#57928 closed）登记重判窗待启动。

## 4. 波 C 验收门（C18，本波）

1. 终验三面复跑：C1 SDK 宿主 + C5 离线可建链（digest==Stage A 锚 + 帧时）+ C2 文档门。
2. 发布件核验：C5 dist 门（签名/SBOM/红臂）+ C6 license 门 + GAP-01~03 处置状态核验（维持 open → 发布口径注明）。
3. 全量回归：守卫套件五条（budget_eval --strict 零 estimated）+ 波 A 五门 + 波 B 五门 + 波 C 全门 --gate 复跑 + Stage A digest 18/18 锚 + G16 M-g 18/18 + G17-MD-F1 焦点格新鲜多样本中位如实登记。
4. soak 汇总：波 A 10010 帧在案 + C4 故障臂在案 1010 帧引用 + 波 C SDK 面增量 soak ≥1000 帧。
5. G33 四件套落盘（本文件族）+ G31+ 战役总登记（milestones/g31_plus_campaign_record.md）+ §8 close-out 实测 facts + 零降级三面终判。

## 5. 编号纪律

CI 数字步骤零消费（波 C 十八门 + C18 验收面均未占号；registry/number_ledger.json CI_step.next_free=525 落盘前实测维持）；RFC 段一件消费（RFC-0048，C15）；U 段一件消费（U-59，C1）；RXS/RD/SG/MR/D/RX_error 共享段零消费；evidence 前缀 `g33_baseline_` 经 check_schemas 一处纯追加登记（g31_baseline_/g32_baseline_ 同律跳过路由）。
