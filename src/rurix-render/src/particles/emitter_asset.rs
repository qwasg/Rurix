//! G35-8 声明式 emitter 资产 host 面(作者面)——门 `g35.wave8.authoring`
//! (RFC-0049 §3/§4.11;契约事实源 = milestones/g35/G35_CONTRACT.md D-G35-8)。
//!
//! ## v1 十字段闭集(RFC-0049 §3 冻结,F17;禁增删)
//!
//! JSON 资产(host 单源;加性扩展走 schema MINOR 修订非 stable):
//! `name`(str 非空)/ `pos`/`spread`/`vel_base`/`vel_spread`([f32;3])/
//! `life_base`(f32 > 0)/ `gravity_y`(f32)/ `emit_curve`
//! (`{"kind":"const","value":f32≥0}` | `{"kind":"step","frames":[u32…严格递增],
//! "values":[f32≥0…]}`,frames/values 等长非空)/ `render`
//! (`"billboard"|"mesh"` 闭集)/ `blend`(`"additive"|"alpha"` 闭集)。
//!
//! ## fail-closed 纪律(禁默认值兜底)
//!
//! 未知字段([`AssetError::UnknownField`])/ 缺字段([`AssetError::MissingField`])/
//! 类型错([`AssetError::Type`])/ 闭集外枚举([`AssetError::EnumOutOfSet`])/
//! 数值域违约([`AssetError::Domain`])= typed Err;JSON 语法违例(含重复键/
//! 尾部余留/非有限数字字面)= [`AssetError::Json`]。零外部 crate:最小 JSON
//! 子集解析器为库面本地实现(`g14_3_lane_body.rs` bin-local 先例同型子集——
//! 该先例为 bin-local 不可复用,本文件按 G35-8 任务字面落库面;对象/数组/
//! 字符串/数字/字面量,重复键拒、控制字符拒、深度限 16)。
//!
//! ## 参数化映射与曲线求值(纯函数,确定)
//!
//! - [`EmitterAsset::to_desc`] → [`core::EmitterDesc`](super::core::EmitterDesc)
//!   六标量域(pos/spread/vel_base/vel_spread/life_base/gravity_y);
//! - [`EmitterAsset::emit_count_at`](frame) → 每帧 emit_count:const = 恒值
//!   取整(floor);step = 阶梯查表(最后一个 `frames[i] ≤ frame` 的
//!   `floor(values[i])`,`frame < frames[0]` 时 0)。v1 口径 = 逐帧取整
//!   (本门任务字面);RFC §3 「floor 累计差分」配额口径归 megakernel 装配面
//!   (G35 收口批),如实登记不冒充。
//! - `render`/`blend` v1 只做闭集校验与登记(类型面 [`RenderMode`]/
//!   [`BlendMode`]);渲染臂(billboard splat / mesh TLAS / 半透明双臂)归
//!   G35-3/G35-4 波,本文件不接线。
//!
//! ## 热重载语义([`EmitterRuntime::reload`])
//!
//! 资产重载 = **纯参数面变化**(EmitterDesc/curve 整体替换):粒子池/pid 序列
//! 由调用方(probe/SDK/车道)持有,重载**不重置**;曲线帧钟([`EmitterRuntime::frame`])
//! 单调不重置;`reload(asset)` 后**下一帧生效**(下一次 [`EmitterRuntime::next_emit_count`]
//! /下一次 `core::frame` 消费新 desc)。判别(probe/单测承载):重载前后轨迹
//! digest 必异 + 已存活粒子位置连续(重载不瞬移旧粒子——新 gravity 下冻结
//! 运算序单步重放 bitwise 全等)。
//!
//! ## 确定性
//!
//! 解析/曲线求值/重载全 host 纯函数;同资产双解析位级同构(单测 to_bits 面)。

use super::core::EmitterDesc;

/// 资产解析/校验 typed 错误闭集(fail-closed;禁默认值兜底)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    /// JSON 语法/结构违例(重复键/尾部余留/深度越界/非有限数字字面同归)。
    Json(String),
    /// 顶层非对象。
    NotObject,
    /// 十字段闭集缺字段(嵌套面记 `emit_curve.<k>` 路径)。
    MissingField(String),
    /// 闭集外未知字段(嵌套面记 `emit_curve.<k>` 路径)。
    UnknownField(String),
    /// 字段类型错(expected = 期待类型字面)。
    Type {
        /// 字段路径。
        field: String,
        /// 期待类型字面。
        expected: &'static str,
    },
    /// 枚举值越闭集(render/blend/emit_curve.kind)。
    EnumOutOfSet {
        /// 字段路径。
        field: String,
        /// 实得值。
        got: String,
    },
    /// 数值域违约(非有限/负值/空表/长度不等/非严格递增)。
    Domain {
        /// 字段路径。
        field: String,
        /// 违约说明。
        why: String,
    },
}

