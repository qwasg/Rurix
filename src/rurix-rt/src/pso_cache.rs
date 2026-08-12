//! G8.2 M30 PSO cache 主机面(RXS-0314~0316;RFC-0019 §4.1.4;门 g8.p0.m30.pso_cache)。
//!
//! 四个子面(设计案 G8.2_SHADER_PLATFORM_DESIGN §3.1 逐字):
//! 1. **pso_key 计算**(纯 host 可单测):`pso_key = SHA-256("rurix.pso-key.v1\0" || preimage)`,
//!    preimage 七段闭集规范编码(CanonW 律:u32 LE / 字符串 length-prefix / digest 原始 32
//!    字节)。**device identity 不入 key**(跨机 golden 可比对),单独构成 RXS-0315 cache
//!    artifact identity。SHA-256 复用 `rurix-pkg` 手写实现(RXS-0306 同源,零外部 crate)。
//! 2. **固定场景 collector**:冻结 workload 描述表 = 5 条 pipeline(三 compute〔saxpy 嵌入
//!    codegen 真产物 + fill/atomics 手编见证〕+ 一 graphics〔tri_vs/tri_fs 嵌入真产物〕+ 一
//!    rt〔meshrt raygen/miss/closesthit 嵌入见证〕),全部**确定性字节**;interface/profile/
//!    variant 三轴取 M31/M32/M29 既有空编码常量(语义 = fixture 无 reflection 轴真值,诚实
//!    且确定)。输出 JSON(RXS-0314 字段位 `{pso_key, kind_tag, stage_digests[],
//!    fixed_function_digest}`)+ golden `tests/pso/pso_keys.golden.json` checked-in。
//! 3. **持久化 store**(RXS-0315 逐字):单文件 `rurix_pso_cache.bin` = magic `"RXPSOC\x01"` +
//!    schema_version + device identity 段(pipelineCacheUUID[16]+vendorID+deviceID+
//!    driverVersion,实测自 VkPhysicalDeviceProperties)+ key-set digest + 分支 tag;装载核验
//!    序 ①magic/version → ②device identity 逐字段 →(分支 tag,keyset 前即拒)→ ③keyset
//!    digest,任一不符**丢弃全量重建**绝不部分命中,rebuild_reason 枚举 {schema, version,
//!    device_identity, keyset, none}。解析层纯 safe 长度前置校验(磁盘 blob = 不可信输入)。
//! 4. **双分支 manager + stall 计数器**:binary 分支(VK_KHR_pipeline_binary 在位**必走**,
//!    RXS-0316 强制律)/ VkPipelineCache 冻结 fallback;warm 全部 create 带
//!    FAIL_ON_PIPELINE_COMPILE_REQUIRED,COMPILE_REQUIRED 即 `runtime_compile_stalls += 1`;
//!    cold 记 `precache_build_count`。计数器 = [`PsoCacheManager`] 公开字段,进 evidence。
//!
//! device 会话(pipeline 创建/捕获/命中判定)在 `vk.rs` M30 FFI append 段(U27/U31 同一
//! vk FFI 边界扩注);本模块零 unsafe。

use std::path::{Path, PathBuf};

use rurix_pkg::sha256;

use crate::vk;

// ─────────────────────────── 冻结常量(RXS-0314) ───────────────────────────

/// pso_key 定义域(RXS-0314:`pso_key = SHA-256("rurix.pso-key.v1\0" || preimage)`)。
const PSO_KEY_DOMAIN: &[u8] = b"rurix.pso-key.v1\0";
/// interface_hash 轴空编码定义域(M31 `reflection.rs` IFACE_DOMAIN 同源串;fixture 无
/// reflection 轴真值 → 冻结常量 = 该定义域串裸 digest,镜像 PROFILE_NONE/EMPTY_DOMAIN
/// 两既有空编码「sha256(domain 串)」律)。
const IFACE_NONE_DOMAIN: &[u8] = b"rurix.shader-interface.v1\0";
/// `selected_profile_digest` 轴空编码定义域(M32 同源;无 profile 恒既有常量,RXS-0304)。
const PROFILE_NONE_DOMAIN: &[u8] = b"rurix.profile-none.v1\0";
/// permutation 空域 digest 定义域(M29 同源;collector 单测锚定其字面,M32 smoke 同一字面)。
#[cfg(test)]
const PERM_EMPTY_DOMAIN: &[u8] = b"rurix.permutation-domain-empty.v1\0";
/// `compiler`(RXS-0306 同源字串)。
const COMPILER: &str = "rurixc";
/// `compiler_version`(workspace 版本字串,与 rurixc `CARGO_PKG_VERSION` 同值,RXS-0306 同源)。
const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// `edition`(RXS-0306 同源;MVP 期唯一 edition Rx0)。
const EDITION: &str = "Rx0";
/// `target`(RXS-0306 同源;canonical 反射目标)。
const TARGET: &str = "vulkan";

/// pipeline 种类 tag(RXS-0314 段 1:0=compute / 1=graphics / 2=rt)。
pub const KIND_COMPUTE: u32 = 0;
/// graphics 种类 tag。
pub const KIND_GRAPHICS: u32 = 1;
/// rt 种类 tag。
pub const KIND_RT: u32 = 2;

/// stage tag(RXS-0290 ShaderStage 枚举声明序,与 rurixc `codegen::stage_tag` 同一字面)。
pub const STAGE_VERTEX: u32 = 0;
/// fragment stage tag。
pub const STAGE_FRAGMENT: u32 = 1;
/// compute stage tag。
pub const STAGE_COMPUTE: u32 = 2;
/// raygen stage tag。
pub const STAGE_RAYGEN: u32 = 5;
/// closesthit stage tag。
pub const STAGE_CLOSESTHIT: u32 = 6;
/// miss stage tag。
pub const STAGE_MISS: u32 = 8;

// VK 创建面值(vulkan_core.h / vk.rs 常量同源;创建计划用,不入 key——key 只含 stage_tag)。
const VK_STAGE_VERTEX_BIT: u32 = 0x1;
const VK_STAGE_FRAGMENT_BIT: u32 = 0x10;
const VK_STAGE_COMPUTE_BIT: u32 = 0x20;
const VK_STAGE_RAYGEN_BIT: u32 = 0x100;
const VK_STAGE_CLOSEST_HIT_BIT: u32 = 0x400;
const VK_STAGE_MISS_BIT: u32 = 0x800;
const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
const VK_FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
const VK_DESCRIPTOR_TYPE_STORAGE_IMAGE: u32 = 3;
const VK_DESCRIPTOR_TYPE_STORAGE_BUFFER: u32 = 7;
const VK_DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR: u32 = 1_000_150_000;
const VK_RT_GROUP_GENERAL: u32 = 0;
const VK_RT_GROUP_TRIANGLES_HIT_GROUP: u32 = 1;
const VK_SHADER_UNUSED: u32 = u32::MAX;

// ─────────────────────────── CanonW 规范编码(RXS-0305 律) ───────────────────────────

/// CanonW 规范编码写器(u32 LE / 字符串 length-prefix / digest 原始 32 字节;
/// 与 rurixc `reflection.rs` CanonW 同一编码律,M30 主机面零 unsafe 复刻)。
struct CanonW {
    buf: Vec<u8>,
}

impl CanonW {
    fn new() -> Self {
        CanonW { buf: Vec::new() }
    }
    fn u32v(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn strv(&mut self, s: &str) {
        self.u32v(u32::try_from(s.len()).unwrap_or(u32::MAX));
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
}

// ─────────────────────────── 空编码常量(M31/M32/M29 既有) ───────────────────────────

/// interface_hash 轴空编码常量(fixture 无 reflection 轴真值;每 stage 同值)。
fn iface_none_digest() -> [u8; 32] {
    sha256::digest(IFACE_NONE_DOMAIN)
}

/// `selected_profile_digest` 轴空编码常量(无 profile 恒此值;M32 smoke PROFILE_NONE_DIGEST
/// 同一字面,单测锚定)。
fn profile_none_digest() -> [u8; 32] {
    sha256::digest(PROFILE_NONE_DOMAIN)
}

// ─────────────────────────── pso_key(RXS-0314) ───────────────────────────

/// 单 stage 的 key 输入(stage_tag + artifact digest;按 stage_tag 升序)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageKeyInput {
    pub stage_tag: u32,
    pub artifact_digest: [u8; 32],
}

/// pso_key 计算输入(七段 preimage 闭集的材料;RXS-0314)。
pub struct PsoKeyInput<'a> {
    pub kind_tag: u32,
    pub stages: &'a [StageKeyInput],
    /// 段 6 固定功能状态规范编码(compute = u32 计数 0;graphics = attachment format/blend/
    /// depth 状态/顶点布局规范序列;rt = group 拓扑摘要)。
    pub fixed_function_canonical: &'a [u8],
}

/// pso_key preimage 七段闭集规范编码(RXS-0314;**单一事实源**——`pso_key` 与
/// RXS-0355 加性扩展 `pso_key_with_membership` 共用,域分离 + 定界编码 by
/// construction):① kind_tag u32;② 计数 + 逐 stage(stage_tag u32 + artifact
/// digest 32B,升序);③ 计数 + 逐 stage(stage_tag u32 + interface_hash 轴空编码
/// 常量 32B,同序);④ selected_profile_digest 轴空编码常量 32B;⑤ variant_key
/// 字符串(未选择恒空串);⑥ 固定功能状态规范编码;⑦ compiler/compiler_version/
/// edition/target(RXS-0306 同源字串)。
fn pso_preimage(input: &PsoKeyInput<'_>) -> Vec<u8> {
    let iface = iface_none_digest();
    let profile = profile_none_digest();
    let mut w = CanonW::new();
    w.u32v(input.kind_tag);
    w.u32v(input.stages.len() as u32);
    for s in input.stages {
        w.u32v(s.stage_tag);
        w.bytes(&s.artifact_digest);
    }
    w.u32v(input.stages.len() as u32);
    for s in input.stages {
        w.u32v(s.stage_tag);
        w.bytes(&iface);
    }
    w.bytes(&profile);
    w.strv(""); // variant_key:未选择恒空串(RXS-0309 既有空编码)
    w.bytes(input.fixed_function_canonical);
    w.strv(COMPILER);
    w.strv(COMPILER_VERSION);
    w.strv(EDITION);
    w.strv(TARGET);
    w.buf
}

//@ spec: RXS-0314
/// `pso_key = SHA-256("rurix.pso-key.v1\0" || preimage)`——纯 host 函数(无 GPU 依赖,
/// 可单测)。preimage 七段闭集见 [`pso_preimage`](编码律逐字注释在该函数)。
pub fn pso_key(input: &PsoKeyInput<'_>) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(PSO_KEY_DOMAIN);
    h.update(&pso_preimage(input));
    h.finalize()
}

