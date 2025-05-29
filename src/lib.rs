mod kvstore;
use std::fs;
use std::io::Result;
use std::path::Path;
use std::path::PathBuf;

/// RCask is a wrapper around the KVStore which manages the disk storage size does not exceed a limit.
/// This is done using a blocking compaction process which is fired after a certain number of writes
/// to the log file.
pub struct RCask {
    directory: String,
    pattern: String,
    max_writes: u64,
    store: kvstore::KVStore,
    writes: u64,
}

impl RCask {
    /// Creates a new RCask instance.
    /// It scans the specified directory for log files matching the given pattern.
    /// /// If no matching files are found, it creates a new log file with the specified pattern.
    /// The `max_writes` parameter specifies the maximum number of writes before compaction is triggered.
    pub fn init(directory: String, pattern: String, max_writes: u64) -> Result<Self> {
        fs::create_dir_all(&directory)?; // Ensure directory exists

        let logs = fs::read_dir(&directory)?;
        let mut paths = Vec::new();
        for file in logs {
            let file = file?;
            let path = file.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if file_name.starts_with(&pattern) && file_name.ends_with(".log") {
                paths.push(path);
            }
        }

        paths.sort();

        let store = if let Some(path) = paths.last() {
            kvstore::KVStore::new(&path)?
        } else {
            // Create the first segment (e.g., data.0.log) if none exist
            let initial_path = PathBuf::from(format!("{}/{}.0.log", directory, pattern));
            kvstore::KVStore::new(&initial_path)?
        };

        Ok(RCask {
            directory,
            pattern,
            max_writes,
            store,
            writes: 0,
        })
    }

    /// Creates a new RCask instance with a default `max_writes` of 10,000.
    /// This is a convenience method for initializing the store without specifying `max_writes`.
    pub fn new(directory: String, pattern: String) -> Result<Self> {
        return Self::init(directory, pattern, 10000);
    }

    /// Sets a key-value pair in the store.
    /// If the number of writes exceeds `max_writes`, it triggers a compaction process.
    pub fn set<T: AsRef<[u8]>, U: AsRef<[u8]>>(&mut self, key: T, value: U) -> Result<()> {
        return match self.store.set(key, value) {
            Ok(_) => {
                self.writes += 1;
                if self.writes >= self.max_writes {
                    self.compact()?;
                }
                Ok(())
            }
            Err(e) => {
                return Err(e);
            }
        };
    }

    /// Retrieves the value associated with a given key in string format.
    pub fn get(&mut self, key: &str) -> Result<Option<String>> {
        return self.store.get(key);
    }

    fn compact(&mut self) -> Result<()> {
        // 1. Get the path for the new (compacted) segment file.
        let next_segment = self.get_next_segment_path();
        let segment_path = PathBuf::from(&next_segment);

        let mut new_store = kvstore::KVStore::new(Path::new(&segment_path))?;

        // 2. Iterate over all keys in the current store and write them to the new store.
        for (key, value) in self.store.get_all_key_values()? {
            new_store.set(key, value)?;
        }

        // 3. Replace the current store with the new store.
        fs::remove_file(Path::new(&self.store.path))?;
        self.store = new_store;

        // 4. Reset the write count.
        self.writes = 0;

        return Ok(());
    }

    fn get_next_segment_path(&self) -> String {
        let Ok(logs) = fs::read_dir(&self.directory) else {
            // Should not happen if new() worked.
            return format!("{}/{}.0.log", self.directory, self.pattern);
        };

        let mut indices: Vec<u64> = logs
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if !path.is_file() {
                    return None;
                }

                let file_name = path.file_name()?.to_str()?;
                if !file_name.starts_with(&self.pattern) || !file_name.ends_with(".log") {
                    return None;
                }

                let stem = path.file_stem()?.to_str()?;
                stem.rsplit('.').next()?.parse::<u64>().ok()
            })
            .collect();

        indices.sort_unstable();
        let next_index = indices.last().map_or(0, |&x| x + 1);
        return format!("{}/{}.{}.log", self.directory, self.pattern, next_index);
    }
}
