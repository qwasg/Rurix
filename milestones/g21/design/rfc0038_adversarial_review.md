<!-- Assisted-by: Cursor Agent（G21.1 治理波） -->
# RFC-0038 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 解析夹具无遮挡 ⇒ 无偏性检验是否代表真实场景 | high | accepted — 参考臂判据面收窄为「采样策略无偏性与方差收益」（p̂ 即 f 时无偏性可逐字验证）；含遮挡 shadow ray 面 = device/集成波承接锚显式登记，不冒充 |
| F2 方差收益阈 >2 是否手写 | medium | accepted — 阈为对照下界非通过线校准：等验证预算 uniform vs RIS-16 的实测收益量级（64 灯夹具 measured ≥2 显著性），probe 落 measured 值入档；争议时只追加重判 |
| F3 时域合并置信漂移（无界 m） | high | accepted — M-cap 截断硬编码入 merge 语义 + 单测锚（m_cap 截断断言） |
| F4 SER capability 取证依赖 vulkaninfo 外部工具 | medium | accepted — 取证件落全量 stdout 存档 + 扩展字面 grep 双件；工具不可得时 disposition = not-measurable 如实登记不冒充 |
| F5 M100 低档面溅射 | high | accepted — restir_reservoir 独立加性模块零接线；multi_light.rs 0-byte 由 M-a facts git-diff 机核 |
