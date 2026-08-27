//! `shader_library` — G9.3 M107 shader library IR 函数级组合链接 + 变体工程级
//! 总预算(g9.p1.m107.shader_library_ir_link;spec/gpu_driven_submit.md RXS-0356;
//! RFC-0023 §4.5/§4.6)。纯 host、safe。
//!
//! - **IR 链接主轴与 v1 边界**(RXS-0356 L1;RFC-0023 §4.5 逐字):编译期 IR
//!   链接——module 级着色函数以稳定符号导出([`LibraryUnit`] / [`ExportedSymbol`]),
//!   链接期按 manifest **显式声明的拓扑**([`LinkTopology`])把「材质函数 ×
//!   lighting 函数 × pass 入口」组合物化;v1 只做**函数级符号链接**,禁跨
//!   module 泛型单态化,**禁隐式全图链接**(被引符号必须落在拓扑声明的 unit
//!   集内,否则 = 符号缺失诊断)。
//! - **interface hash 确定性**(RXS-0356 L2):链接后 interface hash **重算**
//!   ([`crate::reflection::interface_hash_of`]——M31 canonical bytes + 域分离
//!   SHA-256 机构单一事实源,RXS-0306 定义面不变)并随链接记录写回 manifest
//!   面([`to_manifest_json`];M85 manifest 本体 0-byte,接线归 CI 门代理);
//!   manifest 记录链接拓扑(哪个 module 的哪个符号进哪个变体);**同输入双构建
//!   interface hash 相等**;拓扑 → 产物 digest 重算相等(审计可回放——
//!   [`LinkedArtifact::canonical`] 为拓扑规范字节,`artifact_digest` 为其域分离
//!   压缩)。IR 链接发生在 permutation 求解之后(变体 key 确定 → manifest 查
//!   拓扑 → 组合物化 → artifact digest 进 DDC;`--permutation-select` 路径
//!   RXS-0310 承载)。
//! - **链接合法性 fail-closed**(RXS-0356 L3):跨 module 函数链接的类型契约 =
//!   既有阶段间接口契约(RXS-0155)+ reflection 接口事实同一提取律(单一事实
//!   源——被引符号的期望 interface hash 对照实际导出);符号缺失/类型契约失配/
//!   接口失配/循环链接 → 编译期确定性诊断([`LinkError`] typed `Err`,无最近邻
//!   回退,沿 RXS-0310 选择律先例),不设 UB。
//! - **变体工程级总预算硬失败**(RXS-0356 L4;RFC-0023 §4.6 逐字):per-entry
//!   budget(RXS-0310 既有,permutation.rs)之外新增**工程级总预算门**
//!   ([`audit_project_variants`])——超预算**装配期硬失败**(typed `Err`
//!   [`VariantAuditError::TotalBudgetExceeded`],非警告、非 panic;诊断码实现期
//!   从工具段按实际可达类别领取,不预造,故本模块不落 RX 码);审计报告 schema
//!   = `rurix.variant-audit-report.v1`([`to_audit_json`];per-entry 行由上游
//!   permutation 报告派生,axis 贡献分解归 permutation.rs `REPORT_SCHEMA_ID`
//!   既有面);审计恒等式 `enumerated == pruned + emitted` **工程级**成立(逐
//!   行与合计双重断言);manifest 声明变体 ∪ DDC 产物闭合(`ddc_hits > emitted`
//!   = 声明外产物,确定性拒绝);**死变体只报告不自动删**(删除是人的决定)。
//!
//! device/CI 接线点(留 CI 门代理,`ci/g9_shader_library_ir_link_smoke.py`,
//! symbolic key `g9.p1.m107.shader_library_ir_link`):组合物化产物(SPIR-V/DXIL)
//! 的 codegen 接线与 DDC 往返在本模块下游;本模块交付链接拓扑/interface hash/
//! 产物 digest 与预算门的**库面单一事实源**。

use std::collections::BTreeMap;

use rurix_pkg::sha256;

/// 链接产物 schema 标识(拓扑规范字节的版本前缀;与 permutation.rs「canonical
/// 字节自带前缀,digest = SHA-256(完整 canonical 字节)」同一律)。
const LINK_DOMAIN_V1: &[u8] = b"rurix.shader-library-link.v1\0";

/// 变体审计报告 schema 标识(RXS-0356 L4:沿 `rurix.permutation-report.v1` 先例
/// 新建)。
pub const VARIANT_AUDIT_SCHEMA_ID: &str = "rurix.variant-audit-report.v1";
/// 变体审计报告 schema 版本。
pub const VARIANT_AUDIT_SCHEMA_VERSION: u32 = 1;

// ═══════════════════════ canonical 编码(RXS-0305 CanonW 律) ═══════════════════════

/// canonical bytes 写入器(与 reflection.rs/permutation.rs CanonW 同一律:u32 小端
/// 定宽、length-prefix UTF-8 字符串、u32 计数列表;本模块自持有副本,零跨模块
/// 私有依赖)。
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

// ═══════════════════════ 库单元与导出面(RXS-0356 L1) ═══════════════════════

/// 跨 module 符号引用(函数级;类型契约核验面):`expected_interface` = 被引符号
/// 的 interface hash(`reflection::interface_hash_of(其 interface canonical)`,
/// reflection 接口事实同一提取律——单一事实源,RXS-0356 L3)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SymbolRequirement {
    /// 被引符号所在 unit 名(必须落在链接拓扑声明的 unit 集内——禁隐式全图链接)。
    pub unit: String,
    /// 被引符号名。
    pub symbol: String,
    /// 期望的 interface hash(类型契约)。
    pub expected_interface: [u8; 32],
}

