import pandas as pd
import requests


class combine(list):
    pass


runs = {
    "baseline": "https://haste.zneix.eu/raw/ucuqyrohab.apache",
    "no-logging": "https://haste.zneix.eu/raw/vobojasegy.apache",
    "global,no-logging": "https://haste.zneix.eu/raw/zibexafuko.apache",
    "global,no-logging,backlog-1024,keepalive-os": "https://haste.zneix.eu/raw/enaxusiguk.apache",
    "global,no-logging,backlog-1024,keepalive-os,snmalloc": "https://haste.zneix.eu/raw/medibijyky.apache",
    "ntex,no-logging,backlog-1024,keepalive-os": "https://haste.zneix.eu/raw/reniwanawy.apache",
    "ntex,no-logging,backlog-1024,keepalive-os,no-allocations": "https://haste.zneix.eu/raw/hamosaceba.apache",
    "ntex,no-logging,backlog-1024,keepalive-os,no-allocations,binary-payload": "https://haste.zneix.eu/raw/itamalihoh.apache",
    "localhost": "https://haste.zneix.eu/raw/fycaperety.apache",
    "domain,no-nginx": "https://haste.zneix.eu/raw/imucukajig.apache",
    "domain,nginx": "https://haste.zneix.eu/raw/pygyzoryty.apache",
    "domain,nginx,logging=info,stdout": "https://haste.zneix.eu/raw/qufomacysi.apache",
    "domain,nginx,logging=error,stdout": "https://haste.zneix.eu/raw/pynivyqydu.apache",
    "domain,nginx,logging=info,unix-pipe": "https://haste.zneix.eu/raw/ifawomujyg.apache",
    "domain,nginx,logging=info,fs-buffered": "https://haste.zneix.eu/raw/utedofabev.apache",
    "localhost,logging=uninitialized,no-backend": "https://haste.zneix.eu/raw/xotoxecuzo.apache",
    "localhost,logging=uninitialized,file-backend": "https://haste.zneix.eu/raw/qisagymyki.apache",
    "localhost,logging=uninitialized,redis-backend": "https://haste.zneix.eu/raw/nygykagace.apache",
    "localhost,logging=uninitialized,redis-backend,instances=2": combine(
        [
            "https://haste.zneix.eu/raw/uhadosanus.apache",
            "https://haste.zneix.eu/raw/vojopizuru.apache",
        ]
    ),
    "localhost,python:fastapi+uvloop+coredis+hiredis,logging=off,redis-backend": "https://haste.zneix.eu/raw/ovyzikamuh.apache"
}

for url in runs:
    stats = runs[url]
    should_combine = isinstance(stats, combine)

    if isinstance(stats, str):
        stats = (stats,)

    runs[url] = [requests.get(stat).text.strip().split("\n")[1:] for stat in stats]
    if should_combine:
        runs[url] = combine(runs[url])


for config in runs:
    readings = []

    # skip the first `warmup` seconds
    warmup = 3
    should_combine = isinstance(runs[config], combine)
    for i, run in enumerate(runs[config]):
        run_readings = []

        for line in run[warmup:]:
            before, after = line.split("ms:")
            time = before.rsplit(maxsplit=1)[-1].strip()
            requests = after.split()[0].strip()

            run_readings.append((int(time), int(requests)))

        if not readings:
            readings = run_readings
        else:
            run_n = i + 1
            for j, (time, rps) in enumerate(run_readings):
                time += readings[j][0] * run_n
                time /= run_n + 1
                if should_combine:
                    rps += readings[j][1]
                else:
                    rps += readings[j][1] * run_n
                    rps /= run_n + 1

                readings[j] = (time, rps)

    df = pd.DataFrame(readings, columns=("ms", "requests"))
    df["rps"] = df["requests"] / df["ms"] * 1000

    rps = df["rps"].mean()
    stddev = df["rps"].std()
    print(f"|{config}|`{int(rps):,}`|`{stddev:,.2f}`|")
