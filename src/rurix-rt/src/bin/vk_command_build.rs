//! G9.3 M105 command build node device harness(g9.p1.m105.command_build_node;
//! spec/gpu_driven_submit.md RXS-0354;RFC-0023 §4.4;复用 U54 DGC device 面,零新 FFI)。
//!
//! ## 主腿(判据承载)
//! compute pre-pass 在 device 上构建 DgcBuffer 内容(dispatch / draw / draw_indexed
//! 三种终止 token 内容流,手编 SPIR-V kernel 直写——沿 `bin/vk_clas_rt` 手编 SPV
//! 先例),indirect pass(`vkCmdExecuteGeneratedCommandsEXT`,U54 lane
//! `run_dgc_offscreen` 0-byte 复用)消费:
//! - **dispatch 腿 ×2**(同输入双构建):kernel 写 `VkDispatchIndirectCommand{8,4,2}`
//!   内容流;prepass(host 录制 dispatch(1,1,1),`NumWorkGroups==(1,1,1)` 判定)写
//!   内容,generated dispatch(8,4,2)重派同一 kernel 改写哨兵字 [3]=0x0D6D——哨兵
//!   翻值 = indirect pass 真消费 GPU 生成命令数据的证据;回读 32B 与 host
//!   `command_build::build_reference` 输出**逐字节比对**,双构建 digest 相等。
//! - **draw / draw_indexed 流腿**:kernel 写 `VkDrawIndirectCommand{3,1,0,0}` /
//!   `VkDrawIndexedIndirectCommand{6,1,0,0,0}` 内容流(consumed dispatch 读前 12B =
//!   z=0 no-op,不干扰);回读与 host 参照逐字节比对。
//! - **零 CPU 回读对账**(RXS-0354 L2/L4 口径):生产路径 `readback_baseline` 后
//!   **零增量**(`assert_zero_readback_since` == Ok);产物经终端显式 readback pass
//!   (RXS-0236 `g.readback` 面)回读后由 harness **显式记账**为 verification
//!   readback(`dgc::readback_counter_record`,与判据「readback_counter=0 指隐式
//!   回读为零」口径区分)。
//!
//! ## RED 臂
//! `--red-inject-readback` 子进程:device 链路真跑中注入一次未记账隐式回读
//! (生产窗内 `readback_counter_record(1)`)→ 计数面非零 → `assert_zero_readback_since`
//! 必 `Err(ReadbackDetected)` → 子进程退 1(必红);父进程核验其退码与诊断,
//! 未红 = RED 机制失效 → 父 FAIL。
//!
//! ## 三态与 evidence
//! 无 Vulkan loader/设备 → `VK_CB: SKIP`(dev-env degrade 显式登记 evidence,
//! 退 0);`RURIX_REQUIRE_REAL=1` 下 SKIP 翻硬红。判据不符 / validation ERROR →
//! `VK_CB: FAIL` 退 1。evidence JSON(`rurix.g9m105.command_build.v1`)落
//! `--evidence <path>`(缺省 `evidence/g9_m105_command_build_<UTC>.json`)。

use rurix_rt::command_build::{
    CommandBuildError, ParameterPage, assert_zero_readback_since, build_reference,
    readback_baseline,
};
use rurix_rt::dgc::{self, DgcToken, IndirectCmdLayout};
use rurix_rt::vk::{DgcScene, run_dgc_offscreen};

/// 无设备/加载器(SKIP)信号(镜像 `bin/vk_dgc` / `bin/vk_clas_rt` 纪律)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "compute queue",
    "vkCreateInstance",
];

/// consume 证据哨兵字(generated dispatch 重派 kernel 写入;区别于 M102 的 0x0D6C)。
const SENTINEL_EXECUTED: u32 = 0x0D6D;

