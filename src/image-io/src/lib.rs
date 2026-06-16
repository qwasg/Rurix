//! image-io — Rurix 确定性图像序列输出包骨架(M7.2,D-M7-2)。
//!
//! 条款:spec/imageio.md RXS-0114 ~ RXS-0117(图像缓冲与像素类型面 / 无损格式
//! 优先与格式选择 / 确定性字节布局与 header 规范化 / 图像序列落盘接口)。
//!
//! 纪律:**host-only 单路径**(不引入 device codegen,区别于 stdlib 双路径);
//! **全 safe**(`unsafe_code = "deny"`,继承 workspace lints);**零外部依赖**
//! (标准库 + 手写确定性 PPM P6 编码),纯函数、确定性——同一输入在不同机器 /
//! 时刻产**逐字节一致**字节流,为 M7.4 UC-03 demo 出图落盘与 G-M7-1 逐帧
//! content SHA-256 复现铺底。
//!
//! 运行期失败(格式不支持 / 写入失败)以**库层 [`ImageError`] 错误值**表达
//! (`Result::Err`),**不分配编译器 RX 段位**(spec/imageio.md §3)。

use std::path::{Path, PathBuf};

/// 像素抽象:确定性量化到 8-bit RGB 三通道(PPM P6 无 alpha 通道,RXS-0116)。
pub trait Pixel: Copy {
    /// 按 RXS-0116 确定量化产 `[R, G, B]` 三个 `u8`(通道序 R, G, B)。
    fn to_rgb8(self) -> [u8; 3];
}

/// RGB 像素(分量 `f32`,通道序 R, G, B;RXS-0114)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// RGBA 像素(分量 `f32`,通道序 R, G, B, A;RXS-0114)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgb {
    /// 构造 RGB 像素(RXS-0114)。
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

impl Rgba {
    /// 构造 RGBA 像素(RXS-0114)。
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// `f32` 分量 → `u8` 确定量化(RXS-0116):钳制 `[0,1]`(NaN→0)后就近取整
/// `floor(clamp(c) * 255.0 + 0.5)`(半值向上),映射到 `[0, 255]`。
fn quantize_u8(c: f32) -> u8 {
    let clamped = if c.is_nan() { 0.0 } else { c.clamp(0.0, 1.0) };
    (clamped * 255.0 + 0.5).floor() as u8
}

impl Pixel for Rgb {
    fn to_rgb8(self) -> [u8; 3] {
        [
            quantize_u8(self.r),
            quantize_u8(self.g),
            quantize_u8(self.b),
        ]
    }
}

impl Pixel for Rgba {
    fn to_rgb8(self) -> [u8; 3] {
        // RXS-0116:PPM P6 无 alpha 通道,编码丢弃 alpha,仅写 RGB。
        [
            quantize_u8(self.r),
            quantize_u8(self.g),
            quantize_u8(self.b),
        ]
    }
}

/// 行主序图像缓冲(宽 × 高,统一像素元素类型;RXS-0114)。
///
/// 像素 `(x, y)`(列 `x`、行 `y`)的线性下标为 `y * width + x`。
#[derive(Debug, Clone, PartialEq)]
pub struct ImageBuffer<P: Pixel> {
    width: u32,
    height: u32,
    pixels: Vec<P>,
}

impl<P: Pixel> ImageBuffer<P> {
    /// 构造 `width × height` 缓冲,每像素初始化为 `fill`(RXS-0114)。
    pub fn new(width: u32, height: u32, fill: P) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            pixels: vec![fill; len],
        }
    }

    /// 宽(列数)。
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 高(行数)。
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 行主序线性下标 `y * width + x`;越界返回 `None`(确定性,不进入 UB)。
    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    /// 取 `(x, y)` 处像素副本(值语义);越界返回 `None`(RXS-0114)。
    pub fn get(&self, x: u32, y: u32) -> Option<P> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// 置 `(x, y)` 处像素为 `p`;越界返回 `false`、不写越界存储(RXS-0114)。
    pub fn set(&mut self, x: u32, y: u32, p: P) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.pixels[i] = p;
                true
            }
            None => false,
        }
    }
}

