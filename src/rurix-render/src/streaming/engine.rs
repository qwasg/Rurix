//! 流送引擎每帧协议(RFC-0016 §4.G4;报告6 §2.4 预算化——UE 5.6 Fast
//! Geometry Streaming 思想:「每帧 I/O 字节数、转码页数、staging 上传字节
//! 数」三预算计数器必须从 P1 就存在,流送系统最常见的失败模式不是算法错,
//! 而是单帧工作量无界)。

use std::collections::HashMap;

use crate::graph::types::{PageRequest, STREAM_PAGE_SIZE, StreamingBudget};

use super::pool::{InsertOutcome, PagePool};
use super::resource::PagedResource;

/// 每帧度量报告(埋点口径:数字进 evidence 不进硬门,RFC-0016 §4.0-4)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickReport {
    /// 本帧新入池页数(root 注册期加载不计;已驻留触帧不计)。
    pub pages_loaded: u32,
    /// 本帧被 LRU 驱逐页数。
    pub pages_evicted: u32,
    /// 本帧 IO 段字节(`read_page` 原始字节合计,仅计实际入池的页)。
    pub bytes_io: u64,
    /// 本帧转码段字节(转码输入字节合计,仅计实际入池的页)。
    pub bytes_transcode: u64,
    /// 本帧上传段字节(入池 payload 合计)。
    pub bytes_upload: u64,
    /// 本帧「预算耗尽即停」触发次数(即停语义下 0 或 1;池全钉住无法腾槽
    /// 同计——同为「本帧无法继续」的停顿信号)。
    pub over_budget_stalls: u32,
    /// 帧末待处理请求队列深度(滚入下帧)。
    pub queue_depth: u32,
}

/// 队列项(去重合并后;`seq` 为提交序全序键——同优先级 FIFO 跨帧保持的
/// 确定性来源)。
#[derive(Debug, Clone, Copy)]
struct Pending {
    request: PageRequest,
    seq: u64,
}

/// 通用页式流送引擎(资源类型无关;几何页/纹理页同栈,报告6 §3 核心决策)。
///
/// 每帧协议:
/// 1. [`submit_requests`](StreamingEngine::submit_requests)——同页去重(合并
///    取最高优先级,FIFO 位次取最早);已驻留页零成本触帧,不进队列;
/// 2. [`tick`](StreamingEngine::tick)——优先级高→低、同级 FIFO 处理队列,
///    三预算**分段扣账**(io = `read_page` 字节 → transcode = 转码输入字节
///    → upload = 入池字节),任一段装不下当前页**即停**,当前及后续请求原序
///    滚入下帧;预算每帧重置由调用方每帧传入新 [`StreamingBudget`] 体现;
/// 3. root 页注册即强制加载钉住(同步、不占帧预算——宁可注册期阻塞,不可
///    无根可渲,报告6 §2.4)。
///
/// staging 图外语义:本引擎只做 host 侧的读取/转码/入池调度,全部工作在
/// render graph 图外;payload 经 staging 上传后,消费点以 acquire 屏障接入
/// 图内(RFC-0016 §4.0-3),本模块不实做屏障。
pub struct StreamingEngine {
    pool: PagePool,
    resources: Vec<Box<dyn PagedResource>>,
    resource_index: HashMap<u32, usize>,
    pending: Vec<Pending>,
    next_seq: u64,
    /// (resource, page) → 首次请求帧(pop-in 判定用;页驻留即清除)。
    first_requested: HashMap<(u32, u32), u32>,
    pop_in_threshold: u32,
    pop_in_count: u64,
}

impl StreamingEngine {
    /// 引擎 = 固定槽池 + 空注册表。pop-in 阈值默认 1 帧:请求帧未驻留、
    /// `load_frame - request_frame >= threshold` 才驻留即计(报告6 §6
    /// 「pop-in 计数:选中页未驻留的帧数」)。
    pub fn new(pool_capacity: usize) -> Self {
        Self {
            pool: PagePool::new(pool_capacity),
            resources: Vec::new(),
            resource_index: HashMap::new(),
            pending: Vec::new(),
            next_seq: 0,
            first_requested: HashMap::new(),
            pop_in_threshold: 1,
            pop_in_count: 0,
        }
    }

