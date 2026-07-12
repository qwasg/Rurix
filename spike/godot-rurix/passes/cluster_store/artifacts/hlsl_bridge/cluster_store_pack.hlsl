// GRX-014 segment S2: math-equivalent HLSL cluster_store kernel (the complete
// compute store segment of ClusterBuilderRD::bake_cluster(), "Pack 3D Cluster
// Elements"; single kernel, no mode/variant switches, so no subset cut).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout). See PASS_CONTRACT.md sec 5.3 for the route
// rationale (cluster_store is an all raw-buffer SSBO pass; the workaround is
// used because the Rurix DXIL lowering has no u32 buffer views, no integer
// bit-operation lowering, and no findLSB/findMSB intrinsic). It does NOT
// imply real_gpu_pass=true.
//
// Math target: Godot's
// servers/rendering/renderer_rd/shaders/cluster_store.glsl (main, L46-119),
// ported 1:1:
//   * one thread per cluster; [numthreads(8, 8, 1)] mirrors local_size 8x8x1;
//     dispatch (ceil(cluster_screen_size.x / 8), ceil(cluster_screen_size.y
//     / 8), 1)
//   * outer while scans usage words; inner while walks set bits via
//     firstbitlow (glsl findLSB) and clears them with bits &= ~(1 << bit)
//   * z-range decode: from_z = firstbitlow(z_range), to_z =
//     firstbithigh(z_range) + 1 (glsl findMSB); touches_near -> from_z = 0;
//     touches_far -> to_z = 32; z_range == 0 guard skips the element
//   * per-slice packed write: minmax == 0 -> 0xFFFF init; elem_min =
//     min(orig, minmax & 0xFFFF); elem_max = max(orig + 1, minmax >> 16)
//     ("always store plus one, so zero means range is empty"); store
//     elem_min | (elem_max << 16)
//   * existence bitmap: dst[dst_offset + (orig >> 5)] |= 1 << (orig & 0x1F)
//   * no atomics: every write lands in the owning cluster's own
//     (cluster, type) blocks, so threads never contend (mirrors the native
//     shader)
//   * the destination buffer is assumed zero-cleared (the native
//     bake_cluster buffer_clear), exactly like the glsl
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): the bake_cluster
// rasterization segment (cluster_render.glsl proxy-mesh draw) that produces
// the cluster_render input is NOT replaced; the native buffer clears and the
// render_element_count == 0 early-out stay native.
//
// Binding surface (matches artifacts/cluster_store_descriptor_layout.json):
//   t0 space0 : StructuredBuffer<uint>               cluster_render (set0
//               binding1 of cluster_store.glsl; raster-segment output: per
//               cluster, max_render_element_count_div_32 usage-bitmap words
//               then one z_range word per element)
//   t1 space0 : StructuredBuffer<RenderElementData>  render_elements (set0
//               binding3; 80-byte stride; this kernel reads only the four
//               leading u32 fields)
//   u0 space0 : RWStructuredBuffer<uint>             cluster_store (set0
//               binding2; per (cluster,type) block = [existence bitmap
//               max_cluster_element_count_div_32 words][32 Z-slice
//               (min u16 | (max+1) u16) words])
//   b0 space0 : 32-byte constants matching Godot's
//               ClusterBuilderSharedDataRD::ClusterStore::PushConstant
//               exactly (8 dwords, all u32; see resource_mapping.md).

struct RenderElementData {
    uint element_type;        // glsl `type` (0-3; ELEMENT_TYPE_MAX = 4)
    uint touches_near;        // glsl bool (4-byte)
    uint touches_far;         // glsl bool (4-byte)
    uint original_index;
    float4 transform_inv_c0;  // mat3x4 column 0 (raster segment only; carried)
    float4 transform_inv_c1;  // mat3x4 column 1 (raster segment only; carried)
    float4 transform_inv_c2;  // mat3x4 column 2 (raster segment only; carried)
    float3 scale;             // raster segment only; carried
    uint pad;                 // C++ has_wide_spot_angle; carried
};

