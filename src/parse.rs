use configparser::ini::{Ini, IniDefault};
use const_format::concatcp;
use indexmap::{map::Entry, IndexMap};
use lazy_static::lazy_static;
use regex::{Captures, Match, Regex};
use std::collections::HashSet;
use std::fmt::Error;
use std::fs::File as FsFile;
use std::io::{self, BufReader, Write};
use std::io::{BufRead, BufWriter};
use std::{borrow::Cow, fmt::format, path::Path};
use tempfile::NamedTempFile;

// Taken from
// https://developer.android.com/guide/topics/resources/string-resource.html#StylingWithHTML
const ANDROID_SUPPORTED_TAGS: &'static [&'static str] = &[
    "annotation",
    "a",
    "i",
    "cite",
    "dfn",
    "b",
    "em",
    "big",
    "small",
    "font",
    "tt",
    "s",
    "strike",
    "del",
    "u",
    "sup",
    "sub",
    "ul",
    "li",
    "br",
    "div",
    "span",
    "p",
];

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
    pub value: StringValue,
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone)]
pub enum StringValue {
    Single(String),
    Plural { quantities: Vec<PluralValue> },
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone)]
pub struct PluralValue {
    /// quantity can be: "zero", "one", "two", "few", "many", and "other"
    pub quantity: String,
    pub text: String,
}

