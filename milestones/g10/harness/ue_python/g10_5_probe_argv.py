#!/usr/bin/env python3
"""G10.5 harness — sys.argv 形态探针。Assisted-by: Kimi-K3（G10.5a 波）"""
import sys

import unreal

unreal.log("ARGV: sys.argv=" + repr(sys.argv))
try:
    import unreal as ue

    cl = ue.SystemLibrary.get_system_name()
    unreal.log("ARGV: sysname=" + cl)
except Exception as e:
    unreal.log("ARGV: " + repr(e))