impl AssetError {
    /// 错误类名(probe typed 退出 token / 单测判别面;闭集七类)。
    pub fn kind_name(&self) -> &'static str {
        match self {
            AssetError::Json(_) => "Json",
            AssetError::NotObject => "NotObject",
            AssetError::MissingField(_) => "MissingField",
            AssetError::UnknownField(_) => "UnknownField",
            AssetError::Type { .. } => "Type",
            AssetError::EnumOutOfSet { .. } => "EnumOutOfSet",
            AssetError::Domain { .. } => "Domain",
        }
    }
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Json(m) => write!(f, "Json: {m}"),
            AssetError::NotObject => write!(f, "NotObject: 顶层须为对象"),
            AssetError::MissingField(k) => write!(f, "MissingField: 缺字段 {k}"),
            AssetError::UnknownField(k) => write!(f, "UnknownField: 闭集外字段 {k}"),
            AssetError::Type { field, expected } => {
                write!(f, "Type: 字段 {field} 期待 {expected}")
            }
            AssetError::EnumOutOfSet { field, got } => {
                write!(f, "EnumOutOfSet: 字段 {field} 值 {got:?} 越闭集")
            }
            AssetError::Domain { field, why } => write!(f, "Domain: 字段 {field} {why}"),
        }
    }
}

/// 渲染档闭集(v1 只做校验与登记;渲染臂归 G35-3〔billboard/mesh TLAS〕波)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// 相机朝向四边形 splat 臂。
    Billboard,
    /// 实例网格粒子 TLAS 臂。
    Mesh,
}

/// 混合档闭集(v1 只做校验与登记;半透明双臂归 G35-4 波)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    /// 加性(定点累加臂,RFC-0049 §4.6)。
    Additive,
    /// 半透明(排序/WBOIT 双臂,RFC-0049 §4.8)。
    Alpha,
}

/// 发射曲线闭集(v1:const | step;求值 = [`EmitterAsset::emit_count_at`])。
#[derive(Clone, Debug, PartialEq)]
pub enum EmitCurve {
    /// 恒值:每帧 emit_count = floor(value)。
    Const {
        /// 每帧配额(f32 ≥ 0 有限;解析期校验)。
        value: f32,
    },
    /// 阶梯查表:帧 f 取最后一个 `frames[i] ≤ f` 的 `floor(values[i])`;
    /// `f < frames[0]` 时 0。frames 严格递增非空,values 等长。
    Step {
        /// 阶梯起始帧表(u32 严格递增非空)。
        frames: Vec<u32>,
        /// 阶梯值表(与 frames 等长;f32 ≥ 0 有限)。
        values: Vec<f32>,
    },
}

/// 声明式 emitter 资产(v1 十字段闭集,RFC-0049 §3 冻结)。
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterAsset {
    /// 资产名(非空;登记/诊断面,不入 EmitterDesc)。
    pub name: String,
    /// 发射中心(EmitterDesc.pos)。
    pub pos: [f32; 3],
    /// 位置半幅(EmitterDesc.spread)。
    pub spread: [f32; 3],
    /// 初速基值(EmitterDesc.vel_base)。
    pub vel_base: [f32; 3],
    /// 初速半幅(EmitterDesc.vel_spread)。
    pub vel_spread: [f32; 3],
    /// 寿命基值(> 0;EmitterDesc.life_base)。
    pub life_base: f32,
    /// 重力 y(v1 唯一积分外力;EmitterDesc.gravity_y)。
    pub gravity_y: f32,
    /// 发射曲线(每帧 emit_count 求值面)。
    pub emit_curve: EmitCurve,
    /// 渲染档(v1 闭集校验登记;渲染臂归 G35-3)。
    pub render: RenderMode,
    /// 混合档(v1 闭集校验登记;半透明臂归 G35-4)。
    pub blend: BlendMode,
}

// ---------------------------------------------------------------------------
// 最小 JSON 子集解析(库面本地;g14_3_lane_body bin-local 先例同型子集——
// 对象/数组/字符串/数字/true/false/null,重复键拒,深度限 16,fail-closed)
// ---------------------------------------------------------------------------