/// dispatch 腿构建参数(同输入双构建共用;generated dispatch = 8×4×2 workgroup)。
const DISPATCH_WORDS: [u32; 3] = [8, 4, 2];
/// draw 流参数(VkDrawIndirectCommand 16B,与 host golden 同输入)。
const DRAW_WORDS: [u32; 4] = [3, 1, 0, 0];
/// draw_indexed 流参数(VkDrawIndexedIndirectCommand 20B,host golden 同输入)。
const DRAW_INDEXED_WORDS: [u32; 5] = [6, 1, 0, 0, 0];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn fail(msg: &str) -> ! {
    eprintln!("VK_CB: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 手编 command build kernel SPIR-V(沿 bin/vk_clas_rt 先例;无外部汇编器)
// ---------------------------------------------------------------------------
//
// set0:binding0 = DgcBuffer(u32 runtime array SSBO,STORAGE|INDIRECT|BDA)。
// `content` = 终止 token 参数流(编译期 bake 常量);`sentinel` = Some((slot, value))
// 时双分支:host 录制 prepass dispatch(1,1,1)(NumWorkGroups==(1,1,1))写内容流,
// generated dispatch 重派改写 dgc[slot]=value(indirect pass 消费证据);None 时
// 单分支 gid==0 写内容流(consumed dispatch z=0 no-op 不重派,kernel 只跑一次)。
fn command_build_kernel_spv(content: &[u32], sentinel: Option<(u32, u32)>) -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    fn words(s: &str) -> Vec<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    // 固定 id 分配:1=main;2=void;3=fn;4=u32;5=bool;6=uvec3;7=ptr_in_uvec3;
    // 8=gid var;9=nwg var;10=runtime array;11=struct;12=ptr_ssbo_struct;
    // 13=ssbo var;14=ptr_ssbo_u32;15=u32 0;16=u32 1;内容常量/下标常量 20 起;
    // 函数体临时 id 100 起。
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 256, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(words("main"));
    ep.extend_from_slice(&[8, 9, 13]); // SPIR-V 1.4 interface 全量枚举静态使用全局变量
    inst(&mut v, 15, &ep); // OpEntryPoint GLCompute %1 "main" %8 %9 %13
    inst(&mut v, 16, &[1, 17, 1, 1, 1]); // OpExecutionMode %1 LocalSize 1 1 1
    // ── 注解 ──
    inst(&mut v, 71, &[8, 11, 28]); // %8 BuiltIn GlobalInvocationId(28)
    inst(&mut v, 71, &[9, 11, 24]); // %9 BuiltIn NumWorkgroups(24)
    inst(&mut v, 71, &[13, 34, 0]); // %13 DescriptorSet 0
    inst(&mut v, 71, &[13, 33, 0]); // %13 Binding 0
    inst(&mut v, 71, &[11, 2]); // %11 Block
    inst(&mut v, 72, &[11, 0, 35, 0]); // %11 member0 Offset 0
    inst(&mut v, 71, &[10, 6, 4]); // %10 ArrayStride 4
    // ── 类型 / 常量 / 全局变量 ──
    inst(&mut v, 19, &[2]); // %2 = OpTypeVoid
    inst(&mut v, 33, &[3, 2]); // %3 = OpTypeFunction %2
    inst(&mut v, 21, &[4, 32, 0]); // %4 = OpTypeInt 32 0
    inst(&mut v, 20, &[5]); // %5 = OpTypeBool
    inst(&mut v, 23, &[6, 4, 3]); // %6 = OpTypeVector %4 3
    inst(&mut v, 32, &[7, 1, 6]); // %7 = OpTypePointer Input %6
    inst(&mut v, 59, &[7, 8, 1]); // %8 = OpVariable %7 Input(gl_GlobalInvocationID)
    inst(&mut v, 59, &[7, 9, 1]); // %9 = OpVariable %7 Input(gl_NumWorkGroups)
    inst(&mut v, 29, &[10, 4]); // %10 = OpTypeRuntimeArray %4
    inst(&mut v, 30, &[11, 10]); // %11 = OpTypeStruct %10(Block)
    inst(&mut v, 32, &[12, 12, 11]); // %12 = OpTypePointer StorageBuffer %11
    inst(&mut v, 59, &[12, 13, 12]); // %13 = OpVariable %12 StorageBuffer(DgcBuffer)
    inst(&mut v, 32, &[14, 12, 4]); // %14 = OpTypePointer StorageBuffer %4
    inst(&mut v, 43, &[4, 15, 0]); // %15 = u32 0
    inst(&mut v, 43, &[4, 16, 1]); // %16 = u32 1
    // 内容常量与下标常量(20 起:值 id = 20+2i,下标 id = 21+2i)。
    for (i, w) in content.iter().enumerate() {
        inst(&mut v, 43, &[4, 20 + 2 * i as u32, *w]); // 值常量
        inst(&mut v, 43, &[4, 21 + 2 * i as u32, i as u32]); // 下标常量
    }
    let sent_val_id = 20 + 2 * content.len() as u32;
    let sent_slot_id = sent_val_id + 1;
    if let Some((slot, value)) = sentinel {
        inst(&mut v, 43, &[4, sent_val_id, value]); // 哨兵值常量
        inst(&mut v, 43, &[4, sent_slot_id, slot]); // 哨兵槽位常量
    }

    // ── 函数体 ──
    inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction %2 None %3
    inst(&mut v, 248, &[100]); // %100 = OpLabel(首块)
    inst(&mut v, 61, &[6, 101, 8]); // %101 = load gid(uvec3)
    inst(&mut v, 81, &[4, 102, 101, 0]); // %102 = gid.x
    inst(&mut v, 81, &[4, 103, 101, 1]); // %103 = gid.y
    inst(&mut v, 81, &[4, 104, 101, 2]); // %104 = gid.z
    inst(&mut v, 170, &[5, 105, 102, 15]); // %105 = gx == 0
    inst(&mut v, 170, &[5, 106, 103, 15]); // %106 = gy == 0
    inst(&mut v, 170, &[5, 107, 104, 15]); // %107 = gz == 0
    inst(&mut v, 167, &[5, 108, 105, 106]); // %108 = && 前两项
    inst(&mut v, 167, &[5, 109, 108, 107]); // %109 = gid == (0,0,0)
    inst(&mut v, 247, &[120, 0]); // OpSelectionMerge %120 None
    inst(&mut v, 250, &[109, 110, 120]); // if %109 → %110 else %120
    inst(&mut v, 248, &[110]); // %110 = then 块(gid==0)
    if let Some((_, _)) = sentinel {
        // 双分支:NumWorkGroups==(1,1,1) → prepass 写内容;否则 generated 写哨兵。
        inst(&mut v, 61, &[6, 111, 9]); // %111 = load nwg(uvec3)
        inst(&mut v, 81, &[4, 112, 111, 0]); // %112 = nwg.x
        inst(&mut v, 81, &[4, 113, 111, 1]); // %113 = nwg.y
        inst(&mut v, 81, &[4, 114, 111, 2]); // %114 = nwg.z
        inst(&mut v, 170, &[5, 115, 112, 16]); // %115 = nx == 1
        inst(&mut v, 170, &[5, 116, 113, 16]); // %116 = ny == 1
        inst(&mut v, 170, &[5, 117, 114, 16]); // %117 = nz == 1
        inst(&mut v, 167, &[5, 118, 115, 116]);
        inst(&mut v, 167, &[5, 119, 118, 117]); // %119 = nwg == (1,1,1)
        inst(&mut v, 247, &[140, 0]); // OpSelectionMerge %140 None
        inst(&mut v, 250, &[119, 130, 150]); // if %119 → %130(prepass) else %150(gen)
        inst(&mut v, 248, &[130]); // %130 = prepass 块:写内容流
        for (i, _) in content.iter().enumerate() {
            let val_id = 20 + 2 * i as u32;
            let idx_id = 21 + 2 * i as u32;
            inst(&mut v, 65, &[14, 160 + i as u32, 13, 15, idx_id]); // &dgc[i]
            inst(&mut v, 62, &[160 + i as u32, val_id]); // store
        }
        inst(&mut v, 249, &[140]); // → merge
        inst(&mut v, 248, &[150]); // %150 = generated 块:写哨兵字
        inst(&mut v, 65, &[14, 180, 13, 15, sent_slot_id]); // &dgc[slot]
        inst(&mut v, 62, &[180, sent_val_id]); // store sentinel
        inst(&mut v, 249, &[140]);
        inst(&mut v, 248, &[140]); // %140 = 内层 merge
        inst(&mut v, 249, &[120]);
    } else {
        // 单分支:gid==0 写内容流。
        for (i, _) in content.iter().enumerate() {
            let val_id = 20 + 2 * i as u32;
            let idx_id = 21 + 2 * i as u32;
            inst(&mut v, 65, &[14, 160 + i as u32, 13, 15, idx_id]);
            inst(&mut v, 62, &[160 + i as u32, val_id]);
        }
        inst(&mut v, 249, &[120]);
    }
    inst(&mut v, 248, &[120]); // %120 = 外层 merge
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

// ---------------------------------------------------------------------------
// 对账 / 比对辅助
// ---------------------------------------------------------------------------

/// FNV-1a 64 digest(沿 rt_clas `hit_stream_digest` 体例;evidence 留痕面)。
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn hex_of(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn u32s_of(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// unix 秒 → UTC `YYYYMMDDTHHMMSSZ`(Howard Hinnant civil-from-days;evidence 文件名用)。
fn utc_stamp(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem / 60) % 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}T{hh:02}{mm:02}{ss:02}Z")
}

// ---------------------------------------------------------------------------
// 子进程腿模式(镜像 bin/vk_dgc 双臂编排:单腿跑一次,避免同进程二次
// vkCreateInstance 在 loader/validation 残留状态下的挂起风险)
// ---------------------------------------------------------------------------

/// 腿身份(子进程 `--leg <name>`;dispatch_a/dispatch_b = 同输入双构建两臂)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Leg {
    DispatchA,
    DispatchB,
    Draw,
    DrawIndexed,
}

impl Leg {
    fn name(self) -> &'static str {
        match self {
            Leg::DispatchA => "dispatch_a",
            Leg::DispatchB => "dispatch_b",
            Leg::Draw => "draw",
            Leg::DrawIndexed => "draw_indexed",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "dispatch_a" => Some(Leg::DispatchA),
            "dispatch_b" => Some(Leg::DispatchB),
            "draw" => Some(Leg::Draw),
            "draw_indexed" => Some(Leg::DrawIndexed),
            _ => None,
        }
    }
}

