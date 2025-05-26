mod kvstore;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let mut store = kvstore::KVStore::new(Path::new("kv_store.bin"))?;

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