/// JSON 值(解析中间面;数字保留 raw + integral 供 u32 域判)。
#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num { raw: String, v: f64, integral: bool },
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct JParser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl JParser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.i < self.b.len() && self.b[self.i] == c {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("期待 '{}' @{}", c as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        if self.depth >= 16 {
            return Err("嵌套深度越 16".into());
        }
        self.ws();
        let Some(&c) = self.b.get(self.i) else {
            return Err("意外结尾".into());
        };
        match c {
            b'{' => {
                self.i += 1;
                self.depth += 1;
                let mut pairs: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Obj(pairs));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    if pairs.iter().any(|(ek, _)| ek == &k) {
                        return Err(format!("重复键 {k}"));
                    }
                    self.expect(b':')?;
                    let v = self.value()?;
                    pairs.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("对象缺 ,/}".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Obj(pairs))
            }
            b'[' => {
                self.i += 1;
                self.depth += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("数组缺 ,/]".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Arr(items))
            }
            b'"' => Ok(Json::Str(self.string()?)),
            b't' => self.lit("true", Json::Bool(true)),
            b'f' => self.lit("false", Json::Bool(false)),
            b'n' => self.lit("null", Json::Null),
            _ => self.number(),
        }
    }

    fn lit(&mut self, s: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            Err(format!("字面 {s} 不符 @{}", self.i))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("期待字符串 @{}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return Err("字符串未闭合".into());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return Err("转义未闭合".into());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        // 资产域足够子集:\b \f \uXXXX 不支持(fail-closed 拒)。
                        _ => return Err("非法/不支持转义(资产子集:\\\" \\\\ \\/ \\n \\r \\t)".into()),
                    }
                }
                0x00..=0x1F => return Err("未转义控制字符".into()),
                _ => {
                    // 原始 UTF-8 直透(输入为 &str 保证合法 UTF-8)。
                    let s = std::str::from_utf8(&self.b[self.i - 1..])
                        .map_err(|_| "UTF-8".to_string())?;
                    let ch = s.chars().next().ok_or_else(|| "字符串截断".to_string())?;
                    out.push(ch);
                    self.i += ch.len_utf8() - 1;
                }
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let mut saw_digit = false;
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
            saw_digit = true;
        }
        if !saw_digit {
            return Err(format!("非法数字 @{start}"));
        }
        let mut integral = true;
        if self.b.get(self.i) == Some(&b'.') {
            integral = false;
            self.i += 1;
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("小数点缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if matches!(self.b.get(self.i), Some(b'e') | Some(b'E')) {
            integral = false;
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("指数缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        let raw = std::str::from_utf8(&self.b[start..self.i])
            .map_err(|_| "数字 UTF-8".to_string())?
            .to_owned();
        let v: f64 = raw.parse().map_err(|_| format!("数字解析 {raw}"))?;
        if !v.is_finite() {
            return Err(format!("数字 {raw} 非有限"));
        }
        Ok(Json::Num { raw, v, integral })
    }
}

fn json_parse(text: &str) -> Result<Json, AssetError> {
    let mut p = JParser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let v = p.value().map_err(AssetError::Json)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(AssetError::Json("尾部余留字节".into()));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// 十字段闭集校验(fail-closed)
// ---------------------------------------------------------------------------

/// 十字段闭集(RFC-0049 §3 冻结序;禁增删)。
const FIELDS: [&str; 10] = [
    "name",
    "pos",
    "spread",
    "vel_base",
    "vel_spread",
    "life_base",
    "gravity_y",
    "emit_curve",
    "render",
    "blend",
];

fn get<'a>(pairs: &'a [(String, Json)], key: &str) -> Result<&'a Json, AssetError> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .ok_or_else(|| AssetError::MissingField(key.to_string()))
}

/// f64 → f32 域收窄(cast 后须有限;JSON 字面已排 NaN/Inf,超 f32 域字面在此拒)。
fn to_f32(field: &str, v: f64) -> Result<f32, AssetError> {
    let x = v as f32;
    if !x.is_finite() {
        return Err(AssetError::Domain {
            field: field.to_string(),
            why: format!("f32 域越界(字面 {v:e})"),
        });
    }
    Ok(x)
}

fn f32_scalar(field: &str, v: &Json) -> Result<f32, AssetError> {
    match v {
        Json::Num { v, .. } => to_f32(field, *v),
        _ => Err(AssetError::Type {
            field: field.to_string(),
            expected: "f32",
        }),
    }
}

fn f32x3(field: &str, v: &Json) -> Result<[f32; 3], AssetError> {
    let Json::Arr(items) = v else {
        return Err(AssetError::Type {
            field: field.to_string(),
            expected: "[f32;3]",
        });
    };
    if items.len() != 3 {
        return Err(AssetError::Type {
            field: field.to_string(),
            expected: "[f32;3](长度必须为 3)",
        });
    }
    let mut out = [0.0f32; 3];
    for (i, it) in items.iter().enumerate() {
        let Json::Num { v, .. } = it else {
            return Err(AssetError::Type {
                field: format!("{field}[{i}]"),
                expected: "f32",
            });
        };
        out[i] = to_f32(&format!("{field}[{i}]"), *v)?;
    }
    Ok(out)
}

fn str_field<'a>(field: &str, v: &'a Json) -> Result<&'a str, AssetError> {
    match v {
        Json::Str(s) => Ok(s),
        _ => Err(AssetError::Type {
            field: field.to_string(),
            expected: "string",
        }),
    }
}