/// key-set digest(RXS-0315 header 段:**全部 pso_key 按字节序排序拼接**的 SHA-256)。
pub fn keyset_digest(keys: &[[u8; 32]]) -> [u8; 32] {
    let mut sorted: Vec<[u8; 32]> = keys.to_vec();
    sorted.sort();
    let mut h = sha256::Sha256::new();
    for k in &sorted {
        h.update(k);
    }
    h.finalize()
}

// ─────────────────── RXS-0355(G9.3 M106)pso_key 加性第八段 ───────────────────

/// Execution Set 内容身份 digest 定义域(RXS-0355;set 规范字节的域分离压缩)。
const EXECUTION_SET_DOMAIN: &[u8] = b"rurix.execution-set.v1\0";

//@ spec: RXS-0355
/// Execution Set 内容身份 digest(manifest 成员枚举面;
/// `SHA-256("rurix.execution-set.v1\0" || set.canonical_bytes())`)。句柄物理值为
/// 实现确定、非 stable(RFC-0023 §4.0-5);本 digest = **内容身份**(失效重建逐位
/// 比对面,RXS-0355 L3)。always-on 的 `execution_set` 模块携规范字节,digest 压缩
/// 归本 lane(SHA-256 单一事实源 = rurix-pkg,零跨 crate 复制;default(CUDA-only)
/// 构建依赖面 0-byte 不动)。
pub fn execution_set_identity(set: &crate::execution_set::ExecutionSet) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(EXECUTION_SET_DOMAIN);
    h.update(set.canonical_bytes());
    h.finalize()
}

//@ spec: RXS-0355
/// Execution Set 成员身份装配(set 内容身份 digest + 成员索引;cache key 第八段
/// 加性扩展字段的载荷,类型单一事实源 = `crate::execution_set`)。
pub fn execution_set_membership(
    set: &crate::execution_set::ExecutionSet,
    member_index: u32,
) -> crate::execution_set::ExecutionSetMembership {
    crate::execution_set::ExecutionSetMembership {
        set_identity: execution_set_identity(set),
        member_index,
    }
}

//@ spec: RXS-0355
/// `pso_key` 七段闭集的**加性扩展**(RXS-0355 L1 逐字:「execution set 成员身份」
/// 字段;沿 RXS-0347 尾随可选字段 0-drift 先例):`membership = None` ≡ 既有
/// [`pso_key`] 逐字节(既有 golden `pso_keys.golden.json` 不动);`Some(m)` =
/// preimage **尾随第八段**(`m.canonical_encode()`,36B 定长定界——不得以
/// 「空编码为计数 0」冒充缺省)。
pub fn pso_key_with_membership(
    input: &PsoKeyInput<'_>,
    membership: Option<&crate::execution_set::ExecutionSetMembership>,
) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(PSO_KEY_DOMAIN);
    h.update(&pso_preimage(input));
    if let Some(m) = membership {
        h.update(&m.canonical_encode());
    }
    h.finalize()
}

// ─────────────────────────── 固定 fixture 集(collector) ───────────────────────────

/// 单着色阶段 fixture(stage_tag 升序;SPIR-V **确定性字节**)。
pub struct FixtureStage {
    pub stage_tag: u32,
    pub stage_bit: u32,
    pub spv_words: Vec<u32>,
    pub entry: String,
}

/// fixture 种类创建参数。
pub enum FixtureKind {
    Compute,
    Graphics {
        vertex_stride: u32,
        vertex_attrs: Vec<(u32, u32, u32)>,
        color_format: u32,
        extent: (u32, u32),
    },
    Rt {
        groups: Vec<vk::PsoRtGroupPlan>,
        max_recursion: u32,
    },
}

/// 冻结 workload 描述表的一条 pipeline(5 条;RXS-0314 collector 输入)。
pub struct PsoFixture {
    pub name: &'static str,
    pub kind_tag: u32,
    pub stages: Vec<FixtureStage>,
    pub bindings: Vec<vk::PsoBindingPlan>,
    pub push_constant_size: u32,
    pub kind: FixtureKind,
}

/// SPIR-V 字节 → u32 字流(小端;build.rs 嵌入 .spv 消费律,vk.rs 先例同律)。
fn spv_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// SPIR-V 字节 SHA-256(artifact digest,RXS-0314 段 2;原始字节非字流,定界无歧义)。
fn artifact_digest(bytes: &[u8]) -> [u8; 32] {
    sha256::digest(bytes)
}

/// compute SPIR-V 接口解析(纯 host;单一事实源 = 产物字节,镜像 vk.rs
/// `marshalling_ordinal_matches_codegen_binding` 测试的解析律):(SSBO binding 升序表,
/// push-constant 块字节数 = Block 结构成员 Offset 顺排末位 + 4;无 Block 块 = 0)。
/// 返回 `(bindings, push_constant_size)`。
fn parse_compute_iface(spv: &[u32]) -> (Vec<u32>, u32) {
    const OP_DECORATE: u16 = 71;
    const OP_MEMBER_DECORATE: u16 = 72;
    const DEC_BLOCK: u32 = 2;
    const DEC_BINDING: u32 = 33;
    const DEC_OFFSET: u32 = 35;
    let mut bindings: Vec<u32> = Vec::new();
    let mut block_structs: Vec<u32> = Vec::new();
    let mut member_offsets: Vec<(u32, u32, u32)> = Vec::new();
    let mut i = 5usize; // header 5 字后为指令流
    while i < spv.len() {
        let wc = (spv[i] >> 16) as usize;
        let op = (spv[i] & 0xffff) as u16;
        if wc == 0 {
            break;
        }
        let end = (i + wc).min(spv.len());
        let ops = &spv[i + 1..end];
        match op {
            OP_DECORATE if ops.len() >= 3 && ops[1] == DEC_BINDING => bindings.push(ops[2]),
            OP_DECORATE if ops.len() >= 2 && ops[1] == DEC_BLOCK => block_structs.push(ops[0]),
            OP_MEMBER_DECORATE if ops.len() >= 4 && ops[2] == DEC_OFFSET => {
                member_offsets.push((ops[0], ops[1], ops[3]));
            }
            _ => {}
        }
        i += wc;
    }
    bindings.sort_unstable();
    bindings.dedup();
    let pc_size = if let Some(&pc) = block_structs.first() {
        let max_off = member_offsets
            .iter()
            .filter(|(s, _, _)| *s == pc)
            .map(|(_, _, off)| *off)
            .max()
            .unwrap_or(0);
        max_off + 4 // 标量顺排 4 字节(RXS-0208 marshalling 布局;单测锚 saxpy)
    } else {
        0
    };
    (bindings, pc_size)
}

