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
A persistent counter backend(s) may be selected and enabled at compile using cargo features. For example, in order to use the file backend, use the following command:  `cargo build --release --features=backend-file`

### File backend
Periodically saves the counter to a text file.

| Cargo Feature   |
| --------------- |
| `backend-file`  |

| Env Variable              | Description                                                                                                                                                                                           | Default      |
| :-----------------------: | ----------------------------------------------------------------------------                                                                                                                          | ---------    |
| `CLICKY_COUNTER_FILE`     | Path to the file that should store the number of clicks.                                                                                                                                              | `clicky.txt` |
| `CLICKY_SYNC_FREQUENCY`   | The frequency with which the counter should be written to the file. Must be a number between 1 and 65536, followed by `ms`, `m`, or `s`. Providing an empty string or `0` will be treated as default. | `1s`         |