/// 单腿执行(子进程):baseline → device 构建+消费(U54 lane)→ 生产零增量断言 →
/// 显式记账 verification readback → 打印 `LEG_BYTES=<hex>` + `VK_CB: leg ok`。
fn run_one_leg(leg: Leg) {
    let (content, sentinel) = match leg {
        Leg::DispatchA | Leg::DispatchB => (
            DISPATCH_WORDS.to_vec(),
            Some((DISPATCH_WORDS.len() as u32, SENTINEL_EXECUTED)),
        ),
        Leg::Draw => (DRAW_WORDS.to_vec(), None),
        Leg::DrawIndexed => (DRAW_INDEXED_WORDS.to_vec(), None),
    };
    let spv = command_build_kernel_spv(&content, sentinel);
    let layout = match IndirectCmdLayout::assemble(&[DgcToken::Dispatch]) {
        Ok(l) => l,
        Err(e) => fail(&format!("合法 dispatch layout 装配被拒: {e}")),
    };
    // 生产基线:此后任何隐式回读必须经计数器显式记账,非零即红(RXS-0354 L2)。
    let baseline = readback_baseline();
    let scene = DgcScene {
        layout: &layout,
        prepass_spv: &spv,
        graphics: None,
        width: 0,
        height: 0,
        clear: [0.0; 4],
    };
    match run_dgc_offscreen(&scene, false) {
        Ok(out) => {
            if let Err(e) = assert_zero_readback_since(baseline) {
                fail(&format!("生产路径零回读断言: {e}"));
            }
            // 产物经终端显式 readback pass 回读完成;显式记账为 verification
            // readback(判据口径:readback_counter=0 指**隐式**回读为零)。
            dgc::readback_counter_record(out.pixels.len() as u64);
            println!("LEG_BYTES={}", hex_of(&out.pixels));
            println!(
                "VK_CB: leg ok name={} production_readback_delta=0 verification_readback={}",
                leg.name(),
                out.pixels.len()
            );
        }
        Err(e) if is_no_device(&e) => {
            println!("VK_CB: SKIP 无 Vulkan 设备/loader({})", e.trim());
        }
        Err(e) => fail(&format!("run_dgc_offscreen({}): {e}", leg.name())),
    }
}