/// 稳定符号导出(module 级着色函数;`interface_canonical` = reflection 接口事实
/// 同一提取律的 canonical bytes 产物,函数体不进接口面——RXS-0306 分离规则)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExportedSymbol {
    /// 符号名(unit 内唯一)。
    pub name: String,
    /// 接口事实 canonical bytes(reflection 提取律产物)。
    pub interface_canonical: Vec<u8>,
    /// 跨 module 符号引用表(本符号的链接需求;可为空)。
    pub requires: Vec<SymbolRequirement>,
}

/// 库单元(module;着色函数以稳定符号导出)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LibraryUnit {
    /// unit 名(链接域内唯一;审计回放与 manifest 拓扑记录面)。
    pub name: String,
    /// 导出符号表。
    pub exports: Vec<ExportedSymbol>,
}

/// 链接拓扑(manifest **显式声明**;禁隐式全图链接,RXS-0356 L1)。`units` 的
/// 声明序不进产物(合并序规范化为 unit 名字节序——同输入集双构建相等)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkTopology {
    /// 变体 key(permutation 求解产物,RXS-0310 路径承载)。
    pub variant_key: String,
    /// 参与链接的 unit 名集(显式拓扑;合并序规范化)。
    pub units: Vec<String>,
    /// pass 入口所在 unit(材质函数 × lighting 函数 × pass 入口的组合锚)。
    pub entry_unit: String,
    /// pass 入口符号。
    pub entry_symbol: String,
}

// ═══════════════════════ 链接诊断(fail-closed,RXS-0356 L3) ═══════════════════════

/// IR 链接违例(编译期确定性诊断,typed `Err`;无最近邻回退,沿 RXS-0310 选择律
/// 先例;库面诊断不占 RX 码——诊断码实现期从实际可达类别领取,不预造)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LinkError {
    /// 拓扑声明的 unit 不在输入库单元集内(符号缺失族)。
    MissingUnit {
        /// 缺失的 unit 名。
        unit: String,
    },
    /// 输入库单元表内 unit 重名(链接域歧义,fail-closed)。
    DuplicateUnit {
        /// 重名 unit。
        unit: String,
    },
    /// 被引/入口符号在链接域内不存在(符号缺失)。
    MissingSymbol {
        /// 符号所在 unit。
        unit: String,
        /// 符号名。
        symbol: String,
    },
    /// 同一 unit 内符号重名导出(符号解析歧义,fail-closed)。
    DuplicateSymbol {
        /// unit 名。
        unit: String,
        /// 重名符号。
        symbol: String,
    },
    /// 类型契约失配/接口失配(期望 interface hash ≠ 实际导出)。
    InterfaceMismatch {
        /// 引用方 unit。
        from_unit: String,
        /// 引用方符号。
        from_symbol: String,
        /// 被引 unit。
        to_unit: String,
        /// 被引符号。
        to_symbol: String,
    },
    /// 循环链接(符号级依赖环;`cycle` = 规范化环路径展示)。
    CyclicLink {
        /// 环路径(`a::b -> c::d -> a::b` 形态;确定性)。
        cycle: String,
    },
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::MissingUnit { unit } => write!(
                f,
                "IR 链接符号缺失:拓扑声明的 unit `{unit}` 不在输入库单元集内(禁隐式全图链接,fail-closed,RXS-0356 L3)"
            ),
            LinkError::DuplicateUnit { unit } => write!(
                f,
                "IR 链接输入歧义:unit `{unit}` 重名(fail-closed,RXS-0356 L3)"
            ),
            LinkError::MissingSymbol { unit, symbol } => write!(
                f,
                "IR 链接符号缺失:`{unit}::{symbol}` 在链接域内不存在(无最近邻回退,fail-closed,RXS-0356 L3)"
            ),
            LinkError::DuplicateSymbol { unit, symbol } => write!(
                f,
                "IR 链接符号歧义:unit `{unit}` 内符号 `{symbol}` 重名导出(fail-closed,RXS-0356 L3)"
            ),
            LinkError::InterfaceMismatch {
                from_unit,
                from_symbol,
                to_unit,
                to_symbol,
            } => write!(
                f,
                "IR 链接类型契约失配:`{from_unit}::{from_symbol}` 对 `{to_unit}::{to_symbol}` 的期望 interface hash 与实际导出不符(接口失配,fail-closed,RXS-0356 L3)"
            ),
            LinkError::CyclicLink { cycle } => write!(
                f,
                "IR 循环链接:{cycle}(符号级依赖环,fail-closed,RXS-0356 L3)"
            ),
        }
    }
}

impl std::error::Error for LinkError {}

// ═══════════════════════ 链接产物(RXS-0356 L2) ═══════════════════════

/// manifest 拓扑记录的一行(哪个 module 的哪个符号进哪个变体)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkedMember {
    /// unit 名。
    pub unit: String,
    /// 符号名。
    pub symbol: String,
    /// 该符号的 interface hash。
    pub interface_hash: [u8; 32],
}

/// 链接拓扑 manifest 记录(写回 manifest 面;规范键 `(unit, symbol)` 字节序)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkManifestRecord {
    /// 变体 key。
    pub variant_key: String,
    /// 组合产物 interface hash(重算写回,RXS-0356 L2)。
    pub interface_hash: [u8; 32],
    /// 产物 digest(拓扑规范字节的域分离压缩;审计可回放纵)。
    pub artifact_digest: [u8; 32],
    /// 成员表(规范序)。
    pub members: Vec<LinkedMember>,
}

/// 组合链接产物(确定性;同输入双构建逐字段相等)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkedArtifact {
    /// 变体 key。
    pub variant_key: String,
    /// 合并接口 canonical bytes(unit/符号规范序;interface hash 的输入材料)。
    pub interface_canonical: Vec<u8>,
    /// 链接后 interface hash(`reflection::interface_hash_of` 重算;写回 manifest,
    /// RXS-0306 定义面不变)。
    pub interface_hash: [u8; 32],
    /// 链接拓扑规范字节(版本前缀起始;审计可回放纵——拓扑 → digest 重算相等)。
    pub canonical: Vec<u8>,
    /// 产物 digest(`SHA-256(完整 canonical 字节)`,permutation.rs 同律;进 DDC 面)。
    pub artifact_digest: [u8; 32],
    /// manifest 拓扑记录。
    pub manifest: LinkManifestRecord,
}

