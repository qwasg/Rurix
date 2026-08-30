/// G37 W2 ris_nee(夜航 night_0830)灯片表 + 功率 CDF 装配模块——GI2 反弹
/// RIS 选灯 / 44k 灯片 CDF 面光 NEE 两 kernel 臂(g31_realism.rx 第 8 链位,
/// SPV = g31_realism_ris.spv)的 host 侧单源。窗口 bin 合入提案消费同一文件
/// (`include!("g37_w2/g31_ris_lamps.rs")`);g37_w2_selfcheck 纯 CPU 机核。
///
/// ## 布局契约(kernel `lamp_tbl: View<global, f32>` 逐字同源)
/// - 头 4 f32:[0] = 灯片数 Q(f32 精确整数,Q ≤ 65536)/[1] = 总功率
///   (诊断量)/[2..4) 预留恒 0。
/// - CDF 段 [4..4+Q):功率前缀和归一(f64 累加 → f32 下投,单调不减,末项
///   强制 1.0);kernel 侧 16 步定长二分,选中 pdf_k = cdf[k]−cdf[k−1] 的
///   f32 差 = 采样测度(与二分计数语义自洽 ⇒ 无偏)。
/// - 记录段 [4+Q..4+Q+16Q) 逐灯片 16 f32:
///   [A(3) e1(3) e2(3) 单位法线(3) 面积(1) Le(3)](A = 顶点 0,e1/e2 =
///   邻边;面上点 = A + u·e1 + v·e2,u+v ≤ 1 折叠律)。
///
/// ## 谓词与口径(A1 `extract_lamp_lights` 逐字同律,双侧一致)
/// - 表成员 = emission 任一通道 > 0 且 tri_mat ≠ tri_none(排除 quad 灯尾
///   段;kernel 侧 emission 直取置零门用 mats 发射均值 >0 同谓词)。
/// - 功率 = max3(π·Le_c·area)(A1 通量峰值口径;Lambert 单面发射体);
///   零面积成员保留在表(索引/谓词两侧对齐)但功率 0 ⇒ pdf 0 永不被选,
///   射线测度零不可命中,无能量缺口。
/// - 确定性:三角升序单趟扫描 + f64 前缀和,无哈希容器——同输入同字节
///   (双构建 == 由 selfcheck 机核)。
mod g31_ris_lamps {
    /// 表头长度(f32)。
    pub const G31_RIS_LAMP_HDR: usize = 4;
    /// 逐灯片记录步幅(f32)。
    pub const G31_RIS_LAMP_STRIDE: usize = 16;
    /// 灯片数上限(kernel 16 步定长二分覆盖域 2^16;越界 fail-closed)。
    pub const G31_RIS_LAMP_MAX: usize = 65536;
    /// 关臂哑表字节数(--gi2-ris on 而 --gi2-nee off 绑定面:header Q=0 +
    /// 首记录触达域 [4..20) 保底读 ⇒ 20 f32 = 80 B 全零)。
    pub const G31_RIS_LAMP_DUMMY_BYTES: usize = 80;

    /// 构建统计(登记面;REPORT/装配日志消费)。
    pub struct G31RisLampStats {
        /// 表成员数 Q(= emissive 三角数,谓词见模块头)。
        pub emissive_tris: usize,
        /// 零面积成员数(保留在表,功率 0 永不被选——如实登记)。
        pub zero_area_tris: usize,
        /// f32 CDF 差下溢成员数(f64 功率 > 0 但 f32 前缀差 == 0 ⇒ 永不被
        /// 选;能量损失上界 = 其功率占比,如实登记)。
        pub pdf_underflow_tris: usize,
        /// 总功率(f64 精确累加;头 [1] 为其 f32 下投)。
        pub total_power: f64,
        /// 表体 f32 长度(= HDR + Q + 16Q)。
        pub table_f32_len: usize,
    }

