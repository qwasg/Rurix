#!/usr/bin/env python3
"""G9.1 治理波 measured baseline 探针 — RTX 4070 Ti VRAM / AS 构建耗时实测。

通过 ctypes 直连 vulkan-1.dll（零新 Rust/C++ 代码、零 src/spec/conformance 改动）：
  1. 枚举物理设备并锁定 NVIDIA GPU，读取 device local heap（VRAM 容量基线）；
  2. 实测一次固定规模三角形网格（256x256 grid = 130050 tris）的 BLAS 构建：
     host 墙钟包住 vkQueueSubmit + vkQueueWaitIdle（构建为同步等待，墙钟即构建延迟上限），
     1 次 warmup + 5 次 trial 取中位数；同时落 AS storage/scratch 字节数（VRAM 占用基线锚）；
  3. capability snapshot：G9 阻塞性前置扩展在位性（DGC / descriptor_buffer /
     mesh_shader / SER / CLAS / ray_query / acceleration_structure）逐布尔落证据；
  4. VkPhysicalDeviceAccelerationStructurePropertiesKHR 关键上限落证据。

诚实边界：build 耗时为 host 墙钟含 submit 开销（偏保守，不偏小）；不以任何
estimated/理论值充数；任一环节失败即非零退出（fail-closed），不写 SKIP 证据。
产物：evidence/g9_vram_as_baseline_<UTC>.json（evidence_level=measured_local）。
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import statistics
import subprocess
import sys
import time
from ctypes import (
    POINTER,
    Structure,
    Union,
    byref,
    c_char,
    c_char_p,
    c_float,
    c_uint8,
    c_uint32,
    c_uint64,
    c_void_p,
)
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# ---- Vulkan 常量 ------------------------------------------------------------
VK_SUCCESS = 0
ST_APPLICATION_INFO = 0
ST_INSTANCE_CREATE_INFO = 1
ST_DEVICE_QUEUE_CREATE_INFO = 2
ST_DEVICE_CREATE_INFO = 3
ST_SUBMIT_INFO = 4
ST_MEMORY_ALLOCATE_INFO = 5
ST_BUFFER_CREATE_INFO = 12
ST_COMMAND_POOL_CREATE_INFO = 39
ST_COMMAND_BUFFER_ALLOCATE_INFO = 40
ST_COMMAND_BUFFER_BEGIN_INFO = 42
ST_MEMORY_ALLOCATE_FLAGS_INFO = 1000060000
ST_PHYSICAL_DEVICE_FEATURES_2 = 1000059000
ST_PHYSICAL_DEVICE_PROPERTIES_2 = 1000059001
ST_BUFFER_DEVICE_ADDRESS_INFO = 1000244001
ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES = 1000257000
# AS 相关 sType 权威值（vulkan_core.h VK_HEADER_VERSION 290 实测核对）：
#   BUILD_GEOMETRY_INFO=1000150000 / TRIANGLES_DATA=1000150005 / GEOMETRY=1000150006
#   PHYS_DEV_AS_FEATURES=1000150013 / PHYS_DEV_AS_PROPERTIES=1000150014
#   AS_CREATE_INFO=1000150017 / AS_BUILD_SIZES_INFO=1000150020
# （错位史：特性链曾误填 1000150015=RAY_TRACING_PIPELINE_CREATE_INFO，
#   驱动静默跳过 → accelerationStructure 特性未启用 → build 时 DEVICE_LOST）
ST_AS_BUILD_GEOMETRY_INFO_KHR = 1000150000
ST_AS_GEOMETRY_TRIANGLES_DATA_KHR = 1000150005
ST_AS_GEOMETRY_KHR = 1000150006
ST_PHYSICAL_DEVICE_AS_FEATURES_KHR = 1000150013
ST_PHYSICAL_DEVICE_AS_PROPERTIES_KHR = 1000150014
ST_AS_CREATE_INFO_KHR = 1000150017
ST_AS_BUILD_SIZES_INFO_KHR = 1000150020

VK_API_VERSION_1_2 = (1 << 22) | (2 << 12)
VK_QUEUE_GRAPHICS_BIT = 0x1
VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT = 0x1
VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT = 0x2
VK_MEMORY_PROPERTY_HOST_COHERENT_BIT = 0x4
VK_MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT = 0x1
VK_BUFFER_USAGE_STORAGE_BUFFER_BIT = 0x20
VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT = 0x00020000
VK_BUFFER_USAGE_AS_BUILD_INPUT_READ_ONLY_BIT_KHR = 0x00080000
VK_BUFFER_USAGE_AS_STORAGE_BIT_KHR = 0x00100000
VK_FORMAT_R32G32B32_SFLOAT = 106
VK_INDEX_TYPE_UINT32 = 1
VK_INDEX_TYPE_NONE_KHR = 1000165000
VK_GEOMETRY_TYPE_TRIANGLES_KHR = 0
VK_GEOMETRY_OPAQUE_BIT_KHR = 0x1
VK_BUILD_AS_MODE_BUILD_KHR = 0
VK_BUILD_AS_PREFER_FAST_TRACE_BIT_KHR = 0x4
VK_AS_TYPE_BOTTOM_LEVEL_KHR = 0
VK_AS_BUILD_TYPE_DEVICE_KHR = 0
VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU = 2

NEEDED_DEVICE_EXTS = [
    b"VK_KHR_acceleration_structure",
    b"VK_KHR_deferred_host_operations",
    b"VK_KHR_buffer_device_address",
]
SNAPSHOT_EXTS = [
    "VK_KHR_acceleration_structure",
    "VK_KHR_ray_query",
    "VK_EXT_mesh_shader",
    "VK_EXT_device_generated_commands",
    "VK_EXT_descriptor_buffer",
    "VK_NV_cluster_acceleration_structure",
    "VK_EXT_ray_tracing_invocation_reorder",
    "VK_NV_displacement_micromap",
    "VK_EXT_opacity_micromap",
]

GRID_N = int(os.environ.get("G9_PROBE_GRID_N", "256"))  # 256x256 quads → 2*N*N 三角；顶点 (N+1)^2
BUILD_FLAGS = int(os.environ.get("G9_PROBE_BUILD_FLAGS", str(VK_BUILD_AS_PREFER_FAST_TRACE_BIT_KHR)))
GEO_FLAGS = int(os.environ.get("G9_PROBE_GEO_FLAGS", str(VK_GEOMETRY_OPAQUE_BIT_KHR)))
TRIALS = 5


# ---- 结构体 ------------------------------------------------------------------
class VkApplicationInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("pApplicationName", c_char_p),
        ("applicationVersion", c_uint32), ("pEngineName", c_char_p),
        ("engineVersion", c_uint32), ("apiVersion", c_uint32),
    ]


class VkInstanceCreateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32),
        ("pApplicationInfo", c_void_p), ("enabledLayerCount", c_uint32),
        ("ppEnabledLayerNames", c_void_p), ("enabledExtensionCount", c_uint32),
        ("ppEnabledExtensionNames", c_void_p),
    ]


class VkPhysicalDeviceProperties(Structure):
    _fields_ = [
        ("apiVersion", c_uint32), ("driverVersion", c_uint32), ("vendorID", c_uint32),
        ("deviceID", c_uint32), ("deviceType", c_uint32), ("deviceName", c_char * 256),
        ("pipelineCacheUUID", c_uint8 * 16), ("_limits_and_sparse", c_uint8 * 532),
    ]


class VkMemoryType(Structure):
    _fields_ = [("propertyFlags", c_uint32), ("heapIndex", c_uint32)]


class VkMemoryHeap(Structure):
    _fields_ = [("size", c_uint64), ("flags", c_uint32)]


class VkPhysicalDeviceMemoryProperties(Structure):
    _fields_ = [
        ("memoryTypeCount", c_uint32), ("memoryTypes", VkMemoryType * 32),
        ("memoryHeapCount", c_uint32), ("memoryHeaps", VkMemoryHeap * 16),
    ]


class VkExtensionProperties(Structure):
    _fields_ = [("extensionName", c_char * 256), ("specVersion", c_uint32)]


class VkQueueFamilyProperties(Structure):
    _fields_ = [
        ("queueFlags", c_uint32), ("queueCount", c_uint32), ("timestampValidBits", c_uint32),
        ("minImageTransferGranularity_w", c_uint32), ("minImageTransferGranularity_h", c_uint32),
        ("minImageTransferGranularity_d", c_uint32),
    ]


class VkDeviceQueueCreateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32),
        ("queueFamilyIndex", c_uint32), ("queueCount", c_uint32),
        ("pQueuePriorities", POINTER(c_float)),
    ]


class VkPhysicalDeviceBufferDeviceAddressFeatures(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("bufferDeviceAddress", c_uint32), ("bufferDeviceAddressCaptureReplay", c_uint32),
        ("bufferDeviceAddressMultiDevice", c_uint32),
    ]


class VkPhysicalDeviceAccelerationStructureFeaturesKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("accelerationStructure", c_uint32), ("accelerationStructureCaptureReplay", c_uint32),
        ("accelerationStructureIndirectBuild", c_uint32), ("accelerationStructureHostCommands", c_uint32),
        ("descriptorBindingAccelerationStructureUpdateAfterBind", c_uint32),
    ]


class VkDeviceCreateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32),
        ("queueCreateInfoCount", c_uint32), ("pQueueCreateInfos", c_void_p),
        ("enabledLayerCount", c_uint32), ("ppEnabledLayerNames", c_void_p),
        ("enabledExtensionCount", c_uint32), ("ppEnabledExtensionNames", c_void_p),
        ("pEnabledFeatures", c_void_p),
    ]


class VkBufferCreateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32),
        ("size", c_uint64), ("usage", c_uint32), ("sharingMode", c_uint32),
        ("queueFamilyIndexCount", c_uint32), ("pQueueFamilyIndices", c_void_p),
    ]


class VkMemoryRequirements(Structure):
    _fields_ = [("size", c_uint64), ("alignment", c_uint64), ("memoryTypeBits", c_uint32)]


class VkMemoryAllocateFlagsInfo(Structure):
    _fields_ = [("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32), ("deviceMask", c_uint32)]


class VkMemoryAllocateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("allocationSize", c_uint64), ("memoryTypeIndex", c_uint32),
    ]


class VkBufferDeviceAddressInfo(Structure):
    _fields_ = [("sType", c_uint32), ("pNext", c_void_p), ("buffer", c_uint64)]


class VkCommandPoolCreateInfo(Structure):
    _fields_ = [("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32), ("queueFamilyIndex", c_uint32)]


class VkCommandBufferAllocateInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("commandPool", c_uint64),
        ("level", c_uint32), ("commandBufferCount", c_uint32),
    ]


class VkCommandBufferBeginInfo(Structure):
    _fields_ = [("sType", c_uint32), ("pNext", c_void_p), ("flags", c_uint32), ("pInheritanceInfo", c_void_p)]


class VkSubmitInfo(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("waitSemaphoreCount", c_uint32), ("pWaitSemaphores", c_void_p), ("pWaitDstStageMask", c_void_p),
        ("commandBufferCount", c_uint32), ("pCommandBuffers", POINTER(c_uint64)),
        ("signalSemaphoreCount", c_uint32), ("pSignalSemaphores", c_void_p),
    ]


class VkDeviceOrHostAddressKHR(Union):
    _fields_ = [("deviceAddress", c_uint64), ("hostAddress", c_void_p)]


class VkDeviceOrHostAddressConstKHR(Union):
    _fields_ = [("deviceAddress", c_uint64), ("hostAddress", c_void_p)]


class VkASTrianglesDataKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("vertexFormat", c_uint32), ("vertexData", VkDeviceOrHostAddressConstKHR),
        ("vertexStride", c_uint64), ("maxVertex", c_uint32),
        ("indexType", c_uint32), ("indexData", VkDeviceOrHostAddressConstKHR),
        ("transformData", VkDeviceOrHostAddressConstKHR),
    ]


class VkASGeometryDataKHR(Union):
    # 真实 union 尺寸 = max(triangles=64, instances=32, aabbs=32) = 64 字节；
    # pad 过大将把 VkASGeometryKHR.flags 推离真实偏移 88，勿改。
    _fields_ = [("triangles", VkASTrianglesDataKHR), ("_pad", c_uint8 * 64)]


class VkASGeometryKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("geometryType", c_uint32), ("geometry", VkASGeometryDataKHR), ("flags", c_uint32),
    ]


class VkASBuildRangeInfoKHR(Structure):
    _fields_ = [
        ("primitiveCount", c_uint32), ("primitiveOffset", c_uint32),
        ("firstVertex", c_uint32), ("transformOffset", c_uint32),
    ]


class VkASBuildGeometryInfoKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("type", c_uint32), ("flags", c_uint32), ("mode", c_uint32),
        ("srcAccelerationStructure", c_uint64), ("dstAccelerationStructure", c_uint64),
        ("geometryCount", c_uint32), ("pGeometries", c_void_p), ("ppGeometries", c_void_p),
        ("scratchData", VkDeviceOrHostAddressKHR),
    ]


class VkASBuildSizesInfoKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("accelerationStructureSize", c_uint64), ("updateScratchSize", c_uint64),
        ("buildScratchSize", c_uint64),
    ]


class VkASCreateInfoKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p), ("createFlags", c_uint32),
        ("buffer", c_uint64), ("offset", c_uint64), ("size", c_uint64),
        ("type", c_uint32), ("deviceAddress", c_uint64),
    ]


class VkPhysicalDeviceAccelerationStructurePropertiesKHR(Structure):
    _fields_ = [
        ("sType", c_uint32), ("pNext", c_void_p),
        ("maxGeometryCount", c_uint64), ("maxInstanceCount", c_uint64), ("maxPrimitiveCount", c_uint64),
        ("maxPerStageDescriptorAccelerationStructures", c_uint32),
        ("maxPerStageDescriptorUpdateAfterBindAccelerationStructures", c_uint32),
        ("maxDescriptorSetAccelerationStructures", c_uint32),
        ("maxDescriptorSetUpdateAfterBindAccelerationStructures", c_uint32),
        ("minAccelerationStructureScratchOffsetAlignment", c_uint32),
    ]


class VkPhysicalDeviceProperties2(Structure):
    _fields_ = [("sType", c_uint32), ("pNext", c_void_p), ("properties", VkPhysicalDeviceProperties)]


def die(msg: str) -> None:
    print(f"[g9_vram_as_baseline_probe] FATAL: {msg}", file=sys.stderr)
    sys.exit(2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=None, help="evidence 输出路径（默认 evidence/g9_vram_as_baseline_<UTC>.json）")
    args = parser.parse_args()

    vk = ctypes.WinDLL("vulkan-1")

    # 句柄/返回值均为 64 位：显式声明签名，避免 ctypes 默认 c_int 截断溢出
    # （数组迭代产出裸 Python int，未声明 argtypes 时按 32 位 int 转换会 OverflowError）。
    vk.vkCreateInstance.argtypes = [c_void_p, c_void_p, POINTER(c_uint64)]
    vk.vkCreateInstance.restype = c_uint32
    vk.vkEnumeratePhysicalDevices.argtypes = [c_uint64, POINTER(c_uint32), c_void_p]
    vk.vkEnumeratePhysicalDevices.restype = c_uint32
    vk.vkGetPhysicalDeviceProperties.argtypes = [c_uint64, c_void_p]
    vk.vkGetPhysicalDeviceMemoryProperties.argtypes = [c_uint64, c_void_p]
    vk.vkEnumerateDeviceExtensionProperties.argtypes = [c_uint64, c_char_p, POINTER(c_uint32), c_void_p]
    vk.vkEnumerateDeviceExtensionProperties.restype = c_uint32
    vk.vkGetPhysicalDeviceProperties2.argtypes = [c_uint64, c_void_p]
    vk.vkGetPhysicalDeviceQueueFamilyProperties.argtypes = [c_uint64, POINTER(c_uint32), c_void_p]
    vk.vkCreateDevice.argtypes = [c_uint64, c_void_p, c_void_p, POINTER(c_uint64)]
    vk.vkCreateDevice.restype = c_uint32
    vk.vkGetDeviceQueue.argtypes = [c_uint64, c_uint32, c_uint32, POINTER(c_uint64)]
    vk.vkGetDeviceProcAddr.argtypes = [c_uint64, c_char_p]
    vk.vkGetDeviceProcAddr.restype = c_void_p
    vk.vkCreateBuffer.argtypes = [c_uint64, c_void_p, c_void_p, POINTER(c_uint64)]
    vk.vkCreateBuffer.restype = c_uint32
    vk.vkGetBufferMemoryRequirements.argtypes = [c_uint64, c_uint64, c_void_p]
    vk.vkAllocateMemory.argtypes = [c_uint64, c_void_p, c_void_p, POINTER(c_uint64)]
    vk.vkAllocateMemory.restype = c_uint32
    vk.vkBindBufferMemory.argtypes = [c_uint64, c_uint64, c_uint64, c_uint64]
    vk.vkBindBufferMemory.restype = c_uint32
    vk.vkGetBufferDeviceAddress.argtypes = [c_uint64, c_void_p]
    vk.vkGetBufferDeviceAddress.restype = c_uint64
    vk.vkMapMemory.argtypes = [c_uint64, c_uint64, c_uint64, c_uint64, c_uint32, POINTER(c_void_p)]
    vk.vkMapMemory.restype = c_uint32
    vk.vkUnmapMemory.argtypes = [c_uint64, c_uint64]
    vk.vkCreateCommandPool.argtypes = [c_uint64, c_void_p, c_void_p, POINTER(c_uint64)]
    vk.vkCreateCommandPool.restype = c_uint32
    vk.vkAllocateCommandBuffers.argtypes = [c_uint64, c_void_p, POINTER(c_uint64)]
    vk.vkAllocateCommandBuffers.restype = c_uint32
    vk.vkBeginCommandBuffer.argtypes = [c_uint64, c_void_p]
    vk.vkBeginCommandBuffer.restype = c_uint32
    vk.vkEndCommandBuffer.argtypes = [c_uint64]
    vk.vkEndCommandBuffer.restype = c_uint32
    vk.vkQueueSubmit.argtypes = [c_uint64, c_uint32, c_void_p, c_uint64]
    vk.vkQueueSubmit.restype = c_uint32
    vk.vkQueueWaitIdle.argtypes = [c_uint64]
    vk.vkQueueWaitIdle.restype = c_uint32
    vk.vkResetCommandBuffer.argtypes = [c_uint64, c_uint32]
    vk.vkResetCommandBuffer.restype = c_uint32

    # --- instance ---
    app = VkApplicationInfo(
        sType=ST_APPLICATION_INFO, pNext=None,
        pApplicationName=b"g9-vram-as-baseline-probe", applicationVersion=1,
        pEngineName=b"rurix-ci", engineVersion=1, apiVersion=VK_API_VERSION_1_2,
    )
    ici = VkInstanceCreateInfo(
        sType=ST_INSTANCE_CREATE_INFO, pNext=None, flags=0,
        pApplicationInfo=ctypes.cast(byref(app), c_void_p),
        enabledLayerCount=0, ppEnabledLayerNames=None,
        enabledExtensionCount=0, ppEnabledExtensionNames=None,
    )
    instance = c_uint64(0)
    if vk.vkCreateInstance(byref(ici), None, byref(instance)) != VK_SUCCESS:
        die("vkCreateInstance 失败（无 Vulkan loader？）")

    count = c_uint32(0)
    vk.vkEnumeratePhysicalDevices(instance, byref(count), None)
    if count.value == 0:
        die("零物理设备")
    devs = (c_uint64 * count.value)()
    vk.vkEnumeratePhysicalDevices(instance, byref(count), devs)

    # 选 NVIDIA 独显（vendorID 0x10DE），找不到则取第一个 discrete
    pdev = None
    props = VkPhysicalDeviceProperties()
    for d in devs:
        vk.vkGetPhysicalDeviceProperties(d, byref(props))
        if props.vendorID == 0x10DE and props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU:
            pdev = d
            break
    if pdev is None:
        for d in devs:
            vk.vkGetPhysicalDeviceProperties(d, byref(props))
            if props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU:
                pdev = d
                break
    if pdev is None:
        die("无 discrete GPU")
    vk.vkGetPhysicalDeviceProperties(pdev, byref(props))
    gpu_name = props.deviceName.decode("utf-8", "replace").rstrip("\x00")
    drv = props.driverVersion
    driver_str = f"{(drv >> 22) & 0x3FF}.{(drv >> 14) & 0xFF}.{(drv >> 6) & 0xFF}"

    # --- memory properties（VRAM 基线）---
    mem = VkPhysicalDeviceMemoryProperties()
    vk.vkGetPhysicalDeviceMemoryProperties(pdev, byref(mem))
    heaps = [
        {"size": mem.memoryHeaps[i].size, "flags": mem.memoryHeaps[i].flags}
        for i in range(mem.memoryHeapCount)
    ]
    device_local_heap = max((h["size"] for h in heaps if h["flags"] & VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT), default=0)
    if device_local_heap == 0:
        die("无 device local heap")

    # --- extension snapshot ---
    ext_count = c_uint32(0)
    vk.vkEnumerateDeviceExtensionProperties(pdev, None, byref(ext_count), None)
    ext_arr = (VkExtensionProperties * ext_count.value)()
    vk.vkEnumerateDeviceExtensionProperties(pdev, None, byref(ext_count), ext_arr)
    present = {e.extensionName.decode("ascii").rstrip("\x00") for e in ext_arr}
    snapshot = {name: (name in present) for name in SNAPSHOT_EXTS}
    for needed in NEEDED_DEVICE_EXTS:
        if needed.decode() not in present:
            die(f"缺必需设备扩展 {needed.decode()}（RTX 4070 Ti 应具；缺失即 fail-closed）")

    # --- AS properties（properties2 链）---
    as_props = VkPhysicalDeviceAccelerationStructurePropertiesKHR(
        sType=ST_PHYSICAL_DEVICE_AS_PROPERTIES_KHR, pNext=None
    )
    props2 = VkPhysicalDeviceProperties2(
        sType=ST_PHYSICAL_DEVICE_PROPERTIES_2, pNext=ctypes.cast(byref(as_props), c_void_p)
    )
    vk.vkGetPhysicalDeviceProperties2(pdev, byref(props2))

    # --- queue family ---
    qcount = c_uint32(0)
    vk.vkGetPhysicalDeviceQueueFamilyProperties(pdev, byref(qcount), None)
    qprops = (VkQueueFamilyProperties * qcount.value)()
    vk.vkGetPhysicalDeviceQueueFamilyProperties(pdev, byref(qcount), qprops)
    qfam = next((i for i, q in enumerate(qprops) if q.queueFlags & VK_QUEUE_GRAPHICS_BIT), None)
    if qfam is None:
        die("无 graphics queue family")

    # --- device ---
    bda_feat = VkPhysicalDeviceBufferDeviceAddressFeatures(
        sType=ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES, pNext=None,
        bufferDeviceAddress=1, bufferDeviceAddressCaptureReplay=0, bufferDeviceAddressMultiDevice=0,
    )
    as_feat = VkPhysicalDeviceAccelerationStructureFeaturesKHR(
        sType=ST_PHYSICAL_DEVICE_AS_FEATURES_KHR, pNext=ctypes.cast(byref(bda_feat), c_void_p),
        accelerationStructure=1, accelerationStructureCaptureReplay=0,
        accelerationStructureIndirectBuild=0, accelerationStructureHostCommands=0,
        descriptorBindingAccelerationStructureUpdateAfterBind=0,
    )
    prio = c_float(1.0)
    qci = VkDeviceQueueCreateInfo(
        sType=ST_DEVICE_QUEUE_CREATE_INFO, pNext=None, flags=0,
        queueFamilyIndex=qfam, queueCount=1, pQueuePriorities=POINTER(c_float)(prio),
    )
    ext_names = (c_char_p * len(NEEDED_DEVICE_EXTS))(*NEEDED_DEVICE_EXTS)
    dci = VkDeviceCreateInfo(
        sType=ST_DEVICE_CREATE_INFO, pNext=ctypes.cast(byref(as_feat), c_void_p), flags=0,
        queueCreateInfoCount=1, pQueueCreateInfos=ctypes.cast(byref(qci), c_void_p),
        enabledLayerCount=0, ppEnabledLayerNames=None,
        enabledExtensionCount=len(NEEDED_DEVICE_EXTS),
        ppEnabledExtensionNames=ctypes.cast(ext_names, c_void_p),
        pEnabledFeatures=None,
    )
    device = c_uint64(0)
    if vk.vkCreateDevice(pdev, byref(dci), None, byref(device)) != VK_SUCCESS:
        die("vkCreateDevice 失败（AS/BDA feature 链被拒）")
    queue = c_uint64(0)
    vk.vkGetDeviceQueue(device, qfam, 0, byref(queue))

    gp = vk.vkGetDeviceProcAddr
    gp.restype = c_void_p
    vkCreateAS = ctypes.CFUNCTYPE(c_uint32, c_uint64, c_void_p, c_void_p, POINTER(c_uint64))(
        gp(device, b"vkCreateAccelerationStructureKHR"))
    vkGetASSizes = ctypes.CFUNCTYPE(
        None, c_uint64, c_uint32, c_void_p, POINTER(c_uint32), c_void_p)(
        gp(device, b"vkGetAccelerationStructureBuildSizesKHR"))
    vkCmdBuildAS = ctypes.CFUNCTYPE(
        None, c_uint64, c_uint32, c_void_p, POINTER(POINTER(VkASBuildRangeInfoKHR)))(
        gp(device, b"vkCmdBuildAccelerationStructuresKHR"))
    vkDestroyAS = ctypes.CFUNCTYPE(None, c_uint64, c_uint64, c_void_p)(
        gp(device, b"vkDestroyAccelerationStructureKHR"))
    if not all([vkCreateAS, vkGetASSizes, vkCmdBuildAS, vkDestroyAS]):
        die("AS 扩展函数指针缺失")

    # --- 内存类型选择 ---
    def find_mem_type(bits: int, flags: int) -> int:
        for i in range(mem.memoryTypeCount):
            if (bits & (1 << i)) and (mem.memoryTypes[i].propertyFlags & flags) == flags:
                return i
        print("  [diag] memory types: " + ", ".join(
            f"{i}:{mem.memoryTypes[i].propertyFlags:#x}" for i in range(mem.memoryTypeCount)))
        die(f"无匹配内存类型 bits={bits:#x} flags={flags:#x}（ReBAR heap 缺失？）")
        return -1

    host_vis_idx = None
    dev_local_idx = None

    def create_buffer(size: int, usage: int, mem_flags: int):
        bci = VkBufferCreateInfo(
            sType=ST_BUFFER_CREATE_INFO, pNext=None, flags=0, size=size,
            usage=usage, sharingMode=0, queueFamilyIndexCount=0, pQueueFamilyIndices=None,
        )
        buf = c_uint64(0)
        if vk.vkCreateBuffer(device, byref(bci), None, byref(buf)) != VK_SUCCESS:
            die("vkCreateBuffer 失败")
        req = VkMemoryRequirements()
        vk.vkGetBufferMemoryRequirements(device, buf, byref(req))
        nonlocal host_vis_idx, dev_local_idx
        mt = find_mem_type(req.memoryTypeBits, mem_flags)
        fl = VkMemoryAllocateFlagsInfo(
            sType=ST_MEMORY_ALLOCATE_FLAGS_INFO, pNext=None,
            flags=VK_MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT, deviceMask=0,
        )
        ai = VkMemoryAllocateInfo(
            sType=ST_MEMORY_ALLOCATE_INFO, pNext=ctypes.cast(byref(fl), c_void_p),
            allocationSize=req.size, memoryTypeIndex=mt,
        )
        m = c_uint64(0)
        if vk.vkAllocateMemory(device, byref(ai), None, byref(m)) != VK_SUCCESS:
            die("vkAllocateMemory 失败")
        if vk.vkBindBufferMemory(device, buf, m, 0) != VK_SUCCESS:
            die("vkBindBufferMemory 失败")
        return buf, m

    def buf_addr(buf: c_uint64) -> int:
        info = VkBufferDeviceAddressInfo(sType=ST_BUFFER_DEVICE_ADDRESS_INFO, pNext=None, buffer=buf.value)
        return vk.vkGetBufferDeviceAddress(device, byref(info))

    # --- 网格数据（host visible coherent + device address）---
    nv = (GRID_N + 1) * (GRID_N + 1)
    ntri = 2 * GRID_N * GRID_N
    verts = []
    for y in range(GRID_N + 1):
        for x in range(GRID_N + 1):
            verts.extend((float(x), float(y), 0.0))
    idx = []
    for y in range(GRID_N):
        for x in range(GRID_N):
            v0 = y * (GRID_N + 1) + x
            v1 = v0 + 1
            v2 = v0 + GRID_N + 1
            v3 = v2 + 1
            idx.extend((v0, v2, v1, v1, v2, v3))
    vbytes = (c_float * len(verts))(*verts)
    ibytes = (c_uint32 * len(idx))(*idx)

    # 顶点/索引缓冲需 device address + host 可写：本驱动上 sysmem heap（HOST_VISIBLE）
    # 的 DEVICE_ADDRESS 分配不可映射（VK_ERROR_MEMORY_MAP_FAILED，实测隔离），
    # 故走 ReBAR heap（DEVICE_LOCAL|HOST_VISIBLE|HOST_COHERENT）；缺失即 fail-closed。
    vbuf, vmem = create_buffer(
        len(verts) * 4,
        VK_BUFFER_USAGE_AS_BUILD_INPUT_READ_ONLY_BIT_KHR | VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
        | VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
    )
    ibuf, imem = create_buffer(
        len(idx) * 4,
        VK_BUFFER_USAGE_AS_BUILD_INPUT_READ_ONLY_BIT_KHR | VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
        | VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
    )
    ptr = c_void_p()
    rc = vk.vkMapMemory(device, vmem, 0, len(verts) * 4, 0, byref(ptr))
    if rc != VK_SUCCESS:
        die(f"vkMapMemory vbuf rc={rc}")
    ctypes.memmove(ptr, vbytes, len(verts) * 4)
    vk.vkUnmapMemory(device, vmem)
    rc = vk.vkMapMemory(device, imem, 0, len(idx) * 4, 0, byref(ptr))
    if rc != VK_SUCCESS:
        die(f"vkMapMemory ibuf rc={rc}")
    ctypes.memmove(ptr, ibytes, len(idx) * 4)
    vk.vkUnmapMemory(device, imem)

    tri = VkASTrianglesDataKHR(
        sType=ST_AS_GEOMETRY_TRIANGLES_DATA_KHR, pNext=None,
        vertexFormat=VK_FORMAT_R32G32B32_SFLOAT,
        vertexData=VkDeviceOrHostAddressConstKHR(deviceAddress=buf_addr(vbuf)),
        vertexStride=12, maxVertex=nv - 1,
        indexType=(VK_INDEX_TYPE_UINT32 if not os.environ.get("G9_PROBE_NO_INDEX") else VK_INDEX_TYPE_NONE_KHR),
        indexData=(VkDeviceOrHostAddressConstKHR(deviceAddress=buf_addr(ibuf))
                   if not os.environ.get("G9_PROBE_NO_INDEX") else VkDeviceOrHostAddressConstKHR(deviceAddress=0)),
        transformData=VkDeviceOrHostAddressConstKHR(deviceAddress=0),
    )
    geo = VkASGeometryKHR(
        sType=ST_AS_GEOMETRY_KHR, pNext=None,
        geometryType=VK_GEOMETRY_TYPE_TRIANGLES_KHR,
        geometry=VkASGeometryDataKHR(triangles=tri),
        flags=GEO_FLAGS,
    )
    build_info = VkASBuildGeometryInfoKHR(
        sType=ST_AS_BUILD_GEOMETRY_INFO_KHR, pNext=None,
        type=VK_AS_TYPE_BOTTOM_LEVEL_KHR, flags=BUILD_FLAGS,
        mode=VK_BUILD_AS_MODE_BUILD_KHR,
        srcAccelerationStructure=0, dstAccelerationStructure=0,
        geometryCount=1, pGeometries=ctypes.cast(byref(geo), c_void_p), ppGeometries=None,
        scratchData=VkDeviceOrHostAddressKHR(deviceAddress=0),
    )
    prim_counts = (c_uint32 * 1)(ntri)
    sizes = VkASBuildSizesInfoKHR(sType=ST_AS_BUILD_SIZES_INFO_KHR, pNext=None)
    vkGetASSizes(device, VK_AS_BUILD_TYPE_DEVICE_KHR, byref(build_info), prim_counts, byref(sizes))
    if sizes.accelerationStructureSize == 0 or sizes.buildScratchSize == 0:
        die("AS build sizes 为零")

    as_buf, as_mem = create_buffer(
        sizes.accelerationStructureSize,
        VK_BUFFER_USAGE_AS_STORAGE_BIT_KHR | VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
    )
    scratch_buf, scratch_mem = create_buffer(
        sizes.buildScratchSize + 256,
        VK_BUFFER_USAGE_STORAGE_BUFFER_BIT | VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
    )
    align = max(int(as_props.minAccelerationStructureScratchOffsetAlignment), 128)
    scratch_addr = (buf_addr(scratch_buf) + align - 1) & ~(align - 1)
    build_info.scratchData = VkDeviceOrHostAddressKHR(deviceAddress=scratch_addr)

    if os.environ.get("G9_PROBE_DEBUG"):
        print(f"  [dbg] as_props: maxGeom={as_props.maxGeometryCount} "
              f"maxPrim={as_props.maxPrimitiveCount} scratchAlign={as_props.minAccelerationStructureScratchOffsetAlignment}")
        print(f"  [dbg] sizes: as={sizes.accelerationStructureSize} scratch={sizes.buildScratchSize} update={sizes.updateScratchSize}")
        print(f"  [dbg] addrs: vbuf={buf_addr(vbuf):#x} ibuf={buf_addr(ibuf):#x} scratch={scratch_addr:#x}")
        print(f"  [dbg] ntri={ntri} nv={nv} qfam={qfam}")
        import struct as _st
        _bi = ctypes.string_at(byref(build_info), ctypes.sizeof(build_info))
        _geo = ctypes.string_at(byref(geo), ctypes.sizeof(geo))
        print(f"  [dbg] sizeof build_info={len(_bi)} geo={len(_geo)} tri={ctypes.sizeof(VkASTrianglesDataKHR)}")
        print(f"  [dbg] build_info: sType={_st.unpack_from('<I',_bi,0)[0]} type={_st.unpack_from('<I',_bi,16)[0]} "
              f"flags={_st.unpack_from('<I',_bi,20)[0]} mode={_st.unpack_from('<I',_bi,24)[0]} "
              f"geomCount={_st.unpack_from('<I',_bi,48)[0]} pGeom={_st.unpack_from('<Q',_bi,56)[0]:#x} "
              f"scratch={_st.unpack_from('<Q',_bi,72)[0]:#x}")
        print(f"  [dbg] geo: sType={_st.unpack_from('<I',_geo,0)[0]} geomType={_st.unpack_from('<I',_geo,16)[0]} "
              f"flags={_st.unpack_from('<I',_geo,88)[0]}")
        print(f"  [dbg] tri: sType={_st.unpack_from('<I',_geo,24)[0]} vfmt={_st.unpack_from('<I',_geo,40)[0]} "
              f"vdata={_st.unpack_from('<Q',_geo,48)[0]:#x} vstride={_st.unpack_from('<Q',_geo,56)[0]} "
              f"maxV={_st.unpack_from('<I',_geo,64)[0]} itype={_st.unpack_from('<I',_geo,68)[0]} "
              f"idata={_st.unpack_from('<Q',_geo,72)[0]:#x}")
    _no_build = bool(os.environ.get("G9_PROBE_NO_BUILD"))

    pool_ci = VkCommandPoolCreateInfo(
        sType=ST_COMMAND_POOL_CREATE_INFO, pNext=None, flags=0, queueFamilyIndex=qfam
    )
    pool = c_uint64(0)
    if vk.vkCreateCommandPool(device, byref(pool_ci), None, byref(pool)) != VK_SUCCESS:
        die("vkCreateCommandPool 失败")
    cb_alloc = VkCommandBufferAllocateInfo(
        sType=ST_COMMAND_BUFFER_ALLOCATE_INFO, pNext=None, commandPool=pool.value,
        level=0, commandBufferCount=1,
    )
    cmd = c_uint64(0)
    if vk.vkAllocateCommandBuffers(device, byref(cb_alloc), byref(cmd)) != VK_SUCCESS:
        die("vkAllocateCommandBuffers 失败")

    def build_once() -> float:
        as_handle = c_uint64(0)
        aci = VkASCreateInfoKHR(
            sType=ST_AS_CREATE_INFO_KHR, pNext=None, createFlags=0,
            buffer=as_buf.value, offset=0, size=sizes.accelerationStructureSize,
            type=VK_AS_TYPE_BOTTOM_LEVEL_KHR, deviceAddress=0,
        )
        if vkCreateAS(device, byref(aci), None, byref(as_handle)) != VK_SUCCESS:
            die("vkCreateAccelerationStructureKHR 失败")
        build_info.dstAccelerationStructure = as_handle.value
        begin = VkCommandBufferBeginInfo(
            sType=ST_COMMAND_BUFFER_BEGIN_INFO, pNext=None, flags=0, pInheritanceInfo=None
        )
        if vk.vkBeginCommandBuffer(cmd, byref(begin)) != VK_SUCCESS:
            die("vkBeginCommandBuffer 失败")
        range_info = VkASBuildRangeInfoKHR(
            primitiveCount=ntri, primitiveOffset=0, firstVertex=0, transformOffset=0
        )
        prange = POINTER(VkASBuildRangeInfoKHR)(range_info)
        if not _no_build:
            vkCmdBuildAS(cmd, 1, byref(build_info), POINTER(POINTER(VkASBuildRangeInfoKHR))(prange))
        if vk.vkEndCommandBuffer(cmd) != VK_SUCCESS:
            die("vkEndCommandBuffer 失败")
        submit = VkSubmitInfo(
            sType=ST_SUBMIT_INFO, pNext=None,
            waitSemaphoreCount=0, pWaitSemaphores=None, pWaitDstStageMask=None,
            commandBufferCount=1, pCommandBuffers=POINTER(c_uint64)(cmd),
            signalSemaphoreCount=0, pSignalSemaphores=None,
        )
        t0 = time.perf_counter()
        rc = vk.vkQueueSubmit(queue, 1, byref(submit), 0)
        if rc != VK_SUCCESS:
            die(f"vkQueueSubmit rc={rc}")
        rc = vk.vkQueueWaitIdle(queue)
        if rc != VK_SUCCESS:
            die(f"vkQueueWaitIdle rc={rc}")
        ms = (time.perf_counter() - t0) * 1000.0
        vkDestroyAS(device, as_handle, None)
        if vk.vkResetCommandBuffer(cmd, 0) != VK_SUCCESS:
            die("vkResetCommandBuffer 失败")
        return ms

    build_once()  # warmup
    trials = [round(build_once(), 4) for _ in range(TRIALS)]
    median_ms = round(statistics.median(trials), 4)

    # --- nvidia-smi 辅助环境事实（非判据源，仅镜像）---
    smi = {}
    try:
        out = subprocess.run(
            ["nvidia-smi", "--query-gpu=name,memory.total,driver_version", "--format=csv,noheader"],
            capture_output=True, text=True, timeout=15,
        )
        if out.returncode == 0:
            smi = {"raw": out.stdout.strip()}
    except Exception:
        smi = {"raw": "nvidia-smi unavailable"}

    base_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT
    ).stdout.strip()

    now = datetime.now(timezone.utc)
    ts = now.strftime("%Y%m%dT%H%M%SZ")
    out_path = Path(args.out) if args.out else ROOT / "evidence" / f"g9_vram_as_baseline_{ts}.json"

    evidence = {
        "schema_version": 1,
        "subject": "g9_vram_as_baseline",
        "evidence_level": "measured_local",
        "milestone": "G9.1 governance-only measured baseline",
        "base_commit": base_commit,
        "timestamp": now.isoformat().replace("+00:00", "Z"),
        "environment": {
            "os": "Microsoft Windows 11",
            "gpu_name": gpu_name,
            "gpu_driver_decoded": driver_str,
            "gpu_driver_raw": drv,
            "python_version": sys.version.split()[0],
            "probe": "ci/g9_vram_as_baseline_probe.py（ctypes 直连 vulkan-1.dll，零新 Rust/C++）",
            "nvidia_smi_mirror": smi,
            "clock_lock_applicability": "AS 构建耗时为 host 墙钟含 vkQueueSubmit+vkQueueWaitIdle 同步等待（偏保守不含偏小）；GPU 未锁频；1 warmup + 5 trial 取中位。",
        },
        "commands": [
            {
                "seq": 1,
                "command": "py -3 ci/g9_vram_as_baseline_probe.py",
                "exit_code": 0,
                "note": "ctypes Vulkan：枚举设备/heap/扩展 → 建 130050 三角固定网格 BLAS → submit+waitIdle 同步计时",
            }
        ],
        "sampling": {
            "trials": TRIALS,
            "warmup": 1,
            "timer": "time.perf_counter 包住 vkQueueSubmit + vkQueueWaitIdle",
            "method": "固定 256x256 grid（130050 三角形、66049 顶点）BLAS（PREFER_FAST_TRACE、opaque）逐 trial 新建 AS 构建并同步等待；中位数为代表值。",
        },
        "results": {
            "metrics": {
                "vram_device_local_heap_bytes": device_local_heap,
                "blas_build_ms_130ktris": median_ms,
                "blas_storage_bytes_130ktris": sizes.accelerationStructureSize,
                "blas_scratch_bytes_130ktris": sizes.buildScratchSize,
            },
            "unit": {"vram_device_local_heap_bytes": "bytes", "blas_build_ms_130ktris": "ms",
                     "blas_storage_bytes_130ktris": "bytes", "blas_scratch_bytes_130ktris": "bytes"},
            "trial_values_ms": trials,
            "mesh": {"grid_n": GRID_N, "vertices": nv, "triangles": ntri,
                     "build_flags": "PREFER_FAST_TRACE | opaque"},
            "as_limits": {
                "maxGeometryCount": as_props.maxGeometryCount,
                "maxInstanceCount": as_props.maxInstanceCount,
                "maxPrimitiveCount": as_props.maxPrimitiveCount,
                "minAccelerationStructureScratchOffsetAlignment": as_props.minAccelerationStructureScratchOffsetAlignment,
            },
            "memory_heaps": heaps,
        },
        "capability_snapshot": {
            "extensions": snapshot,
            "note": "G9 D3 阻塞性前置（VK_EXT_device_generated_commands / VK_EXT_descriptor_buffer）与 D1/D2/D3 可选面（CLAS/mesh/SER/ray_query）实测在位性；micromap 两枚为禁止面/观察面诚实登记，不作承诺。",
        },
        "notes": "G9.1 governance-only baseline：只证明测量已建立，不证明任何 G9 实现达标。",
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[g9_vram_as_baseline_probe] OK gpu={gpu_name!r} driver={driver_str}")
    print(f"  vram_device_local_heap_bytes = {device_local_heap}")
    print(f"  blas_build_ms_130ktris       = {median_ms} (trials={trials})")
    print(f"  blas_storage_bytes_130ktris  = {sizes.accelerationStructureSize}")
    print(f"  blas_scratch_bytes_130ktris  = {sizes.buildScratchSize}")
    print(f"  extensions: {json.dumps(snapshot)}")
    print(f"  evidence → {out_path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