    pub fn pool(&self) -> &PagePool {
        &self.pool
    }

    pub fn pop_in_count(&self) -> u64 {
        self.pop_in_count
    }

    pub fn pop_in_threshold(&self) -> u32 {
        self.pop_in_threshold
    }

    pub fn set_pop_in_threshold(&mut self, frames: u32) {
        self.pop_in_threshold = frames;
    }

    /// 当前待处理队列深度(tick 前的滚存 + 本帧新提交)。
    pub fn queue_len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_resident(&self, resource: u32, page: u32) -> bool {
        self.pool.lookup(resource, page).is_some()
    }

    /// 注册资源:root 页强制加载并钉住(同步、不占帧预算)。
    ///
    /// 配置错误即 panic(确定性失败,不静默降级):`resource_id` 重复、root
    /// 页号越界、root/转码产物超 128KB、池容量不足以钉住全部 root 页。
    pub fn register_resource(&mut self, resource: Box<dyn PagedResource>) {
        let id = resource.resource_id();
        assert!(
            !self.resource_index.contains_key(&id),
            "resource_id {id} 重复注册"
        );
        for &root in resource.root_pages() {
            assert!(
                root < resource.page_count(),
                "resource {id} root 页号 {root} 越界(page_count {})",
                resource.page_count()
            );
            let raw = resource.read_page(root);
            assert!(
                raw.len() <= STREAM_PAGE_SIZE as usize,
                "resource {id} root 页 {root} 超 128KB"
            );
            let payload = resource.transcode(root, &raw);
            assert!(
                payload.len() <= STREAM_PAGE_SIZE as usize,
                "resource {id} root 页 {root} 转码产物超 128KB"
            );
            match self.pool.insert(id, root, payload, true) {
                InsertOutcome::Inserted { .. } => {}
                InsertOutcome::PoolFull => {
                    panic!("resource {id} root 页超池容量:池须 ≥ 全资源 root 页合计")
                }
            }
        }
        let idx = self.resources.len();
        self.resources.push(resource);
        self.resource_index.insert(id, idx);
    }

    /// 提交反馈请求:同页合并取最高优先级(FIFO 位次与首请求帧取最早);
    /// 已驻留页直接触帧,不进队列(零成本)。
    pub fn submit_requests(&mut self, requests: &[PageRequest]) {
        for &req in requests {
            if self.pool.touch(req.resource, req.page_index).is_some() {
                // 已驻留:触帧刷新 LRU;注册期 root 加载等路径可能残留跟踪,
                // 驻留即清(不计 pop-in——root 加载非请求驱动)。
                self.first_requested.remove(&(req.resource, req.page_index));
                continue;
            }
            if let Some(p) = self.pending.iter_mut().find(|p| {
                p.request.resource == req.resource && p.request.page_index == req.page_index
            }) {
                // 去重合并:优先级取高,位次(seq)不动。
                p.request.priority = p.request.priority.max(req.priority);
            } else {
                let seq = self.next_seq;
                self.next_seq += 1;
                self.pending.push(Pending { request: req, seq });
            }
            let e = self
                .first_requested
                .entry((req.resource, req.page_index))
                .or_insert(req.frame);
            *e = (*e).min(req.frame);
        }
    }

