//! `execution_set` — G9.3 M106 Execution Set 与 PSO 衔接
//! (g9.p1.m106.execution_set_pso;spec/gpu_driven_submit.md RXS-0355;RFC-0023 §4.2)。
//!
//! 纯 host、safe(本模块零 `unsafe`)、always-on(不随 `vulkan` feature 门控):
//!
//! - **Execution Set 语义**(RXS-0355 L1;RFC-0023 §4.2.1 逐字):同一
//!   graphics/compute 状态、仅 shader 不同的管线数组,GPU 侧索引切换;材质变体
//!   为自然消费方(同一 pass 状态模板下按 material ID 索引切换 shader)。成员 =
//!   PSO cache 条目集合的子集视图,成员身份以 **pso_key**(RXS-0314 七段闭集)
//!   携带;cache key 的加性扩展「execution set 成员身份」字段(第八段,尾随
//!   加性 0-drift)单一事实源归 `pso_cache.rs`(`pso_key_with_membership`,
//!   feature `vulkan` 门控;本模块携带成员身份**类型**与 set 内容身份的规范
//!   字节,digest 压缩归 vulkan lane——always-on 面不复制 SHA-256,供应链
//!   纪律与 `pso_cache.rs` 依赖面 0-byte 不动)。
//! - **manifest 成员枚举**(RXS-0355 L1/L3):set 内容身份 = 规范字节流
//!   ([`ExecutionSet::canonical_bytes`];成员声明序 = GPU 侧索引);失效(设备
//!   丢失/显式重建)后重建对同输入确定——重建产物成员枚举与 manifest **逐位
//!   一致**;句柄物理值为实现确定、非 stable(RFC-0023 §4.0-5),本模块只作
//!   存在性/确定性声明。
//! - **capability 缺失 fail-closed**(RXS-0355 L2/L4;RFC-0023 §4.2.2 逐字):
//!   `submit.execution_set` capability ID(rurixc `capability_check` 闭集第
//!   十三项,RXS-0349 预留位 G9.3 转正,本模块**消费**不重定)区分两条路径;
//!   请求 GPU 侧索引切换而 profile/snapshot 不含该 capability → **显式不可
//!   表达诊断**(typed `Err`,不静默降级为模拟);D3D12 无 Execution Set 对应
//!   能力 → 诚实降级 CPU 侧 PSO 切换,降级路径**显式登记**「GPU 侧 shader
//!   索引切换不可表达」([`DegradationRegistration`],不静默模拟,P-01)。
//!
//! device 接线点(留 CI 门代理,`ci/g9_execution_set_pso_smoke.py`,symbolic key
//! `g9.p1.m106.execution_set_pso`):`VkIndirectExecutionSetEXT` FFI 面归 vk.rs
//! (U 号实现期按实测 `U.next_free` 顺位登记);本模块只消费 PSO key 材料与
//! capability 事实,不触 device。

use crate::dgc::DgcBackend;

// ═══════════════════════ capability 门控(RXS-0355 L2/L4) ═══════════════════════

/// `submit.execution_set` capability ID(RXS-0355 L2:RXS-0349 预留位转正;
/// 与 rurixc `capability_check::CapabilityId::SubmitExecutionSet` 同一字面——
/// 编译期闭集归 rurixc,本常量 = 该 ID 在 rt 装配/装载期的实测锚,镜像
/// dgc.rs `DGC_REQUIRED_EXTENSION` 体例)。
pub const EXECUTION_SET_CAPABILITY_ID: &str = "submit.execution_set";

/// D3D12 诚实降级登记事实字面(RXS-0355 L4 逐字:「GPU 侧 shader 索引切换不可
/// 表达」;显式登记,不静默模拟 P-01)。
pub const D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE: &str =
    "gpu_shader_index_switch_inexpressible(GPU 侧 shader 索引切换不可表达)";

/// Execution Set 路径选择请求(RXS-0355 L4:两路径由 capability ID 区分,
/// profile 选择律裁定 fallback)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExecutionSetRequest {
    /// 由 capability 事实裁定分流:具备 `submit.execution_set` → GPU 侧索引
    /// 切换;缺失 → 诚实降级 CPU 侧 PSO 切换(**显式登记**,非静默模拟)。
    Auto,
    /// 显式请求 GPU 侧索引切换:profile/snapshot 不含该 capability → 显式
    /// 不可表达诊断(typed `Err`,不静默降级为模拟)。
    RequireGpuIndexSwitch,
}

