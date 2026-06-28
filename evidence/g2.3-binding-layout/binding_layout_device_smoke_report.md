# G2.3 绑定布局推导产物 device 真跑 / G-G2-3 收口取证(E2b-4)

> 配套机器证据:`evidence/g2.3-binding-layout/binding_layout_device_smoke_20260628.json`
> 复跑脚本:`ci/dxil_binding_device_smoke.py`(PR Smoke 步骤 47 接线)
> 状态:**measured_local**(真实 D3D12 hardware + signed dxc pin)。G-G2-3 验收门**签署**与
> 真实 CI run URL 回填归 **owner**(AI 不代签、不伪造 run URL)。

## 1. 目的

E2b-3 已用 host 侧 golden(`tests/dxil/binding/fs_tex_samp.binding-golden`)把绑定布局推导产物
(SPIR-V 资源绑定装饰 + RTS0 容器)的确定性 SHA-256 定型为 owner-blessed 回归锚。E2b-4 补齐
该 golden 单测**结构上无法证明**的另一半:推导出的 **RTS0 root signature 在真实 D3D12 device
上是否被接受**,以及绑定的 `Texture2D`/`Sampler` 是否经该 root signature 在硬件上真正出图。

## 2. signed pin 纪律(owner 要求)

- 签名 pin = `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`(含 `dxc.exe` + `dxv.exe` +
  `dxil.dll`,DXC 1.9.2602.24)。
- PATH 上的 Vulkan SDK `dxc`(无 `dxv.exe` / `dxil.dll`)**不**作为 signed 依据;
  `locate_signed_dxc_dir` 强制三件齐备方认定签名 pin。

## 3. 端到端链路(measured)

1. `cargo run -p rurixc --features dxil-backend --example emit_binding_rts0` 经**公开**
   `binding_layout::infer_root_signature` + `serialize_rts0` 落盘生产可达资源子集
   {`Texture2D<f32> tex`(SRV), `Sampler samp`} 的 RTS0 容器(148 字节)。
2. **绑定 device 输入到已 bless 的 golden**:落盘 RTS0 的 SHA-256 =
   `409b6a1e64888136889ad1602a2b0fda10ea7bf00ff3da3aabe2428fecc2c0a2`,与 E2b-3 已定型的
   golden 基线 `rts0.bytes.sha256` **逐字节一致**(脚本断言;不一致即红)。
3. signed dxc 编译 textured VS/PS(`Texture2D g_tex : register(t0)` /
   `SamplerState g_samp : register(s0)`)→ `dxv.exe` 验签接受。
4. 真实 D3D12 hardware(`NVIDIA GeForce RTX 4070 Ti`):
   - **`CreateRootSignature(rurix RTS0 字节)` → accept**(device 直接解析 Rurix 序列化的
     RTS0 容器,非经 `D3D12SerializeRootSignature` 重建);
   - 以该 root signature 建 textured PSO,经推导出的 **SRV(t0)/ Sampler(s0)descriptor
     table** 绑定 1×1 已知纹素 `(64,127,255,255)` 的纹理 + point sampler,离屏 draw →
     readback 中心像素 = `64,127,255,255`(采样路径在硬件上经 Rurix root signature 真正生效)。

## 4. device 级红路径(反 YAML-only)

- **red**:篡改 RTS0 容器 fourcc(`DXBC` 首字节翻转)→ `CreateRootSignature` **reject** —
  证明上面的 accept 是 device 对 RTS0 容器的**真实解析**,而非对任意字节的 no-op 放行。
- **green**:未篡改的 148 字节 RTS0 → accept + textured draw 像素命中。
- harness 同次运行内打印 `rurix_rts0=accept tamper_rts0=reject`,二者同时成立方判 PASS。

## 5. 不在本步范围(owner / 后续)

- 🔒 具体 register/space/binding 物理布局与 RTS0 字节布局**不**因本次 device 接受而冻结为
  stable 语言/ABI(RFC-0005 §4.5;RXS-0162 先例);device accept 仅证当前实现确定产物
  device 可消费,非 stable 承诺。
- G-G2-3 验收门**签署**、milestone 状态翻转、真实 CI run URL 回填:归 owner。
- 本步未改 `error_codes.json` / `deferred.json` / `spike_gating.json` / spec 语义正文 /
  message-key / binding emit 逻辑 / golden digest baseline。
