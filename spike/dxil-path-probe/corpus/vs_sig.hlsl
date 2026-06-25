// SPIKE(RD-010) B 路图形签名取证语料 — vertex / 富 SV 签名。
// 输入:SV_VertexID(系统值)+ 顶点属性 POSITION/NORMAL/TEXCOORD0;
// 输出:SV_Position(系统值)+ 插值 varying NORMAL/TEXCOORD0/COLOR。
// 承 RXS-0154/0159 的 SV 映射;测 B 链对 SV_VertexID/SV_Position + 用户语义名的保真。
struct VSIn {
    uint vid : SV_VertexID;
    float3 pos : POSITION;
    float3 nrm : NORMAL;
    float2 uv : TEXCOORD0;
};
struct VSOut {
    float4 pos : SV_Position;
    float3 nrm : NORMAL;
    float2 uv : TEXCOORD0;
    float4 col : COLOR;
};
cbuffer Camera : register(b0) {
    float4x4 mvp;
};
VSOut main(VSIn input) {
    VSOut o;
    o.pos = mul(mvp, float4(input.pos, 1.0f));
    o.nrm = input.nrm;
    o.uv = input.uv;
    o.col = float4(float(input.vid & 255) / 255.0f, input.uv, 1.0f);
    return o;
}