/// 诚实降级登记(RXS-0355 L4:降级路径必须显式登记「GPU 侧 shader 索引切换
/// 不可表达」;登记即产物,调用方须将其写入装配/装载证据面)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DegradationRegistration {
    /// 降级发生的后端(D3D12 无 Execution Set 对应能力)。
    pub backend: DgcBackend,
    /// 登记事实字面(= [`D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE`])。
    pub fact: &'static str,
}

/// Execution Set 路径(RXS-0355 L1/L4)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExecutionSetPath {
    /// GPU 侧索引切换(`VkIndirectExecutionSetEXT` 语义;capability 实测在位)。
    GpuIndexSwitch,
    /// 诚实降级:CPU 侧 PSO 切换再录 `ExecuteIndirect`(显式登记,不伪造 GPU
    /// 侧索引,不静默模拟)。
    CpuPsoSwitchDegraded(DegradationRegistration),
}

/// Execution Set 装配/路径选择失败的 typed `Err`(fail-closed;库面装配诊断
/// 沿 dgc.rs `DgcError` 先例,不占 RX 码)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExecSetError {
    /// 空成员集(Execution Set = 管线数组,空数组不可表达)。
    EmptyMemberSet,
    /// 成员重名(GPU 侧索引/材质 ID 索引歧义,fail-closed 不覆盖)。
    DuplicateMember {
        /// 重名成员名。
        name: String,
    },
    /// capability 缺失 fail-closed(RXS-0355 L4;RXS-0313 口径,禁静默模拟)。
    CapabilityMissing,
    /// 显式请求 GPU 侧索引切换而后端/profile 不可表达(显式不可表达诊断,
    /// 不静默降级为模拟;NVPTX 行「—」不承诺,RXS-0348 §3-5)。
    GpuIndexSwitchInexpressible {
        /// 不可表达的后端。
        backend: DgcBackend,
    },
}

impl std::fmt::Display for ExecSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecSetError::EmptyMemberSet => write!(
                f,
                "Execution Set 空成员集(管线数组至少一员;fail-closed,RXS-0355)"
            ),
            ExecSetError::DuplicateMember { name } => write!(
                f,
                "Execution Set 成员 `{name}` 重名(GPU 侧索引歧义;fail-closed,RXS-0355)"
            ),
            ExecSetError::CapabilityMissing => write!(
                f,
                "capability.runtime_snapshot_mismatch: required capabilities missing from \
                 device capability snapshot: [submit.execution_set] (fail-closed, \
                 RXS-0355 L4 / RXS-0313; 禁静默模拟 P-01)"
            ),
            ExecSetError::GpuIndexSwitchInexpressible { backend } => write!(
                f,
                "Execution Set GPU 侧索引切换在 {backend:?} 后端不可表达(显式不可表达诊断, \
                 不静默降级为模拟,RXS-0355 L4 / RXS-0348 §3-4)"
            ),
        }
    }
}

impl std::error::Error for ExecSetError {}

/// Execution Set 路径选择(RXS-0355 L4;profile 选择律 RXS-0312 在编译期裁定
/// 变体,本函数 = 装配/装载期的 capability 事实分流面,fail-closed):
///
/// - `Vulkan` + capability 实测在位 → [`ExecutionSetPath::GpuIndexSwitch`];
/// - `D3D12`(无对应能力)→ 诚实降级 [`ExecutionSetPath::CpuPsoSwitchDegraded`]
///   并**显式登记**(`Auto`);`RequireGpuIndexSwitch` → 显式不可表达诊断;
/// - `Nvptx` → 不可表达诊断(执行面「—」不承诺,RXS-0348 §3-5);
/// - capability 缺失而 `Auto` → [`ExecSetError::CapabilityMissing`](不静默模拟;
///   调用方须选择降级变体——编译期 fallback 已由 profile 选择律裁定)。
///
/// `available_capabilities` = 设备实测 capability 表(探测归 vk.rs lane;合成
/// 注入仅供 host 单测负臂,镜像 `dgc::verify_dgc_snapshot` 体例)。
//@ spec: RXS-0355
pub fn select_execution_set_path(
    backend: DgcBackend,
    request: ExecutionSetRequest,
    available_capabilities: &[&str],
) -> Result<ExecutionSetPath, ExecSetError> {
    match backend {
        DgcBackend::Vulkan => {
            if available_capabilities.contains(&EXECUTION_SET_CAPABILITY_ID) {
                Ok(ExecutionSetPath::GpuIndexSwitch)
            } else {
                // capability 缺失:Auto 与 Require 同判 fail-closed(不存在任何
                // 「尽力而为」的静默模拟路径;by construction)。
                Err(ExecSetError::CapabilityMissing)
            }
        }
        DgcBackend::D3D12 => match request {
            ExecutionSetRequest::Auto => Ok(ExecutionSetPath::CpuPsoSwitchDegraded(
                DegradationRegistration {
                    backend: DgcBackend::D3D12,
                    fact: D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE,
                },
            )),
            ExecutionSetRequest::RequireGpuIndexSwitch => {
                Err(ExecSetError::GpuIndexSwitchInexpressible {
                    backend: DgcBackend::D3D12,
                })
            }
        },
        DgcBackend::Nvptx => Err(ExecSetError::GpuIndexSwitchInexpressible {
            backend: DgcBackend::Nvptx,
        }),
    }
}

