# rcask
![crates.io](https://img.shields.io/crates/v/rcask.svg) ![Build Passing](https://github.com/Ashwin-1709/rcask/actions/workflows/rust.yml/badge.svg)

`rcask` is a [bitcask](https://docs.riak.com/riak/kv/2.2.3/setup/planning/backend/bitcask/index.html) inspired, rust-based in-memory key-value store built on the core concepts of **log-structured storage engines**.

---

## What does it have?

* **In-Memory Index:** Key value lookups are supported through `HashMap` that stores the exact disk offset for each key.
* **Log-Structured Persistence:** Data is appended to a file in a sequential "log" fashion.
* **Compaction:** Automatically compacts log files after a configurable number of writes to keep disk usage under control.
* **Data Integrity:** Keys are read and validated during retrieval to help detect potential data corruption.
* **Storage APIs** `set` and `get` operations for storing and retrieving string-based key-value pairs.
* **Crash Recovery:** The in-memory index is rebuilt from the log file upon initialization, ensuring data persistence across application restarts.

---

## Examples

```rust
use rcask::RCask;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut store = RCask::init(
        "./".to_string(), // Directory to store logs
        "log".to_string(), // Pattern for log files
        3)?; // Maximum number of writes before compaction

    // Default store with max_writes as 10000 before compaction
    let mut default_store = RCask::new(
        "./".to_string(), // Directory to store logs
        "default_log".to_string(), // Pattern for log files
    )?;

    store.set("key1", "value1")?;
    store.set("key2", "value2")?;
    store.set("key3", "value3")?;

    println!("Value for key1: {:?}", store.get("key1")?);
    println!("Value for key2: {:?}", store.get("key2")?);
    println!("Value for key3: {:?}", store.get("key3")?);

    store.set("key1", "response: { values: ['A', 'B', 'C'] } ")?;
    println!("Updated value for key1: {:?}", store.get("key1")?);

    return Ok(());
}
```

