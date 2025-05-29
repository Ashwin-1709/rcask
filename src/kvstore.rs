use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::vec;

/// A single key-value store that persists data to a file.
pub struct KVStore {
    index: HashMap<String, u64>,
    file: File,
    pub path: String,
}

impl KVStore {
    /// Creates a new KVStore instance.
    /// If the file exists, it will open it and load the existing index.
    /// If the file does not exist, it will create a new one.
    pub fn new(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .expect("failed to open keystore file");

        let mut store = KVStore {
            index: HashMap::new(),
            file: file,
            path: path.to_string_lossy().to_string(),
        };

        store.load()?;

        return Ok(store);
    }

    pub fn get_all_key_values(&mut self) -> io::Result<HashMap<String, Vec<u8>>> {
        let mut entries = HashMap::new();
        // Clone keys to avoid borrowing issues while calling get_value_bytes
        let keys: Vec<String> = self.index.keys().cloned().collect();

        for key in keys {
            // We read the value for each key using its offset.
            // This ensures we get the *latest* value according to the index.
            if let Some(value_bytes) = self.get_value_bytes(&key)? {
                entries.insert(key, value_bytes);
            }
            // We ignore keys that might exist in the index but fail to read,
            // though this indicates potential issues.
        }
        Ok(entries)
    }

    /// Rebuilds the in-memory index by reading through the entire file.
    /// This is called when the KVStore is initialized to restore state.
    pub fn load(&mut self) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;

        loop {
            let offset = self.file.seek(SeekFrom::Current(0))?;
            match self.read() {
                Ok(key) => {
                    let key_str = String::from_utf8(key)
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
                    self.index.insert(key_str, offset);
                }
                Err(_) => {
                    break;
                }
            }

            // Read value to move the cursor forward
            match self.read() {
                Ok(_) => {}
                Err(_) => {
                    break;
                }
            }
        }
        return Ok(());
    }

    /// Reads a string from the file.
    /// It first reads the length of the string (u64),
    /// then reads the string bytes based on that length.
    fn read(&mut self) -> Result<Vec<u8>, io::Error> {
        // Read the length of the value.
        let mut length_bytes = [0; 8];
        if self.file.read_exact(&mut length_bytes).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Failed to read length bytes",
            ));
        }

        // Read the value bytes based on the length.
        let mut value_bytes = vec![0; usize::from_le_bytes(length_bytes) as usize];
        if self.file.read_exact(&mut value_bytes).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Failed to read value bytes",
            ));
        }
        return Ok(value_bytes.to_vec());
    }

    /// Helper function to read a length-prefixed byte array from the file.
    /// It first reads a u64 length, then reads that many bytes.
    /// This is used for reading both keys and values.
    fn read_bytes(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut length_buffer = [0; 8];

        // Read the length of the upcoming data.
        self.file.read_exact(&mut length_buffer)?;

        let length = usize::from_le_bytes(length_buffer);

        // Read the actual data based on the length.
        let mut data_bytes = vec![0; length];
        self.file.read_exact(&mut data_bytes)?;

        return Ok(data_bytes);
    }

    /// Sets a key-value pair in the store.
    /// The value is now a generic byte slice.
    ///
    /// The data is written in the format:
    /// [key_length: u64] [key_bytes] [value_length: u64] [value_bytes]
    /// The offset of the key (start of its entry) is then stored in the in-memory index.
    pub fn set<T: AsRef<[u8]>, U: AsRef<[u8]>>(&mut self, key: T, value: U) -> io::Result<()> {
        // Get the current file offset.
        let offset = self.file.seek(SeekFrom::Current(0))?;

        // Byte slices for key and value.
        let key_bytes = key.as_ref();
        let value_bytes = value.as_ref();

        // Write key length (u64)
        // Helper closure to retry write_all up to 3 times
        let mut retry_write = |buf: &[u8]| -> io::Result<()> {
            let mut attempts = 0;
            loop {
                match self.file.write_all(buf) {
                    Ok(_) => return Ok(()),
                    Err(_) if attempts < 2 => {
                        attempts += 1;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
        };

        retry_write(&(key_bytes.len() as u64).to_le_bytes())?;
        retry_write(key_bytes)?;
        retry_write(&(value_bytes.len() as u64).to_le_bytes())?;
        retry_write(value_bytes)?;

        // Store the offset for the key in the index
        self.index
            .insert(String::from_utf8_lossy(key_bytes).to_string(), offset);
        Ok(())
    }

    /// Retrieves the value associated with a given key in string format.
    ///
    /// It first retrieves the value bytes using the `get` method,
    /// then attempts to convert those bytes to a String.
    pub fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        return match self.get_value_bytes(key) {
            Ok(Some(value_bytes)) => {
                // Convert the value bytes to a String.
                self.to_string(value_bytes)
            }
            Ok(None) => Ok(None), // Key not found
            Err(e) => Err(e),     // Propagate other I/O errors
        };
    }

    /// Retrieves the value associated with a given key.
    /// The returned value is now a generic `Vec<u8>`.
    ///
    /// It uses the stored offset to seek directly to the key's position in the file,
    /// then reads the key (to advance pointer) and finally the value bytes.
    fn get_value_bytes(&mut self, key: &str) -> io::Result<Option<Vec<u8>>> {
        // 1. Check if the key exists in the index.
        let &offset = match self.index.get(key) {
            Some(o) => o,
            None => return Ok(None), // Key not found in index
        };

        // Seek to the stored offset (start of the key-value entry).
        self.file.seek(SeekFrom::Start(offset))?;

        // 2. Read the key and validate it to ensure there is no data corruption.
        match self.read_bytes() {
            Ok(key_bytes) => {
                let key_str = String::from_utf8(key_bytes)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

                // Validate that the key matches the requested key.
                if key_str != key {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Data corruption: key mismatch",
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok(None); // Incomplete entry, return None
            }
            Err(e) => return Err(e), // Propagate other I/O errors
        }

        // 3. Read the value bytes.
        match self.read_bytes() {
            Ok(value_bytes) => Ok(Some(value_bytes)),
            // If EOF is reached *after* reading the key but before the value, it's an incomplete entry.
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e), // Propagate other I/O errors
        }
    }

    /// Converts a vector of bytes to a String.
    fn to_string(&self, bytes: Vec<u8>) -> io::Result<Option<String>> {
        return String::from_utf8(bytes)
            .map(Some)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err));
    }
}