/// 手编 compute 见证 `vk_fill`(`out[gid.x] = 42`;SSBO set0/binding0,无 push constants;
/// SPIR-V 1.0 BufferBlock 旧式 SSBO——与 codegen 产物同风,本机驱动/validation 实测)。
/// 约 25 指令,结构自检见单测 `hand_written_spv_wellformed`。
//@ spec: RXS-0314
pub fn fill_spv() -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    // header: magic / version 1.0 / generator 0 / bound 20 / schema 0。
    let mut v = vec![0x0723_0203, 0x0001_0000, 0, 20, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    inst(&mut v, 15, &[5, 15, 0x6E69_616D, 0, 10]); // OpEntryPoint GLCompute %15 "main" %10
    inst(&mut v, 16, &[15, 17, 64, 1, 1]); // OpExecutionMode %15 LocalSize 64 1 1
    inst(&mut v, 71, &[10, 11, 28]); // OpDecorate %10 BuiltIn GlobalInvocationId
    inst(&mut v, 71, &[5, 3]); // OpDecorate %5 BufferBlock(SSBO 旧式)
    inst(&mut v, 71, &[7, 34, 0]); // OpDecorate %7 DescriptorSet 0
    inst(&mut v, 71, &[7, 33, 0]); // OpDecorate %7 Binding 0
    inst(&mut v, 72, &[5, 0, 35, 0]); // OpMemberDecorate %5 0 Offset 0
    inst(&mut v, 71, &[4, 6, 4]); // OpDecorate %4 ArrayStride 4(Decoration 6)
    inst(&mut v, 19, &[1]); // %1 = OpTypeVoid
    inst(&mut v, 33, &[2, 1]); // %2 = OpTypeFunction %1
    inst(&mut v, 21, &[3, 32, 0]); // %3 = OpTypeInt 32 0(uint)
    inst(&mut v, 29, &[4, 3]); // %4 = OpTypeRuntimeArray %3
    inst(&mut v, 30, &[5, 4]); // %5 = OpTypeStruct %4
    inst(&mut v, 32, &[6, 2, 5]); // %6 = OpTypePointer Uniform %5
    inst(&mut v, 59, &[6, 7, 2]); // %7 = OpVariable %6 Uniform
    inst(&mut v, 23, &[8, 3, 3]); // %8 = OpTypeVector %3 3
    inst(&mut v, 32, &[9, 1, 8]); // %9 = OpTypePointer Input %8
    inst(&mut v, 59, &[9, 10, 1]); // %10 = OpVariable %9 Input(gid)
    inst(&mut v, 32, &[11, 2, 3]); // %11 = OpTypePointer Uniform %3
    inst(&mut v, 32, &[12, 1, 3]); // %12 = OpTypePointer Input %3
    inst(&mut v, 43, &[3, 13, 0]); // %13 = OpConstant %3 0
    inst(&mut v, 43, &[3, 14, 42]); // %14 = OpConstant %3 42
    inst(&mut v, 54, &[1, 15, 0, 2]); // %15 = OpFunction %1 None %2
    inst(&mut v, 248, &[16]); // %16 = OpLabel
    inst(&mut v, 65, &[12, 17, 10, 13]); // %17 = OpAccessChain %12 %10 %13(gid.x)
    inst(&mut v, 61, &[3, 18, 17]); // %18 = OpLoad %3 %17
    inst(&mut v, 65, &[11, 19, 7, 13, 18]); // %19 = OpAccessChain %11 %7 %13 %18
    inst(&mut v, 62, &[19, 14]); // OpStore %19 %14
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// 手编 compute 见证 `vk_atomics_w1`(`OpAtomicIAdd(&out[0], 1)`;SSBO set0/binding0,
/// Device scope + Relaxed semantics——32 位整型 storage buffer 原子 core 无 feature 门槛)。
//@ spec: RXS-0314
pub fn atomics_spv() -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    // header: magic / version 1.0 / generator 0 / bound 21 / schema 0。
    let mut v = vec![0x0723_0203, 0x0001_0000, 0, 21, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    inst(&mut v, 15, &[5, 15, 0x6E69_616D, 0]); // OpEntryPoint GLCompute %15 "main"
    inst(&mut v, 16, &[15, 17, 64, 1, 1]); // OpExecutionMode %15 LocalSize 64 1 1
    inst(&mut v, 71, &[5, 3]); // OpDecorate %5 BufferBlock
    inst(&mut v, 71, &[7, 34, 0]); // OpDecorate %7 DescriptorSet 0
    inst(&mut v, 71, &[7, 33, 0]); // OpDecorate %7 Binding 0
    inst(&mut v, 72, &[5, 0, 35, 0]); // OpMemberDecorate %5 0 Offset 0
    inst(&mut v, 71, &[4, 6, 4]); // OpDecorate %4 ArrayStride 4(Decoration 6)
    inst(&mut v, 19, &[1]); // %1 = OpTypeVoid
    inst(&mut v, 33, &[2, 1]); // %2 = OpTypeFunction %1
    inst(&mut v, 21, &[3, 32, 0]); // %3 = OpTypeInt 32 0(uint)
    inst(&mut v, 29, &[4, 3]); // %4 = OpTypeRuntimeArray %3
    inst(&mut v, 30, &[5, 4]); // %5 = OpTypeStruct %4
    inst(&mut v, 32, &[6, 2, 5]); // %6 = OpTypePointer Uniform %5
    inst(&mut v, 59, &[6, 7, 2]); // %7 = OpVariable %6 Uniform
    inst(&mut v, 32, &[11, 2, 3]); // %11 = OpTypePointer Uniform %3
    inst(&mut v, 43, &[3, 13, 0]); // %13 = OpConstant %3 0(member/元素下标)
    inst(&mut v, 43, &[3, 14, 1]); // %14 = OpConstant %3 1(scope Device / 加数)
    inst(&mut v, 43, &[3, 16, 0]); // %16 = OpConstant %3 0(semantics Relaxed)
    inst(&mut v, 54, &[1, 15, 0, 2]); // %15 = OpFunction %1 None %2
    inst(&mut v, 248, &[17]); // %17 = OpLabel
    inst(&mut v, 65, &[11, 18, 7, 13, 13]); // %18 = OpAccessChain %11 %7 %13 %13(arr[0])
    inst(&mut v, 234, &[3, 19, 18, 14, 16, 14]); // %19 = OpAtomicIAdd %3 %18 %14 %16 %14
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// 嵌入 codegen 真产物(build.rs 经 vulkan_codegen 产;确定性字节,跨 clean build 逐字节相等)。
const SAXPY_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saxpy.spv"));
const TRI_VS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tri_vs.spv"));
const TRI_FS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tri_fs.spv"));
const RT_RAYGEN_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/meshrt_raygen.spv"));
const RT_MISS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/meshrt_miss.spv"));
const RT_CLOSESTHIT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/meshrt_closesthit.spv"));

/// 冻结 workload 描述表(RXS-0314 collector 输入;5 条 pipeline)。
/// 任一嵌入语料为空(build 降级)→ 确定性 `Err`(fail-closed,不伪造 key)。
//@ spec: RXS-0314
pub fn pso_fixtures() -> Result<Vec<PsoFixture>, String> {
    for (name, spv) in [
        ("saxpy.spv", SAXPY_SPV),
        ("tri_vs.spv", TRI_VS_SPV),
        ("tri_fs.spv", TRI_FS_SPV),
        ("meshrt_raygen.spv", RT_RAYGEN_SPV),
        ("meshrt_miss.spv", RT_MISS_SPV),
        ("meshrt_closesthit.spv", RT_CLOSESTHIT_SPV),
    ] {
        if spv.is_empty() {
            return Err(format!(
                "嵌入 SPIR-V 语料 {name} 为空(build.rs codegen 降级;fail-closed,不伪造 pso_key)"
            ));
        }
    }
    let mut fixtures: Vec<PsoFixture> = Vec::new();

    // ── compute ×3(saxpy 嵌入真产物 + fill/atomics 手编见证)──
    let compute_fixtures: [(&'static str, &[u8], Vec<u32>); 3] = [
        ("vk_saxpy", SAXPY_SPV, spv_words(SAXPY_SPV)),
        ("vk_fill", &[], fill_spv()),
        ("vk_atomics_w1", &[], atomics_spv()),
    ];
    for (name, embedded, words) in &compute_fixtures {
        let entry =
            vk::entry_point_name(words).ok_or_else(|| format!("{name} SPIR-V 无 OpEntryPoint"))?;
        let (bindings, pc_size) = parse_compute_iface(words);
        let plans = bindings
            .iter()
            .map(|&b| vk::PsoBindingPlan {
                set: 0,
                binding: b,
                descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                stage_flags: VK_STAGE_COMPUTE_BIT,
            })
            .collect();
        let _ = embedded;
        fixtures.push(PsoFixture {
            name,
            kind_tag: KIND_COMPUTE,
            stages: vec![FixtureStage {
                stage_tag: STAGE_COMPUTE,
                stage_bit: VK_STAGE_COMPUTE_BIT,
                spv_words: words.clone(),
                entry,
            }],
            bindings: plans,
            push_constant_size: pc_size,
            kind: FixtureKind::Compute,
        });
    }

    // ── graphics ×1(tri_vs + tri_fs 嵌入真产物;2×vec4 顶点输入 stride 32)──
    let vs_words = spv_words(TRI_VS_SPV);
    let fs_words = spv_words(TRI_FS_SPV);
    let vs_entry =
        vk::entry_point_name(&vs_words).ok_or_else(|| "tri_vs 无 OpEntryPoint".to_owned())?;
    let fs_entry =
        vk::entry_point_name(&fs_words).ok_or_else(|| "tri_fs 无 OpEntryPoint".to_owned())?;
    fixtures.push(PsoFixture {
        name: "vk_tri",
        kind_tag: KIND_GRAPHICS,
        stages: vec![
            FixtureStage {
                stage_tag: STAGE_VERTEX,
                stage_bit: VK_STAGE_VERTEX_BIT,
                spv_words: vs_words,
                entry: vs_entry,
            },
            FixtureStage {
                stage_tag: STAGE_FRAGMENT,
                stage_bit: VK_STAGE_FRAGMENT_BIT,
                spv_words: fs_words,
                entry: fs_entry,
            },
        ],
        bindings: Vec::new(),
        push_constant_size: 0,
        kind: FixtureKind::Graphics {
            vertex_stride: 32,
            vertex_attrs: vec![
                (0, VK_FORMAT_R32G32B32A32_SFLOAT, 0),
                (1, VK_FORMAT_R32G32B32A32_SFLOAT, 16),
            ],
            color_format: VK_FORMAT_R8G8B8A8_UNORM,
            extent: (64, 64),
        },
    });

    // ── rt ×1(meshrt raygen/miss/closesthit 嵌入见证;3 group 拓扑 + set0 AS / set1
    //    storage image,镜像 run_ray_tracing_offscreen 描述符布局)──
    let rg_words = spv_words(RT_RAYGEN_SPV);
    let ms_words = spv_words(RT_MISS_SPV);
    let ch_words = spv_words(RT_CLOSESTHIT_SPV);
    for (n, w) in [
        ("raygen", &rg_words),
        ("miss", &ms_words),
        ("closesthit", &ch_words),
    ] {
        if vk::entry_point_name(w).is_none() {
            return Err(format!("meshrt_{n} 无 OpEntryPoint"));
        }
    }
    fixtures.push(PsoFixture {
        name: "vk_rt_min",
        kind_tag: KIND_RT,
        stages: vec![
            FixtureStage {
                stage_tag: STAGE_RAYGEN,
                stage_bit: VK_STAGE_RAYGEN_BIT,
                spv_words: rg_words,
                entry: "main".to_owned(),
            },
            FixtureStage {
                stage_tag: STAGE_CLOSESTHIT,
                stage_bit: VK_STAGE_CLOSEST_HIT_BIT,
                spv_words: ch_words,
                entry: "main".to_owned(),
            },
            FixtureStage {
                stage_tag: STAGE_MISS,
                stage_bit: VK_STAGE_MISS_BIT,
                spv_words: ms_words,
                entry: "main".to_owned(),
            },
        ],
        bindings: vec![
            vk::PsoBindingPlan {
                set: 0,
                binding: 0,
                descriptor_type: VK_DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
                stage_flags: VK_STAGE_RAYGEN_BIT,
            },
            vk::PsoBindingPlan {
                set: 1,
                binding: 0,
                descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_IMAGE,
                stage_flags: VK_STAGE_RAYGEN_BIT,
            },
        ],
        push_constant_size: 0,
        kind: FixtureKind::Rt {
            groups: vec![
                vk::PsoRtGroupPlan {
                    group_type: VK_RT_GROUP_GENERAL,
                    general_shader: 0,
                    closest_hit_shader: VK_SHADER_UNUSED,
                },
                vk::PsoRtGroupPlan {
                    group_type: VK_RT_GROUP_GENERAL,
                    general_shader: 2,
                    closest_hit_shader: VK_SHADER_UNUSED,
                },
                vk::PsoRtGroupPlan {
                    group_type: VK_RT_GROUP_TRIANGLES_HIT_GROUP,
                    general_shader: VK_SHADER_UNUSED,
                    closest_hit_shader: 1,
                },
            ],
            max_recursion: 1,
        },
    });
    Ok(fixtures)
}

/// fixture → device 会话创建计划(借用在位 fixture;P-11 单一事实源,逐字重放)。
pub fn fixture_plan<'a>(fixture: &'a PsoFixture) -> vk::PsoPipelinePlan<'a> {
    let stages: Vec<vk::PsoStagePlan<'a>> = fixture
        .stages
        .iter()
        .map(|s| vk::PsoStagePlan {
            stage_bit: s.stage_bit,
            spv: &s.spv_words,
            entry: &s.entry,
        })
        .collect();
    // 注意:PsoPipelinePlan 持 stages 切片——经 Box 泄漏免除自引用(fixture 生命期 ≥ run)。
    let stages: &'a [vk::PsoStagePlan<'a>] = Box::leak(stages.into_boxed_slice());
    let kind = match &fixture.kind {
        FixtureKind::Compute => vk::PsoPlanKind::Compute,
        FixtureKind::Graphics {
            vertex_stride,
            vertex_attrs,
            color_format,
            extent,
        } => vk::PsoPlanKind::Graphics {
            vertex_stride: *vertex_stride,
            vertex_attrs,
            color_format: *color_format,
            extent: *extent,
        },
        FixtureKind::Rt {
            groups,
            max_recursion,
        } => vk::PsoPlanKind::Rt {
            groups,
            max_recursion: *max_recursion,
        },
    };
    vk::PsoPipelinePlan {
        name: fixture.name,
        kind_tag: fixture.kind_tag,
        stages,
        bindings: &fixture.bindings,
        push_constant_size: fixture.push_constant_size,
        kind,
    }
}

