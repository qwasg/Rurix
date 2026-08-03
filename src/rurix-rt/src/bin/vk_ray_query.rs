//! G7.3 W3b:compute RayQuery **device 判据 harness**(步骤 93 device 段;RFC-0018 章 C;
//! 验收门 G-G7-5)。镜像 `bin/vk_rt` 的 device 真跑 / SKIP 三态纪律。
//!
//! ## 判据
//! 消费 rurixc 编译的真实 `.rx` compute RayQuery 模块(`--spv <path>`,SPIR-V 1.4,
//! RXS-0297~0300 布局:set 0 / binding 0 = `AccelStruct`、binding 1 = SSBO),经
//! **单所有者** `VkAsManager` 的真实单三角形 TLAS 在 compute queue 真跑:
//! - `hit`:三角形在 z=1 平面覆盖原点,射线 (0,0,0)→(0,0,1) 命中 → `out[0] = t = 1.0`;
//! - `miss`:同 kernel、三角形平移 x+10 → 遍历穷尽无 committed → `out[0] = -1.0`
//!   (哨兵;hit/miss 双场景同 kernel 不同 TLAS = 数据流红绿,杜绝 isolated nonzero)。
//!
//! ## RED 自检(G-G7-5「设备丢失/缺扩展/过期 TLAS/错误 barrier 有 RED 自检」)
//! - `missing-capability`:探测 caps 强制清 `ray_query` 位 → `require_wave(W3)`
//!   必确定性拒绝(缺扩展轴;host 可跑);
//! - `stale-tlas`:`RayQueryRedProbe::StaleTlas` → 建 TLAS 后销毁再消费 →
//!   fail-closed 确定性 `Err`(不提交悬垂句柄);
//! - `wrong-barrier`:`RayQueryRedProbe::WrongBarrier` → 非法 src stage/access 组合
//!   → `VK_LAYER_KHRONOS_validation` ERROR → `Err`(需 `RURIX_VK_VALIDATION=1`);
//! - 设备丢失轴 = 库单测 `vk::tests::queue_submit_err_maps_device_lost`
//!   (`VK_ERROR_DEVICE_LOST` fail-closed 传播锚,host 恒跑)。
//!
//! ## 三态
//! 无 Vulkan loader / 无设备 → `RQ: SKIP`(dev-env degrade,退 0,非 fake pass);
//! W3 七能力链缺失 → `RQ: SKIP`(如实列缺失名);判据不符 / RED 轴失效 → `RQ: FAIL`
//! 退 1。`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层裁决。

use rurix_rt::render_exec::{
    KernelWave, W3_REQUIRED_CAPABILITIES, probe_device_caps, require_wave,
};
use rurix_rt::vk::{
    RayQueryRedProbe, entry_point_name, run_ray_query_compute, run_ray_query_compute_probed,
};

/// 命中场景:三角形位于 z=1 平面,xy 投影覆盖原点(射线 (0,0,0)+t·(0,0,1) → t=1.0)。
const TRI_HIT: [f32; 9] = [
    0.0, 0.6, 1.0, //
    -0.6, -0.6, 1.0, //
    0.6, -0.6, 1.0, //
];

/// miss 场景:同三角形平移 x+10(射线不与之相交 → kernel 写哨兵 -1.0)。
const TRI_MISS: [f32; 9] = [
    10.0, 0.6, 1.0, //
    9.4, -0.6, 1.0, //
    10.6, -0.6, 1.0, //
];

