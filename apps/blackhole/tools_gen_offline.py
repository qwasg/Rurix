#!/usr/bin/env python3
# apps/blackhole offline.rx 一次性源码生成器(作者期运行,非构建期 codegen;
# 生成的 offline.rx 冻结为源码事实,Rurix 侧无 build.rs 等价物)。
# 必要性:Rurix 无动态字符串,write_ppm(RXS-0199)路径仅接受字符串字面量,
# 144 帧序列 → 144 臂字面量 if-chain。
N = 144  # 与 params.rx OFFLINE_FRAMES 一致

HEADER = '''// blackhole 离线视频帧序列入口(非实时高采样档;v3 新增)。
// 管线与 realtime.rx 逐帧同构(同一 bh_render/bh_bloom_h/bh_bloom_v 三 pass,
// 相机轨道 OFFLINE_DPHI = 2π/144 整圈),差异:
//   · 无 Present 窗口,纯离屏渲染 → 每帧 download → write_ppm 落盘
//     (RXS-0199;路径为字符串字面量 if-chain,Rurix 无动态字符串,链由
//     tools_gen_offline.py 作者期一次性生成后冻结为源码事实);
//   · 超采样 OFFLINE_SSAA = 4(16 spp,v4 画质无限档;实时档 SSAA = 2);
//   · 帧数 OFFLINE_FRAMES = 144(24fps × 6s),帧序列供外部 ffmpeg 合成 MP4。
// 末行打印 OFFLINE_OK frames=<n>;数值行经 CRT putchar(RXS-0195)。

mod params;
mod dmath;
mod render_core;
mod starfield;
mod render;

extern "C" {
    fn putchar(c: i32) -> i32;
}

// present-real cabi 静态库链接系统库接线(RXS-0195 #[link];母本 = realtime.rx:
// 静态库整链时 rxp_* 目标文件引用 D3D12/DXGI/user32 符号,即使本入口不调 Present)
#[link(name = "user32")]
#[link(name = "d3d12")]
#[link(name = "dxgi")]
#[link(name = "d3dcompiler")]
extern "C" {
}

// u32 十进制逐位输出(ruridrop 母本:除数法自最高位,免反转溢出)
fn print_u32(v: u32) {
    let mut div: u32 = 1;
    while v / div >= 10u32 {
        div = div * 10u32;
    }
    while div > 0u32 {
        let d = (v / div) % 10u32;
        unsafe { putchar((48u32 + d) as i32) };
        div = div / 10u32;
    }
}

fn main() -> i32 {
    // ---- 渲染常量 ----
    let rw = params::REND_W;
    let rh = params::REND_H;
    let rw32 = rw as u32;
    let rh32 = rh as u32;
    let gx = (rw + 15) / 16;
    let gy = (rh + 15) / 16;
    let npix = rw * rh;
    let nrgb = npix * 3;
    let inv_w = 1.0 / (rw as f32);
    let inv_h = 1.0 / (rh as f32);
    let aspect = (rw as f32) / (rh as f32);
    let vs = params::VIEW_SCALE;
    let scale_x = vs * aspect;
    let scale_y = 0.0 - vs;

    // ---- 相机常量 ----
    let cam_r = params::CAM_R;
    let th_c = params::CAM_THETA;
    let stc = dmath::rx_sin(th_c);
    let ctc = dmath::rx_cos(th_c);

    let ctx = Context::create();
    let stream = ctx.stream();

    // ---- 缓冲(fout = 呈现域 0…255;hppm = [0,1] 量化前缓冲) ----
    let mut hrgb = ctx.alloc_pinned(nrgb);
    let mut hppm = ctx.alloc_pinned(nrgb);
    let mut fbuf = ctx.alloc(nrgb);
    let mut tbuf = ctx.alloc(nrgb);
    let mut fout = ctx.alloc(nrgb);

    // ---- 离屏帧循环:轨道相机整圈 + 三 pass 管线 + 逐帧落盘 ----
    let mut fi: u32 = 0;
    while fi < params::OFFLINE_FRAMES {
        let ph_c = params::CAM_PH0 + (fi as f32) * params::OFFLINE_DPHI;
        let spc = dmath::rx_sin(ph_c);
        let cpc = dmath::rx_cos(ph_c);
        // e_r(θ 自 +y,φ 在 xz 平面)
        let erx = stc * cpc;
        let ery = ctc;
        let erz = stc * spc;
        let cam_x = cam_r * erx;
        let cam_y = cam_r * ery;
        let cam_z = cam_r * erz;
        // fw = −e_r(指向原点)
        let fwx = 0.0 - erx;
        let fwy = 0.0 - ery;
        let fwz = 0.0 - erz;
        // ri = normalize(cross(fw, (0,1,0))) = normalize(−fwz, 0, fwx)
        let r0x = 0.0 - fwz;
        let r0z = fwx;
        let rl = render_core::len3(r0x, 0.0, r0z);
        let rinv = 1.0 / dmath::rx_max(rl, 0.000001);
        let rix = r0x * rinv;
        let riz = r0z * rinv;
        // up = cross(ri, fw)
        let upx = render_core::cross_x(0.0, riz, fwy, fwz);
        let upy = render_core::cross_y(rix, riz, fwx, fwz);
        let upz = render_core::cross_z(rix, 0.0, fwx, fwy);
        // 横滚 CAM_ROLL:ri/up 绕 fw 旋转(v7 参考图盘带右升 ~12°)
        let cro = dmath::rx_cos(params::CAM_ROLL);
        let sro = dmath::rx_sin(params::CAM_ROLL);
        let rix2 = rix * cro + upx * sro;
        let riy2 = upy * sro;
        let riz2 = riz * cro + upz * sro;
        let upx2 = upx * cro - rix * sro;
        let upy2 = upy * cro;
        let upz2 = upz * cro - riz * sro;
        let time = (fi as f32) * params::FRAME_DT;
        // 三 pass:① RK4 测地线 HDR → fbuf;② 亮通+水平 tent → tbuf;
        // ③ 垂直 tent + 合成 + ACES tone_map → fout(0…255)
        stream.launch(render::bh_render, GridDim(gx, gy), BlockDim(16, 16),
            (fbuf, rw, rh, cam_x, cam_y, cam_z, th_c, ph_c,
             fwx, fwy, fwz, rix2, riy2, riz2, upx2, upy2, upz2,
             inv_w, inv_h, scale_x, scale_y, time, params::OFFLINE_SSAA));
        stream.launch(render::bh_bloom_h, GridDim(gx, gy), BlockDim(16, 16),
            (tbuf, fbuf, rw, rh, params::BLOOM_THRESH));
        stream.launch(render::bh_bloom_v, GridDim(gx, gy), BlockDim(16, 16),
            (fout, fbuf, tbuf, rw, rh, params::BLOOM_STRENGTH));
        stream.sync();
        fout.download(&mut hrgb);
        // 0…255 → [0,1](write_ppm 域,RXS-0116 量化口径)
        let mut pi: usize = 0;
        while pi < nrgb {
            hppm.set(pi, hrgb.get(pi) * 0.00392156862745098);
            pi = pi + 1;
        }
        // ---- 文件名字面量 if-chain(Rurix 无动态字符串) ----
'''

