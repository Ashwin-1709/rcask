# rstore

`rstore` is a simple, Rust-based in-memory key-value store built on the core concepts of **log-structured storage engines**.

---

## What does it have?

* **In-Memory Index:** Key value lookups are supported through `HashMap` that stores the exact disk offset for each key.
* **Log-Structured Persistence:** Data is appended to a file in a sequential "log" fashion.
* **Data Integrity:** Keys are read and validated during retrieval to help detect potential data corruption.
* **Storage APIs** `set` and `get` operations for storing and retrieving string-based key-value pairs.
* **Crash Recovery:** The in-memory index is rebuilt from the log file upon initialization, ensuring data persistence across application restarts.

---

## Examples

```rust
use rstore::kvstore::KVStore;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = KVStore::new(Path::new("kv_store.bin"))?;

    store.set("key1", "value1")?;
    store.set("key2", "value2")?;
    store.set("key3", "value3")?;
    
    println!("Value for key1: {:?}", store.get("key1")?);
    println!("Value for key2: {:?}", store.get("key2")?);
    println!("Value for key3: {:?}", store.get("key3")?);

    store.set("key1", "response: { values: ['A', 'B', 'C'] } ")?;
    println!("Updated value for key1: {:?}", store.get("key1")?);

    Ok(())
}
```

