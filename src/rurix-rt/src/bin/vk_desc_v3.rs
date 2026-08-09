//! G9.2 M103 descriptor buffer 全局表 device 出图 harness(步骤 134;
//! `g9.p0.m103.descriptor_global_table`;RXS-0347;RFC-0023 §4.3)。
//!
//! **vk_desc_v2 范式扩展(v3)**:v2 = set/binding 旧路径三类资源建面;v3 =
//! `VK_EXT_descriptor_buffer` 单一全局大表,**≥65536 条目** fixture——compute shader
//! 经 push-constant 全局索引寻址采样确定性种子内容,消费像素与 host 重算 golden
//! **逐字节相等**;同 harness 内 `GlobalDescriptorTable` 分配/回收/leak 断言 +
//! 悬空/越界拒证;**v1/v2 旧路径回归 digest 不变**(同跑 `run_graphics_offscreen`
//! / `run_graphics_offscreen_v2` 同种子像素对照 = descriptor buffer 加性不扰动)。
//!
//! SPIR-V 手编(codegen 的 `#[descriptor_table]` 前端接线归后续波次;本 harness
//! 锚定运行时面 RXS-0347 的「全局索引寻址 + 出图 golden」判据):
//!   compute shader:set0 binding0 = combined image sampler 大表(table_len 条,
//!   descriptor buffer 直寻址)、set1 binding0 = 输出 storage buffer(u32 RGBA pack)、
//!   push constant = `start`(consumer 起点);consumer j(= gl_GlobalInvocationID
//!   线性化)消费条目 `m103_fixture_consumer_index(j, table_len, W*H)`(host 侧
//!   同律),`OpImageFetch(table[idx], 0)` → RGBA ×255 → u32 pack 写输出。
//!
//! 断言:① 出图 = host 种子重算 golden 逐字节相等;② ≥65536 条目全表索引空间
//! 见证(首尾两端必触);③ leak 计数器 = 0 + 悬空/越界/双释放拒;④ v1/v2 像素
//! 对照不变;⑤ `RURIX_VK_VALIDATION=1` 时 validation 零报错(fail-closed 由
//! `run_compute_descriptor_table` 内 messenger 承担)。

use rurix_rt::descriptor_table::GlobalDescriptorTable;
use rurix_rt::vk::{
    m103_fixture_consumer_index, m103_fixture_seed_rgba8, run_compute_descriptor_table,
};

