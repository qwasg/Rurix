//! RXS-0167:DXIL + RTS0 → graphics PSO 装配一致性(host 侧 safe 核验)。
//!
//! 把图形=B DXIL 着色器对象(VS/PS 接口 + 资源绑定反射)与 RFC-0005 推导的 RTS0
//! root signature + 渲染目标/深度格式装配为 graphics PSO 装配描述。以**编译期推导的
//! 单一事实源**(P-11)为准:RTS0 由资源使用推导([`rurixc::binding_layout`]),运行时
//! 不手维护第二份。装配不一致 → strict-only 显式错(无运行期 fallback)。
//!
//! 🔒 PSO / RTS0 的具体二进制物理布局、host↔运行时 / host↔DXIL FFI ABI 不冻结为
//! stable(实现确定、gate 后、非 stable;承 RFC-0004 §4.6(a) / RFC-0005 RXS-0165)。

use rurixc::binding_layout::{
    Psv0Reflection, RootSignature, check_binding_consistency, infer_register_assignments,
    infer_root_signature, serialize_rts0,
};
use rurixc::mir::ResourceBinding;

use crate::Format;
use crate::error::Uc04Error;

/// graphics PSO 装配描述(装配输入)。
///
/// P-11:`resources` 为着色器资源使用(编译期源于着色阶段签名 RXS-0156)的**单一
/// 事实源**;RTS0 / register 意图均由其推导,不在此手维护第二份。
pub struct GraphicsPsoDesc {
    /// 着色器资源使用(P-11 单一事实源)。
    pub resources: Vec<ResourceBinding>,
    /// 产物 DXIL 反射出的资源绑定(与推导意图一致性核验对象,RXS-0166)。
    pub reflected: Psv0Reflection,
    /// PS 输出签名渲染目标数(SV_Target 数)。
    pub ps_render_target_outputs: usize,
    /// 渲染目标格式集(MRT color 目标)。
    pub rtv_formats: Vec<Format>,
    /// 深度模板格式(无深度目标 → `None`)。
    pub dsv_format: Option<Format>,
    /// 是否写深度。
    pub depth_write: bool,
}

/// 装配出的 graphics PSO 描述(host 侧;不触 device)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledPso {
    /// RFC-0005 推导的 root signature(P-11 单一事实源)。
    pub root_signature: RootSignature,
    /// RTS0 容器字节(由 `root_signature` 机械序列化;非 stable ABI)。
    pub rts0_bytes: Vec<u8>,
    /// 渲染目标格式集。
    pub rtv_formats: Vec<Format>,
    /// 深度模板格式。
    pub dsv_format: Option<Format>,
}

