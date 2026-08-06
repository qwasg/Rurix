//! 不透明句柄(RFC-0017 §4.A2,冻结接口):`BodyId(u64)` / `ShapeId(u64)` =
//! index 32b(低)+ generation 32b(高)。FFI 边界只过 u64,不过原生指针(§4.C3);
//! 渲染器/宿主只握本类型,永不见原生 Jolt/Rapier 指针(§4.C4 审计判据)。
//!
//! generation 纪律(I-6 评审修订)由 [`crate::arena`] 执行:槽位复用时
//! generation 单调递增;32b generation 空间耗尽的槽位退休不再分配(回绕复活
//! 路径类型面消灭);world 生命周期内 index 池耗尽 → `Err(PoolExhausted)`。

use std::fmt;

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// 由 arena 部件构造(index 低 32b + generation 高 32b)。
            pub(crate) fn new(index: u32, generation: u32) -> Self {
                Self((u64::from(generation) << 32) | u64::from(index))
            }

            /// 槽位 index(低 32b)。
            pub fn index(self) -> u32 {
                self.0 as u32
            }

            /// generation(高 32b,槽位复用时单调递增)。
            pub fn generation(self) -> u32 {
                (self.0 >> 32) as u32
            }

            /// u64 位表示(FFI/持久化边界只过此值,§4.A2)。
            pub fn to_bits(self) -> u64 {
                self.0
            }

            /// 自持久化/journal 位表示还原(失效 generation 仍由 arena 门禁)。
            pub fn from_bits(bits: u64) -> Self {
                Self(bits)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", self.index(), self.generation())
            }
        }
    };
}

opaque_id! {
    /// body 不透明句柄(§4.A2;索引/generation 语义见模块文档)。
    BodyId
}

opaque_id! {
    /// shape 不透明句柄(§4.A2;每个 body 恰持一个 shape,生命周期随 body)。
    ShapeId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_layout_index_low_generation_high() {
        let id = BodyId::new(0xABCD_EF01, 0x1234_5678);
        assert_eq!(id.index(), 0xABCD_EF01);
        assert_eq!(id.generation(), 0x1234_5678);
        assert_eq!(id.to_bits(), 0x1234_5678_ABCD_EF01);
        // 规范序/Hash 基于 u64 位表示(generation 优先于 index)。
        assert!(BodyId::new(0, 2) > BodyId::new(u32::MAX, 1));
    }

    #[test]
    fn body_and_shape_ids_are_distinct_types() {
        let b = BodyId::new(1, 1);
        let s = ShapeId::new(1, 1);
        assert_eq!(b.to_bits(), s.to_bits());
        assert_eq!(format!("{b}"), "1#1");
    }
}
