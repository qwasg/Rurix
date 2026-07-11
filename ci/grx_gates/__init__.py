# GRX per-pass gate modules (industrialized GRX-011+ scaffolding).
#
# One module per downstream GRX pass (grx011 ssao_blur, grx012 taa_resolve,
# ...). Each module exports `evaluate() -> dict` per the interface documented in
# `_common.py`. The passes are registered, in order, in
# `ci/godot_rurix_toolchain_probe.py::GRX_GATE_SEQUENCE`; the probe walks them
# fail-closed to decide whether to advance `next_action` past the grx010
# hand-off. This package ships with only `__init__.py` + `_common.py`; Wave 2
# adds `grx011_ssao_blur.py` as the first real gate module.
#
# Schema/version: gate-module interface v1 (see `_common.GATE_INTERFACE_VERSION`).
