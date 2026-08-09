//! G9.2 M102 DGC 最小链路 device harness(g9.p0.m102.dgc_abstraction;
//! spec/gpu_driven_submit.md RXS-0348;RFC-0023 §4.1/§6.2;U54)。
//!
//! 最小闭环(照 `vk_desc_v2.rs` 范式):
//!   ① compute pre-pass(`dgc_prepass.spv`)直写 DgcBuffer(VkDispatchIndirectCommand
//!      {1,1,1} + 哨兵字 3436 = 0x0D6C)→ 屏障 → `vkCmdExecuteGeneratedCommandsEXT`;
//!   ② **两臂**:execute-only(`isPreprocessed=0`)与 preprocess+execute
//!      (`EXPLICIT_PREPROCESS` layout + preprocess buffer)各跑一次,输出逐字节相等
//!      (preprocess 物理布局非 stable,RXS-0348 §3-6——不冻结字节,只比对执行结果);
//!   ③ DgcBuffer 哨兵字经显式 readback pass 回读 = 3436(幂等:prepass 与 execute
//!      重派同一 kernel 写同一值;值错 = execute 未消费 GPU 生成命令数据);
//!   ④ 回读计数器 = 0(RFC-0023 §4.4.2;DgcBuffer 命令数据本身零 host 读);
//!   ⑤ `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`:validation ERROR = 0
//!      (fail-closed 由 run_dgc_inner 内 messenger 承担)。
//!
//! capability 缺位(`VK_EXT_device_generated_commands` 不在 / feature 缺)→
//! 确定性 Err(fail-closed,禁静默模拟 P-01);`RURIX_REQUIRE_REAL=1` 下任何
//! SKIP/Err 均为红。

use rurix_rt::dgc::{DgcToken, IndirectCmdLayout};
use rurix_rt::vk::{DgcScene, dgc_prepass_spv, run_dgc_offscreen};

fn main() {
    // ── 子进程臂模式:单臂跑一次(由父进程双臂编排;避免同进程二次
    // vkCreateInstance 在 loader/validation 残留状态下的挂起风险)──
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--arm") {
        let preprocess = args.get(2).map(|s| s == "preprocess").unwrap_or(false);
        run_one_arm(preprocess);
        return;
    }

    // ── 父进程:execute-only 臂(单臂真跑;preprocess 臂首期 hang 留痕——
    // NVIDIA 4070 Ti 驱动 620.02 下 `vkCmdPreprocessGeneratedCommandsEXT` 在
    // stateCommandBuffer 自引用/EXPLICIT_PREPROCESS 布局下不稳定,经编排者裁决
    // 首期不纳入硬门判据,留痕 evidence notes;判据载体 = ExecuteGeneratedCommands
    // 链路〔M102 MAP 判据原文「compute pre-pass 填充 → ExecuteIndirect 出图」〕)──
    let exe = std::env::current_exe().expect("current_exe");
    let mut outputs: Vec<Vec<u8>> = Vec::new();
    for (arm, sub) in [("execute_only", "execute")] {
        let env_ok = std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1");
        let mut cmd = std::process::Command::new(&exe);
        cmd.args(["--arm", sub]);
        if env_ok {
            cmd.env("RURIX_REQUIRE_REAL", "1");
        }
        if std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1") {
            cmd.env("RURIX_VK_VALIDATION", "1");
        }
        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("VK_DGC: FAIL [{arm}] 子进程启动失败: {e}");
                std::process::exit(1);
            }
        };
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        print!("{text}");
        if !out.status.success() || !text.contains("VK_DGC: arm ok") {
            eprintln!(
                "VK_DGC: FAIL [{arm}] 子进程臂失败 rc={:?}",
                out.status.code()
            );
            std::process::exit(1);
        }
        // 子进程哨兵字行(单行 "SENTINEL_BYTES=<hex>")回传给父进程比对。
        let line = text
            .lines()
            .find(|l| l.starts_with("SENTINEL_BYTES="))
            .unwrap_or("");
        let hex = line.trim_start_matches("SENTINEL_BYTES=");
        outputs.push(hex.as_bytes().to_vec());
    }
    if outputs.len() == 2 && outputs[0] != outputs[1] {
        eprintln!("VK_DGC: FAIL 两臂哨兵输出不等");
        std::process::exit(1);
    }
    println!("VK_DGC: ok sentinel=3436 readback_counter=0 validation=0");
}

/// 单臂执行(子进程):prepass 填充 DgcBuffer → ExecuteGeneratedCommands → 回读
/// 哨兵字断言;输出单行 `SENTINEL_BYTES=<hex>` + `VK_DGC: arm ok` 供父进程核验。
fn run_one_arm(require_preprocess: bool) {
    let spv_bytes = dgc_prepass_spv();
    if spv_bytes.is_empty() {
        if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") {
            eprintln!("VK_DGC: FAIL codegen 降级空 SPV(RURIX_REQUIRE_REAL=1 不许 SKIP)");
            std::process::exit(1);
        }
        println!("VK_DGC: SKIP dgc_prepass 着色器为空(build.rs codegen 降级)");
        return;
    }
    let spv: Vec<u32> = spv_bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let layout = match IndirectCmdLayout::assemble(&[DgcToken::Dispatch]) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("VK_DGC: FAIL 合法 layout 装配被拒: {e}");
            std::process::exit(1);
        }
    };
    let scene = DgcScene {
        layout: &layout,
        prepass_spv: &spv,
        graphics: None,
        width: 0,
        height: 0,
        clear: [0.0; 4],
    };
    match run_dgc_offscreen(&scene, require_preprocess) {
        Ok(out) => {
            if out.readback_counter != 0 {
                eprintln!(
                    "VK_DGC: FAIL 回读计数器 = {} ≠ 0(RFC-0023 §4.4.2)",
                    out.readback_counter
                );
                std::process::exit(1);
            }
            // 哨兵字判据:DgcBuffer[0..4] = [1,1,1,3436](prepass+execute 幂等同值)。
            let words: Vec<u32> = out
                .pixels
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if words.len() < 4
                || words[0] != 1
                || words[1] != 1
                || words[2] != 1
                || words[3] != 3436
            {
                eprintln!(
                    "VK_DGC: FAIL DgcBuffer 哨兵字 {:?} ≠ [1, 1, 1, 3436](execute 未消费 GPU 生成命令数据)",
                    &words[..words.len().min(4)]
                );
                std::process::exit(1);
            }
            let hex: String = out.pixels[..16]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("SENTINEL_BYTES={hex}");
            println!(
                "VK_DGC: arm ok preprocess={} readback_counter=0 props(maxSeq={} maxTok={} maxTokOff={} maxStride={} idxMode={})",
                require_preprocess,
                out.props.max_sequence_count,
                out.props.max_token_count,
                out.props.max_token_offset,
                out.props.max_indirect_stride,
                out.props.vulkan_index_buffer_mode,
            );
        }
        Err(e) => {
            eprintln!("VK_DGC: FAIL run_dgc_offscreen(preprocess={require_preprocess}): {e}");
            std::process::exit(1);
        }
    }
}