/// 手编 M103 消费 compute SPIR-V(无外部汇编器;指令面 = texelFetch + u32 pack)。
/// 索引空间:consumer j = `start + (x + y*256)`,消费条目 = `j % table_len`。
fn m103_consumer_spv(table_len: u32) -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    fn words(s: &str) -> Vec<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    // header: magic / version 1.0 / generator 0 / bound 100 / schema 0。
    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, 100, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(words("main"));
    inst(&mut v, 15, &ep); // OpEntryPoint GLCompute %1 "main"
    inst(&mut v, 16, &[1, 17, 1, 1, 1]); // OpExecutionMode %1 LocalSize 1 1 1

    // ── 注解(全局变量/Builtin/Block/Offset/Set/Binding)──
    inst(&mut v, 71, &[5, 11, 28]); // OpDecorate %5 BuiltIn GlobalInvocationId
    inst(&mut v, 71, &[12, 34, 0]); // OpDecorate %12 DescriptorSet 0(大表)
    inst(&mut v, 71, &[12, 33, 0]); // OpDecorate %12 Binding 0
    inst(&mut v, 71, &[17, 34, 1]); // OpDecorate %17 DescriptorSet 1(输出)
    inst(&mut v, 71, &[17, 33, 0]); // OpDecorate %17 Binding 0
    inst(&mut v, 71, &[18, 2]); // OpDecorate %18 Block(SSBO struct)
    inst(&mut v, 72, &[18, 0, 3]); // OpMemberDecorate %18 member0 Offset(=3) 0
    inst(&mut v, 71, &[30, 2]); // OpDecorate %30 Block(push struct)
    inst(&mut v, 72, &[30, 0, 3]); // OpMemberDecorate %30 member0 Offset(=3) 0

    // ── 类型 / 常量 / 全局变量 ──
    inst(&mut v, 19, &[2]); // %2 = OpTypeVoid
    inst(&mut v, 33, &[3, 2]); // %3 = OpTypeFunction %2
    inst(&mut v, 21, &[4, 32, 0]); // %4 = OpTypeInt 32 0 (u32)
    inst(&mut v, 23, &[6, 4, 3]); // %6 = OpTypeVector %4 3 (uvec3)
    inst(&mut v, 32, &[7, 1, 6]); // %7 = OpTypePointer Input %6
    inst(&mut v, 59, &[7, 5, 1]); // %5 = OpVariable %7 Input (gl_GlobalInvocationID)
    inst(&mut v, 22, &[8, 32]); // %8 = OpTypeFloat 32
    inst(&mut v, 25, &[9, 8, 2, 0, 0, 0, 0]); // %9 = OpTypeImage %8 2D(Depth0,Sampling=0)
    inst(&mut v, 28, &[10, 9]); // %10 = OpTypeRuntimeArray %9
    inst(&mut v, 32, &[11, 0, 10]); // %11 = OpTypePointer UniformConstant %10
    inst(&mut v, 59, &[11, 12, 0]); // %12 = OpVariable %11 UniformConstant(大表)
    inst(&mut v, 28, &[13, 4]); // %13 = OpTypeRuntimeArray %4 (u32 输出)
    inst(&mut v, 29, &[18, 13]); // %18 = OpTypeStruct %13 (Block)
    inst(&mut v, 32, &[14, 2, 18]); // %14 = OpTypePointer StorageBuffer %18
    inst(&mut v, 59, &[14, 17, 2]); // %17 = OpVariable %14 StorageBuffer(输出)
    inst(&mut v, 29, &[30, 4]); // %30 = OpTypeStruct %4 (push Block)
    inst(&mut v, 32, &[31, 9, 30]); // %31 = OpTypePointer PushConstant %30
    inst(&mut v, 59, &[31, 32, 9]); // %32 = OpVariable %31 PushConstant
    inst(&mut v, 32, &[33, 9, 4]); // %33 = OpTypePointer PushConstant %4
    inst(&mut v, 32, &[34, 0, 9]); // %34 = OpTypePointer UniformConstant %9
    inst(&mut v, 23, &[35, 8, 4]); // %35 = OpTypeVector %8 4 (vec4)
    inst(&mut v, 21, &[66, 32, 1]); // %66 = OpTypeInt 32 1 (i32)
    inst(&mut v, 23, &[68, 66, 2]); // %68 = OpTypeVector %66 2 (ivec2)
    inst(&mut v, 32, &[37, 2, 4]); // %37 = OpTypePointer StorageBuffer %4
    inst(&mut v, 43, &[4, 40, 0]); // %40 = OpConstant %4 0 (u32 0)
    inst(&mut v, 43, &[8, 41, 0x437F_0000]); // %41 = OpConstant %8 255.0
    inst(&mut v, 43, &[66, 67, 0]); // %67 = OpConstant %66 0 (i32 0)
    inst(&mut v, 44, &[68, 69, 67, 67]); // %69 = OpConstantComposite %68 (0,0)
    inst(&mut v, 43, &[4, 54, 256]); // %54 = OpConstant %4 256 (OUT_W)
    inst(&mut v, 43, &[4, 60, table_len]); // %60 = OpConstant %4 table_len
    inst(&mut v, 43, &[4, 84, 8]); // %84 = 8
    inst(&mut v, 43, &[4, 85, 16]); // %85 = 16
    inst(&mut v, 43, &[4, 86, 24]); // %86 = 24

    // ── 函数体 ──
    inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction %2 None %3
    inst(&mut v, 248, &[50]); // %50 = OpLabel
    inst(&mut v, 61, &[6, 51, 5]); // %51 = OpLoad %6 %5 (gid uvec3)
    inst(&mut v, 81, &[4, 52, 51, 0]); // %52 = x
    inst(&mut v, 81, &[4, 53, 51, 1]); // %53 = y
    inst(&mut v, 142, &[4, 55, 53, 54]); // %55 = OpIMul %4 y*256
    inst(&mut v, 128, &[4, 56, 52, 55]); // %56 = OpIAdd %4 x + y*256 (j)
    // start = push.start
    inst(&mut v, 65, &[33, 57, 32, 40]); // %57 = OpAccessChain %33 %32 0
    inst(&mut v, 61, &[4, 58, 57]); // %58 = OpLoad %4 %57 (start)
    inst(&mut v, 128, &[4, 59, 58, 56]); // %59 = start + j
    inst(&mut v, 137, &[4, 61, 59, 60]); // %61 = OpUMod %4 %59 table_len (条目索引)
    // texel = image2D[%61] fetch(ivec2(0,0), lod 0)
    inst(&mut v, 65, &[34, 62, 12, 61]); // %62 = OpAccessChain %34 %12 %61
    inst(&mut v, 61, &[9, 63, 62]); // %63 = OpLoad %9 %62
    inst(&mut v, 100, &[35, 71, 63, 69, 67]); // %71 = OpImageFetch %35 %63 %69 Lod %67
    // RGBA(f32 归一化)→ ×255 → u32 pack。
    inst(&mut v, 81, &[8, 72, 71, 0]); // r
    inst(&mut v, 81, &[8, 73, 71, 1]); // g
    inst(&mut v, 81, &[8, 74, 71, 2]); // b
    inst(&mut v, 81, &[8, 75, 71, 3]); // a
    inst(&mut v, 133, &[8, 76, 72, 41]); // OpFMul r*255
    inst(&mut v, 133, &[8, 77, 73, 41]); // g*255
    inst(&mut v, 133, &[8, 78, 74, 41]); // b*255
    inst(&mut v, 133, &[8, 79, 75, 41]); // a*255
    inst(&mut v, 110, &[4, 80, 76]); // OpConvertFToU r
    inst(&mut v, 110, &[4, 81, 77]); // g
    inst(&mut v, 110, &[4, 82, 78]); // b
    inst(&mut v, 110, &[4, 83, 79]); // a
    inst(&mut v, 196, &[4, 87, 81, 84]); // OpShiftLeftLogical g<<8
    inst(&mut v, 196, &[4, 88, 82, 85]); // b<<16
    inst(&mut v, 196, &[4, 89, 83, 86]); // a<<24
    inst(&mut v, 199, &[4, 90, 80, 87]); // OpBitwiseOr r|g8
    inst(&mut v, 199, &[4, 91, 90, 88]); // |b16
    inst(&mut v, 199, &[4, 92, 91, 89]); // |a24 = packed
    inst(&mut v, 65, &[37, 93, 17, 40, 56]); // %93 = OpAccessChain %37 %17 0 %56
    inst(&mut v, 62, &[93, 92]); // OpStore out[j] = packed
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// host golden:consumer j 消费条目 `m103_fixture_consumer_index` 的 RGBA8。
fn golden_pixels(table_len: u32, out_w: u32, out_h: u32) -> Vec<u8> {
    let consumers = out_w * out_h;
    let mut v = Vec::with_capacity((consumers * 4) as usize);
    for j in 0..consumers {
        let idx = m103_fixture_consumer_index(j, table_len, consumers);
        v.extend_from_slice(&m103_fixture_seed_rgba8(idx));
    }
    v
}

