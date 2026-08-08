# A4 / M70 vehicle 薄壳假绿清零 notes(2026-08-08)

## 基线(00_vehicle_baseline.txt)

旧 `vehicle_subject_pass`(mod.rs 94-99)只断言 `cook_deterministic_double` + `wheels>=2` +
digest 非空,单 bool 自指充绿——典型 thin-shell,6 腿名义全部无实现。

## 处置:目标 A(真实现 6 腿),无降级

按 G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §5.3 六腿逐腿实现,`g8-physics-gates vehicle`
吐逐腿布尔,`ci/g8_wave6d_exit_check.py` 逐腿断言(任一腿 False/缺失 → wave6d.exit 红,
禁单 bool 自指):

| 腿 | 实现 | 证伪臂(selftest 实测红) |
|---|---|---|
| asset_roundtrip | canonical_json → strict parse → 再序列化字节相等 + digest 相等;未知 schema_version fail-closed(内建) | asset_id 篡改 → 检出 |
| fixed_input_replay_hash_equal | 240 tick 固定输入双跑末态 hash 全等 + 输入日志(journal 形态)序列化往返后再放全等 | 篡改 tick=100 油门 → hash 不等 |
| rollback_correction_converges | tick=120 快照 serialize/parse 恢复 + 重放 121..240 → 与连续模拟末态 hash 逐位收敛 | 重放序列篡改 tick=210 制动 → 不收敛 |
| tire_light_object_contact_regression_golden | 轮胎推挤轻物体接触 trace(tick,pen,obj_x bits)digest == 冻结 golden;非空断言 events>=3(实测 37) | 轻物体初始位置 +0.05m → digest 偏离 golden |
| state_serialization_roundtrip | tick=137 中途态 serialize→parse→再序列化字节相等 + state hash 相等;NaN/尾部垃圾 fail-closed | gear 字段篡改 → 检出 |
| telemetry_trace_golden | 240 行遥测(rpm/gear/vx/y/obj_x/susp0 bits)digest == 冻结 golden;行数非空断言 | 篡改 tick=50 油门 → digest 偏离 |

golden 两条为 2026-08-08 本机首跑 measured 后冻结的常量(legs.rs GOLDEN_*_DIGEST),
比较对象是真 sim 现算 digest,falsify 臂证明比较非空转。

## 诚实边界(不充绿的点)

- sim 为纯 Rust 确定性 fixture 模型:解析式地面 bump + 解析轻物体动量交换;**未声称**
  Jolt 世界 cast_ray/AddForceAtPoint 集成(§5.2 运行时形态的后半步),也未声称 Jolt
  VehicleConstraint 行为对拍。六腿判据全部为状态版本化/replay/rollback/golden,
  与该边界一致(RFC-0021 §4.D1)。
- golden/evidence 均为 measured_local、同 build 同画像有效(f32 逐位),不跨平台宣称。

## 验证记录

- `g8-physics-gates vehicle` → 6/6 腿 true(final_state_hash 36db4fb9…,contact 37 事件)。
- `py -3 ci/g8_wave6d_exit_check.py --selftest` → 12/12 臂 PASS(6 条真二进制 falsify 全红
  + 6 条聚合篡改全红),exit 0。
- M72 零回归:`py -3 ci/g8_cloth_product_chain_smoke.py --gate g8.p1.m72.cloth_product_chain`
  → PASS,evidence/g8_m72_cloth_product_chain_20260808T030955Z.json。
- `py -3 ci/g8_wave6d_exit_check.py --gate g8.wave.6d.exit` → VERDICT = PASS,
  evidence/g8_wave6d_exit_20260808T031010Z.json(subjects[0].legs 逐腿布尔落盘)。
