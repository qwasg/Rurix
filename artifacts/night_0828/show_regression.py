import json
from pathlib import Path
d = json.loads(Path(r"H:/rurix/artifacts/night_0828/regression_full/regression_summary.json").read_text(encoding="utf-8"))
for c in d["cells"]:
    if c["match"]:
        mark = "MATCH"
    elif c["rc"] != 0:
        mark = f"ERR rc={c['rc']}"
    else:
        mark = "DRIFT"
    fresh = (c.get("fresh") or "")[:26]
    anchor = (c.get("anchor") or "")[:26]
    print(f"{c['cell']:38s} {mark:10s} fresh={fresh} anchor={anchor}")
print("zero_drift:", d["matched"], "/", d["total"])
