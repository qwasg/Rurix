// SPIKE(RD-010) B 路图形签名取证语料 — pixel / 富 SV 签名 + 多渲染目标。
// 输入:SV_Position(系统值)+ 插值 varying NORMAL/TEXCOORD0 + SV_IsFrontFace(系统值);
// 输出:SV_Target0 + SV_Target1(多渲染目标)。
// 测 B 链对 SV_Position/SV_IsFrontFace 入 + SV_Target MRT 出 + 插值限定符的保真。
struct PSIn {
    float4 pos : SV_Position;
    float3 nrm : NORMAL;
    float2 uv : TEXCOORD0;
    bool front : SV_IsFrontFace;
};
struct PSOut {
    float4 color : SV_Target0;
    float4 normal : SV_Target1;
};
PSOut main(PSIn input) {
    PSOut o;
    float s = input.front ? 1.0f : 0.5f;
    o.color = float4(input.uv * s, 0.25f, 1.0f);
    o.normal = float4(normalize(input.nrm) * 0.5f + 0.5f, 1.0f);
    return o;
}
