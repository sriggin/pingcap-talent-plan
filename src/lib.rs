#[macro_use]
extern crate failure;
extern crate bincode;
extern crate byteorder;
extern crate serde;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, failure::Error>;

pub struct KvStore<'a> {
    path: &'a Path,
}

impl<'a> KvStore<'a> {
    pub fn open(path: &'a Path) -> Result<KvStore<'a>> {
        Ok(KvStore { path })
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.append(&Record::Set(key, value))
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        let log_path = self.current_log()?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(log_path)?;
        let mut records = self.read(&mut file)?;
        match records.remove(&key) {
            Option::Some(_) => {
                self.append(&Record::Rm(key))?;
                Ok(())
            }
            Option::None => Err(format_err!("Key not found")),
        }
    }

    pub fn get(&self, key: String) -> Result<Option<String>> {
        let log_path = self.current_log()?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(log_path)?;
        let records = self.read(&mut file)?;
        if let Option::Some(pos) = records.get(&key) {
            file.seek(SeekFrom::Start(*pos))?;
            let record: Record = bincode::deserialize_from(&mut file)?;
            if let Record::Set(_, value) = record {
                Ok(Option::Some(value))
            } else {
                Err(format_err!("Unexpected record: {:?}", record))
            }
        } else {
            Ok(Option::None)
        }
    }

    // ---

    fn current_log(&self) -> Result<PathBuf> {
        let mut path_buf = self.path.to_path_buf();
        path_buf.push("temp.log");
        Ok(path_buf)
    }

    fn append(&self, rec: &Record) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.current_log()?)?;
        bincode::serialize_into(file, rec)?;
        Ok(())
    }

    fn read(&self, mut file: &mut File) -> Result<HashMap<String, u64>> {
        let mut commands = HashMap::new();
        let file_length = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        loop {
            let cur = file.seek(SeekFrom::Current(0))?;
            if cur >= file_length {
                break;
            }
            let record: Record = bincode::deserialize_from(&mut file)?;
            match record {
                Record::Set(key, _) => commands.insert(key, cur),
                Record::Rm(key) => commands.remove(&key),
            };
        }
        Ok(commands)
    }
}

#[derive(Deserialize, Serialize, Debug)]
enum Record {
    Set(String, String),
    Rm(String),
}

use byteorder::{NativeEndian, ReadBytesExt};
use std::convert::TryInto;

/// Serialization format
/// --------------------
/// Word 1: [ ABBB ]     -- A = Type, BBB = Key length = KL
/// Word 2:              -- Value length = VL
/// Word 3-${CEIL(KL/4)} -- Key (Bytes 9-+KL)
/// -- POS=${CEIL(KL/4)} + 3
/// Word $POS-${CEIL(VL/4)} -- Value (Bytes ${POS*4}-+VL)
/// -----
/// Byte 1:   Type (Set | Rm)
/// Byte 2-4: Key Length = KL
/// Byte 5-8: Value Length = VL
/// Byte 9-${KL+8}: Key
/// Byte ${KL+9}-${KL+9+VL}: Value
impl Record {
    fn key(self) -> String {
        match self {
            Record::Set(key, _) => key,
            Record::Rm(key) => key,
        }
    }

    // this is all unused yay
    fn from_reader<S: Seek + Read>(reader: &mut S) -> Result<Record> {
        fn read_string<S: Seek + Read>(reader: &mut S, length: u32) -> Result<String> {
            let size = length.try_into().unwrap();
            let mut bytes = vec![0u8; size];
            reader.read_exact(&mut bytes)?;
            let str = std::str::from_utf8(&bytes)?;
            Ok(str.to_owned())
        }

        let mut word = [0u8; 4];
        reader.read_exact(&mut word)?;
        let header = reader.read_u32::<NativeEndian>()?;
        let record_type = header & 0xFF000000 >> 24;
        let key_length = header & 0x00FFFFFF;
        match record_type {
            1 => {
                let value_length = reader.read_u32::<NativeEndian>()?;
                let key = read_string(reader, key_length)?;
                let value = read_string(reader, value_length)?;
                Ok(Record::Set(key, value))
            }
            2 => {
                let key = read_string(reader, key_length)?;
                Ok(Record::Rm(key))
            }
            other => Err(format_err!("Invalid record type {}", other)),
        }
    }
}