StructuredBuffer<uint> cluster_render : register(t0, space0);
StructuredBuffer<RenderElementData> render_elements : register(t1, space0);
RWStructuredBuffer<uint> cluster_store : register(u0, space0);

cbuffer ClusterStoreParams : register(b0, space0) {
    // Mirrors ClusterStore::PushConstant / cluster_store.glsl Params.
    uint cluster_render_data_size;         // dword 0 (words per cluster, src)
    uint max_render_element_count_div_32;  // dword 1
    uint2 cluster_screen_size;             // dwords 2-3
    uint render_element_count_div_32;      // dword 4
    uint max_cluster_element_count_div_32; // dword 5
    uint pad1;                             // dword 6
    uint pad2;                             // dword 7
};

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID) {
    uint2 pos = dispatch_id.xy;
    if (pos.x >= cluster_screen_size.x || pos.y >= cluster_screen_size.y) {
        return; // cluster_store.glsl L47-50 bounds check
    }

    // Base offset for this cluster (cluster_store.glsl L55-56).
    uint base_offset = pos.x + cluster_screen_size.x * pos.y;
    uint src_offset = base_offset * cluster_render_data_size;

    uint render_element_offset = 0u;

    // Check all render_elements and see which one was written to
    // (cluster_store.glsl L61-118).
    while (render_element_offset < render_element_count_div_32) {
        uint bits = cluster_render[src_offset + render_element_offset];
        while (bits != 0u) {
            // If bits exist, check the render_element.
            uint index_bit = firstbitlow(bits); // glsl findLSB
            uint index = render_element_offset * 32u + index_bit;
            uint element_type = render_elements[index].element_type;

            uint z_range_offset = src_offset + max_render_element_count_div_32 + index;
            uint z_range = cluster_render[z_range_offset];

            // If object was written, z was written, but check just in case
            // (cluster_store.glsl L73: "should always be > 0").
            if (z_range != 0u) {
                uint from_z = firstbitlow(z_range);       // glsl findLSB
                uint to_z = firstbithigh(z_range) + 1u;   // glsl findMSB + 1

                if (render_elements[index].touches_near != 0u) {
                    from_z = 0u;
                }
                if (render_elements[index].touches_far != 0u) {
                    to_z = 32u;
                }

                // Find cluster offset in the buffer used for indexing in the
                // renderer (cluster_store.glsl L87).
                uint dst_offset =
                    (base_offset +
                     element_type * (cluster_screen_size.x * cluster_screen_size.y)) *
                    (max_cluster_element_count_div_32 + 32u);

                uint orig_index = render_elements[index].original_index;
                // Store this index in the Z slices by setting the relevant
                // bits (cluster_store.glsl L91-105).
                for (uint i = from_z; i < to_z; i++) {
                    uint slice_ofs = dst_offset + max_cluster_element_count_div_32 + i;

                    uint minmax = cluster_store[slice_ofs];

                    if (minmax == 0u) {
                        minmax = 0xFFFFu; // min 0xFFFF, max 0
                    }

                    uint elem_min = min(orig_index, minmax & 0xFFFFu);
                    // Always store plus one, so zero means range is empty
                    // when not written to.
                    uint elem_max = max(orig_index + 1u, minmax >> 16);

                    minmax = elem_min | (elem_max << 16);
                    cluster_store[slice_ofs] = minmax;
                }

                uint store_word = orig_index >> 5;
                uint store_bit = orig_index & 0x1Fu;

                // Store the actual render_element index at the end, so the
                // rendering code can reference it (cluster_store.glsl L111).
                cluster_store[dst_offset + store_word] |= 1u << store_bit;
            }

            bits &= ~(1u << index_bit); // clear the bit to continue iterating
        }

        render_element_offset++;
    }
}