/// 段 6 固定功能状态规范编码(RXS-0314:graphics = attachment format/blend/depth 状态/
/// 顶点布局规范序列;compute = 空〔计数 0〕;rt = group 拓扑摘要〔group 数 + 逐 group
/// kind tag 序列〕)。编码 = u32 计数前缀 + u32 字流(CanonW 律)。
pub fn fixed_function_canonical(fixture: &PsoFixture) -> Vec<u8> {
    let mut w = CanonW::new();
    match &fixture.kind {
        FixtureKind::Compute => {
            w.u32v(0);
        }
        FixtureKind::Graphics {
            vertex_stride,
            vertex_attrs,
            color_format,
            ..
        } => {
            // 字流:[attachment format, blend_enable, depth_test, depth_write, depth_format,
            //       binding 数, stride, input_rate, attr 数, 逐 attr(location, format, offset)]
            let words: &[u32] = &[
                *color_format,
                0, // blend_enable = 0(冻结 fixture 关混合)
                0, // depth_test_enable = 0
                0, // depth_write_enable = 0
                0, // depth_format = VK_FORMAT_UNDEFINED
                1, // vertex binding 数
                *vertex_stride,
                0, // input_rate = VERTEX
                vertex_attrs.len() as u32,
            ];
            w.u32v(words.len() as u32 + 3 * vertex_attrs.len() as u32);
            for &x in words {
                w.u32v(x);
            }
            for &(loc, fmt, off) in vertex_attrs {
                w.u32v(loc);
                w.u32v(fmt);
                w.u32v(off);
            }
        }
        FixtureKind::Rt { groups, .. } => {
            w.u32v(groups.len() as u32);
            for g in groups {
                w.u32v(g.group_type);
            }
        }
    }
    w.buf
}

/// collector 单条记录(RXS-0314 字段位:`{pso_key, kind_tag, stage_digests[],
/// fixed_function_digest}`;name 供人读,golden 一并冻结)。
pub struct CollectorRecord {
    pub name: String,
    pub pso_key: [u8; 32],
    pub kind_tag: u32,
    pub stage_digests: Vec<StageKeyInput>,
    pub fixed_function_digest: [u8; 32],
}

/// collector:冻结 fixture 集 → key 集合(纯 host;确定性,跨 clean build 逐字节相等)。
pub fn collect_records(fixtures: &[PsoFixture]) -> Vec<CollectorRecord> {
    fixtures
        .iter()
        .map(|f| {
            let stage_digests: Vec<StageKeyInput> = f
                .stages
                .iter()
                .map(|s| StageKeyInput {
                    stage_tag: s.stage_tag,
                    artifact_digest: artifact_digest(words_to_bytes(&s.spv_words).as_slice()),
                })
                .collect();
            let ff = fixed_function_canonical(f);
            let key = pso_key(&PsoKeyInput {
                kind_tag: f.kind_tag,
                stages: &stage_digests,
                fixed_function_canonical: &ff,
            });
            CollectorRecord {
                name: f.name.to_owned(),
                pso_key: key,
                kind_tag: f.kind_tag,
                stage_digests,
                fixed_function_digest: sha256::digest(&ff),
            }
        })
        .collect()
}

/// u32 字流 → 小端字节(artifact digest 与嵌入 .spv 原始字节同一材料:手编见证无文件形,
/// 经同一 `words_to_bytes` 律字节化——与 rurixc `vulkan_codegen::words_to_bytes` 同律)。
fn words_to_bytes(words: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(words.len() * 4);
    for w in words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

fn hex_of(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

//@ spec: RXS-0314
/// collector JSON(RXS-0314 字段位 + keyset_digest;`--collector-only` stdout 与
/// golden `tests/pso/pso_keys.golden.json` 同源,双跑逐字节相等 = key_generation_deterministic)。
pub fn collector_json(records: &[CollectorRecord]) -> String {
    let keys: Vec<[u8; 32]> = records.iter().map(|r| r.pso_key).collect();
    let ksd = keyset_digest(&keys);
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"schema\": \"rurix.pso-keys.v1\",\n");
    s.push_str(&format!("  \"compiler\": \"{COMPILER}\",\n"));
    s.push_str(&format!(
        "  \"compiler_version\": \"{COMPILER_VERSION}\",\n"
    ));
    s.push_str(&format!("  \"edition\": \"{EDITION}\",\n"));
    s.push_str(&format!("  \"target\": \"{TARGET}\",\n"));
    s.push_str(&format!("  \"keyset_digest\": \"{}\",\n", hex_of(&ksd)));
    s.push_str("  \"records\": [\n");
    for (i, r) in records.iter().enumerate() {
        s.push_str("    {\n");
        s.push_str(&format!("      \"name\": \"{}\",\n", r.name));
        s.push_str(&format!("      \"pso_key\": \"{}\",\n", hex_of(&r.pso_key)));
        s.push_str(&format!("      \"kind_tag\": {},\n", r.kind_tag));
        s.push_str("      \"stage_digests\": [\n");
        for (j, sd) in r.stage_digests.iter().enumerate() {
            s.push_str(&format!(
                "        {{ \"stage_tag\": {}, \"digest\": \"{}\" }}{}\n",
                sd.stage_tag,
                hex_of(&sd.artifact_digest),
                if j + 1 == r.stage_digests.len() {
                    ""
                } else {
                    ","
                }
            ));
        }
        s.push_str("      ],\n");
        s.push_str(&format!(
            "      \"fixed_function_digest\": \"{}\"\n",
            hex_of(&r.fixed_function_digest)
        ));
        s.push_str(&format!(
            "    }}{}\n",
            if i + 1 == records.len() { "" } else { "," }
        ));
    }
    s.push_str("  ]\n}\n");
    s
}

// ─────────────────────────── store(RXS-0315) ───────────────────────────

/// store 单文件名(RXS-0315:`<dir>/rurix_pso_cache.bin`)。
pub const STORE_FILE_NAME: &str = "rurix_pso_cache.bin";
/// store magic(7 字节 `"RXPSOC\x01"`;RXS-0315 header 首段)。
pub const STORE_MAGIC: &[u8; 7] = b"RXPSOC\x01";
/// store schema_version(u32 LE;演进升版,fail-closed 核验序①)。
pub const STORE_SCHEMA_VERSION: u32 = 1;
/// header 定长 = magic7 + version4 + uuid16 + vendor4 + device4 + driver4 + keyset32 + branch4。
const STORE_HEADER_LEN: usize = 7 + 4 + 16 + 4 + 4 + 4 + 32 + 4;

/// rebuild_reason 枚举(RXS-0315 装载核验任一不符即重建;`none` = 干净命中无重建)。
/// 注:分支 tag 不符(RXS-0316「keyset 前即拒」)与 payload 解析失败(截断/越界 = 格式层
/// 违例)归 `schema`——payload 模式/格式与协商分支/格式规范不符,schema 类失配。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RebuildReason {
    Schema,
    Version,
    DeviceIdentity,
    Keyset,
    None,
}

impl RebuildReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RebuildReason::Schema => "schema",
            RebuildReason::Version => "version",
            RebuildReason::DeviceIdentity => "device_identity",
            RebuildReason::Keyset => "keyset",
            RebuildReason::None => "none",
        }
    }
}

/// store payload(RXS-0315:binary = per-pso_key blob 表〔key 32B + u32 LE 长度前缀 blob,
/// 按 key 字节序〕;cache = vkGetPipelineCacheData 整 blob〔u32 LE 长度前缀〕)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PsoStorePayload {
    Binary(Vec<([u8; 32], Vec<u8>)>),
    Cache(Vec<u8>),
}

/// device identity 段(RXS-0315;全部实测自 VkPhysicalDeviceProperties,禁手写)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StoreDeviceIdentity {
    pub pipeline_cache_uuid: [u8; 16],
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_version: u32,
}

/// 解码后的 store(header 三段 + payload;纯 safe 解析产物)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PsoStore {
    pub identity: StoreDeviceIdentity,
    pub keyset_digest: [u8; 32],
    pub branch: u32,
    pub payload: PsoStorePayload,
}

//@ spec: RXS-0315
/// store 编码(RXS-0315 格式逐字;binary payload 按 pso_key 字节序排序)。
pub fn encode_store(store: &PsoStore) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(STORE_MAGIC);
    out.extend_from_slice(&STORE_SCHEMA_VERSION.to_le_bytes());
    out.extend_from_slice(&store.identity.pipeline_cache_uuid);
    out.extend_from_slice(&store.identity.vendor_id.to_le_bytes());
    out.extend_from_slice(&store.identity.device_id.to_le_bytes());
    out.extend_from_slice(&store.identity.driver_version.to_le_bytes());
    out.extend_from_slice(&store.keyset_digest);
    out.extend_from_slice(&store.branch.to_le_bytes());
    match &store.payload {
        PsoStorePayload::Binary(entries) => {
            let mut sorted: Vec<([u8; 32], Vec<u8>)> = entries.clone();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, blob) in &sorted {
                out.extend_from_slice(key);
                out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                out.extend_from_slice(blob);
            }
        }
        PsoStorePayload::Cache(blob) => {
            out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
            out.extend_from_slice(blob);
        }
    }
    out
}