/// 组合链接(RXS-0356 L1/L2/L3):拓扑声明 unit 集内做函数级符号合并——
///
/// 1. 拓扑核验(显式性):unit 缺失/输入重名/unit 内符号重名 → fail-closed;
/// 2. 符号解析 + 类型契约核验:被引符号缺失 → [`LinkError::MissingSymbol`];
///    期望 interface hash ≠ 实际导出 → [`LinkError::InterfaceMismatch`];
/// 3. 循环链接检测(符号级依赖图,规范化迭代序)→ [`LinkError::CyclicLink`];
/// 4. 合并物化:接口 canonical(unit/符号字节序)→ interface hash 重算
///    (reflection 机构单一事实源);拓扑规范字节 → 产物 digest。
///
/// 确定性:输入 `units` 切片次序与 `topology.units` 声明序**不进产物**(内部
/// 规范化排序);同输入双构建逐字段相等。
///
/// # Errors
/// 见 [`LinkError`] 各变体(fail-closed,不设 UB)。
//@ spec: RXS-0356
pub fn link_shader_library(
    units: &[LibraryUnit],
    topology: &LinkTopology,
) -> Result<LinkedArtifact, LinkError> {
    // ── ① 拓扑核验:输入 unit 表重名 → 歧义 fail-closed;拓扑声明 unit 须在场。
    let mut by_name: BTreeMap<&str, &LibraryUnit> = BTreeMap::new();
    for u in units {
        if by_name.insert(u.name.as_str(), u).is_some() {
            return Err(LinkError::DuplicateUnit {
                unit: u.name.clone(),
            });
        }
    }
    let mut selected: Vec<&LibraryUnit> = Vec::with_capacity(topology.units.len());
    for name in &topology.units {
        let Some(u) = by_name.get(name.as_str()) else {
            return Err(LinkError::MissingUnit { unit: name.clone() });
        };
        selected.push(u);
    }
    selected.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    selected.dedup_by(|a, b| a.name == b.name);

    // ── ② 符号表(链接域 = 拓扑声明 unit 集;unit 内符号重名 → 歧义)──
    // 键 (unit, symbol) → (导出符号, 其 interface hash);BTreeMap 迭代序 = 规范序。
    let mut scope: BTreeMap<(String, String), (&ExportedSymbol, [u8; 32])> = BTreeMap::new();
    for u in &selected {
        let mut seen: BTreeMap<&str, ()> = BTreeMap::new();
        for e in &u.exports {
            if seen.insert(e.name.as_str(), ()).is_some() {
                return Err(LinkError::DuplicateSymbol {
                    unit: u.name.clone(),
                    symbol: e.name.clone(),
                });
            }
            let hash = crate::reflection::interface_hash_of(&e.interface_canonical);
            scope.insert((u.name.clone(), e.name.clone()), (e, hash));
        }
    }
    // 入口符号必须在域内。
    let entry_key = (topology.entry_unit.clone(), topology.entry_symbol.clone());
    if !scope.contains_key(&entry_key) {
        return Err(LinkError::MissingSymbol {
            unit: topology.entry_unit.clone(),
            symbol: topology.entry_symbol.clone(),
        });
    }
    // ── ③ 类型契约核验(引用目标在域 + 期望 hash == 实际)──
    for ((from_unit, from_symbol), (e, _)) in &scope {
        for req in &e.requires {
            let key = (req.unit.clone(), req.symbol.clone());
            let Some((_, actual)) = scope.get(&key) else {
                return Err(LinkError::MissingSymbol {
                    unit: req.unit.clone(),
                    symbol: req.symbol.clone(),
                });
            };
            if *actual != req.expected_interface {
                return Err(LinkError::InterfaceMismatch {
                    from_unit: from_unit.clone(),
                    from_symbol: from_symbol.clone(),
                    to_unit: req.unit.clone(),
                    to_symbol: req.symbol.clone(),
                });
            }
        }
    }
    // ── ④ 循环链接检测(符号级依赖图;三色标记,规范化迭代序)──
    let mut marks: BTreeMap<(String, String), bool> = BTreeMap::new(); // true = Done
    let mut stack: Vec<(String, String)> = Vec::new();
    for key in scope.keys() {
        if !marks.contains_key(key) {
            visit_link_node(key, &scope, &mut marks, &mut stack)?;
        }
    }
    // ── ⑤ 合并物化:接口 canonical → interface hash;拓扑 canonical → 产物 digest ──
    let mut iw = CanonW::new();
    iw.u32v(selected.len() as u32);
    for u in &selected {
        iw.strv(&u.name);
        let mut exports: Vec<&ExportedSymbol> = u.exports.iter().collect();
        exports.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        iw.u32v(exports.len() as u32);
        for e in &exports {
            iw.strv(&e.name);
            iw.u32v(e.interface_canonical.len() as u32);
            iw.bytes(&e.interface_canonical);
        }
    }
    let interface_canonical = iw.buf;
    let interface_hash = crate::reflection::interface_hash_of(&interface_canonical);

    let mut w = CanonW::new();
    w.bytes(LINK_DOMAIN_V1);
    w.strv(&topology.variant_key);
    w.strv(&topology.entry_unit);
    w.strv(&topology.entry_symbol);
    w.u32v(selected.len() as u32);
    for u in &selected {
        w.strv(&u.name);
        let mut exports: Vec<&ExportedSymbol> = u.exports.iter().collect();
        exports.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        w.u32v(exports.len() as u32);
        for e in &exports {
            w.strv(&e.name);
            w.u32v(e.interface_canonical.len() as u32);
            w.bytes(&e.interface_canonical);
            let mut reqs: Vec<&SymbolRequirement> = e.requires.iter().collect();
            reqs.sort_by(|a, b| {
                (a.unit.as_bytes(), a.symbol.as_bytes())
                    .cmp(&(b.unit.as_bytes(), b.symbol.as_bytes()))
            });
            w.u32v(reqs.len() as u32);
            for r in reqs {
                w.strv(&r.unit);
                w.strv(&r.symbol);
                w.bytes(&r.expected_interface);
            }
        }
    }
    let canonical = w.buf;
    let artifact_digest = sha256::digest(&canonical);

    let members: Vec<LinkedMember> = scope
        .iter()
        .map(|((unit, symbol), (_, hash))| LinkedMember {
            unit: unit.clone(),
            symbol: symbol.clone(),
            interface_hash: *hash,
        })
        .collect();
    Ok(LinkedArtifact {
        variant_key: topology.variant_key.clone(),
        interface_canonical,
        interface_hash,
        canonical,
        artifact_digest,
        manifest: LinkManifestRecord {
            variant_key: topology.variant_key.clone(),
            interface_hash,
            artifact_digest,
            members,
        },
    })
}

