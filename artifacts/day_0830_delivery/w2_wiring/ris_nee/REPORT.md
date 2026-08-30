# G37 W2 ris_nee:GI2 反弹 RIS 选灯 + 44k 灯片 CDF 面光 NEE(两 kernel 臂)

修 day_0828 HANDOVER §C.11「GI2 反弹无 quad NEE」缺陷本体(44k emissive 灯片
仅反弹 ray 直取命中,反弹 NEE 只连 12 聚类点光,方差高)——EVAL_RESTIR §9.3 推荐
的两个 1 臂当量低配替身(方差源头收缩,优先于降噪器;TODO #6 修复路径)。
本报告 = 设计说明 + 实做清单 + SPV sha 前后自证 + **窗口 bin 精确合入提案**
(窗口 bin 只读——被合入 agent 占用,本会话零触碰)+ tsrq clamp K 阶梯命令
(EVAL_DENOISE 第 0 级)。所有修改处注释「G37 W2 ris_nee」。**禁跑 GPU 纪律
全程执行:本会话零 GPU、零 --release、零 target-night。**

## 一、kernel 段设计(kernels/g31_realism.rx 就地 gate 化追加,第 8 链位)

### 签名扩展
`lamp_tbl: View<global, f32>` 追加在 `tri_transp` 之后、`out_color` 之前
(链式超集律:新最高链位)。SPIR-V binding 计数 transp 20 → **ris 21**
(spirv-dis 核对:21 个 Binding 声明,最高下标 20;+1 = lamp_tbl)。

### 新 params 槽(REAL_PARAMS_LEN=72 不变,原 [69..72) 预留槽全数启用)
| 槽 | 语义 | 默认 |
|---|---|---|
| `[69]` | `--gi2-ris` 门(反弹点 M 候选 RIS 选灯) | 0(off) |
| `[70]` | `ris_m` 候选数(kernel 钳 [1,16]) | 6 |
| `[71]` | `--gi2-nee` 门(灯片功率 CDF 面光 NEE) | 0(off) |

### 双臂机制(全部在 GI2 反弹循环内,gi_hit/关门恒零迭代)
1. **既有「均匀选 1 点光 ×point_count」块**:计数 ×(1−ris_gate)(ris on 整块
   让位蓄水池);nee on 时块内追加 `gi_elig = 1 − nee_gate·gate(lr>0)` 乘进
   keep——A1 聚类代表灯(lr>0 数据契约,契约 delta 灯恒 lr=0,lane_body
   「契约灯半径恒 0.0」冻结注)让位灯片真域,均匀选中代表灯的样本作废
   (×point_count 权重下契约灯子和仍无偏,方差换不双计如实登记)。
2. **新蓄水池块(ris|nee 任一 on 激活)**:混合候选池 M 个候选(ris on:M =
   [70];nee 单开:M=1)→ WRS 蓄水池选 1(`u < w/w_sum` 除法比较形,g28 冻结
   链同源;f32 w_sum 单帧 M≤16 精度域同 g28 device 腿口径)→ 全反弹段仅 1 条
   阴影射线 → RIS 无偏式 `contribution = f(x_sel)·vis·W`,
   `W = w_sum/(M·phat_sel)`,`w_i = phat_i/p_src_i`,phat = max3(未遮挡贡献)
   (A1 max3 口径)。候选循环纯 ALU/表读(while 计数门合法形),阴影射线 =
   「if 包 ray query」白名单形(灯循环同构);「if 包 while」禁形不触
   (W1 已修但绕行律沿用,任务要求)。
   - **池混合 β** = point_active·(1 − quad_active·0.75) ∈ {0, 0.25, 1}:
     ris 单开 β=1(纯点域;**M=1 时与既有估计器数学同式**——w = phat·pcount,
     W = pcount,自洽性锚);nee 单开 β=0 且 M=1(**经典单样本面光 NEE**:
     contribution = Le·cosθ_s·|cosθ_l|/d²·area/pdf_k·vis);双开 β=0.25
     (灯片域主导——44k 灯片 = 夜景反弹主光源,常数登记)。
   - **点光候选**:均匀 1/point_count,p_src = β/point_count;phat 折
     gate_d·gate_cs·elig(nee 让位律同式)·point_active。
   - **灯片候选**:功率 CDF 采样 = **16 步定长二分**(2^16=65536 ≥ 表上限,
     装配 fail-closed;branchless 计数语义 pos = #{k: cdf[k] ≤ u},选中钳
     [0,Q−1]);`pdf_k = cdf[k] − cdf[k−1]` **f32 差 = 采样测度**(与二分计数
     自洽 ⇒ 无偏封闭);面上点 = (u,v) 折叠律 u+v≤1;p_src = (1−β)·pdf_k/area。
   - 随机维 = R2/R3 既有格 + 黄金比共轭 k×0.3819660112501051 独立相位(refl
     臂同族)+ 候选序号 ×0.6180339887498949 步进(soft 臂同形)+ 帧旋转
     params[52];闭式无跨帧状态,固定输入双跑位级一致。
3. **firefly clamp**:两通道全汇入 gi_nee_* → 既有 `gi_l*.min(params[53])`
   逐通道 clamp(新旧同口径,零新 clamp 槽)。

### 能量口径(不双计——选定「NEE 覆盖 ⇒ 直取置零」互斥式,如实登记)
- **nee on ⇒ 反弹 ray 直击灯片时 emission 直取置零**(`rn_emk = 1 −
  nee_gate·gate(max3(mats 发射均值)>0)`):表成员谓词与装配侧逐字同源
  (emission 任一通道>0;tri_mat==SLAB_TRI_NONE 的 quad 灯尾段两侧同排除,
  bistro quads=0 空集)。**同一传输积分换估计器**——反弹顶点对灯片面域的
  余弦采样直取与面光 NEE 覆盖同一积分域,期望不变方差降(非 MIS,选最简
  互斥式;gitex 臂逐像素 emission 覆写同被门,两来源一口径)。
- **nee on ⇒ 12 聚类代表灯让位**(上述 elig 律;直接光主命中面的 12 代表
  点光不动,只动反弹 NEE 域);4 契约 delta 灯(lr=0)保留。
- **反射臂 NEE / AO / 主命中直接光零触碰**(反弹段独享;反射命中点单点光
  NEE 的 quad 缺口留窗,transp 臂「GI2 NEE 视玻璃不透明」律不变)。
- **射线预算/反弹**:off/off = 1(既有);ris = 1;nee = 2(契约点 1 + 灯片
  1);ris+nee = 1(混合池)。
- **已知近似登记**:①灯片 NEE 用逐灯片 mats 均值 Le(Phase F 标定律「贴图
  均值==契约 Le」⇒ 面积分口径能量一致;逐像素 emissive 贴图 NEE 留窗);
  ②灯片法线双面 |cosθ_l|(emission 直取无朝向判定,口径一致);③f32 CDF
  差下溢成员永不被选(bistro 实测数装配日志登记,能量损失上界 = 其功率
  占比;零面积成员射线测度零不可命中,无缺口);④host 前置:ris|nee on ⇒
  points 缓冲 ≥ 1 盏真布局(候选读保底;bistro 契约恒 4 盏,装配 fail-closed)。

## 二、灯片表/CDF 构建策略与大小

**策略 = 全量 44k,不截断**(登记:截断 top-N + 余量合并项引入合并灯的采样
歧义与第二套谓词;全量表 O(log Q) 二分成本与 N 无关,体积可忽略)。

- 单源:`src/rurix-render/src/bin/g37_w2/g31_ris_lamps.rs`
  (`mod g31_ris_lamps`,g31_pso_warmup 模块组织同形;include! 消费)。
- 谓词/口径与 A1 `extract_lamp_lights` 逐字同律:emission 任一通道>0 且
  tri_mat≠SLAB_TRI_NONE;功率 = max3(π·Le_c·area)(A1 通量峰值口径)。
- 布局(kernel 头注逐字同源):头 4 f32([0]=Q,[1]=总功率,[2..4) 预留)
  + CDF [4..4+Q)(**f64 前缀和 → 归一 → f32 下投,末项强制 1.0**,单调不减)
  + 记录 [4+Q..) 逐灯片 16 f32 `[A(3) e1(3) e2(3) 单位法线(3) 面积 Le(3)]`。
- **尺寸(bistro Q=44,024 在案数)**:4 + 44,024 + 44,024×16 = **748,412 f32
  = 2,993,648 B ≈ 2.85 MiB**(对照 texel heap 282.7 MiB,可忽略);关臂哑表
  80 B(header Q=0 + kernel 保底读域 [0..20))。
- **双构建确定性**:升序单趟扫描 + f64 前缀和,无哈希容器——同输入同字节;
  selfcheck 机核双构建位级 ==(下节)。
- fail-closed:零命中/平行数组长度失配/顶点索引越界/Q>65536(二分覆盖域)/
  功率非正或非有限,任一破即 Err。
- 统计登记面:Q/零面积数/f32 pdf 下溢数/总功率/表长(装配日志 eprintln +
  REPORT 消费)。

## 三、实做清单(本会话已改文件)

| 文件 | 修改 |
|---|---|
| `src/rurix-render/kernels/g31_realism.rx` | 头注 params 表 [69..72) 登记 + 臂⑧侧表登记;签名 +lamp_tbl(第 8 链位);GI2 段三处 gate 化:①双门定义 + 既有单点光 NEE 计数 ×(1−ris_gate) + elig 让位律 ②蓄水池块(混合池 M 候选 + CDF 二分 + 面上点 + WRS + 1 阴影射线 + RIS 权重)③合成行 emission 直取 rn_emk 门。其余段落 0 改写 |
| `src/rurix-render/src/bin/g37_w2/g31_ris_lamps.rs` | **新增**:灯片表 + 功率 CDF 构建模块(布局/谓词/确定性/fail-closed 见 §二) |
| `src/rurix-render/src/bin/g37_w2_selfcheck.rs` | 追加 ris_lamps 机核:夹具解析断言(CDF 首/末项、记录段逐槽、零面积占位)、双构建位级 ==、四 fail-closed 拒臂、哑表触达域;主 JSON +"ris_lamps" 字段(additive,schema v1 不变) |
| 新工件 | `.tmp/night_0830/spv/g31_realism_ris.spv`(315,048 B) |

未触:`g31_window_present.rs`(只读纪律)、`g14_3_lane_body.rs`、母版
`kernels/g31_texture_nrm_gi.rx`、既有 .spv 全部、`milestones/`、`registry/`、
`ci/`、target-night。

## 四、SPV sha256 前后自证(`.tmp/night_0830/w2_ris_sha_{before,after}.txt`,Compare-Object = **ALL SAME(12 件)**)

| 文件 | sha256(前 = 后,SAME) |
|---|---|
| g31_realism_f0.spv | `b3dffbe6292f2ed7d837352ea4e0efb870aaf298d321f4444b65377b5edc4915` |
| g31_realism_ao.spv | `76fff402be5d07775f8c5d95fef84dd5ff444ab0ed101dd1a23f7f8bf46398ee` |
| g31_realism_soft.spv | `4eca2067f87fabca726185e6e1af29754b25beb3b41f0620c670293df20583bd` |
| g31_realism_refl.spv | `e418990c240570d4d6fe4fde0fa60184e31ca914a517b723627958df718b73be` |
| g31_realism_gitex.spv | `a0a3c821b8ec8aee6989b9068c19a59e44fc92c79f3a40c98298a591d8a32f1b` |
| g31_realism_nrm.spv | `0c68fc49456798890ad1680cbb2ea4d4be0d53e54ba20e42f4e88e4501f5f4b6` |
| g31_realism_transp.spv(W2 transparency 臂) | `35983d0f405169ec84bf222f4a12ec8bf8dfd7d471eefb12488eea7dd34c4f8b` |
| g31_texture_nrm_gi.spv(冻结) | `fd22cb19c563efc7187b9ea61bcc27afdad56afbf880886af4ca9f541d14e6f7` |
| g31_texture_nrm_gi_gi2.spv(冻结) | `75d08aec5ec89f0d028f2753d8aecafc626383c2b34e5034d10adc41c17da7a4` |
| g31_texture_nrm_gi_em.spv(冻结) | `bdd23a3a14e01cdd325e020c8689adc26cf7db55e7088df70d0d1ca5ff870e25` |
| kernels/g31_texture_nrm_gi.rx(母版源,0-byte) | `9ec07050121611da424dcfbc2cc469a8ad39ced33f1db4bbb0780abe56504c9c` |
| g14_3_direct_gi.spv(m_c 冻结) | `970e13b9b13e66fe5c5f72f94ca0473d4e0771158af56bfb33f8891b7670da36` |

**新增**:`.tmp/night_0830/spv/g31_realism_ris.spv` =
`622a1c33c18e645c22f91566b4c4e3e3a2871298d6b81007379dedef3425feba`
(315,048 B)。源码-下位工件 divergence 律照旧:g31_realism.rx 现源 = 第 8
链位超集,7 个下位工件承载各自锚定字节 0-byte 不再复编。

## 五、编译/机核验证

- rurixc 现建 `cargo build -p rurixc --features vulkan-backend --bin rurixc`
  (dev,`CARGO_TARGET_DIR=.tmp/w2_build` 侧目录;transparency 同形);编译
  `rurixc src\rurix-render\kernels\g31_realism.rx --target vulkan -o
  .tmp\night_0830\spv\g31_realism_ris.spv` **一次过**:内嵌 spirv-val
  accepted + PATH 独立 `spirv-val` rc=0 双过。
- `cargo check -p rurix-render` rc=0(dev,默认 features——selfcheck bin 无
  required-features,模块编译面在内)。
- `cargo run -p rurix-render --bin g37_w2_selfcheck` rc=0:
  `"ris_lamps":{"emissive_tris":3,"zero_area_tris":1,"pdf_underflow_tris":0,
  "total_power":9.424778223,"table_f32_len":55,"double_build":"bitexact"}`
  (夹具 = 3π 总功率解析对账;lut/pso 两既有段全绿不变)。

## 六、窗口 bin 精确合入提案(g31_window_present.rs;内容锚 = 现文件唯一字面)

> 纪律:AE 下标族「新侧表尾挂必新族 + guard 最先 + assert 连号」(红修 #2
> 律);绑定占位律(下位链工件缺绑定即 layout 失配);「默认字面才换」换载。
> 全部新增注释「G37 W2 ris_nee」。

### 锚① 模块 include(锚:`include!("g37_w2/g31_lut_assets.rs");`,其后插)
```rust
include!("g37_w2/g31_ris_lamps.rs");
```

### 锚② SPV 常量(锚:`const G31_DEFAULT_SPV_REALISM_TRANSP: &str = ".tmp/night_0829/spv/g31_realism_transp.spv";`,其后插)
```rust
/// G37 W2 臂⑧ --gi2-ris/--gi2-nee 链工件(transp 超集 + GI2 反弹 RIS 选灯/
/// 灯片 CDF 面光 NEE 段;params[69..72) 门控在内——签名 +lamp_tbl 灯片表
/// 〔19 路 View,新最高链位〕;能量口径 = nee on 时反弹直击灯片 emission
/// 置零 + 聚类代表灯让位灯片真域,详 w2_wiring/ris_nee/REPORT.md)。
const G31_DEFAULT_SPV_REALISM_RIS: &str = ".tmp/night_0830/spv/g31_realism_ris.spv";
```
同时把其下 `G31_REAL_PARAMS_LEN` 文档注释中「`[69..72) 预留恒 0`」改为
「`[69] gi2-ris 门 [70] ris_m [71] gi2-nee 门(G37 W2 ris_nee)`」。

### 锚③ 资源下标(锚:`const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_TRANSP: usize = 44;`,其后插)
```rust
// G37 W2 ris_nee:lamp_tbl 灯片表下标(--gi2-ris|--gi2-nee on 面;kernel
// 签名序 = tri_transp 之后新最高链位——ris|nee on 而 transp off 时
// tri_transp 绑 tri_count×0.0 零表恒占 35〔43〕位,lamp_tbl 恒 36〔44〕)。
const G31_U_LAMPTBL_TEXNRM: u32 = 36;
const G31_U_RESOURCE_COUNT_TEXNRM_RIS: usize = 37;
const G31_U_LAMPTBL_TEXNRM_BLOOM: u32 = 44;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_RIS: usize = 45;
```

### 锚④ scene 屏障计划(锚:`G31_U_PLAN_SCENE_TEXNRM_TRANSP` 常量块尾 `];`,其后插;bloom 同形锚 `G31_U_PLAN_SCENE_TEXNRM_BLOOM_TRANSP` 块尾)
```rust
/// G37 W2 ris_nee scene pass 屏障计划(TRANSP 超集 + lamp_tbl)。
const G31_U_PLAN_SCENE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    /* G31_U_PLAN_SCENE_TEXNRM_TRANSP 全 17 项逐字 */
    (G31_U_LAMPTBL_TEXNRM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    /* G31_U_PLAN_SCENE_TEXNRM_BLOOM_TRANSP 全项逐字 */
    (G31_U_LAMPTBL_TEXNRM_BLOOM, TargetState::StorageReadWrite),
];
```
(展开时把注释占位替换为对应 TRANSP 计划成员的逐字拷贝 + 尾项;保守超集
逐字声明律。)

### 锚⑤ AE 下标族(锚:`const G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP: u32 = 46;`,其后插)
```rust
/// G37 W2 ris_nee:A2 追加资源下标(lamp_tbl 尾挂后 AE 三件再顺延 +1:
/// tex+nrm+…+transp(占位)+ris 37..=39 / ×bloom 45..=47——红修 #2 律:
/// 新侧表尾挂必新 AE 下标族,双接线 + assert 连号,ris guard 先于 transp)。
const G31_U_AE_STATE_TEXNRM_RIS: u32 = 37;
const G31_U_AE_PARAMS_TEXNRM_RIS: u32 = 38;
const G31_U_AE_PARTIALS_TEXNRM_RIS: u32 = 39;
const G31_U_AE_STATE_TEXNRM_BLOOM_RIS: u32 = 45;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS: u32 = 46;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS: u32 = 47;
```

### 锚⑥ AE reduce/state 屏障计划(锚:`G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_TRANSP` 常量块尾 `];`,其后插;成员形态与 _TRANSP 四计划逐字同构,仅换 _RIS 下标)
```rust
const G31_U_PLAN_AE_REDUCE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_RIS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
```

### 锚⑦ descs 函数 `g31_lane_descs_tex_nrm`(三处)
a. 签名(锚:`    tri_transp_bytes: Option<&'x [u8]>,` 非 bloom 函数内,其后插):
```rust
    lamp_tbl_bytes: Option<&'x [u8]>,
```
b. tri_transp 尾挂块改判 + 新块(锚:非 bloom 函数内
`        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_TRANSP);\n    }`,
整块替换为):
```rust
        if lamp_tbl_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_TRANSP);
        }
    } else {
        assert!(
            lamp_tbl_bytes.is_none(),
            "G37 W2 ris_nee: lamp_tbl Some 须 tri_transp 同 Some(transp off 面调用点传 tri_count×0.0 零表——kernel 签名序 fail-closed)"
        );
    }
    // G37 W2 ris_nee:lamp_tbl 尾挂 36(kernel 签名序 = tri_transp 之后新
    // 最高链位;None = 上述面逐字 0-byte)。
    if let Some(lt) = lamp_tbl_bytes {
        d.resources.push(init(lt)); // G31_U_LAMPTBL_TEXNRM
        sb.push(G31_U_LAMPTBL_TEXNRM);
        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_RIS);
    }
```
(注意:原 transp 块的 `if let Some(tp) = tri_transp_bytes {` 头两行 push 不
动,仅其 assert 行起替换。)
c. 屏障选择头分支(锚:`    d.barriers[0] = if tri_transp_bytes.is_some() {`
非 bloom 函数内,替换为):
```rust
    d.barriers[0] = if lamp_tbl_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_RIS
    } else if tri_transp_bytes.is_some() {
```

### 锚⑧ descs 函数 `g31_lane_descs_tex_nrm_bloom`(同形三处,bloom 下标族)
签名 +`lamp_tbl_bytes`;transp 块 assert 改判 + lamp_tbl 尾挂 44
(`G31_U_LAMPTBL_TEXNRM_BLOOM`/`G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_RIS`);
屏障头分支 `G31_U_PLAN_SCENE_TEXNRM_BLOOM_RIS`。函数 doc 注释各追加一行
`/// G37 W2 ris_nee:lamp_tbl_bytes = --gi2-ris|--gi2-nee on 面(尾挂 36
〔44〕;transp off 时调用点传 tri_transp 零表保持签名序)。`

### 锚⑨ lane 状态机(四处)
a. 字段(锚:`    transparency: bool,` 结构体内,其后插):
```rust
    /// G37 W2 臂⑧ GI2 反弹 RIS/NEE(--gi2-ris/--gi2-nee on 车道创建后经
    /// set_gi2_ris 一次性挂载 → prepare_update 置 params[69..72);off =
    /// false ⇒ 三槽不写,参数面 0-byte)。
    gi2_ris: bool,
    gi2_ris_m: f32,
    gi2_nee: bool,
```
b. 构造默认(锚:`            transparency: false,`,其后插):
```rust
            gi2_ris: false,
            gi2_ris_m: 6.0,
            gi2_nee: false,
```
c. 挂载方法(锚:`fn set_transparency(&mut self) {` 方法块尾 `}` 之后插):
```rust
    /// G37 W2 臂⑧ GI2 反弹 RIS/NEE 挂载(任一 on 车道创建后一次性;off
    /// 车道不调用 ⇒ 不写,参数面 0-byte)。
    fn set_gi2_ris(&mut self, ris: bool, ris_m: f32, nee: bool) {
        self.gi2_ris = ris;
        self.gi2_ris_m = ris_m;
        self.gi2_nee = nee;
    }
```
d. prepare_update(锚:realism 扩面条件 `            || self.transparency`,
替换为):
```rust
            || self.transparency
            // G37 W2 ris_nee:臂⑧并入 realism 扩面门。
            || self.gi2_ris
            || self.gi2_nee
```
(锚:`            if self.transparency {\n                scene_params[68] = 1.0;\n            }`,其后插):
```rust
            // G37 W2 ris_nee:params[69..72)(RIS 门/候选数/NEE 门;[52]
            // 帧旋转由 gi2 pack 已写——CLI 已裁须随 --gi2 on)。
            if self.gi2_ris {
                scene_params[69] = 1.0;
                scene_params[70] = self.gi2_ris_m;
            }
            if self.gi2_nee {
                scene_params[71] = 1.0;
            }
```

### 锚⑩ CLI(四处)
a. 声明(锚:`    let mut transp_alpha: Option<f32> = None;`,其后插):
```rust
    // G37 W2 臂⑧:GI2 反弹 RIS 选灯 / 灯片 CDF 面光 NEE(默认 off 零漂移;
    // on = scene pass 换载 g31_realism_ris.spv〔签名 +lamp_tbl,新最高链位〕
    // + 灯片表/CDF 装配〔g37_w2/g31_ris_lamps.rs〕+ params[69..72) 门)。
    let mut gi2_ris = false;
    let mut gi2_ris_m: Option<usize> = None;
    let mut gi2_nee = false;
```
b. parse 臂(锚:`"--transp-alpha" => {` 臂块尾 `            }`,其后插):
```rust
            // G37 W2 臂⑧:--gi2-ris/--gi2-ris-m/--gi2-nee 闭集(默认 off)。
            "--gi2-ris" => {
                gi2_ris = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2-ris 档 {other} 越闭集(off|on)")),
                };
            }
            "--gi2-ris-m" => {
                gi2_ris_m = Some(
                    take_arg(&args, &mut i)
                        .parse::<usize>()
                        .unwrap_or_else(|_| fail("--gi2-ris-m 非 usize")),
                );
            }
            "--gi2-nee" => {
                gi2_nee = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2-nee 档 {other} 越闭集(off|on)")),
                };
            }
```
c. 校验 + 换载(锚:transparency 校验块尾
`    if transparency\n        && (spv_texture == …REALISM_NRM)\n    {\n        spv_texture = G31_DEFAULT_SPV_REALISM_TRANSP.to_owned();\n    }`,其后插):
```rust
    // G37 W2 臂⑧ --gi2-ris/--gi2-nee 闭集校验 + 链换载(默认字面才换;
    // ris|nee 为新最高链位,与 realism 七臂正交组合)。
    if (gi2_ris || gi2_nee) && !gi2 {
        fail("--gi2-ris/--gi2-nee 须随 --gi2 on(反弹段属 GI2 加性臂,fail-closed)");
    }
    if (gi2_ris || gi2_nee) && !(smooth_nrm && textures) {
        fail("--gi2-ris/--gi2-nee 须随 --smooth-normals on 且 --textures on(g31_realism 链基座,fail-closed)");
    }
    if !gi2_ris && gi2_ris_m.is_some() {
        fail("--gi2-ris-m 须随 --gi2-ris on(off 面零消费,fail-closed)");
    }
    let gi2_ris_m_v = gi2_ris_m.unwrap_or(6);
    if gi2_ris && !(1..=16).contains(&gi2_ris_m_v) {
        fail("--gi2-ris-m 必须 ∈ [1,16](kernel 蓄水池候选域,fail-closed)");
    }
    if (gi2_ris || gi2_nee)
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT
            || spv_texture == G31_DEFAULT_SPV_REALISM_REFL
            || spv_texture == G31_DEFAULT_SPV_REALISM_GITEX
            || spv_texture == G31_DEFAULT_SPV_REALISM_NRM
            || spv_texture == G31_DEFAULT_SPV_REALISM_TRANSP)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_RIS.to_owned();
    }
```
d. realism 汇总门(锚:`    let realism_any =\n        metal_f0 || rt_ao || soft_shadows || rt_reflect || gi2_tex || normal_maps || transparency;`,替换尾段为):
```rust
    let realism_any = metal_f0
        || rt_ao
        || soft_shadows
        || rt_reflect
        || gi2_tex
        || normal_maps
        || transparency
        // G37 W2 ris_nee:臂⑧并入(triem 回退/tri_base 哑表/params 扩容)。
        || gi2_ris
        || gi2_nee;
```

### 锚⑪ full 预设并入(建议采纳——EVAL_RESTIR §9.3 字面「修复路径」;full 语义变更重锚归 W4)
(锚:`const QUALITY_FULL_EXPANSION: [&str; 20] = [` … `"--transparency",\n        ];`)
数组改 `[&str; 22]`,`"--transparency",` 后插 `"--gi2-ris",` 与
`"--gi2-nee",`(`--gi2-ris-m` 不进 dup 表 = 可与 full 组合微调,rt-ao 子参数
同律);赋值区(锚:`        transparency = true;`)其后插:
```rust
        // G37 W2 ris_nee:两臂并入 full(EVAL_RESTIR §9.3 修复路径;十七臂
        // → 十九臂,full 语义变更重锚归 W4;--gi2-ris-m 走默认 6)。
        gi2_ris = true;
        gi2_nee = true;
```

### 锚⑫ era 外装配字节面(锚:trinm 回退真表块
`    let (trinm_fb_bytes, tri_tan_dummy): (Vec<u8>, Vec<u8>) = if transparency && !normal_maps {`
条件替换为 `if (transparency || gi2_ris || gi2_nee) && !normal_maps {`,其块尾 `};` 后插):
```rust
    // G37 W2 臂⑧:ris|nee on 而 transparency off 面的 tri_transp 零表
    // (_ris SPV 签名含 tri_transp,阴影重走段 [prim] 无门保底读 ⇒ 须
    // tri_count 全尺寸 0.0〔= 不透明,kernel tp_gate=0 双保险〕)。
    let tri_transp_zero_bytes: Vec<u8> = if (gi2_ris || gi2_nee) && !transparency {
        bytes_f32(&vec![0.0f32; scene.tri_count])
    } else {
        Vec::new()
    };
    // G37 W2 臂⑧:lamp_tbl 字节面(extent 无关,era 外一次构建;nee on =
    // 灯片表 + 功率 CDF 真表〔g37_w2/g31_ris_lamps.rs 单源,确定性双构建〕;
    // ris on 而 nee off = 80B 零哑表〔header Q=0,kernel 保底读域〕;两臂
    // off = 空 vec 零消费〔desc None〕)。前置:points 非空(kernel 候选读
    // 保底;bistro 契约恒 4 盏)。
    let lamp_tbl_bytes: Vec<u8> = if gi2_nee {
        if scene.points.is_empty() {
            fail("臂⑧ --gi2-nee 须场景 points 非空(kernel 候选读保底,fail-closed)");
        }
        let (v, st) = g31_ris_lamps::build_lamp_table(
            &scene.positions,
            &scene.indices,
            &scene.emission,
            &scene.tri_mat,
            SLAB_TRI_NONE,
        )
        .unwrap_or_else(|e| fail(&format!("臂⑧ lamp_tbl 装配: {e}")));
        eprintln!(
            "{GTAG}: G37 W2 臂⑧ 灯片表 {} 片(零面积 {}/pdf 下溢 {},总功率 {:.3},{} f32 = {} B)",
            st.emissive_tris,
            st.zero_area_tris,
            st.pdf_underflow_tris,
            st.total_power,
            st.table_f32_len,
            st.table_f32_len * 4
        );
        bytes_f32(&v)
    } else if gi2_ris {
        if scene.points.is_empty() {
            fail("臂⑧ --gi2-ris 须场景 points 非空(kernel 候选读保底,fail-closed)");
        }
        vec![0u8; g31_ris_lamps::G31_RIS_LAMP_DUMMY_BYTES]
    } else {
        Vec::new()
    };
```

