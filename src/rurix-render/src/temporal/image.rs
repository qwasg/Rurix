//! 时域底座基础图像容器(报告7 §2.1:host 纯 Rust CPU 参考实现的公共载体,
//! 同时是 device shader 的对拍金标准与单测载体)。
//!
//! UV 约定:uv ∈ \[0,1\],原点左上(与帧图纹理一致),纹素 (x,y) 中心位于
//! ((x+0.5)/w, (y+0.5)/h);采样越界一律 clamp 边界。

/// f32 像素容器(行主序、通道交错,c ∈ 1..=4)。
#[derive(Debug, Clone, PartialEq)]
pub struct ImageF32 {
    pub w: u32,
    pub h: u32,
    /// 通道数(1=深度/mask,2=MV,3=RGB/YCoCg,4=RGBA)。
    pub c: u32,
    /// 长度 = w*h*c;布局 (((y*w)+x)*c)+ch。
    pub data: Vec<f32>,
}

impl ImageF32 {
    /// 零初始化图像;c 必须 ∈ 1..=4。
    pub fn new(w: u32, h: u32, c: u32) -> Self {
        assert!(w >= 1 && h >= 1, "图像尺寸必须 ≥1");
        assert!((1..=4).contains(&c), "通道数必须 ∈ 1..=4");
        Self {
            w,
            h,
            c,
            data: vec![0.0; (w * h * c) as usize],
        }
    }

    /// 按 (x, y, ch) 过程式构造(单测合成场景用)。
    pub fn from_fn(w: u32, h: u32, c: u32, f: impl Fn(u32, u32, u32) -> f32) -> Self {
        let mut img = Self::new(w, h, c);
        for y in 0..h {
            for x in 0..w {
                for ch in 0..c {
                    img.set(x, y, ch, f(x, y, ch));
                }
            }
        }
        img
    }

    /// 同尺寸同通道判定。
    pub fn same_shape(&self, other: &ImageF32) -> bool {
        self.w == other.w && self.h == other.h && self.c == other.c
    }

    fn index(&self, x: u32, y: u32, ch: u32) -> usize {
        debug_assert!(x < self.w && y < self.h && ch < self.c);
        ((y * self.w + x) * self.c + ch) as usize
    }

    pub fn get(&self, x: u32, y: u32, ch: u32) -> f32 {
        self.data[self.index(x, y, ch)]
    }

    pub fn set(&mut self, x: u32, y: u32, ch: u32, v: f32) {
        let i = self.index(x, y, ch);
        self.data[i] = v;
    }

    /// 取前 3 通道(要求 c ≥ 3;RGB/YCoCg 像素访问)。
    pub fn pixel3(&self, x: u32, y: u32) -> [f32; 3] {
        [self.get(x, y, 0), self.get(x, y, 1), self.get(x, y, 2)]
    }

    /// 写前 3 通道(要求 c ≥ 3)。
    pub fn set_pixel3(&mut self, x: u32, y: u32, p: [f32; 3]) {
        self.set(x, y, 0, p[0]);
        self.set(x, y, 1, p[1]);
        self.set(x, y, 2, p[2]);
    }

    /// 最近邻采样(clamp 边界;uv 约定见模块文档)。
    pub fn sample_nearest(&self, u: f32, v: f32, ch: u32) -> f32 {
        let x = (u * self.w as f32).floor() as i32;
        let y = (v * self.h as f32).floor() as i32;
        let x = x.clamp(0, self.w as i32 - 1) as u32;
        let y = y.clamp(0, self.h as i32 - 1) as u32;
        self.get(x, y, ch)
    }

    /// 双线性采样(clamp 边界;uv 约定见模块文档)。
    pub fn sample_bilinear(&self, u: f32, v: f32, ch: u32) -> f32 {
        let xf = u * self.w as f32 - 0.5;
        let yf = v * self.h as f32 - 0.5;
        let x0 = xf.floor() as i32;
        let y0 = yf.floor() as i32;
        let fx = xf - x0 as f32;
        let fy = yf - y0 as f32;
        let cx = |xx: i32| xx.clamp(0, self.w as i32 - 1) as u32;
        let cy = |yy: i32| yy.clamp(0, self.h as i32 - 1) as u32;
        let (xa, xb) = (cx(x0), cx(x0 + 1));
        let (ya, yb) = (cy(y0), cy(y0 + 1));
        let top = self.get(xa, ya, ch) * (1.0 - fx) + self.get(xb, ya, ch) * fx;
        let bot = self.get(xa, yb, ch) * (1.0 - fx) + self.get(xb, yb, ch) * fx;
        top * (1.0 - fy) + bot * fy
    }

