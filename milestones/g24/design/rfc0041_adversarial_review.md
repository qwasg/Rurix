<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# RFC-0041 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 清册重判可流于形式（十一条批量 maintain） | high | accepted — 逐条 backfill 条件字面核验入 evidence detail（机核事实：工具/资产/上游状态逐条实测）；close 仅当字面成立，maintain 必附最新核验事实非复述 |
| F2 HDR 设备半实测依赖显示器状态 | medium | accepted — vulkaninfo 表面色彩空间为当前设备+驱动+显示链实况；HDR token absent 时 maintain-SDR 是该实况下唯一诚实结论；显示链变化 = reeval_anchor |
| F3 M120 数据半「在案」是否足以构成重判输入 | medium | accepted — M120 绿件为 measured 冻结带（七算法帧时/内存/质量曲线 + 精确档 diff=0）；strand 强制精确 OIT 的裁决输入即该数据面，需求半独立核验 |
| F4 SAFE-GPU 处置滑向永久搁置 | medium | accepted — 本期显式两态：defer-to-G25+（战役终审窗点名）或评估性关闭留痕；不允许无锚 defer |
| F5 历史 RD history 批量追加的只追加合规 | low | accepted — 逐条独立 history entry（date/event/evidence 三字段全）+ 互锁 append-only 机核（下期 interlock vs 本期 base） |