/// emit_curve 子对象闭集校验(kind 分派;子键闭集随 kind 冻结)。
fn parse_curve(v: &Json) -> Result<EmitCurve, AssetError> {
    let Json::Obj(pairs) = v else {
        return Err(AssetError::Type {
            field: "emit_curve".to_string(),
            expected: "object",
        });
    };
    let kind = str_field("emit_curve.kind", get(pairs, "kind").map_err(|_| {
        AssetError::MissingField("emit_curve.kind".to_string())
    })?)?;
    match kind {
        "const" => {
            for (k, _) in pairs {
                if k != "kind" && k != "value" {
                    return Err(AssetError::UnknownField(format!("emit_curve.{k}")));
                }
            }
            let vv = get(pairs, "value")
                .map_err(|_| AssetError::MissingField("emit_curve.value".to_string()))?;
            let value = f32_scalar("emit_curve.value", vv)?;
            if value < 0.0 {
                return Err(AssetError::Domain {
                    field: "emit_curve.value".to_string(),
                    why: "须 ≥ 0".to_string(),
                });
            }
            Ok(EmitCurve::Const { value })
        }
        "step" => {
            for (k, _) in pairs {
                if k != "kind" && k != "frames" && k != "values" {
                    return Err(AssetError::UnknownField(format!("emit_curve.{k}")));
                }
            }
            let fj = get(pairs, "frames")
                .map_err(|_| AssetError::MissingField("emit_curve.frames".to_string()))?;
            let vj = get(pairs, "values")
                .map_err(|_| AssetError::MissingField("emit_curve.values".to_string()))?;
            let Json::Arr(fitems) = fj else {
                return Err(AssetError::Type {
                    field: "emit_curve.frames".to_string(),
                    expected: "[u32…]",
                });
            };
            let Json::Arr(vitems) = vj else {
                return Err(AssetError::Type {
                    field: "emit_curve.values".to_string(),
                    expected: "[f32…]",
                });
            };
            if fitems.is_empty() {
                return Err(AssetError::Domain {
                    field: "emit_curve.frames".to_string(),
                    why: "须非空".to_string(),
                });
            }
            if fitems.len() != vitems.len() {
                return Err(AssetError::Domain {
                    field: "emit_curve".to_string(),
                    why: format!("frames({}) 与 values({}) 长度不等", fitems.len(), vitems.len()),
                });
            }
            let mut frames = Vec::with_capacity(fitems.len());
            for (i, it) in fitems.iter().enumerate() {
                let Json::Num {
                    raw,
                    integral: true,
                    ..
                } = it
                else {
                    return Err(AssetError::Type {
                        field: format!("emit_curve.frames[{i}]"),
                        expected: "u32(整数字面)",
                    });
                };
                let n: u32 = raw.parse().map_err(|_| AssetError::Domain {
                    field: format!("emit_curve.frames[{i}]"),
                    why: format!("u32 越域({raw})"),
                })?;
                frames.push(n);
            }
            if !frames.windows(2).all(|w| w[0] < w[1]) {
                return Err(AssetError::Domain {
                    field: "emit_curve.frames".to_string(),
                    why: "须严格递增".to_string(),
                });
            }
            let mut values = Vec::with_capacity(vitems.len());
            for (i, it) in vitems.iter().enumerate() {
                let x = f32_scalar(&format!("emit_curve.values[{i}]"), it)?;
                if x < 0.0 {
                    return Err(AssetError::Domain {
                        field: format!("emit_curve.values[{i}]"),
                        why: "须 ≥ 0".to_string(),
                    });
                }
                values.push(x);
            }
            Ok(EmitCurve::Step { frames, values })
        }
        other => Err(AssetError::EnumOutOfSet {
            field: "emit_curve.kind".to_string(),
            got: other.to_string(),
        }),
    }
}

impl EmitterAsset {
    /// 解析 + 十字段闭集 fail-closed 校验(纯函数;同资产双解析位级同构)。
    pub fn parse(text: &str) -> Result<EmitterAsset, AssetError> {
        let doc = json_parse(text)?;
        let Json::Obj(pairs) = &doc else {
            return Err(AssetError::NotObject);
        };
        // 闭集:未知字段先拒(解析器已拒重复键)。
        for (k, _) in pairs {
            if !FIELDS.contains(&k.as_str()) {
                return Err(AssetError::UnknownField(k.clone()));
            }
        }
        // 缺字段(闭集全员必须在场;get 内 typed MissingField)。
        for f in FIELDS {
            let _ = get(pairs, f)?;
        }
        let name = str_field("name", get(pairs, "name")?)?.to_string();
        if name.is_empty() {
            return Err(AssetError::Domain {
                field: "name".to_string(),
                why: "须非空".to_string(),
            });
        }
        let pos = f32x3("pos", get(pairs, "pos")?)?;
        let spread = f32x3("spread", get(pairs, "spread")?)?;
        let vel_base = f32x3("vel_base", get(pairs, "vel_base")?)?;
        let vel_spread = f32x3("vel_spread", get(pairs, "vel_spread")?)?;
        let life_base = f32_scalar("life_base", get(pairs, "life_base")?)?;
        if life_base <= 0.0 {
            return Err(AssetError::Domain {
                field: "life_base".to_string(),
                why: "须 > 0".to_string(),
            });
        }
        let gravity_y = f32_scalar("gravity_y", get(pairs, "gravity_y")?)?;
        let emit_curve = parse_curve(get(pairs, "emit_curve")?)?;
        let render = match str_field("render", get(pairs, "render")?)? {
            "billboard" => RenderMode::Billboard,
            "mesh" => RenderMode::Mesh,
            other => {
                return Err(AssetError::EnumOutOfSet {
                    field: "render".to_string(),
                    got: other.to_string(),
                });
            }
        };
        let blend = match str_field("blend", get(pairs, "blend")?)? {
            "additive" => BlendMode::Additive,
            "alpha" => BlendMode::Alpha,
            other => {
                return Err(AssetError::EnumOutOfSet {
                    field: "blend".to_string(),
                    got: other.to_string(),
                });
            }
        };
        Ok(EmitterAsset {
            name,
            pos,
            spread,
            vel_base,
            vel_spread,
            life_base,
            gravity_y,
            emit_curve,
            render,
            blend,
        })
    }