    /// 每帧处理:优先级高→低、同级 FIFO(seq 全序)。三预算分段扣账,任一
    /// 段装不下当前页即停,当前及后续请求原序滚入下帧。
    ///
    /// 未注册资源 / 越界页号的请求(过期或坏反馈)确定性丢弃,不阻塞队列。
    pub fn tick(&mut self, frame: u32, budget: &StreamingBudget) -> TickReport {
        self.pending.sort_by(|a, b| {
            b.request
                .priority
                .cmp(&a.request.priority)
                .then(a.seq.cmp(&b.seq))
        });
        let pending = std::mem::take(&mut self.pending);
        let mut leftover: Vec<Pending> = Vec::new();
        let mut report = TickReport::default();
        let mut stalled = false;
        for p in pending {
            if stalled {
                leftover.push(p);
                continue;
            }
            let req = p.request;
            if self.pool.touch(req.resource, req.page_index).is_some() {
                // 本帧前序请求已加载:触帧 + pop-in 结算,出队。
                self.resolve_pop_in(req.resource, req.page_index, frame);
                continue;
            }
            let Some(&res_idx) = self.resource_index.get(&req.resource) else {
                self.first_requested.remove(&(req.resource, req.page_index));
                continue;
            };
            let resource = &*self.resources[res_idx];
            if req.page_index >= resource.page_count() {
                self.first_requested.remove(&(req.resource, req.page_index));
                continue;
            }
            // 分段扣账:先取确定性尺寸,再逐段核对预算;任一段装不下即停
            // (未提交的读取/转码不计字节,下帧重试时重新计量——确定性)。
            let raw = resource.read_page(req.page_index);
            assert!(
                raw.len() <= STREAM_PAGE_SIZE as usize,
                "resource {} page {} 超 128KB",
                req.resource,
                req.page_index
            );
            let payload = resource.transcode(req.page_index, &raw);
            assert!(
                payload.len() <= STREAM_PAGE_SIZE as usize,
                "resource {} page {} 转码产物超 128KB",
                req.resource,
                req.page_index
            );
            let io_need = raw.len() as u64;
            let tc_need = raw.len() as u64;
            let up_need = payload.len() as u64;
            if report.bytes_io.saturating_add(io_need) > budget.io_bytes
                || report.bytes_transcode.saturating_add(tc_need) > budget.transcode_bytes
                || report.bytes_upload.saturating_add(up_need) > budget.upload_bytes
            {
                stalled = true;
                report.over_budget_stalls += 1;
                leftover.push(p);
                continue;
            }
            match self
                .pool
                .insert(req.resource, req.page_index, payload, false)
            {
                InsertOutcome::Inserted { evicted, .. } => {
                    if evicted.is_some() {
                        report.pages_evicted += 1;
                    }
                    report.bytes_io += io_need;
                    report.bytes_transcode += tc_need;
                    report.bytes_upload += up_need;
                    report.pages_loaded += 1;
                    self.resolve_pop_in(req.resource, req.page_index, frame);
                }
                InsertOutcome::PoolFull => {
                    stalled = true;
                    report.over_budget_stalls += 1;
                    leftover.push(p);
                }
            }
        }
        report.queue_depth = leftover.len() as u32;
        self.pending = leftover;
        report
    }