### 锚⑬ descs 调用点(三处,锚均在 tex_descs 构造块内)
a. nm_ref 回退条件(锚:`            } else if transparency {\n                Some((trinm_fb_bytes.as_slice(), tri_tan_dummy.as_slice()))`,条件替换为
`            } else if transparency || gi2_ris || gi2_nee {`)。
b. transp_ref(锚:`            let transp_ref = if transparency {\n                Some(tri_transp_bytes.as_slice())\n            } else {\n                None\n            };`,整块替换为):
```rust
            let transp_ref = if transparency {
                Some(tri_transp_bytes.as_slice())
            } else if gi2_ris || gi2_nee {
                // G37 W2 ris_nee:零表占位保持 kernel 签名序。
                Some(tri_transp_zero_bytes.as_slice())
            } else {
                None
            };
            // G37 W2 臂⑧:lamp_tbl(nee 真表/ris 哑表/off None——链下位
            // 工件无本绑定,多余绑定即 layout 失配)。
            let ris_ref = if gi2_ris || gi2_nee {
                Some(lamp_tbl_bytes.as_slice())
            } else {
                None
            };
```
c. 两调用点传参(锚:两处 `                    transp_ref,`,各其后插):
```rust
                    ris_ref,
```

### 锚⑭ AE 施加 match(锚:`                match (smooth_nrm, bloom) {\n                    (true, true) if transparency => g31_apply_autoexp(`,在 transparency 两分支**之前**插——guard 序 = 挂载序最尾者最先):
```rust
                    // G37 W2 ris_nee:_RIS guard 最先(lamp_tbl 挂载序最尾
                    // 即下标最高;W1 assert 连号为错配保护网)。
                    (true, true) if gi2_ris || gi2_nee => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_RIS,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_RIS,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_RIS,
                    ),
                    (true, false) if gi2_ris || gi2_nee => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_RIS,
                        G31_U_AE_PARAMS_TEXNRM_RIS,
                        G31_U_AE_PARTIALS_TEXNRM_RIS,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_RIS,
                        G31_U_PLAN_AE_STATE_TEXNRM_RIS,
                    ),
```

