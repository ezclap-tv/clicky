import time

import uvloop
import coredis
from fastapi import FastAPI, Request

KEY = "CLICKY_COUNTER"

class Counter:

    def __init__(self, client: 'coredis.client.Redis'):
        self.count = 0
        
        self.time = time.time()
        self.previous_remote_count = 0
        self.client = client

    def should_refresh(self):
        return time.time() - self.time > 1
    
    def _add(self, n: int):
        self.count += n
        return self.count - n

    async def add(self, n: int):
        if self.should_refresh():
            await self.sync()
        return self._add(n)

    async def get(self):
        if self.should_refresh():
            await self.sync()
        return self.count

    def get_new_count(self, remote_count: int):
        if remote_count < self.previous_remote_count:
            remote_count = self.previous_remote_count
        remote_contribution = remote_count - self.previous_remote_count
        local_count = self._add(remote_contribution)
        local_contribution = local_count - self.previous_remote_count
        remote_count = remote_count + local_contribution
        self.previous_remote_count = remote_count
        return remote_count

    async def sync(self):
        self.time = time.time()
        async with await self.client.pipeline() as pipe:
            try:
                await pipe.watch(KEY)
                remote_count = int(await pipe.get(KEY))
                remote_count = self.get_new_count(remote_count)
                await pipe.set(KEY, remote_count)
                # print("synced count at", remote_count)
            except coredis.WatchError:
                print("failed to sync count at", self.count)


uvloop.install()

app = FastAPI()
client = coredis.Redis()
counter = Counter(coredis.Redis())

@app.get("/")
async def sync():
    return await counter.get()

@app.post("/")
async def submit(request: Request):
    count = int(await request.body())
    return await counter.add(count) + count