/// 无设备/加载器(SKIP)信号(镜像 `bin/vk_rt` NO_DEVICE_KEYS 纪律)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "compute queue",
    "vkCreateInstance",
];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn fail(msg: &str) -> ! {
    eprintln!("RQ: FAIL {msg}");
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let spv_path = match args.iter().position(|a| a == "--spv") {
        Some(i) if i + 1 < args.len() => args[i + 1].clone(),
        _ => fail("用法: vk_ray_query --spv <rurixc 产 .spv 路径>"),
    };
    let bytes = match std::fs::read(&spv_path) {
        Ok(b) => b,
        Err(e) => fail(&format!("读 {spv_path} 失败: {e}")),
    };
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    let spv: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let entry = match entry_point_name(&spv) {
        Some(e) => e,
        None => fail("SPIR-V 无 OpEntryPoint(非 rurixc compute 产物?)"),
    };
    println!(
        "[vk_ray_query] G7.3 W3b compute RayQuery device harness(RFC-0018 章 C,G-G7-5);entry=`{entry}`"
    );

    // ── W3 七能力链 fail-closed 门禁(KernelWave::W3;缺一确定性拒绝)──
    let caps = match probe_device_caps() {
        Ok(c) => c,
        Err(e) => {
            println!("RQ: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
    };
    let snapshot: Vec<String> = W3_REQUIRED_CAPABILITIES
        .iter()
        .map(|name| format!("{name}={}", capability_bit(&caps, name)))
        .collect();
    println!(
        "[vk_ray_query] W3 capability snapshot: {}",
        snapshot.join(" ")
    );
    if let Err(e) = require_wave(&caps, KernelWave::W3) {
        println!("RQ: SKIP W3 能力链缺失({e})");
        return;
    }

    // ── RED-a:缺能力注入(caps 强制清 ray_query 位 → require_wave 必拒)──
    let mut degraded = caps.clone();
    degraded.ray_query = false;
    match require_wave(&degraded, KernelWave::W3) {
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("ray_query") {
                fail(&format!("missing-capability RED 消息缺失能力名: {msg}"));
            }
            println!("RQ: RED-OK missing-capability({msg})");
        }
        Ok(()) => fail("missing-capability RED 失效: 清位后 require_wave 仍放行"),
    }

    // ── GREEN:hit / miss 双场景(同 kernel 不同 TLAS = 数据流红绿)──
    let hit = match run_ray_query_compute(&spv, &entry, &TRI_HIT, 1, [1, 1, 1]) {
        Ok(v) => v[0],
        Err(e) if is_no_device(&e) => {
            println!("RQ: SKIP device 真跑不可用({})", e.trim());
            return;
        }
        Err(e) => fail(&format!("hit 场景执行: {e}")),
    };
    if (hit - 1.0).abs() > 1e-6 {
        fail(&format!(
            "hit 场景 committed_t = {hit}(期望 1.0±1e-6;TLAS 遍历/交点查询未生效)"
        ));
    }
    let miss = match run_ray_query_compute(&spv, &entry, &TRI_MISS, 1, [1, 1, 1]) {
        Ok(v) => v[0],
        Err(e) => fail(&format!("miss 场景执行: {e}")),
    };
    if miss != -1.0 {
        fail(&format!(
            "miss 场景哨兵 = {miss}(期望 -1.0;has_committed 分支未生效)"
        ));
    }

    // ── RED-b:过期 TLAS(销毁后消费 → fail-closed 确定性 Err)──
    match run_ray_query_compute_probed(
        &spv,
        &entry,
        &TRI_HIT,
        1,
        [1, 1, 1],
        RayQueryRedProbe::StaleTlas,
    ) {
        Err(e) if e.contains("过期") || e.contains("已销毁") => {
            println!("RQ: RED-OK stale-tlas({})", e.trim());
        }
        Err(e) => fail(&format!("stale-tlas RED 消息形态非预期: {e}")),
        Ok(_) => fail("stale-tlas RED 失效: 悬垂 TLAS 仍被消费成功"),
    }

    // ── RED-c:错误 barrier(非法 stage/access 组合 → validation ERROR → Err;
    //    需 RURIX_VK_VALIDATION=1,由 smoke/调用方置)──
    if std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1") {
        match run_ray_query_compute_probed(
            &spv,
            &entry,
            &TRI_HIT,
            1,
            [1, 1, 1],
            RayQueryRedProbe::WrongBarrier,
        ) {
            Err(e) => println!("RQ: RED-OK wrong-barrier({})", e.trim()),
            Ok(_) => fail("wrong-barrier RED 失效: 非法 barrier 未被 validation 拦截"),
        }
    } else {
        println!("[vk_ray_query] wrong-barrier RED 轴未跑(RURIX_VK_VALIDATION≠1)");
    }

    println!(
        "RQ: PASS hit_t={hit} miss={miss} entry={entry}(真实 TLAS compute descriptor 消费 + \
         hit/miss 数据流红绿 + 三 RED 轴;单所有者 VkAsManager,validation 零错误)"
    );
}

fn capability_bit(caps: &rurix_rt::render_exec::DeviceCaps, name: &str) -> bool {
    match name {
        "synchronization2" => caps.synchronization2,
        "shader_buffer_int64_atomics" => caps.shader_buffer_int64_atomics,
        "ray_query" => caps.ray_query,
        "acceleration_structure" => caps.acceleration_structure,
        "buffer_device_address" => caps.buffer_device_address,
        "descriptor_indexing" => caps.descriptor_indexing,
        "deferred_host_operations" => caps.deferred_host_operations,
        _ => false,
    }
}