### 锚⑮ set_autoexp 选择块(锚:`                        let (pi, ti) = if textures && smooth_nrm && transparency && bloom {`,在其**之前**插两分支——与锚⑭ match 序逐字同构):
```rust
                        let (pi, ti) = if textures && smooth_nrm && (gi2_ris || gi2_nee) && bloom {
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS,
                            )
                        } else if textures && smooth_nrm && (gi2_ris || gi2_nee) {
                            (G31_U_AE_PARAMS_TEXNRM_RIS, G31_U_AE_PARTIALS_TEXNRM_RIS)
                        } else if textures && smooth_nrm && transparency && bloom {
```
(原首分支降为第三分支,`let (pi, ti) =` 头移到新首分支。)

### 锚⑯ 挂载点(锚:`                    if transparency {\n                        l.set_transparency();\n                    }`,其后插):
```rust
                    // G37 W2 臂⑧:--gi2-ris/--gi2-nee → params[69..72)
                    // (off 不挂载 ⇒ 三槽不写参数面 0-byte)。
                    if gi2_ris || gi2_nee {
                        l.set_gi2_ris(gi2_ris, gi2_ris_m_v as f32, gi2_nee);
                    }
```

## 七、GPU 验收步骤(主agent执行;RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,VUID=0 门;run_arm.py 环形态)

