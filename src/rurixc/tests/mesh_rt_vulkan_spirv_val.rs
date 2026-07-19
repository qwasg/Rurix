//! G3.6 PR-Mc/Md(RFC-0013 §4.E5/E6 / 验收门 G-G3-6):mesh/task/RT 六执行模型
//! MIR→SPIR-V 编码 → `spirv-val --target-env vulkan1.2` / `spv1.4` accept。库级全链见证
//! (图形/mesh/RT emit 无 CLI `--emit` 通道,镜像 sampling_vulkan_spirv_val /
//! bindless_vulkan_spirv_val;device 像素判据〔程序化网格 / 三件套命中·miss 双色〕归
//! bin/vk_mesh + bin/vk_rt + 步骤 66/67 主循环 vk 运行时)。
//!
//! **per-entry SPIR-V 1.4 分叉**(§4.E6,Q-M-SpirvVersion):mesh/RT 入口 header 恒 1.4 +
//! interface 全量枚举;既有 compute/vertex/fragment 维持 1.0(字节零漂移,见
//! `dxil_spirv` / `vulkan_codegen` 既有 golden 单测)。
//!
//! spirv-val 三态:工具在位 → 每阶段 .spv accept(严格);缺工具 / 不可执行 → SKIP
//! (dev-env degrade,非 fake pass;退出码判定非 grep)。**device 真跑前唯一的 mesh/RT
//! SPIR-V 合法性机验闸门**。

#![cfg(feature = "vulkan-backend")]

use std::process::Command;

use rurixc::vulkan_codegen::{mesh_rt_witness_corpus, words_to_bytes};

/// SPIR-V 1.4 header 版本字(RFC-0013 §4.E6)。
const SPIRV_VERSION_1_4: u32 = 0x0001_0400;

/// 每阶段产非空 SPIR-V + magic 0x07230203 + header 版本 = 1.4(工具无关,恒跑;
/// per-entry 版本轴的机核锚点)。
#[test]
fn mesh_rt_corpus_emits_spirv_1_4() {
    for (name, words) in mesh_rt_witness_corpus() {
        assert!(words.len() >= 5, "{name} SPIR-V 过短({} 字)", words.len());
        assert_eq!(words[0], 0x0723_0203, "{name} SPIR-V magic 不符");
        assert_eq!(
            words[1], SPIRV_VERSION_1_4,
            "{name} 入口须 emit SPIR-V 1.4(per-entry 版本轴,§4.E6)"
        );
        let bytes = words_to_bytes(&words);
        assert_eq!(&bytes[0..4], &0x0723_0203u32.to_le_bytes());
    }
}

/// 每阶段 SPIR-V 过 `spirv-val --target-env vulkan1.2` **且** `spv1.4`(工具在位 accept /
/// 缺工具 SKIP 三态;退出码判定,反 grep stdout)。合规判定以 spirv-val 退出码为准,不以
/// 驱动宽容度为准(§4.E6 校验轴)。
#[test]
fn mesh_rt_corpus_pass_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    let target_envs = ["vulkan1.2", "spv1.4"];
    let mut validated = 0usize;
    for (name, words) in mesh_rt_witness_corpus() {
        let bytes = words_to_bytes(&words);
        let spv = std::env::temp_dir().join(format!("rurix_g36_{}_{name}.spv", std::process::id()));
        if std::fs::write(&spv, &bytes).is_err() {
            eprintln!("[SKIP] 写临时 .spv 失败(dev-env degrade)");
            return;
        }
        for env in target_envs {
            let out = Command::new(&tool)
                .arg("--target-env")
                .arg(env)
                .arg(&spv)
                .output();
            match out {
                // spawn 失败(工具不在 PATH)→ SKIP(PATH-defer 兜底,非 fake)。
                Err(_) => {
                    let _ = std::fs::remove_file(&spv);
                    eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                    return;
                }
                Ok(o) if o.status.success() => {
                    validated += 1;
                    eprintln!("[OK] spirv-val --target-env {env} accept: {name}");
                }
                Ok(o) => {
                    let _ = std::fs::remove_file(&spv);
                    panic!(
                        "spirv-val --target-env {env} 拒绝 {name}: stdout={} stderr={}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    );
                }
            }
        }
        let _ = std::fs::remove_file(&spv);
    }
    let corpus_len = mesh_rt_witness_corpus().len();
    assert_eq!(
        validated,
        corpus_len * target_envs.len(),
        "spirv-val 应对全部 {corpus_len} 阶段 × {} target-env accept",
        target_envs.len()
    );
}
