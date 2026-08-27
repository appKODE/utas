# utas

`utas` is a command-line tool that converts
[Twine](https://github.com/mobiata/twine)-style localization files into
native string resources for **Android** and **iOS**.

## Name

- **Original spelling (Mongolian Cyrillic):** утас
- **Romanization:** utas
- **Pronunciation (IPA):** /ʊˈtʰas/, colloquially reduced to [ʊts] (Khalkha
  Mongolian tends to minimize or drop short unstressed vowels)

The name comes from the Mongolian word for "thread" / "string" (as of an
instrument) / "wire" — by extension also "telephone" or "phone line" (e.g.
гар утас, literally "hand wire", meaning "mobile phone"). Both of its
vowels (у, а) belong to the back ("hard") vowel harmony class in Mongolian,
so the word is pronounced firmly/hard throughout, without the palatalized,
"soft" articulation that front-vowel Mongolian words get.

It reads `.txt` files written in Twine's INI-like format and generates:

- **Android**: `values-<locale>/<name>.xml` resource files (`<string>` and
  `<plurals>` entries), with correct handling of region qualifiers
  (`en-GB` -> `values-en-rGB`), obsolete/legacy language codes (`he` -> `iw`,
  `id` -> `in`), and simplified/traditional Chinese script qualifiers.
- **iOS**: `<locale>.lproj/<Name>.strings` and `<Name>.stringsdict` files,
  with placeholder syntax converted from Twine (`%@`) to the platform's
  positional format (`%1$s`), and support for filling in missing
  translations from a default language.

Both platforms get correct escaping of special characters, HTML tag
preservation for supported markup, and printf-style placeholder
numbering/positioning.

## What it does

Given one or more Twine `.txt` source files structured as:

```
[[Src1]]
  [greeting]
    en = Hello %@
    ru = Привет %@

  [cows]
    en:one = %d cow
    en:other = %d cows
    ru:one = %d корова
    ru:other = %d коров
```

`utas` parses every locale/key pair, groups plural forms by quantity
(`zero`, `one`, `two`, `few`, `many`, `other`), and emits per-locale
resource files ready to drop into an Android or iOS project.

### CLI usage

```
utas <platform> <input_dir> <output_dir> [default_lang] [file_name]
```

| Argument       | Required | Description                                                                 |
|----------------|----------|-------------------------------------------------------------------------------|
| `platform`     | yes      | `android` or `ios`                                                            |
| `input_dir`    | yes      | Directory containing Twine `.txt` source files                                |
| `output_dir`   | yes      | Directory to write generated resources into                                   |
| `default_lang` | no       | Locale code used as the fallback for missing translations                     |
| `file_name`    | no       | iOS only — base name for the generated `.strings`/`.stringsdict` files (defaults to `Localizable`) |

Examples:

```bash
# Android: generate values*/strings.xml, falling back to "en" for missing keys
utas android ./twine ./app/src/main/res en

# iOS: generate en.lproj/Localizable.strings, ru.lproj/Localizable.strings, etc.
utas ios ./twine ./MyApp/Resources en
```

## Project structure

```
.
├── src/                  # utas binary crate
│   ├── main.rs           # CLI argument parsing and pipeline orchestration
│   ├── parse.rs          # Twine .txt -> internal File/Section/Key model
│   ├── android_gen.rs    # internal model -> Android values-*/*.xml
│   └── ios_gen.rs        # internal model -> iOS *.lproj/*.strings(dict)
├── crates/
│   └── file/              # small helper crate for comparing files/dirs,
│       └── src/            # used by integration tests to diff expected
│                            # vs. generated output
├── tests/
│   ├── test.rs            # integration tests: run the built binary against
│   │                        # fixtures and compare output byte-for-byte
│   └── cases/
│       ├── android/case*/  # input/ + output/ fixture pairs for Android
│       └── ios/case*/      # input/ + output/ fixture pairs for iOS
├── .github/workflows/
│   ├── push.yml            # build + test on every push/PR to master
│   ├── linux-release.yml   # build + publish a Linux release binary
│   ├── mac-release.yml     # build + publish a macOS release binary
│   └── win-release.yml     # build + publish a Windows release binary
├── Cargo.toml               # utas binary crate manifest
├── Cargo.lock
└── LICENSE
```

## Building and testing locally

```bash
cargo build --release
cargo test               # unit + integration tests for the utas crate
cargo test -p file       # tests for the file-comparison helper crate
```

## Releasing via GitHub Actions

Releases are fully automated: pushing a git tag with the right suffix
triggers a build for the corresponding platform(s) and publishes a
**draft** GitHub Release with the compiled binary attached.

| Tag pattern       | Workflow             | Runner           | Artifact                    |
|--------------------|-----------------------|------------------|------------------------------|
| `*release-linux`   | `linux-release.yml`   | `ubuntu-latest`  | `utas-release-linux.tar.gz`  |
| `*release-mac`     | `mac-release.yml`     | `macos-latest`   | `utas-release-mac-os.zip`    |
| `*release-win`     | `win-release.yml`     | `windows-latest` | `utas-release-windows.zip`   |
| `*release`         | all three above       | all three        | all three artifacts          |

Each release workflow:

1. Runs `cargo test` (and `cargo test -p file`) — the release build only
   proceeds if tests pass.
2. Builds `utas` in release mode (`cargo build --release`).
3. Compresses the resulting binary into a platform-specific archive.
4. Creates a **draft** GitHub Release for the pushed tag and uploads the
   archive as a release asset, via
   [`softprops/action-gh-release`](https://github.com/softprops/action-gh-release).

### How to cut a release

1. Make sure `master` is green (the `push.yml` workflow passes).
2. Pick a tag name ending in the suffix for the platform(s) you want to
   release, e.g. `v1.2.0-release` to build for all three platforms, or
   `v1.2.0-release-mac` to build macOS only.
3. Tag and push:

   ```bash
   git tag v1.2.0-release
   git push origin v1.2.0-release
   ```

4. Watch the corresponding workflow(s) run under the **Actions** tab.
5. Once finished, a **draft** release will appear under **Releases** with
   the built archive(s) attached. Review the draft, edit the release notes
   if needed, and publish it manually — drafts are never published
   automatically.

To release for every platform at once, use a tag ending in exactly
`release` (not `release-<platform>`), since all three workflows also match
the bare `*release` pattern.
