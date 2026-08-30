# Day 0829 真实感战役:六画质臂 A/B 判据工具

Phase 1 预制面。三件套:`ab_metrics.py`(单入口参数化指标)+ `raw2png.py`
(视觉对照,a2 适配版)+ 本速查。全部读 day_0828 同款 presented raw dump。

## raw dump 格式(源码确认结论)

出处 = `src/rurix-render/src/bin/g31_window_present.rs` 写盘段(day_0828 语料
`a2_autoexp/raw2png.py`、`b_textures/b_visual_metrics.py` 同一消费口径):

| 偏移 | 长度 | 内容 |
|---|---|---|
| 0 | 4 B | `w: u32` 小端 |
| 4 | 4 B | `h: u32` 小端 |
| 8 | `w*h*4` B | BGRA8 打包像素(swapchain `bgra8_unorm`,字节序 `[b,g,r,a]`,u8) |

**纠偏**:raw dump 是 display-encode(tonemap + 编码)后的 8bit presented
字节,**不是 f32**。f32 面只有两条别的链,均不用于本战役 A/B:

- `RURIX_G31_DUMP_F32=1` env 门控 TEMP 归因 dump(写
  `.tmp/g31_gates/hzb/last_f32.bin`,无 w/h 头、裸 f32 LE,标注"毕后删除",
  非验收面;a2b 曾用它做 encode parity 取证);
- bench 车道 `converged.exr` / `frames/frame_*.exr`(f32 RGB,走
  `ci/g10_exr_lib.decode_exr`,是另一条 EXR 链,与 raw 无关)。

本工具读入后 BGRA→RGB、`/255` 归一化 [0,1] float64 再算指标;比较均在
display 域(A/B 同域自比,方向性判据成立)。

## dump 旗标(真实名称)与命令形状

`g31_window_present` 相关旗标(源码 CLI 解析段确认):

- `--dump-present-raw <path>`:**末帧** presented BGRA8 raw dump(上表布局)。
- `--dump-present-every <n>`:每 n 帧追加 dump,路径 = `<base>.f<帧号:04>`
  (如 `on.raw.f0080`);**须随 `--dump-present-raw`**(fail-closed)。
  帧号 `fi` 从 0 计且**含 warmup**(`total = warmup + frames`,`fi % n == 0`
  时写),末帧另写 `<base>` 本体。a2 先例 `off.raw.f0000/.f0080/.f0160/.f0240
  + off.raw` 即 every=80。
- `--present-luma-out <json>`:逐帧 presented 亮度序列 sidecar(伴生验证面)。
- `--dump-last-frame <raw.bin>`:同布局但**须随 `--slab-table`**(B3 对拍
  专用),画质臂不用。

命令形状(day_0828 先例:二进制 = `target-night/release/g31_window_present.exe`,
env 纪律 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,显式臂另加
`RURIX_G18_AMBIENT=0.004`〔`--quality full` 预设自注入无需 env〕;批跑走
`ci/gpu_device_lock.gpu_device_lock()` 单锁):

```powershell
# 形状 A:静态契约相机末帧 A/B 对(f4 视觉主证同式,96f)
target-night/release/g31_window_present.exe --frames 96 --warmup 2 --hidden `
  --quality full `
  --dump-present-raw artifacts/day_0829_realism/<arm>/png/on.raw `
  --evidence artifacts/day_0829_realism/<arm>/ev/on.json
# off 臂 = 去掉被测旗标(或 --quality full 换 all-off/七臂显式),其余全同。

# 形状 B:周期多帧 dump(noise 时域面 / 轨迹对照;a2 dolly 同式)
target-night/release/g31_window_present.exe --frames 320 --warmup 10 --hidden `
  --quality full --auto-move dolly `
  --dump-present-raw artifacts/day_0829_realism/<arm>/png/on.raw `
  --dump-present-every 80 `
  --present-luma-out artifacts/day_0829_realism/<arm>/ev/on_luma.json `
  --evidence artifacts/day_0829_realism/<arm>/ev/on.json
```