    /// pop-in 结算:请求帧未驻留、且 `load_frame - request_frame >= 阈值`
    /// 才驻留 → 计数 +1;驻留即清跟踪(驱逐后再请求重新起计)。
    fn resolve_pop_in(&mut self, resource: u32, page: u32, load_frame: u32) {
        if let Some(req_frame) = self.first_requested.remove(&(resource, page))
            && load_frame.saturating_sub(req_frame) >= self.pop_in_threshold
        {
            self.pop_in_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::feedback::{
        FEEDBACK_BASE_GEOMETRY_LOD, FEEDBACK_BASE_TEXTURE_MISS, FeedbackBuilder,
    };

    /// 合成页资源:页内容 = 确定性模式(resource ^ page ^ 字节序号),可选
    /// XOR 转码(自逆变换,验证确定性往返)。
    struct Synthetic {
        id: u32,
        roots: Vec<u32>,
        pages: Vec<Vec<u8>>,
        xor_key: Option<u8>,
    }

    impl Synthetic {
        fn new(id: u32, page_count: u32, page_len: usize, roots: &[u32]) -> Self {
            let pages = (0..page_count)
                .map(|p| {
                    (0..page_len)
                        .map(|i| (id as u8) ^ (p as u8) ^ (i as u8))
                        .collect()
                })
                .collect();
            Self {
                id,
                roots: roots.to_vec(),
                pages,
                xor_key: None,
            }
        }

        fn with_xor(mut self, key: u8) -> Self {
            self.xor_key = Some(key);
            self
        }

        fn raw_page(&self, page: u32) -> Vec<u8> {
            self.pages[page as usize].clone()
        }
    }

    impl PagedResource for Synthetic {
        fn resource_id(&self) -> u32 {
            self.id
        }
        fn page_count(&self) -> u32 {
            self.pages.len() as u32
        }
        fn root_pages(&self) -> &[u32] {
            &self.roots
        }
        fn read_page(&self, page: u32) -> Vec<u8> {
            self.pages[page as usize].clone()
        }
        fn transcode(&self, _page: u32, raw: &[u8]) -> Vec<u8> {
            match self.xor_key {
                None => raw.to_vec(),
                Some(k) => raw.iter().map(|b| b ^ k).collect(),
            }
        }
    }

    fn budget(io: u64, tc: u64, up: u64) -> StreamingBudget {
        StreamingBudget {
            io_bytes: io,
            transcode_bytes: tc,
            upload_bytes: up,
        }
    }

    fn req(resource: u32, page: u32, priority: u32, frame: u32) -> PageRequest {
        PageRequest {
            resource,
            page_index: page,
            priority,
            frame,
        }
    }

    fn page_data(engine: &StreamingEngine, resource: u32, page: u32) -> Vec<u8> {
        let slot = engine.pool().lookup(resource, page).expect("页驻留");
        engine.pool().slot_data(slot).to_vec()
    }

    /// root 页注册即强制加载钉住(不占帧预算);空预算空队列的 tick = 全零
    /// 报告。
    #[test]
    fn root_pages_pinned_at_registration() {
        let mut engine = StreamingEngine::new(4);
        engine.register_resource(Box::new(Synthetic::new(1, 16, 64, &[0, 1])));
        assert!(engine.is_resident(1, 0));
        assert!(engine.is_resident(1, 1));
        for root in [0, 1] {
            let slot = engine.pool().lookup(1, root).expect("root 常驻");
            assert!(engine.pool().is_pinned(slot));
        }
        assert_eq!(engine.pool().resident_count(), 2);
        let expect = Synthetic::new(1, 16, 64, &[0, 1]);
        assert_eq!(page_data(&engine, 1, 0), expect.raw_page(0));
        assert_eq!(engine.tick(0, &budget(0, 0, 0)), TickReport::default());
    }

    /// resource_id 重复注册 = 配置错误,panic(确定性失败)。
    #[test]
    #[should_panic(expected = "重复注册")]
    fn duplicate_resource_id_panics() {
        let mut engine = StreamingEngine::new(4);
        engine.register_resource(Box::new(Synthetic::new(1, 4, 64, &[0])));
        engine.register_resource(Box::new(Synthetic::new(1, 4, 64, &[0])));
    }

    /// 优先级高→低;同级 FIFO(提交序,跨帧保持)。
    #[test]
    fn priority_then_fifo_order() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 16, 100, &[0])));
        let b = budget(150, 1_000_000, 1_000_000); // 每帧恰 1 页
        engine.submit_requests(&[
            req(1, 3, 5, 0),
            req(1, 1, 9, 0),
            req(1, 2, 5, 0),
            req(1, 4, 1, 0),
        ]);
        // 期望加载序:page1(9) → page3(5,先提交) → page2(5) → page4(1)。
        let mut loaded_order = Vec::new();
        for (f, page) in [1u32, 3, 2, 4].iter().enumerate() {
            let r = engine.tick(f as u32, &b);
            assert_eq!(r.pages_loaded, 1);
            assert!(engine.is_resident(1, *page));
            loaded_order.push(*page);
        }
        assert_eq!(loaded_order, vec![1, 3, 2, 4]);
        assert_eq!(engine.tick(4, &b).queue_depth, 0);
    }

    /// 同页去重取最高优先级(FIFO 位次取最早);每页只加载一次、字节只计
    /// 一次。
    #[test]
    fn dedup_merges_highest_priority() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 16, 100, &[0])));
        let b = budget(150, 1_000_000, 1_000_000); // 每帧恰 1 页
        engine.submit_requests(&[req(1, 5, 3, 0), req(1, 9, 1, 0)]);
        engine.submit_requests(&[req(1, 5, 8, 0), req(1, 5, 2, 0)]);
        assert_eq!(engine.queue_len(), 2); // 4 次提交去重为 2 页
        let r0 = engine.tick(0, &b);
        assert_eq!(r0.pages_loaded, 1);
        assert!(engine.is_resident(1, 5)); // 合并优先级 8 胜出先载
        assert!(!engine.is_resident(1, 9));
        let r1 = engine.tick(1, &b);
        assert_eq!(r1.pages_loaded, 1);
        assert!(engine.is_resident(1, 9));
        assert_eq!(r0.bytes_io + r1.bytes_io, 200);
    }

    /// 已驻留请求零成本触帧:不进队列、不耗预算,且刷新 LRU 免驱逐。
    #[test]
    fn resident_request_zero_cost_touch() {
        let mut engine = StreamingEngine::new(3);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 100, &[0])));
        let b = budget(1_000_000, 1_000_000, 1_000_000);
        engine.submit_requests(&[req(1, 1, 1, 0), req(1, 2, 1, 0)]);
        assert_eq!(engine.tick(0, &b).pages_loaded, 2);
        // 已驻留再请求:零字节、零加载、零队列。
        engine.submit_requests(&[req(1, 1, 1, 1)]);
        assert_eq!(engine.queue_len(), 0);
        assert_eq!(engine.tick(1, &b), TickReport::default());
        // 触帧生效:page1 比 page2 新 → 新页 page3 驱逐 page2。
        engine.submit_requests(&[req(1, 3, 1, 2)]);
        let r = engine.tick(2, &b);
        assert_eq!((r.pages_loaded, r.pages_evicted), (1, 1));
        assert!(engine.is_resident(1, 1));
        assert!(!engine.is_resident(1, 2));
        assert!(engine.is_resident(1, 3));
    }

    /// io 段限额精确锚定每帧页数(2×1000 ≤ 2500 < 3×1000),三段独立计量。
    #[test]
    fn io_budget_segment_caps_pages() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0])));
        let b = budget(2500, u64::MAX, u64::MAX);
        engine.submit_requests(&[
            req(1, 1, 1, 0),
            req(1, 2, 1, 0),
            req(1, 3, 1, 0),
            req(1, 4, 1, 0),
            req(1, 5, 1, 0),
        ]);
        let r = engine.tick(0, &b);
        assert_eq!(r.pages_loaded, 2);
        assert_eq!(
            (r.bytes_io, r.bytes_transcode, r.bytes_upload),
            (2000, 2000, 2000)
        );
        assert_eq!(r.over_budget_stalls, 1);
        assert_eq!(r.queue_depth, 3);
    }

    /// transcode 段限额(转码输入字节):每帧恰 1 页;io/upload 不限时也只
    /// 计实际入池页的字节。
    #[test]
    fn transcode_budget_segment_caps_pages() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0])));
        let b = budget(u64::MAX, 1500, u64::MAX);
        engine.submit_requests(&[req(1, 1, 1, 0), req(1, 2, 1, 0)]);
        let r = engine.tick(0, &b);
        assert_eq!(r.pages_loaded, 1);
        assert_eq!(
            (r.bytes_io, r.bytes_transcode, r.bytes_upload),
            (1000, 1000, 1000)
        );
        assert_eq!(r.over_budget_stalls, 1);
        assert_eq!(r.queue_depth, 1);
    }

    /// upload 段限额(入池 payload 字节):每帧恰 1 页。
    #[test]
    fn upload_budget_segment_caps_pages() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0])));
        let b = budget(u64::MAX, u64::MAX, 1500);
        engine.submit_requests(&[req(1, 1, 1, 0), req(1, 2, 1, 0)]);
        let r = engine.tick(0, &b);
        assert_eq!(r.pages_loaded, 1);
        assert_eq!(r.bytes_upload, 1000);
        assert_eq!(r.over_budget_stalls, 1);
        assert_eq!(r.queue_depth, 1);
    }

    /// 预算耗尽:剩余请求原序滚入下帧(同级 FIFO 跨帧保持);预算每帧重置
    /// (每帧都有新额度可用)。
    #[test]
    fn budget_rollover_preserves_order_and_resets() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0])));
        let b = budget(1500, 1_000_000, 1_000_000); // 每帧恰 1 页
        engine.submit_requests(&[req(1, 3, 5, 0), req(1, 1, 5, 0), req(1, 2, 5, 0)]);
        for (f, page) in [3u32, 1, 2].iter().enumerate() {
            let r = engine.tick(f as u32, &b);
            assert_eq!(r.pages_loaded, 1);
            assert_eq!(r.bytes_io, 1000); // 预算每帧重置
            assert!(engine.is_resident(1, *page));
        }
        assert_eq!(engine.tick(3, &b).queue_depth, 0);
    }

    /// 自定义转码(XOR)确定性:入池 payload = raw ⊕ key;再转码一次往返
    /// 还原(转码接口留口的确定性语义锚定,RFC-0016 §9.1 R-4)。
    #[test]
    fn xor_transcode_deterministic_roundtrip() {
        let mut engine = StreamingEngine::new(4);
        let res = Synthetic::new(1, 4, 128, &[0]).with_xor(0xA5);
        let expected_raw = res.raw_page(2);
        engine.register_resource(Box::new(res));
        let b = budget(1_000_000, 1_000_000, 1_000_000);
        engine.submit_requests(&[req(1, 2, 1, 0)]);
        let r = engine.tick(0, &b);
        assert_eq!(
            (r.bytes_io, r.bytes_transcode, r.bytes_upload),
            (128, 128, 128)
        );
        let stored = page_data(&engine, 1, 2);
        let expected: Vec<u8> = expected_raw.iter().map(|b| b ^ 0xA5).collect();
        assert_eq!(stored, expected);
        // XOR 自逆:再转码一次 = 原始字节(确定性往返)。
        let roundtrip: Vec<u8> = stored.iter().map(|b| b ^ 0xA5).collect();
        assert_eq!(roundtrip, expected_raw);
    }

    /// 驱逐-重载往返:被驱逐页再请求 → 重新加载,字节与首载一致。
    #[test]
    fn eviction_reload_roundtrip_bytes_identical() {
        let mut engine = StreamingEngine::new(3);
        let res = Synthetic::new(1, 8, 256, &[0]);
        let expected = res.raw_page(1);
        engine.register_resource(Box::new(res));
        let b = budget(1_000_000, 1_000_000, 1_000_000);
        engine.submit_requests(&[req(1, 1, 1, 0), req(1, 2, 1, 0)]);
        let r0 = engine.tick(0, &b);
        assert_eq!((r0.pages_loaded, r0.bytes_io), (2, 512));
        let first = page_data(&engine, 1, 1);
        assert_eq!(first, expected);
        // 挤走 page1(池满:page1 是最久未触的未钉住页)。
        engine.submit_requests(&[req(1, 3, 1, 1)]);
        let r1 = engine.tick(1, &b);
        assert_eq!(r1.pages_evicted, 1);
        assert!(!engine.is_resident(1, 1));
        // 重载:字节与首载一致,IO 字节重新计量。
        engine.submit_requests(&[req(1, 1, 1, 2)]);
        let r2 = engine.tick(2, &b);
        assert_eq!((r2.pages_loaded, r2.bytes_io), (1, 256));
        assert_eq!(page_data(&engine, 1, 1), first);
    }

    /// pop-in 计数:`load_frame - request_frame >= 阈值` 才驻留 → +1。
    #[test]
    fn pop_in_counting_threshold() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0])));
        engine.set_pop_in_threshold(2);
        let b = budget(1500, 1_000_000, 1_000_000); // 每帧恰 1 页
        engine.submit_requests(&[req(1, 1, 1, 0), req(1, 2, 1, 0), req(1, 3, 1, 0)]);
        engine.tick(0, &b); // page1 请求帧即载(diff 0)→ 不计
        assert_eq!(engine.pop_in_count(), 0);
        engine.tick(1, &b); // page2 diff 1 < 2 → 不计
        assert_eq!(engine.pop_in_count(), 0);
        engine.tick(2, &b); // page3 diff 2 >= 2 → +1
        assert_eq!(engine.pop_in_count(), 1);
    }

    /// 未注册资源 / 越界页号请求:确定性丢弃,不阻塞队列、不结算 pop-in。
    #[test]
    fn invalid_requests_dropped() {
        let mut engine = StreamingEngine::new(4);
        engine.register_resource(Box::new(Synthetic::new(1, 4, 100, &[0])));
        let b = budget(1_000_000, 1_000_000, 1_000_000);
        engine.submit_requests(&[
            req(99, 0, 9, 0), // 未注册资源
            req(1, 99, 5, 0), // 越界页号
            req(1, 1, 1, 0),
        ]);
        let r = engine.tick(0, &b);
        assert_eq!(r.pages_loaded, 1);
        assert_eq!(r.queue_depth, 0);
        assert!(engine.is_resident(1, 1));
        assert_eq!(engine.pop_in_count(), 0);
    }

    /// 反馈桥接入:类目基值决定量级(几何 > 纹理),与登记序无关。
    #[test]
    fn feedback_category_base_orders_loading() {
        let mut engine = StreamingEngine::new(8);
        engine.register_resource(Box::new(Synthetic::new(1, 8, 1000, &[0]))); // 几何
        engine.register_resource(Box::new(Synthetic::new(2, 8, 1000, &[0]))); // 纹理
        let mut fb = FeedbackBuilder::new(0);
        // 纹理先登记(证明排序不依赖登记序,只依赖优先级公式)。
        fb.add(2, 1, FEEDBACK_BASE_TEXTURE_MISS, 999);
        fb.add(1, 1, FEEDBACK_BASE_GEOMETRY_LOD, 1);
        engine.submit_requests(&fb.build());
        let b = budget(1500, 1_000_000, 1_000_000); // 每帧恰 1 页
        engine.tick(0, &b);
        assert!(engine.is_resident(1, 1)); // 几何先载
        assert!(!engine.is_resident(2, 1));
        engine.tick(1, &b);
        assert!(engine.is_resident(2, 1));
    }

    /// 端到端:几何 16 页 + 纹理 8 页合成资源,逐帧请求驱动;TickReport
    /// 累计数字与 pop-in 计数按人工场景精确锚定(报告6 §6 验证口径)。
    #[test]
    fn end_to_end_geometry_and_texture() {
        let mut engine = StreamingEngine::new(32);
        engine.register_resource(Box::new(Synthetic::new(1, 16, 1000, &[0]))); // 几何
        engine.register_resource(Box::new(Synthetic::new(2, 8, 1000, &[0]))); // 纹理
        assert!(engine.is_resident(1, 0) && engine.is_resident(2, 0));
        let b = budget(2500, 2500, 2500); // 每帧恰 2 页(1000B/页)
        // 帧 0:LOD cut 选中几何页 1..=8(同优先级 FIFO)+ 纹理页 1..=2(低
        // 优先级类目)。
        engine.submit_requests(&[
            req(1, 1, 100, 0),
            req(1, 2, 100, 0),
            req(1, 3, 100, 0),
            req(1, 4, 100, 0),
            req(1, 5, 100, 0),
            req(1, 6, 100, 0),
            req(1, 7, 100, 0),
            req(1, 8, 100, 0),
            req(2, 1, 10, 0),
            req(2, 2, 10, 0),
        ]);
        // 10 页 × 1000B,每帧 2 页 → 5 帧;几何优先 → 帧 0..4 全几何,帧 4 纹理。
        let mut total = TickReport::default();
        for f in 0..5u32 {
            let r = engine.tick(f, &b);
            total.pages_loaded += r.pages_loaded;
            total.pages_evicted += r.pages_evicted;
            total.bytes_io += r.bytes_io;
            total.bytes_transcode += r.bytes_transcode;
            total.bytes_upload += r.bytes_upload;
            total.over_budget_stalls += r.over_budget_stalls;
        }
        // 全部驻留。
        for p in 1..=8u32 {
            assert!(engine.is_resident(1, p));
        }
        for p in 1..=2u32 {
            assert!(engine.is_resident(2, p));
        }
        // 累计锚定:10 页 × 1000B,恒等转码三段同字节;池 32 槽无驱逐;
        // 帧 0..3 各因第 3 页装不下触发即停,帧 4 队列清空不停。
        assert_eq!(total.pages_loaded, 10);
        assert_eq!(total.pages_evicted, 0);
        assert_eq!(
            (total.bytes_io, total.bytes_transcode, total.bytes_upload),
            (10_000, 10_000, 10_000)
        );
        assert_eq!(total.over_budget_stalls, 4);
        // pop-in(默认阈值 1):帧 0 加载的 2 页 diff 0 不计;帧 1..4 加载的
        // 8 页 diff ≥ 1 → 8 次。
        assert_eq!(engine.pop_in_count(), 8);
        assert_eq!(engine.queue_len(), 0);
    }
}