// ═══════════════════════ Execution Set 本体(RXS-0355 L1/L3) ═══════════════════════

/// Execution Set 成员身份(RXS-0355 L1:cache key 加性扩展「execution set 成员
/// 身份」字段的**载荷类型**单一事实源)。规范编码 = `set_identity` 32B +
/// `member_index` u32 LE(定界 by construction);经 `pso_cache::
/// pso_key_with_membership` 尾随进 preimage 第八段(vulkan lane)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExecutionSetMembership {
    /// set 内容身份 digest(`pso_cache::execution_set_identity` 产物,vulkan
    /// lane;内容身份 = [`ExecutionSet::canonical_bytes`] 的 SHA-256 域分离压缩)。
    pub set_identity: [u8; 32],
    /// 成员索引(声明序 = GPU 侧索引;材质变体按 material ID 索引切换)。
    pub member_index: u32,
}

impl ExecutionSetMembership {
    /// 成员身份规范编码(第八段尾随材料;32B digest + u32 LE 索引,定长定界)。
    #[must_use]
    pub fn canonical_encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(36);
        out.extend_from_slice(&self.set_identity);
        out.extend_from_slice(&self.member_index.to_le_bytes());
        out
    }
}

/// Execution Set 成员声明(输入面;`pso_key` = RXS-0314 七段闭集产物,PSO
/// cache 条目子集视图的键)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecutionSetMemberSpec {
    /// 成员名(manifest 记录面;材质变体的自然标识)。
    pub name: String,
    /// 成员 PSO key(同状态仅换 shader 的条目键;pso_cache `pso_key` 产物)。
    pub pso_key: [u8; 32],
}

/// Execution Set 声明(输入面):同一 graphics/compute 状态模板 + 仅 shader
/// 不同的成员表。`state_canonical` = 同状态模板的规范编码(调用方供给;
/// 固定功能状态面归 RXS-0314 段 6 同一编码律)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecutionSetSpec {
    /// set 诊断名(manifest 记录面)。
    pub name: String,
    /// 同状态模板规范编码(仅 shader 不同;状态事实由调用方规范编码供给)。
    pub state_canonical: Vec<u8>,
    /// 成员表(声明序 = GPU 侧索引)。
    pub members: Vec<ExecutionSetMemberSpec>,
}

/// Execution Set 成员(构建产物;索引 = 声明序)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecutionSetMember {
    /// 成员名。
    pub name: String,
    /// 成员 PSO key。
    pub pso_key: [u8; 32],
    /// GPU 侧索引(声明序;材质 ID 索引切换的消费锚)。
    pub index: u32,
}

/// Execution Set(构建产物;句柄物理值非 stable,内容身份 = 规范字节流)。
///
/// **失效与重建**(RXS-0355 L3):[`ExecutionSet::invalidate`] 标记句柄失效
/// (设备丢失/显式重建场景)并推进版本代;[`ExecutionSet::build`] 对同一
/// [`ExecutionSetSpec`] 重建 → 规范字节与成员枚举**逐位一致**(重建确定性;
/// 版本代是句柄世代,不进内容身份)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecutionSet {
    name: String,
    state_canonical: Vec<u8>,
    members: Vec<ExecutionSetMember>,
    /// set 内容身份规范字节(manifest 成员枚举单一事实源;digest 归 vulkan lane)。
    canonical: Vec<u8>,
    /// 句柄世代(失效重建语义;不进 canonical——内容身份与世代分离)。
    version: u64,
    /// 句柄有效性(失效 = 需重建;类型层不阻止读取内容身份,执行面消费前须
    /// 重建——vk.rs lane 纪律)。
    valid: bool,
}