```powershell
# 0) 合入后构建窗口 bin(主agent构建纪律/target 目录);装配日志应见
#    「G37 W2 臂⑧ 灯片表 44024 片(…总功率 …,748412 f32 = 2993648 B)」
# 1) all-off 锚零漂移(off 面不载新 SPV):== 55e4a92d…
g31_window_present.exe --frames 8 --warmup 2 --hidden --evidence ev_alloff.json
# 2) 两臂单开最小组合,各双跑位级(digest run1 == run2 + VUID=0)
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-ris on --evidence ev_ris_1.json
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-nee on --evidence ev_nee_1.json
# 3) 双臂组合 + full 预设(若锚⑪采纳:--quality full 已含两臂)双跑位级 + 帧时
#    记账(基线十七臂,预算 11.11ms;成本预估:ris M=6 ≈ +6×~80ALU+96 表读/px·
#    反弹,nee 臂 +1 条阴影射线/px·反弹——A1 斜率口径预估 +0.2~0.5ms,实测为准)
g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_full19_1.json
# 4) 无 AE A/B(红修 #1 定形;EXPLICIT_NOAE 十七臂显式 + 臂旗标;掩码 = GI2
#    间接光面;判读 tools/ab_metrics.py 跳头版;dump 形状 run_arm.py 定形)
#    判据建议:nee 臂 = 反弹通道方差显降(conv 协议 std_p95,C 相口径)+
#    |Δmean| 有界(能量口径换代:代表灯反弹让位真面光,幅度登记);
#    ris 臂 = 同均值方差降(选灯升级不改积分域)。
# 5) 正交抽查:ris on 而 transparency off(零表占位链位)/nee on 而 gi2-tex off
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-ris on --gi2-nee on --gi2-ris-m 8 --evidence ev_orth.json
```

