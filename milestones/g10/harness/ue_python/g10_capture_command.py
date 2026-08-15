#!/usr/bin/env python3
"""G10.2 harness — 出图命令面闭集的结构化参数生成器草案（RFC-0027 §4.1.3）。

只允许 spike 实证的三臂命令形态；schema 外开关/参数注入即 fail-closed（本模块以白名单校验实现）；
禁 shell 字符串拼接注入——所有参数经类型校验后以参数列表（非 shell 字符串）产出，供 subprocess
list-form 调用。命令面/queue 配置 digest 供 provenance.capture_arm 字段拼接。

臂 A（MRQ 主路）：-game -MoviePipelineConfig=<queue> -windowed -resx/-resy -log -notexturestreaming -Unattended
臂 B（快速截屏）：-game -benchmark -fps=<N> -seconds=<N> -ResX/-ResY -execcmds=<白名单模板> -unattended -log -FixedSeed
臂 C（Python 编排）：-ExecutePythonScript=<script>.py（g10_mrq_render.py 路径白名单校验）

地位：DRAFT 占位可解析形态，待引擎可用后实测修订。
Assisted-by: Kimi-K3（G10.2 波）
"""
import hashlib
import json
import os
import re

import g10_determinism as det

_EDITOR_CMD = "UnrealEditor-Cmd.exe"  # 安装后由环境画像登记实测绝对路径；主机绝对路径不入签名面


class CommandSurfaceError(ValueError):
    """命令面越界：schema 外开关/参数注入。"""


def _check_token(name, v):
    if not isinstance(v, str) or not re.fullmatch(r"[A-Za-z0-9_./:\\-]+", v):
        raise CommandSurfaceError(f"{name}: illegal token {v!r}")
    return v


def build_arm_a(uproject, map_package, queue_asset, resx, resy, offscreen=False):
    """臂 A（MRQ 批量臂）参数列表。"""
    args = [
        _EDITOR_CMD,
        _check_token("uproject", uproject),
        _check_token("map", map_package),
        "-game",
        f"-MoviePipelineConfig={_check_token('queue', queue_asset)}",
        "-windowed",
        f"-resx={int(resx)}",
        f"-resy={int(resy)}",
        "-log",
        "-notexturestreaming",
        "-Unattended",
    ]
    if offscreen:
        # -renderoffscreen 5.8 可用性为 spike 待验证项——首日实测登记前默认 False
        args.append("-renderoffscreen")
    return args


def build_arm_b(uproject, map_package, resx, resy, fps, seconds):
    """臂 B（HighResShot 快速臂）参数列表；execcmds 走白名单模板（r.ResetViewState + HighResShot <W>x<H>）。"""
    return [
        _EDITOR_CMD,
        _check_token("uproject", uproject),
        _check_token("map", map_package),
        "-game",
        "-benchmark",
        f"-fps={int(fps)}",
        f"-seconds={int(seconds)}",
        f"-ResX={int(resx)}",
        f"-ResY={int(resy)}",
        f'-execcmds="{det.arm_b_execmds_template(resx, resy)}"',
        "-unattended",
        "-log",
        "-FixedSeed",
    ]


def build_arm_c(uproject, map_package, script_path, job_json):
    """臂 C（Python 编排臂）参数列表。"""
    return [
        _EDITOR_CMD,
        _check_token("uproject", uproject),
        _check_token("map", map_package),
        "-game",
        f"-ExecutePythonScript={_check_token('script', script_path)}",
        "--",
        _check_token("job", job_json),
        "-unattended",
        "-log",
    ]


def command_surface_digest(args):
    """命令面配置 digest（capture_arm 签名面组件之一）：参数列表 canonical join 的 SHA-256。
    首位可执行文件名固定不入 digest（主机绝对路径禁入签名面）。"""
    payload = "\x00".join(args[1:]).encode("utf-8")
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def describe(args):
    return {"argv_tail": args[1:], "command_surface_digest": command_surface_digest(args)}


if __name__ == "__main__":
    demo = build_arm_b("K:/Epic/UE_5.8/G10RefRender/G10RefRender.uproject",
                       "/Game/Maps/G10_CornellBox", 1920, 1080, 30, 10)
    print(json.dumps(describe(demo), ensure_ascii=False, indent=2))
