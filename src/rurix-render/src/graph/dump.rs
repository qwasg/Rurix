//! 图 dump(报告5 §5 验证方法第四层 + §6 调试上下文缓解:P3 观测的最简形态自
//! P0 保留;RFC-0016 章 A)。
//!
//! 手写 JSON 序列化(零外部依赖),输出合法 JSON:pass(声明/车道/前屏障批)、
//! 资源(类别/生命周期/槽位/最后写入者)、屏障批(按 pass)、fence 对、池审计。

use std::fmt::Write as _;

use crate::graph::compile::CompiledGraph;
use crate::graph::types::{Barrier, ResAccess, ResourceKind};

impl CompiledGraph {
    /// 导出整图 JSON(字段名冻结,供 CI/观测工具消费)。
    #[must_use]
    pub fn dump_json(&self) -> String {
        let mut o = String::with_capacity(2048);
        o.push_str("{\"graph\":{");
        let _ = write!(
            o,
            "\"pass_count\":{},\"resource_count\":{},\"culled_passes\":{},\"culled_resources\":{}",
            self.passes().len(),
            self.resources().len(),
            self.culled_pass_count(),
            self.culled_resource_count()
        );
        o.push_str("},\"passes\":[");
        for (i, p) in self.passes().iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(o, "{{\"id\":{},\"name\":", p.id().0);
            esc_into(&mut o, p.name());
            let _ = write!(o, ",\"queue\":\"{:?}\",\"reads\":[", p.queue());
            access_list(&mut o, p.reads());
            o.push_str("],\"writes\":[");
            access_list(&mut o, p.writes());
            o.push_str("],\"barriers_before\":[");
            barrier_list(&mut o, p.barriers_before());
            o.push_str("]}");
        }
        o.push_str("],\"resources\":[");
        for (i, r) in self.resources().iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(o, "{{\"id\":{},\"name\":", r.id().0);
            esc_into(&mut o, r.name());
            let kind = match r.kind() {
                ResourceKind::Buffer { .. } => "Buffer",
                ResourceKind::Texture2d { .. } => "Texture2d",
            };
            let _ = write!(
                o,
                ",\"kind\":\"{kind}\",\"imported\":{},\"byte_size\":{}",
                r.imported(),
                r.byte_size()
            );
            match r.lifetime() {
                Some(iv) => {
                    let _ = write!(
                        o,
                        ",\"lifetime\":{{\"first\":{},\"last\":{}}}",
                        iv.first_use.0, iv.last_use.0
                    );
                }
                None => o.push_str(",\"lifetime\":null"),
            }
            match r.slot() {
                Some(s) => {
                    let _ = write!(
                        o,
                        ",\"slot\":{{\"bucket\":{},\"slot\":{},\"size\":{}}}",
                        s.bucket, s.slot, s.size
                    );
                }
                None => o.push_str(",\"slot\":null"),
            }
            match r.last_writer() {
                Some(w) => {
                    let _ = write!(o, ",\"last_writer\":{}", w.0);
                }
                None => o.push_str(",\"last_writer\":null"),
            }
            o.push('}');
        }
        o.push_str("],\"barriers\":[");
        for (i, (pid, batch)) in self.barriers().iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(o, "{{\"pass\":{},\"batch\":[", pid.0);
            barrier_list(&mut o, batch);
            o.push_str("]}");
        }
        o.push_str("],\"fences\":[");
        for (i, f) in self.fences().iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(
                o,
                "{{\"signal_after\":{},\"wait_before\":{},\"value\":{}}}",
                f.signal_after.0, f.wait_before.0, f.value
            );
        }
        let _ = write!(
            o,
            "],\"pool\":{{\"high_water\":{},\"no_alias_peak\":{},\"slot_count\":{}}}}}",
            self.pool().high_water(),
            self.pool().no_alias_peak(),
            self.pool().slot_count()
        );
        o
    }
}

fn access_list(o: &mut String, list: &[ResAccess]) {
    for (i, a) in list.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(o, "{{\"res\":{},\"access\":\"{:?}\"}}", a.res.0, a.access);
    }
}

fn barrier_list(o: &mut String, list: &[Barrier]) {
    for (i, b) in list.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "{{\"res\":{},\"sync_before\":\"{:?}\",\"sync_after\":\"{:?}\",\"access_before\":\"{:?}\",\"access_after\":\"{:?}\",\"layout_before\":\"{:?}\",\"layout_after\":\"{:?}\"}}",
            b.res.0,
            b.sync_before,
            b.sync_after,
            b.access_before,
            b.access_after,
            b.layout_before,
            b.layout_after
        );
    }
}