pub fn parse<T: AsRef<Path>>(path: T) -> Result<File, String> {
    let mut default = IniDefault::default();
    default.case_sensitive = true;
    default.delimiters = vec!['='];
    default.comment_symbols = vec!['#'];
    let mut config = Ini::new_from_defaults(default);

    // See NOTE_DEDUPLICATING_KEYS
    let temp_file =
        NamedTempFile::new().map_err(|_| "failed to create temporary file".to_string())?;
    dedup_keys(&path, &temp_file).map_err(|error| (error.to_string() + " failed to dedup keys").to_string())?;
    let map = config.load(temp_file)?;

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

// TODO remove this function and write a custom parser
// See NOTE_DEDUPLICATING_KEYS
fn dedup_keys<T: AsRef<Path>, W: Write>(path: &T, temp_file: W) -> io::Result<()> {
    let f = FsFile::open(path)?;
    let f = BufReader::new(f);
    let mut of = BufWriter::new(temp_file);
    let mut keys: HashSet<String> = HashSet::new();

    for line in f.lines() {
        let l = line?;
        let maybe_key = l.trim();
        let out_line: String;
        if keys.iter().any(|x| x == maybe_key) {
            out_line = format!(
                "[{}{}]\n",
                maybe_key.trim_matches(|c| c == '[' || c == ']'),
                DEDUP_SUFFIX
            );
        } else {
            if maybe_key.starts_with('[') && !maybe_key.starts_with("[[") {
                keys.insert(maybe_key.to_string());
            }
            out_line = format!("{}\n", maybe_key);
        }
        of.write(out_line.as_bytes())?;
    }
    Ok(())
}

const DEDUP_SUFFIX: &str = "_dedup";

const PLACEHOLDER_FLAGS_WIDTH_PRECISION_LENGTH: &str =
    r"([-+0#,])?(\d+|\*)?(\.(\d+|\*))?(hh?|ll?|L|z|j|t|q)?";
const PLACEHOLDER_PARAMETER_FLAGS_WIDTH_PRECISION_LENGTH: &str =
    concatcp!(r"(\d+\$)?", PLACEHOLDER_FLAGS_WIDTH_PRECISION_LENGTH);
const PLACEHOLDER_TYPES: &str = "[diufFeEgGxXoscpaA@]";
const PLACEHOLDER_REGEX: &str = concatcp!(
    "%",
    PLACEHOLDER_PARAMETER_FLAGS_WIDTH_PRECISION_LENGTH,
    PLACEHOLDER_TYPES
);
const NON_NUMBERED_PLACEHOLDER_REGEX: &str = concatcp!(
    "%(",
    PLACEHOLDER_FLAGS_WIDTH_PRECISION_LENGTH,
    PLACEHOLDER_TYPES,
    ")"
);
const SINGLE_PERCENT_REGEX: &str = r"([^%][%][^%]|[^%][%]$|^[%]$|^[%][^%])";

fn key_from_locale_value_map(
    name: String,
    raw_localizations: IndexMap<String, Option<String>>,
) -> Result<Key, String> {
    if raw_localizations.keys().any(|l| l.contains(':')) {
        key_from_locale_plural_value_map(
            name.strip_suffix(DEDUP_SUFFIX).unwrap_or(&name),
            raw_localizations,
        )
    } else {
        key_from_locale_single_value_map(
            name.strip_suffix(DEDUP_SUFFIX).unwrap_or(&name),
            raw_localizations,
        )
    }
}

fn key_from_locale_single_value_map(
    name: &str,
    raw_localizations: IndexMap<String, Option<String>>,
) -> Result<Key, String> {
    let mut localizations: Vec<LocalizedString> = Vec::with_capacity(raw_localizations.len());
    for (locale_name, string_value_opt) in raw_localizations {
        if locale_name == "comment" || locale_name == "tags" {
            continue;
        }
        let Some(string_value) = string_value_opt else {
            println!("skipped key \"{}\" because it's empty", locale_name);
            continue;
        };
        let loc_str = LocalizedString {
            language_code: locale_name,
            value: StringValue::Single(parse_localized_string_value(string_value)?),
        };
        localizations.push(loc_str)
    }
    let key = Key {
        name: name.to_string(),
        localizations,
    };
    Ok(key)
}

fn key_from_locale_plural_value_map(
    name: &str,
    raw_localizations: IndexMap<String, Option<String>>,
) -> Result<Key, String> {
    let mut localizations: IndexMap<String, LocalizedString> =
        IndexMap::with_capacity(raw_localizations.len());
    for (locale_name_and_quantity, string_value_opt) in raw_localizations {
        let Some(string_value) = string_value_opt else {
            println!("skipped key \"{}\" for \"{}\" because it's empty", locale_name_and_quantity, name);
            continue;
        };
        let (locale_name, quantity) = locale_name_and_quantity
            .split_once(':')
            .unwrap_or_else(|| (&locale_name_and_quantity, "other"));
        let entry = localizations
            .entry(locale_name.to_string())
            .or_insert(LocalizedString {
                language_code: locale_name.to_string(),
                value: StringValue::Plural {
                    quantities: Vec::new(),
                },
            });
        let loc_str_value = &mut entry.value;
        let StringValue::Plural { quantities } = loc_str_value else {
            continue;
        };
        quantities.push(PluralValue {
            quantity: quantity.to_string(),
            text: parse_localized_string_value(string_value)?,
        });
    }
    let key = Key {
        name: name.to_string(),
        localizations: localizations.into_iter().map(|(_, value)| value).collect(),
    };
    Ok(key)
}

fn parse_localized_string_value(raw_value: String) -> Result<String, String> {
    lazy_static! {
        static ref PLACEHOLDER_REGEX_RE: Regex = Regex::new(PLACEHOLDER_REGEX).unwrap();
    }
    let mut value = raw_value;
    value = maybe_escape_characters(&value).to_string();
    value = maybe_replace_single_percent_with_double_percent(&value).to_string();
    if !PLACEHOLDER_REGEX_RE.is_match(&value) {
        return Ok(value);
    }
    value = convert_twine_string_placeholder(&value).to_string();
    value = maybe_add_positional_numbers(&value).to_string();
    Ok(value)
}

fn convert_twine_string_placeholder(raw_value: &str) -> Cow<str> {
    lazy_static! {
        static ref TWINE_STRING_REPLACE_REGEX: Regex = Regex::new(
            format!(
                r"%({})@",
                PLACEHOLDER_PARAMETER_FLAGS_WIDTH_PRECISION_LENGTH
            )
            .as_str()
        )
        .unwrap();
    }
    // TODO @dz @Parse avoid allocating new string if there's no match
    TWINE_STRING_REPLACE_REGEX.replace_all(&raw_value, r"%${1}s")
}

fn maybe_add_positional_numbers(input: &str) -> Cow<str> {
    lazy_static! {
        static ref NON_NUMBERED_PLACEHOLDER_REGEX_RE: Regex =
            Regex::new(NON_NUMBERED_PLACEHOLDER_REGEX).unwrap();
    }
    let non_numbered_count = NON_NUMBERED_PLACEHOLDER_REGEX_RE.find_iter(&input).count();
    if non_numbered_count <= 1 {
        return Cow::from(input);
    }
    let mut i = 0;
    NON_NUMBERED_PLACEHOLDER_REGEX_RE.replace_all(&input, |caps: &Captures| {
        i += 1;
        format!("%{}${}", i, &caps[1])
    })
}

fn maybe_replace_single_percent_with_double_percent(input: &str) -> Cow<str> {
    lazy_static! {
        static ref SINGLE_PERCENT_REGEX_RE: Regex = Regex::new(SINGLE_PERCENT_REGEX).unwrap();
        static ref PLACEHOLDER_REGEX_RE: Regex = Regex::new(PLACEHOLDER_REGEX).unwrap();
    }
    // Regex crate doesn't support negative lookahead which is used in
    // twine/placholder.rb for this case, so something else is invented here.
    // - use two Regexes: r1 = SINGLE_PERCENT_REGEX, r2 = PLACEHOLDER_REGEX
    // - iterate the matches of r1 and use r2.find_at(match) == match.start
    //   to see if this is a placholder-match
    // - if it is not a placeholder match, then it is a percent match,
    SINGLE_PERCENT_REGEX_RE.replace_all(input, |caps: &Captures| {
        let whole_match = caps.get(0).unwrap();
        // NOTE "percent match" can have first character not exactly being "%", for example
        // for "100% hello" it will be "% ".
        // So additional index adjustement is needed to correctly compare with "placeholder match" start
        let start = percent_start(&whole_match);
        let is_placeholder =
            matches!(PLACEHOLDER_REGEX_RE.find_at(input, start), Some(m) if m.start() == start);
        if is_placeholder {
            whole_match.as_str().to_string()
        } else {
            whole_match.as_str().replace('%', "%%")
        }
    })
}

fn percent_start(m: &Match) -> usize {
    m.start() + m.as_str().find('%').unwrap()
}

fn maybe_escape_characters(input: &str) -> Cow<str> {
    let needs_escaping =
        input.contains("&") || input.contains("<") || input.contains("'") || input.contains("\"");
    if needs_escaping {
        if ANDROID_SUPPORTED_TAGS
            .iter()
            .any(|tag| input.contains(&format!("<{tag}")))
        {
            escape_input_with_html_tags(input)
        } else {
            // fast path
            Cow::Owned(escape_with_no_html_tags(input))
        }
    } else {
        Cow::Borrowed(input)
    }
}

fn escape_with_no_html_tags(input: &str) -> String {
    return input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace("'", "\\'")
        .replace("\"", "\\\"");
}

fn escape_input_with_html_tags(input: &str) -> Cow<str> {
    // contains [start,end) indexes of tag regions
    let mut tag_regions: Vec<(usize, usize)> = Vec::new();
    for tag in ANDROID_SUPPORTED_TAGS {
        let mut start = 0;
        while start < input.len() {
            let Some(s) = input[start..].find(&format!("<{tag}")) else {
                break;
            };
            let abs_start = start + s;
            let Some(e) = input[abs_start + tag.len() + 1..].find(&format!("{tag}>")) else {
                break;
            };
            // "annotation" + ">"
            let abs_end = (abs_start + tag.len() + 1) + e + (tag.len() + 1);
            tag_regions.push((abs_start, abs_end));
            start = abs_end;
        }
    }
    if tag_regions.is_empty() {
        return Cow::Borrowed(input);
    }
    let mut result = String::new();
    if tag_regions.len() == 1 {
        let region = tag_regions[0];
        result.push_str(&escape_with_no_html_tags(&input[0..region.0]));
        result.push_str(&input[region.0..region.1]);
        result.push_str(&escape_with_no_html_tags(&input[region.1..]))
    } else {
        tag_regions.sort_by_key(|r| r.0);
        // fully escape parts:
        // - before the first tag
        // - between tags
        // - after last tag
        result.push_str(&escape_with_no_html_tags(&input[0..tag_regions[0].0]));
        result.push_str(&input[tag_regions[0].0..tag_regions[0].1]);
        for trs in tag_regions.windows(2) {
            result.push_str(&escape_with_no_html_tags(&input[trs[0].1..trs[1].0]));
            result.push_str(&input[trs[1].0..trs[1].1]);
        }
        result.push_str(&escape_with_no_html_tags(
            &input[tag_regions[tag_regions.len() - 1].1..],
        ));
    }
    return Cow::Owned(result);
}

#[test]
fn parses_simple_string() {
    let input = "Lorem ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "Lorem ipsum".to_string());
}

#[test]
fn parses_single_placeholder() {
    let input = "Lorem %d ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "Lorem %d ipsum",);
}

#[test]
fn parses_single_string_placeholder() {
    let input = "Lorem %@ ipsum".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "Lorem %s ipsum".to_string(),);
}