/// 纯 safe 读取器(长度前置校验;越界/截断 = 确定性 `Err`,磁盘 blob 视作不可信输入)。
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self
            .pos
            .checked_add(n)
            .is_none_or(|end| end > self.buf.len())
        {
            return Err("store 截断/越界(长度前置校验 fail-closed)".into());
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

//@ spec: RXS-0315
/// store 解码(纯 safe 长度前置校验;格式违例 = `Err`,调用方走重建)。
/// binary payload 表项必须按 key 字节序**严格升序**(写出方排序;乱序 = 格式违例)。
pub fn decode_store(bytes: &[u8]) -> Result<PsoStore, String> {
    let mut r = Reader { buf: bytes, pos: 0 };
    let magic = r.take(7)?;
    if magic != STORE_MAGIC {
        return Err("magic 不符".into());
    }
    let _version = r.u32()?;
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(r.take(16)?);
    let vendor_id = r.u32()?;
    let device_id = r.u32()?;
    let driver_version = r.u32()?;
    let mut keyset = [0u8; 32];
    keyset.copy_from_slice(r.take(32)?);
    let branch = r.u32()?;
    let payload = match branch {
        vk::PSO_BRANCH_BINARY => {
            let mut entries: Vec<([u8; 32], Vec<u8>)> = Vec::new();
            let mut prev: Option<[u8; 32]> = None;
            while r.pos < bytes.len() {
                let mut key = [0u8; 32];
                key.copy_from_slice(r.take(32)?);
                let len = r.u32()? as usize;
                let blob = r.take(len)?.to_vec();
                if let Some(p) = prev {
                    if key <= p {
                        return Err("binary payload 表项未按 key 字节序升序".into());
                    }
                }
                prev = Some(key);
                entries.push((key, blob));
            }
            PsoStorePayload::Binary(entries)
        }
        vk::PSO_BRANCH_CACHE => {
            let len = r.u32()? as usize;
            let blob = r.take(len)?.to_vec();
            if r.pos != bytes.len() {
                return Err("cache payload 尾部冗余字节".into());
            }
            PsoStorePayload::Cache(blob)
        }
        _ => return Err(format!("未知分支 tag {branch}")),
    };
    Ok(PsoStore {
        identity: StoreDeviceIdentity {
            pipeline_cache_uuid: uuid,
            vendor_id,
            device_id,
            driver_version,
        },
        keyset_digest: keyset,
        branch,
        payload,
    })
}

//@ spec: RXS-0315
/// **装载核验序**(强制,任一不符 = 丢弃全量重建,绝不部分命中):
/// ① magic/schema_version → ② device identity 逐字段 →(分支 tag,keyset 前即拒)→
/// ③ key-set digest。返回 (rebuild_reason, 干净 store);reason ≠ None ⇒ store = None。
pub fn verify_store(
    bytes: &[u8],
    expected_identity: &StoreDeviceIdentity,
    expected_keyset: &[u8; 32],
    expected_branch: u32,
) -> (RebuildReason, Option<PsoStore>) {
    // ① magic/schema_version(先解码前判 magic,免全量解析;格式违例同归 schema)。
    if bytes.len() < STORE_HEADER_LEN || &bytes[..7] != STORE_MAGIC {
        return (RebuildReason::Schema, None);
    }
    let version = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]);
    if version != STORE_SCHEMA_VERSION {
        return (RebuildReason::Version, None);
    }
    let store = match decode_store(bytes) {
        Ok(s) => s,
        Err(_) => return (RebuildReason::Schema, None),
    };
    // ② device identity 逐字段。
    if store.identity != *expected_identity {
        return (RebuildReason::DeviceIdentity, None);
    }
    // 分支 tag 不符 = keyset 前即拒(RXS-0316 两分支不得混用同一 store 文件)。
    if store.branch != expected_branch {
        return (RebuildReason::Schema, None);
    }
    // ③ key-set digest。
    if &store.keyset_digest != expected_keyset {
        return (RebuildReason::Keyset, None);
    }
    (RebuildReason::None, Some(store))
}

//@ spec: RXS-0315
/// 篡改注入面(测试用):对 header 四轴(schema/version/driver identity/keyset)确定性篡改;
/// 篡改后装载必须走重建路径且输出仍正确(`no_false_hit`)。纯函数可单测。
pub fn tamper_store(bytes: &[u8], axis: &str) -> Result<Vec<u8>, String> {
    if bytes.len() < STORE_HEADER_LEN {
        return Err("store 过短,无法篡改".into());
    }
    let mut out = bytes.to_vec();
    match axis {
        // magic 首字节翻转(schema 轴)。
        "schema" => out[0] ^= 0xFF,
        // schema_version 翻转(version 轴;offset 7)。
        "version" => out[7] ^= 0xFF,
        // pipelineCacheUUID 首字节翻转(driver identity 轴;offset 11)。
        "driver_uuid" => out[11] ^= 0xFF,
        // key-set digest 首字节翻转(keyset 轴;offset 11+16+12 = 39)。
        "keyset" => out[39] ^= 0xFF,
        _ => {
            return Err(format!(
                "未知篡改轴 `{axis}`(闭集 schema/version/driver_uuid/keyset)"
            ));
        }
    }
    Ok(out)
}

/// vendor blob 容器(binary 分支 per-key blob 内层;**vendor 内容非 stable,不入 golden**;
/// 一条 pipeline 可产 N 个 binary〔VK_KHR_pipeline_binary,顺序 = 捕获序,warm 同序〕):
/// `u32 LE count` + 逐条 `u32 LE key_size` + key 字节 + `u32 LE data_size` + data 字节。
pub fn encode_vendor_blob(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (key, data) in pairs {
        out.extend_from_slice(&(key.len() as u32).to_le_bytes());
        out.extend_from_slice(key);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// vendor blob 容器解码(纯 safe 长度前置校验;调用方 = warm 重建 binary 输入)。
/// count ∈ 1..=64(防御上界),key_size ∈ 1..=32,EOF 终界——违例 = 确定性 `Err`。
pub fn decode_vendor_blob(blob: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
    let mut r = Reader { buf: blob, pos: 0 };
    let count = r.u32()? as usize;
    if count == 0 || count > 64 {
        return Err(format!("vendor blob count 非法 {count}"));
    }
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        let klen = r.u32()? as usize;
        if klen == 0 || klen > vk::MAX_PIPELINE_BINARY_KEY_SIZE {
            return Err(format!("vendor key 长度非法 {klen}"));
        }
        let key = r.take(klen)?.to_vec();
        let dlen = r.u32()? as usize;
        let data = r.take(dlen)?.to_vec();
        pairs.push((key, data));
    }
    if r.pos != blob.len() {
        return Err("vendor blob 尾部冗余字节".into());
    }
    Ok(pairs)
}

// ─────────────────────────── manager(RXS-0316) ───────────────────────────

/// 单 key 运行结果(harness JSON / smoke 判据面)。
pub struct PsoKeyRunJson {
    pub name: String,
    pub hit: bool,
    pub stalled: bool,
    pub built: bool,
}

/// 一次 device 运行的完整结果(cold/warm/tamper 三模式同源;harness 转 JSON)。
pub struct PsoRunOutcome {
    pub branch: u32,
    pub pipeline_binary_capability: bool,
    pub pipeline_creation_cache_control: bool,
    pub identity: vk::PsoDeviceIdentity,
    pub keyset_digest: [u8; 32],
    pub rebuild_reason: RebuildReason,
    pub rebuilt: bool,
    pub false_hits: usize,
    pub precache_build_count: usize,
    pub runtime_compile_stalls: usize,
    pub per_key: Vec<PsoKeyRunJson>,
    pub validation_error: bool,
}

/// PSO cache manager(计数器 = 公开字段,RXS-0316 两分支同一定义,进 evidence)。
pub struct PsoCacheManager {
    pub precache_build_count: usize,
    pub runtime_compile_stalls: usize,
}

impl PsoCacheManager {
    pub fn new() -> Self {
        PsoCacheManager {
            precache_build_count: 0,
            runtime_compile_stalls: 0,
        }
    }

    fn identity_of(rep: &vk::PsoSessionReport) -> StoreDeviceIdentity {
        StoreDeviceIdentity {
            pipeline_cache_uuid: rep.identity.pipeline_cache_uuid,
            vendor_id: rep.identity.vendor_id,
            device_id: rep.identity.device_id,
            driver_version: rep.identity.driver_version,
        }
    }

    fn outcome_of(
        rep: vk::PsoSessionReport,
        fixtures: &[PsoFixture],
        keyset: [u8; 32],
        rebuild_reason: RebuildReason,
        rebuilt: bool,
    ) -> PsoRunOutcome {
        let per_key = fixtures
            .iter()
            .zip(rep.outcomes.iter())
            .map(|(f, o)| PsoKeyRunJson {
                name: f.name.to_owned(),
                hit: o.hit,
                stalled: o.stalled,
                built: o.built,
            })
            .collect();
        PsoRunOutcome {
            branch: rep.branch,
            pipeline_binary_capability: rep.pipeline_binary_capability,
            pipeline_creation_cache_control: rep.pipeline_creation_cache_control,
            identity: rep.identity,
            keyset_digest: keyset,
            rebuild_reason,
            rebuilt,
            false_hits: 0, // 篡改 store 核验序拒绝在前,误命中恒 0(RXS-0315 no_false_hit)
            precache_build_count: rep.outcomes.iter().filter(|o| o.built && !o.hit).count(),
            runtime_compile_stalls: rep.outcomes.iter().filter(|o| o.stalled).count(),
            per_key,
            validation_error: rep.validation_error,
        }
    }

    fn plans_of(fixtures: &[PsoFixture]) -> Vec<vk::PsoPipelinePlan<'_>> {
        fixtures.iter().map(fixture_plan).collect()
    }

    /// cold(precache 构建):逐 pso_key 恰好创建一次 pipeline 并捕获落盘(RXS-0316);
    /// 写 `<dir>/rurix_pso_cache.bin`(RXS-0315 格式)。
    //@ spec: RXS-0316
    pub fn cold(&mut self, dir: &Path, fixtures: &[PsoFixture]) -> Result<PsoRunOutcome, String> {
        let records = collect_records(fixtures);
        let keys: Vec<[u8; 32]> = records.iter().map(|r| r.pso_key).collect();
        let keyset = keyset_digest(&keys);
        let plans = Self::plans_of(fixtures);
        let rep = vk::pso_cache_session(&plans, vk::PsoSessionMode::Cold)?;
        self.precache_build_count = rep.outcomes.iter().filter(|o| o.built).count();
        // 组装 store(binary = per-key vendor blob 容器;cache = 整 blob)并落盘。
        let payload = if rep.branch == vk::PSO_BRANCH_BINARY {
            let mut entries: Vec<([u8; 32], Vec<u8>)> = Vec::with_capacity(keys.len());
            for (o, k) in rep.outcomes.iter().zip(keys.iter()) {
                if o.vendor_binaries.is_empty() {
                    return Err("cold binary 分支缺 vendor 捕获".into());
                }
                entries.push((*k, encode_vendor_blob(&o.vendor_binaries)));
            }
            PsoStorePayload::Binary(entries)
        } else {
            let blob = rep
                .cache_blob
                .clone()
                .ok_or_else(|| "cold cache 分支缺 cache blob".to_owned())?;
            PsoStorePayload::Cache(blob)
        };
        let store = PsoStore {
            identity: Self::identity_of(&rep),
            keyset_digest: keyset,
            branch: rep.branch,
            payload,
        };
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录 {}: {e}", dir.display()))?;
        std::fs::write(dir.join(STORE_FILE_NAME), encode_store(&store))
            .map_err(|e| format!("写 store: {e}"))?;
        Ok(Self::outcome_of(
            rep,
            fixtures,
            keyset,
            RebuildReason::None,
            false,
        ))
    }

    /// warm(**全新进程**,判据字面):装载核验 → 逐 key 重建 pipeline;全部 create 带
    /// FAIL_ON bit。store 缺某 key 的 blob(物理删 blob 反证腿)= 该 key miss → 必记 stall。
    /// 返回 (rebuild_reason, 逐 key hit/stall/built);store 失配 → fail-closed 全量重建。
    //@ spec: RXS-0316
    pub fn warm(&mut self, dir: &Path, fixtures: &[PsoFixture]) -> Result<PsoRunOutcome, String> {
        let records = collect_records(fixtures);
        let keys: Vec<[u8; 32]> = records.iter().map(|r| r.pso_key).collect();
        let keyset = keyset_digest(&keys);
        // probe:空 plans 会话取 device identity/branch/能力位(核验序期望值的实测源)。
        let probe = vk::pso_cache_session(&[], vk::PsoSessionMode::Cold)?;
        let expected_identity = Self::identity_of(&probe);
        let branch = probe.branch;
        let path: PathBuf = dir.join(STORE_FILE_NAME);
        let bytes_opt = std::fs::read(&path).ok();
        let (reason, store) = match &bytes_opt {
            Some(b) => verify_store(b, &expected_identity, &keyset, branch),
            None => (RebuildReason::None, None), // 文件缺失 = 空 warm(非失配)
        };
        match (reason, store) {
            (RebuildReason::None, Some(st)) => {
                let plans = Self::plans_of(fixtures);
                let payload = warm_payload_of(&st, &keys)?;
                let rep = vk::pso_cache_session(&plans, vk::PsoSessionMode::Warm(payload))?;
                self.runtime_compile_stalls = rep.outcomes.iter().filter(|o| o.stalled).count();
                Ok(Self::outcome_of(
                    rep,
                    fixtures,
                    keyset,
                    RebuildReason::None,
                    false,
                ))
            }
            (RebuildReason::None, None) => {
                // 无 store:全 miss warm(逐 key 必 stall;计数器能红语义面)。
                let plans = Self::plans_of(fixtures);
                let payload = empty_warm_payload(branch, keys.len());
                let rep = vk::pso_cache_session(&plans, vk::PsoSessionMode::Warm(payload))?;
                self.runtime_compile_stalls = rep.outcomes.iter().filter(|o| o.stalled).count();
                Ok(Self::outcome_of(
                    rep,
                    fixtures,
                    keyset,
                    RebuildReason::None,
                    false,
                ))
            }
            (r, _) => {
                // fail-closed 全量重建(RXS-0315 绝不部分命中):cold 会话重建 store。
                let plans = Self::plans_of(fixtures);
                let rep = vk::pso_cache_session(&plans, vk::PsoSessionMode::Cold)?;
                let payload = if rep.branch == vk::PSO_BRANCH_BINARY {
                    let mut entries: Vec<([u8; 32], Vec<u8>)> = Vec::with_capacity(keys.len());
                    for (o, k) in rep.outcomes.iter().zip(keys.iter()) {
                        if o.vendor_binaries.is_empty() {
                            return Err("重建 binary 分支缺 vendor 捕获".into());
                        }
                        entries.push((*k, encode_vendor_blob(&o.vendor_binaries)));
                    }
                    PsoStorePayload::Binary(entries)
                } else {
                    PsoStorePayload::Cache(
                        rep.cache_blob
                            .clone()
                            .ok_or_else(|| "重建 cache 分支缺 blob".to_owned())?,
                    )
                };
                let store = PsoStore {
                    identity: Self::identity_of(&rep),
                    keyset_digest: keyset,
                    branch: rep.branch,
                    payload,
                };
                std::fs::create_dir_all(dir).map_err(|e| format!("建目录: {e}"))?;
                std::fs::write(&path, encode_store(&store))
                    .map_err(|e| format!("写 store: {e}"))?;
                Ok(Self::outcome_of(rep, fixtures, keyset, r, true))
            }
        }
    }

    /// tamper 四轴(RXS-0315 IR):确定性篡改 store header 对应段后尝试 warm——必须走
    /// 重建路径(rebuild_reason 正确)且输出仍正确(no_false_hit;重建 = cold 全量)。
    //@ spec: RXS-0315
    pub fn tamper(
        &mut self,
        dir: &Path,
        fixtures: &[PsoFixture],
        axis: &str,
    ) -> Result<PsoRunOutcome, String> {
        let path = dir.join(STORE_FILE_NAME);
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("tamper 前读 store {}: {e}", path.display()))?;
        let tampered = tamper_store(&bytes, axis)?;
        std::fs::write(&path, &tampered).map_err(|e| format!("tamper 写回: {e}"))?;
        self.warm(dir, fixtures)
    }

    /// 能红反证腿(RXS-0316「warm 前删除单条 key 的持久化数据 → 该 key 必须记 stall」):
    /// 物理改写 store 文件——binary 分支删第 `n` 条 payload 表项(header 原样,核验序仍过,
    /// 该 key 无 blob 可用 → miss → stall);cache 分支 blob 单体式不可删单 key,清整 blob
    /// 为等价全 miss 面(red_leg_scope 如实记 `all_keys`)。
    //@ spec: RXS-0316
    pub fn drop_key_blob(&mut self, dir: &Path, n: usize) -> Result<String, String> {
        let path = dir.join(STORE_FILE_NAME);
        let bytes = std::fs::read(&path).map_err(|e| format!("drop 前读 store: {e}"))?;
        let mut store = decode_store(&bytes).map_err(|e| format!("drop 前解析 store: {e}"))?;
        let scope = match &mut store.payload {
            PsoStorePayload::Binary(entries) => {
                if n >= entries.len() {
                    return Err(format!("drop-key 下标 {n} 越界({} 条)", entries.len()));
                }
                entries.remove(n);
                "single_key".to_owned()
            }
            PsoStorePayload::Cache(blob) => {
                blob.clear();
                "all_keys".to_owned()
            }
        };
        std::fs::write(&path, encode_store(&store)).map_err(|e| format!("drop 写回: {e}"))?;
        Ok(scope)
    }
}

