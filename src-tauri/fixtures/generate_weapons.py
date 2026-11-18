import json, sys
from pathlib import Path
src, out = Path(sys.argv[1]), Path(sys.argv[2])
files = {"primary": "Primary.json", "secondary": "Secondary.json", "kitgun": "Secondary.json",
         "melee": "Melee.json", "zaw": "Melee.json", "archgun": "Arch-Gun.json",
         "sentinel": "SentinelWeapons.json"}
groups = {f: [] for f in set(files.values()) | {"Arch-Melee.json"}}
for w in sorted(json.loads((src / "riven_weapons.json").read_text())["data"], key=lambda w: w["slug"]):
    en = w["i18n"]["en"]
    groups[files[w["group"]]].append({
        "uniqueName": w["gameRef"], "name": en["name"], "icon": en.get("icon", ""),
        "wfmRivenUrl": w["slug"], "wfmRivenId": w["id"], "rivenType": w["rivenType"],
        "category": w["group"], "disposition": w.get("disposition", 0),
        "subType": None,
    })
for name, rows in groups.items():
    (out / name).write_text(json.dumps(rows, indent=1, ensure_ascii=False) + "\n")
    print(name, len(rows))
