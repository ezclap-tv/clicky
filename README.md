# Clicky

endpoint | method | body | response
--|:--|:--|:--
`/`|`GET`| none | plaintext number
`/`|`POST`| plaintext number in the range 0-500 | plaintext number

## Building
A generic release build of the server can be obtained with `cargo build --release`.
In order to build the server for the current machine, set the RUSTFLAGS variable:

```bash
$ RUSTFLAGS="-Ctarget-cpu=native" cargo build --release
$ ./target/release/clicky
```

## Saving the count
A persistent counter backend may be selected and enabled at compile time using cargo features. For example, in order to use the file backend, use the following command:

```bash
$ cargo build --release --features=backend-file
```

### File backend
Periodically saves the counter to a text file.

| Cargo Feature   |
| --------------- |
| `backend-file`  |

| Env Variable | Description | Default |
| :----------: | -------- | ------- |
| `CLICKY_COUNTER_FILE` | Path to the file that should store the number of clicks. | `clicky.txt` |
| `CLICKY_SYNC_FREQUENCY` | See [humantime::parse_duration](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) for supported formats | `1s` |


### Redis backend
Periodically saves the counter to a [Redis](https://redis.com/) instance, synchronizing if needed. This backend supports multiple Clicky servers counting in parallel.

| Cargo Feature   |
| --------------- |
| `backend-redis`  |

| Env Variable | Description | Default |
| :----------: | -------- | ------- |
| `CLICKY_REDIS_URL` | URL of the redis instance to use. | `redis://127.0.0.1:6379` |
| `CLICKY_SYNC_FREQUENCY` | See [humantime::parse_duration](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) for supported formats | `1s` |
