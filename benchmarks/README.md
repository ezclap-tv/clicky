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

Additionally, see the `main` and [`benchmarks-1`](https://github.com/ezclap-tv/clicky/tree/benchmarks-1) for the source code.

## Results
### Backend Benchmarks
- Rust benchmarks use Actix-web with `KeepAlive::Os` and the default backlog (`1024`)
- Go benchmark uses the fastest go web framework, Atreugo
- Python benchmark
  - Uses the fastest Python web framework, FastAPI, accelerated by C extensions
  - Uses uvloop, a C async event loop implementation that is known to speed up async Python programs by 2 to 4x. 
  - Uses a C Redis client, Hiredis, with the Coredis async library
  - Uses Gunicorn with Uvicorn workers, one per CPU core


| Configuration                                                           | RPS       | Standard Deviation |
| -----------------------------------------------------                   | --------- | ------------------ |
| Localhost, Logging=off, No backend                                      | `270,262` | `2,089.54`         |
| Localhost, Logging=off, File backend                                    | `272,643` | `1,536.49`         |
| Localhost, Logging=off, Redis backend                                   | `274,410` | `1,680.54`         |
| Localhost, Logging=off, Redis backend, Instances=2, Clients=2           | `289,549` | `17,861.72`        |
| Go, Atreugo, Logging=off, Redis backend                                 | `245,382` | `3,066.76`         |
| Python, FastAPI+Uvloop+Coredis/Hiredis, Logging=off, Gunicorn+Uvicorn   | `38,908`  | `201.45`           |


### Infrastructure Benchmarks (same datacenter, <i>already</i> outdated)
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

### Configuration Benchmarks (Outdated)

NOTE: after more digging, I've discovered ntex spawns a thread both per physical and virtual  core, while actix spawns a thread only per physical core. This might explain the performance difference. Also, ntex threads consume the same amount of CPU consistently (around 25% per core), while actix threads jump from 15 to ~40avg to 60. Actix seems to take more time to deallocate memory than ntex, probably because ntex uses pools to allocate temporaries rather than the system allocator.


<details>
 <summary>(NOTE: outdated benchmarks)</summary>

| Configuration                                                        | RPS       | Standard Deviation |
|----------------------------------------------------------------------|-----------|--------------------|
| Baseline, no logging                                                 | `219,502` | `10,962.64`        |
| Global counter, no logging                                           | `220,135` | `8,756.91`         |
| Global counter, no logging, `KeepAlive::Os`                          | `231,202` | `5,237.95`         |
| Global counter, no logging, `KeepAlive::Os`, `snmalloc`              | `230,914` | `6,937.38`         |
| Ntex, no logging, `KeepAlive::Os`                                    | `224,109` | `3,415.54`         |
| Ntex, no logging, `KeepAlive::Os`, no allocations                    | `226,791` | `1,854.91`         |
| Ntex, no logging, `KeepAlive::Os`, no allocations, binary payload    | `226,093` | `2,407.74`         |

</details>

## Preprocessing Code
See `stats.py`.