impl ExecutionSet {
    /// 构建(RXS-0355 L1;装配期 fail-closed:空成员集/成员重名 → typed `Err`)。
    /// 成员索引 = 声明序(0..n,GPU 侧索引切换的消费锚)。
    ///
    /// # Errors
    /// [`ExecSetError::EmptyMemberSet`] / [`ExecSetError::DuplicateMember`]。
    //@ spec: RXS-0355
    pub fn build(spec: &ExecutionSetSpec) -> Result<Self, ExecSetError> {
        Self::build_at_version(spec, 0)
    }

    /// 失效重建(RXS-0355 L3):同输入 spec 重建 → canonical 与成员枚举逐位
    /// 一致;`version` = 新句柄世代(调用方自失效句柄 `version() + 1` 推进)。
    ///
    /// # Errors
    /// 同 [`ExecutionSet::build`]。
    //@ spec: RXS-0355
    pub fn rebuild(spec: &ExecutionSetSpec, version: u64) -> Result<Self, ExecSetError> {
        Self::build_at_version(spec, version)
    }

    fn build_at_version(spec: &ExecutionSetSpec, version: u64) -> Result<Self, ExecSetError> {
        if spec.members.is_empty() {
            return Err(ExecSetError::EmptyMemberSet);
        }
        let mut members: Vec<ExecutionSetMember> = Vec::with_capacity(spec.members.len());
        for (i, m) in spec.members.iter().enumerate() {
            if members
                .iter()
                .any(|x: &ExecutionSetMember| x.name == m.name)
            {
                return Err(ExecSetError::DuplicateMember {
                    name: m.name.clone(),
                });
            }
            members.push(ExecutionSetMember {
                name: m.name.clone(),
                pso_key: m.pso_key,
                index: u32::try_from(i).unwrap_or(u32::MAX),
            });
        }
        // 内容身份规范字节(单一事实源;CanonW 律:u32 LE / 字符串 length-prefix /
        // digest 原始 32 字节;声明序即成员序——GPU 索引语义的一部分,不重排)。
        let mut canonical = Vec::new();
        canonical.extend_from_slice(b"rurix.execution-set.v1\0");
        canonical.extend_from_slice(&(spec.name.len() as u32).to_le_bytes());
        canonical.extend_from_slice(spec.name.as_bytes());
        canonical.extend_from_slice(&(spec.state_canonical.len() as u32).to_le_bytes());
        canonical.extend_from_slice(&spec.state_canonical);
        canonical.extend_from_slice(&(members.len() as u32).to_le_bytes());
        for m in &members {
            canonical.extend_from_slice(&(m.name.len() as u32).to_le_bytes());
            canonical.extend_from_slice(m.name.as_bytes());
            canonical.extend_from_slice(&m.pso_key);
            canonical.extend_from_slice(&m.index.to_le_bytes());
        }
        Ok(ExecutionSet {
            name: spec.name.clone(),
            state_canonical: spec.state_canonical.clone(),
            members,
            canonical,
            version,
            valid: true,
        })
    }

    /// set 诊断名(只读)。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 同状态模板规范编码(只读)。
    #[must_use]
    pub fn state_canonical(&self) -> &[u8] {
        &self.state_canonical
    }

    /// 成员表(声明序 = GPU 侧索引;manifest 成员枚举面)。
    #[must_use]
    pub fn members(&self) -> &[ExecutionSetMember] {
        &self.members
    }

    /// set 内容身份规范字节(manifest 记录/失效重建逐位比对面;digest 压缩归
    /// `pso_cache::execution_set_identity`,vulkan lane)。
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }

    /// 句柄世代(失效重建语义面;不进内容身份)。
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// 句柄有效性(失效后执行面消费前须重建——vk.rs lane 纪律)。
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// 标记句柄失效(设备丢失/显式重建场景,RXS-0355 L3);世代推进由
    /// [`ExecutionSet::rebuild`] 完成(本调用不改内容身份)。
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

