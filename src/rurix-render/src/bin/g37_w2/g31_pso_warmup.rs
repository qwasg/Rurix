/// G37 W2(夜航 night_0830)PSO precache/warmup 消费胶水——TODO #82/#113
/// 收口件:material/pso_cache.rs(冻结本体 0-byte)预测/预编译/告警 API 的
/// 窗口 demo 生产车道消费面。
///
/// ## 侦察事实(设计前提;见 REPORT.md 全文)
/// 窗口 bin 全部 pipeline 于 `DeviceFrameSession` 构造期一次性创建
/// (rurix-rt render_exec `create_persistent_frame` 逐 pass 循环建管线,
/// 会话内 `ComputePipelineKey{spv_hash,entry}` 去重);运行期 `FrameUpdate`
/// 仅 binding/push override,**无管线创建能力**。唯一运行期重建点 = era
/// 重建(resize/风暴/最小化恢复 → 'eras 循环重入,同变体集全量重建)。
/// ⇒ #113「启动走一遍 PSO」天然满足;本模块把该事实变成受门保护的断言:
/// **变体账本**(UE PSO precaching 口径)——era 0 = 预测集登记面
/// (`PsoCache::precache`,不告警);era ≥1 = `get_or_compile` 面,未命中
/// 即「运行期新 PSO 变体遭遇」告警 +1,验收归零(`pso_runtime_creates`)。
///
/// ## 映射约定(compute mega 车道 → 冻结 `PsoDesc` 变体键)
/// 生产车道为 compute 单 kernel 全材质(材质不引入 shader 排列——
/// pso_cache.rs 自述语义),变体身份 = SPV 字节内容(pass 诊断名**不进键**,
/// 同 SPV 多 pass 复用 = 同变体,与 rurix-rt 会话级
/// `ComputePipelineKey{spv_hash,entry}` 去重同判;pass 名只落报告行):
/// - `vs_entry` = `"spv:<fnv1a64 十六进制>"`(SPV 字节内容哈希;与会话级
///   `spv_hash` 同算法同语义)
/// - `fs_entry` = `"compute"` 常量字面(变体类别标注)
/// - `color_formats = []` / `depth_format = None` / `blend = Opaque` /
///   `cull = None`(compute 变体管线状态自由度为空,取闭集哨值)
/// 材质×pass 笛卡尔积预测器(`predict_precache_list`)在本形态无消费
/// (变体数与材质数解耦)——如实登记,不冒充接线。
mod g31_pso_warmup {
    use rurix_render::material::pso_cache::{BlendMode, CullMode, PsoCache, PsoDesc};

    const FNV1A64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;