/// store → warm payload(binary:逐 key vendor 容器解码;store 缺某 key 表项 = miss,
/// 该 key 必记 stall——删 blob 反证腿统一语义;cache:整 blob 原样)。
fn warm_payload_of<'a>(
    store: &'a PsoStore,
    keys: &[[u8; 32]],
) -> Result<vk::PsoWarmPayload<'a>, String> {
    match &store.payload {
        PsoStorePayload::Binary(entries) => {
            let mut blobs: Vec<Option<vk::PsoBinaryBlobRef<'a>>> = Vec::with_capacity(keys.len());
            // 容器解码物在 session 期间须存活——泄漏免除(store 字节恒为静态不可信输入,
            // 每 harness 进程一次,泄漏有界)。
            for k in keys.iter() {
                // 缺该 key 的 blob(物理删 blob 反证腿 / 表项边界截断)= miss → 该 key
                // 必记 stall(RXS-0316 能红语义;绝不误命中)。
                let Some((_, blob)) = entries.iter().find(|(ek, _)| ek == k) else {
                    blobs.push(None);
                    continue;
                };
                let pairs = decode_vendor_blob(blob)?;
                let refs: Vec<(&'a [u8], &'a [u8])> = pairs
                    .into_iter()
                    .map(|(k2, d)| {
                        (
                            Box::leak(k2.into_boxed_slice()) as &'a [u8],
                            Box::leak(d.into_boxed_slice()) as &'a [u8],
                        )
                    })
                    .collect();
                let refs_ref: &'a [(&'a [u8], &'a [u8])] = Box::leak(refs.into_boxed_slice());
                blobs.push(Some(vk::PsoBinaryBlobRef { binaries: refs_ref }));
            }
            Ok(vk::PsoWarmPayload::Binary(blobs))
        }
        PsoStorePayload::Cache(blob) => Ok(vk::PsoWarmPayload::Cache(blob)),
    }
}

/// 空 warm payload(无 store 全 miss 面;计数器能红语义)。
fn empty_warm_payload(branch: u32, n: usize) -> vk::PsoWarmPayload<'static> {
    if branch == vk::PSO_BRANCH_BINARY {
        vk::PsoWarmPayload::Binary(vec![None; n])
    } else {
        vk::PsoWarmPayload::Cache(&[])
    }
}