CHAIN = ''.join(
    f'        if fi == {i}u32 {{\n'
    f'            write_ppm("apps/blackhole/frames/f_{i:04d}.ppm", rw32, rh32, &hppm);\n'
    f'        }}\n'
    for i in range(N)
)

FOOTER = '''        print_u32(fi);
        unsafe { putchar(10) };
        fi = fi + 1u32;
    }

    // "OFFLINE_OK frames=<n>\\n"
    unsafe { putchar(79) };
    unsafe { putchar(70) };
    unsafe { putchar(70) };
    unsafe { putchar(76) };
    unsafe { putchar(73) };
    unsafe { putchar(78) };
    unsafe { putchar(69) };
    unsafe { putchar(95) };
    unsafe { putchar(79) };
    unsafe { putchar(75) };
    unsafe { putchar(32) };
    unsafe { putchar(102) };
    unsafe { putchar(114) };
    unsafe { putchar(97) };
    unsafe { putchar(109) };
    unsafe { putchar(101) };
    unsafe { putchar(115) };
    unsafe { putchar(61) };
    print_u32(params::OFFLINE_FRAMES);
    unsafe { putchar(10) };

    ctx.sync();
    0
}
'''

import pathlib
out = pathlib.Path(__file__).parent / "src" / "offline.rx"
out.write_text(HEADER + CHAIN + FOOTER, encoding="utf-8")
print(f"generated {out} ({N} frame arms)")