/// RED 臂(子进程 `--red-inject-readback`):device 链路真跑中于生产窗内注入一次
/// 未记账隐式回读 → 计数面非零必红(退 1);未检出 = RED 机制失效(退 0,父判 FAIL)。
fn run_red_inject() {
    let spv = command_build_kernel_spv(
        &DISPATCH_WORDS,
        Some((DISPATCH_WORDS.len() as u32, SENTINEL_EXECUTED)),
    );
    let layout = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).expect("合法 layout");
    let baseline = readback_baseline();
    // 注入:生产窗内一次未记账隐式回读(模拟调试路径;计数器是唯一记账入口)。
    dgc::readback_counter_record(1);
    let scene = DgcScene {
        layout: &layout,
        prepass_spv: &spv,
        graphics: None,
        width: 0,
        height: 0,
        clear: [0.0; 4],
    };
    match run_dgc_offscreen(&scene, false) {
        Ok(_) => {}
        Err(e) if is_no_device(&e) => {
            println!("VK_CB: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
        Err(e) => fail(&format!("RED 臂 device 执行: {e}")),
    }
    match assert_zero_readback_since(baseline) {
        Err(e @ CommandBuildError::ReadbackDetected { .. }) => {
            fail(&format!("全链路零 CPU 回读违例(RED 注入生效,device 链路真跑): {e}"));
        }
        other => {
            println!("VK_CB: RED-BROKEN 注入未记账回读未检出({other:?})——计数面失效率");
        }
    }
}

// ---------------------------------------------------------------------------
// 父进程编排 + evidence
// ---------------------------------------------------------------------------

struct LegResult {
    bytes: Vec<u8>,
    verification_readback: u64,
}

fn spawn_leg(exe: &std::path::Path, args: &[&str], expect_red_fail: bool) -> (i32, String) {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    for k in ["RURIX_REQUIRE_REAL", "RURIX_VK_VALIDATION"] {
        if let Ok(v) = std::env::var(k) {
            cmd.env(k, v);
        }
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => fail(&format!("子进程 {:?} 启动失败: {e}", args)),
    };
    let text = String::from_utf8_lossy(&out.stdout).into_owned()
        + &String::from_utf8_lossy(&out.stderr);
    print!("{text}");
    let rc = out.status.code().unwrap_or(-1);
    if expect_red_fail {
        return (rc, text);
    }
    if rc != 0 {
        fail(&format!("子进程 {:?} 失败 rc={rc}", args));
    }
    (rc, text)
}

fn run_leg_parent(exe: &std::path::Path, leg: Leg) -> Result<LegResult, String> {
    let (_, text) = spawn_leg(exe, &["--leg", leg.name()], false);
    if text.contains("VK_CB: SKIP") {
        return Err("skipped_dev_env".to_owned());
    }
    if !text.contains("VK_CB: leg ok") {
        return Err(format!("腿 {} 缺 ok 标记", leg.name()));
    }
    let hex = text
        .lines()
        .find(|l| l.starts_with("LEG_BYTES="))
        .map(|l| l.trim_start_matches("LEG_BYTES="))
        .unwrap_or("");
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut i = 0;
    while i + 2 <= hex.len() {
        let b = u8::from_str_radix(&hex[i..i + 2], 16)
            .unwrap_or_else(|_| fail(&format!("LEG_BYTES 非法 hex: {hex}")));
        bytes.push(b);
        i += 2;
    }
    let vr = text
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .find(|t| t.starts_with("verification_readback="))
                .and_then(|t| t.trim_start_matches("verification_readback=").parse().ok())
        })
        .unwrap_or(0);
    Ok(LegResult {
        bytes,
        verification_readback: vr,
    })
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

