# Dayone Export

A small utility to export a locally stored [DayOne](https://dayoneapp.com) journal into a folder of markdown files.

I use this for referencing and digesting my journal entries in [Obsidian](https://obsidian.md), however there is nothing
specific about the exported content to Obsidian.

## Usage

```
Usage: dayone-export-standalone [OPTIONS] --journal <JOURNAL> --database <DATABASE> --vault <VAULT> --default-output <DEFAULT_OUTPUT>

Options:
  -j, --journal <JOURNAL>
          The name of the journal to be exported
  -d, --database <DATABASE>
          Path to the dayone sqlite database
  -v, --vault <VAULT>
          The root of the vault which will be searched for existing entries
  -o, --default-output <DEFAULT_OUTPUT>
          Where to place new entries that have not yet been exported
  -w, --overwrite
          If existing files should be updated with newer DayOne content if available
  -h, --help
          Print help information
  -V, --version
          Print version information
```

## Possible Improvements

- Allow access to recorded audios.
- Finish tag support
- Replace links to other DayOne entries with `[[WikiLinks]]`.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

