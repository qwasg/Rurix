<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# RFC-0036 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 host-only 参考臂是否构成「兑现」 | high | accepted — 判据面收窄为「host 参考臂实现 + 质量程序产对照 + 口径独立登记」三件；device/vendor 车道显式 out-of-scope 登记不冒充；G13 TSR host→device 分波先例 |
| F2 frame-hold 对照阈是否过弱 | medium | accepted — frame-hold 为零成本下界，逐帧严格优于（非均值优于）+ 双跑位级确定性；阈值零手写符合 P-09 |
| F3 presented_fps 与 real fps 混算风险 | high | accepted — 两口径并列输出永不混算，M-a facts 单列断言；G13-N7 字面 0-byte |
| F4 vendor 臂 not-available 滥用风险 | medium | accepted — 逐臂 disposition 附 rationale 字面 + registry provenance；310.5.2 生产默认维持路径同 G17 M-b 双门禁 |
| F5 默认臂 digest 锚溅射 | high | accepted — framegen 加性独立模块零接线；g14_3_pipeline_perf 0-byte；M-a facts 含 digest 锚零漂移断言 |