/// 无损图像格式(无损优先序:PPM P6 优先 / PNG 次;RXS-0115)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// PPM(P6,二进制 RGB)——M7.2 落地的首选无损格式。
    Ppm,
    /// PNG(无损)——次选,加性后续(本轮未实现,`encode` 返回 `UnsupportedFormat`)。
    Png,
}

impl ImageFormat {
    /// 规范化文件扩展名(含点);PNG 为后续扩展(RXS-0117 帧命名)。
    fn extension(self) -> &'static str {
        match self {
            ImageFormat::Ppm => ".ppm",
            ImageFormat::Png => ".png",
        }
    }
}

/// image-io 库层错误值(RXS-0115 / RXS-0117;**不映射编译器 RX 段位**,
/// spec/imageio.md §3——运行期失败以 `Result::Err` 表达)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// 目标格式未被当前实现支持(如本轮 `Png`)。
    UnsupportedFormat,
    /// 落盘写入失败(目录不存在 / IO 错误);携带细节描述。
    WriteFailed(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::UnsupportedFormat => write!(f, "image-io: 不支持的图像格式"),
            ImageError::WriteFailed(d) => write!(f, "image-io: 写入失败: {d}"),
        }
    }
}

impl std::error::Error for ImageError {}

/// image-io 操作统一结果别名。
pub type ImageResult<T> = Result<T, ImageError>;

/// 编码图像缓冲为目标格式的确定字节流(RXS-0115);无损优先序见 [`ImageFormat`]。
///
/// 成功 → 确定 `Vec<u8>`;格式不支持 → `Err(ImageError::UnsupportedFormat)`,
/// 不产生部分 / 非确定字节流。
pub fn encode<P: Pixel>(buf: &ImageBuffer<P>, fmt: ImageFormat) -> ImageResult<Vec<u8>> {
    match fmt {
        ImageFormat::Ppm => Ok(encode_ppm(buf)),
        // PNG 为加性后续(RXS-0115);本轮以库层错误值表达,不分配 RX 段位。
        ImageFormat::Png => Err(ImageError::UnsupportedFormat),
    }
}

/// PPM P6 确定编码(RXS-0116):规范化 header `"P6\n{w} {h}\n255\n"` + 行主序
/// (上→下、左→右)逐像素通道序 R,G,B 的 `u8` 字节。
fn encode_ppm<P: Pixel>(buf: &ImageBuffer<P>) -> Vec<u8> {
    let header = format!("P6\n{} {}\n255\n", buf.width, buf.height);
    let mut bytes = Vec::with_capacity(header.len() + buf.pixels.len() * 3);
    bytes.extend_from_slice(header.as_bytes());
    // 行主序:行 y 自上而下,列 x 自左而右(RXS-0116 像素数据序)。
    for y in 0..buf.height {
        for x in 0..buf.width {
            let i = y as usize * buf.width as usize + x as usize;
            let [r, g, b] = buf.pixels[i].to_rgb8();
            bytes.push(r);
            bytes.push(g);
            bytes.push(b);
        }
    }
    bytes
}

/// 单帧落盘记录(RXS-0117):帧序号 / 规范化文件名 / 帧字节长度。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameRecord {
    /// 序列内帧序号(自 0 起单调递增)。
    pub index: u32,
    /// 规范化文件名(零填充定宽序号 + 格式扩展名)。
    pub file_name: String,
    /// 帧确定字节流长度。
    pub byte_len: usize,
}

/// 确定性图像序列 sink(RXS-0117):逐帧编码 → 落盘到目录;逐帧 content
/// SHA-256 可核对、同输入两次落盘逐字节一致(不引入时间戳 / 随机量)。
#[derive(Debug, Clone)]
pub struct ImageSequence {
    dir: PathBuf,
    count: u32,
}