#[test]
fn parses_multiple_placeholders() {
    let input = "Lorem %@ ipsum %.2f sir %,d amet %%".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "Lorem %1$s ipsum %2$.2f sir %3$,d amet %%");
}

#[test]
fn parses_multiple_placeholders_keeping_order_if_present() {
    let input = "Lorem %3$@ ipsum %1$.2f sir %2$,d amet".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "Lorem %3$s ipsum %1$.2f sir %2$,d amet",);
}

#[test]
fn parses_html_tags_and_related_characters_with_proper_escaping() {
    for tag in ANDROID_SUPPORTED_TAGS {
        let input = format!(
            "У нас было <{tag}>38</{tag}> попугаев в <{tag} link=\"hello\">чистой</{tag}> \"упаковке\", на <unsupported>которой</unsupported> было указано: 38 < 89 && 88 >= 55",
        );
        let result = parse_localized_string_value(input).unwrap();
        assert_eq!(
            result,
            format!("У нас было <{tag}>38</{tag}> попугаев в <{tag} link=\"hello\">чистой</{tag}> \\\"упаковке\\\", на &lt;unsupported>которой&lt;/unsupported> было указано: 38 &lt; 89 &amp;&amp; 88 >= 55")
        )
    }
}

#[test]
fn parses_html_tags_and_related_characters_with_proper_escaping_different_tags() {
    let input = "У нас было <b>38</b> попугаев в <i>чистой</i> упаковке".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        "У нас было <b>38</b> попугаев в <i>чистой</i> упаковке",
    )
}

