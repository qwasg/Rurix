//! 极简三维向量运算(全仓零外部依赖纪律;仅本 crate 内部使用)。

#[inline]
pub(crate) fn vsub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub(crate) fn vscale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[inline]
pub(crate) fn vdot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub(crate) fn vlen(a: [f32; 3]) -> f32 {
    vdot(a, a).sqrt()
}

#[inline]
pub(crate) fn vdist(a: [f32; 3], b: [f32; 3]) -> f32 {
    vlen(vsub(a, b))
}

#[inline]
pub(crate) fn vcross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 归一化;零向量返回 None(调用方按退化处理)。
#[inline]
pub(crate) fn vnorm(a: [f32; 3]) -> Option<[f32; 3]> {
    let l = vlen(a);
    if l <= 1e-12 {
        None
    } else {
        Some(vscale(a, 1.0 / l))
    }
}
