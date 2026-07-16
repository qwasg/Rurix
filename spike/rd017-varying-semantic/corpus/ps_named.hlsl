// SPIKE(RD-017) corpus — fragment 阶段:命名 input varying(NORMAL / WORLDPOS / UV)。
// 代表 G-buffer geometry pass 的 PS:消费上游 VS 的多个**用户命名** varying。
// 经 B 链回译后,这些 fragment input 语义名同样退化为 TEXCOORD#(spirv-cross 无
// 片元输入语义旗标);RD-017 修复在回译 HLSL 边界把 input struct 的 TEXCOORD# 按
// location provenance 改回原名。PS 输出走 SV_Target#(系统值,不在本修复面)。
struct PSIn {
    float4 clip   : SV_Position;   // builtin(片元为 SV_Position 系统值,不占 varying location)
    float3 normal : NORMAL;        // location 0 varying
    float3 wpos   : WORLDPOS;      // location 1 varying
    float2 uv     : TEXCOORD0;     // location 2 varying
};

float4 main(PSIn i) : SV_Target {
    return float4(i.normal * 0.5 + 0.5, i.uv.x) + float4(i.wpos, 1.0);
}