#[test]
fn parses_html_tags_and_related_characters_with_proper_escaping_only_tag() {
    let input = "<b>вот ведь</b>".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, "<b>вот ведь</b>",)
}

#[test]
fn parses_html_tags_and_related_characters_with_proper_escaping_one_tag() {
    let input = "Неожиданный амперсанд &, меньше < и кавычки \" и одинарные ' <a href=\"hello\">вот ведь</a>".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        "Неожиданный амперсанд &amp;, меньше &lt; и кавычки \\\" и одинарные \\' <a href=\"hello\">вот ведь</a>",
    )
}

#[test]
fn replaces_percent_with_double_percent() {
    let input =
        "100% Lorem %@ ipsum %.2f 20% sir %d amet 8% and %% untouched, ending with 42%".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        "100%% Lorem %1$s ipsum %2$.2f 20%% sir %3$d amet 8%% and %% untouched, ending with 42%%"
    );
}

#[test]
fn parses_single_quotes_with_proper_escaping() {
    let input = "Я очень люблю одинарные ' кавычки '".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, r"Я очень люблю одинарные \' кавычки \'");
}

#[test]
fn parses_double_quotes_with_proper_escaping() {
    let input = r#"Я очень люблю двойные " кавычки ""#.to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(result, r#"Я очень люблю двойные \" кавычки \""#);
}

#[test]
fn replaces_percent_with_double_percent_wihout_placeholders() {
    let input = "% of 100% Lorem ipsum amet 8% and %% untouched, ending with 42%".to_string();
    let result = parse_localized_string_value(input).unwrap();
    assert_eq!(
        result,
        "%% of 100%% Lorem ipsum amet 8%% and %% untouched, ending with 42%%"
    );
}

