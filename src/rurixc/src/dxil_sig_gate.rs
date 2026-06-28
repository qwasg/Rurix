//! 强制签名一致性校验门 `signature_gate`(G2.2 PR-D2 分片 3,RXS-0159/0162;
//! RFC-0004 §4.4 比较域 / §4.6(a) ABI 不冻结)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`;未启用时整模块不编入 rurixc,
//! 默认(PTX)路径零漂移(R6.7)。
//!
//! # 职责
//! 比对**译后 DXIL 签名**([`DxilSignatures`],ISG1/OSG1)与 **MIR 意图签名**
//! (`&[IoSigElem]`),任何用户声明 / 可观察签名元素未保真即显式失败(strict-only,
//! P-01 / R2.3~R2.4),绝不静默通过。校验门是 B 路 codegen **不可裁剪**组成
//! (R2.5 / Property 5):无任何「跳过校验直接产物」的配置 / 开关 / env;校验失败的
//! 入口绝不返回成功、绝不产 golden。
//!
//! # 比对域(R2.2,做这些)
//! - **用户语义名**:`IoSigKind::Varying` / `Interpolate` 的 `field_name` 须在译后
//!   对应方向签名中以**等价语义名**出现,未退化为通用名(如 `color`→`TEXCOORD`)。
//! - **系统值(SV_*)**:`IoSigKind::Builtin(name)` 须映射到对应 DXIL 系统值
//!   (名 / sysvalue 任一命中即视为达成,如 `position`→`SV_Position`/`POS`)。
//! - **被使用输入**:声明的外部输入(`dir == In`)若在译后签名中缺失 / 被消除且不可
//!   等价保留 → [`SigGateError::SigDroppedInput`]。
//! - **阶段间 location / 链接配对**:以**语义名等价**为链接键核实(location 编号本身
//!   属 ABI,不比对);链接键缺失即报错。
//!
//! # 绝对不比对(R2.7 / Property 7 ABI 中立)
//! 寄存器编号([`SigElement::register`])、`index`、顺序、component mask、packing、
//! 字节偏移、容器 part 排序——均属外部 conformance(RFC-0004 §4.6(a))。校验门判定
//! 结果对它们的任意合规变化**必须不变**:本实现仅按名 / sysvalue / 被用性 / 链接键
//! **搜索**(非按位置),故天然满足 ABI 中立。
//!
//! # 🔒 禁区
//! `IoSigElem` / `MirIoType` 结构上仅可表达标量 / 向量,无法表达资源句柄 / 描述符 /
//! 采样器,故纹理访问语义在本层不可达;校验门不发明任何 ABI 二进制布局 / UB 契约。

pub mod signature_gate {
    use crate::mir::{IoDir, IoSigElem, IoSigKind};
    use crate::toolchain::{DxilSignatures, SigElement};

