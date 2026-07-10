Texture2D<float> src_luminance : register(t0, space0);
RWTexture2D<float> dst_luminance : register(u0, space0);

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID)
{
    float value = src_luminance.Load(int3(dispatch_id.xy, 0));
    dst_luminance[dispatch_id.xy] = value;
}
