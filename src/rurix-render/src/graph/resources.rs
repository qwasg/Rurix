//! 资源节点与 transient 池分桶(报告5 §2.3/§5;RFC-0016 章 A)。
//!
//! 录制期只有描述符与句柄,物理分配延迟到编译后的 transient 池;imported 资源
//! 图只推导状态转换不管理内存(报告5 §2.3 约束一),故分桶函数只服务 transient。

use crate::graph::types::{ResourceDesc, ResourceId, ResourceKind};

/// 图内资源节点(录制期描述符 + 句柄;句柄与生命周期绑定,越期使用由编译器拒)。
#[derive(Debug)]
pub(crate) struct ResourceNode {
    pub(crate) id: ResourceId,
    pub(crate) desc: ResourceDesc,
}

impl ResourceNode {
    /// 是否纹理(layout 轴仅对纹理有意义;buffer 恒 Undefined——契约注释)。
    pub(crate) fn is_texture(&self) -> bool {
        matches!(self.desc.kind, ResourceKind::Texture2d { .. })
    }
}

/// transient 池分桶键(类别 × 尺寸级,报告5 §5「按对齐/用途类别分池」的 P1 最小形)。
///
/// - buffer 与纹理物理类别不同,永不共槽;
/// - 尺寸级 = `byte_size` 的 log2 上取整档——同级内容许跨 format 别名(尺寸兼容
///   即可共享物理页,格式差异由别名交接的 `layout_before=Undefined` 丢弃语义吸收);
/// - 桶内槽序号见 [`crate::graph::transient`]。
pub(crate) fn pool_bucket(kind: &ResourceKind) -> u32 {
    let category: u32 = match kind {
        ResourceKind::Buffer { .. } => 0,
        ResourceKind::Texture2d { .. } => 1,
    };
    category * 128 + size_class(kind.byte_size())
}

/// `byte_size` 的 log2 上取整档(0/1 → 0;>2^x → x+1)。
fn size_class(byte_size: u64) -> u32 {
    if byte_size <= 1 {
        0
    } else {
        64 - (byte_size - 1).leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::TextureFormat;

    fn tex_1mb() -> ResourceKind {
        ResourceKind::Texture2d {
            width: 512,
            height: 512,
            format: TextureFormat::Rgba8Unorm,
            mip_levels: 1,
        }
    }

    /// 分桶:buffer 与纹理类别隔离(同尺寸也不共槽)。
    #[test]
    fn bucket_separates_categories() {
        let buf = ResourceKind::Buffer { size: 1024 * 1024 };
        assert_ne!(pool_bucket(&tex_1mb()), pool_bucket(&buf));
    }

    /// 尺寸级 = log2 上取整:档内同桶,跨档异桶。
    #[test]
    fn bucket_size_class_log2_ceil() {
        let b = |size: u64| pool_bucket(&ResourceKind::Buffer { size });
        assert_eq!(b(0), b(1));
        assert_eq!(b(700_000), b(900_000)); // 同属 2^20 档
        assert_ne!(b(1024 * 1024), b(1024 * 1024 + 1)); // 2^20 与 2^21 档
    }

    /// mip 链尺寸 = base + base/3 保守上界(契约 ResourceKind::byte_size 口径)。
    #[test]
    fn byte_size_mip_chain_upper_bound() {
        let k = ResourceKind::Texture2d {
            width: 4,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
            mip_levels: 3,
        };
        assert_eq!(k.byte_size(), 16 * 4 + (16 * 4) / 3);
    }
}
