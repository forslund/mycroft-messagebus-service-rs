use std::fs::read_to_string;
use std::path::Path;
use std::string::String;
use std::io;

use serde_json::{Result, Value};

fn read_config(filename: &str) -> io::Result<String> {
    let path = Path::new(&filename);
    let s = read_to_string(path)?;
    let mut ret_val = String::new();
    for line in s.split('\n') {
        if !line.trim().starts_with("//") {
            ret_val.push_str(line);
            ret_val.push_str("\n");
        }
    }
    Ok(ret_val)
}


pub fn load() -> Result<Value> {
    let read_data = read_config("./mycroft.conf").unwrap();
    let config: Value = serde_json::from_str(read_data.as_str())?;
    Ok(config)
}
