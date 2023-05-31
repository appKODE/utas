use anyhow::{anyhow, Ok, Result};
use std::{
    collections::{hash_map, HashMap},
    io::Write,
    path::Path,
};

use std::fs;

use crate::parse::{File, Key, LocalizedString, Section};

#[derive(PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Clone)]
pub struct Locale {
    value: String,
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone)]
pub struct StrLines {
    value: Vec<String>,
}

pub struct GenResult {
    value: HashMap<Locale, StrLines>,
}

impl GenResult {
    pub fn write(&self, dir: impl AsRef<Path>, file_name: &str) -> Result<()> {
        for (locale, lines) in &self.value {
            let subpath = dir.as_ref().join(format!("values-{}", locale.value));
            if !subpath.is_dir() {
                fs::create_dir(&subpath)?;
            }
            let filepath = subpath.join(format!("{}.xml", file_name));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(filepath)?;
            file.write("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".as_bytes())?;
            file.write("\n".as_bytes())?;
            file.write("<resources>\n".as_bytes())?;
            for line in &lines.value {
                let spaced = format!("  {}\n", line);
                file.write(spaced.as_bytes())?;
            }
            file.write("</resources>\n".as_bytes())?;
        }
        Ok(())
    }
}

pub fn generate(source: &File) -> Result<GenResult> {
    if source.sections.len() > 1 {
        panic!("Expected only one section currenlty")
    };

    let Some(keys) = source.sections.first().map(|section| &section.keys) else {
        return Err(anyhow!("Expected at least one section"))
    };

    let mut result: HashMap<Locale, StrLines> = HashMap::new();
    for key in keys {
        let str_name = &key.name;
        for str in &key.localizations {
            let code = Locale {
                value: str.language_code.clone(),
            };
            let value = generate_str_value(str_name, &str.value);
            let mut current = match result.remove(&code) {
                Some(current) => current.value,
                None => Vec::with_capacity(keys.len()),
            };
            current.push(value);
            result.insert(code, StrLines { value: current });
        }
    }

    Ok(GenResult { value: result })
}

pub fn generate_str_value(str_name: &String, str_value: &str) -> String {
    let open_tag = format!("<string name=\"{}\">", str_name);
    let close_tag = "</string>";
    let mut value = String::from(open_tag);
    value.push_str(str_value);
    value.push_str(close_tag);
    value
}

// -----------------------------  test tools ------------------------------
fn plain_str(lang: &str, txt: &str) -> LocalizedString {
    LocalizedString {
        language_code: lang.to_string(),
        value: txt.to_string(),
    }
}

fn key(name: &str, localizations: Vec<LocalizedString>) -> Key {
    Key {
        name: name.to_string(),
        localizations: localizations,
    }
}

fn sorted_strings(input: GenResult) -> Vec<(Locale, StrLines)> {
    let mut result = Vec::with_capacity(input.value.len());
    let mut keys: Vec<&Locale> = input.value.keys().collect();
    keys.sort();
    for key in keys {
        result.push((key.clone(), input.value.get(&key).unwrap().clone()))
    }
    result
}

// ------------------------------- tests -----------------------------------
#[test]
fn generate_1_lang_1_str() -> Result<()> {
    let localizations_kek = vec![plain_str("ru", "Кек")];
    let keys = vec![key("kek", localizations_kek)];
    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([(
        Locale {
            value: "ru".to_string(),
        },
        StrLines {
            value: vec!["<string name=\"kek\">Кек</string>".to_string()],
        },
    )]);

    let expected = GenResult { value: map };

    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));

    Ok(())
}

#[test]
fn generate_1_lang_2_str() -> Result<()> {
    let localizations_kek = vec![plain_str("ru", "Кек")];
    let localizations_lil = vec![plain_str("ru", "Лил")];

    let keys = vec![key("kek", localizations_kek), key("lil", localizations_lil)];

    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([(
        Locale {
            value: "ru".to_string(),
        },
        StrLines {
            value: vec![
                "<string name=\"kek\">Кек</string>".to_string(),
                "<string name=\"lil\">Лил</string>".to_string(),
            ],
        },
    )]);

    let expected = GenResult { value: map };

    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));

    Ok(())
}

#[test]
fn generate_3_lang_2_str() -> Result<()> {
    let localizations_find = vec![plain_str("ru", "Найти"), plain_str("en", "Find")];
    let localizations_search = vec![
        plain_str("ru", "Поиск"),
        plain_str("mn", "Хайх"),
        plain_str("en", "Search"),
    ];
    let keys = vec![
        Key {
            name: "find".to_string(),
            localizations: localizations_find,
        },
        Key {
            name: "search".to_string(),
            localizations: localizations_search,
        },
    ];
    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([
        (
            Locale {
                value: "ru".to_string(),
            },
            StrLines {
                value: vec![
                    "<string name=\"find\">Найти</string>".to_string(),
                    "<string name=\"search\">Поиск</string>".to_string(),
                ],
            },
        ),
        (
            Locale {
                value: "en".to_string(),
            },
            StrLines {
                value: vec![
                    "<string name=\"find\">Find</string>".to_string(),
                    "<string name=\"search\">Search</string>".to_string(),
                ],
            },
        ),
        (
            Locale {
                value: "mn".to_string(),
            },
            StrLines {
                value: vec!["<string name=\"search\">Хайх</string>".to_string()],
            },
        ),
    ]);

    let expected = GenResult { value: map };

    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));

    Ok(())
}

#[test]
fn generate_1_lang_1_str_2_placeholders() -> Result<()> {
    let localizations_add = vec![LocalizedString {
        language_code: "mn".to_string(),
        value: "%1$s нэмэх %2$d".to_string(),
    }];
    let keys = vec![key("add", localizations_add)];
    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([(
        Locale {
            value: "mn".to_string(),
        },
        StrLines {
            value: vec!["<string name=\"add\">%1$s нэмэх %2$d</string>".to_string()],
        },
    )]);

    let expected = GenResult { value: map };

    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));

    Ok(())
}

#[test]
fn generate_error_if_empty_sections() -> Result<()> {
    let source = File { sections: vec![] };

    let actual = generate(&source);
    assert!(actual.is_err());

    Ok(())
}
