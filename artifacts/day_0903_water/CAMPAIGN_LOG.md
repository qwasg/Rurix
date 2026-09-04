# day_0903_water 战役日志(G41 水面渲染前端)

> 任务:「参考 HPWater 项目中的水面实现方案,并复现到本项目中。然后联网搜索
> 新的水面场景,以便展示水面效果」。
> 编排形态(如实):主 agent 串行承担侦察 / 实施 / 验收三层;两个并行侦察子
> agent 先行产出渲染器架构报告与场景/治理报告。GPU 真跑全程
> `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`。**未 commit,入库归 owner**。

## 波次

| 波 | 内容 | 终态 |
|---|---|---|
| W0 | 侦察:HPWater 全 28 源文件通读;渲染器架构 + 场景/治理双侦察报告;许可分析定 clean-room 路线 | 完成 |
| W1 | host 金标准 `world::water_surface`(波方程 / 泻湖 / 参数面 / 公式面)+ 24 单测 | 24/24 绿 |
| W2 | 五 kernel + 展示车道 + Cargo 接线;真设备出图 | 门内 5/5 编译 |
| W3 | 波方程 device↔host 对拍探针 + measured 冻结带 + RED 臂 | `1.2218952e-6` 在带内 |
| W4 | 视觉定标(曝光 / 反照率 / 散射 / 泡沫 / 浅化)+ 七臂 A/B | 8 组 digest 互异 |
| W5 | 联网检索水景资产;Poly Haven CC0 HDRI 下载 + 烘焙工具 + `--env-lut` | `lakeside_sunrise` 落地 |
| W6 | 门 `ci/g41_water_smoke.py` + Mini-RFC 0050 + 数字台账 + 战役文档 | 门 11/11 绿 |

## 关键缺陷与修法(施工实录,供后来者少走弯路)

本役在 device 侧连撞四类问题,逐一定位。四条都写进了对应 kernel 的头注:

1. **哨兵灾难性抵消**。`out_depth = big + (view_z − big) · hit` 在 f32 下**恒得 0**
   (1e30 ± 57 == 1e30 ⇒ 差为 −1e30 ⇒ 和为 0)。后果:整幅深度为 0,水面"在场景
   之前"的门全关,**水完全不渲染**。修法:哨兵↔有限值的择一一律用**乘性**
   `a·(1−g) + b·g`。同类修法另施于 `rock_t` / `hit_t`。
2. **PowerShell `Set-Content -Encoding UTF8` 写入 BOM**。`.rx` 词法器拒
   `\u{feff}`,而我把编译输出 `| Out-Null` 吞掉了 —— 于是连续几轮"改了代码但
   输出字节完全相同"的假象,浪费数轮定位。修法:改用
   `[System.IO.File]::WriteAllText(..., UTF8Encoding($false))`,且**编译退出码
   永远显式打印**。
3. **相位函数漏 1/4π**。HG 写成 `(1−g²)/denom^1.5` 而 Rayleigh 用的是归一化式
   `3(1+cos²θ)/16π`,两项不同量纲,合成相位整体偏大 ≈ 4π ⇒ 内散射过亮、
   **深水反比浅水亮**的"发光牛奶池"。修法:HG 补 1/4π,host/device 同步。
4. **`if` 包多个 `while` 的 codegen 失效**(与 `g31_realism.rx` 已登记缺陷同型)。
   礁石段命中距离恒 0 而轮廓门正确。**四类成因逐项排除**(动态下标 → 改循环内
   锁存;哨兵抵消 → 改乘性;跨段浮点相等 → 改容差;多累加器锁存 → 改完全展开)
   后症状不变,遂判为后端缺陷并移除礁石(循环 4→2)。详见 REPORT §6.1。

另有两处**物理/口径**修正:
- 曝光:`SkyPreset::ev100` 是标定源资产的记录 EV,**不是**绝对物理曝光
  (`world::sky` 的 `sun_color` 已归一化到 ~0.9)。先按 `1/(1.2·2^ev100)` 用 →
  整幅压黑 10⁵ 倍;改按**相对补偿** `2^(14.5 − ev100)` 后四档一致。
- 浅水波**浅化**:不收波幅时波谷把水面压到海床下,岸边出现沿沙纹的硬边黑弧洞。
  按 `smoothstep(0, 2·amp+0.3, depth)` 收幅后消除。

## 检索实录(「联网搜索新的水面场景」)

- Khronos `glTF-Sample-Assets` 147 模型全量检索:**无水景**。
- NVIDIA ORCA:`Sun Temple` = CC-BY-**NC-SA**,`Bistro` = CC-BY-4.0 但无水体
  → Sun Temple 因 NC/SA 违反资产白名单**出局**。
- Sketchfab:检索到 CC-BY-4.0 水景多件(Bathhouse 158k 三角 / Pool in the
  Mountains 25k / LAKE 33k / Forgotten Sanctuary Lake 1.85M),许可合规,但
  **下载 API 强制 OAuth**(需用户令牌),且生产装载面只吃
  `.gltf + 外置 .bin + DDS(BC1/BC3)`,需另写 JPEG→DXT 转换链 → 本役未采用,
  登记为可选路径。
- **Poly Haven HDRI(CC0-1.0)**:采用。白名单最宽松一档、公开 API 免鉴权、
  与 `world::sky` 四档预设的标定来源同源。实取 `lakeside_sunrise`。

| W7 | 短片腿:`--dump-raw/--dump-raw-every` 逐帧 raw(与 rain_night 同布局)+ `make_water_clip.py` + `.gitignore` 战役块;首版六滴 0.7–0.9 强度叠加后波峰泡沫成"白漆斑",按 canonical 尺度(0.38–0.55)重出 | `lagoon_orbit.mp4` 10.0 s |

## 出图清单(`previews/`,留盘)

四天空档 `lagoon_{noon,clear,golden,sunset}.png`(1600×900)+
八臂 A/B `arm_*.png`(1280×720)+ 实拍环境 `lagoon_lakeside_hdri.png` +
自定义波源 `lagoon_drops.png`;环绕短片 `lagoon_orbit.mp4`(300 帧 @30 fps,
`--cam-orbit` + 六滴脚本波源;第 40 帧的低角日光反射带为本车道最有说服力的一帧)。