impl ImageSequence {
    /// 以落盘目录构造空序列(RXS-0117)。
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            count: 0,
        }
    }

    /// 已累加帧数。
    pub fn frame_count(&self) -> u32 {
        self.count
    }

    /// 第 `index` 帧的规范化路径(零填充 5 位序号 + 扩展名;确定性命名,RXS-0117)。
    pub fn frame_path(&self, index: u32, fmt: ImageFormat) -> PathBuf {
        self.dir
            .join(format!("frame_{:05}{}", index, fmt.extension()))
    }

    /// 编码一帧并落盘,返回 [`FrameRecord`](RXS-0117)。
    ///
    /// 编码失败(格式不支持)→ `Err(UnsupportedFormat)`(此时不递增帧号、不落盘);
    /// 写入失败 → `Err(WriteFailed)`。
    pub fn push_frame<P: Pixel>(
        &mut self,
        buf: &ImageBuffer<P>,
        fmt: ImageFormat,
    ) -> ImageResult<FrameRecord> {
        let bytes = encode(buf, fmt)?;
        let path = self.frame_path(self.count, fmt);
        write_bytes(&path, &bytes)?;
        let record = FrameRecord {
            index: self.count,
            file_name: file_name_of(&path),
            byte_len: bytes.len(),
        };
        self.count += 1;
        Ok(record)
    }
}

/// 落盘字节(写入失败映射库层 `WriteFailed`,不 panic,RXS-0117)。
fn write_bytes(path: &Path, bytes: &[u8]) -> ImageResult<()> {
    std::fs::write(path, bytes).map_err(|e| ImageError::WriteFailed(e.to_string()))
}

