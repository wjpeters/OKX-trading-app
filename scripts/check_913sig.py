"""Compare the OKX port with the supplied indicator's original label conditions."""
import json
import math
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
source_path = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "pine-scripts/913sig-okx.pine"
report_path = Path(sys.argv[2]) if len(sys.argv) > 2 else root / "evidence/913sig-verification.json"
source = source_path.read_text()


def expected(values):
    sf = sr = bf = br = None
    falling, rising, traces = [], [], []
    for i, close in enumerate(values):
        pf = i >= 4 and close < values[i - 4]
        pr = i >= 4 and close > values[i - 4]
        qf = i >= 2 and close < values[i - 2]
        qr = i >= 2 and close > values[i - 2]
        sf = (1 if sf is None else sf + 1) if pf else None
        sr = (1 if sr is None else sr + 1) if pr else None
        bf = (1 if bf is None else bf + 1) if sf is not None and sf > 9 and qf else None
        br = (1 if br is None else br + 1) if sr is not None and sr > 9 and qr else None
        falling.append(sf if sf in (7, 8, 9) else {2: 11, 3: 12, 4: 13}.get(bf, 0))
        rising.append(sr if sr in (7, 8, 9) else {2: 11, 3: 12, 4: 13}.get(br, 0))
        traces.append((sf, sr, bf, br))
    return falling, rising, traces


bonus_break = [200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189,
               188, 187, 186, 185.5, 184, 186, 183, 184.5, 182, 181,
               180, 179, 178, 177, 176, 175]
bonus_equal = bonus_break.copy()
bonus_equal[17] = bonus_equal[15]
setup_equal = bonus_break.copy()
setup_equal[17] = setup_equal[13]
cases = {
    "stijgende reeks": list(range(100, 160)),
    "dalende reeks": list(range(160, 100, -1)),
    "gelijke koersen": [100] * 60,
    "omkering en herstart": list(range(100, 130)) + list(range(130, 80, -1)) + list(range(80, 115)),
    "setup loopt door, bonus wordt onderbroken": bonus_break,
    "bonusonderbreking spiegelbeeld": [400 - x for x in bonus_break],
    "gelijkheid bonuslookback": bonus_equal,
    "gelijkheid setuplookback": setup_equal,
    "5000 candles met trends": [1000 + 50 * math.sin(i / 17) + 10 * math.sin(i / 3) + i // 300 for i in range(5000)],
}
checks = []
for name, values in cases.items():
    candles = [{"ts": 1700000000000 + i * 60000, "open": str(v), "high": str(v + 1),
                "low": str(v - 1), "close": str(v), "volume": "10", "closed": True}
               for i, v in enumerate(values)]
    job = {"instrument": "BTC-EUR", "timeframe": "1m", "step": 60000,
           "source": source, "candles": candles}
    process = subprocess.run(
        ["rtk", "proxy", "docker", "exec", "-i", "okx-tradingterminal-pine-1", "node",
         "--max-old-space-size=128", "--permission", "--allow-fs-read=/app", "/app/run.mjs"],
        input=json.dumps(job), capture_output=True, text=True, timeout=10,
    )
    assert process.returncode == 0, process.stdout + process.stderr
    result = json.loads(process.stdout)
    assert result["ok"], result
    falling, rising, _ = expected(values)
    plots = {plot["title"]: [point["value"] for point in plot["data"]] for plot in result["plots"]}
    assert plots == {"Dalend signaal": falling, "Stijgend signaal": rising}, name
    assert result["intents"] == [], name
    assert all([point["time"] for point in plot["data"]] == [bar["ts"] for bar in candles]
               for plot in result["plots"]), name
    checks.append({"name": name, "passed": True, "bars": len(values),
                   "falling_events": sum(bool(x) for x in falling),
                   "rising_events": sum(bool(x) for x in rising),
                   "duration_ms": round(result["duration_ms"], 1)})

# Verify that the corner-case fixtures really reach the intended branches.
assert expected(bonus_break)[2][17][0] > 9 and expected(bonus_break)[2][17][2] is None
assert expected(bonus_break)[0][19] == 11
assert expected(bonus_equal)[2][17][0] > 9 and expected(bonus_equal)[2][17][2] is None
assert expected(setup_equal)[2][17][0] is None
report = {"runtime": "PineTS 0.11.0 / closed-bar profile 1", "source": source_path.name,
          "oracle": "Original na-based counters and label conditions supplied by Willem",
          "checks": checks, "all_passed": True,
          "scope": "Script calculation only; no orders, accounts or bot changes"}
report_path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
