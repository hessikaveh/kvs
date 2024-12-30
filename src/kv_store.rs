use anyhow::{Context, Result};
use std::{collections::btree_map::BTreeMap, path::Path};

use crate::wal::{Commands, WriteAheadLog};

pub struct KvStore {
    inner: BTreeMap<String, String>,
    write_ahead_log: WriteAheadLog,
}

impl KvStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut db_tree = BTreeMap::new();
        let mut wal_file = WriteAheadLog::new(path.as_ref());
        for command in wal_file.iter(0) {
            match command {
                Commands::Set { key, value } => {
                    db_tree.insert(key.to_owned(), value.to_owned());
                }
                Commands::Rm { key } => {
                    db_tree.remove(&key.to_owned());
                }
                _ => {}
            }
        }
        Ok(Self {
            inner: db_tree,
            write_ahead_log: wal_file,
        })
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.inner.insert(key.to_owned(), value.to_owned());

        let set_command = Commands::Set {
            key: key.to_owned(),
            value: value.to_owned(),
        };
        self.write_ahead_log.append(set_command);

        Ok(())
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let get_command = Commands::Get {
            key: key.to_owned(),
        };
        self.write_ahead_log.append(get_command);
        Ok(self.inner.get(&key).cloned())
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        let _ = self.inner.remove(&key).context("Key not found")?;
        let rm_command = Commands::Rm {
            key: key.to_owned(),
        };
        self.write_ahead_log.append(rm_command);
        Ok(())
    }
}