    /// 参数化映射:资产 → host 金标准 [`EmitterDesc`](super::core::EmitterDesc)
    /// 六标量域(纯函数;name/render/blend/emit_curve 为资产级登记面不入 desc)。
    pub fn to_desc(&self) -> EmitterDesc {
        EmitterDesc {
            pos: self.pos,
            spread: self.spread,
            vel_base: self.vel_base,
            vel_spread: self.vel_spread,
            life_base: self.life_base,
            gravity_y: self.gravity_y,
        }
    }

    /// 曲线求值:帧 `frame` 的 emit_count(纯函数,确定)。const = floor(value);
    /// step = 阶梯查表(最后一个 `frames[i] ≤ frame` 的 floor(values[i]),
    /// `frame < frames[0]` 时 0)。值域 ≥ 0 由解析期保证;floor 后 `as u32`
    /// 饱和转换(容量钳制归调用方 `min(requested, cap − n_curr)`,RFC §4.4 F7)。
    pub fn emit_count_at(&self, frame: u32) -> u32 {
        match &self.emit_curve {
            EmitCurve::Const { value } => value.floor() as u32,
            EmitCurve::Step { frames, values } => {
                let mut out = 0u32;
                for (f, v) in frames.iter().zip(values.iter()) {
                    if *f <= frame {
                        out = v.floor() as u32;
                    } else {
                        break;
                    }
                }
                out
            }
        }
    }
}

/// 热重载语义承载:资产参数面 + 曲线帧钟。粒子池/pid 序列由调用方持有——
/// [`reload`](Self::reload) 只替换参数面(下一帧生效),帧钟/池/pid 不重置。
#[derive(Clone, Debug)]
pub struct EmitterRuntime {
    asset: EmitterAsset,
    frame: u32,
}

impl EmitterRuntime {
    /// 以已校验资产建运行时(帧钟自 0 起)。
    pub fn new(asset: EmitterAsset) -> Self {
        Self { asset, frame: 0 }
    }

    /// 当前资产参数面(desc/curve 消费入口)。
    pub fn asset(&self) -> &EmitterAsset {
        &self.asset
    }

    /// 标量域可变面(SDK `set_param` 闭集消费;结构性替换走 [`reload`](Self::reload))。
    pub fn asset_mut(&mut self) -> &mut EmitterAsset {
        &mut self.asset
    }

    /// 曲线帧钟(已求值帧数;reload 不重置)。
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// 热重载 = 纯参数面替换(EmitterDesc/curve 整体换新);帧钟不重置、
    /// 调用方持有的粒子池/pid 序列不重置——下一帧(下一次
    /// [`next_emit_count`](Self::next_emit_count)/下一次 `core::frame`)生效。
    pub fn reload(&mut self, asset: EmitterAsset) {
        self.asset = asset;
    }

    /// 求当前帧 emit 配额并推进帧钟(求值 = [`EmitterAsset::emit_count_at`]
    /// 纯函数;钟单调,reload 跨帧连续)。
    pub fn next_emit_count(&mut self) -> u32 {
        let c = self.asset.emit_count_at(self.frame);
        self.frame += 1;
        c
    }
}

#[cfg(test)]
mod tests {
    use super::super::core::{ParticlePools, frame};
    use super::super::rand_table;
    use super::*;
    use std::collections::HashMap;

