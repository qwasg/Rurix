//! `permutation` — M29 shader permutation 域、canonical key、裁剪/预算/报告
//! (G8.2 硬门 `g8.p0.m29.shader_permutation`;RFC-0019 §4.3;
//! spec/rendering_platform.md RXS-0308~0310)。纯 host、safe。
//!
//! 四子面:
//!
//! - **域声明闭集**(RXS-0308):entry 级 `#[permutation(...)]` 属性(多条可叠加),
//!   子句闭集 = `axis(NAME, bool|enum(m0,..)|int(LO,HI))` / `forbid(NAME = 值, ..)`
//!   (等式合取封闭子集)/ `budget(N)`(每 entry 至多一条)。违例 = 编译期确定性
//!   诊断 **RX3019** `shader.permutation_domain_invalid`(typeck 段,经
//!   [`crate::shader_stages::check`] 挂接);`#[permutation]` 附着非着色入口函数
//!   同码拒。泛型着色函数不产 entry(RXS-0304),其上标注不参与求解。
//! - **canonical key 与 domain digest**(RXS-0309):axis 按名字节序;二进制 key =
//!   `"rurix.permutation-key.v1\0"` + 逐 axis `(name, type_tag u32 LE, 值规范编码)`;
//!   字符串 key = `NAME=value;NAME=value`;`permutation_domain_digest =
//!   SHA-256("rurix.permutation-domain.v1\0" || 去前缀段)`(本实现 canonical 字节
//!   自带前缀,digest = SHA-256(完整 canonical 字节),与 spec 定义逐字节等价);
//!   **空域恒** `SHA-256("rurix.permutation-domain-empty.v1\0")`(RXS-0304 空编码
//!   0 漂移)。声明序/路径/进程因素不进任何 key 或 digest;组合→key 单射(全轴
//!   覆盖 + 定界编码,by construction)。
//! - **裁剪·预算·报告**(RXS-0310):`enumerated = ∏|axis|`(整数算术,先算不物化)
//!   → 超 budget 硬失败(**RX7023** `toolchain.permutation_budget_exceeded`,CLI
//!   `--permutation-budget=N` 覆盖 attr 声明值,上限含等号)+ axis contribution
//!   report;否则笛卡尔枚举 → 逐 forbid 行裁剪(`pruned`)→ 余集 `emitted`;
//!   恒等式 `enumerated == pruned + emitted` 是结构保证也是报告断言字段。
//!   `--permutation-select=KEY`:KEY ∉ 合法集 = RX3019 类确定性错误(禁最接近
//!   回退);选中后 reflection `variant_key = KEY`、`permutation_domain_digest`
//!   真值化,`pipeline_key` 随之分裂(RXS-0306 preimage 既含二字段,零新接缝)。
//!
//! SHA-256 复用 `rurix-pkg` 手写实现(RXS-0306 同源);编码沿 RXS-0305 CanonW 律
//! (u32 LE 定宽、length-prefix UTF-8 字符串、u32 计数列表)。
//!
//! v1 诚实边界:
//! - int 轴区间界以 meta 整数字面量表达(非负;负界属 parser meta 字面量子集外,
//!   维持解析错误——模型层 `i64` 编码已预留,放开走 RXS-0308 加性修订);
//! - budget 判定在 `--emit=permutations` 与 `--permutation-select` 求解路径执行
//!   (组合物化发生处);普通 host EXE 编译只做 RX3019 域校验,不物化组合表;
//! - per-variant body specialization codegen 不在本条款范围(RXS-0310 冻结注)。

use crate::ast::{self, FnColor, LitKind, MetaInner, MetaKind, ShaderStage};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::span::Span;
use rurix_pkg::sha256;

/// RX3019(RXS-0308;permutation 域声明违例,typeck 段)。
pub const E_PERMUTATION_DOMAIN: ErrorCode = ErrorCode(3019);
/// RX7023(RXS-0310;permutation 预算超限,工具段)。
pub const E_PERMUTATION_BUDGET: ErrorCode = ErrorCode(7023);

/// 组合 canonical key 版本前缀(RXS-0309 二进制形态)。
const PERM_KEY_DOMAIN: &[u8] = b"rurix.permutation-key.v1\0";
/// 规范域字节版本前缀(RXS-0309)。
const PERM_DOMAIN_V1: &[u8] = b"rurix.permutation-domain.v1\0";
/// 空域 digest 定义域(RXS-0304 空编码;与 M31 既有常量逐字节一致,0 漂移)。
const PERM_EMPTY_DOMAIN: &[u8] = b"rurix.permutation-domain-empty.v1\0";

/// 报告产物 schema 标识(RXS-0310 报告律)。
pub const REPORT_SCHEMA_ID: &str = "rurix.permutation-report.v1";
/// 报告 schema 版本。
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// 空 permutation 域的规范 digest(RXS-0304/0309;M31 基线常量,0 字节漂移)。
pub fn empty_domain_digest() -> [u8; 32] {
    sha256::digest(PERM_EMPTY_DOMAIN)
}

// ═══════════════════════ 域模型(RXS-0308) ═══════════════════════

/// axis 值域(三类闭集,RFC-0019 §4.3 冻结面)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AxisDomain {
    /// `bool` —— 组合值 ∈ {`false`, `true`}(规范序)。
    Bool,
    /// `enum(m0, m1, ...)` —— identifier 枚举,≥1 成员;成员序是值域语义的一部分。
    Enum(Vec<String>),
    /// `int(LO, HI)` —— 闭区间整数枚举,`LO <= HI`。
    Int(i64, i64),
}

impl AxisDomain {
    /// 值域基数。
    pub fn size(&self) -> u128 {
        match self {
            AxisDomain::Bool => 2,
            AxisDomain::Enum(ms) => ms.len() as u128,
            AxisDomain::Int(lo, hi) => (hi - lo + 1) as u128,
        }
    }

    /// 规范编码类型标签(RXS-0309:bool=0 / enum=1 / int=2)。
    fn type_tag(&self) -> u32 {
        match self {
            AxisDomain::Bool => 0,
            AxisDomain::Enum(_) => 1,
            AxisDomain::Int(..) => 2,
        }
    }

    /// 报告面类型名。
    fn type_name(&self) -> &'static str {
        match self {
            AxisDomain::Bool => "bool",
            AxisDomain::Enum(_) => "enum",
            AxisDomain::Int(..) => "int",
        }
    }

    /// 规范序全部组合值。
    fn values(&self) -> Vec<PermValue> {
        match self {
            AxisDomain::Bool => vec![PermValue::Bool(false), PermValue::Bool(true)],
            AxisDomain::Enum(ms) => ms.iter().cloned().map(PermValue::Enum).collect(),
            AxisDomain::Int(lo, hi) => (*lo..=*hi).map(PermValue::Int).collect(),
        }
    }
}

/// 组合值(带类型;渲染与规范编码按 axis 类型)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PermValue {
    /// bool 组合值。
    Bool(bool),
    /// enum 组合值(成员名)。
    Enum(String),
    /// int 组合值。
    Int(i64),
}

impl PermValue {
    /// 字符串形态渲染(RXS-0309:bool → `false`/`true`;int → 十进制;enum → 成员名)。
    fn render(&self) -> String {
        match self {
            PermValue::Bool(b) => if *b { "true" } else { "false" }.to_owned(),
            PermValue::Enum(m) => m.clone(),
            PermValue::Int(v) => v.to_string(),
        }
    }
}

/// 一根 axis 声明。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AxisDecl {
    /// axis 名(标识符)。
    pub name: String,
    /// 值域。
    pub domain: AxisDomain,
}

/// 一条 forbid 行(等式合取:组合同时满足行内全部等式即被裁剪,RXS-0308)。
/// 规范化后行内等式按 axis 名字节序排序。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ForbidRow {
    /// 等式表 `(axis 名, 值)`。
    pub equations: Vec<(String, PermValue)>,
    /// 诊断锚点(forbid 子句 span;不进任何 canonical 编码)。
    pub span: Span,
}