#[test]
fn parses_plural_form_keys() {
    let mut input = IndexMap::new();
    input.insert(
        "en:one".to_string(),
        Some("%d ruble %d bear and 1 vodka".to_string()),
    );
    input.insert(
        "en:many".to_string(),
        Some("%d rubles %d bears and 1 vodka".to_string()),
    );
    input.insert(
        "ru:one".to_string(),
        Some("%d рубль %d медведь и 1 водка".to_string()),
    );
    input.insert(
        "ru:zero".to_string(),
        Some("нет рублей нет медведей и 1 водка".to_string()),
    );
    input.insert(
        "ru:other".to_string(),
        Some("много рублей много медведей и 2 водки".to_string()),
    );
    let result = key_from_locale_value_map("receipt_example".to_string(), input).unwrap();
    let loc = result.localizations;

    assert_eq!(loc.len(), 2);
    assert_eq!(loc[0].language_code, "en".to_string());
    match &loc[0].value {
        StringValue::Plural { quantities } => {
            assert_eq!(
                quantities[0],
                PluralValue {
                    quantity: "one".to_string(),
                    text: "%1$d ruble %2$d bear and 1 vodka".to_string()
                }
            );
            assert_eq!(
                quantities[1],
                PluralValue {
                    quantity: "many".to_string(),
                    text: "%1$d rubles %2$d bears and 1 vodka".to_string()
                }
            )
        }
        StringValue::Single(_) => panic!("expected plural value"),
    }
    assert_eq!(loc[1].language_code, "ru".to_string());
    match &loc[1].value {
        StringValue::Plural { quantities } => {
            assert_eq!(
                quantities[0],
                PluralValue {
                    quantity: "one".to_string(),
                    text: "%1$d рубль %2$d медведь и 1 водка".to_string()
                }
            );
            assert_eq!(
                quantities[1],
                PluralValue {
                    quantity: "zero".to_string(),
                    text: "нет рублей нет медведей и 1 водка".to_string()
                }
            );
            assert_eq!(
                quantities[2],
                PluralValue {
                    quantity: "other".to_string(),
                    text: "много рублей много медведей и 2 водки".to_string()
                }
            )
        }
        StringValue::Single(_) => panic!("expected plural value"),
    }
}

#[test]
fn parses_plural_keys_when_some_locales_miss_quantity() {
    let mut input = IndexMap::new();
    input.insert(
        "en:one".to_string(),
        Some("%d ruble %d bear and 1 vodka".to_string()),
    );
    input.insert(
        "en:many".to_string(),
        Some("%d rubles %d bears and 1 vodka".to_string()),
    );
    input.insert(
        "ru".to_string(),
        Some("%d рубль %d медведь и 1 водка".to_string()),
    );
    input.insert("uz".to_string(), Some("оглы углы %d маглы".to_string()));
    let result = key_from_locale_value_map("receipt_example".to_string(), input).unwrap();
    let loc = result.localizations;

    assert_eq!(loc.len(), 3);
    assert_eq!(loc[0].language_code, "en".to_string());
    match &loc[0].value {
        StringValue::Plural { quantities } => {
            assert_eq!(
                quantities[0],
                PluralValue {
                    quantity: "one".to_string(),
                    text: "%1$d ruble %2$d bear and 1 vodka".to_string()
                }
            );
            assert_eq!(
                quantities[1],
                PluralValue {
                    quantity: "many".to_string(),
                    text: "%1$d rubles %2$d bears and 1 vodka".to_string()
                }
            )
        }
        StringValue::Single(_) => panic!("expected plural value"),
    }
    assert_eq!(loc[1].language_code, "ru".to_string());
    match &loc[1].value {
        StringValue::Plural { quantities } => {
            assert_eq!(
                quantities[0],
                PluralValue {
                    quantity: "other".to_string(),
                    text: "%1$d рубль %2$d медведь и 1 водка".to_string()
                }
            );
        }
        StringValue::Single(_) => panic!("expected plural value"),
    }
    assert_eq!(loc[2].language_code, "uz".to_string());
    match &loc[2].value {
        StringValue::Plural { quantities } => {
            assert_eq!(
                quantities[0],
                PluralValue {
                    quantity: "other".to_string(),
                    text: "оглы углы %d маглы".to_string()
                }
            );
        }
        StringValue::Single(_) => panic!("expected plural value"),
    }
}

// NOTE_DEDUPLICATING_KEYS
// Twine format allows duplicate keys, for example there could be a plurals
// string and a regular string with the same key name.
// But configparser-rs library which is currently used for parsing, would
// merge these keys in one Map entry which makes it impossible for us
// to distinguish one localized key from another.
//
// Therefore we currently do *a lot* of extra work by forcefully copying
// each file and deduplicating keys on the fly.
// This copying is done even if no duplicates exist.
//
// The correct way would be to ditch configparser-rs (which is made for INI files),
// and instead parse by ourselves: walk txt file line-by-line and as each key
// is read, produce StringValue from it (stream-like parsing)
