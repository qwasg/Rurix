//! 资源类型注册接口(报告6 §3「映射的关键决策」:几何页、KTX2 纹理页、未来
//! 的 SVT 页都是注册进通用运行时的资源类型;RFC-0016 §4.G4)。

/// 可分页流送资源(报告6 §2.4 页式驻留;§5 数据结构)。
///
/// 实现侧契约:
/// - 全部方法**确定性**:同输入同输出(离线数据内存持有,`read_page` 模拟 IO
///   源;host 单测逐字节锚定,报告6 §6「解压页与离线参考逐字节一致」);
/// - 单页字节 ≤ [`STREAM_PAGE_SIZE`](crate::graph::types::STREAM_PAGE_SIZE)
///   (128KB,Nanite 页共识,报告6 #3),引擎侧对 `read_page`/`transcode` 产物
///   做断言;
/// - 引擎只对已注册页号(`0..page_count`)调用 `read_page`/`transcode`,越界
///   请求在进入资源前即被丢弃。
pub trait PagedResource {
    /// 流送资源注册号(对应 [`crate::graph::types::PageRequest::resource`];
    /// 引擎内唯一,重复注册 panic)。
    fn resource_id(&self) -> u32;
    /// 资源内页总数。
    fn page_count(&self) -> u32;
    /// DAG 顶层常驻页(root pages):注册即强制加载并钉住,永不驱逐——
    /// 「永远有可渲染的东西」(报告6 §2.4 Nanite root page 常驻;RFC-0016
    /// §4.C 序列化预留常驻标志的兑现点)。
    fn root_pages(&self) -> &[u32];
    /// 读取页原始字节(≤128KB;模拟 IO 源)。
    fn read_page(&self, page: u32) -> Vec<u8>;
    /// 页转码(确定性转换;输入 = `read_page` 原始字节,输出 = ≤128KB 入池
    /// payload)。
    ///
    /// 默认恒等——真 KTX2/BasisU → BC 转码归 RD-037+ 存续(RFC-0016 §9.1
    /// R-4 裁决,本期页 payload 为未压缩/简单打包档),本接口留口;实现必须
    /// 同输入同输出。
    fn transcode(&self, _page: u32, raw: &[u8]) -> Vec<u8> {
        raw.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{PageRequest, STREAM_PAGE_SIZE, StreamingBudget};

    /// 冻结契约消费(编译层面证明):`PageRequest`/`StreamingBudget`/
    /// `STREAM_PAGE_SIZE` 引用自 `graph::types` 单源,字段名/类型/布局以冻结
    /// 文件为准(G5_PLAN §2;types.rs 头注「不得漂移」)。
    #[test]
    fn frozen_contract_consumption() {
        assert_eq!(STREAM_PAGE_SIZE, 128 * 1024);
        let req = PageRequest {
            resource: 7,
            page_index: 3,
            priority: 42,
            frame: 9,
        };
        assert_eq!(
            (req.resource, req.page_index, req.priority, req.frame),
            (7, 3, 42, 9)
        );
        // repr(C) GPU 回读缓冲元素,16B 定长(types.rs 自有锚定测试,此处复核)。
        assert_eq!(core::mem::size_of::<PageRequest>(), 16);
        let budget = StreamingBudget {
            io_bytes: 1,
            transcode_bytes: 2,
            upload_bytes: 3,
        };
        assert_eq!(
            (budget.io_bytes, budget.transcode_bytes, budget.upload_bytes),
            (1, 2, 3)
        );
    }

    struct Identity {
        raw: Vec<u8>,
    }

    impl PagedResource for Identity {
        fn resource_id(&self) -> u32 {
            0
        }
        fn page_count(&self) -> u32 {
            1
        }
        fn root_pages(&self) -> &[u32] {
            &[]
        }
        fn read_page(&self, _page: u32) -> Vec<u8> {
            self.raw.clone()
        }
    }

    /// 恒等转码(默认实现)字节一致——真转码归 RD-037+ 存续(RFC-0016 §9.1
    /// R-4),本期接口留口的默认语义即「payload = 原始页」。
    #[test]
    fn default_transcode_is_identity() {
        let r = Identity {
            raw: (0u8..=255).collect(),
        };
        let raw = r.read_page(0);
        assert_eq!(r.transcode(0, &raw), raw);
    }
}