/// 一个 entry 的 permutation 域(规范化形态:axis 按名字节序;forbid 行间按
/// 整行规范字节序——声明序不影响任何 key/digest,RXS-0309 确定性律)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PermDomain {
    /// axis 表(名字节序)。
    pub axes: Vec<AxisDecl>,
    /// forbid 行表(规范序)。
    pub forbids: Vec<ForbidRow>,
    /// attr 声明的预算上限(未声明 = None → 求解取 `u32::MAX` 哨兵)。
    pub budget: Option<u32>,
}

/// 域声明违例(RX3019 载体;fail-closed,不产部分报告/反射产物)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PermInvalid {
    /// 诊断上下文({detail} 参数)。
    pub detail: String,
    /// 诊断锚点(违例子句/属性 span)。
    pub span: Span,
}

// ═══════════════════════ canonical 编码(RXS-0305 CanonW 律) ═══════════════════════

/// canonical bytes 写入器(与 reflection.rs CanonW 同一律:u32 小端定宽、
/// length-prefix 字符串、u32 计数列表;本模块自持有副本,零跨模块私有依赖)。
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
    fn i64v(&mut self, v: i64) {
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

/// 组合值规范编码(RXS-0309:bool = u32 LE 0/1;enum = 成员名 length-prefix;
/// int = i64 LE)。
fn encode_value(w: &mut CanonW, v: &PermValue) {
    match v {
        PermValue::Bool(b) => w.u32v(u32::from(*b)),
        PermValue::Enum(m) => w.strv(m),
        PermValue::Int(x) => w.i64v(*x),
    }
}

impl PermDomain {
    /// 规范化:axis 按名字节序;forbid 行内等式按 axis 名字节序、行间按整行
    /// 规范字节序。构造侧(`extract_domain`)完成后调用,保证声明序不影响
    /// 任何 key/digest(RXS-0309 确定性律)。
    fn normalize(&mut self) {
        self.axes
            .sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        for row in &mut self.forbids {
            row.equations
                .sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        }
        // 行间按整行规范字节序(sort_by_key 稳定,与整行字节序比较等价)。
        self.forbids.sort_by_key(encode_forbid_row);
        debug_assert!(
            self.forbids
                .windows(2)
                .all(|w| encode_forbid_row(&w[0]) <= encode_forbid_row(&w[1])),
            "forbid 行间须按整行字节序"
        );
    }

    /// 规范域字节(RXS-0309):版本前缀起始;axis 表 → forbid 行表 → budget
    /// (未声明 = `0xFFFF_FFFF` 哨兵)。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = CanonW::new();
        w.bytes(PERM_DOMAIN_V1);
        w.u32v(self.axes.len() as u32);
        for a in &self.axes {
            w.strv(&a.name);
            w.u32v(a.domain.type_tag());
            match &a.domain {
                AxisDomain::Bool => {}
                AxisDomain::Enum(ms) => {
                    w.u32v(ms.len() as u32);
                    for m in ms {
                        w.strv(m);
                    }
                }
                AxisDomain::Int(lo, hi) => {
                    w.i64v(*lo);
                    w.i64v(*hi);
                }
            }
        }
        w.u32v(self.forbids.len() as u32);
        for row in &self.forbids {
            w.bytes(&encode_forbid_row(row));
        }
        w.u32v(self.budget.unwrap_or(u32::MAX));
        w.buf
    }

    /// `permutation_domain_digest = SHA-256("rurix.permutation-domain.v1\0" ||
    /// canonical_domain_bytes 去前缀段)`(RXS-0309)。canonical 字节以前缀起始,
    /// 故 digest = SHA-256(完整 canonical 字节),与 spec 定义逐字节等价。
    pub fn digest(&self) -> [u8; 32] {
        sha256::digest(&self.canonical_bytes())
    }

    /// `enumerated = ∏|axis|`(RXS-0310;整数算术,不物化组合表)。
    pub fn enumerated(&self) -> u128 {
        self.axes.iter().map(|a| a.domain.size()).product()
    }

    /// 求解(RXS-0310):预算判定在组合物化前完成;`enumerated > budget` = 硬失败。
    /// 有效 budget = CLI 覆盖值 > attr 声明值 > `u32::MAX` 哨兵(上限含等号)。
    pub fn solve(&self, budget_override: Option<u32>) -> Result<Solution, BudgetExceeded> {
        let budget = budget_override.or(self.budget).unwrap_or(u32::MAX);
        let enumerated = self.enumerated();
        if enumerated > u128::from(budget) {
            return Err(BudgetExceeded { enumerated, budget });
        }
        // 笛卡尔枚举(axis 规范序;数值下标推进,与声明序无关)。
        let domains: Vec<Vec<PermValue>> = self.axes.iter().map(|a| a.domain.values()).collect();
        let mut combos: Vec<Vec<PermValue>> = vec![Vec::new()];
        for vals in &domains {
            let mut next: Vec<Vec<PermValue>> = Vec::with_capacity(combos.len() * vals.len());
            for c in &combos {
                for v in vals {
                    let mut c2 = c.clone();
                    c2.push(v.clone());
                    next.push(c2);
                }
            }
            combos = next;
        }
        let mut pruned: u128 = 0;
        let mut keys: Vec<KeyedCombination> = Vec::new();
        for combo in combos {
            if self.is_forbidden(&combo) {
                pruned += 1;
                continue;
            }
            keys.push(KeyedCombination {
                string_key: self.string_key(&combo),
                binary_key: self.binary_key(&combo),
            });
        }
        // keys[] 按字符串字节序(RXS-0310 报告律);二进制 key 与字符串 key 一一
        // 对应(同一排序、同一组合)。
        keys.sort_by(|a, b| a.string_key.as_bytes().cmp(b.string_key.as_bytes()));
        debug_assert_eq!(enumerated, pruned + keys.len() as u128);
        Ok(Solution {
            enumerated,
            pruned,
            keys,
        })
    }

    /// forbid 匹配:任一行全部等式满足即被裁剪(等式合取,RXS-0308)。
    fn is_forbidden(&self, combo: &[PermValue]) -> bool {
        self.forbids.iter().any(|row| {
            row.equations.iter().all(|(name, val)| {
                self.axes
                    .iter()
                    .position(|a| &a.name == name)
                    .is_some_and(|i| &combo[i] == val)
            })
        })
    }

    /// 组合的字符串形态 key(RXS-0309:`NAME=value;NAME=value`,axis 名字节序)。
    pub fn string_key(&self, combo: &[PermValue]) -> String {
        self.axes
            .iter()
            .zip(combo.iter())
            .map(|(a, v)| format!("{}={}", a.name, v.render()))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// 组合的二进制 canonical key(RXS-0309:版本前缀 + 按 axis 名字节序的
    /// `(name, type_tag u32 LE, 值规范编码)` 序列)。
    pub fn binary_key(&self, combo: &[PermValue]) -> Vec<u8> {
        let mut w = CanonW::new();
        w.bytes(PERM_KEY_DOMAIN);
        for (a, v) in self.axes.iter().zip(combo.iter()) {
            w.strv(&a.name);
            w.u32v(a.domain.type_tag());
            encode_value(&mut w, v);
        }
        w.buf
    }

    /// `--permutation-select=KEY` 校验(RXS-0310 选择律):KEY(字符串形态)∈ 合法
    /// 组合集 → Ok(规范字符串形态);∉ → 确定性错误(**禁**「最接近」回退/模糊
    /// 匹配,精确字节比对)。预算律同求解路径(选择需物化合法集)。
    pub fn validate_select_key(
        &self,
        key: &str,
        budget_override: Option<u32>,
    ) -> Result<String, SelectError> {
        let sol = self.solve(budget_override)?;
        match sol.keys.iter().find(|k| k.string_key == key) {
            Some(k) => Ok(k.string_key.clone()),
            None => Err(SelectError::InvalidKey {
                detail: format!(
                    "`--permutation-select` 的 key `{key}` 不在该 entry 的合法组合集内({} 个合法 key;精确匹配,无最接近回退,RXS-0310)",
                    sol.keys.len()
                ),
            }),
        }
    }
}

/// 一条 forbid 行的规范字节(行内等式已按 axis 名字节序;u32 计数 + 逐等式
/// `(name, 值规范编码)`)。
fn encode_forbid_row(row: &ForbidRow) -> Vec<u8> {
    let mut w = CanonW::new();
    w.u32v(row.equations.len() as u32);
    for (name, val) in &row.equations {
        w.strv(name);
        encode_value(&mut w, val);
    }
    w.buf
}

/// 求解结果(RXS-0310;恒等式 `enumerated == pruned + emitted` 结构保证)。
#[derive(Clone, Debug)]
pub struct Solution {
    /// 组合全集基数(∏|axis|)。
    pub enumerated: u128,
    /// 被至少一条 forbid 行裁剪的组合数。
    pub pruned: u128,
    /// 合法组合(emitted = `keys.len()`;按字符串 key 字节序)。
    pub keys: Vec<KeyedCombination>,
}

/// 一个合法组合的双形态 key(二进制 canonical + 字符串展示,一一对应)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyedCombination {
    /// 字符串形态(`NAME=value;NAME=value`)。
    pub string_key: String,
    /// 二进制 canonical key。
    pub binary_key: Vec<u8>,
}

