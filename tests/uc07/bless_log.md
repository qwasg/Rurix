# UC-07 离线 golden manifest bless 审批记录(只追加)

> 任何 `tests/uc07/golden_manifest` 的新增/修改必须同 PR 在本表追加一行
> (RFC-0010 §4.4 ③ blessed 哈希软门;契约 G-MS1-4;MS1 CI_GATES §5 第 5 项
> tests/uc07 golden bless 纪律)。重 bless 命令:
> `RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`(以本次 device 真跑
> digest 重写 manifest;bless 后仍须走完全部硬门——确定性两跑一致 + refcpu 容差 +
> 数据流红绿,软门 bless 不豁免硬门)。触发面:驱动升级 / JIT 版号变更 / 渲染或
> 仿真语义面合法演进导致逐帧 SHA-256 漂移;漂移未经本表留痕即 CI 步骤 53 红。
>
> golden_manifest 格式:每行 `<sha256>  frame_%04d.ppm`(冒烟档
> N=4096 / 160×120 / 8spp / 2 帧,RFC-0010 §4.3)。
>
> 🔒 本表数据行避「日」+「期」连写子串(bless 守卫按该子串识别表头,镜像
> tests/stable/bless_log.md 纪律);数据行用 ISO 数字日号即可。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-07-15 | tests/uc07/golden_manifest 首份 bless(frame_0000.ppm=05b59ff2a93e25b06b830f84fa2f0c4764b2f0468a461d9a1e53c138d4890963,frame_0001.ppm=e9c2c2c2191cf1679a47d689f5f9c33f3303c3317ebd2d1afb1e8d992ae4dd89) | RFC-0010 §4.4 ③ blessed 哈希软门首次定基:MS1.3 UC-07 ruridrop 离线冒烟档(N=4096 / 160×120 / 8spp / 2 帧)于 RTX 4070 Ti(driver 620.02,CUDA v13.3)device 真跑取得——同机两次运行逐帧量化 PPM 字节 SHA-256 一致(硬门①)+ GPU 帧 vs refcpu 入口 host 重放逐像素全等(硬门②:\|Δ\|≤1 占比 1.000000,max 0)+ 篡改 sim_forces 重力常数 GRAVITY 10.0→2.5 经同一 rx build 链重编 → 逐帧 digest 变红、原树复原绿(数据流红绿)。命令:`RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py` 重写后全部硬门照走 PASS(CI 步骤 53 同 PR 接线)。 | qwasg/白栀(agent 完全自主签署,AGENTS v3.0 硬规则 1;MS1.3/G-MS1-4) |