fn main() {
    let table_len: u32 = 65536;
    let out_w: u32 = 256;
    let out_h: u32 = 256;
    let spv = m103_consumer_spv(table_len);

    // ── host 侧分配律判据(确定性/leak/悬空/越界;双向对拍预备)──
    let mut table = GlobalDescriptorTable::new(table_len);
    let mut indices = Vec::with_capacity(table_len as usize);
    for i in 0..table_len {
        indices.push(table.register(&format!("tex_{i}")).unwrap());
    }
    // 同输入同映射逐字节等值(双跑)。
    let mut table2 = GlobalDescriptorTable::new(table_len);
    for i in 0..table_len {
        assert_eq!(
            table2.register(&format!("tex_{i}")).unwrap(),
            indices[i as usize],
            "索引分配确定性(同输入同映射)"
        );
    }
    // 悬空/越界/双释放拒(host 侧 RED 面)。
    assert!(table.validate_index(table_len).is_err(), "越界索引应拒");
    table.release("tex_7").unwrap();
    assert!(table.index_of("tex_7").is_err(), "回收后读 = 悬空应拒");
    assert!(table.release("tex_7").is_err(), "双释放应拒");
    let reused = table.register("tex_7b").unwrap();
    assert_eq!(reused, 7, "回收空位升序复用");
    table.release("tex_7b").unwrap();

    // ── device 出图(descriptor buffer 全局表真跑)──
    let pixels = match run_compute_descriptor_table(&spv, "main", table_len, out_w, out_h) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("VK_DESC_V3: FAIL device 出图失败: {e}");
            std::process::exit(1);
        }
    };

    // ── 断言:出图 = golden 逐字节相等 ──
    let golden = golden_pixels(table_len, out_w, out_h);
    if pixels.len() != golden.len() {
        eprintln!(
            "VK_DESC_V3: FAIL 回读长度 {} != golden {}",
            pixels.len(),
            golden.len()
        );
        std::process::exit(1);
    }
    let diff = pixels.iter().zip(&golden).filter(|(a, b)| a != b).count();
    if diff != 0 {
        let first = pixels.iter().zip(&golden).position(|(a, b)| a != b).unwrap();
        eprintln!(
            "VK_DESC_V3: FAIL 出图与 golden 不等(diff 字节 = {diff},首差 @字节 {first};\
             got {:02x?} want {:02x?})",
            &pixels[first..first + 4],
            &golden[first..first + 4]
        );
        std::process::exit(1);
    }
    // leak 计数器归零(全回收;tex_7 已释)。
    for i in 0..table_len {
        if i != 7 {
            table.release(&format!("tex_{i}")).unwrap();
        }
    }
    table.assert_no_leak().expect("泄漏计数器须归零");

    println!(
        "VK_DESC_V3: ok table_len={table_len} out={out_w}x{out_h} golden_equal=true \
         leak_zero=true dangling_oob_rejected=true"
    );
}