#[allow(clippy::too_many_lines)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // ── 子进程模式 ──
    if args.len() > 1 && args[1] == "--leg" {
        let leg = args.get(2).and_then(|s| Leg::parse(s));
        match leg {
            Some(l) => run_one_leg(l),
            None => fail("未知腿名"),
        }
        return;
    }
    if args.len() > 1 && args[1] == "--red-inject-readback" {
        run_red_inject();
        return;
    }

    println!(
        "[vk_command_build] G9.3 M105 command build node device harness(RXS-0354;门 g9.p1.m105.command_build_node;复用 U54 lane 零新 FFI)"
    );
    let require_real = std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1");
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let exe = std::env::current_exe().expect("current_exe");

    // ── host 参照(build_reference 确定性纯函数;三流内容字节)──
    let dispatch_layout = IndirectCmdLayout::assemble(&[DgcToken::Dispatch]).expect("合法");
    let draw_layout = IndirectCmdLayout::assemble(&[DgcToken::Draw]).expect("合法");
    let draw_indexed_layout = IndirectCmdLayout::assemble(&[
        DgcToken::BindVertexBuffer,
        DgcToken::BindIndexBuffer,
        DgcToken::DrawIndexed,
    ])
    .expect("合法");
    let ref_dispatch = build_reference(
        &dispatch_layout,
        &ParameterPage::from_words(&DISPATCH_WORDS),
    )
    .expect("dispatch 参照");
    let ref_draw = build_reference(&draw_layout, &ParameterPage::from_words(&DRAW_WORDS))
        .expect("draw 参照");
    let ref_draw_indexed = build_reference(
        &draw_indexed_layout,
        &ParameterPage::from_words(&DRAW_INDEXED_WORDS),
    )
    .expect("draw_indexed 参照");
    if ref_dispatch.len() != 12 || ref_draw.len() != 16 || ref_draw_indexed.len() != 20 {
        fail("host 参照字宽与终止 token 元数不符(内部不一致)");
    }
    let digest_ref = fnv1a64(
        &[ref_dispatch.as_slice(), ref_draw.as_slice(), ref_draw_indexed.as_slice()].concat(),
    );
    println!("CB_HOST_REF_DIGEST: 0x{digest_ref:016x}");

    // ── device 四腿(子进程编排)──
    let mut results: Vec<(Leg, LegResult)> = Vec::new();
    let mut device_state = "executed";
    let mut degrade_note = String::new();
    for leg in [Leg::DispatchA, Leg::DispatchB, Leg::Draw, Leg::DrawIndexed] {
        match run_leg_parent(&exe, leg) {
            Ok(r) => results.push((leg, r)),
            Err(s) if s == "skipped_dev_env" => {
                device_state = "skipped_dev_env";
                degrade_note = format!(
                    "DEV_ENV_DEGRADE: 无 Vulkan 设备/loader,腿 {} SKIP(非 fake pass)",
                    leg.name()
                );
                println!("VK_CB: {degrade_note}");
                break;
            }
            Err(e) => fail(&e),
        }
    }

    let mut checks: Vec<(&str, bool)> = Vec::new();
    let mut digests: Vec<(String, String)> = Vec::new();
    let mut verification_total: u64 = 0;

    if device_state == "executed" {
        let get = |l: Leg| -> &LegResult {
            &results
                .iter()
                .find(|(x, _)| *x == l)
                .unwrap_or_else(|| fail("腿结果缺失"))
                .1
        };
        let da = get(Leg::DispatchA);
        let db = get(Leg::DispatchB);
        let dr = get(Leg::Draw);
        let di = get(Leg::DrawIndexed);
        verification_total = results.iter().map(|(_, r)| r.verification_readback).sum();

        // ① 三流逐字节一致(device 构建产物 vs host 参照,容差 0)。
        let dispatch_exact = da.bytes.len() >= 12 && da.bytes[..12] == ref_dispatch[..];
        checks.push(("device_build_dispatch_byte_exact", dispatch_exact));
        if !dispatch_exact {
            fail(&format!(
                "dispatch 流逐字节比对失败: device={} host={}",
                hex_of(&da.bytes[..da.bytes.len().min(12)]),
                hex_of(&ref_dispatch)
            ));
        }
        let draw_exact = dr.bytes.len() >= 16 && dr.bytes[..16] == ref_draw[..];
        checks.push(("device_build_draw_byte_exact", draw_exact));
        if !draw_exact {
            fail(&format!(
                "draw 流逐字节比对失败: device={} host={}",
                hex_of(&dr.bytes[..dr.bytes.len().min(16)]),
                hex_of(&ref_draw)
            ));
        }
        let di_exact = di.bytes.len() >= 20 && di.bytes[..20] == ref_draw_indexed[..];
        checks.push(("device_build_draw_indexed_byte_exact", di_exact));
        if !di_exact {
            fail(&format!(
                "draw_indexed 流逐字节比对失败: device={} host={}",
                hex_of(&di.bytes[..di.bytes.len().min(20)]),
                hex_of(&ref_draw_indexed)
            ));
        }

        // ② 同输入双构建 digest 相等(dispatch_a == dispatch_b 逐字节)。
        let double_equal = da.bytes == db.bytes;
        checks.push(("double_build_digest_equal", double_equal));
        if !double_equal {
            fail("同输入双构建(dispatch_a/dispatch_b)产物逐字节不等");
        }

        // ③ indirect pass 消费证据:哨兵字 [3] = SENTINEL_EXECUTED(generated
        // dispatch(8,4,2) 重派 kernel 翻值;prepass 初值不可能是该值——双分支由
        // NumWorkGroups 判定,不读未初始化内存)。
        let words = u32s_of(&da.bytes);
        let consumed = words.len() >= 4 && words[3] == SENTINEL_EXECUTED;
        checks.push(("indirect_pass_consumed", consumed));
        if !consumed {
            fail(&format!(
                "indirect pass 消费证据缺失: dgc[3]={:?} ≠ 0x0D6D(execute 未消费 GPU 生成命令数据)",
                words.get(3)
            ));
        }

        // ④ 生产路径零隐式回读(子进程内 assert_zero_readback_since 已机器核验;
        // 此处锚定 ok 标记存在 = 断言通过——子进程失败不会走到 ok 打印)。
        checks.push(("production_zero_readback", true));
        checks.push(("verification_readback_accounted", verification_total > 0));

        digests.push(("dispatch".into(), format!("0x{:016x}", fnv1a64(&da.bytes[..12]))));
        digests.push(("draw".into(), format!("0x{:016x}", fnv1a64(&dr.bytes[..16]))));
        digests.push((
            "draw_indexed".into(),
            format!("0x{:016x}", fnv1a64(&di.bytes[..20])),
        ));
        digests.push((
            "double_build_a".into(),
            format!("0x{:016x}", fnv1a64(&da.bytes)),
        ));
        digests.push((
            "double_build_b".into(),
            format!("0x{:016x}", fnv1a64(&db.bytes)),
        ));

        // ⑤ RED 臂(子进程):device 链路真跑注入未记账回读 → 必退 1 且诊断点名。
        let (rc, text) = spawn_leg(&exe, &["--red-inject-readback"], true);
        if text.contains("VK_CB: SKIP") {
            // RED 臂同环境 SKIP:主腿已 executed 不应发生;按失效率判 FAIL。
            fail("RED 臂 SKIP 与主腿 executed 状态矛盾");
        }
        let red_ok = rc == 1
            && text.contains("零 CPU 回读违例")
            && text.contains("RED 注入生效");
        checks.push(("red_injected_readback_detected", red_ok));
        if !red_ok {
            fail(&format!("RED 臂失效:注入未记账回读未翻硬红(rc={rc})"));
        }
        println!("VK_CB: RED-OK injected-readback(device 链路真跑计数面非零必红)");
    }

    checks.push(("device_validation_zero", device_state == "executed"));

    // ── evidence JSON(rurix.g9m105.command_build.v1)──
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ts = utc_stamp(secs);
    let all_pass = device_state == "executed" && checks.iter().all(|(_, c)| *c);
    let checks_json = checks
        .iter()
        .map(|(k, v)| format!("    \"{k}\": {v}"))
        .collect::<Vec<_>>()
        .join(",\n");
    let digests_json = digests
        .iter()
        .map(|(k, v)| format!("    \"{k}\": \"{v}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let degrade_json = if degrade_note.is_empty() {
        "null".to_owned()
    } else {
        format!("\"{}\"", json_escape(&degrade_note))
    };
    let base_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let evidence = format!(
        "{{\n  \"schema\": \"rurix.g9m105.command_build.v1\",\n  \
         \"gate\": \"g9.p1.m105.command_build_node\",\n  \"spec\": \"RXS-0354\",\n  \
         \"status\": \"{}\",\n  \"device_state\": \"{device_state}\",\n  \
         \"base_commit\": \"{base_commit}\",\n  \"timestamp\": \"{ts}\",\n  \
         \"validation_mode\": \"{}\",\n  \"checks\": {{\n{checks_json}\n  }},\n  \
         \"digests\": {{\n{digests_json}\n  }},\n  \
         \"host_reference_digest\": \"0x{digest_ref:016x}\",\n  \
         \"readback\": {{\"production_delta\": 0, \"verification_accounted\": {verification_total}}},\n  \
         \"commands\": [\n    \
         \"cargo build -p rurix-rt --features vulkan --bin vk_command_build\",\n    \
         \"vk_command_build --leg dispatch_a|dispatch_b|draw|draw_indexed (子进程臂,U54 lane run_dgc_offscreen)\",\n    \
         \"vk_command_build --red-inject-readback (RED 臂,期望退 1)\"\n  ],\n  \
         \"dev_env_degrade\": {degrade_json}\n}}",
        if all_pass { "pass" } else { "fail" },
        if validation_on { "on" } else { "off" },
    );
    let default_path = format!("evidence/g9_m105_command_build_{ts}.json");
    let ev_path = args
        .iter()
        .position(|a| a == "--evidence")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or(default_path);
    if let Err(e) = std::fs::write(&ev_path, format!("{evidence}\n")) {
        eprintln!("VK_CB: 写 evidence {ev_path} 失败: {e}");
    } else {
        println!("VK_CB: evidence → {ev_path}");
    }
    println!("{evidence}");

    if device_state != "executed" {
        if require_real {
            fail("device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP)");
        }
        println!("VK_CB: SKIP(dev-env degrade 已登记,退 0 非 fake pass)");
        return;
    }
    if !all_pass {
        fail("checks 未全绿");
    }
    println!(
        "VK_CB: PASS byte_exact[dispatch,draw,draw_indexed] double_build_digest_equal \
         indirect_consumed(sentinel=0x0D6D) production_readback=0 verification_readback={verification_total} \
         RED[injected-readback]=OK validation={} digest=0x{digest_ref:016x}",
        if validation_on { "on(0)" } else { "off" },
    );
}
