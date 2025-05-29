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
