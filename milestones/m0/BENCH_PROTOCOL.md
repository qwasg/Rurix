# M0 基准操作规程 — L0/L1

> 所属契约:[M0_CONTRACT.md](M0_CONTRACT.md)
> 版本:v1.0(2026-06-11)
> 协议来源:r11 §1(全部数字照搬,本文不另造数字)→ 已被 08 §4 采纳为 `rx bench` 设计基线;M0 期以独立 harness 脚本形态执行(工具化为 RD-003)。
> 硬件:RTX 4070 Ti(开发机,自托管 runner,14 §8)。

---

## 1. 分层定位

| 层 | 内容 | M0 范围 |
|---|---|---|
| L0 | 环境验证(锁频/温度稳态/进程隔离/环境画像) | 全量建成 |
| L1 | 微基准 | 仅 SAXPY + bandwidthTest(其余 → RD-002) |
| L2–L4 | 模式/mini-app/端到端 | 不在 M0(08 §4:L3 随 G0 demo) |

## 2. L0 环境验证(每次采样前置,不可跳过)

### 2.1 锁频规程

1. `nvidia-smi -pm 1`(持久模式;Windows 驱动若不支持则记录并跳过,留痕进画像)。
2. `nvidia-smi -q -d SUPPORTED_CLOCKS` 查询本机支持的时钟档位——**锁频目标值以本命令输出为准**(Boost Clock 验收,08 §4;禁止凭记忆或官网参数填写)。
3. `nvidia-smi -lgc <sm_clock>,<sm_clock>` 锁 SM 时钟;`nvidia-smi -lmc <mem_clock>` 锁显存时钟(命令形式来源 r11 §1.2)。
4. 验证:NVML 读回当前时钟与目标一致,写入环境画像 `clocks.locked=true`。
5. 解锁(会话结束):`nvidia-smi -rgc; nvidia-smi -rmc`。

**降级规则**:任一步失败 → 照常采样但证据标 `evidence_level=unlocked`(未锁频运行间差异可达 50%+,r11 §1.4),该证据**不得**用于回填预算阈值。

### 2.2 温度稳态与隔离

- 温度稳态:warmup 后 GPU 温度进入风扇曲线平衡窗(r11 §1.1 SPEC training-run 哲学);探测器记录采样起止温度。
- 进程隔离:采样期间 NVML 枚举 GPU 上无其他计算进程;违例 → 终止本轮并留痕。
- WDDM 现实:driver model / HAGS / TDR 配置必须入画像(08 §2.3);HAGS 不假设更快,仅作 A/B 维度记录。

### 2.3 环境画像

字段集与校验以 [evidence_schema.json](evidence_schema.json) 为唯一事实源。受限环境可降级取值(如无管理员权限读不到 TDR 注册表 → 字段填 `"unavailable"`),**但 schema 不变**(14 §5 约定)。

## 3. L1 采样协议(r11 §1.3 数字全套照搬)

| 步骤 | 参数 | 来源 |
|---|---|---|
| warmup | ≥10 次迭代 | r11(MLPerf v4.0 / SOL-ExecBench) |
| 稳态判定 | 连续 5 次迭代 CV < 5% | r11 §1.1 |
| warmup 超时保护 | 单次迭代 300s 终止 | r11 §1.1 |
| L2 缓存清理 | 每次 timed 迭代前清零 256MB device buffer | r11 §1.1 |
| timed 采样 | 50 次 × 3 trials | r11 §1.3.1 |
| trial 内统计 | 中位数 | r11 §1.3.1 |
| 跨 trial 汇总 | trimmed mean(去头尾 20%) | r11 §1.3.1 |
| 异常剔除 | IQR | r11 §1.3.2 |
| 置信区间 | bootstrap 95% CI | r11 §1.3 |
| 计时 | 统一 CUDA Event;测量区前后 `cuStreamSynchronize` 刷 WDDM batch | r11 §1.4 / 08 §4 |

**三次运行规则(契约 G-M0-1)**:上表是一次"运行"的内部协议;预算回填值 = **三次进程级独立运行**(每次重新装载、重新过 L0)各自 trimmed mean 的再次 trimmed mean。任一次 `evidence_level != measured_local` 则整组作废。

## 4. M0 基线定义

### 4.1 SAXPY(L1,内存密集型,核心指标:有效带宽 GB/s,r11 §8)

| 项 | 定义 |
|---|---|
| 语义 | `y[i] = a * x[i] + y[i]`,f32 |
| 实现 | 手写 PTX(`.target sm_89` / `compute_89` 基线,00 §5)+ Driver API 装载 |
| 问题规模 | 以 2^24 元素为主档;另记 2^20 / 2^28 两档作规模敏感性参考(主档进预算,参考档进证据 JSON) |
| 有效带宽 | `3 * N * sizeof(f32) / t`(读 x、读 y、写 y) |
| 正确性 | host 参考实现逐元素比对(f32 精确相等——SAXPY 无重排,FMA 与否在 PTX 中显式固定) |

> 问题规模档位为本项目设定(`estimated` 性质的工程选择,非 r11 数字);若实测发现主档未打满带宽,允许在 close-out 前经 Direct PR 调档,调档记录留痕。

### 4.2 bandwidthTest 等价(L1,核心指标:GB/s)

| 方向 | 内存类型 | API |
|---|---|---|
| H2D | pageable / pinned 各一组 | `cuMemcpyHtoD`(pinned 经 `cuMemAllocHost`) |
| D2H | pageable / pinned 各一组 | `cuMemcpyDtoH` |
| D2D | device | `cuMemcpyDtoD` |

传输尺寸主档 256MB,协议同 §3。

## 5. 记录与回归

- 每组采样产出一份证据 JSON(schema 见 [evidence_schema.json](evidence_schema.json)),归档 `evidence/` 目录,文件名 `<bench>_<yyyymmdd>_<seq>.json`。
- 预算断言写入 [m0_budget.json](m0_budget.json)(`m0.` 命名空间,14 §3)。
- 回归判定(M0 建机制,M1+ 生效):基线/候选各 30 样本,Mann-Whitney U(p<0.05)+ 效应量门(Cohen's r > 0.3);报警阈值 GPU 1% Warning / 5% Critical(r11 §7 / 08 §4)。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
