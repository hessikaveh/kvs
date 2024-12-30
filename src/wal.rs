use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

#[derive(Debug)]
struct AtomicLogPointer {
    offset: AtomicU64,   // The byte offset within the file
    sequence: AtomicU64, // A monotonically increasing number representing order of operations
}

impl AtomicLogPointer {
    fn new() -> Self {
        Self {
            offset: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
        }
    }

    fn advance(&self, bytes_written: u64) {
        self.offset.fetch_add(bytes_written, Ordering::SeqCst);
        self.sequence.fetch_add(1, Ordering::SeqCst);
    }

    fn current(&self) -> (u64, u64) {
        (
            self.offset.load(Ordering::SeqCst),
            self.sequence.load(Ordering::SeqCst),
        )
    }
}

pub struct WriteAheadLog {
    writer: BufWriter<std::fs::File>,
    reader: BufReader<std::fs::File>,
    pointer: Arc<AtomicLogPointer>,
}

impl WriteAheadLog {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.as_ref())
            .expect("Failed to open log file.");

        let reader = BufReader::new(file.try_clone().expect("Failed to clone file."));
        let mut writer = BufWriter::new(file);

        // Seek the writer to the end of the file to allow appending
        let end_offset = writer
            .seek(SeekFrom::End(0))
            .expect("Failed to seek to end");
        info!("Recovered WAL at offset: {}", end_offset);

        let pointer = Arc::new(AtomicLogPointer::new());
        pointer.offset.store(end_offset, Ordering::SeqCst);

        Self {
            writer,
            reader,
            pointer,
        }
    }

    // Append a command to the WAL
    pub fn append(&mut self, command: Commands) {
        // Serialize the command using MessagePack
        let serialized = encode::to_vec(&command).expect("Failed to serialize command");

        // Write serialized data to the log
        self.writer
            .write_all(&serialized)
            .expect("Failed to write to WAL");
        self.writer.flush().expect("Failed to flush writer");

        // Update the log pointer
        self.pointer.advance(serialized.len() as u64);
        info!(
            "Appended command: {:?}, Log Pointer: {:?}",
            command,
            self.pointer.current()
        );
    }

    /// Returns an iterator to read commands from the log
    pub fn iter(&mut self, start_offset: u64) -> CommandIterator {
        self.reader
            .seek(SeekFrom::Start(start_offset))
            .expect("Failed to seek to start offset");

        CommandIterator {
            reader: self.reader.by_ref(),
        }
    }
}

/// Iterator for reading commands from the WAL
pub struct CommandIterator<'a> {
    reader: &'a mut BufReader<std::fs::File>,
}

impl<'a> Iterator for CommandIterator<'a> {
    type Item = Commands;

    fn next(&mut self) -> Option<Self::Item> {
        match decode::from_read::<_, Commands>(&mut self.reader) {
            Ok(command) => Some(command),
            Err(_) => None, // End of log or invalid entry
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_write_and_read_log() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
        // Use a temporary file for testing
        let temp_file = "test_wal.log";

        // Create and initialize the WAL
        let mut wal = WriteAheadLog::new(temp_file);

        // Commands to test
        let commands = vec![
            Commands::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
            Commands::Get {
                key: "key1".to_string(),
            },
            Commands::Rm {
                key: "key1".to_string(),
            },
        ];

        // Append commands to the WAL
        for command in &commands {
            wal.append(command.clone());
        }

        // Read commands back using the iterator
        let read_commands: Vec<Commands> = wal.iter(0).collect();

        // Ensure commands read match the commands written
        assert_eq!(read_commands, commands);

        for (index, command) in wal.iter(0).enumerate() {
            info!("Read command {}: {:?}", index + 1, command);
        }
        // Clean up the test file
        std::fs::remove_file(temp_file).expect("Failed to clean up test file");
    }
}