## 八、tsrq clamp K 阶梯(EVAL_DENOISE 第 0 级,零代码 GPU 实验;K 旋钮 = `--tsrq-clamp`〔tsr_params[20],须随 --tsr-quality on——full 已含;子参数不进 dup 表可与 full 直接组合〕)

```powershell
# 全程 GPU 锁内;RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1
# ── 窗口腿(presented 口径;K=0 基线 = --quality full 裸跑,k_on 门位级恒等)──
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_k0.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 3   --evidence ev_k3.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 2   --evidence ev_k2.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 1.5 --evidence ev_k15.json
# ── bench conv 腿(EVAL_DENOISE §8 首选判据口径;K=0 基线 = D 相 arm4 在案
#    d_metrics.json 可直接对照,无须重跑)──
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 3   --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k3
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 2   --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k2
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 1.5 --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k15
# 判读:artifacts/day_0828/d_tsr/d_ladder.py 的 ARMS 表指到 kladder/k{X}/
#   bistro-interior/tier100/tsr_device(四 ROI conv std_p95 vs arm4 K=0 在案
#   基线,frame_01*.exr stride2×16 末段口径)。
# 双判据(EVAL_DENOISE §8):①微光点再降(四 ROI std_p95)②远小灯保真
#   (dolly f0240 远灯 ROI 亮度不降超阈——K 过小误杀合法孤立小灯,
#   g31_tsr_resolve_q.rx:28 原注;D 相 dolly 协议同位)。
# 产出 tsrq_clamp_ladder.json:存在正区间 ⇒ 定档入 full 复评;不存在 ⇒
#   登记「K 档关死,降噪投资转第 1 级(tsrq v4 方差引导)」。
```

## 附:与 W2 transparency 臂的合入次序

两臂同改 `g31_realism.rx` 域但**本臂已含 transparency 段**(第 8 链位 = 第 7
链位超集,transp SPV 35983d0f 冻结不动);窗口 bin 合入锚点集与 transparency
提案(w2_wiring/transparency/REPORT.md,已合入)零冲突——本提案全部锚点取
transparency 合入后字面。若 full 预设并入两臂,重锚一次归 W4(与 transparency
重锚同窗)。