    /// 双线性采样前 3 通道(要求 c ≥ 3)。
    pub fn sample_bilinear3(&self, u: f32, v: f32) -> [f32; 3] {
        [
            self.sample_bilinear(u, v, 0),
            self.sample_bilinear(u, v, 1),
            self.sample_bilinear(u, v, 2),
        ]
    }

    /// 逐通道最小值(长度 = c)。
    pub fn min_per_channel(&self) -> Vec<f32> {
        let mut out = vec![f32::INFINITY; self.c as usize];
        for (i, &v) in self.data.iter().enumerate() {
            let ch = i % self.c as usize;
            out[ch] = out[ch].min(v);
        }
        out
    }

    /// 逐通道最大值(长度 = c)。
    pub fn max_per_channel(&self) -> Vec<f32> {
        let mut out = vec![f32::NEG_INFINITY; self.c as usize];
        for (i, &v) in self.data.iter().enumerate() {
            let ch = i % self.c as usize;
            out[ch] = out[ch].max(v);
        }
        out
    }

    /// 逐通道线性插值 out = a*(1-t) + b*t(要求同尺寸同通道)。
    pub fn lerp(a: &ImageF32, b: &ImageF32, t: f32) -> ImageF32 {
        assert!(a.same_shape(b), "lerp 要求同尺寸同通道");
        let mut out = a.clone();
        for (o, (&va, &vb)) in out.data.iter_mut().zip(a.data.iter().zip(b.data.iter())) {
            *o = va * (1.0 - t) + vb * t;
        }
        out
    }

    /// 均方误差(全通道平均,f64 累加;收敛验收计量用)。
    pub fn mse(a: &ImageF32, b: &ImageF32) -> f64 {
        assert!(a.same_shape(b), "mse 要求同尺寸同通道");
        let acc: f64 = a
            .data
            .iter()
            .zip(b.data.iter())
            .map(|(&va, &vb)| {
                let d = f64::from(va) - f64::from(vb);
                d * d
            })
            .sum();
        acc / a.data.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> ImageF32 {
        // 2x2 单通道 [[0,1],[2,3]]
        let mut img = ImageF32::new(2, 2, 1);
        img.set(0, 0, 0, 0.0);
        img.set(1, 0, 0, 1.0);
        img.set(0, 1, 0, 2.0);
        img.set(1, 1, 0, 3.0);
        img
    }

    #[test]
    fn bilinear_center_and_clamp() {
        let img = quad();
        // 纹素中心精确取值
        assert!((img.sample_bilinear(0.25, 0.25, 0) - 0.0).abs() < 1e-6);
        assert!((img.sample_bilinear(0.75, 0.25, 0) - 1.0).abs() < 1e-6);
        assert!((img.sample_bilinear(0.25, 0.75, 0) - 2.0).abs() < 1e-6);
        // 四纹素正中 = 均值
        assert!((img.sample_bilinear(0.5, 0.5, 0) - 1.5).abs() < 1e-6);
        // 越界 clamp 到角/边
        assert!((img.sample_bilinear(-1.0, -1.0, 0) - 0.0).abs() < 1e-6);
        assert!((img.sample_bilinear(2.0, 2.0, 0) - 3.0).abs() < 1e-6);
        assert!((img.sample_bilinear(0.25, -5.0, 0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn nearest_sampling() {
        let img = quad();
        assert!((img.sample_nearest(0.25, 0.25, 0) - 0.0).abs() < 1e-6);
        assert!((img.sample_nearest(0.75, 0.75, 0) - 3.0).abs() < 1e-6);
        assert!((img.sample_nearest(0.6, 0.1, 0) - 1.0).abs() < 1e-6);
        // 越界 clamp
        assert!((img.sample_nearest(-2.0, 9.0, 0) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn min_max_lerp_mse() {
        let a = ImageF32::from_fn(4, 4, 3, |x, y, ch| (x + y + ch) as f32 * 0.1);
        let mn = a.min_per_channel();
        let mx = a.max_per_channel();
        assert_eq!(mn.len(), 3);
        assert!((mn[0] - 0.0).abs() < 1e-6 && (mx[2] - 0.8).abs() < 1e-6);
        let b = ImageF32::from_fn(4, 4, 3, |_, _, _| 1.0);
        let mid = ImageF32::lerp(&a, &b, 0.5);
        for i in 0..mid.data.len() {
            assert!((mid.data[i] - (a.data[i] * 0.5 + 0.5)).abs() < 1e-6);
        }
        assert!(ImageF32::mse(&a, &a).abs() < 1e-12);
        // 与常数 1 图像的 MSE = mean((v-1)^2) > 0
        assert!(ImageF32::mse(&a, &b) > 0.0);
    }
}
