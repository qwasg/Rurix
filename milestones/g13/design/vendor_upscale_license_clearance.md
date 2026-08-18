<!-- Assisted-by: Kimi-K3（G13 许可清结+互锁解锁） -->
# G13 M-a 许可前置 — vendor 超分许可清结留痕（owner 法律面）

> **性质**：G13_CONTRACT G-G13-3 / §7 裁决 5「M-a 许可前置条款」的 owner 法律面清结留痕面（G12.3 NRD 许可取证先例同模——评估 ≠ 接入，owner 法律面清结为开工硬前置）。本文件落盘即 M-a 许可前置清结凭据：清结状态 **pending → cleared**（2026-08-18）。
> 清结基准日：2026-08-18；许可文本取证面转引 G12.3 评估报告（milestones/g12/design/nrd_vendor_denoise_evaluation.md §3，2026-08-17 联网实测）与 G13 立项前调研报告（2026-08-18 主会话留痕）。

---

## §1 清结裁决（owner 法律面，2026-08-18）

- **owner 裁决**：owner 于 2026-08-18 主会话明确回复「我接受 DLSS 许可，继续」——即 owner 法律面接受 **NVIDIA RTX SDKs LICENSE**（DLSS/Streamline 面，自定义协议非 OSI；G12.3 评估报告 §3 登记字面），授权 G13.2 vendor 超分接入波开工。
- **清结状态**：M-a 许可前置 **pending → cleared**（本文件落盘时点）；G-G13-3 四条件之③「M-a 许可前置 owner 法律面清结留痕」由此兑现，互锁 validator 事实门③的机器核验面 = 本文件在树且五要素字面齐备（Streamline / NGX / FSR / owner / 清结）。

## §2 三许可面逐项清结登记

| 许可面 | 许可形态 | 取证面 | owner 法律面清结 |
|---|---|---|---|
| DLSS SR（Streamline SDK 2.10.3 开源框架 + NGX 签名专有 DLL，Vulkan interop 臂） | 自定义 NVIDIA RTX SDKs LICENSE（非 OSI，GitHub API NOASSERTION） | G12.3 评估报告 §3（2026-08-17 联网实测同族许可字面：协议许可文体「governs the use of the NVIDIA RTX software development kits, including the DLSS SDK, NGX SDK, RTXGI SDK, RTXDI SDK and/or NRD SDK」） | **cleared**——owner 2026-08-18 明确接受（§1） |
| FSR 3.1.5（同一 UpscaleBackend 冻结面同接口档） | MIT（OSI 批准） | G13 立项前调研报告技术事实面（FSR 3.1.5，MIT） | **cleared**——MIT 许可零障碍确认（redistribution/集成无法律障碍；著作权声明保留义务沿 MIT 字面执行） |
| NRD（RD040-nrd 面） | 自定义 NVIDIA RTX SDKs LICENSE（非 OSI） | G12.3 评估报告 §3 登记（自定义 NVIDIA RTX SDKs LICENSE 非 OSI） | **cleared**——与 DLSS 同批清结（同一 NVIDIA RTX SDKs LICENSE 协议面，owner 同批接受）；NRD 接入本身仍维持评估不接线（RD-040 backfill 三条件另判，字面 0-byte） |

## §3 清结边界（诚实登记）

- 本清结 = owner 法律面接受许可条款的留痕，不构成法律意见；使用/再分发/归属条款逐条义务（NVIDIA RTX SDKs LICENSE 协议文体 + MIT 保留声明字面）在 G13.2 接入面按许可字面执行。
- vendor SDK 二进制（Streamline / NGX / FSR）**不入 git**——外部缓存 + 许可/digest 登记形态承载（G13 立项裁决 10 / RFC-0027 许可边界字面 0-byte）。
- 不得以 FSR MIT 面宽松冒充 DLSS NGX 面清结——本清结对 DLSS/Streamline/NGX 面与 FSR 面逐项独立登记（§2）；DLSS 面清结凭据 = owner 2026-08-18 明确接受字面（§1），非 FSR 面宽松外推。
- UpscaleBackend / temporal 底座 0-byte 维持（G13 立项裁决 6）；本清结零 src/spec/conformance 改动、零 vendor SDK vendoring/接线、零 RFC 消费。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-18 | 初版：owner 法律面清结留痕——DLSS/Streamline（Streamline SDK 2.10.3 + NGX 签名 DLL）NVIDIA RTX SDKs LICENSE owner 明确接受 + FSR 3.1.5 MIT 零障碍确认 + NRD 与 DLSS 同批清结登记；M-a 许可前置 pending → cleared |