    /// 灯片表构建(纯函数,双构建确定性;任一破约 Err fail-closed)。
    /// 入参为逐三角平行数组(SceneData 字段直传;`tri_none` =
    /// SLAB_TRI_NONE 字面,quad 灯尾段排除谓词)。
    pub fn build_lamp_table(
        positions: &[[f32; 3]],
        indices: &[[u32; 3]],
        emission: &[[f32; 3]],
        tri_mat: &[u32],
        tri_none: u32,
    ) -> Result<(Vec<f32>, G31RisLampStats), String> {
        if indices.len() != emission.len() || indices.len() != tri_mat.len() {
            return Err(format!(
                "ris_lamps: 平行数组长度失配 indices={} emission={} tri_mat={}(fail-closed)",
                indices.len(),
                emission.len(),
                tri_mat.len()
            ));
        }
        // ① 升序单趟扫描(确定性根;A1 同谓词)。
        struct Rec {
            a: [f32; 3],
            e1: [f32; 3],
            e2: [f32; 3],
            n: [f32; 3],
            area: f32,
            le: [f32; 3],
            power: f64,
        }
        let mut recs: Vec<Rec> = Vec::new();
        let mut zero_area = 0usize;
        for (k, le) in emission.iter().enumerate() {
            if !(le[0] > 0.0 || le[1] > 0.0 || le[2] > 0.0) || tri_mat[k] == tri_none {
                continue;
            }
            let idx = indices[k];
            let mut v = [[0.0f32; 3]; 3];
            for (slot, &vi) in idx.iter().enumerate() {
                v[slot] = *positions.get(vi as usize).ok_or_else(|| {
                    format!("ris_lamps: tri{k} 顶点索引 {vi} 越 positions 表(fail-closed)")
                })?;
            }
            if !(le[0].is_finite() && le[1].is_finite() && le[2].is_finite()) {
                return Err(format!("ris_lamps: tri{k} emission 非有限(fail-closed)"));
            }
            let e1 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
            let e2 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
            let cx = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let cl = (cx[0] * cx[0] + cx[1] * cx[1] + cx[2] * cx[2]).sqrt();
            let area = 0.5 * cl;
            if !area.is_finite() {
                return Err(format!("ris_lamps: tri{k} 面积非有限(fail-closed)"));
            }
            // 单位法线(零面积成员取 0 向量占位——功率 0 永不被选,kernel
            // 零消费;非零面积 = cross/|cross|)。
            let n = if cl > 0.0 {
                [cx[0] / cl, cx[1] / cl, cx[2] / cl]
            } else {
                zero_area += 1;
                [0.0, 0.0, 0.0]
            };
            // 功率 = max3(π·Le_c·area)(A1 通量峰值口径,f64 精确累加域)。
            let p = f64::from(std::f32::consts::PI)
                * f64::from(area)
                * f64::from(le[0].max(le[1]).max(le[2]));
            recs.push(Rec {
                a: v[0],
                e1,
                e2,
                n,
                area,
                le: *le,
                power: p,
            });
        }
        let q = recs.len();
        if q == 0 {
            return Err("ris_lamps: 表成员零命中(emission>0 且非灯面尾段无一满足——臂无消费面,fail-closed)".into());
        }
        if q > G31_RIS_LAMP_MAX {
            return Err(format!(
                "ris_lamps: 灯片数 {q} > {G31_RIS_LAMP_MAX}(kernel 16 步二分覆盖域,fail-closed)"
            ));
        }
        // ② 功率 CDF(f64 前缀和 → 归一 → f32 下投;末项强制 1.0)。
        let total: f64 = recs.iter().map(|r| r.power).sum();
        if !(total.is_finite() && total > 0.0) {
            return Err(format!("ris_lamps: 总功率 {total} 非正/非有限(fail-closed)"));
        }
        let mut out: Vec<f32> = Vec::with_capacity(G31_RIS_LAMP_HDR + q + q * G31_RIS_LAMP_STRIDE);
        out.push(q as f32);
        out.push(total as f32);
        out.push(0.0);
        out.push(0.0);
        let mut prefix = 0.0f64;
        let mut prev_c = 0.0f32;
        let mut pdf_underflow = 0usize;
        for (i, r) in recs.iter().enumerate() {
            prefix += r.power;
            let c = if i + 1 == q {
                1.0f32
            } else {
                (prefix / total) as f32
            };
            if r.power > 0.0 && c - prev_c <= 0.0 {
                pdf_underflow += 1;
            }
            prev_c = c;
            out.push(c);
        }
        // ③ 记录段(16 f32/灯片,kernel 布局逐字)。
        for r in &recs {
            out.extend_from_slice(&r.a);
            out.extend_from_slice(&r.e1);
            out.extend_from_slice(&r.e2);
            out.extend_from_slice(&r.n);
            out.push(r.area);
            out.extend_from_slice(&r.le);
        }
        let stats = G31RisLampStats {
            emissive_tris: q,
            zero_area_tris: zero_area,
            pdf_underflow_tris: pdf_underflow,
            total_power: total,
            table_f32_len: out.len(),
        };
        debug_assert_eq!(stats.table_f32_len, G31_RIS_LAMP_HDR + q + q * G31_RIS_LAMP_STRIDE);
        Ok((out, stats))
    }
}
