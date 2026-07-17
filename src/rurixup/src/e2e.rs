//! 端到端安装时长(measured)冷启动验收 evidence 字段面校验(spec/release.md
//! RXS-0219,EA1.2 / RFC-0012 §4.10;裁决 C 两段式)。
//!
//! [`validate_install_e2e`] 对 `evidence/ea1_install_e2e_<yyyymmdd>_<segment>.json`
//! 做**纯离线字段名存在性面校验**(host 可测,不触网、不触盘):必需字段名齐备
//! → `Ok`;缺任一必需字段名 → `Err` 且缺字段可枚举。schema 权威定义在
//! `milestones/ea1/install_e2e_evidence_schema.json`(JSON Schema Draft-7,字段面 +
//! 类型),e2e 实档随 D-EA1-5 冷启动取证(measured_local)经 `check_schemas` 路由
//! 校验;本模块只锚定 schema 字段名面 + 纯离线校验器(RXS-0219 测试锚定源)。
//!
//! 纪律:纯 safe(`unsafe_code=deny`),零外部依赖(不引 JSON schema 库,与 crate
//! 手写确定性 JSON 纪律一致);字段面存在性校验 = crate 既有手写 JSON 解析同族。

/// 冷启动 evidence 必需**顶层**字段名(RFC §4.10 schema 字段清单)。
pub const REQUIRED_TOP_FIELDS: [&str; 12] = [
    "segment",
    "host",
    "toolchain_version",
    "t_start",
    "t_end",
    "duration_s",
    "steps",
    "digest_levels_verified",
    "bytes_downloaded",
    "bandwidth_note",
    "attempt",
    "pass",
];

/// `host` 对象必需子字段名(环境画像)。
pub const REQUIRED_HOST_FIELDS: [&str; 4] = ["os", "cpu", "gpu", "driver"];

/// `steps[]` 元素必需子字段名(逐步计时)。
pub const REQUIRED_STEP_FIELDS: [&str; 4] = ["name", "cmd", "exit", "duration_s"];

/// 提取 JSON 文本中出现的全部键名(匹配 `"<ident>"` 紧跟 `:` 的 token)。纯离线、
/// 零依赖(与 `bundle.rs` / `channel.rs` 手写 line-scan 解析同族);字段名存在性面
/// 校验够用(schema 字段面,非全 JSON Schema 语义校验)。
fn collect_keys(json: &str) -> std::collections::BTreeSet<String> {
    let bytes = json.as_bytes();
    let mut keys = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // 读到闭合引号(不处理转义键名——evidence 键名均为 ASCII 标识符)。
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            let token = &json[start..j];
            // 跳过空白,判定其后是否为 `:`(= JSON 键)。
            let mut k = j + 1;
            while k < bytes.len()
                && (bytes[k] == b' ' || bytes[k] == b'\n' || bytes[k] == b'\t' || bytes[k] == b'\r')
            {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' {
                keys.insert(token.to_string());
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    keys
}

/// 冷启动 evidence 字段名存在性面校验(RXS-0219):合法样例必需字段名齐备 →
/// `Ok(())`;缺任一必需字段名(顶层 / host 子 / steps 元素子)→ `Err(缺字段枚举)`
/// (字典序,可枚举)。**纯离线、纯确定性**,不触网、不触盘。
pub fn validate_install_e2e(json: &str) -> Result<(), Vec<String>> {
    let keys = collect_keys(json);
    let mut missing: Vec<String> = REQUIRED_TOP_FIELDS
        .iter()
        .chain(REQUIRED_HOST_FIELDS.iter())
        .chain(REQUIRED_STEP_FIELDS.iter())
        .filter(|f| !keys.contains(**f))
        .map(|f| f.to_string())
        .collect();
    missing.sort();
    missing.dedup();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合法样例(字段齐备,含 host 子字段与 steps 元素子字段;RFC §4.10 schema 字段面)。
    fn legal_sample() -> String {
        r#"{
  "segment": "vm_rxcheck",
  "host": { "os": "Windows 11", "cpu": "VM 4vCPU", "gpu": "none", "driver": "n/a" },
  "toolchain_version": "1.1.0",
  "t_start": "2026-07-17T10:00:00+08:00",
  "t_end": "2026-07-17T10:07:12+08:00",
  "duration_s": 432,
  "steps": [
    { "name": "download_rurixup", "cmd": "curl ...", "exit": 0, "duration_s": 12 },
    { "name": "rx_check", "cmd": "rx check hello_kernel.rx", "exit": 0, "duration_s": 3 }
  ],
  "digest_levels_verified": 4,
  "bytes_downloaded": 15728640,
  "bandwidth_note": "~50 Mbps 家宽",
  "attempt": 1,
  "pass": true
}
"#
        .to_string()
    }

    //@ spec: RXS-0219
    // 合法样例(必需字段名齐备,含 host{os,cpu,gpu,driver} 与 steps[{name,cmd,exit,
    // duration_s}] 子字段)→ Ok。
    #[test]
    fn install_e2e_schema_fields_validate_legal_sample() {
        assert_eq!(validate_install_e2e(&legal_sample()), Ok(()));
    }

    //@ spec: RXS-0219
    // 缺任一必需字段名(顶层 bandwidth_note / host 子字段 gpu)→ Err + 缺字段枚举。
    #[test]
    fn install_e2e_schema_detects_missing_field() {
        // 缺顶层 bandwidth_note(唯一顶层字段)。
        let no_bw = legal_sample().replace("\"bandwidth_note\": \"~50 Mbps 家宽\",\n  ", "");
        match validate_install_e2e(&no_bw) {
            Err(missing) => assert_eq!(missing, vec!["bandwidth_note".to_string()]),
            Ok(()) => panic!("缺 bandwidth_note 未被检出(字段面校验失效)"),
        }

        // 缺 host 子字段 gpu(唯一 host 子字段)。
        let no_gpu = legal_sample().replace(", \"gpu\": \"none\"", "");
        match validate_install_e2e(&no_gpu) {
            Err(missing) => assert_eq!(missing, vec!["gpu".to_string()]),
            Ok(()) => panic!("缺 host.gpu 未被检出(字段面校验失效)"),
        }

        // 合法样例对称自检(防门过严把绿判红)。
        assert!(validate_install_e2e(&legal_sample()).is_ok());
    }
}
