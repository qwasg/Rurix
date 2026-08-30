**D6 GGX 高光材质加性臂：成功，全部验收绿灯。**

## 改动文件清单（4 个源文件 + 1 个 SPV 产物）

| 文件 | 改动 |
|---|---|
| `src/rurix-render/kernels/g18_smooth_nrm.rx` | +第 9 路 `tri_mr` view（签名 trinrm 后/out 前）；params[48]>0.5 均匀分支门 GGX 臂：tri_mr 唯一读取包在门内（关臂 8B 哑表零触达）；h=normalize(wi+wo)、D=Trowbridge-Reitz(α=rough²)、G=Smith Schlick-GGX(k=α/2)、F=Schlick(F0=mix(0.04,albedo,metal))、rough 钳 [0.05,1]；spec 进**独立 Lo 域累加器**（不经 albedo·inv_pi 漫反射耦合）输出尾加——关臂 +0.0 非负域恒等，不改既有乘加序 |
| `src/.../g14_3_lane/g14_3_lane_body.rs` | MatRec += `roughnessFactor`（glTF 缺省 1.0）；`assemble_scene_nrm_mr`（tri_mr 2 f32/tri，matless=[0,1]，quad 尾段恒 0，off 不读不装）；PARAMS_LEN 48→56（g34/g35 两侧同派生自洽，实证零漂移）；`pack_frame_params_ggx`（nrm 签名 0-byte 委托，v[48]=1 仅 smooth_nrm&&ggx）；`UnifiedTsrLane.set_ggx`；MegaSmoothNrm 23→24 SSBO（U_TRI_MR=23） |
| `src/.../g14_3_pipeline_perf.rs` | `--ggx off|on`（默认 off）；**须随 --smooth-normals on（fail-closed）**，互斥集照 D2 纪律（gi/dyn/skin/cluster/wp/vendor 由 smooth-normals 校验链覆盖） |
| `src/.../g31_window_present.rs` | nrm/nrm_bloom 两描述组 += 8B 零哑表第 9 绑定（下标 25/33）——kernel 扩签名全绑要求；窗口车道恒 params[48]=0，已有 --dither/--bloom/--dump-present-raw/--smooth-normals 全保留 |

母版 `g18_light_transport_depth.rx`/`g14_3_direct_gi.rx` 0-byte；G36 文件零触碰。

## 自验结论（GPU 锁内，target-night）

- **零漂移全锚 PASS**：默认臂 8f == `f39e9808`（PARAMS_LEN 扩面零影响）｜snrm off 8f == `6b46f70a`｜snrm off **128f 收敛 == `778f1dfc`**｜窗口 off == `5596a730` / on == `b02b08b57` / combo == `12d5dc91` + validation 全静默
- **接线生效**：on 双跑位级一致 + on≠off（128f 收敛 = `ec395575`）
- **帧时**：scene_gpu off 973µs → on 989µs，**GGX 增量 +16µs（+1.65%）**（bistro 4 点光；quad 面光臂空循环）
- **视觉**：on 臂 128f EXR→ACES PNG —— 20.2% 像素变化（全 70 材质 metal=0.4 ⇒ 广域釉面 sheen 符合预期），diff ×4 图呈**结构化贴面**（地板灯光 sheen、柜台正面高光梯度、桌沿/椅腿 specular pop），无 NaN 黑洞、无全屏噪、无腐败

## CLI 用法

```bash
g14_3_pipeline_perf --bench|--render --scene bistro-interior --tier 100 \
    --backend tsr_device [--presentation-profile night] \
    --smooth-normals on --ggx on
```

## 已知留窗

1. 窗口车道无 `--ggx` 旗标（恒关臂 + 哑表保底；若后续开启需同法线面组合 + 真表接线）
2. F0 用 mats 存量 albedo（已乘 1−metallic）——bistro metal 恒 0.4 无黑洞；metal=1 极端资产会 F0→0（如实登记）
3. quad 面光 GGX 路径 bistro 无覆盖（quads=0；cornell 为 Split 形态与 smooth-normals fail-closed 互斥）——代码同式已就位但未实证
4. `--ggx on` + 显式 `--spv-scene` 覆盖面未加 fail-closed（照 D2 评审既有登记同律，低危）

证据全部落 `artifacts/night_0828/d6_ggx/`：`d6_verdict.json`（总裁决）+ `verify_summary.json` / `d6w_summary.json` / `visual_summary.json` + 三脚本（`d6_baseline_pre.py` / `d6_verify.py` / `d6w_verify.py` / `d6_visual.py` 可复跑）+ `ggx_off/on_aces.png` / `ggx_diff_x4.png` / 三组对照裁剪。

Done. Let me write the final response to the parent agent with all the required information: changed files list + self-verify digest/frame time/visual conclusion + CLI usage + success + known gaps.