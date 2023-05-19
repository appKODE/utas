use configparser::ini::Ini;
use std::path::Path;

pub struct File {
    pub sections: Vec<Section>,
}

pub struct Section {
    pub keys: Vec<Key>,
}

pub struct Key {
    pub strings: Vec<LocalizedString>,
}

pub struct LocalizedString {
    pub language_code: String,
    pub value: Vec<Token>,
}

pub enum Token {
    Text(String),
    Placeholder(String),
}

pub fn parse<T: AsRef<Path>>(path: T) -> Result<File, String> {
    let mut config = Ini::new();
    let map = config.load(path)?;
    println!("{:?}", map);
    Err(String::from("not implemented"))
}