/// 取路径文件名(用于 [`FrameRecord::file_name`])。
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0114
    // 图像缓冲与像素类型面:Rgb/Rgba 构造、ImageBuffer 行主序 get/set、越界确定性。
    #[test]
    fn rgb_rgba_construct_and_buffer_row_major() {
        let p = Rgb::new(0.25, 0.5, 0.75);
        assert_eq!(
            p,
            Rgb {
                r: 0.25,
                g: 0.5,
                b: 0.75
            }
        );
        let q = Rgba::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(q.a, 0.4);

        let mut buf = ImageBuffer::new(3, 2, Rgb::new(0.0, 0.0, 0.0));
        assert_eq!(buf.width(), 3);
        assert_eq!(buf.height(), 2);
        // 行主序:(x=2, y=1) 线性下标 = 1*3 + 2 = 5(末像素)。
        assert!(buf.set(2, 1, Rgb::new(1.0, 0.0, 0.0)));
        assert_eq!(buf.get(2, 1), Some(Rgb::new(1.0, 0.0, 0.0)));
        assert_eq!(buf.get(0, 0), Some(Rgb::new(0.0, 0.0, 0.0)));
        // 越界确定性:返回 None / false,不进入 UB。
        assert_eq!(buf.get(3, 0), None);
        assert!(!buf.set(0, 2, Rgb::new(1.0, 1.0, 1.0)));
    }

    //@ spec: RXS-0115
    // 无损格式优先与格式选择:PPM P6 成功产字节流;PNG 次选本轮未实现 → 库层
    // UnsupportedFormat 错误值(不分配 RX 段位)。
    #[test]
    fn format_priority_ppm_ok_png_unsupported() {
        let buf = ImageBuffer::new(1, 1, Rgb::new(0.0, 0.0, 0.0));
        let ppm = encode(&buf, ImageFormat::Ppm);
        assert!(ppm.is_ok());
        assert!(!ppm.unwrap().is_empty());
        assert_eq!(
            encode(&buf, ImageFormat::Png),
            Err(ImageError::UnsupportedFormat)
        );
    }

    //@ spec: RXS-0116
    // 确定性字节布局与 header 规范化:PPM P6 header golden、行主序 + 通道序 R,G,B、
    // f32→u8 确定量化、同输入两次编码逐字节一致、Rgba 丢弃 alpha。
    #[test]
    fn ppm_header_and_byte_layout_deterministic() {
        // 2x1:像素0 = (1,0,0) → 255,0,0;像素1 = (0,1,0) → 0,255,0。
        let mut buf = ImageBuffer::new(2, 1, Rgb::new(0.0, 0.0, 0.0));
        buf.set(0, 0, Rgb::new(1.0, 0.0, 0.0));
        buf.set(1, 0, Rgb::new(0.0, 1.0, 0.0));
        let bytes = encode(&buf, ImageFormat::Ppm).unwrap();

        let header = b"P6\n2 1\n255\n";
        assert_eq!(&bytes[..header.len()], header);
        // header 之后:行主序 + 通道序 R,G,B。
        assert_eq!(&bytes[header.len()..], &[255, 0, 0, 0, 255, 0]);

        // 同输入两次编码逐字节一致(确定性,RXS-0116)。
        let bytes2 = encode(&buf, ImageFormat::Ppm).unwrap();
        assert_eq!(bytes, bytes2);

        // f32→u8 确定量化边界:钳制 + 就近取整(半值向上)+ NaN→0。
        assert_eq!(quantize_u8(0.0), 0);
        assert_eq!(quantize_u8(1.0), 255);
        assert_eq!(quantize_u8(-1.0), 0);
        assert_eq!(quantize_u8(2.0), 255);
        assert_eq!(quantize_u8(f32::NAN), 0);
        assert_eq!(quantize_u8(0.5), 128); // floor(0.5*255 + 0.5) = floor(128.0) = 128

        // Rgba 编码丢弃 alpha,仅写 RGB(与同 RGB 的 Rgb 字节一致)。
        let mut rgba_buf = ImageBuffer::new(1, 1, Rgba::new(0.0, 0.0, 0.0, 0.0));
        rgba_buf.set(0, 0, Rgba::new(1.0, 0.0, 0.0, 0.123));
        let rgba_bytes = encode(&rgba_buf, ImageFormat::Ppm).unwrap();
        assert_eq!(
            &rgba_bytes[..],
            &[
                b'P', b'6', b'\n', b'1', b' ', b'1', b'\n', b'2', b'5', b'5', b'\n', 255, 0, 0
            ]
        );
    }

    //@ spec: RXS-0117
    // 图像序列落盘接口:frame_path 规范化命名、push_frame 落盘字节与 encode 一致、
    // 序列同输入两次落盘逐字节一致(content 可核对地基)。
    #[test]
    fn image_sequence_frame_naming_and_repro() {
        let base =
            std::env::temp_dir().join(format!("rurix_imgio_test_{}_{}", std::process::id(), "seq"));
        let dir_a = base.join("run_a");
        let dir_b = base.join("run_b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        let frames = build_fixed_frames();

        // 帧命名规范化(零填充 5 位 + .ppm)。
        let seq_probe = ImageSequence::new(&dir_a);
        assert_eq!(
            seq_probe
                .frame_path(0, ImageFormat::Ppm)
                .file_name()
                .unwrap(),
            "frame_00000.ppm"
        );
        assert_eq!(
            seq_probe
                .frame_path(12, ImageFormat::Ppm)
                .file_name()
                .unwrap(),
            "frame_00012.ppm"
        );

        let bytes_a = encode_sequence_to(&dir_a, &frames);
        let bytes_b = encode_sequence_to(&dir_b, &frames);

        // 两次落盘逐帧逐字节一致(确定性序列,RXS-0117)。
        assert_eq!(bytes_a, bytes_b);
        // 落盘字节与直接 encode 一致(push_frame 不引入额外字节)。
        for (i, frame) in frames.iter().enumerate() {
            let direct = encode(frame, ImageFormat::Ppm).unwrap();
            assert_eq!(bytes_a[i], direct);
        }

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 固定输入帧序列(确定性,无随机 / 时间戳):3 帧 4x3 渐变。
    fn build_fixed_frames() -> Vec<ImageBuffer<Rgb>> {
        let mut frames = Vec::new();
        for f in 0..3u32 {
            let mut buf = ImageBuffer::new(4, 3, Rgb::new(0.0, 0.0, 0.0));
            for y in 0..3u32 {
                for x in 0..4u32 {
                    let r = (x as f32) / 3.0;
                    let g = (y as f32) / 2.0;
                    let b = (f as f32) / 2.0;
                    buf.set(x, y, Rgb::new(r, g, b));
                }
            }
            frames.push(buf);
        }
        frames
    }

    /// 把帧序列落盘到目录,返回逐帧落盘字节(从磁盘读回,背书真落盘)。
    fn encode_sequence_to(dir: &Path, frames: &[ImageBuffer<Rgb>]) -> Vec<Vec<u8>> {
        let mut seq = ImageSequence::new(dir);
        let mut out = Vec::new();
        for frame in frames {
            let rec = seq.push_frame(frame, ImageFormat::Ppm).unwrap();
            let path = dir.join(&rec.file_name);
            let on_disk = std::fs::read(&path).unwrap();
            assert_eq!(on_disk.len(), rec.byte_len);
            out.push(on_disk);
        }
        out
    }
}