/// 循环检测 DFS(三色;`marks` 缺省 = 未访问,false = 在栈,true = 完成)。
fn visit_link_node(
    key: &(String, String),
    scope: &BTreeMap<(String, String), (&ExportedSymbol, [u8; 32])>,
    marks: &mut BTreeMap<(String, String), bool>,
    stack: &mut Vec<(String, String)>,
) -> Result<(), LinkError> {
    marks.insert(key.clone(), false);
    stack.push(key.clone());
    let Some((e, _)) = scope.get(key) else {
        return Ok(()); // 不可达:调用点以 scope.keys() 为域;防御性早返。
    };
    let mut deps: Vec<(String, String)> = e
        .requires
        .iter()
        .map(|r| (r.unit.clone(), r.symbol.clone()))
        .collect();
    deps.sort();
    deps.dedup();
    for dep in deps {
        match marks.get(&dep) {
            Some(false) => {
                // 回边 → 环:自栈中 dep 首现处起到栈顶再回 dep(确定性展示)。
                let start = stack.iter().position(|k| *k == dep).unwrap_or(0);
                let mut cycle: Vec<String> = stack[start..]
                    .iter()
                    .map(|(u, s)| format!("{u}::{s}"))
                    .collect();
                cycle.push(format!("{}::{}", dep.0, dep.1));
                return Err(LinkError::CyclicLink {
                    cycle: cycle.join(" -> "),
                });
            }
            Some(true) => {}
            None => visit_link_node(&dep, scope, marks, stack)?,
        }
    }
    stack.pop();
    marks.insert(key.clone(), true);
    Ok(())
}

// ═══════════════════════ manifest 拓扑记录 JSON(RXS-0356 L2) ═══════════════════════

/// JSON 串转义(与 reflection.rs 同一防御面)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 链接拓扑 manifest 记录 → 确定性 JSON(键序固定、UTF-8、LF 行尾;不含路径/
/// 时间戳,RXS-0305 禁用面同律)。「写回 manifest」的产物面;M85 manifest 本体
/// (RXS-0317)0-byte,接线归 CI 门代理。
pub fn to_manifest_json(record: &LinkManifestRecord) -> String {
    let compiler_version = env!("CARGO_PKG_VERSION");
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"schema\": \"rurix.shader-library-link.v1\",\n");
    s.push_str("  \"schema_version\": 1,\n");
    s.push_str("  \"compiler\": \"rurixc\",\n");
    s.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(compiler_version)
    ));
    s.push_str("  \"edition\": \"Rx0\",\n");
    s.push_str(&format!(
        "  \"variant_key\": \"{}\",\n",
        json_escape(&record.variant_key)
    ));
    s.push_str(&format!(
        "  \"interface_hash\": \"{}\",\n",
        sha256::hex(&record.interface_hash)
    ));
    s.push_str(&format!(
        "  \"artifact_digest\": \"{}\",\n",
        sha256::hex(&record.artifact_digest)
    ));
    if record.members.is_empty() {
        s.push_str("  \"members\": []\n");
    } else {
        s.push_str("  \"members\": [\n");
        for (k, m) in record.members.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"unit\": \"{}\", \"symbol\": \"{}\", \"interface_hash\": \"{}\"}}{}\n",
                json_escape(&m.unit),
                json_escape(&m.symbol),
                sha256::hex(&m.interface_hash),
                if k + 1 == record.members.len() {
                    ""
                } else {
                    ","
                },
            ));
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

// ═══════════════════════ 变体工程级总预算(RXS-0356 L4) ═══════════════════════

/// 变体审计 per-entry 行(工程级:`module::pass` 归属经 `name` 承载;计数由上游
/// permutation 报告派生,axis 贡献分解归 permutation.rs 既有报告面)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VariantAuditEntry {
    /// 归属标识(`module::pass` 形态约定)。
    pub name: String,
    /// 组合全集基数(∏|axis|,permutation 求解产物)。
    pub enumerated: u128,
    /// 被 forbid 裁剪数。
    pub pruned: u128,
    /// 合法发射数(manifest 声明变体数)。
    pub emitted: u128,
    /// DDC 命中数(命中率分解的分子;分母 = `emitted`,整数不浮点)。
    pub ddc_hits: u128,
    /// workload 引用位(死变体 = manifest 声明而无引用;只报告不自动删)。
    pub referenced: bool,
}