    /// 校验门失败(strict-only;任务7 已按真实可达类别映射 6xxx,emit 点在
    /// `dxil_codegen::emit_b_error`)。
    ///
    /// 任务7 落码核查(honor `registry/deferred.json`:RX6009 已被 RD-013 阶段 I/O
    /// body 数据流降级引用占用,不复用)——本枚举两变体改派下一空号:
    /// `SigMismatch` → `RX6011` `codegen.dxil_sig_mismatch`(输出方向未保真);
    /// `SigDroppedInput` → `RX6012` `codegen.dxil_sig_dropped_input`(声明输入被消除)。
    /// 本模块只定义错误语义,不直接发码、不改 `registry/error_codes.json`。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SigGateError {
        /// 用户声明 / 可观察签名元素在译后 DXIL 中缺失 / 改名 / 错配 / 静默改写
        /// (输出方向元素未保真;输入方向的「改名 / 错配」亦归此)。
        SigMismatch {
            /// 失配的诊断上下文(语义名 / 系统值 / 方向)。
            detail: String,
        },
        /// 源码声明的外部输入(`dir == In`)在译后签名中被消除且不可等价保留
        /// (含「声明但未用」被 DCE 的情形;R2.4)。
        SigDroppedInput {
            /// 被消除输入的诊断上下文。
            detail: String,
        },
    }

    impl std::fmt::Display for SigGateError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SigGateError::SigMismatch { detail } => {
                    write!(f, "DXIL 签名不一致(用户语义名/系统值未保真): {detail}")
                }
                SigGateError::SigDroppedInput { detail } => {
                    write!(f, "DXIL 声明输入被消除且不可等价保留: {detail}")
                }
            }
        }
    }

    impl std::error::Error for SigGateError {}

    /// 阶段间接口链接核对失败(RXS-0160;strict-only)。
    ///
    /// **错误码(G2.3 PR-E2b-2 已落,owner 裁定方案 B)**:错链经
    /// [`crate::dxil_codegen::emit_stage_link_error`] 落 `RX6014`
    /// `codegen.dxil_stage_link_mismatch`——owner 裁定**新开 RX6014**(6xxx 段当时下一
    /// 空号;`RX6008`/`RX6009` 分别由 RD-012/RD-013 预留不复用),**不**复用 RX6011
    /// (RX6011 = 单阶段输出签名不一致,语义不同)。本枚举只定义链接核对的失败语义,
    /// **不**直接发码、**不**改 `registry/error_codes.json`(发码在 dxil_codegen 边界)。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum StageLinkError {
        /// fragment 输入 varying 在上游 vertex 输出中无同**语义名等价**链接键
        /// (错链:缺链接,RXS-0160 L2)。
        Unlinked {
            /// 失链的诊断上下文(fragment 输入语义名 / 类型)。
            detail: String,
        },
        /// 语义名等价的链接键两端**类型不一致**或**插值限定不一致**
        /// (错链:类型 / 插值失配,RXS-0160 L3)。
        LinkMismatch {
            /// 失配的诊断上下文(语义名 / 两端类型或插值限定)。
            detail: String,
        },
    }

    impl std::fmt::Display for StageLinkError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                StageLinkError::Unlinked { detail } => {
                    write!(
                        f,
                        "DXIL 阶段间接口错链(fragment 输入无上游 vertex 输出链接键): {detail}"
                    )
                }
                StageLinkError::LinkMismatch { detail } => {
                    write!(f, "DXIL 阶段间接口错链(链接键类型/插值限定失配): {detail}")
                }
            }
        }
    }

    impl std::error::Error for StageLinkError {}

    /// 强制签名一致性校验门(不可裁剪,无旁路)。
    ///
    /// 比对译后 DXIL 签名 `actual`(ISG1/OSG1)与 MIR 意图签名 `intent`:
    /// - 用户语义名(varying/interpolate)须在对应方向以等价语义名出现;
    /// - 系统值(builtin)须映射到对应 DXIL 系统值;
    /// - 声明的外部输入(`dir == In`)缺失 / 被消除 → [`SigGateError::SigDroppedInput`];
    /// - 输出方向元素缺失 / 改名 → [`SigGateError::SigMismatch`]。
    ///
    /// **不**比对寄存器号 / index / 顺序 / mask / packing / 字节偏移 / part 排序
    /// (R2.7 / Property 7 ABI 中立):仅按名 / sysvalue / 被用性搜索,顺序无关。
    ///
    /// # Errors
    /// 任一用户声明 / 可观察元素未保真 → 对应 [`SigGateError`](strict-only,
    /// 上层映射 6xxx 并终止该入口产物)。
    pub fn check(actual: &DxilSignatures, intent: &[IoSigElem]) -> Result<(), SigGateError> {
        for elem in intent {
            let sig = match elem.dir {
                IoDir::In => &actual.input,
                IoDir::Out => &actual.output,
            };
            match &elem.kind {
                // 系统值:须映射到对应 DXIL 系统值(名 / sysvalue 任一命中)。
                IoSigKind::Builtin(name) => {
                    let found = sig.iter().any(|e| sysvalue_matches(e, name));
                    if !found {
                        return Err(missing_error(elem, &builtin_detail(elem, name)));
                    }
                }
                // 用户语义名:varying/interpolate 须以等价语义名出现,未退化为通用名。
                IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                    let found = sig
                        .iter()
                        .any(|e| semantic_name_matches(&e.name, &elem.field_name));
                    if !found {
                        return Err(missing_error(elem, &varying_detail(elem)));
                    }
                }
            }
        }
        Ok(())
    }

    /// 阶段间接口 → 阶段链接一致性核对(RXS-0160;不可裁剪,无旁路,strict-only)。
    ///
    /// 以**语义名等价**为链接键(复用 [`semantic_name_matches`],大小写无关 + 剥语义
    /// index 后缀,对齐 RXS-0159)核实 vertex 输出方向 varying/interpolate(`vs_out_sig`
    /// 中 `dir == Out`)与 fragment 输入方向 varying/interpolate(`fs_in_sig` 中
    /// `dir == In`)的跨阶段配对:
    /// - fragment 输入 varying 无同语义名链接键 → [`StageLinkError::Unlinked`];
    /// - 链接键两端类型不一致 / 插值限定不一致 → [`StageLinkError::LinkMismatch`]。
    ///
    /// **不**比对 location 编号 / 寄存器 / mask(ABI 中立,RFC-0004 §4.6(a);对齐
    /// [`check`] 的 Property 7)。builtin 系统值(`position`/`frag_coord` 等)为阶段内
    /// 系统值(经光栅器,非跨阶段用户 varying),**不**参与链接核对(RXS-0160 L1)。
    ///
    /// # Errors
    /// 任一 fragment 输入 varying 错链 → 对应 [`StageLinkError`](strict-only;上层映射
    /// 6xxx 并终止该联编产物,错误码归类待 owner 裁,见 [`StageLinkError`])。
    pub fn check_stage_link(
        vs_out_sig: &[IoSigElem],
        fs_in_sig: &[IoSigElem],
    ) -> Result<(), StageLinkError> {
        // vertex 输出方向的跨阶段 varying(builtin 系统值不参与链接核对,L1)。
        let vs_varyings: Vec<&IoSigElem> = vs_out_sig
            .iter()
            .filter(|e| matches!(e.dir, IoDir::Out) && is_link_varying(&e.kind))
            .collect();
        // fragment 输入方向的跨阶段 varying,逐个核实链接键。
        for fin in fs_in_sig
            .iter()
            .filter(|e| matches!(e.dir, IoDir::In) && is_link_varying(&e.kind))
        {
            // 链接键 = 语义名等价(不比对 location,ABI 中立)。
            let Some(vout) = vs_varyings
                .iter()
                .find(|v| semantic_name_matches(&v.field_name, &fin.field_name))
            else {
                return Err(StageLinkError::Unlinked {
                    detail: format!(
                        "fragment 输入 varying `{}`(类型 {:?})在上游 vertex 输出中无同语义名链接键",
                        fin.field_name, fin.ty
                    ),
                });
            };
            // 类型一致性(MirIoType 全等;不取 location/mask)。
            if vout.ty != fin.ty {
                return Err(StageLinkError::LinkMismatch {
                    detail: format!(
                        "链接键 `{}` 两端类型失配:vertex 输出 {:?} ↔ fragment 输入 {:?}",
                        fin.field_name, vout.ty, fin.ty
                    ),
                });
            }
            // 插值限定一致性(Varying↔Varying / Interpolate(x)↔Interpolate(x))。
            if !interp_matches(&vout.kind, &fin.kind) {
                return Err(StageLinkError::LinkMismatch {
                    detail: format!(
                        "链接键 `{}` 两端插值限定失配:vertex 输出 {:?} ↔ fragment 输入 {:?}",
                        fin.field_name, vout.kind, fin.kind
                    ),
                });
            }
        }
        Ok(())
    }

    /// 跨阶段链接 varying 判定:仅 `Varying` / `Interpolate(_)` 参与链接核对;
    /// `Builtin` 系统值为阶段内(经光栅器),不参与(RXS-0160 L1)。
    fn is_link_varying(kind: &IoSigKind) -> bool {
        matches!(kind, IoSigKind::Varying | IoSigKind::Interpolate(_))
    }

    /// 插值限定一致性:`Varying`↔`Varying` 或 `Interpolate(x)`↔`Interpolate(x)`
    /// (同模式名)视为一致;`Varying`↔`Interpolate` 或不同插值模式 → 失配。
    fn interp_matches(a: &IoSigKind, b: &IoSigKind) -> bool {
        match (a, b) {
            (IoSigKind::Varying, IoSigKind::Varying) => true,
            (IoSigKind::Interpolate(x), IoSigKind::Interpolate(y)) => x == y,
            _ => false,
        }
    }

    /// 缺失元素的错误归类(判定边界,见模块/任务报告):
    /// - `dir == In`:声明的外部输入缺失 → [`SigGateError::SigDroppedInput`]。
    ///   `IoSigElem` 不携 `used` 标志,无法区分「源码本就未用」与「被错误消除」,
    ///   故**向上取严**(倾向 SigDroppedInput,R6.8 / 设计决策2)。
    /// - `dir == Out`:输出方向元素缺失 / 改名 → [`SigGateError::SigMismatch`]。
    fn missing_error(elem: &IoSigElem, detail: &str) -> SigGateError {
        match elem.dir {
            IoDir::In => SigGateError::SigDroppedInput {
                detail: detail.to_owned(),
            },
            IoDir::Out => SigGateError::SigMismatch {
                detail: detail.to_owned(),
            },
        }
    }

    fn builtin_detail(elem: &IoSigElem, name: &str) -> String {
        format!(
            "builtin `{name}`(field `{}`, dir {:?})的系统值未在译后 {} 签名出现",
            elem.field_name,
            elem.dir,
            dir_label(elem.dir),
        )
    }

    fn varying_detail(elem: &IoSigElem) -> String {
        format!(
            "用户语义名 `{}`(dir {:?})未在译后 {} 签名以等价名出现(疑退化为通用名)",
            elem.field_name,
            elem.dir,
            dir_label(elem.dir),
        )
    }

    fn dir_label(dir: IoDir) -> &'static str {
        match dir {
            IoDir::In => "输入(ISG1)",
            IoDir::Out => "输出(OSG1)",
        }
    }

    /// 用户语义名等价:大小写无关,剥离 DXIL 名尾随数字(语义 index 后缀,如
    /// `COLOR0`→`COLOR`)后与 `field_name` 全等。能识别退化为通用名(如声明 `color`
    /// 但译后为 `TEXCOORD0` → 不等 → 失配)。**不**比对 index 数字本身(属 ABI 维度)。
    fn semantic_name_matches(dxil_name: &str, field_name: &str) -> bool {
        let lhs = strip_trailing_digits(dxil_name).to_ascii_uppercase();
        let rhs = field_name.trim().to_ascii_uppercase();
        !rhs.is_empty() && lhs == rhs
    }

    /// 系统值命中:MIR builtin 名映射到 DXIL 系统值 token 集合,与该元素的 `name`
    /// 或 `sysvalue`(均大写、剥尾随数字)任一相等即命中。兼容注释表缩写(`POS`/
    /// `VERTID`)与元数据全名(`SV_Position`)两种形态。
    fn sysvalue_matches(e: &SigElement, builtin_name: &str) -> bool {
        let tokens = builtin_sv_tokens(builtin_name);
        let cand_name = strip_trailing_digits(&e.name).to_ascii_uppercase();
        let cand_sv = strip_trailing_digits(&e.sysvalue).to_ascii_uppercase();
        tokens.iter().any(|t| cand_name == *t || cand_sv == *t)
    }

    /// MIR builtin 名 → 可接受的 DXIL 系统值 token(全大写,已剥尾随数字)。
    /// 兼容 DXIL 注释表缩写(`POS`/`VERTID`/`INSTID`/`DEPTH`)与元数据全名
    /// (`SV_POSITION` 等)。未建模名(编码器本应已拒)防御性派生 `SV_<UPPER>`。
    fn builtin_sv_tokens(name: &str) -> Vec<String> {
        let toks: &[&str] = match name {
            // 裁剪空间位置(vertex out)/ 窗口空间坐标(fragment in)。
            "position" | "frag_coord" => &["SV_POSITION", "POS"],
            // 顶点 / 实例索引(vertex in)。
            "vertex_index" => &["SV_VERTEXID", "VERTID"],
            "instance_index" => &["SV_INSTANCEID", "INSTID"],
            // 片元深度(fragment out)。
            "frag_depth" | "depth" => &["SV_DEPTH", "DEPTH"],
            // 点尺寸(D3D 无独立系统值缩写,SPIR-V PointSize 不落 DXIL SV;按全名核实,
            // 真跑下通常不达 → strict-only 倾向报错,留待带工具链环境细化)。
            "point_size" => &["SV_POINTSIZE"],
            other => {
                return vec![format!("SV_{}", other.to_ascii_uppercase())];
            }
        };
        toks.iter().map(|t| (*t).to_owned()).collect()
    }

    /// 剥离尾随 ASCII 数字(语义 index 后缀,如 `TEXCOORD0`→`TEXCOORD`)。
    fn strip_trailing_digits(s: &str) -> &str {
        s.trim_end_matches(|c: char| c.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::signature_gate::{SigGateError, StageLinkError, check, check_stage_link};
    use crate::hir::PrimTy;
    use crate::mir::{IoDir, IoSigElem, IoSigKind, MirIoType};
    use crate::toolchain::{DxilSignatures, SigElement};

    /// 便捷构造译后 DXIL [`SigElement`](register/index 为 ABI 维度,测试中随意取)。
    fn sig(name: &str, sysvalue: &str, index: u32, register: &str, used: bool) -> SigElement {
        SigElement {
            name: name.to_owned(),
            index,
            sysvalue: sysvalue.to_owned(),
            register: register.to_owned(),
            used,
        }
    }

    /// 便捷构造 MIR 意图 [`IoSigElem`]。
    fn io(name: &str, kind: IoSigKind, ty: MirIoType, dir: IoDir) -> IoSigElem {
        IoSigElem {
            field_name: name.to_owned(),
            kind,
            ty,
            dir,
        }
    }

    /// 最小 vertex 意图:position(builtin out)+ color(varying out)+
    /// vertex_index(builtin in)。
    fn vertex_intent() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
        ]
    }

    /// 与 [`vertex_intent`] 保真的译后签名:输出 SV_Position + COLOR0,输入 VERTID。
    fn vertex_faithful_sigs() -> DxilSignatures {
        DxilSignatures {
            output: vec![
                sig("SV_Position", "POS", 0, "0", true),
                sig("COLOR", "NONE", 0, "1", true),
            ],
            input: vec![sig("SV_VertexID", "VERTID", 0, "0", true)],
        }
    }

    /// accept(工具无关):保名一致 + SV 真达(elemcount>0)→ `check` 返回 `Ok`。
    //@ spec: RXS-0159
    #[test]
    fn accept_faithful_signature_passes() {
        let intent = vertex_intent();
        let sigs = vertex_faithful_sigs();
        assert!(
            check(&sigs, &intent).is_ok(),
            "保名一致 + SV 真达应通过校验门"
        );
    }

    /// accept:系统值以注释表缩写(POS/VERTID)出现亦命中(双形态兼容)。
    //@ spec: RXS-0159
    #[test]
    fn accept_sysvalue_abbrev_form() {
        // name 列为空,系统值仅在 sysvalue 缩写列(注释表常见形态)。
        let sigs = DxilSignatures {
            output: vec![
                sig("", "POS", 0, "0", true),
                sig("COLOR", "NONE", 0, "1", true),
            ],
            input: vec![sig("", "VERTID", 0, "0", true)],
        };
        assert!(check(&sigs, &vertex_intent()).is_ok(), "缩写系统值应命中");
    }

    /// reject:输出 varying 语义名退化为通用名(color→TEXCOORD0)→ `SigMismatch`。
    //@ spec: RXS-0159
    #[test]
    fn reject_renamed_varying_is_sig_mismatch() {
        let mut sigs = vertex_faithful_sigs();
        // color → TEXCOORD0(退化为通用名)。
        sigs.output[1] = sig("TEXCOORD", "NONE", 0, "1", true);
        match check(&sigs, &vertex_intent()) {
            Err(SigGateError::SigMismatch { .. }) => {}
            other => panic!("改名 varying 应 SigMismatch,实得 {other:?}"),
        }
    }

    /// reject:输出系统值缺失(SV_Position 不在输出)→ `SigMismatch`。
    //@ spec: RXS-0159
    #[test]
    fn reject_missing_output_sysvalue_is_sig_mismatch() {
        let mut sigs = vertex_faithful_sigs();
        sigs.output.remove(0); // 去掉 SV_Position。
        match check(&sigs, &vertex_intent()) {
            Err(SigGateError::SigMismatch { .. }) => {}
            other => panic!("缺失输出系统值应 SigMismatch,实得 {other:?}"),
        }
    }

    /// reject:声明的外部输入(vertex_index,dir In)被消除 → `SigDroppedInput`
    /// (trivial passthrough DCE 红例域,R2.4)。
    //@ spec: RXS-0159
    #[test]
    fn reject_dropped_input_is_sig_dropped_input() {
        let mut sigs = vertex_faithful_sigs();
        sigs.input.clear(); // 输入被 DCE 消除。
        match check(&sigs, &vertex_intent()) {
            Err(SigGateError::SigDroppedInput { .. }) => {}
            other => panic!("声明输入被消除应 SigDroppedInput,实得 {other:?}"),
        }
    }

    /// reject:声明的 varying 输入被消除 → `SigDroppedInput`。
    //@ spec: RXS-0159
    #[test]
    fn reject_dropped_varying_input() {
        let intent = vec![io(
            "in_uv",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 2),
            IoDir::In,
        )];
        let sigs = DxilSignatures::default(); // 输入空(被消除)。
        match check(&sigs, &intent) {
            Err(SigGateError::SigDroppedInput { .. }) => {}
            other => panic!("varying 输入被消除应 SigDroppedInput,实得 {other:?}"),
        }
    }

    /// ABI 中立(Property 7):仅改 register/index/顺序,不改名/sysvalue/used →
    /// 判定不变(仍 `Ok`)。
    //@ spec: RXS-0162
    #[test]
    fn abi_neutral_register_index_order_invariant() {
        let intent = vertex_intent();
        // 基线:保真 → Ok。
        assert!(check(&vertex_faithful_sigs(), &intent).is_ok());

        // 变体:打乱输出顺序 + 任意改 register/index(ABI 维度),名/sysvalue/used 不变。
        let mutated = DxilSignatures {
            output: vec![
                // 顺序与基线相反,register/index 任意改。
                sig("COLOR", "NONE", 7, "42", true),
                sig("SV_Position", "POS", 3, "99", true),
            ],
            input: vec![sig("SV_VertexID", "VERTID", 5, "13", true)],
        };
        assert!(
            check(&mutated, &intent).is_ok(),
            "仅改 register/index/顺序不应改变校验门判定(ABI 中立)"
        );
    }

    /// ABI 中立续:语义 index 后缀(COLOR0 vs COLOR1)不影响名等价判定。
    //@ spec: RXS-0162
    #[test]
    fn abi_neutral_semantic_index_suffix_invariant() {
        let intent = vec![io(
            "color",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::Out,
        )];
        for dxil_name in ["COLOR", "COLOR0", "COLOR1", "color2"] {
            let sigs = DxilSignatures {
                output: vec![sig(dxil_name, "NONE", 0, "0", true)],
                input: Vec::new(),
            };
            assert!(
                check(&sigs, &intent).is_ok(),
                "`{dxil_name}` 应与 `color` 等价(剥尾随数字 + 大小写无关)"
            );
        }
    }

    // ════════════════════ 手写 PBT(任务6;无 proptest/quickcheck 依赖) ════════════════════
    //
    // 本仓库无属性测试框架,沿用 `dxil_spirv.rs::property1_*` 的「程序化合成 + 突变循环
    // + 断言」风格,可执行地证明:
    //   - **Property 4(校验门完备性)**:对**任意**「未保真」产物,`check` 必 `Err`、绝不
    //     `Ok`(`property4_any_unfaithful_mutation_is_rejected`)。
    //   - **Property 7(ABI 中立)**:对保真基线仅施加 ABI 维度(顺序 / register / index /
    //     语义 index 后缀)变化,`check` 仍 `Ok`——证明突变器测的是「未保真」而非「布局变化」
    //     (`property7_abi_only_remix_still_accepts`)。
    //
    // 「不旁路」(Property 5)的可执行佐证落在 `dxil_codegen.rs` 的 B 链接缝侧
    // (`property5_siggate_failure_routes_to_6xxx_never_silent`):校验门无 skip 参数,
    // 失败经既有诊断通道落 6xxx,绝不静默 / 产物。

    // ── 保真基线集(accept 基线;每元素语义在同方向内唯一,避免突变误命中) ──

    /// fragment 意图:in_color(interpolate flat,in)+ frag_coord(builtin,in)+
    /// out_color(varying,out)。
    fn fragment_intent() -> Vec<IoSigElem> {
        vec![
            io(
                "in_color",
                IoSigKind::Interpolate("flat".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ]
    }

    /// 多元素 vertex 意图:position(builtin out)+ normal(varying out)+
    /// uv(interpolate out)+ instance_index(builtin in)+ in_pos(varying in)。
    fn vs_multi_intent() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
            io(
                "instance_index",
                IoSigKind::Builtin("instance_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
            io(
                "in_pos",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
        ]
    }

    /// 保真基线集合(tag, intent)。每个基线经 [`synth_actual`] 合成的译后签名必 `Ok`。
    fn faithful_baselines() -> Vec<(&'static str, Vec<IoSigElem>)> {
        vec![
            ("vertex", vertex_intent()),
            ("fragment", fragment_intent()),
            ("vs_multi", vs_multi_intent()),
        ]
    }

    /// MIR builtin 名 → 一组保真的译后(DXIL 名, sysvalue)。仅覆盖基线用到的 builtin。
    fn builtin_pair(name: &str) -> (&'static str, &'static str) {
        match name {
            "position" | "frag_coord" => ("SV_Position", "POS"),
            "vertex_index" => ("SV_VertexID", "VERTID"),
            "instance_index" => ("SV_InstanceID", "INSTID"),
            "frag_depth" | "depth" => ("SV_Depth", "DEPTH"),
            other => panic!("baseline 用到未建模 builtin `{other}`(需补 builtin_pair 助手)"),
        }
    }

    /// 由单个意图元素**程序化合成**一个保真译后 [`SigElement`](by-construction 保名 /
    /// 系统值;register/index 取 ABI 占位值,校验门不比对)。
    fn synth_elem(elem: &IoSigElem) -> SigElement {
        match &elem.kind {
            IoSigKind::Builtin(name) => {
                let (n, sv) = builtin_pair(name);
                sig(n, sv, 0, "0", true)
            }
            IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                sig(&elem.field_name.to_ascii_uppercase(), "NONE", 0, "0", true)
            }
        }
    }

    /// 由意图集合合成**完全保真**的译后签名(accept 合成基线)。
    fn synth_actual(intent: &[IoSigElem]) -> DxilSignatures {
        let mut input = Vec::new();
        let mut output = Vec::new();
        for elem in intent {
            let se = synth_elem(elem);
            match elem.dir {
                IoDir::In => input.push(se),
                IoDir::Out => output.push(se),
            }
        }
        DxilSignatures { input, output }
    }

    /// 单点「未保真」突变种类(Property 4 突变器核心)。
    #[derive(Clone, Copy, Debug)]
    enum Mutation {
        /// 删除该意图元素的译后支持(→ 缺失:Out 改名退化 / In 被消除)。
        DropSupport,
        /// 退化改名为通用语义名(COLOR→TEXCOORD 类;与任何声明名/系统值不等价)。
        DegenerateRename,
        /// 抹掉/改写名与系统值(builtin 退化;通用名亦不命中任何 varying 语义)。
        WipeNameSysvalue,
    }

    const MUTATIONS: [Mutation; 3] = [
        Mutation::DropSupport,
        Mutation::DegenerateRename,
        Mutation::WipeNameSysvalue,
    ];

    /// 在保真合成基线上,对第 `target` 个意图元素施加单点突变 `m`,其余元素保持保真。
    /// `TEXCOORD`/`NONE` 通用名故意选取为**不**与任何基线声明语义名 / 系统值等价。
    fn synth_mutated(intent: &[IoSigElem], target: usize, m: Mutation) -> DxilSignatures {
        let mut input = Vec::new();
        let mut output = Vec::new();
        for (i, elem) in intent.iter().enumerate() {
            let se = if i == target {
                match m {
                    Mutation::DropSupport => continue,
                    Mutation::DegenerateRename => sig("TEXCOORD", "NONE", 9, "9", true),
                    Mutation::WipeNameSysvalue => sig("NONE", "NONE", 9, "9", true),
                }
            } else {
                synth_elem(elem)
            };
            match elem.dir {
                IoDir::In => input.push(se),
                IoDir::Out => output.push(se),
            }
        }
        DxilSignatures { input, output }
    }

    /// 对保真签名仅施加 **ABI 维度** 变化(打乱顺序 + 改 register/index + 语义 index
    /// 后缀),名 / 系统值语义不变。校验门据 Property 7 必须判定不变。
    fn abi_remix(sigs: &DxilSignatures) -> DxilSignatures {
        fn remix(v: &[SigElement]) -> Vec<SigElement> {
            let mut out: Vec<SigElement> = v
                .iter()
                .enumerate()
                .map(|(k, e)| SigElement {
                    // 追加语义 index 后缀(ABI 维度;校验门剥尾随数字后语义不变)。
                    name: format!("{}{}", e.name, k),
                    index: e.index.wrapping_add(7),
                    sysvalue: e.sysvalue.clone(),
                    register: format!("reg{}", k + 42),
                    used: e.used,
                })
                .collect();
            out.reverse(); // 打乱 part 顺序(ABI 维度)。
            out
        }
        DxilSignatures {
            input: remix(&sigs.input),
            output: remix(&sigs.output),
        }
    }

    /// accept(合成基线):每个保真基线经 [`synth_actual`] 合成的译后签名 → `Ok`。
    //@ spec: RXS-0159
    #[test]
    fn synth_faithful_baselines_all_accept() {
        for (tag, intent) in faithful_baselines() {
            assert!(
                check(&synth_actual(&intent), &intent).is_ok(),
                "{tag} 合成保真基线(保名一致 + SV 真达)应通过校验门"
            );
        }
    }

    /// **PBT — Property 4(校验门完备性)**:遍历「基准 × 元素 × 突变」笛卡尔积,
    /// 每个单点「未保真」突变组合 `check` 必 `Err`、**绝无 `Ok`**;且错误类按方向精确:
    /// Out 元素未保真 → `SigMismatch`,In 元素未保真 → `SigDroppedInput`。
    //@ spec: RXS-0159
    #[test]
    fn property4_any_unfaithful_mutation_is_rejected() {
        let baselines = faithful_baselines();
        let mut combos = 0usize;
        let mut errs = 0usize;
        for (tag, intent) in &baselines {
            // 突变器前置不变式:未突变的合成基线必 Ok(确保后续 Err 来自突变本身)。
            assert!(
                check(&synth_actual(intent), intent).is_ok(),
                "{tag} 合成保真基线应 Ok(突变器前置)"
            );
            for target in 0..intent.len() {
                for m in MUTATIONS {
                    combos += 1;
                    let mutated = synth_mutated(intent, target, m);
                    match check(&mutated, intent) {
                        Ok(()) => panic!(
                            "{tag} 元素#{target} 突变 {m:?} 竟通过校验门(Property 4 违反:零 Ok 要求)"
                        ),
                        Err(e) => {
                            errs += 1;
                            match (intent[target].dir, &e) {
                                (IoDir::Out, SigGateError::SigMismatch { .. }) => {}
                                (IoDir::In, SigGateError::SigDroppedInput { .. }) => {}
                                mism => panic!(
                                    "{tag} 元素#{target} 突变 {m:?} 错误类与方向不符: {mism:?}"
                                ),
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(combos, errs, "每个突变组合都必须 Err(Property 4:零 Ok)");
        // 3 基线(3+3+5=11 元素)× 3 突变 = 33 组合,规模可观。
        assert_eq!(combos, 33, "基准×元素×突变组合规模应为 33,实得 {combos}");
        eprintln!("[PBT P4] 基准×元素×突变 = {combos} 组合,全部 Err、零 Ok ✓");
    }

    /// **PBT — Property 7(ABI 中立对照,不得误报)**:对每个保真基线仅施加 ABI 维度
    /// 变化(顺序 / register / index / 语义 index 后缀)→ `check` 仍 `Ok`。证明突变器
    /// 测的是「未保真」而非「布局变化」(R2.7)。
    //@ spec: RXS-0162
    #[test]
    fn property7_abi_only_remix_still_accepts() {
        for (tag, intent) in faithful_baselines() {
            let base = synth_actual(&intent);
            assert!(check(&base, &intent).is_ok(), "{tag} 合成基线应 Ok");
            let remixed = abi_remix(&base);
            assert!(
                check(&remixed, &intent).is_ok(),
                "{tag} 仅 ABI 维度(顺序/register/index/语义 index 后缀)变化不得改变判定(ABI 中立)"
            );
        }
    }

    /// reject(显式补强,呼应任务6 突变枚举):抹掉/改写一个 builtin 的名与系统值 →
    /// `SigMismatch`(输出方向)。PBT 已覆盖,此处留显式红例便于诊断定位。
    //@ spec: RXS-0159
    #[test]
    fn reject_rewritten_builtin_sysvalue_is_sig_mismatch() {
        let mut sigs = vertex_faithful_sigs();
        // 改写 SV_Position 的名与系统值为不相关 token(builtin 退化)。
        sigs.output[0] = sig("FOO", "BAR", 0, "0", true);
        match check(&sigs, &vertex_intent()) {
            Err(SigGateError::SigMismatch { .. }) => {}
            other => panic!("改写 builtin 系统值应 SigMismatch,实得 {other:?}"),
        }
    }

    // ════════════════ RXS-0160:阶段间接口 → 阶段链接一致性核对 ════════════════
    //
    // `check_stage_link(vs_out_sig, fs_in_sig)` 以语义名等价为链接键核实 vertex 输出
    // varying ↔ fragment 输入 varying 的语义名 / 类型 / 插值一致性(builtin 系统值不参与,
    // location 不比对 ABI 中立)。错链 → StageLinkError(strict-only;错误码归类待 owner)。

    /// 链接一致的 vertex 输出集(position builtin out + color varying out +
    /// uv interpolate(perspective) out)。
    fn vs_link_out() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ]
    }

    /// 与 [`vs_link_out`] 链接一致的 fragment 输入集(frag_coord builtin in +
    /// color varying in + uv interpolate(perspective) in)。
    fn fs_link_in() -> Vec<IoSigElem> {
        vec![
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
        ]
    }

    /// accept(工具无关):链接一致(语义名 / 类型 / 插值全配对,builtin 不参与)→ `Ok`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_consistent_passes() {
        assert!(
            check_stage_link(&vs_link_out(), &fs_link_in()).is_ok(),
            "vertex out ↔ fragment in 链接一致应通过链接核对"
        );
    }

    /// accept:fragment 无输入 varying(仅 builtin frag_coord)→ 无链接键需求 → `Ok`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_no_fragment_varying_passes() {
        let fs_in = vec![io(
            "frag_coord",
            IoSigKind::Builtin("position".to_owned()),
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::In,
        )];
        assert!(
            check_stage_link(&vs_link_out(), &fs_in).is_ok(),
            "fragment 仅 builtin 输入(无 varying)应无错链"
        );
    }

    /// accept(ABI 中立):vertex 输出顺序打乱 + 链接键语义 index 后缀(vertex `color0`
    /// ↔ fragment `color`,`semantic_name_matches` 剥 vertex 侧尾随数字)不改变链接判定。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_abi_neutral_order_and_index_suffix() {
        // vertex 输出:color 带语义 index 后缀(剥尾随数字后语义不变)+ 顺序与基线相反。
        let mut vs = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "color0",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ];
        vs.reverse();
        // fragment 输入:color / uv(无后缀)。
        let fs = vec![
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
        ];
        assert!(
            check_stage_link(&vs, &fs).is_ok(),
            "顺序 / 语义 index 后缀(ABI 维度)不应改变链接判定"
        );
    }

    /// reject:fragment 输入 varying(`extra`)在 vertex 输出无同语义名 → `Unlinked`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_unlinked_fragment_input() {
        let fs = vec![io(
            "extra",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::In,
        )];
        match check_stage_link(&vs_link_out(), &fs) {
            Err(StageLinkError::Unlinked { .. }) => {}
            other => panic!("缺链接键应 Unlinked,实得 {other:?}"),
        }
    }

    /// reject:链接键语义名等价但类型失配(color 两端 vec4 ↔ vec2)→ `LinkMismatch`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_type_mismatch() {
        let fs = vec![io(
            "color",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 2), // 上游 vertex 输出为 vec4。
            IoDir::In,
        )];
        match check_stage_link(&vs_link_out(), &fs) {
            Err(StageLinkError::LinkMismatch { .. }) => {}
            other => panic!("链接键类型失配应 LinkMismatch,实得 {other:?}"),
        }
    }

    /// reject:链接键语义名 / 类型一致但插值限定失配(uv:perspective ↔ flat)→
    /// `LinkMismatch`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_interpolation_mismatch() {
        let fs = vec![io(
            "uv",
            IoSigKind::Interpolate("flat".to_owned()), // 上游为 perspective。
            MirIoType::Vector(PrimTy::F32, 2),
            IoDir::In,
        )];
        match check_stage_link(&vs_link_out(), &fs) {
            Err(StageLinkError::LinkMismatch { .. }) => {}
            other => panic!("插值限定失配应 LinkMismatch,实得 {other:?}"),
        }
    }

    /// reject:链接键类型一致但 Varying ↔ Interpolate 种类失配 → `LinkMismatch`。
    //@ spec: RXS-0160
    #[test]
    fn stage_link_varying_vs_interpolate_mismatch() {
        // vertex 输出 color 为普通 Varying;fragment 输入 color 声明为 Interpolate。
        let fs = vec![io(
            "color",
            IoSigKind::Interpolate("flat".to_owned()),
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::In,
        )];
        match check_stage_link(&vs_link_out(), &fs) {
            Err(StageLinkError::LinkMismatch { .. }) => {}
            other => panic!("Varying↔Interpolate 种类失配应 LinkMismatch,实得 {other:?}"),
        }
    }
}
