use anyhow::{anyhow, Ok, Result};
use std::{collections::HashMap, io::Write, path::Path};

use std::fs;

use crate::parse::{File, Key, LocalizedString, PluralValue, Section, StringValue};

#[derive(PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Clone)]
pub struct Locale {
    value: String,
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone)]
pub struct StrLines {
    value: Vec<Line>,
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone)]
pub struct Line {
    name: String,
    value: StringValue,
}

impl Line {
    fn format(&self) -> Vec<String> {
        match &self.value {
            StringValue::Single(text) => vec![generate_str_value(&self.name, text)],
            StringValue::Plural { quantities } => generate_plural_value(&self.name, quantities),
        }
    }
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
                let formatted = line.format();
                for item in formatted {
                    file.write(format!("  {}\n", item).as_bytes())?;
                }
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
    let keys_len = keys.len();
    for key in keys {
        let str_name = &key.name;
        for str in &key.localizations {
            let code = Locale {
                value: str.language_code.clone(),
            };

            let current = &mut result
                .entry(code)
                .or_insert(StrLines {
                    value: Vec::with_capacity(keys_len),
                })
                .value;

            current.push(Line {
                name: str_name.clone(),
                value: str.value.clone(),
            })
        }
    }

    Ok(GenResult { value: result })
}

fn generate_str_value(str_name: &str, str_value: &str) -> String {
    String::from(format!(
        "<string name=\"{}\">{}</string>",
        str_name, str_value
    ))
}

fn generate_plural_value(str_name: &String, items: &Vec<PluralValue>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(items.len() + 2);
    result.push(format!("<plurals name=\"{}\">", str_name));

    for item in items {
        result.push(format!(
            "  <item quantity=\"{}\">{}</item>",
            item.quantity, item.text
        ));
    }
    result.push("</plurals>".to_string());
    result
}

// -----------------------------  test tools ------------------------------
fn plain_str(lang: &str, txt: &str) -> LocalizedString {
    LocalizedString {
        language_code: lang.to_string(),
        value: StringValue::Single(txt.to_string()),
    }
}

fn plurals(lang: &str, quantities: Vec<PluralValue>) -> LocalizedString {
    LocalizedString {
        language_code: lang.to_string(),
        value: StringValue::Plural { quantities },
    }
}

fn plural_val(quantity: &str, text: &str) -> PluralValue {
    PluralValue {
        quantity: quantity.to_string(),
        text: text.to_string(),
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

fn single(name: &str, text: &str) -> Line {
    return Line {
        name: name.to_string(),
        value: StringValue::Single(text.to_string()),
    };
}

fn plural(name: &str, items: Vec<PluralValue>) -> Line {
    return Line {
        name: name.to_string(),
        value: StringValue::Plural { quantities: items },
    };
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
            value: vec![single("kek", "Кек")],
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
            value: vec![single("kek", "Кек"), single("lil", "Лил")],
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
                value: vec![single("find", "Найти"), single("search", "Поиск")],
            },
        ),
        (
            Locale {
                value: "en".to_string(),
            },
            StrLines {
                value: vec![single("find", "Find"), single("search", "Search")],
            },
        ),
        (
            Locale {
                value: "mn".to_string(),
            },
            StrLines {
                value: vec![single("search", "Хайх")],
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
        value: StringValue::Single("%1$s нэмэх %2$d".to_string()),
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
            value: vec![single("add", "%1$s нэмэх %2$d")],
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

#[test]
fn generate_1_lang_1_simple_plural() -> Result<()> {
    let localizations_songs = vec![plurals("mn", vec![plural_val("other", "%d дуу")])];
    let keys = vec![Key {
        name: "songs".to_string(),
        localizations: localizations_songs,
    }];
    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([(
        Locale {
            value: "mn".to_string(),
        },
        StrLines {
            value: vec![plural(
                "songs",
                vec![PluralValue {
                    quantity: "other".to_string(),
                    text: "%d дуу".to_string(),
                }],
            )],
        },
    )]);
    let expected = GenResult { value: map };
    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));
    Ok(())
}

#[test]
fn generate_1_lang_1_str_1_plurals() -> Result<()> {
    let localizations_chicken = vec![plain_str("en", "Chicken")];
    let localizations_cows = vec![plurals(
        "en",
        vec![
            plural_val("one", "%d cow"),
            plural_val("two", "%d cows"),
            plural_val("other", "33 copy-on-writes"),
        ],
    )];
    let keys = vec![
        Key {
            name: "chicken".to_string(),
            localizations: localizations_chicken,
        },
        Key {
            name: "cows".to_string(),
            localizations: localizations_cows,
        },
    ];
    let source = File {
        sections: vec![Section { keys }],
    };
    let map = HashMap::from([(
        Locale {
            value: "en".to_string(),
        },
        StrLines {
            value: vec![
                single("chicken", "Chicken"),
                plural(
                    "cows",
                    vec![
                        PluralValue {
                            quantity: "one".to_string(),
                            text: "%d cow".to_string(),
                        },
                        PluralValue {
                            quantity: "two".to_string(),
                            text: "%d cows".to_string(),
                        },
                        PluralValue {
                            quantity: "other".to_string(),
                            text: "33 copy-on-writes".to_string(),
                        },
                    ],
                ),
            ],
        },
    )]);
    let expected = GenResult { value: map };

    let actual = generate(&source)?;
    assert_eq!(sorted_strings(expected), sorted_strings(actual));

    Ok(())
}