/// 变体审计违例(装配期硬失败 = typed `Err`,非警告、非 panic;库面诊断不占
/// RX 码——诊断码实现期从工具段按实际可达类别领取,不预造,RXS-0356 L4)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VariantAuditError {
    /// 工程级总预算超限(合计 emitted > 声明总预算;硬失败)。
    TotalBudgetExceeded {
        /// 工程级合计 emitted。
        total_emitted: u128,
        /// 声明的工程级总预算。
        total_budget: u32,
    },
    /// 审计恒等式违例(`enumerated != pruned + emitted`;输入完整性 fail-closed)。
    InconsistentIdentity {
        /// 违例行归属标识。
        entry: String,
    },
    /// 声明外产物(DDC 命中数超出行 emitted;manifest 声明变体 ∪ DDC 产物
    /// 不闭合,fail-closed)。
    UndeclaredArtifact {
        /// 违例行归属标识。
        entry: String,
    },
}

impl std::fmt::Display for VariantAuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantAuditError::TotalBudgetExceeded {
                total_emitted,
                total_budget,
            } => write!(
                f,
                "变体工程级总预算超限:合计 emitted {total_emitted} > 总预算 {total_budget}\
                 (装配期硬失败,非警告,RXS-0356 L4;conformance `variant_budget_exceeded.rx` 同族)"
            ),
            VariantAuditError::InconsistentIdentity { entry } => write!(
                f,
                "变体审计恒等式违例:`{entry}` 的 enumerated != pruned + emitted(输入完整性 fail-closed,RXS-0356 L4)"
            ),
            VariantAuditError::UndeclaredArtifact { entry } => write!(
                f,
                "变体审计闭合违例:`{entry}` 的 DDC 命中超出 manifest 声明变体(声明外产物,fail-closed,RXS-0356 L4)"
            ),
        }
    }
}

impl std::error::Error for VariantAuditError {}

/// 变体审计报告(`rurix.variant-audit-report.v1`;死变体清单 = 报告字段,**只
/// 报告不自动删**——删除是人的决定,RXS-0356 L4 逐字)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VariantAuditReport {
    /// per-entry 行(规范键 = 归属标识字节序)。
    pub entries: Vec<VariantAuditEntry>,
    /// 工程级合计 enumerated。
    pub total_enumerated: u128,
    /// 工程级合计 pruned。
    pub total_pruned: u128,
    /// 工程级合计 emitted(总预算门的判定量)。
    pub total_emitted: u128,
    /// 工程级合计 DDC 命中。
    pub total_ddc_hits: u128,
    /// 死变体清单(声明而无 workload 引用;规范序)。
    pub dead_variants: Vec<String>,
    /// 声明的工程级总预算。
    pub total_budget: u32,
}

/// 工程级变体预算门 + 审计报告(RXS-0356 L4):
///
/// 1. 行规范化(归属标识字节序;输入次序不进产物);
/// 2. 逐行断言审计恒等式 `enumerated == pruned + emitted` 与闭合性
///    (`ddc_hits <= emitted`);违例 → typed `Err`(fail-closed);
/// 3. 工程级合计;**总预算门**:`total_emitted > total_budget` →
///    [`VariantAuditError::TotalBudgetExceeded`](装配期硬失败,上限含等号);
/// 4. 死变体清单(`referenced == false`;只报告)。
///
/// # Errors
/// 见 [`VariantAuditError`] 各变体。
//@ spec: RXS-0356
pub fn audit_project_variants(
    mut entries: Vec<VariantAuditEntry>,
    total_budget: u32,
) -> Result<VariantAuditReport, VariantAuditError> {
    entries.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let mut total_enumerated: u128 = 0;
    let mut total_pruned: u128 = 0;
    let mut total_emitted: u128 = 0;
    let mut total_ddc_hits: u128 = 0;
    let mut dead_variants: Vec<String> = Vec::new();
    for e in &entries {
        if e.enumerated != e.pruned + e.emitted {
            return Err(VariantAuditError::InconsistentIdentity {
                entry: e.name.clone(),
            });
        }
        if e.ddc_hits > e.emitted {
            return Err(VariantAuditError::UndeclaredArtifact {
                entry: e.name.clone(),
            });
        }
        total_enumerated += e.enumerated;
        total_pruned += e.pruned;
        total_emitted += e.emitted;
        total_ddc_hits += e.ddc_hits;
        if !e.referenced {
            dead_variants.push(e.name.clone());
        }
    }
    if total_emitted > u128::from(total_budget) {
        return Err(VariantAuditError::TotalBudgetExceeded {
            total_emitted,
            total_budget,
        });
    }
    Ok(VariantAuditReport {
        entries,
        total_enumerated,
        total_pruned,
        total_emitted,
        total_ddc_hits,
        dead_variants,
        total_budget,
    })
}

/// 审计报告 → 确定性 JSON(`rurix.variant-audit-report.v1`;键序固定、UTF-8、
/// LF 行尾、整数不浮点;无路径/时间戳,RXS-0305 禁用面同律)。双次生成逐字节
/// 相等(golden 锚定面归 CI 门代理)。
pub fn to_audit_json(report: &VariantAuditReport) -> String {
    let compiler_version = env!("CARGO_PKG_VERSION");
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{VARIANT_AUDIT_SCHEMA_ID}\",\n"));
    s.push_str(&format!(
        "  \"schema_version\": {VARIANT_AUDIT_SCHEMA_VERSION},\n"
    ));
    s.push_str("  \"compiler\": \"rurixc\",\n");
    s.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(compiler_version)
    ));
    s.push_str("  \"edition\": \"Rx0\",\n");
    s.push_str(&format!(
        "  \"total_enumerated\": {},\n",
        report.total_enumerated
    ));
    s.push_str(&format!("  \"total_pruned\": {},\n", report.total_pruned));
    s.push_str(&format!("  \"total_emitted\": {},\n", report.total_emitted));
    s.push_str(&format!(
        "  \"total_ddc_hits\": {},\n",
        report.total_ddc_hits
    ));
    s.push_str(&format!("  \"total_budget\": {},\n", report.total_budget));
    if report.dead_variants.is_empty() {
        s.push_str("  \"dead_variants\": [],\n");
    } else {
        let dead = report
            .dead_variants
            .iter()
            .map(|d| format!("\"{}\"", json_escape(d)))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("  \"dead_variants\": [{dead}],\n"));
    }
    if report.entries.is_empty() {
        s.push_str("  \"entries\": []\n");
    } else {
        s.push_str("  \"entries\": [\n");
        for (k, e) in report.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"name\": \"{}\", \"enumerated\": {}, \"pruned\": {}, \"emitted\": {}, \"ddc_hits\": {}, \"referenced\": {}}}{}\n",
                json_escape(&e.name),
                e.enumerated,
                e.pruned,
                e.emitted,
                e.ddc_hits,
                e.referenced,
                if k + 1 == report.entries.len() { "" } else { "," },
            ));
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