// ─────────────────────────── 单测(纯 host,≥8 项) ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fixture(kind_tag: u32, seed: u8) -> ([u8; 32], Vec<StageKeyInput>, Vec<u8>) {
        let stages = vec![StageKeyInput {
            stage_tag: STAGE_COMPUTE,
            artifact_digest: sha256::digest(&[seed; 8]),
        }];
        let ff = vec![0u8; 4];
        let key = pso_key(&PsoKeyInput {
            kind_tag,
            stages: &stages,
            fixed_function_canonical: &ff,
        });
        (key, stages, ff)
    }

    /// RXS-0314:key 计算确定性(双次逐字节相等)。
    //@ spec: RXS-0314
    #[test]
    fn pso_key_deterministic_byte_equal() {
        let (k1, stages, ff) = sample_fixture(KIND_COMPUTE, 7);
        let k2 = pso_key(&PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &stages,
            fixed_function_canonical: &ff,
        });
        assert_eq!(k1, k2, "同输入双跑 pso_key 须逐字节相等");
    }

    /// RXS-0314:preimage 七段判别力——逐段微扰必改 key(域分离 + 定界编码 by construction)。
    //@ spec: RXS-0314
    #[test]
    fn preimage_seven_segments_discriminate() {
        let (base, stages, ff) = sample_fixture(KIND_COMPUTE, 7);
        // 段① kind_tag。
        let k = pso_key(&PsoKeyInput {
            kind_tag: KIND_GRAPHICS,
            stages: &stages,
            fixed_function_canonical: &ff,
        });
        assert_ne!(k, base, "kind_tag 微扰必改 key");
        // 段② stage digest。
        let s2 = vec![StageKeyInput {
            stage_tag: STAGE_COMPUTE,
            artifact_digest: sha256::digest(&[8u8; 8]),
        }];
        let k = pso_key(&PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &s2,
            fixed_function_canonical: &ff,
        });
        assert_ne!(k, base, "stage digest 微扰必改 key");
        // 段② stage_tag(同 digest 异 tag)。
        let s3 = vec![StageKeyInput {
            stage_tag: STAGE_VERTEX,
            artifact_digest: stages[0].artifact_digest,
        }];
        let k = pso_key(&PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &s3,
            fixed_function_canonical: &ff,
        });
        assert_ne!(k, base, "stage_tag 微扰必改 key");
        // 段⑥ 固定功能状态。
        let ff2 = vec![1u8, 0, 0, 0];
        let k = pso_key(&PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &stages,
            fixed_function_canonical: &ff2,
        });
        assert_ne!(k, base, "固定功能状态微扰必改 key");
        // 段② stage 数(计数前缀定界)。
        let s4: Vec<StageKeyInput> = Vec::new();
        let k = pso_key(&PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &s4,
            fixed_function_canonical: &ff,
        });
        assert_ne!(k, base, "stage 计数微扰必改 key");
    }

    /// RXS-0314:collector 输出与 checked-in golden **逐字节全等**(key 集合全等判据 +
    /// fixture 非空哨兵;嵌入语料降级时本测试确定性失败 = 诚实红,非假绿)。
    //@ spec: RXS-0314
    #[test]
    fn collector_key_set_equals_golden() {
        let fixtures = pso_fixtures().expect("fixture 集构建(嵌入语料须非空)");
        assert_eq!(fixtures.len(), 5, "冻结 workload 表 = 5 条 pipeline");
        let records = collect_records(&fixtures);
        let json = collector_json(&records);
        let golden_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("pso")
            .join("pso_keys.golden.json");
        let golden = std::fs::read_to_string(&golden_path)
            .unwrap_or_else(|e| panic!("读 golden {}: {e}", golden_path.display()));
        assert_eq!(
            json, golden,
            "collector 输出与 golden 不逐字节相等(见 tests/pso/pso_keys.golden.json;变更需重生成并 review)"
        );
        // keyset digest 交叉锚:records 键集 → digest 与 JSON 内字段一致。
        let keys: Vec<[u8; 32]> = records.iter().map(|r| r.pso_key).collect();
        let ksd = hex_of(&keyset_digest(&keys));
        assert!(
            json.contains(&format!("\"keyset_digest\": \"{ksd}\"")),
            "JSON keyset_digest 字段与 records 键集 digest 不一致"
        );
    }

    fn sample_store(branch: u32) -> PsoStore {
        let payload = if branch == vk::PSO_BRANCH_BINARY {
            PsoStorePayload::Binary(vec![
                (
                    sha256::digest(b"k2"),
                    encode_vendor_blob(&[(vec![9u8; 32], vec![1, 2, 3])]),
                ),
                (
                    sha256::digest(b"k1"),
                    encode_vendor_blob(&[(vec![8u8; 32], vec![4, 5]), (vec![7u8; 32], vec![6])]),
                ),
            ])
        } else {
            PsoStorePayload::Cache(vec![1, 2, 3, 4, 5, 6, 7, 8])
        };
        PsoStore {
            identity: StoreDeviceIdentity {
                pipeline_cache_uuid: [7u8; 16],
                vendor_id: 0x10de,
                device_id: 0x2782,
                driver_version: 620_002,
            },
            keyset_digest: sha256::digest(b"keyset"),
            branch,
            payload,
        }
    }

    /// RXS-0315:store roundtrip(两分支 encode→decode 逐字段相等;binary 表项重排序归一)。
    //@ spec: RXS-0315
    #[test]
    fn store_roundtrip() {
        for branch in [vk::PSO_BRANCH_BINARY, vk::PSO_BRANCH_CACHE] {
            let store = sample_store(branch);
            let bytes = encode_store(&store);
            let back = decode_store(&bytes).expect("decode_store");
            assert_eq!(back.identity, store.identity, "identity roundtrip");
            assert_eq!(back.keyset_digest, store.keyset_digest, "keyset roundtrip");
            assert_eq!(back.branch, store.branch, "branch roundtrip");
            match (&back.payload, &store.payload) {
                (PsoStorePayload::Binary(a), PsoStorePayload::Binary(b)) => {
                    let mut bs = b.clone();
                    bs.sort_by(|x, y| x.0.cmp(&y.0));
                    assert_eq!(*a, bs, "binary payload 排序归一后相等");
                }
                (PsoStorePayload::Cache(a), PsoStorePayload::Cache(b)) => {
                    assert_eq!(a, b, "cache payload roundtrip")
                }
                _ => panic!("payload 分支不一致"),
            }
        }
    }

    /// RXS-0315:核验序①——magic/schema 篡改 → rebuild_reason = schema。
    //@ spec: RXS-0315
    #[test]
    fn verify_tamper_schema_reason_schema() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        let t = tamper_store(&bytes, "schema").unwrap();
        let (reason, st) = verify_store(&t, &store.identity, &store.keyset_digest, store.branch);
        assert_eq!(reason, RebuildReason::Schema);
        assert!(st.is_none(), "失配即拒,绝不部分命中");
    }

    /// RXS-0315:核验序①——version 篡改 → rebuild_reason = version。
    //@ spec: RXS-0315
    #[test]
    fn verify_tamper_version_reason_version() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        let t = tamper_store(&bytes, "version").unwrap();
        let (reason, st) = verify_store(&t, &store.identity, &store.keyset_digest, store.branch);
        assert_eq!(reason, RebuildReason::Version);
        assert!(st.is_none());
    }

    /// RXS-0315:核验序②——driver identity(uuid)篡改 → rebuild_reason = device_identity。
    //@ spec: RXS-0315
    #[test]
    fn verify_tamper_driver_identity_reason_device_identity() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        let t = tamper_store(&bytes, "driver_uuid").unwrap();
        let (reason, st) = verify_store(&t, &store.identity, &store.keyset_digest, store.branch);
        assert_eq!(reason, RebuildReason::DeviceIdentity);
        assert!(st.is_none());
        // 逐字段面:vendor/device/driver 任一篡改同判。
        for off in [27usize, 31, 35] {
            let mut t2 = bytes.clone();
            t2[off] ^= 0xFF;
            let (r2, _) = verify_store(&t2, &store.identity, &store.keyset_digest, store.branch);
            assert_eq!(r2, RebuildReason::DeviceIdentity, "offset {off} 逐字段核验");
        }
    }

    /// RXS-0315:核验序③——keyset digest 篡改 → rebuild_reason = keyset。
    //@ spec: RXS-0315
    #[test]
    fn verify_tamper_keyset_reason_keyset() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        let t = tamper_store(&bytes, "keyset").unwrap();
        let (reason, st) = verify_store(&t, &store.identity, &store.keyset_digest, store.branch);
        assert_eq!(reason, RebuildReason::Keyset);
        assert!(st.is_none());
        // 干净装载对照:reason = none 且 store 回。
        let (r_ok, st_ok) =
            verify_store(&bytes, &store.identity, &store.keyset_digest, store.branch);
        assert_eq!(r_ok, RebuildReason::None);
        assert!(st_ok.is_some());
    }

    /// RXS-0316:分支 tag 不符 = keyset 前即拒(两分支不得混用同一 store 文件;归 schema 类)。
    //@ spec: RXS-0316
    #[test]
    fn verify_branch_tag_rejected_before_keyset() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        // 以 cache 分支期望装载 binary store:identity/keyset 全对,唯分支不符 → schema 拒。
        let (reason, st) = verify_store(
            &bytes,
            &store.identity,
            &store.keyset_digest,
            vk::PSO_BRANCH_CACHE,
        );
        assert_eq!(reason, RebuildReason::Schema);
        assert!(st.is_none());
    }

    /// RXS-0315:截断 blob fail-closed(逐偏移截断:非表项边界 = 确定性 Err → 重建;
    /// 表项边界截断 = 可解析但条目严格减少 → 缺失 key 走 miss/stall 语义〔与删 blob
    /// 反证腿同源〕,绝不部分**误命中**;全程无 panic)。
    //@ spec: RXS-0315
    #[test]
    fn truncated_blob_fail_closed() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        let full_entries = match &store.payload {
            PsoStorePayload::Binary(e) => e.len(),
            _ => unreachable!(),
        };
        let mut saw_err = false;
        let mut saw_boundary = false;
        for cut in 0..bytes.len() {
            let t = &bytes[..cut];
            match decode_store(t) {
                Err(_) => saw_err = true,
                Ok(partial) => {
                    saw_boundary = true;
                    match &partial.payload {
                        PsoStorePayload::Binary(e) => assert!(
                            e.len() < full_entries,
                            "边界截断至 {cut} 条目必严格减少(缺 key 走 miss/stall)"
                        ),
                        _ => panic!("分支漂移"),
                    }
                }
            }
        }
        assert!(saw_err, "非边界截断必 fail-closed Err");
        assert!(saw_boundary, "表项边界截断样本须在场(2 表项样本)");
        // 缺 key 表项 → warm payload 该 key 为 None(miss → stall;绝不误命中)。
        let partial = {
            let full = encode_store(&store);
            // 删第 0 条表项后的合法 store(header 原样)。
            let mut s2 = store.clone();
            match &mut s2.payload {
                PsoStorePayload::Binary(e) => {
                    e.remove(0);
                }
                _ => unreachable!(),
            }
            let _ = full;
            s2
        };
        let keys: Vec<[u8; 32]> = match &store.payload {
            PsoStorePayload::Binary(e) => e.iter().map(|(k, _)| *k).collect(),
            _ => unreachable!(),
        };
        let payload = warm_payload_of(&partial, &keys).expect("warm payload");
        match payload {
            vk::PsoWarmPayload::Binary(blobs) => {
                let missing = blobs.iter().filter(|b| b.is_none()).count();
                assert_eq!(missing, 1, "缺 1 表项 → 恰 1 key miss(必记 stall)");
            }
            _ => panic!("分支漂移"),
        }
    }

    /// RXS-0315:key-set digest 排序确定性(输入乱序 digest 不变)。
    //@ spec: RXS-0315
    #[test]
    fn keyset_digest_sorted_deterministic() {
        let a = sha256::digest(b"aaa");
        let b = sha256::digest(b"bbb");
        let c = sha256::digest(b"ccc");
        assert_eq!(keyset_digest(&[a, b, c]), keyset_digest(&[c, a, b]));
        assert_ne!(keyset_digest(&[a, b]), keyset_digest(&[a, b, c]));
    }

    /// RXS-0355(G9.3 M106):pso_key 第八段加性扩展——`None` ≡ 既有 `pso_key`
    /// 逐字节(0-drift);`Some` 双跑逐字节相等、异于基 key、随成员索引与 set
    /// 身份区分;**失效重建确定性**(同 spec 重建 → 同一 set 身份 digest → 同一
    /// 成员扩展 key,逐位一致)。
    //@ spec: RXS-0355
    #[test]
    fn execution_set_membership_extension() {
        use crate::execution_set::{
            ExecutionSet, ExecutionSetMemberSpec, ExecutionSetMembership, ExecutionSetSpec,
        };
        let (base, stages, ff) = sample_fixture(KIND_COMPUTE, 7);
        let input = PsoKeyInput {
            kind_tag: KIND_COMPUTE,
            stages: &stages,
            fixed_function_canonical: &ff,
        };
        // 0-drift:None ≡ 既有 pso_key(与既有 golden 同源)。
        assert_eq!(
            pso_key_with_membership(&input, None),
            base,
            "membership 缺省 ≡ 既有 pso_key 逐字节(0-drift)"
        );
        // set 构建(成员 = PSO cache 条目子集视图;成员键 = 既有 pso_key)。
        let spec = ExecutionSetSpec {
            name: "mat_set".to_owned(),
            state_canonical: vec![0xC0, 0xFF, 0xEE, 0x00],
            members: vec![
                ExecutionSetMemberSpec {
                    name: "m0".to_owned(),
                    pso_key: base,
                },
                ExecutionSetMemberSpec {
                    name: "m1".to_owned(),
                    pso_key: sha256::digest(b"m1-pso"),
                },
            ],
        };
        let set = ExecutionSet::build(&spec).expect("合法 set");
        let m0 = execution_set_membership(&set, 0);
        let k1 = pso_key_with_membership(&input, Some(&m0));
        let k2 = pso_key_with_membership(&input, Some(&m0));
        assert_eq!(k1, k2, "成员扩展 key 双跑逐字节相等");
        assert_ne!(k1, base, "第八段尾随必改 key(加性字段有区分力)");
        // 成员索引区分(同 set 同基 key,异索引)。
        let m1 = ExecutionSetMembership {
            set_identity: m0.set_identity,
            member_index: 1,
        };
        assert_ne!(
            pso_key_with_membership(&input, Some(&m1)),
            k1,
            "成员索引必入 key"
        );
        // set 身份区分(同索引异 set)。
        let spec2 = ExecutionSetSpec {
            name: "other_set".to_owned(),
            ..spec.clone()
        };
        let set2 = ExecutionSet::build(&spec2).unwrap();
        let m0_other = execution_set_membership(&set2, 0);
        assert_ne!(m0_other.set_identity, m0.set_identity, "异 set 异身份");
        assert_ne!(pso_key_with_membership(&input, Some(&m0_other)), k1);
        // 失效重建确定性(RXS-0355 L3):同 spec 重建 → 成员身份逐位一致 →
        // 扩展 key 逐位一致。
        let rebuilt = ExecutionSet::rebuild(&spec, 1).unwrap();
        let m0r = execution_set_membership(&rebuilt, 0);
        assert_eq!(m0r, m0, "重建产物成员身份逐位一致");
        assert_eq!(pso_key_with_membership(&input, Some(&m0r)), k1);
        // 身份 digest 双跑稳定 + 与异 set 相异。
        assert_eq!(execution_set_identity(&set), execution_set_identity(&set));
        assert_ne!(execution_set_identity(&set), execution_set_identity(&set2));
    }

    /// RXS-0314:空编码常量与 M31/M32/M29 既有字面同一(M32 smoke PROFILE_NONE_DIGEST /
    /// EMPTY_DOMAIN_DIGEST 字面锚;iface 轴空编码 = IFACE domain 串裸 digest 自洽)。
    //@ spec: RXS-0314
    #[test]
    fn empty_encoding_constants_match_m31_literals() {
        assert_eq!(
            hex_of(&profile_none_digest()),
            "2997fd21a324a39e63cd1da6970db88c511e8d025d24fbce0bbb94c5ea8c28b6",
            "profile 空编码常量须 = M32 smoke PROFILE_NONE_DIGEST 字面"
        );
        assert_eq!(
            hex_of(&sha256::digest(PERM_EMPTY_DOMAIN)),
            "160d241dc1681a927e8edbdd07a15e508f9f5aeb68da8bc92274332cb8541f31",
            "permutation 空域常量须 = M32 smoke EMPTY_DOMAIN_DIGEST 字面"
        );
        // iface 轴空编码 = sha256(IFACE domain 串),确定性自检(两次相等 + 异于他域)。
        assert_eq!(iface_none_digest(), sha256::digest(IFACE_NONE_DOMAIN));
        assert_ne!(iface_none_digest(), profile_none_digest());
    }

    /// RXS-0314:手编 compute 见证结构自检(magic/版本/entry `main`/接口解析一致)。
    //@ spec: RXS-0314
    #[test]
    fn hand_written_spv_wellformed() {
        for (name, spv) in [("fill", fill_spv()), ("atomics", atomics_spv())] {
            assert_eq!(spv[0], 0x0723_0203, "{name} SPIR-V magic");
            assert!(
                spv[3] > 19 && spv[3] < 64,
                "{name} bound 覆盖全部 id(实测 {})",
                spv[3]
            );
            assert_eq!(
                vk::entry_point_name(&spv).as_deref(),
                Some("main"),
                "{name} entry = main"
            );
            let (bindings, pc) = parse_compute_iface(&spv);
            assert_eq!(bindings, vec![0], "{name} 单 SSBO binding 0");
            assert_eq!(pc, 0, "{name} 无 push constants");
        }
    }

    /// RXS-0314:saxpy 嵌入真产物接口解析(3 SSBO 连续 binding + push constants;
    /// 与 vk.rs marshalling 测试同一单一事实源)。语料降级 → dev-env degrade 早返。
    //@ spec: RXS-0314
    #[test]
    fn saxpy_iface_parse() {
        if SAXPY_SPV.is_empty() {
            eprintln!("[pso] SKIP: build.rs 未产 saxpy.spv (dev-env degrade)");
            return;
        }
        let words = spv_words(SAXPY_SPV);
        let (bindings, pc) = parse_compute_iface(&words);
        assert_eq!(bindings, vec![0, 1, 2], "saxpy 3 SSBO binding [0,1,2]");
        assert!(
            pc >= 8 && pc % 4 == 0,
            "saxpy push constants ≥ 8 且 4 对齐(实测 {pc})"
        );
        assert!(vk::entry_point_name(&words).is_some());
    }

    /// RXS-0315:vendor blob 容器 roundtrip + 截断 fail-closed(N 对格式)。
    //@ spec: RXS-0315
    #[test]
    fn vendor_blob_roundtrip_and_truncation() {
        let pairs = vec![(vec![5u8; 32], vec![1, 2, 3, 4]), (vec![6u8; 32], vec![7])];
        let blob = encode_vendor_blob(&pairs);
        let back = decode_vendor_blob(&blob).unwrap();
        assert_eq!(back, pairs, "N 对容器 roundtrip");
        for cut in 0..blob.len() {
            assert!(
                decode_vendor_blob(&blob[..cut]).is_err(),
                "截断 {cut} 必 Err"
            );
        }
        assert!(
            decode_vendor_blob(&encode_vendor_blob(&[])).is_err(),
            "count 0 非法"
        );
        assert!(
            decode_vendor_blob(&encode_vendor_blob(&[(vec![], vec![1])])).is_err(),
            "空 key 非法"
        );
    }

    /// RXS-0315:篡改四轴确定性(同输入双次篡改逐字节相等;各轴落点正确)。
    //@ spec: RXS-0315
    #[test]
    fn tamper_axes_deterministic() {
        let store = sample_store(vk::PSO_BRANCH_BINARY);
        let bytes = encode_store(&store);
        for axis in ["schema", "version", "driver_uuid", "keyset"] {
            let t1 = tamper_store(&bytes, axis).unwrap();
            let t2 = tamper_store(&bytes, axis).unwrap();
            assert_eq!(t1, t2, "轴 {axis} 篡改确定性");
            assert_ne!(t1, bytes, "轴 {axis} 篡改必改字节");
        }
        assert!(tamper_store(&bytes, "bogus").is_err(), "闭集外轴确定性 Err");
    }

    /// M30 FFI 布局锚(repr(C) 与 vulkan_core.h 1.3.296 逐字节核对;size/align 断言)。
    //@ spec: RXS-0316
    #[test]
    fn pso_ffi_layout_anchors() {
        use std::mem::{align_of, size_of};
        assert_eq!(
            size_of::<vk::PipelineCacheCreateInfo>(),
            40,
            "PipelineCacheCreateInfo 40"
        );
        assert_eq!(
            size_of::<vk::PipelineCreateFlags2CreateInfoKHR>(),
            24,
            "Flags2 24"
        );
        assert_eq!(
            size_of::<vk::PipelineBinaryKeyKHR>(),
            56,
            "BinaryKey 56(52 + 尾对齐)"
        );
        assert_eq!(align_of::<vk::PipelineBinaryKeyKHR>(), 8);
        assert_eq!(size_of::<vk::PipelineBinaryDataKHR>(), 16, "BinaryData 16");
        assert_eq!(
            size_of::<vk::PipelineBinaryKeysAndDataKHR>(),
            24,
            "KeysAndData 24"
        );
        assert_eq!(
            size_of::<vk::PipelineCreateInfoKHR>(),
            16,
            "CreateInfoKHR 16"
        );
        assert_eq!(
            size_of::<vk::PipelineBinaryCreateInfoKHR>(),
            40,
            "BinaryCreateInfo 40"
        );
        assert_eq!(size_of::<vk::PipelineBinaryInfoKHR>(), 32, "BinaryInfo 32");
        assert_eq!(
            size_of::<vk::PipelineBinaryDataInfoKHR>(),
            24,
            "BinaryDataInfo 24"
        );
        assert_eq!(
            size_of::<vk::PipelineBinaryHandlesInfoKHR>(),
            32,
            "BinaryHandlesInfo 32"
        );
        assert_eq!(
            size_of::<vk::ReleaseCapturedPipelineDataInfoKHR>(),
            24,
            "ReleaseCaptured 24"
        );
        assert_eq!(
            size_of::<vk::PhysicalDevicePipelineCreationCacheControlFeatures>(),
            24,
            "CacheControlFeatures 24"
        );
        assert_eq!(
            size_of::<vk::PhysicalDeviceMaintenance5FeaturesKHR>(),
            24,
            "Maintenance5Features 24"
        );
        assert_eq!(
            size_of::<vk::PhysicalDevicePipelineBinaryFeaturesKHR>(),
            24,
            "PipelineBinaryFeatures 24"
        );
        assert_eq!(
            size_of::<vk::PsoPropertiesBlob>(),
            2048,
            "PropertiesBlob 2048"
        );
        assert_eq!(
            align_of::<vk::PsoPropertiesBlob>(),
            8,
            "PropertiesBlob align 8"
        );
    }
}