    /// SPV 字节内容哈希(rurix-rt 会话级 pipeline 去重键同算法;本模块私有
    /// 实现——pso_cache 冻结面不导出 fnv,不反向添依赖)。
    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut h = FNV1A64_OFFSET;
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(FNV1A64_PRIME);
        }
        h
    }

    /// compute pass → 冻结 `PsoDesc` 变体键(映射约定见模块头;pass 名
    /// 不进键——同 SPV 多 pass 复用判同变体)。
    pub fn compute_variant_desc(spv: &[u8]) -> PsoDesc {
        PsoDesc {
            vs_entry: format!("spv:{:016x}", fnv1a64(spv)),
            fs_entry: "compute".to_owned(),
            color_formats: Vec::new(),
            depth_format: None,
            blend: BlendMode::Opaque,
            cull: CullMode::None,
        }
    }

    /// 预测集登记行(报告面;SHA-256 = provenance 对账锚)。
    pub struct G31PsoPlanned {
        pub pass_name: String,
        pub spv_fnv64: u64,
        pub spv_sha256: String,
        pub spv_bytes: usize,
    }

    /// 运行期新变体遭遇行(验收归零;非零即告警登记)。
    pub struct G31PsoRuntimeCreate {
        pub pass_name: String,
        pub spv_fnv64: u64,
        pub session_index: u32,
    }

    /// PSO 变体账本(era 0 = precache 面;era ≥1 = 运行期守护面)。
    ///
    /// 句柄型参 `u64` = 变体键稳定哈希回填(账本不持真 VkPipeline——真管线
    /// 由 `DeviceFrameSession` 构造期创建,账本承载 UE precache 口径的
    /// 「变体新颖性」守护;跨 era 的 Vulkan 层管线重建税与 VkPipelineCache
    /// 复用为 rurix-rt 面留窗项,如实登记不冒充)。
    pub struct G31PsoLedger {
        cache: PsoCache<u64>,
        sessions: u32,
        planned: Vec<G31PsoPlanned>,
        runtime_creates: Vec<G31PsoRuntimeCreate>,
    }

    impl G31PsoLedger {
        pub fn new() -> Self {
            Self {
                cache: PsoCache::new(),
                sessions: 0,
                planned: Vec::new(),
                runtime_creates: Vec::new(),
            }
        }

        /// 会话(era)开始登记;返回会话序号(0 = 启动 precache 面)。
        pub fn begin_session(&mut self) -> u32 {
            let idx = self.sessions;
            self.sessions += 1;
            idx
        }

        /// 登记一个 compute pass 变体。会话 0 = precache 路(冻结
        /// `PsoCache::precache`,幂等不告警,并记入预测集);会话 ≥1 =
        /// `get_or_compile` 路(命中零开销;未命中 = 运行期新变体遭遇,
        /// 告警 +1 并登记行)。返回「本次是否运行期新变体」。
        pub fn register(&mut self, pass_name: &str, spv: &[u8]) -> bool {
            let desc = compute_variant_desc(spv);
            if self.sessions <= 1 {
                if !self.cache.contains(&desc) {
                    self.planned.push(G31PsoPlanned {
                        pass_name: pass_name.to_owned(),
                        spv_fnv64: fnv1a64(spv),
                        spv_sha256: sha256_hex(spv),
                        spv_bytes: spv.len(),
                    });
                }
                self.cache.precache(std::iter::once(&desc), |d| d.stable_hash());
                false
            } else {
                let before = self.cache.warnings();
                self.cache.get_or_compile(&desc, |d| d.stable_hash());
                let miss = self.cache.warnings() > before;
                if miss {
                    self.runtime_creates.push(G31PsoRuntimeCreate {
                        pass_name: pass_name.to_owned(),
                        spv_fnv64: fnv1a64(spv),
                        session_index: self.sessions - 1,
                    });
                }
                miss
            }
        }

        /// 运行期新变体遭遇计数(= 冻结面 `PsoCache::warnings`;验收 == 0)。
        pub fn runtime_creates(&self) -> u64 {
            self.cache.warnings()
        }

        /// 去重后变体总数。
        pub fn unique_variants(&self) -> usize {
            self.cache.len()
        }

        /// 会话(era)总数。
        pub fn sessions(&self) -> u32 {
            self.sessions
        }

        /// sidecar 报告 JSON(单行;schema 字面
        /// `rurix.g31.pso_warmup_report.v1`——主 evidence schema
        /// additionalProperties:false 冻结,新字段一律 sidecar,
        /// day_0829 战役证据外置同律)。
        pub fn report_json(&self) -> String {
            let mut planned = String::from("[");
            for (k, p) in self.planned.iter().enumerate() {
                if k > 0 {
                    planned.push(',');
                }
                planned.push_str(&format!(
                    "{{\"pass\":\"{}\",\"spv_fnv64\":\"{:016x}\",\"spv_sha256\":\"{}\",\"spv_bytes\":{}}}",
                    p.pass_name, p.spv_fnv64, p.spv_sha256, p.spv_bytes
                ));
            }
            planned.push(']');
            let mut runtime = String::from("[");
            for (k, r) in self.runtime_creates.iter().enumerate() {
                if k > 0 {
                    runtime.push(',');
                }
                runtime.push_str(&format!(
                    "{{\"pass\":\"{}\",\"spv_fnv64\":\"{:016x}\",\"session_index\":{}}}",
                    r.pass_name, r.spv_fnv64, r.session_index
                ));
            }
            runtime.push(']');
            format!(
                "{{\"schema\":\"rurix.g31.pso_warmup_report.v1\",\"sessions\":{},\"unique_variants\":{},\"pso_precache_count\":{},\"pso_runtime_creates\":{},\"planned\":{},\"runtime_create_rows\":{}}}",
                self.sessions,
                self.cache.len(),
                self.planned.len(),
                self.cache.warnings(),
                planned,
                runtime
            )
        }
    }

    impl Default for G31PsoLedger {
        fn default() -> Self {
            Self::new()
        }
    }

    /// SHA-256 十六进制(rurix-pkg 同源;SPV provenance 对账锚)。
    fn sha256_hex(bytes: &[u8]) -> String {
        let d = rurix_pkg::sha256::digest(bytes);
        let mut s = String::with_capacity(64);
        for b in d {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}