noise 采样建议:静态相机 + `--dump-present-every 8`,只取**后段**帧
(TSR 收敛后,如 fi ≥ 总帧 2/3 段)进 `noise` 子命令,避免收敛期污染
(d_tsr conv 协议同思路)。

## 工具用法

`ab_metrics.py` 六个指标子命令 + 自测(全部输出 JSON 到 stdout,`--out` 另存;
`--rect x,y,w,h` 可多次,`--label` 逐 rect 命名):

```powershell
py -3 ab_metrics.py luma on.raw                                  # 全屏 linear+log2 统计
py -3 ab_metrics.py crop on.raw --rect 1400,150,480,270 --label wall
py -3 ab_metrics.py diff off.raw on.raw --rect 100,100,200,200   # SAD/变化像素占比
py -3 ab_metrics.py grad on.raw --rect 1100,800,480,270          # Sobel 梯度能量
py -3 ab_metrics.py edge on.raw --rect 700,700,300,120 --axis x  # 10-90 过渡带宽(px)
py -3 ab_metrics.py noise on.raw.f0160 on.raw.f0240 on.raw       # 帧间方差
py -3 ab_metrics.py selftest                                     # 8x8 合成自测
py -3 raw2png.py on.raw off.raw --gain 4 --gamma 2.2             # 暗部提亮查看
```

要点:`diff --thresh` 默认 0.5/255(精确等价 u8 域 ≥1 级差);`edge` 须单
rect + `--axis`(x = 过渡带沿水平展开),crop 应框住**单一**过渡带(影缘),
输出平均剖面宽 `profile_width_10_90_px` 与逐线宽 `line_width_mean_px` 双口径;
`crop` 的 `rgb_frac` = 通道占比(色偏判据用);`noise` 口径 = 逐像素跨帧 std
(d_metrics/c_noise 同式)。

## 六臂判据速查

A/B 恒式:off 臂 = 基线(被测旗标关),on 臂 = 仅开被测旗标,其余全同;
每臂 = 指标判据 + `raw2png.py` 并排视觉对照。rect 坐标按当日契约相机逐臂
定位(1920x1080 先例 ROI 可作起点:wall 1400,150,480,270 / floor
1100,800,480,270 / dark_arch 360,0,360,180 / dark_table 560,560,560,200)。
阈值数字为预制建议值,当日战役定标为准。

| 臂 | 子命令 | 判据方向 |
|---|---|---|
| ① 金属 F0 修复 | `crop`(金属件 rect)+ `diff`(非金属 rect) | 金属 crop luma mean ↑(高光/环境响应增强);非金属 rect `changed_frac = 0`(修复不外溢) |
| ② RT AO | `crop`(角落/接触缝 rect)+ `luma`(全屏) | 角落 crop mean ↓(接触暗化出现);全屏 mean 降幅 < 5%(不整体压暗) |
| ③ RT 反射 | `grad` + `crop`(光滑面 rect) | 光滑面 crop `grad_mean` ↑(镜像结构出现,平坦高光→有内容);crop luma 不失控(mean 变化有界) |
| ④ 法线贴图 | `grad`(全屏 + 材质 rect)+ `noise` | `grad_mean` 全屏 ↑(表面细节增量);`temporal_std` 不升(细节是几何不是噪声) |
| ⑤ 点光软阴影 | `edge`(影缘 rect,`--axis` 垂直影缘) | `profile_width_10_90_px` ↑(半影展宽);影芯/影外 crop luma 基本不变(只软化不漂移) |
| ⑥ GI2 贴图反弹 | `crop`(间接光区 `rgb_frac`)+ `noise` | 间接光区 `rgb_frac` 向反弹面贴图色偏移(如向红墙 r 占比 ↑);`temporal_std` 不升(反弹不引新噪) |

## 自测

`py -3 ab_metrics.py selftest`:numpy 合成 8x8 raw(恒定色/提亮/水平渐变/
双噪声帧,tempfile 目录即用即弃)跑全六子命令,递归断言输出无 NaN/Inf +
方向性断言(自差=0、渐变 grad>0、10-90 宽 ≈5.6px、噪声 std>0),输出摘要 JSON。