/// 字符串 JSON 转义(引号/反斜杠/控制字符;其余 Unicode 直排)。
fn esc_into(o: &mut String, s: &str) {
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
}

#[cfg(test)]
mod tests {
    use crate::graph::compile::CompileOptions;
    use crate::graph::graph::RenderGraph;
    use crate::graph::types::{
        AccessKind, PassDesc, QueueClass, ResAccess, ResourceDesc, ResourceKind, TextureFormat,
    };

    /// dump 为合法 JSON(括号平衡/字符串闭合)且关键字段齐全。
    #[test]
    fn dump_json_is_valid_and_complete() {
        let mut g = RenderGraph::new();
        let mk = |name: &str| ResourceDesc {
            name: name.to_owned(),
            kind: ResourceKind::Texture2d {
                width: 512,
                height: 512,
                format: TextureFormat::Rgba8Unorm,
                mip_levels: 1,
            },
            imported: false,
        };
        let a = g.create(mk("gbuf:Albedo"));
        let ao = g.create(mk("ao:Raw"));
        let bb = g.import(mk("backbuffer"));
        // 名字含引号/反斜杠:转义面压力测试。
        let weird = g.create(mk("we\"ird\\name"));
        let ra = |res, access| ResAccess { res, access };
        let pd = |name: &str, queue, reads, writes| PassDesc {
            name: name.to_owned(),
            queue,
            reads,
            writes,
        };
        g.add_pass(pd(
            "gbuffer",
            QueueClass::Graphics,
            vec![],
            vec![
                ra(a, AccessKind::ColorTarget),
                ra(weird, AccessKind::ColorTarget),
            ],
        ));
        g.add_pass(pd(
            "ao",
            QueueClass::AsyncCompute,
            vec![ra(a, AccessKind::ShaderRead)],
            vec![ra(ao, AccessKind::ShaderWrite)],
        ));
        g.add_pass(pd(
            "lighting",
            QueueClass::Graphics,
            vec![
                ra(weird, AccessKind::ShaderRead),
                ra(ao, AccessKind::ShaderRead),
            ],
            vec![ra(bb, AccessKind::ColorTarget)],
        ));
        let c = g.compile(CompileOptions::default()).expect("合法图");
        let json = c.dump_json();
        assert_balanced(&json);
        for key in [
            "\"passes\"",
            "\"resources\"",
            "\"barriers\"",
            "\"fences\"",
            "\"pool\"",
            "\"high_water\"",
            "\"no_alias_peak\"",
            "\"signal_after\"",
            "\"wait_before\"",
            "\"lifetime\"",
            "\"slot\"",
            "\"last_writer\"",
            "\"barriers_before\"",
            "\"queue\"",
            "\"culled_passes\"",
        ] {
            assert!(json.contains(key), "dump 缺字段 {key}");
        }
        assert!(json.contains("\"gbuffer\""));
        assert!(json.contains("AsyncCompute"));
        assert!(json.contains("ShaderReadOnly"));
        assert!(json.contains("ColorAttachment"));
        // 转义:引号/反斜杠名字原样转义后仍平衡(上方 assert_balanced 已覆盖)。
        assert!(json.contains("we\\\"ird\\\\name"));
        // fence 对序列化:gbuffer(0) 后 signal、lighting(2) 前 wait。
        assert!(json.contains("\"signal_after\":0"));
        assert!(json.contains("\"wait_before\":2"));
    }

    /// 括号平衡 + 字符串闭合检查(忽略转义字符串内容)。
    fn assert_balanced(json: &str) {
        let (mut curly, mut square) = (0i32, 0i32);
        let mut in_str = false;
        let mut esc = false;
        for ch in json.chars() {
            if in_str {
                if esc {
                    esc = false;
                } else if ch == '\\' {
                    esc = true;
                } else if ch == '"' {
                    in_str = false;
                }
                continue;
            }
            match ch {
                '"' => in_str = true,
                '{' => curly += 1,
                '}' => curly -= 1,
                '[' => square += 1,
                ']' => square -= 1,
                _ => {}
            }
            assert!(curly >= 0 && square >= 0, "括号负平衡: {json}");
        }
        assert_eq!((curly, square, in_str), (0, 0, false), "JSON 未闭合");
    }
}
