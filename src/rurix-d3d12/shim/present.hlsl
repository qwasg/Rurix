// present.hlsl — shim 私有固定 present pass（RFC-0001 §4.2.2）。
//
// **不是 Rurix shader codegen，不扩张 G2**：这是 rurix-d3d12 shim 的私有固定资产，
// 构建期经 dxc/fxc 编译为嵌入式 DXBC（build.rs；缺 shader compiler 直接构建失败，
// 不做运行期编译或静默 fallback）。读共享 f32 RGB buffer（行主序紧密，分量 0…255，
// 与 RXS-0121 sr_tonemap 输出同义），nearest 放大到窗口，/255 写 R8G8B8A8_UNORM backbuffer。

cbuffer Dims : register(b0)
{
    uint render_w;
    uint render_h;
    uint window_w;
    uint window_h;
};

// 共享 committed buffer：render_w*render_h*3 个 float（CUDA kernel 写入端）。
ByteAddressBuffer rgb : register(t0);

struct VSOut
{
    float4 pos : SV_Position;
};

// 全屏三角形（无顶点缓冲，SV_VertexID 0..2）。
VSOut VSMain(uint vid : SV_VertexID)
{
    VSOut o;
    float2 uv = float2((vid << 1) & 2, vid & 2);
    o.pos = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    return o;
}

float4 PSMain(VSOut i) : SV_Target
{
    uint px = (uint) i.pos.x;
    uint py = (uint) i.pos.y;
    // nearest-neighbor：窗口像素 → render 像素（render 与 window 尺寸分离，RFC-0001 §4.2.2）。
    uint rx = (window_w == render_w) ? px : (px * render_w) / max(window_w, 1u);
    uint ry = (window_h == render_h) ? py : (py * render_h) / max(window_h, 1u);
    rx = min(rx, render_w - 1u);
    ry = min(ry, render_h - 1u);
    uint base = (ry * render_w + rx) * 3u; // float 索引
    float r = asfloat(rgb.Load(base * 4u + 0u));
    float g = asfloat(rgb.Load(base * 4u + 4u));
    float b = asfloat(rgb.Load(base * 4u + 8u));
    return float4(r / 255.0, g / 255.0, b / 255.0, 1.0);
}
