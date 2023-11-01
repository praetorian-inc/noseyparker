use anyhow::Result;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load a value from a YAML file.
pub fn load_yaml_file<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T> {
    let path = path.as_ref();
    let infile = File::open(path)?;
    let reader = BufReader::new(infile);
    let result = serde_yaml::from_reader(reader)?;
    Ok(result)
}
