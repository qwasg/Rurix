<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# RFC-0042 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 「表面 0-byte ⇒ 终态维持」推理可被加性面接线绕过 | high | accepted — 加性面零接线独立核验（四模块生产车道模块引用扫描）与表面 0-byte 双条件合取；任一命中 ⇒ 重测程序显式触发，不静默维持 |
| F2 焦点格单测一轮的统计效力 | medium | accepted — 单测定位为「新鲜度登记」非达标判定输入（终判定盘 = 最新 18 格 evidence + 0-byte 证明）；单轮 ratio 与终值显著偏离时如实登记并触发复测程序 |
| F3 归档闭集完整性不可机核 | medium | accepted — 归档表逐节行数机核（七期 P2 行数 + RD 8 + 清册 11 对账）入 M-d facts；缺节即红 |
| F4 「诚实红终判」永久化 | low | accepted — 终判附 G26+ 承接锚（NGX 分解 profiling/UE 插桩两面 = RFC-0032 重判条件同源），非关闭性定论 |
| F5 战役收官后 defer 链断裂风险 | medium | accepted — M-d 归档表为 G26+ 法定输入唯一载体（各期 P2 表原始锚不回写），链续性由归档完整性机核保证 |