/// RXS-0167:把 DXIL 接口 + RTS0 + 渲染目标/深度格式装配为 graphics PSO 描述。
///
/// # Errors
/// - 渲染目标数失配 / 深度状态矛盾 → [`Uc04Error::PsoTargetMismatch`](RX6018)。
/// - 着色器资源绑定反射 ↔ RTS0 推导意图失配 → [`Uc04Error::Rts0PsoMismatch`](RX6019;
///   复用 [`check_binding_consistency`])。
pub fn assemble_graphics_pso(desc: &GraphicsPsoDesc) -> Result<AssembledPso, Uc04Error> {
    // L2:PS 输出签名 ↔ 渲染目标格式集基数一致。
    if desc.ps_render_target_outputs != desc.rtv_formats.len() {
        return Err(Uc04Error::PsoTargetMismatch {
            detail: format!(
                "PS 输出渲染目标数 {} 与渲染目标格式集基数 {} 不一致",
                desc.ps_render_target_outputs,
                desc.rtv_formats.len()
            ),
        });
    }
    // L2:深度写入意图 ↔ 深度模板格式存在。
    if desc.depth_write && desc.dsv_format.is_none() {
        return Err(Uc04Error::PsoTargetMismatch {
            detail: "深度写入意图但缺深度模板格式".to_owned(),
        });
    }
    // L3:着色器资源绑定反射 ↔ RTS0 推导意图一致性(P-11:意图自资源使用推导)。
    let intent =
        infer_register_assignments(&desc.resources).map_err(|e| Uc04Error::Rts0PsoMismatch {
            detail: format!("绑定布局推导失败(RFC-0005): {e}"),
        })?;
    check_binding_consistency(&intent, &desc.reflected).map_err(|e| {
        Uc04Error::Rts0PsoMismatch {
            detail: format!("RTS0 推导意图 ↔ 产物反射不一致: {e}"),
        }
    })?;
    // P-11:RTS0 由同一资源使用推导(运行时不手维护第二份)。
    let root_signature =
        infer_root_signature(&desc.resources).map_err(|e| Uc04Error::Rts0PsoMismatch {
            detail: format!("root signature 推导失败(RFC-0005): {e}"),
        })?;
    let rts0_bytes = serialize_rts0(&root_signature);
    Ok(AssembledPso {
        root_signature,
        rts0_bytes,
        rtv_formats: desc.rtv_formats.clone(),
        dsv_format: desc.dsv_format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rurixc::binding_layout::{Psv0Resource, RegisterAssignment};
    use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

    /// 单 descriptor 资源绑定。
    fn rb(name: &str, res: MirResourceType) -> ResourceBinding {
        ResourceBinding {
            name: name.to_owned(),
            res,
            count: ResourceCount::One,
        }
    }

    /// lighting pass 资源基线:CBV(光照参数)+ 三个 G-buffer SRV + 一个 Sampler。
    fn lighting_resources() -> Vec<ResourceBinding> {
        vec![
            rb("light_params", MirResourceType::ConstantBuffer),
            rb(
                "g_albedo",
                MirResourceType::Texture2D(rurixc::hir::PrimTy::F32),
            ),
            rb(
                "g_normal",
                MirResourceType::Texture2D(rurixc::hir::PrimTy::F32),
            ),
            rb(
                "g_depth",
                MirResourceType::Texture2D(rurixc::hir::PrimTy::F32),
            ),
            rb("g_samp", MirResourceType::Sampler),
        ]
    }

    /// 由推导意图构造**等价** PSV0 反射(accept 路径:产物如实反射推导意图)。
    fn reflection_from(intent: &[RegisterAssignment]) -> Psv0Reflection {
        Psv0Reflection {
            resources: intent
                .iter()
                .map(|a| Psv0Resource {
                    class: a.class,
                    register: a.register,
                    space: a.space,
                    count: a.span,
                })
                .collect(),
        }
    }

    /// accept:一致输入(单 RT lighting 输出 + 深度 + 反射等价推导意图)→ AssembledPso。
    //@ spec: RXS-0167
    #[test]
    fn assemble_accepts_consistent_lighting_pso() {
        let resources = lighting_resources();
        let intent = infer_register_assignments(&resources).unwrap();
        let desc = GraphicsPsoDesc {
            reflected: reflection_from(&intent),
            resources,
            ps_render_target_outputs: 1,
            rtv_formats: vec![Format::Rgba8Unorm],
            dsv_format: Some(Format::D32Float),
            depth_write: false,
        };
        let pso = assemble_graphics_pso(&desc).expect("一致输入应装配成功");
        assert_eq!(pso.rtv_formats, vec![Format::Rgba8Unorm]);
        assert!(!pso.rts0_bytes.is_empty(), "RTS0 由资源使用推导序列化");
        // 确定性:相同输入两次装配字节全等(P-11 推导确定)。
        let pso2 = assemble_graphics_pso(&desc).unwrap();
        assert_eq!(pso.rts0_bytes, pso2.rts0_bytes);
    }

    /// accept:geometry pass MRT(albedo+normal+depth)三 RT 输出一致。
    //@ spec: RXS-0167
    #[test]
    fn assemble_accepts_geometry_mrt_pso() {
        let resources = vec![rb("obj_params", MirResourceType::ConstantBuffer)];
        let intent = infer_register_assignments(&resources).unwrap();
        let desc = GraphicsPsoDesc {
            reflected: reflection_from(&intent),
            resources,
            ps_render_target_outputs: 3,
            rtv_formats: vec![Format::Rgba8Unorm, Format::Rgba16Float, Format::Rgba8Unorm],
            dsv_format: Some(Format::D32Float),
            depth_write: true,
        };
        assert!(assemble_graphics_pso(&desc).is_ok());
    }

    /// reject:PS 输出数与渲染目标格式集基数不等 → PsoTargetMismatch(RX6018)。
    //@ spec: RXS-0167
    #[test]
    fn assemble_rejects_render_target_count_mismatch() {
        let resources = vec![rb("obj_params", MirResourceType::ConstantBuffer)];
        let intent = infer_register_assignments(&resources).unwrap();
        let desc = GraphicsPsoDesc {
            reflected: reflection_from(&intent),
            resources,
            ps_render_target_outputs: 3,           // 3 个 SV_Target
            rtv_formats: vec![Format::Rgba8Unorm], // 只给 1 个 RTV
            dsv_format: Some(Format::D32Float),
            depth_write: true,
        };
        match assemble_graphics_pso(&desc) {
            Err(e @ Uc04Error::PsoTargetMismatch { .. }) => assert_eq!(e.rx_code(), Some("RX6018")),
            other => panic!("渲染目标数失配应 PsoTargetMismatch,实得 {other:?}"),
        }
    }

    /// reject:深度写入意图但缺深度格式 → PsoTargetMismatch(RX6018)。
    //@ spec: RXS-0167
    #[test]
    fn assemble_rejects_depth_write_without_dsv() {
        let resources = vec![rb("obj_params", MirResourceType::ConstantBuffer)];
        let intent = infer_register_assignments(&resources).unwrap();
        let desc = GraphicsPsoDesc {
            reflected: reflection_from(&intent),
            resources,
            ps_render_target_outputs: 1,
            rtv_formats: vec![Format::Rgba8Unorm],
            dsv_format: None,
            depth_write: true,
        };
        assert!(matches!(
            assemble_graphics_pso(&desc),
            Err(Uc04Error::PsoTargetMismatch { .. })
        ));
    }

    /// reject:产物反射缺一个资源(与推导意图失配)→ Rts0PsoMismatch(RX6019)。
    //@ spec: RXS-0167
    #[test]
    fn assemble_rejects_rts0_reflection_mismatch() {
        let resources = lighting_resources();
        let intent = infer_register_assignments(&resources).unwrap();
        let mut reflected = reflection_from(&intent);
        reflected.resources.pop(); // 产物少反射一个资源 → 资源数失配
        let desc = GraphicsPsoDesc {
            reflected,
            resources,
            ps_render_target_outputs: 1,
            rtv_formats: vec![Format::Rgba8Unorm],
            dsv_format: Some(Format::D32Float),
            depth_write: false,
        };
        match assemble_graphics_pso(&desc) {
            Err(e @ Uc04Error::Rts0PsoMismatch { .. }) => assert_eq!(e.rx_code(), Some("RX6019")),
            other => panic!("RTS0 ↔ 反射失配应 Rts0PsoMismatch,实得 {other:?}"),
        }
    }
}
