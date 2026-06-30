// SPIKE(RD-017) corpus — vertex 阶段:命名 output varying(NORMAL / WORLDPOS / UV)。
// 代表 G-buffer geometry pass 的 VS:把多个**用户命名** varying 传给 fragment。
// 经 dxc -spirv → spirv-cross 回译后,这些 output 语义名退化为通用 TEXCOORD#
// (spirv-cross HLSL 后端无 output 语义旗标、不消费 UserSemantic);RD-017 的修复
// 即在回译 HLSL 边界把 output struct 的 TEXCOORD# 按 location provenance 改回原名。
struct VSOut {
    float4 clip   : SV_Position;   // builtin 系统值(不占 varying location)
    float3 normal : NORMAL;        // location 0 varying
    float3 wpos   : WORLDPOS;      // location 1 varying
    float2 uv     : TEXCOORD0;     // location 2 varying(本就是 TEXCOORD,作对照)
};

VSOut main(float3 inPos : POSITION, float3 inNormal : NORMAL, float2 inUv : TEXCOORD0) {
    VSOut o;
    o.clip   = float4(inPos, 1.0);
    o.normal = inNormal;
    o.wpos   = inPos;
    o.uv     = inUv;
    return o;
}
