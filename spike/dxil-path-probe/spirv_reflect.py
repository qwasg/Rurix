# SPIKE(RD-014) — B 路 strict-only 名保真取证:SPIR-V 二进制最小反射。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:解 -fspv-reflect SPIR-V 的 OpName / UserSemantic /
# BuiltIn / EntryPoint 执行模型,自动导出顶点输入变量名→原始 HLSL 语义映射,
# 供 spirv-cross --set-hlsl-named-vertex-input-semantic 驱动名保真;解析失败 ok=False。
"""SPIR-V 二进制最小反射(纯字节只读,无外部依赖)。

仅提取名保真所需:OpEntryPoint 执行模型 + OpName(<id>→名串)+
OpDecorate BuiltIn(标记内建,排除)+ OpDecorateString UserSemantic(<id>→HLSL 语义串)+
OpVariable StorageClass(辨别 Input/Output)。SPIR-V 头 5 word + 指令流
(word = (WordCount<<16)|Opcode)。
"""
from __future__ import annotations

import struct

_MAGIC = 0x07230203
# 相关 opcode / 枚举(SPIR-V 规范)。
_OP_ENTRY_POINT = 15
_OP_NAME = 5
_OP_DECORATE = 71
_OP_DECORATE_STRING = 5632  # OpDecorateString / OpDecorateStringGOOGLE
_OP_VARIABLE = 59
_DEC_BUILTIN = 11
_DEC_USER_SEMANTIC = 5635
_SC_INPUT = 1
_SC_OUTPUT = 3
_EM_NAMES = {0: "Vertex", 1: "TessControl", 2: "TessEval", 3: "Geometry",
             4: "Fragment", 5: "GLCompute", 6: "Kernel", 7: "TaskNV", 8: "MeshNV"}


def _read_string(words: list[int], start: int, count: int) -> str:
    """从 word 数组解 SPIR-V 字面字符串(小端字节,null 结尾)。"""
    raw = b"".join(struct.pack("<I", words[start + i]) for i in range(count))
    end = raw.find(b"\x00")
    return raw[:end if end >= 0 else len(raw)].decode("utf-8", "replace")


def parse_spirv(data: bytes) -> dict:
    """解 SPIR-V → {ok, exec_model, inputs:[{id,name,user_semantic,builtin}], outputs:[...]}.

    inputs/outputs 按 OpVariable 存储类归类;builtin=True 表示带 BuiltIn 装饰(SV 系统值)。
    name=OpName 串(spirv-cross 据此命名 HLSL 变量),user_semantic=UserSemantic 串(原 HLSL 语义)。
    """
    if not data or len(data) < 20:
        return {"ok": False, "reason": "too_short"}
    magic = struct.unpack("<I", data[:4])[0]
    if magic != _MAGIC:
        return {"ok": False, "reason": f"bad_magic_0x{magic:x}"}
    nwords = len(data) // 4
    words = list(struct.unpack(f"<{nwords}I", data[:nwords * 4]))
    names: dict[int, str] = {}
    user_sem: dict[int, str] = {}
    builtins: set[int] = set()
    storage: dict[int, int] = {}
    exec_model = None
    i = 5
    while i < nwords:
        word = words[i]
        wc = word >> 16
        op = word & 0xFFFF
        if wc == 0:
            return {"ok": False, "reason": f"zero_wordcount@{i}"}
        if op == _OP_ENTRY_POINT and exec_model is None:
            exec_model = words[i + 1]
        elif op == _OP_NAME:
            names[words[i + 1]] = _read_string(words, i + 2, wc - 2)
        elif op == _OP_DECORATE and wc >= 3 and words[i + 2] == _DEC_BUILTIN:
            builtins.add(words[i + 1])
        elif op == _OP_DECORATE_STRING and wc >= 3 and words[i + 2] == _DEC_USER_SEMANTIC:
            user_sem[words[i + 1]] = _read_string(words, i + 3, wc - 3)
        elif op == _OP_VARIABLE and wc >= 4:
            storage[words[i + 2]] = words[i + 3]  # result id → storage class
        i += wc
    inputs, outputs = [], []
    for vid, sc in storage.items():
        rec = {"id": vid, "name": names.get(vid, ""),
               "user_semantic": user_sem.get(vid, ""), "builtin": vid in builtins}
        if sc == _SC_INPUT:
            inputs.append(rec)
        elif sc == _SC_OUTPUT:
            outputs.append(rec)
    inputs.sort(key=lambda r: r["id"])
    outputs.sort(key=lambda r: r["id"])
    return {"ok": True, "exec_model": _EM_NAMES.get(exec_model, str(exec_model)),
            "exec_model_raw": exec_model, "inputs": inputs, "outputs": outputs}


def vertex_input_semantic_flags(refl: dict) -> list[str]:
    """从反射导出 spirv-cross --set-hlsl-named-vertex-input-semantic 参数列表。

    仅顶点阶段、非内建、OpName 与 UserSemantic 均非空的 Input 变量(名保真所需且可驱动)。
    """
    if not refl.get("ok") or refl.get("exec_model") != "Vertex":
        return []
    flags: list[str] = []
    for v in refl["inputs"]:
        if v["builtin"] or not v["name"] or not v["user_semantic"]:
            continue
        flags += ["--set-hlsl-named-vertex-input-semantic", v["name"], v["user_semantic"]]
    return flags
