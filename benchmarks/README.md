## Environment
| Param         | Value                        |
|---------------|------------------------------|
| CPU           | Ryzen 5 3600 (12) @ 3.600GHz |
| RAM           | 64GB                         |
| Kernel        | `5.4.0`                      |
| Soft FD limit | `1048576`                    |
| Hard FD limit | `1048576`                    |
| Client threads | `12` |
| Bench time | 60s |

## Results
NOTE: after more digging, I've discovered ntex spawns a thread both per physical and virtual  core, while actix spawns a thread only per physical core. This might explain the performance difference. Also, ntex threads consume the same amount of CPU consistently (around 25% per core), while actix threads jump from 15 to ~40avg to 60. Actix seems to take more time to deallocate memory than ntex, probably because ntex uses pools to allocate temporaries rather than the system allocator.


| Configuration                                                     | RPS       | Standard Deviation |
|-------------------------------------------------------------------|-----------|--------------------|
| Baseline                                                          | `43,904`  | `7,729.05`         |
| Baseline, no logging                                              | `219,502` | `10,962.64`        |
| Global counter, no logging                                        | `220,135` | `8,756.91`         |
| Global counter, no logging, `KeepAlive::Os`                       | `231,202` | `5,237.95`         |
| Global counter, no logging, `KeepAlive::Os`, `snmalloc`           | `230,914` | `6,937.38`         |
| Ntex, no logging, `KeepAlive::Os`                                 | `224,109` | `3,415.54`         |
| Ntex, no logging, `KeepAlive::Os`, no allocations                 | `226,791` | `1,854.91`         |
| Ntex, no logging, `KeepAlive::Os`, no allocations, binary payload | `226,093` | `2,407.74`         |


### Infrastructure Benchmarks (same datacenter)
Using Nginx reduces the performance by 70%, while logging to a file shaves 25% off that.
60k RPS over the network seems doable.

| Configuration                                                             | RPS       | Standard Deviation |
|---------------------------------------------------------------------------|-----------|--------------------|
| Localhost baseline                                                        | `246,831` | `1,304.38`         |
| Domain, no Nginx                                                          | `240,448` | `1,793.02`         |
| Domain, Nginx proxy                                                       | `83,211`  | `1,414.70`         |
| Domain, Nginx proxy, Logging=info to Stdout                               | `909`     | `319.89`           |
| Domain, Nginx proxy, Logging=error to Stdout                              | `53,615`  | `17,955.89`        |
| Domain, Nginx proxy, Logging=info to Stdout with a unix pipe to a file    | `62,322`  | `8,420.74`         |
| Domain, Nginx proxy, Logging=info to a buffered file                      | `62,358`  | `9,401.33`         |


### Backend Benchmarks
| Configuration                                         | RPS       | Standard Deviation |
| ----------------------------------------------------- | --------- | ------------------ |
| Localhost, Logging=uninitialized, No backend          | `232,045` | `2,939.84`         |
| Localhost, Logging=uninitialized, File (mmap) backend | `232,196` | `4,867.31`         |



## Preprocessing Code
```py
#! python -m pip install requests pandas
import pandas as pd
import requests

urls = {
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
    "localhost,logging=uninitialized,no-backend": "https://haste.zneix.eu/raw/sabifijope.apache",
    "localhost,logging=uninitialized,file-backend": "https://haste.zneix.eu/raw/faqysavywu.apache"
}

for url in urls:
    urls[url] = requests.get(urls[url]).text.strip().split("\n")[1:]

for url in urls:
    readings = []

    # skip the first `warmup` seconds
    warmup = 3
    for line in urls[url][warmup:]:
        before, after = line.split("ms:")
        time = before.rsplit(maxsplit=1)[-1].strip()
        requests = after.split()[0].strip()

        readings.append((int(time), int(requests)))

    df = pd.DataFrame(readings, columns=("ms", "requests"))
    df["rps"] = df["requests"] / df["ms"] * 1000

    rps = df["rps"].mean()
    stddev = df["rps"].std()
    print(f"|{url}|`{int(rps):,}`|`{stddev:,.2f}`|")

```