// ═══════════════════════ 单测(衔接/失效重建/fail-closed) ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> ExecutionSetSpec {
        ExecutionSetSpec {
            name: "deferred_opaque_set".to_owned(),
            state_canonical: vec![0xAA, 0xBB, 0, 0, 0, 0],
            members: vec![
                ExecutionSetMemberSpec {
                    name: "mat_lambert".to_owned(),
                    pso_key: [0x11; 32],
                },
                ExecutionSetMemberSpec {
                    name: "mat_pbr".to_owned(),
                    pso_key: [0x22; 32],
                },
                ExecutionSetMemberSpec {
                    name: "mat_emissive".to_owned(),
                    pso_key: [0x33; 32],
                },
            ],
        }
    }

    /// 衔接正确性(RXS-0355 L1):成员 = PSO cache 条目子集视图,成员索引 =
    /// 声明序(GPU 侧索引);manifest 成员枚举(规范字节)确定性且覆盖全部
    /// 成员 pso_key;成员身份规范编码 = 32B digest + u32 LE 索引。
    //@ spec: RXS-0355
    #[test]
    fn build_membership_and_manifest_enumeration() {
        let set = ExecutionSet::build(&sample_spec()).expect("合法 spec 构建");
        assert_eq!(set.members().len(), 3);
        for (i, m) in set.members().iter().enumerate() {
            assert_eq!(m.index, i as u32, "成员索引 = 声明序");
        }
        assert_eq!(set.members()[0].name, "mat_lambert");
        assert_eq!(set.members()[1].pso_key, [0x22; 32]);
        // 规范字节:域前缀起始 + 成员枚举逐位含全部 pso_key。
        let c = set.canonical_bytes();
        assert!(c.starts_with(b"rurix.execution-set.v1\0"));
        for key in [[0x11; 32], [0x22; 32], [0x33; 32]] {
            assert!(
                c.windows(32).any(|w| w == key),
                "manifest 规范字节须含成员 pso_key"
            );
        }
        // 成员身份编码定长定界(第八段尾随材料)。
        let membership = ExecutionSetMembership {
            set_identity: [0x42; 32],
            member_index: 2,
        };
        let enc = membership.canonical_encode();
        assert_eq!(enc.len(), 36);
        assert_eq!(&enc[..32], &[0x42; 32]);
        assert_eq!(&enc[32..], &2u32.to_le_bytes());
    }

    /// 失效重建确定性(RXS-0355 L3):同输入双构建逐位一致;失效标记不改内容
    /// 身份;重建(同 spec,世代 +1)→ 规范字节与成员枚举逐位一致、世代推进。
    //@ spec: RXS-0355
    #[test]
    fn invalidate_rebuild_deterministic() {
        let spec = sample_spec();
        let a = ExecutionSet::build(&spec).unwrap();
        let b = ExecutionSet::build(&spec).unwrap();
        assert_eq!(a, b, "同输入双构建逐位一致");
        let mut lost = a.clone();
        lost.invalidate();
        assert!(!lost.is_valid(), "失效标记生效");
        assert_eq!(
            lost.canonical_bytes(),
            a.canonical_bytes(),
            "失效不改内容身份"
        );
        let rebuilt = ExecutionSet::rebuild(&spec, lost.version() + 1).unwrap();
        assert!(rebuilt.is_valid());
        assert_eq!(rebuilt.version(), a.version() + 1, "世代推进");
        assert_eq!(
            rebuilt.canonical_bytes(),
            a.canonical_bytes(),
            "重建产物 manifest 逐位一致"
        );
        assert_eq!(rebuilt.members(), a.members(), "重建产物成员枚举逐位一致");
        // 输入扰动(成员表不同)→ 内容身份必变(区分力)。
        let mut spec2 = spec.clone();
        spec2.members[1].pso_key = [0x77; 32];
        let c = ExecutionSet::build(&spec2).unwrap();
        assert_ne!(c.canonical_bytes(), a.canonical_bytes());
    }

    /// 装配期 fail-closed(RXS-0355 L1):空成员集 / 成员重名 → typed `Err`
    /// (非 panic)。
    //@ spec: RXS-0355
    #[test]
    fn build_fail_closed_typed_err() {
        let mut spec = sample_spec();
        spec.members.clear();
        assert_eq!(
            ExecutionSet::build(&spec).expect_err("空成员集须拒"),
            ExecSetError::EmptyMemberSet
        );
        let mut spec = sample_spec();
        spec.members.push(ExecutionSetMemberSpec {
            name: "mat_pbr".to_owned(),
            pso_key: [0x99; 32],
        });
        let err = ExecutionSet::build(&spec).expect_err("重名成员须拒");
        assert_eq!(
            err,
            ExecSetError::DuplicateMember {
                name: "mat_pbr".to_owned()
            }
        );
        assert!(err.to_string().contains("mat_pbr"));
    }

    /// capability 缺失 fail-closed + D3D12 诚实降级(RXS-0355 L2/L4):
    /// - Vulkan + capability 实测在位 → GPU 侧索引切换;
    /// - Vulkan + 缺失(含空 snapshot)→ typed `Err`,诊断沿 RXS-0313 symbolic
    ///   key 且点名缺失 ID(禁静默模拟);
    /// - D3D12 + Auto → 诚实降级 CPU 侧 PSO 切换,**显式登记**「GPU 侧 shader
    ///   索引切换不可表达」;
    /// - D3D12 + RequireGpuIndexSwitch → 显式不可表达诊断(不静默降级为模拟);
    /// - NVPTX → 不可表达(执行面「—」不承诺,RXS-0348 §3-5)。
    //@ spec: RXS-0355
    #[test]
    fn capability_fail_closed_and_honest_degradation() {
        assert_eq!(
            EXECUTION_SET_CAPABILITY_ID, "submit.execution_set",
            "与 rurixc capability_check 闭集第十三项同一字面(RXS-0355 L2)"
        );
        // Vulkan 在位。
        assert_eq!(
            select_execution_set_path(
                DgcBackend::Vulkan,
                ExecutionSetRequest::RequireGpuIndexSwitch,
                &["submit.dgc", "submit.execution_set"],
            ),
            Ok(ExecutionSetPath::GpuIndexSwitch)
        );
        // Vulkan 缺失(Auto 与 Require 同判;空 snapshot 同判)。
        for req in [
            ExecutionSetRequest::Auto,
            ExecutionSetRequest::RequireGpuIndexSwitch,
        ] {
            let err = select_execution_set_path(DgcBackend::Vulkan, req, &["submit.dgc"])
                .expect_err("缺 capability 须拒");
            assert_eq!(err, ExecSetError::CapabilityMissing);
            let text = err.to_string();
            assert!(
                text.contains("capability.runtime_snapshot_mismatch"),
                "诊断须沿 RXS-0313 symbolic key: {text}"
            );
            assert!(
                text.contains("submit.execution_set"),
                "须点名缺失 ID: {text}"
            );
        }
        assert!(
            select_execution_set_path(DgcBackend::Vulkan, ExecutionSetRequest::Auto, &[]).is_err()
        );
        // D3D12 诚实降级:显式登记事实字面,不伪造 GPU 侧索引。
        let path = select_execution_set_path(
            DgcBackend::D3D12,
            ExecutionSetRequest::Auto,
            &["submit.execution_set"],
        )
        .expect("D3D12 Auto → 诚实降级");
        let ExecutionSetPath::CpuPsoSwitchDegraded(reg) = path else {
            panic!("D3D12 须走 CPU 侧 PSO 切换降级路,实得 {path:?}");
        };
        assert_eq!(reg.backend, DgcBackend::D3D12);
        assert_eq!(reg.fact, D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE);
        assert!(
            reg.fact.contains("不可表达"),
            "降级登记须显式声明不可表达事实: {}",
            reg.fact
        );
        // D3D12 显式请求 GPU 侧索引切换 → 显式不可表达诊断(不静默降级)。
        let err = select_execution_set_path(
            DgcBackend::D3D12,
            ExecutionSetRequest::RequireGpuIndexSwitch,
            &["submit.execution_set"],
        )
        .expect_err("D3D12 Require 须显式拒");
        assert_eq!(
            err,
            ExecSetError::GpuIndexSwitchInexpressible {
                backend: DgcBackend::D3D12
            }
        );
        assert!(err.to_string().contains("不可表达"));
        // NVPTX:执行面不承诺。
        assert!(matches!(
            select_execution_set_path(
                DgcBackend::Nvptx,
                ExecutionSetRequest::Auto,
                &["submit.execution_set"]
            ),
            Err(ExecSetError::GpuIndexSwitchInexpressible {
                backend: DgcBackend::Nvptx
            })
        ));
    }
}