    /// 合法样例(probe 内嵌契约样例 A 同形;单测独立字面不镜像)。
    fn legal_json() -> String {
        r#"{
  "name": "unit_fixture",
  "pos": [0.0, 1.0, -0.5],
  "spread": [0.4, 0.2, 0.4],
  "vel_base": [0.0, 3.0, 0.0],
  "vel_spread": [1.0, 0.5, 1.0],
  "life_base": 1.2,
  "gravity_y": -9.8,
  "emit_curve": {"kind": "const", "value": 24.7},
  "render": "billboard",
  "blend": "alpha"
}"#
        .to_string()
    }

    /// 合法样例改一处(字符串替换;测试夹具面)。
    fn mutated(from: &str, to: &str) -> String {
        let s = legal_json();
        assert!(s.contains(from), "夹具不含 {from}");
        s.replace(from, to)
    }

    /// f32 域位级快照(双解析位级同构判据面)。
    fn bits(a: &EmitterAsset) -> Vec<u32> {
        let mut out = Vec::new();
        for v in [a.pos, a.spread, a.vel_base, a.vel_spread] {
            out.extend(v.iter().map(|x| x.to_bits()));
        }
        out.push(a.life_base.to_bits());
        out.push(a.gravity_y.to_bits());
        match &a.emit_curve {
            EmitCurve::Const { value } => out.push(value.to_bits()),
            EmitCurve::Step { frames, values } => {
                out.extend(frames.iter().copied());
                out.extend(values.iter().map(|x| x.to_bits()));
            }
        }
        out
    }

    /// ① 合法样例:解析绿 + 字段逐一精确 + to_desc 映射全等。
    #[test]
    fn legal_asset_parses_and_maps() {
        let a = EmitterAsset::parse(&legal_json()).expect("合法样例必须解析绿");
        assert_eq!(a.name, "unit_fixture");
        assert_eq!(a.pos, [0.0, 1.0, -0.5]);
        assert_eq!(a.spread, [0.4, 0.2, 0.4]);
        assert_eq!(a.vel_base, [0.0, 3.0, 0.0]);
        assert_eq!(a.vel_spread, [1.0, 0.5, 1.0]);
        assert_eq!(a.life_base.to_bits(), 1.2f32.to_bits());
        assert_eq!(a.gravity_y.to_bits(), (-9.8f32).to_bits());
        assert_eq!(a.emit_curve, EmitCurve::Const { value: 24.7 });
        assert_eq!(a.render, RenderMode::Billboard);
        assert_eq!(a.blend, BlendMode::Alpha);
        let d = a.to_desc();
        assert_eq!(d.pos, a.pos);
        assert_eq!(d.spread, a.spread);
        assert_eq!(d.vel_base, a.vel_base);
        assert_eq!(d.vel_spread, a.vel_spread);
        assert_eq!(d.life_base.to_bits(), a.life_base.to_bits());
        assert_eq!(d.gravity_y.to_bits(), a.gravity_y.to_bits());
    }

    /// ② 十种非法逐一 typed Err(缺字段/多字段/类型错/闭集外枚举/域违约/
    /// 语法违例——fail-closed 禁默认值兜底)。
    #[test]
    fn ten_illegal_variants_each_typed_err() {
        // 1. 缺字段(去 life_base 行)。
        let e = EmitterAsset::parse(&mutated("  \"life_base\": 1.2,\n", "")).unwrap_err();
        assert_eq!(e, AssetError::MissingField("life_base".into()));
        // 2. 多字段(闭集外 drag)。
        let e = EmitterAsset::parse(&mutated(
            "\"gravity_y\": -9.8,",
            "\"gravity_y\": -9.8,\n  \"drag\": 0.1,",
        ))
        .unwrap_err();
        assert_eq!(e, AssetError::UnknownField("drag".into()));
        // 3. 类型错:pos 非数组。
        let e = EmitterAsset::parse(&mutated("[0.0, 1.0, -0.5]", "1.0")).unwrap_err();
        assert!(matches!(e, AssetError::Type { ref field, .. } if field == "pos"), "{e}");
        // 4. 类型错:name 非字符串。
        let e = EmitterAsset::parse(&mutated("\"unit_fixture\"", "42")).unwrap_err();
        assert!(matches!(e, AssetError::Type { ref field, .. } if field == "name"), "{e}");
        // 5. 闭集外枚举:render。
        let e = EmitterAsset::parse(&mutated("\"billboard\"", "\"sprite\"")).unwrap_err();
        assert_eq!(
            e,
            AssetError::EnumOutOfSet {
                field: "render".into(),
                got: "sprite".into()
            }
        );
        // 6. 闭集外枚举:blend。
        let e = EmitterAsset::parse(&mutated("\"alpha\"", "\"multiply\"")).unwrap_err();
        assert_eq!(
            e,
            AssetError::EnumOutOfSet {
                field: "blend".into(),
                got: "multiply".into()
            }
        );
        // 7. 闭集外枚举:emit_curve.kind。
        let e = EmitterAsset::parse(&mutated("\"const\"", "\"ramp\"")).unwrap_err();
        assert_eq!(
            e,
            AssetError::EnumOutOfSet {
                field: "emit_curve.kind".into(),
                got: "ramp".into()
            }
        );
        // 8. 缺嵌套字段:const 无 value。
        let e = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"const\"}",
        ))
        .unwrap_err();
        assert_eq!(e, AssetError::MissingField("emit_curve.value".into()));
        // 9. 域违约:step frames/values 长度不等。
        let e = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [0, 8], \"values\": [1.0]}",
        ))
        .unwrap_err();
        assert!(matches!(e, AssetError::Domain { ref field, .. } if field == "emit_curve"), "{e}");
        // 10. 域违约:step frames 非严格递增。
        let e = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [0, 8, 8], \"values\": [1.0, 2.0, 3.0]}",
        ))
        .unwrap_err();
        assert!(
            matches!(e, AssetError::Domain { ref field, .. } if field == "emit_curve.frames"),
            "{e}"
        );
    }

    /// ②b 追加非法面(语法/结构/域,同 fail-closed 闭集)。
    #[test]
    fn extra_illegal_variants_typed_err() {
        // JSON 语法违例。
        assert!(matches!(
            EmitterAsset::parse("{\"name\": ").unwrap_err(),
            AssetError::Json(_)
        ));
        // 重复键(解析器面拒)。
        assert!(matches!(
            EmitterAsset::parse("{\"name\": \"a\", \"name\": \"b\"}").unwrap_err(),
            AssetError::Json(_)
        ));
        // 顶层非对象。
        assert_eq!(
            EmitterAsset::parse("[1, 2]").unwrap_err(),
            AssetError::NotObject
        );
        // pos 长度 ≠ 3。
        let e = EmitterAsset::parse(&mutated("[0.0, 1.0, -0.5]", "[0.0, 1.0]")).unwrap_err();
        assert!(matches!(e, AssetError::Type { ref field, .. } if field == "pos"), "{e}");
        // life_base ≤ 0。
        let e = EmitterAsset::parse(&mutated("\"life_base\": 1.2", "\"life_base\": 0.0"))
            .unwrap_err();
        assert!(matches!(e, AssetError::Domain { ref field, .. } if field == "life_base"), "{e}");
        // emit_curve 闭集外子键(RFC §3 TOML 形 rate 不在 JSON v1 闭集)。
        let e = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"const\", \"value\": 24.7, \"rate\": 1.0}",
        ))
        .unwrap_err();
        assert_eq!(e, AssetError::UnknownField("emit_curve.rate".into()));
        // step frames 浮点字面(u32 域须整数字面)。
        let e = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [0.5], \"values\": [1.0]}",
        ))
        .unwrap_err();
        assert!(
            matches!(e, AssetError::Type { ref field, .. } if field == "emit_curve.frames[0]"),
            "{e}"
        );
        // emit_curve.value 负值。
        let e = EmitterAsset::parse(&mutated("24.7", "-1.0")).unwrap_err();
        assert!(
            matches!(e, AssetError::Domain { ref field, .. } if field == "emit_curve.value"),
            "{e}"
        );
        // kind_name 闭集覆盖(probe typed 退出 token 消费面)。
        assert_eq!(AssetError::NotObject.kind_name(), "NotObject");
        assert_eq!(AssetError::Json("x".into()).kind_name(), "Json");
    }

    /// ③ 曲线求值:const 恒值取整;step 阶梯查表(首帧前 0/边界精确/末段
    /// 恒值);双求值确定。
    #[test]
    fn curve_eval_deterministic_and_exact() {
        let a = EmitterAsset::parse(&legal_json()).unwrap();
        for f in [0u32, 1, 31, 63, 1000] {
            assert_eq!(a.emit_count_at(f), 24, "const 24.7 → floor 24 恒值");
            assert_eq!(a.emit_count_at(f), a.emit_count_at(f), "双求值必须确定");
        }
        let step = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [4, 8, 40], \"values\": [16.0, 48.9, 8.0]}",
        ))
        .unwrap();
        assert_eq!(step.emit_count_at(0), 0, "首阶梯前必须 0");
        assert_eq!(step.emit_count_at(3), 0);
        assert_eq!(step.emit_count_at(4), 16, "边界帧取本阶梯");
        assert_eq!(step.emit_count_at(7), 16);
        assert_eq!(step.emit_count_at(8), 48, "48.9 → floor 48");
        assert_eq!(step.emit_count_at(39), 48);
        assert_eq!(step.emit_count_at(40), 8);
        assert_eq!(step.emit_count_at(u32::MAX), 8, "末段恒值");
    }

    /// ④ 同资产双解析位级同构(struct 全等 + f32 域 to_bits 全等)。
    #[test]
    fn double_parse_bitwise_identical() {
        let t = legal_json();
        let a1 = EmitterAsset::parse(&t).unwrap();
        let a2 = EmitterAsset::parse(&t).unwrap();
        assert_eq!(a1, a2, "双解析 struct 必须全等");
        assert_eq!(bits(&a1), bits(&a2), "双解析 f32 域必须位级同构");
        let step_t = mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [0, 8], \"values\": [16.0, 48.9]}",
        );
        let s1 = EmitterAsset::parse(&step_t).unwrap();
        let s2 = EmitterAsset::parse(&step_t).unwrap();
        assert_eq!(bits(&s1), bits(&s2), "step 曲线双解析必须位级同构");
    }

    /// ⑤ 热重载连续性(借 core::frame 推进):pid 序列连续不重置 + 旧粒子
    /// 跨重载边界轨迹连续(新 gravity 下冻结运算序单步重放 bitwise)+
    /// 重载生效(轨迹 digest 必异)+ 帧钟不重置。
    #[test]
    fn hot_reload_pid_continuity_and_effectiveness() {
        let asset_a = EmitterAsset::parse(&legal_json()).unwrap();
        let asset_b = EmitterAsset::parse(&mutated("\"gravity_y\": -9.8", "\"gravity_y\": -3.0"))
            .unwrap();
        assert_ne!(
            asset_a.gravity_y.to_bits(),
            asset_b.gravity_y.to_bits(),
            "重载资产必须参数可分辨(判据有效性前提)"
        );
        let cap = 512usize;
        let dt = 1.0f32 / 60.0;
        let table = rand_table(7);
        // 冻结运算序单粒子重放(core.rs sim_step 同序;0..2 pos/3..5 vel/6 age)。
        fn advance(s: &mut [f32; 8], dt: f32, g: f32) {
            s[4] += g * dt;
            s[0] += s[3] * dt;
            s[1] += s[4] * dt;
            s[2] += s[5] * dt;
            s[6] += dt;
        }
        // 场景跑器:帧 0..frames,reload_at 处换 asset_b;返回(逐帧池位级
        // 序列摘要, 末帧 pid 集, 边界连续核验计数)。
        let run = |reload_at: Option<u32>| -> (Vec<u32>, u32, usize) {
            let mut rt = EmitterRuntime::new(asset_a.clone());
            let mut a = ParticlePools::with_capacity(cap);
            let mut b = ParticlePools::with_capacity(cap);
            let mut pid_base = 0u32;
            let mut trace: Vec<u32> = Vec::new();
            let mut prev: HashMap<u32, [f32; 8]> = HashMap::new();
            let mut continuity_checked = 0usize;
            for f in 0..12u32 {
                if reload_at == Some(f) {
                    rt.reload(asset_b.clone());
                    assert_eq!(rt.frame(), f, "reload 不得重置帧钟");
                }
                let g = rt.asset().gravity_y;
                let want = rt.next_emit_count() as usize;
                let emit = want.min(cap - a.n);
                let st = frame(&mut a, &mut b, &rt.asset().to_desc(), &table, dt, pid_base, emit);
                // pid 连续:新发射段精确区间 + 无重复 + 幸存 ⊆ 上帧 ∪ 新段。
                let mut cur: HashMap<u32, [f32; 8]> = HashMap::new();
                for i in 0..b.n {
                    let stt = [
                        b.pos_x[i], b.pos_y[i], b.pos_z[i], b.vel_x[i], b.vel_y[i], b.vel_z[i],
                        b.age[i], b.life[i],
                    ];
                    assert!(cur.insert(b.pid[i], stt).is_none(), "帧 {f}: pid 重复");
                }
                for (pid, now) in &cur {
                    if let Some(p) = prev.get(pid) {
                        // 旧粒子跨帧(含重载边界帧)连续:当前活跃 gravity 下
                        // 单步重放 bitwise 全等 = 不瞬移。
                        let mut want_s = *p;
                        advance(&mut want_s, dt, g);
                        for k in 0..7 {
                            assert_eq!(
                                want_s[k].to_bits(),
                                now[k].to_bits(),
                                "帧 {f}: pid {pid} 分量 {k} 跨帧不连续(重载瞬移检出)"
                            );
                        }
                        continuity_checked += 1;
                    } else {
                        assert!(
                            *pid >= pid_base && (*pid as usize) < pid_base as usize + emit,
                            "帧 {f}: pid {pid} 非幸存亦非本帧发射区间"
                        );
                    }
                }
                prev = cur;
                pid_base += emit as u32;
                trace.push(st.alive_total);
                trace.extend(b.pid[..b.n].iter().copied());
                trace.extend(b.pos_y[..b.n].iter().map(|x| x.to_bits()));
                std::mem::swap(&mut a, &mut b);
            }
            (trace, pid_base, continuity_checked)
        };
        let (t_reload, pid_reload, checked) = run(Some(6));
        let (t_baseline, pid_baseline, _) = run(None);
        assert!(checked > 100, "跨重载连续性核验样本量不足({checked})");
        assert_ne!(t_reload, t_baseline, "重载必须生效(轨迹必须可分辨)");
        assert_eq!(
            pid_reload, pid_baseline,
            "同曲线(A const)下 pid 发行数与基线一致——重载不重置 pid 序列"
        );
        // 双跑位级(重载场景确定性)。
        let (t2, _, _) = run(Some(6));
        assert_eq!(t_reload, t2, "重载场景双跑必须位级一致");
    }

    /// ⑥ EmitterRuntime 帧钟:next_emit_count 推进单调;reload 后曲线换新
    /// 且帧钟连续(step 曲线跨重载按全局帧继续查表)。
    #[test]
    fn runtime_clock_monotone_across_reload() {
        let a = EmitterAsset::parse(&legal_json()).unwrap();
        let b = EmitterAsset::parse(&mutated(
            "{\"kind\": \"const\", \"value\": 24.7}",
            "{\"kind\": \"step\", \"frames\": [0, 8, 40], \"values\": [16.0, 48.9, 8.0]}",
        ))
        .unwrap();
        let mut rt = EmitterRuntime::new(a);
        for f in 0..32u32 {
            assert_eq!(rt.frame(), f);
            assert_eq!(rt.next_emit_count(), 24);
        }
        rt.reload(b);
        assert_eq!(rt.frame(), 32, "reload 不重置帧钟");
        assert_eq!(rt.next_emit_count(), 48, "帧 32 落 step [8,40) 段 → 48(下一帧生效语义)");
        for _ in 33..40u32 {
            assert_eq!(rt.next_emit_count(), 48);
        }
        assert_eq!(rt.next_emit_count(), 8, "帧 40 进末段 → 8");
    }
}
