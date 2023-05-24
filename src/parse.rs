use configparser::ini::Ini;
use indexmap::IndexMap;
use std::path::Path;

#[derive(Debug)]
pub struct File {
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Section {
    pub keys: Vec<Key>,
}

/// Represents a string resource key with its localizations
#[derive(Debug)]
pub struct Key {
    pub name: String,
    pub localizations: Vec<LocalizedString>,
}

#[derive(Debug)]
pub struct LocalizedString {
    pub language_code: String,
    pub value: Vec<Token>,
}

#[derive(Debug, PartialEq)]
pub enum Token {
    Text(String),
    Placeholder(String),
}

pub fn parse<T: AsRef<Path>>(path: T) -> Result<File, String> {
    let mut config = Ini::new();
    let map = config.load(path)?;
    // NOTE: twine has this structure
    // [[Section1]]
    // [subsection1]
    //   key1 = value1
    //   key2 = value2
    // [[Section2]]
    // [subsection1]
    //   key1 = value1
    //   key2 = value2
    // but configparser lib will ignore [[SectionX]] sections (see https://github.com/QEDK/configparser-rs/issues/37),
    // so here we will only see [subsection1, subsection2] returned by `config.sections()` and these will be
    // string resource keys.
    // We still will create a single "twine-section" struct in hopes of a future issue fix (seen above), then we'll
    // be able to group "subsections" in "twine-section".
    let mut section = Section {
        keys: Vec::with_capacity(map.len()),
    };
    // Parses
    // [login_screen_title]
    // en = Login
    // ru = Логин
    for (resource_key_name, localizations) in map {
        let key = key_from_locale_value_map(resource_key_name, localizations)?;
        section.keys.push(key);
    }
    Ok(File {
        // For now only supporting a single section, see the comment above
        sections: vec![section],
    })
}

fn key_from_locale_value_map(
    name: String,
    raw_localizations: IndexMap<String, Option<String>>,
) -> Result<Key, String> {
    let mut localizations: Vec<LocalizedString> = Vec::with_capacity(raw_localizations.len());
    for (locale_name, string_value_opt) in raw_localizations {
        let Some(string_value) = string_value_opt else {
            println!("skipped key \"{}\" because it's empty", locale_name);
            continue;
        };
        let loc_str = LocalizedString {
            language_code: locale_name,
            value: parse_localized_string_value(string_value)?,
        };
        localizations.push(loc_str)
    }
    let key = Key {
        name,
        localizations,
    };
    Ok(key)
}

fn parse_localized_string_value(raw_value: String) -> Result<Vec<Token>, String> {
    // TODO @dz actually parse
    Ok(vec![Token::Text(raw_value)])
}

#[test]
fn parses_simple_string() {
    let input = "Lorem ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, vec![Token::Text("Lorem ipsum".to_string())]);
}

#[test]
fn parses_single_placeholder() {
    let input = "Lorem %d ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Text("Lorem ".to_string()),
            Token::Placeholder("%d".to_string()),
            Token::Text(" ipsum".to_string()),
        ]
    );
}

#[test]
fn parses_single_string_placeholder() {
    let input = "Lorem %@ ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Text("Lorem ".to_string()),
            Token::Placeholder("%s".to_string()),
            Token::Text(" ipsum".to_string()),
        ]
    );
}

#[test]
fn parses_multiple_placeholders() {
    let input = "Lorem %@ ipsum %.2f sir %,d amet %%".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Text("Lorem ".to_string()),
            Token::Placeholder("%1$s".to_string()),
            Token::Text(" ipsum ".to_string()),
            Token::Placeholder("%2$.2f".to_string()),
            Token::Text(" sir ".to_string()),
            Token::Placeholder("%3$,d".to_string()),
            Token::Text(" amet %%".to_string()),
        ]
    );
}

#[test]
fn parses_multiple_placeholders_keeping_order_if_present() {
    let input = "Lorem %3$@ ipsum %1$.2f sir %2$,d amet".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Text("Lorem ".to_string()),
            Token::Placeholder("%3$s".to_string()),
            Token::Text(" ipsum ".to_string()),
            Token::Placeholder("%1$.2f".to_string()),
            Token::Text(" sir ".to_string()),
            Token::Placeholder("%2$,d".to_string()),
            Token::Text(" amet".to_string()),
        ]
    );
}