/// 预算超限(RX7023 载体;`enumerated > budget`,物化前判定)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BudgetExceeded {
    /// 组合全集基数(整数算术真值)。
    pub enumerated: u128,
    /// 有效 budget(CLI 覆盖 > attr 声明 > 哨兵)。
    pub budget: u32,
}

/// select 校验失败(RXS-0310 选择律)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SelectError {
    /// KEY ∉ 合法组合集(RX3019 类;禁最接近回退)。
    InvalidKey {
        /// 诊断上下文。
        detail: String,
    },
    /// 求解即超预算(RX7023)。
    Budget(BudgetExceeded),
}

impl From<BudgetExceeded> for SelectError {
    fn from(e: BudgetExceeded) -> Self {
        SelectError::Budget(e)
    }
}

// ═══════════════════════ 属性提取与合法性校验(RXS-0308) ═══════════════════════

/// 单段路径名(非单段 → None)。
fn single_seg(p: &ast::Path) -> Option<&str> {
    match p.segments.as_slice() {
        [seg] => Some(seg.ident.name.as_str()),
        _ => None,
    }
}

/// path-only meta 项的单段名(`bool` / `FOG` / `low`;非 path-only 或非单段 → None)。
fn path_only_seg(mi: &ast::MetaItem) -> Option<&str> {
    if !matches!(mi.kind, MetaKind::Path) {
        return None;
    }
    single_seg(&mi.path)
}

/// meta 整数字面量 → i64(源切片取数字文本,后缀/`_` 容忍;切片不可达/解析失败/
/// 超 i64 → None)。负数不在 meta 字面量子集(parser 层解析错误;模型层 i64 预留)。
fn lit_int(src: &str, lit: &ast::Lit) -> Option<i64> {
    if lit.kind != LitKind::Int {
        return None;
    }
    let text = src.get(lit.span.lo.0 as usize..lit.span.hi.0 as usize)?;
    let digits: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .filter(|c| *c != '_')
        .collect();
    digits.parse::<i64>().ok()
}

fn invalid(detail: String, span: Span) -> PermInvalid {
    PermInvalid { detail, span }
}

/// 自 entry 属性表提取 permutation 域(RXS-0308)。无 `#[permutation]` 标注 →
/// `Ok(None)`(空域);有标注 → 子句闭集解析 + 合法性校验,任一违例 →
/// `Err(PermInvalid)`(RX3019,fail-closed)。返回的域已规范化(声明序不影响
/// key/digest)。
pub fn extract_domain(attrs: &[ast::Attr], src: &str) -> Result<Option<PermDomain>, PermInvalid> {
    let mut axes: Vec<AxisDecl> = Vec::new();
    let mut forbids: Vec<ForbidRow> = Vec::new();
    let mut budget: Option<u32> = None;
    let mut seen = false;

    for attr in attrs {
        if single_seg(&attr.meta.path) != Some("permutation") {
            continue;
        }
        seen = true;
        let MetaKind::List(inner) = &attr.meta.kind else {
            return Err(invalid(
                "`#[permutation]` 须为列表形态 `#[permutation(axis(...)/forbid(...)/budget(N))]`(RXS-0308 子句闭集)".to_owned(),
                attr.span,
            ));
        };
        for entry in inner {
            let MetaInner::Meta(mi) = entry else {
                return Err(invalid(
                    "`#[permutation(...)]` 实参须为 axis/forbid/budget 子句(裸字面量不在闭集,RXS-0308)".to_owned(),
                    attr.span,
                ));
            };
            match single_seg(&mi.path) {
                Some("axis") => parse_axis(mi, src, &mut axes)?,
                Some("forbid") => parse_forbid(mi, src, &mut forbids)?,
                Some("budget") => parse_budget(mi, src, &mut budget)?,
                Some(other) => {
                    return Err(invalid(
                        format!(
                            "未知 `#[permutation]` 子句 `{other}`(闭集 = axis/forbid/budget,RXS-0308)"
                        ),
                        mi.span,
                    ));
                }
                None => {
                    return Err(invalid(
                        "`#[permutation(...)]` 子句名须为单段标识符(RXS-0308)".to_owned(),
                        mi.span,
                    ));
                }
            }
        }
    }
    if !seen {
        return Ok(None);
    }

    // ── 合法性校验(违例 = RX3019,fail-closed;校验先于任何组合枚举)──
    // forbid 引用未知 axis 或该 axis 值域外的值。
    for row in &forbids {
        for (name, val) in &row.equations {
            let Some(axis) = axes.iter().find(|a| &a.name == name) else {
                return Err(invalid(
                    format!("`forbid` 引用未知 axis `{name}`(须先以 `axis(...)` 声明,RXS-0308)"),
                    row.span,
                ));
            };
            let in_domain = match (&axis.domain, val) {
                (AxisDomain::Bool, PermValue::Bool(_)) => true,
                (AxisDomain::Enum(ms), PermValue::Enum(m)) => ms.contains(m),
                (AxisDomain::Int(lo, hi), PermValue::Int(v)) => lo <= v && v <= hi,
                _ => false,
            };
            if !in_domain {
                return Err(invalid(
                    format!(
                        "`forbid` 等式 `{name} = {}` 的值超出 axis `{name}` 的值域(类型须一致且在声明值域内,RXS-0308)",
                        val.render()
                    ),
                    row.span,
                ));
            }
        }
    }

    let mut domain = PermDomain {
        axes,
        forbids,
        budget,
    };
    domain.normalize();
    Ok(Some(domain))
}