// ═══════════════════════ 单测(链接确定性/接口 hash/预算门) ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 三 unit 基准库(材质 × lighting × pass 入口;interface canonical = 确定性
    /// fixture 字节——上游 reflection 提取律产物的替身,链接面只消费字节)。
    fn sample_units() -> Vec<LibraryUnit> {
        let lighting_eval_iface = b"iface:lighting::eval:v1".to_vec();
        let material_shade_iface = b"iface:material::shade:v1".to_vec();
        let pass_main_iface = b"iface:pass::main:v1".to_vec();
        vec![
            LibraryUnit {
                name: "lighting".to_owned(),
                exports: vec![ExportedSymbol {
                    name: "eval".to_owned(),
                    interface_canonical: lighting_eval_iface.clone(),
                    requires: Vec::new(),
                }],
            },
            LibraryUnit {
                name: "material".to_owned(),
                exports: vec![ExportedSymbol {
                    name: "shade".to_owned(),
                    interface_canonical: material_shade_iface,
                    requires: vec![SymbolRequirement {
                        unit: "lighting".to_owned(),
                        symbol: "eval".to_owned(),
                        expected_interface: crate::reflection::interface_hash_of(
                            &lighting_eval_iface,
                        ),
                    }],
                }],
            },
            LibraryUnit {
                name: "pass".to_owned(),
                exports: vec![ExportedSymbol {
                    name: "main".to_owned(),
                    interface_canonical: pass_main_iface,
                    requires: vec![SymbolRequirement {
                        unit: "material".to_owned(),
                        symbol: "shade".to_owned(),
                        expected_interface: crate::reflection::interface_hash_of(
                            b"iface:material::shade:v1",
                        ),
                    }],
                }],
            },
        ]
    }

    fn sample_topology() -> LinkTopology {
        LinkTopology {
            variant_key: "FOG=true;QUALITY=med".to_owned(),
            units: vec![
                "pass".to_owned(),
                "material".to_owned(),
                "lighting".to_owned(),
            ],
            entry_unit: "pass".to_owned(),
            entry_symbol: "main".to_owned(),
        }
    }

    /// 链接确定性(RXS-0356 L2):同输入双构建逐字段相等(interface hash / 拓扑
    /// canonical / 产物 digest / manifest 记录);**输入顺序扰动不变式**——units
    /// 切片逆序 + 拓扑 units 声明逆序 → 产物逐字段相等(声明序不进产物)。
    //@ spec: RXS-0356
    #[test]
    fn link_deterministic_and_order_invariant() {
        let units = sample_units();
        let topo = sample_topology();
        let a = link_shader_library(&units, &topo).expect("合法链接");
        let b = link_shader_library(&units, &topo).expect("双构建");
        assert_eq!(a, b, "同输入双构建逐字段相等");
        // 输入扰动:units 逆序 + 拓扑声明逆序。
        let mut rev_units = sample_units();
        rev_units.reverse();
        let mut rev_topo = sample_topology();
        rev_topo.units.reverse();
        let c = link_shader_library(&rev_units, &rev_topo).expect("扰动输入链接");
        assert_eq!(a, c, "输入顺序扰动不进产物(规范化序)");
        // manifest 记录面:成员规范序 (unit, symbol);拓扑可回放(记录 → 同 digest)。
        let names: Vec<(&str, &str)> = a
            .manifest
            .members
            .iter()
            .map(|m| (m.unit.as_str(), m.symbol.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("lighting", "eval"),
                ("material", "shade"),
                ("pass", "main")
            ],
            "manifest 成员规范键 (unit, symbol) 字节序"
        );
        assert_eq!(a.manifest.interface_hash, a.interface_hash);
        assert_eq!(a.manifest.artifact_digest, a.artifact_digest);
        // manifest JSON 双次逐字节相等 + LF/无 CR。
        let (j1, j2) = (to_manifest_json(&a.manifest), to_manifest_json(&a.manifest));
        assert_eq!(j1, j2);
        assert!(j1.ends_with("}\n") && !j1.contains('\r'));
        assert!(j1.contains("\"schema\": \"rurix.shader-library-link.v1\""));
        assert!(j1.contains("\"variant_key\": \"FOG=true;QUALITY=med\""));
    }

    /// interface hash 稳定性(RXS-0356 L2):产物 interface_hash == 对
    /// `interface_canonical` 以 M31 域分离律(`rurix.shader-interface.v1\0` 前缀)
    /// 独立重算的值(单一事实源复算交叉锚);接口字节微扰 → interface hash 与产物
    /// digest 双翻;拓扑微扰(entry 替换/变体 key 替换)→ 产物 digest 翻、审计
    /// 可回放性保持(同拓扑重算 digest 相等)。
    //@ spec: RXS-0356
    #[test]
    fn interface_hash_stability_and_replay() {
        let units = sample_units();
        let topo = sample_topology();
        let a = link_shader_library(&units, &topo).unwrap();
        // 独立重算:域前缀字面 + interface canonical(不经被测函数)。
        let mut h = sha256::Sha256::new();
        h.update(b"rurix.shader-interface.v1\0");
        h.update(&a.interface_canonical);
        assert_eq!(
            a.interface_hash,
            h.finalize(),
            "interface hash = M31 域分离律独立重算值(RXS-0306 定义面不变)"
        );
        assert_eq!(a.artifact_digest, sha256::digest(&a.canonical));
        assert!(a.canonical.starts_with(b"rurix.shader-library-link.v1\0"));
        // 接口字节微扰 → 双 hash 翻。
        let mut units2 = sample_units();
        units2[0].exports[0].interface_canonical.push(0xFF);
        let mut reqs_fix = units2[1].exports[0].requires.clone();
        reqs_fix[0].expected_interface =
            crate::reflection::interface_hash_of(&units2[0].exports[0].interface_canonical);
        units2[1].exports[0].requires = reqs_fix;
        let b = link_shader_library(&units2, &topo).unwrap();
        assert_ne!(
            a.interface_hash, b.interface_hash,
            "接口微扰 interface hash 必翻"
        );
        assert_ne!(a.artifact_digest, b.artifact_digest);
        // 拓扑微扰(变体 key 替换)→ 产物 digest 翻,interface hash 不动(接口集未变)。
        let mut topo2 = sample_topology();
        topo2.variant_key = "FOG=false;QUALITY=med".to_owned();
        let c = link_shader_library(&units, &topo2).unwrap();
        assert_ne!(a.artifact_digest, c.artifact_digest, "拓扑微扰 digest 必翻");
        assert_eq!(
            a.interface_hash, c.interface_hash,
            "同接口集换变体 key:interface hash 不变(拓扑 ≠ 接口事实)"
        );
        // 可回放:同拓扑重算 digest 相等(审计回放律)。
        let replay = link_shader_library(&units, &topo).unwrap();
        assert_eq!(replay.artifact_digest, a.artifact_digest);
    }

    /// 链接合法性 fail-closed 四红(RXS-0356 L3):符号缺失(unit/符号两级)/
    /// 类型契约失配(接口失配)/循环链接 → typed `Err`(编译期确定性诊断,无
    /// 最近邻回退,非 panic);unit 内符号重名与输入 unit 重名同族拒。
    //@ spec: RXS-0356
    #[test]
    fn link_fail_closed_reds() {
        let units = sample_units();
        // 符号缺失:拓扑声明未知 unit(禁隐式全图链接)。
        let mut topo = sample_topology();
        topo.units.push("ghost".to_owned());
        assert_eq!(
            link_shader_library(&units, &topo).expect_err("未知 unit 须拒"),
            LinkError::MissingUnit {
                unit: "ghost".to_owned()
            }
        );
        // 符号缺失:入口不在域。
        let mut topo = sample_topology();
        topo.entry_symbol = "ghost_main".to_owned();
        assert_eq!(
            link_shader_library(&units, &topo).expect_err("入口缺失须拒"),
            LinkError::MissingSymbol {
                unit: "pass".to_owned(),
                symbol: "ghost_main".to_owned()
            }
        );
        // 符号缺失:引用目标不在拓扑声明 unit 集内(引用域外 = 缺失)。
        let mut topo = sample_topology();
        topo.units.retain(|u| u != "lighting");
        let err = link_shader_library(&units, &topo).expect_err("域外引用须拒");
        assert_eq!(
            err,
            LinkError::MissingSymbol {
                unit: "lighting".to_owned(),
                symbol: "eval".to_owned()
            }
        );
        // 类型契约失配:期望 hash ≠ 实际导出。
        let mut bad = sample_units();
        bad[1].exports[0].requires[0].expected_interface = [0xEE; 32];
        let err = link_shader_library(&bad, &sample_topology()).expect_err("失配须拒");
        assert_eq!(
            err,
            LinkError::InterfaceMismatch {
                from_unit: "material".to_owned(),
                from_symbol: "shade".to_owned(),
                to_unit: "lighting".to_owned(),
                to_symbol: "eval".to_owned(),
            }
        );
        assert!(err.to_string().contains("类型契约失配"));
        // 循环链接:material::shade ⇄ lighting::eval 互引。
        let mut cyclic = sample_units();
        cyclic[0].exports[0].requires.push(SymbolRequirement {
            unit: "material".to_owned(),
            symbol: "shade".to_owned(),
            expected_interface: crate::reflection::interface_hash_of(b"iface:material::shade:v1"),
        });
        let err = link_shader_library(&cyclic, &sample_topology()).expect_err("循环须拒");
        let LinkError::CyclicLink { cycle } = &err else {
            panic!("须为 CyclicLink,实得 {err:?}");
        };
        assert!(
            cycle.contains("material::shade") && cycle.contains("lighting::eval"),
            "环路径须含两符号: {cycle}"
        );
        assert!(cycle.contains(" -> "), "环路径展示形态: {cycle}");
        // unit 内符号重名。
        let mut dup = sample_units();
        let dup_export = dup[0].exports[0].clone();
        dup[0].exports.push(dup_export);
        assert!(matches!(
            link_shader_library(&dup, &sample_topology()),
            Err(LinkError::DuplicateSymbol { .. })
        ));
        // 输入 unit 重名。
        let mut dupu = sample_units();
        let extra = dupu[0].clone();
        dupu.push(extra);
        assert!(matches!(
            link_shader_library(&dupu, &sample_topology()),
            Err(LinkError::DuplicateUnit { .. })
        ));
    }

    /// 变体工程级总预算(RXS-0356 L4):超限 → 装配期硬失败 typed `Err`
    /// (`TotalBudgetExceeded`,非警告、非 panic);上限含等号 GREEN;conformance
    /// 锚定语料 `variant_budget_exceeded.rx` 消费(可消费不可改)。
    //@ spec: RXS-0356
    #[test]
    fn variant_budget_exceeded_red_and_boundary() {
        let entries = vec![
            VariantAuditEntry {
                name: "material::gbuffer".to_owned(),
                enumerated: 6,
                pruned: 1,
                emitted: 5,
                ddc_hits: 3,
                referenced: true,
            },
            VariantAuditEntry {
                name: "lighting::deferred".to_owned(),
                enumerated: 4,
                pruned: 0,
                emitted: 4,
                ddc_hits: 4,
                referenced: true,
            },
        ];
        // 合计 emitted = 9;预算 8 → 硬失败(typed Err,逐字段锚)。
        let err = audit_project_variants(entries.clone(), 8).expect_err("超预算须硬失败");
        assert_eq!(
            err,
            VariantAuditError::TotalBudgetExceeded {
                total_emitted: 9,
                total_budget: 8,
            }
        );
        let text = err.to_string();
        assert!(text.contains("装配期硬失败"), "诊断须声明硬失败: {text}");
        // 上限含等号 GREEN(预算 9)。
        let report = audit_project_variants(entries, 9).expect("预算 = 合计须放行");
        assert_eq!(report.total_emitted, 9);
        assert_eq!(report.total_budget, 9);
        // conformance 锚定语料消费。
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/gpu_driven_submit/reject/variant_budget_exceeded.rx");
        let anchor = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读锚定语料 {}: {e}", path.display()));
        assert!(anchor.contains("RXS-0356"), "锚定语料须携条款号");
        assert!(anchor.contains("g9.p1.m107.shader_library_ir_link"));
    }

    /// 零变体/单变体边界(RXS-0356 L4):空工程 + 预算 0 → GREEN(恒等式
    /// 0 == 0 + 0 工程级成立);单变体 emitted 1 + 预算 1 GREEN、预算 0 RED。
    //@ spec: RXS-0356
    #[test]
    fn variant_budget_zero_and_single_edges() {
        let report = audit_project_variants(Vec::new(), 0).expect("空工程 + 预算 0 须 GREEN");
        assert_eq!(report.total_enumerated, 0);
        assert_eq!(report.total_pruned, 0);
        assert_eq!(report.total_emitted, 0, "恒等式 0 == 0 + 0");
        assert!(report.entries.is_empty());
        assert!(report.dead_variants.is_empty());
        let single = vec![VariantAuditEntry {
            name: "m::p".to_owned(),
            enumerated: 1,
            pruned: 0,
            emitted: 1,
            ddc_hits: 0,
            referenced: true,
        }];
        assert!(audit_project_variants(single.clone(), 1).is_ok());
        assert_eq!(
            audit_project_variants(single, 0).expect_err("单变体超零预算须 RED"),
            VariantAuditError::TotalBudgetExceeded {
                total_emitted: 1,
                total_budget: 0,
            }
        );
    }

    /// 审计恒等式 + 死变体报告 + 闭合性(RXS-0356 L4):工程级
    /// `enumerated == pruned + emitted`;死变体(声明而无引用)列入报告且**不自动
    /// 删**(entries 保留);恒等式违例/声明外产物(ddc_hits > emitted)→ typed
    /// `Err`;报告 JSON 确定性 + schema 字面 + 行规范化(输入乱序不进产物)。
    //@ spec: RXS-0356
    #[test]
    fn variant_audit_identity_dead_and_json() {
        let entries = vec![
            VariantAuditEntry {
                name: "z_pass::late".to_owned(),
                enumerated: 2,
                pruned: 1,
                emitted: 1,
                ddc_hits: 1,
                referenced: false, // 死变体携带行
            },
            VariantAuditEntry {
                name: "a_pass::early".to_owned(),
                enumerated: 8,
                pruned: 2,
                emitted: 6,
                ddc_hits: 5,
                referenced: true,
            },
        ];
        let report = audit_project_variants(entries.clone(), 16).expect("合法审计");
        assert_eq!(report.total_enumerated, 10);
        assert_eq!(report.total_pruned, 3);
        assert_eq!(report.total_emitted, 7);
        assert_eq!(
            report.total_enumerated,
            report.total_pruned + report.total_emitted,
            "工程级恒等式"
        );
        // 死变体只报告不自动删:清单列出,entries 仍保留该行。
        assert_eq!(report.dead_variants, vec!["z_pass::late".to_owned()]);
        assert_eq!(report.entries.len(), 2, "死变体不得自动删");
        // 行规范化:输入乱序 → 报告规范序。
        let mut shuffled = entries.clone();
        shuffled.reverse();
        let report2 = audit_project_variants(shuffled, 16).unwrap();
        assert_eq!(report, report2, "输入次序不进报告");
        assert_eq!(report.entries[0].name, "a_pass::early", "归属标识字节序");
        // JSON:确定性 + schema 字面 + 死变体字段 + 整数不浮点。
        let (j1, j2) = (to_audit_json(&report), to_audit_json(&report));
        assert_eq!(j1, j2, "报告 JSON 双次逐字节相等");
        assert!(j1.ends_with("}\n") && !j1.contains('\r'));
        assert!(j1.contains("\"schema\": \"rurix.variant-audit-report.v1\""));
        assert!(j1.contains("\"dead_variants\": [\"z_pass::late\"]"));
        assert!(j1.contains("\"total_emitted\": 7"));
        // 恒等式违例 → typed Err。
        let mut bad = entries.clone();
        bad[0].pruned = 0;
        assert_eq!(
            audit_project_variants(bad, 16).expect_err("恒等式违例须拒"),
            VariantAuditError::InconsistentIdentity {
                entry: "z_pass::late".to_owned()
            }
        );
        // 声明外产物 → typed Err。
        let mut bad = entries.clone();
        bad[1].ddc_hits = 7; // > emitted 6
        assert_eq!(
            audit_project_variants(bad, 16).expect_err("声明外产物须拒"),
            VariantAuditError::UndeclaredArtifact {
                entry: "a_pass::early".to_owned()
            }
        );
    }
}
