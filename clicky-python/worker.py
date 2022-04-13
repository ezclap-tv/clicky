from uvicorn.workers import UvicornWorker


class Worker(UvicornWorker):
    CONFIG_KWARGS = {"lifespan": "on", "log_level": "critical"}