/// `axis(NAME, 值域)` 子句解析 + axis 级合法性(重名/空值域)。
fn parse_axis(mi: &ast::MetaItem, src: &str, axes: &mut Vec<AxisDecl>) -> Result<(), PermInvalid> {
    let MetaKind::List(items) = &mi.kind else {
        return Err(invalid(
            "`axis(...)` 须为列表形态 `axis(NAME, bool|enum(..)|int(LO, HI))`(RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    let [MetaInner::Meta(name_mi), MetaInner::Meta(domain_mi)] = items.as_slice() else {
        return Err(invalid(
            "`axis(...)` 须恰两实参 `axis(NAME, 值域)`(RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    let Some(name) = path_only_seg(name_mi) else {
        return Err(invalid(
            "`axis(...)` 首实参须为 axis 名标识符(RXS-0308)".to_owned(),
            name_mi.span,
        ));
    };
    if axes.iter().any(|a| a.name == name) {
        return Err(invalid(
            format!("axis `{name}` 重名声明(同 entry 内 NAME 不得重复,RXS-0308)"),
            name_mi.span,
        ));
    }
    let domain = match single_seg(&domain_mi.path) {
        Some("bool") if matches!(domain_mi.kind, MetaKind::Path) => AxisDomain::Bool,
        Some("enum") => {
            let MetaKind::List(members) = &domain_mi.kind else {
                return Err(invalid(
                    "`enum(...)` 须为列表形态 `enum(m0, m1, ...)`(RXS-0308)".to_owned(),
                    domain_mi.span,
                ));
            };
            let mut ms: Vec<String> = Vec::new();
            for m in members {
                let MetaInner::Meta(m_mi) = m else {
                    return Err(invalid(
                        "`enum(...)` 成员须为标识符(字面量成员不在闭集,RXS-0308)".to_owned(),
                        domain_mi.span,
                    ));
                };
                let Some(mn) = path_only_seg(m_mi) else {
                    return Err(invalid(
                        "`enum(...)` 成员须为单段标识符(RXS-0308)".to_owned(),
                        m_mi.span,
                    ));
                };
                ms.push(mn.to_owned());
            }
            if ms.is_empty() {
                return Err(invalid(
                    "空值域:`enum()` 零成员(≥1 个成员,RXS-0308)".to_owned(),
                    domain_mi.span,
                ));
            }
            AxisDomain::Enum(ms)
        }
        Some("int") => {
            let MetaKind::List(bounds) = &domain_mi.kind else {
                return Err(invalid(
                    "`int(...)` 须为列表形态 `int(LO, HI)`(RXS-0308)".to_owned(),
                    domain_mi.span,
                ));
            };
            let [MetaInner::Lit(lo_lit), MetaInner::Lit(hi_lit)] = bounds.as_slice() else {
                return Err(invalid(
                    "`int(LO, HI)` 须恰两整数字面量实参(RXS-0308)".to_owned(),
                    domain_mi.span,
                ));
            };
            let (Some(lo), Some(hi)) = (lit_int(src, lo_lit), lit_int(src, hi_lit)) else {
                return Err(invalid(
                    "`int(LO, HI)` 区间界须为整数字面量(RXS-0308)".to_owned(),
                    domain_mi.span,
                ));
            };
            if lo > hi {
                return Err(invalid(
                    format!("空值域:`int({lo}, {hi})` 且 LO > HI(闭区间须 LO <= HI,RXS-0308)"),
                    domain_mi.span,
                ));
            }
            AxisDomain::Int(lo, hi)
        }
        Some(other) => {
            return Err(invalid(
                format!("未知 axis 值域 `{other}`(三类闭集 = bool/enum(..)/int(LO, HI),RXS-0308)"),
                domain_mi.span,
            ));
        }
        None => {
            return Err(invalid(
                "`axis(...)` 次实参须为值域子句 bool/enum(..)/int(LO, HI)(RXS-0308)".to_owned(),
                domain_mi.span,
            ));
        }
    };
    axes.push(AxisDecl {
        name: name.to_owned(),
        domain,
    });
    Ok(())
}

/// `forbid(NAME = 值, ...)` 子句解析(等式合取;axis/值域引用校验在
/// `extract_domain` 汇总后完成——forbid 可先现于 axis 声明,多条 attr 可叠加)。
fn parse_forbid(
    mi: &ast::MetaItem,
    src: &str,
    rows: &mut Vec<ForbidRow>,
) -> Result<(), PermInvalid> {
    let MetaKind::List(items) = &mi.kind else {
        return Err(invalid(
            "`forbid(...)` 须为列表形态 `forbid(NAME = 值, ...)`(RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    if items.is_empty() {
        return Err(invalid(
            "`forbid()` 至少一条 `NAME = 值` 等式(实参形态错误,RXS-0308)".to_owned(),
            mi.span,
        ));
    }
    let mut equations: Vec<(String, PermValue)> = Vec::new();
    for item in items {
        let MetaInner::Meta(eq) = item else {
            return Err(invalid(
                "`forbid(...)` 实参须为 `NAME = 值` 等式(裸字面量不在闭集,RXS-0308)".to_owned(),
                mi.span,
            ));
        };
        let Some(name) = single_seg(&eq.path) else {
            return Err(invalid(
                "`forbid` 等式左端须为 axis 名标识符(RXS-0308)".to_owned(),
                eq.span,
            ));
        };
        let value = match &eq.kind {
            MetaKind::NameValue(lit) => match lit.kind {
                LitKind::Bool(b) => PermValue::Bool(b),
                LitKind::Int => {
                    let Some(v) = lit_int(src, lit) else {
                        return Err(invalid(
                            "`forbid` 等式 int 值须为整数字面量(RXS-0308)".to_owned(),
                            eq.span,
                        ));
                    };
                    PermValue::Int(v)
                }
                _ => {
                    return Err(invalid(
                        "`forbid` 等式值须为 bool 字面量 / 整数字面量 / enum 成员标识符(RXS-0308)"
                            .to_owned(),
                        eq.span,
                    ));
                }
            },
            MetaKind::NameValuePath(p) => {
                let Some(m) = single_seg(p) else {
                    return Err(invalid(
                        "`forbid` 等式 enum 成员值须为单段标识符(RXS-0308)".to_owned(),
                        eq.span,
                    ));
                };
                PermValue::Enum(m.to_owned())
            }
            _ => {
                return Err(invalid(
                    "`forbid` 等式须为 `NAME = 值` 名值形态(RXS-0308)".to_owned(),
                    eq.span,
                ));
            }
        };
        equations.push((name.to_owned(), value));
    }
    rows.push(ForbidRow {
        equations,
        span: mi.span,
    });
    Ok(())
}

/// `budget(N)` 子句解析(N 正整数;每 entry 至多一条)。
fn parse_budget(
    mi: &ast::MetaItem,
    src: &str,
    budget: &mut Option<u32>,
) -> Result<(), PermInvalid> {
    let MetaKind::List(items) = &mi.kind else {
        return Err(invalid(
            "`budget(N)` 须为列表形态(N 正整数,RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    let [MetaInner::Lit(lit)] = items.as_slice() else {
        return Err(invalid(
            "`budget(N)` 须恰一个正整数字面量实参(RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    let Some(v) = lit_int(src, lit) else {
        return Err(invalid(
            "`budget(N)` 须为整数字面量(RXS-0308)".to_owned(),
            mi.span,
        ));
    };
    if v <= 0 || v > i64::from(u32::MAX) {
        return Err(invalid(
            format!("`budget({v})` 非正整数域(N ∈ [1, 2^32-1],RXS-0308)"),
            mi.span,
        ));
    }
    if budget.is_some() {
        return Err(invalid(
            "`budget(N)` 重复声明(每 entry 至多一条,RXS-0308)".to_owned(),
            mi.span,
        ));
    }
    *budget = Some(v as u32);
    Ok(())
}

// ═══════════════════════ 编译期校验挂接(RX3019,typeck 段) ═══════════════════════

/// permutation 域声明校验(RXS-0308;AST 层,与 `#[numthreads]` 家族同一机械,经
/// [`crate::shader_stages::check`] 于 resolve 后 typeck 前挂接)。校验先于
/// reflection/permutation 求解;任何违例都不得进入组合枚举。
///
/// - `#[permutation]` 附着于非着色入口函数(kernel/compute/vertex/fragment/mesh
///   之外:host fn、task、RT 六阶段)→ RX3019;
/// - 泛型着色函数不产 entry(RXS-0304),其上标注不参与求解(RXS-0308 实现要求),
///   亦不校验(保守不发诊断);
/// - 域声明违例(重名 axis/空值域/forbid 引用未知 axis 或域外值/budget 非正或
///   重复/未知子句/实参形态错误)→ RX3019。
//@ spec: RXS-0308
pub fn check_domains(file: &ast::SourceFile, src: &str, diag: &DiagCtxt) {
    check_domains_rec(&file.items, src, diag);
}

fn check_domains_rec(items: &[ast::Item], src: &str, diag: &DiagCtxt) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => {
                let Some(perm_attr) = it
                    .attrs
                    .iter()
                    .find(|a| single_seg(&a.meta.path) == Some("permutation"))
                else {
                    continue;
                };
                let is_shader_entry = matches!(
                    f.stage,
                    Some(
                        ShaderStage::Vertex
                            | ShaderStage::Fragment
                            | ShaderStage::Compute
                            | ShaderStage::Mesh
                    )
                ) || (f.stage.is_none() && f.color == FnColor::Kernel);
                if !is_shader_entry {
                    diag.struct_error(E_PERMUTATION_DOMAIN, "shader.permutation_domain_invalid")
                        .arg(
                            "detail",
                            format!(
                                "`#[permutation]` 仅可附着着色入口函数(kernel/compute/vertex/fragment/mesh fn);`{}` 不是着色入口(RXS-0308)",
                                f.name.name
                            ),
                        )
                        .span_label(perm_attr.span, "invalid #[permutation] attachment")
                        .emit();
                    continue;
                }
                // 泛型着色函数不产 entry(RXS-0304 口径),其标注不参与求解。
                if !f.generics.params.is_empty() {
                    continue;
                }
                if let Err(inv) = extract_domain(&it.attrs, src) {
                    diag.struct_error(E_PERMUTATION_DOMAIN, "shader.permutation_domain_invalid")
                        .arg("detail", inv.detail)
                        .span_label(inv.span, "invalid permutation domain")
                        .emit();
                }
            }
            ast::ItemKind::Mod(m) => check_domains_rec(&m.items, src, diag),
            _ => {}
        }
    }
}

// ═══════════════════════ entry 枚举(与 reflection 同一口径) ═══════════════════════

/// v1 可枚举着色入口判定(与 `reflection::enumerable_stage` 同一口径:RT 阶段与
/// task 不可枚举;`kernel fn` 归 Compute)。
fn enumerable_stage(stage: Option<ShaderStage>, color: FnColor) -> Option<ShaderStage> {
    match stage {
        None if color == FnColor::Kernel => Some(ShaderStage::Compute),
        Some(
            s @ (ShaderStage::Vertex
            | ShaderStage::Fragment
            | ShaderStage::Compute
            | ShaderStage::Mesh),
        ) => Some(s),
        _ => None,
    }
}

/// 递归枚举 entry(含嵌套 `mod`,路径以 `::` 连接);泛型着色函数不产 entry。
/// 返回 AST 声明序(name_path, FnItem, item attrs)。
fn collect_perm_entries<'a>(
    items: &'a [ast::Item],
    prefix: &str,
    out: &mut Vec<(String, &'a ast::FnItem, &'a [ast::Attr])>,
) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => {
                if !f.generics.params.is_empty() {
                    continue;
                }
                if enumerable_stage(f.stage, f.color).is_some() {
                    out.push((format!("{prefix}{}", f.name.name), f, it.attrs.as_slice()));
                }
            }
            ast::ItemKind::Mod(m) => {
                collect_perm_entries(&m.items, &format!("{prefix}{}::", m.name.name), out);
            }
            _ => {}
        }
    }
}

// ═══════════════════════ 报告(RXS-0310 报告律) ═══════════════════════

/// per-axis 报告面(元数据 + 值域值表;预算超限路径 values 置空——报告只含
/// axis 元数据与计数,不泄漏部分组合表,RXS-0310 实现要求)。
#[derive(Clone, Debug)]
pub struct AxisReport {
    /// axis 名。
    pub name: String,
    /// 类型名(`bool`/`enum`/`int`)。
    pub ty: &'static str,
    /// 值域基数 |axis|。
    pub domain_size: u128,
    /// 规范序值表(字符串形态;预算超限路径为空表)。
    pub values: Vec<String>,
}

/// axis contribution(逐 axis 的 |axis| 与占比;占比为精确有理数
/// `share_num/share_den` = `|axis|/enumerated`,整数对编码,JSON 无浮点)。
#[derive(Clone, Debug)]
pub struct AxisContribution {
    /// axis 名。
    pub name: String,
    /// 值域基数 |axis|。
    pub domain_size: u128,
    /// 占比分子(= |axis|)。
    pub share_num: u128,
    /// 占比分母(= enumerated)。
    pub share_den: u128,
}

/// 单 entry 的 permutation 报告记录(RXS-0310 per-entry 字段闭集 + `name` 标识)。
#[derive(Clone, Debug)]
pub struct EntryReport {
    /// entry identity(源级名称路径)。
    pub name: String,
    /// 阶段 tag(RXS-0290 单一事实源)。
    pub stage_tag: u32,
    /// 域 digest(空域 = M31 既有常量,hex 展示面)。
    pub domain_digest: [u8; 32],
    /// axis 元数据表(规范序)。
    pub axes: Vec<AxisReport>,
    /// 组合全集基数(整数算术;超预算路径仍为真值)。
    pub enumerated: u128,
    /// 裁剪数(超预算路径未知 = None → JSON null)。
    pub pruned: Option<u128>,
    /// 合法数(超预算路径未知 = None → JSON null)。
    pub emitted: Option<u128>,
    /// 合法 key 集合(字符串形态,字节序;超预算路径空表——不泄漏部分组合表)。
    pub keys: Vec<String>,
    /// 逐 axis contribution。
    pub axis_contribution: Vec<AxisContribution>,
    /// 有效 budget(CLI 覆盖 > attr 声明 > `u32::MAX` 哨兵)。
    pub budget: u32,
    /// 本 entry 是否预算超限(硬失败)。
    pub budget_exceeded: bool,
}

/// 编译单元级 permutation 报告。
#[derive(Clone, Debug)]
pub struct PermReport {
    /// entry 记录(规范键 `(name, stage_tag)` 排序)。
    pub entries: Vec<EntryReport>,
    /// 预算超限的 entry 名表(空 = 全绿)。
    pub exceeded: Vec<String>,
}

/// 构建 `--emit=permutations` 报告(RXS-0310):逐 entry 提取域 → 求解(预算判定
/// 在物化前)→ 记录。空域 entry 与非空域 entry 共存(空域 = 单空组合,
/// enumerated 1 / pruned 0 / emitted 1,恒等式成立)。域声明违例在正常管线已被
/// RX3019 关卡拦截;此处保守 fail-closed。
pub fn build_report(
    file: &ast::SourceFile,
    src: &str,
    budget_override: Option<u32>,
) -> Result<PermReport, PermInvalid> {
    let mut raw: Vec<(String, &ast::FnItem, &[ast::Attr])> = Vec::new();
    collect_perm_entries(&file.items, "", &mut raw);

    let mut entries: Vec<EntryReport> = Vec::with_capacity(raw.len());
    let mut exceeded: Vec<String> = Vec::new();
    for (name, f, attrs) in raw {
        let stage = enumerable_stage(f.stage, f.color).expect("枚举口径已过滤");
        let stage_tag = crate::codegen::stage_tag(stage);
        match extract_domain(attrs, src)? {
            None => {
                // 空域:单空组合;digest 恒 M31 既有常量(RXS-0304 空编码 0 漂移)。
                entries.push(EntryReport {
                    name,
                    stage_tag,
                    domain_digest: empty_domain_digest(),
                    axes: Vec::new(),
                    enumerated: 1,
                    pruned: Some(0),
                    emitted: Some(1),
                    keys: vec![String::new()],
                    axis_contribution: Vec::new(),
                    budget: budget_override.unwrap_or(u32::MAX),
                    budget_exceeded: false,
                });
            }
            Some(domain) => {
                let enumerated = domain.enumerated();
                let budget = budget_override.or(domain.budget).unwrap_or(u32::MAX);
                let contribution: Vec<AxisContribution> = domain
                    .axes
                    .iter()
                    .map(|a| AxisContribution {
                        name: a.name.clone(),
                        domain_size: a.domain.size(),
                        share_num: a.domain.size(),
                        share_den: enumerated,
                    })
                    .collect();
                match domain.solve(budget_override) {
                    Ok(sol) => {
                        entries.push(EntryReport {
                            name,
                            stage_tag,
                            domain_digest: domain.digest(),
                            axes: domain
                                .axes
                                .iter()
                                .map(|a| AxisReport {
                                    name: a.name.clone(),
                                    ty: a.domain.type_name(),
                                    domain_size: a.domain.size(),
                                    values: a.domain.values().iter().map(|v| v.render()).collect(),
                                })
                                .collect(),
                            enumerated,
                            pruned: Some(sol.pruned),
                            emitted: Some(sol.keys.len() as u128),
                            keys: sol.keys.iter().map(|k| k.string_key.clone()).collect(),
                            axis_contribution: contribution,
                            budget,
                            budget_exceeded: false,
                        });
                    }
                    Err(_) => {
                        // 超预算硬失败:报告只含 axis 元数据与计数,组合表不物化、
                        // 不泄漏(RXS-0310);axis contribution report 照常产出。
                        exceeded.push(name.clone());
                        entries.push(EntryReport {
                            name,
                            stage_tag,
                            domain_digest: domain.digest(),
                            axes: domain
                                .axes
                                .iter()
                                .map(|a| AxisReport {
                                    name: a.name.clone(),
                                    ty: a.domain.type_name(),
                                    domain_size: a.domain.size(),
                                    values: Vec::new(),
                                })
                                .collect(),
                            enumerated,
                            pruned: None,
                            emitted: None,
                            keys: Vec::new(),
                            axis_contribution: contribution,
                            budget,
                            budget_exceeded: true,
                        });
                    }
                }
            }
        }
    }
    // 规范键排序(字节序字典序;与声明序无关,RXS-0305 同律)。
    entries.sort_by(|a, b| (a.name.as_str(), a.stage_tag).cmp(&(b.name.as_str(), b.stage_tag)));
    Ok(PermReport { entries, exceeded })
}

// ═══════════════════════ 报告 JSON 产物(确定性 canonical JSON) ═══════════════════════

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

fn opt_u128(v: Option<u128>) -> String {
    v.map_or("null".to_owned(), |n| n.to_string())
}

fn entry_report_json(e: &EntryReport, ind: &str) -> String {
    let i2 = format!("{ind}  ");
    let i3 = format!("{ind}    ");
    let mut s = String::new();
    s.push_str(&format!("{ind}{{\n"));
    s.push_str(&format!("{i2}\"name\": \"{}\",\n", json_escape(&e.name)));
    s.push_str(&format!("{i2}\"stage_tag\": {},\n", e.stage_tag));
    s.push_str(&format!(
        "{i2}\"domain_digest\": \"{}\",\n",
        sha256::hex(&e.domain_digest)
    ));
    if e.axes.is_empty() {
        s.push_str(&format!("{i2}\"axes\": [],\n"));
    } else {
        s.push_str(&format!("{i2}\"axes\": [\n"));
        for (k, a) in e.axes.iter().enumerate() {
            let values = a
                .values
                .iter()
                .map(|v| format!("\"{}\"", json_escape(v)))
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!(
                "{i3}{{\"name\": \"{}\", \"type\": \"{}\", \"domain_size\": {}, \"values\": [{}]}}{}\n",
                json_escape(&a.name),
                a.ty,
                a.domain_size,
                values,
                if k + 1 == e.axes.len() { "" } else { "," },
            ));
        }
        s.push_str(&format!("{i2}],\n"));
    }
    s.push_str(&format!("{i2}\"enumerated\": {},\n", e.enumerated));
    s.push_str(&format!("{i2}\"pruned\": {},\n", opt_u128(e.pruned)));
    s.push_str(&format!("{i2}\"emitted\": {},\n", opt_u128(e.emitted)));
    if e.keys.is_empty() {
        s.push_str(&format!("{i2}\"keys\": [],\n"));
    } else {
        let keys = e
            .keys
            .iter()
            .map(|k| format!("\"{}\"", json_escape(k)))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("{i2}\"keys\": [{keys}],\n"));
    }
    if e.axis_contribution.is_empty() {
        s.push_str(&format!("{i2}\"axis_contribution\": [],\n"));
    } else {
        s.push_str(&format!("{i2}\"axis_contribution\": [\n"));
        for (k, c) in e.axis_contribution.iter().enumerate() {
            s.push_str(&format!(
                "{i3}{{\"name\": \"{}\", \"domain_size\": {}, \"share_num\": {}, \"share_den\": {}}}{}\n",
                json_escape(&c.name),
                c.domain_size,
                c.share_num,
                c.share_den,
                if k + 1 == e.axis_contribution.len() { "" } else { "," },
            ));
        }
        s.push_str(&format!("{i2}],\n"));
    }
    s.push_str(&format!("{i2}\"budget\": {},\n", e.budget));
    s.push_str(&format!("{i2}\"budget_exceeded\": {}\n", e.budget_exceeded));
    s.push_str(&format!("{ind}}}"));
    s
}

/// 报告 → 确定性 JSON 产物(键序固定、UTF-8、LF 行尾、整数不浮点;无绝对路径/
/// 文件名/时间戳/进程因素,RXS-0305 禁用面同律)。双次生成逐字节相等。
pub fn to_report_json(report: &PermReport) -> String {
    let compiler_version = env!("CARGO_PKG_VERSION");
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{}\",\n", REPORT_SCHEMA_ID));
    s.push_str(&format!(
        "  \"schema_version\": {},\n",
        REPORT_SCHEMA_VERSION
    ));
    s.push_str("  \"compiler\": \"rurixc\",\n");
    s.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(compiler_version)
    ));
    s.push_str("  \"edition\": \"Rx0\",\n");
    if report.entries.is_empty() {
        s.push_str("  \"entries\": []\n");
    } else {
        s.push_str("  \"entries\": [\n");
        for (k, e) in report.entries.iter().enumerate() {
            s.push_str(&entry_report_json(e, "    "));
            s.push_str(if k + 1 == report.entries.len() {
                "\n"
            } else {
                ",\n"
            });
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::source_map::SourceMap;
    use crate::span::{Edition, SourceId};

    fn parse_src(src: &str) -> (ast::SourceFile, SourceId) {
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        assert!(!diag.has_errors(), "测试源须解析干净");
        (file, id)
    }

    /// 提取指定名的 entry 的 permutation 域(含嵌套 attr 面)。
    fn domain_of(src: &str, fn_name: &str) -> PermDomain {
        let (file, _) = parse_src(src);
        let mut raw: Vec<(String, &ast::FnItem, &[ast::Attr])> = Vec::new();
        collect_perm_entries(&file.items, "", &mut raw);
        let (_, _, attrs) = raw
            .iter()
            .find(|(n, _, _)| n == fn_name)
            .unwrap_or_else(|| panic!("entry {fn_name} 应在枚举面"));
        extract_domain(attrs, src)
            .expect("域提取须成功")
            .expect("须有非空 permutation 域")
    }

    /// 基准源:bool + enum + forbid(basic_domain 语料的同构形)。
    const BASIC: &str = r#"
#[permutation(axis(FOG, bool))]
#[permutation(axis(QUALITY, enum(low, med, high)))]
#[permutation(forbid(FOG = true, QUALITY = low))]
#[permutation(budget(6))]
kernel fn kmain() {}
"#;

    /// 双次生成:canonical key 与 domain digest 逐字节相等(RXS-0309 确定性律)。
    //@ spec: RXS-0309
    #[test]
    fn double_key_generation_byte_identical() {
        let d1 = domain_of(BASIC, "kmain");
        let d2 = domain_of(BASIC, "kmain");
        assert_eq!(d1.canonical_bytes(), d2.canonical_bytes());
        assert_eq!(d1.digest(), d2.digest());
        let s1 = d1.solve(None).expect("求解须成功");
        let s2 = d2.solve(None).expect("求解须成功");
        assert_eq!(s1.keys, s2.keys, "合法 key 集合双次逐字节相等");
        for k in &s1.keys {
            assert_eq!(k.binary_key, d1.binary_key_from_string(&k.string_key));
        }
    }

    impl PermDomain {
        /// 测试辅助:由字符串 key 还原二进制 key(单射验证用)。
        fn binary_key_from_string(&self, key: &str) -> Vec<u8> {
            let combo: Vec<PermValue> = self
                .axes
                .iter()
                .map(|a| {
                    let want = key
                        .split(';')
                        .find_map(|kv| kv.strip_prefix(&format!("{}=", a.name)))
                        .unwrap_or_else(|| panic!("key 缺 axis {}", a.name));
                    a.domain
                        .values()
                        .into_iter()
                        .find(|v| v.render() == want)
                        .unwrap_or_else(|| panic!("值 {want} 在域内"))
                })
                .collect();
            self.binary_key(&combo)
        }
    }

    /// 声明序不变性:axis/forbid 声明序置换 → canonical 字节/digest/key 集合
    /// 全等(RXS-0309 确定性律;axis_order_permuted 语料的同构形)。
    //@ spec: RXS-0309
    #[test]
    fn axis_declaration_order_invariant() {
        let permuted: &str = r#"
#[permutation(axis(QUALITY, enum(low, med, high)))]
#[permutation(forbid(QUALITY = low, FOG = true))]
#[permutation(budget(6))]
#[permutation(axis(FOG, bool))]
kernel fn kmain() {}
"#;
        let a = domain_of(BASIC, "kmain");
        let b = domain_of(permuted, "kmain");
        assert_eq!(
            a.canonical_bytes(),
            b.canonical_bytes(),
            "声明序置换后域字节不变"
        );
        assert_eq!(a.digest(), b.digest());
        let ka: Vec<String> = a
            .solve(None)
            .unwrap()
            .keys
            .iter()
            .map(|k| k.string_key.clone())
            .collect();
        let kb: Vec<String> = b
            .solve(None)
            .unwrap()
            .keys
            .iter()
            .map(|k| k.string_key.clone())
            .collect();
        assert_eq!(ka, kb, "key 集合与声明序无关");
    }

    /// 裁剪正确性:forbid 等式合取恰裁掉匹配组合;合法集合 = golden 五 key。
    //@ spec: RXS-0310
    #[test]
    fn pruning_matches_golden_legal_set() {
        let d = domain_of(BASIC, "kmain");
        let sol = d.solve(None).unwrap();
        assert_eq!(sol.enumerated, 6);
        assert_eq!(sol.pruned, 1);
        let keys: Vec<&str> = sol.keys.iter().map(|k| k.string_key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "FOG=false;QUALITY=high",
                "FOG=false;QUALITY=low",
                "FOG=false;QUALITY=med",
                "FOG=true;QUALITY=high",
                "FOG=true;QUALITY=med",
            ],
            "FOG=true;QUALITY=low 被裁,余五合法 key(字节序)"
        );
    }

    /// 恒等式 enumerated == pruned + emitted(多域形态:含 forbid / 无 forbid /
    /// int 轴 / 单轴)。
    //@ spec: RXS-0310
    #[test]
    fn identity_enumerated_eq_pruned_plus_emitted() {
        let cases: &[(&str, &str)] = &[
            (BASIC, "kmain"),
            (
                "#[permutation(axis(LIGHTS, int(0, 4)))]\n#[permutation(axis(BUMP, bool))]\nkernel fn k() {}\n",
                "k",
            ),
            (
                "#[permutation(axis(FOG, bool))]\n#[permutation(forbid(FOG = false))]\n#[permutation(forbid(FOG = true))]\nkernel fn k() {}\n",
                "k",
            ),
        ];
        for (src, name) in cases {
            let d = domain_of(src, name);
            let sol = d.solve(None).unwrap();
            assert_eq!(
                sol.enumerated,
                sol.pruned + sol.keys.len() as u128,
                "恒等式: {src}"
            );
        }
        // 全覆盖 forbid:全部裁剪 → emitted 0,恒等式仍成立。
        let d = domain_of(
            "#[permutation(axis(FOG, bool))]\n#[permutation(forbid(FOG = false))]\n#[permutation(forbid(FOG = true))]\nkernel fn k() {}\n",
            "k",
        );
        let sol = d.solve(None).unwrap();
        assert_eq!(sol.enumerated, 2);
        assert_eq!(sol.pruned, 2);
        assert_eq!(sol.keys.len(), 0);
    }

    /// 预算边界:budget == enumerated 为 GREEN(上限含等号),enumerated - 1 为 RED
    /// (物化前硬失败);CLI 覆盖优先于 attr 声明。
    //@ spec: RXS-0310
    #[test]
    fn budget_boundary_green_and_red() {
        let d = domain_of(BASIC, "kmain");
        assert!(d.solve(Some(6)).is_ok(), "limit == legal 全集数(=6) GREEN");
        let err = d.solve(Some(5)).expect_err("limit == 5 须 RED");
        assert_eq!(err.enumerated, 6);
        assert_eq!(err.budget, 5);
        // attr 声明 budget(6) 生效(无 CLI 覆盖)。
        assert!(d.solve(None).is_ok());
        // CLI 覆盖 attr(收窄 → RED)。
        assert!(d.solve(Some(1)).is_err());
    }

    /// 空域恒既有常量 SHA-256("rurix.permutation-domain-empty.v1\0"),与 M31 常量
    /// 逐字节一致(RXS-0304/0309 空编码 0 漂移)。
    //@ spec: RXS-0309
    #[test]
    fn empty_domain_digest_is_stable_constant() {
        assert_eq!(
            empty_domain_digest(),
            sha256::digest(b"rurix.permutation-domain-empty.v1\0")
        );
        // M31 smoke 腿⑤硬编码基线(hex 展示面逐字节一致)。
        assert_eq!(
            sha256::hex(&empty_domain_digest()),
            "160d241dc1681a927e8edbdd07a15e508f9f5aeb68da8bc92274332cb8541f31"
        );
        // 无标注 → 空域(不触发真值化路径)。
        let (file, _) = parse_src("kernel fn plain() {}\n");
        let mut raw: Vec<(String, &ast::FnItem, &[ast::Attr])> = Vec::new();
        collect_perm_entries(&file.items, "", &mut raw);
        assert!(
            extract_domain(raw[0].2, "kernel fn plain() {}\n")
                .unwrap()
                .is_none()
        );
    }

    /// select 合法 key → 规范字符串形态;非法 key(域外值/大小写差/缺轴)→
    /// 确定性错误,禁最接近回退(RXS-0310 选择律)。
    //@ spec: RXS-0310
    #[test]
    fn select_valid_and_invalid_keys() {
        let d = domain_of(BASIC, "kmain");
        assert_eq!(
            d.validate_select_key("FOG=true;QUALITY=med", None).unwrap(),
            "FOG=true;QUALITY=med"
        );
        // 被裁剪组合不在合法集。
        assert!(matches!(
            d.validate_select_key("FOG=true;QUALITY=low", None),
            Err(SelectError::InvalidKey { .. })
        ));
        // 域外值。
        assert!(
            d.validate_select_key("FOG=true;QUALITY=ultra", None)
                .is_err()
        );
        // 大小写差 ≠ 命中(禁模糊匹配)。
        assert!(d.validate_select_key("FOG=True;QUALITY=med", None).is_err());
        // 轴序置换 ≠ 命中(字符串形态按 axis 名字节序,精确比对)。
        assert!(d.validate_select_key("QUALITY=med;FOG=true", None).is_err());
    }

    /// 违例各形态拒(重名 axis / 空值域 enum() / 空值域 int(4,0) / forbid 引用
    /// 未知 axis / forbid 引用域外值 / budget 非正 / budget 重复 / 未知子句)。
    //@ spec: RXS-0308
    #[test]
    fn invalid_domain_forms_rejected() {
        let cases: &[&str] = &[
            "#[permutation(axis(FOG, bool))]\n#[permutation(axis(FOG, bool))]\nkernel fn k() {}",
            "#[permutation(axis(Q, enum()))]\nkernel fn k() {}",
            "#[permutation(axis(Q, int(4, 0)))]\nkernel fn k() {}",
            "#[permutation(axis(FOG, bool))]\n#[permutation(forbid(UNKNOWN = true))]\nkernel fn k() {}",
            "#[permutation(axis(FOG, bool))]\n#[permutation(forbid(FOG = 1))]\nkernel fn k() {}",
            "#[permutation(axis(Q, enum(low)))]\n#[permutation(forbid(Q = high))]\nkernel fn k() {}",
            "#[permutation(axis(L, int(0, 4)))]\n#[permutation(forbid(L = 9))]\nkernel fn k() {}",
            "#[permutation(axis(FOG, bool))]\n#[permutation(budget(0))]\nkernel fn k() {}",
            "#[permutation(axis(FOG, bool))]\n#[permutation(budget(2))]\n#[permutation(budget(3))]\nkernel fn k() {}",
            "#[permutation(axis(FOG, bool))]\n#[permutation(wat(FOG))]\nkernel fn k() {}",
            "#[permutation(foo)]\nkernel fn k() {}",
        ];
        for (i, src) in cases.iter().enumerate() {
            let (file, _) = parse_src(src);
            let mut raw: Vec<(String, &ast::FnItem, &[ast::Attr])> = Vec::new();
            collect_perm_entries(&file.items, "", &mut raw);
            assert!(
                extract_domain(raw[0].2, src).is_err(),
                "违例形态 {i} 须拒: {src}"
            );
        }
    }

    /// `#[permutation]` 附着非着色入口(host fn / raygen fn)= RX3019;着色入口
    /// 合法;泛型着色函数不参与求解(RXS-0308/0304)。经 `check_domains` 端到端。
    //@ spec: RXS-0308
    #[test]
    fn attachment_target_discipline() {
        let cases: &[(&str, usize)] = &[
            ("#[permutation(axis(FOG, bool))]\nfn host_fn() {}", 1),
            ("#[permutation(axis(FOG, bool))]\nraygen fn rg() {}", 1),
            ("#[permutation(axis(FOG, bool))]\ntask fn t() {}", 1),
            ("#[permutation(axis(FOG, bool))]\nkernel fn k() {}", 0),
            (
                "#[permutation(axis(FOG, bool))]\nvertex fn v() -> f32 { 0.0 }",
                0,
            ),
            ("#[permutation(axis(FOG, bool))]\ncompute fn c() {}", 0),
        ];
        for (src, want_errors) in cases {
            let src = *src;
            let diag = DiagCtxt::new();
            let mut sm = SourceMap::new();
            let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
            let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
            let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
            assert!(!diag.has_errors(), "解析须干净: {src}");
            check_domains(&file, src, &diag);
            let codes: Vec<u16> = diag
                .emitted()
                .iter()
                .filter_map(|d| d.code.map(|c| c.0))
                .collect();
            assert_eq!(
                codes.len(),
                *want_errors,
                "{src} 诊断数不符(codes={codes:?})"
            );
            assert!(codes.iter().all(|c| *c == 3019), "全为 RX3019: {codes:?}");
        }
        // 泛型着色函数:不参与求解,零诊断。
        let src = "#[permutation(axis(FOG, bool))]\nkernel fn g<T>() {}";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        check_domains(&file, src, &diag);
        assert!(diag.emitted().is_empty(), "泛型着色函数不参与求解");
    }

    /// 组合→key 单射:两不同组合不得产生同一 key(全轴覆盖 + 定界编码,by
    /// construction);字符串与二进制形态一一对应(RXS-0309)。
    //@ spec: RXS-0309
    #[test]
    fn keys_are_injective_across_forms() {
        let d = domain_of(
            "#[permutation(axis(BUMP, bool))]\n#[permutation(axis(LIGHTS, int(0, 4)))]\n#[permutation(axis(Q, enum(low, high)))]\nkernel fn k() {}\n",
            "k",
        );
        let sol = d.solve(None).unwrap();
        assert_eq!(sol.enumerated, 20);
        let mut strings: Vec<&str> = sol.keys.iter().map(|k| k.string_key.as_str()).collect();
        strings.sort_unstable();
        strings.dedup();
        assert_eq!(strings.len(), 20, "字符串 key 单射");
        let mut binaries: Vec<&Vec<u8>> = sol.keys.iter().map(|k| &k.binary_key).collect();
        binaries.sort();
        binaries.dedup();
        assert_eq!(binaries.len(), 20, "二进制 key 单射");
        // 形态对应:二进制 key 含版本前缀。
        for k in &sol.keys {
            assert!(k.binary_key.starts_with(b"rurix.permutation-key.v1\0"));
        }
    }

    /// 域 digest 稳定且域敏感:axis 值域/forbid/budget 任一变化 → digest 必变
    /// (域语义覆盖);budget 未声明 = 0xFFFF_FFFF 哨兵编码(RXS-0309)。
    //@ spec: RXS-0309
    #[test]
    fn domain_digest_covers_domain_semantics() {
        let a = domain_of(BASIC, "kmain");
        let no_forbid = domain_of(
            "#[permutation(axis(FOG, bool))]\n#[permutation(axis(QUALITY, enum(low, med, high)))]\n#[permutation(budget(6))]\nkernel fn kmain() {}\n",
            "kmain",
        );
        let no_budget = domain_of(
            "#[permutation(axis(FOG, bool))]\n#[permutation(axis(QUALITY, enum(low, med, high)))]\n#[permutation(forbid(FOG = true, QUALITY = low))]\nkernel fn kmain() {}\n",
            "kmain",
        );
        let wider_enum = domain_of(
            "#[permutation(axis(FOG, bool))]\n#[permutation(axis(QUALITY, enum(low, med, high, ultra)))]\n#[permutation(forbid(FOG = true, QUALITY = low))]\n#[permutation(budget(8))]\nkernel fn kmain() {}\n",
            "kmain",
        );
        assert_ne!(a.digest(), no_forbid.digest(), "forbid 表是域语义");
        assert_ne!(a.digest(), no_budget.digest(), "budget 是域语义(哨兵区分)");
        assert_ne!(a.digest(), wider_enum.digest(), "值域成员是域语义");
        // 同一域双次 digest 相等。
        assert_eq!(a.digest(), domain_of(BASIC, "kmain").digest());
    }

    /// 报告面:per-entry 字段齐 + 恒等式 + keys 字节序 + 超预算路径组合表不泄漏
    /// (keys 空、pruned/emitted null、axis contribution 在位)+ JSON 双次逐字节
    /// 相等、无路径/CRLF(RXS-0310 报告律/实现要求)。
    //@ spec: RXS-0310
    #[test]
    fn report_shape_identity_and_red_path() {
        let (file, _) = parse_src(BASIC);
        let r1 = build_report(&file, BASIC, None).unwrap();
        let r2 = build_report(&file, BASIC, None).unwrap();
        let (j1, j2) = (to_report_json(&r1), to_report_json(&r2));
        assert_eq!(j1, j2, "报告 JSON 双次逐字节相等");
        assert!(!j1.contains('\r') && j1.ends_with("}\n"));
        assert!(!j1.contains("test.rx"), "文件名不得入报告");
        let e = &r1.entries[0];
        assert_eq!(e.enumerated, 6);
        assert_eq!(e.pruned, Some(1));
        assert_eq!(e.emitted, Some(5));
        assert_eq!(e.keys.len(), 5);
        let mut sorted = e.keys.clone();
        sorted.sort();
        assert_eq!(e.keys, sorted, "keys 字节序");
        assert_eq!(
            e.enumerated,
            e.pruned.unwrap() + e.emitted.unwrap(),
            "恒等式"
        );
        assert_eq!(e.axis_contribution.len(), 2, "axis contribution 在位");
        assert!(!e.budget_exceeded);
        // RED 路径:CLI 收窄预算 → 硬失败,报告照常含 axis 元数据,组合表不泄漏。
        let red = build_report(&file, BASIC, Some(2)).unwrap();
        assert_eq!(red.exceeded, vec!["kmain".to_owned()]);
        let re = &red.entries[0];
        assert!(re.budget_exceeded);
        assert_eq!(re.enumerated, 6);
        assert_eq!(re.pruned, None);
        assert_eq!(re.emitted, None);
        assert!(re.keys.is_empty(), "超预算路径不得泄漏部分组合表");
        assert_eq!(
            re.axis_contribution.len(),
            2,
            "axis contribution report 照常产出"
        );
        let jred = to_report_json(&red);
        assert!(jred.contains("\"budget_exceeded\": true"));
    }

    /// 空域 entry 与非空域 entry 共存:空域报告 = 单空组合 + M31 常量 digest;
    /// 恒等式成立(RXS-0310;empty_domain_entry 语料同构形)。
    //@ spec: RXS-0310
    #[test]
    fn mixed_empty_and_nonempty_domain_entries() {
        let src = "kernel fn plain() {}\n#[permutation(axis(FOG, bool))]\nkernel fn tagged() {}\n";
        let (file, _) = parse_src(src);
        let report = build_report(&file, src, None).unwrap();
        assert_eq!(report.entries.len(), 2);
        let plain = &report.entries[0];
        assert_eq!(plain.name, "plain");
        assert_eq!(plain.domain_digest, empty_domain_digest());
        assert_eq!(plain.enumerated, 1);
        assert_eq!(plain.pruned, Some(0));
        assert_eq!(plain.emitted, Some(1));
        assert_eq!(plain.keys, vec![String::new()]);
        let tagged = &report.entries[1];
        assert_ne!(tagged.domain_digest, empty_domain_digest(), "非空域真值化");
        assert_eq!(tagged.keys, ["FOG=false", "FOG=true"]);
    }
}
