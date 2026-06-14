//! ptxas 干验证关卡红绿真跑(M4.3 / 契约 G-M4-4;spec/device.md RXS-0073)。
//!
//! 复用驱动同款关卡逻辑 [`rurixc::ptxas::dry_gate`](bin 的 `--emit=ptx` 经此关卡)。
//! **真红绿**(对齐真跑铁律,反 YAML-only):
//! - GREEN:合法 PTX 过 `ptxas -arch=sm_89`(`Pass`)。
//! - RED:注入非法 PTX → ptxas 拒绝(`Rejected`,上层映射 `RX6004`)。
//!
//! 无 CUDA 工具链(ptxas 缺失)→ 关卡 `Skipped`,测试降级 SKIP(开发环境;真红绿
//! 在带 CUDA 的 self-hosted runner / 装 Toolkit 的开发机,M4 CI_GATES §1/步骤 17)。

use rurixc::ptxas::{self, PtxasOutcome};

/// 最小合法 sm_89 PTX(空 kernel;ptxas 干验证基线)。
const VALID_PTX: &str = "\
.version 8.0
.target sm_89
.address_size 64

.visible .entry k()
{
    ret;
}
";

//@ spec: RXS-0073
#[test]
fn ptxas_dry_gate_accepts_valid_rejects_invalid() {
    // 探测:无 ptxas → SKIP(开发环境降级)
    if matches!(ptxas::dry_gate(VALID_PTX, "probe"), PtxasOutcome::Skipped) {
        eprintln!(
            "[ptxas_gate] SKIP:无 CUDA 工具链 ptxas(开发环境降级;真红绿在带 CUDA runner,G-M4-4)"
        );
        return;
    }

    // GREEN:合法 PTX 过 ptxas -arch=sm_89
    match ptxas::dry_gate(VALID_PTX, "valid") {
        PtxasOutcome::Pass => eprintln!("[ptxas_gate] GREEN:合法 PTX 过 ptxas -arch=sm_89"),
        other => panic!("合法 PTX 应过 ptxas 干验证,实得 {other:?}"),
    }

    // RED:注入非法 PTX → ptxas 拒绝(驱动 --emit=ptx 路径映射 RX6004,RXS-0073)
    let bad = format!(
        "{VALID_PTX}\n.visible .entry bad()\n{{\n    this_is_not_a_valid_opcode %r0;\n    ret;\n}}\n"
    );
    match ptxas::dry_gate(&bad, "bad") {
        PtxasOutcome::Rejected(reason) => {
            assert!(!reason.is_empty(), "ptxas 拒绝应携 stderr 摘要(RX6004 诊断输入)");
            eprintln!("[ptxas_gate] RED:ptxas 拒绝非法 PTX → RX6004 通道(摘要: {reason})");
        }
        other => panic!("非法 PTX 应被 ptxas -arch=sm_89 拒绝,实得 {other:?}"),
    }
}
