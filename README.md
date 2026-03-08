# WebSift

> **Email, Phone Number, and Link Scraper Tool**  
> A fast, colored terminal scraper written in Rust — ported and enhanced from the original [`websift.sh`](https://github.com/s-r-e-e-r-a-j) bash script.

```
   __          __  _        _____ _  __ _
   \ \        / / | |      / ____(_)/ _| |
    \ \  /\  / /__| |__   | (___  _| |_| |_
     \ \/  \/ / _ \ '_ \   \___ \| |  _| __|
      \  /\  /  __/ |_) |  ____) | | | | |_
       \/  \/ \___|_.__/  |_____/|_|_|  \__|

                 Developer: Sreeraj
```

---

## Table of Contents

- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Usage](#usage)
- [Output Files](#output-files)
- [Scraping Details](#scraping-details)
- [Dependencies](#dependencies)
- [Improvements over the Bash Version](#improvements-over-the-bash-version)
- [Project Structure](#project-structure)
- [Known Limitations](#known-limitations)
- [License](#license)

---

## Features

- 📧 **Email scraping** — extracts email addresses with false-positive filtering (ignores asset filenames like `icon@2x.png`)
- 📞 **Phone number scraping** — supports US formats and basic international numbers with country codes
- 🔗 **Link scraping** — extracts all HTTP/HTTPS URLs from the page, with trailing punctuation stripped
- 🎨 **Colored terminal output** — green/yellow/red status indicators for a clean CLI experience
- 💾 **Save to folder** — optionally save each result type to its own `.txt` file in a named output folder
- 🔁 **Multi-URL sessions** — scrape multiple URLs back-to-back without restarting the binary
- 📊 **Result counts** — shows how many items were found per category, both on-screen and in the save summary
- 🛡️ **Input validation** — rejects malformed URLs and folder names with path separators

---

## Requirements

- [Rust](https://rustup.rs/) 1.70 or later (edition 2021)
- An internet connection (checked automatically on startup)

To install Rust if you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Installation

Clone the repository and build a release binary:

```bash
git clone https://github.com/s-r-e-e-r-a-j/websift
cd websift
cargo build --release
```

The compiled binary will be at:

```
target/release/websift
```

You can optionally move it onto your PATH:

```bash
sudo mv target/release/websift /usr/local/bin/websift
```

---

## Usage

Run directly with Cargo (development):

```bash
cargo run
```

Or run the release binary:

```bash
./target/release/websift
```

### Interactive walkthrough

```
[*] Enter URL to scrape : https://example.com
[*] Scrape emails? (y/n) : y
[*] Scrape phone numbers? (y/n) : y
[*] Scrape links (social media & other)? (y/n) : y

[!] Scraping started...
[*] Fetching page...
[*] Page fetched (48921 bytes).

[*] 3 email(s) found:
  admin@example.com
  info@example.com
  support@example.com

[*] 1 phone number(s) found:
  +1-800-555-0199

[*] 12 link(s) found:
  https://example.com/about
  https://example.com/contact
  https://twitter.com/example
  ...

[*] Save output to a folder? (y/n) : y
[*] Enter folder name to save results : example_results
[*] Output saved in 'example_results'.
[*]   3 email(s)  -> example_results/email_output.txt
[*]   1 phone(s)  -> example_results/phone_output.txt
[*]  12 link(s)   -> example_results/links_output.txt

[*] Scrape another URL? (y/n) : n
[!] Exiting....
```

---

## Output Files

When you choose to save results, WebSift creates a folder with up to three files:

| File | Contents |
|---|---|
| `email_output.txt` | One email address per line, lowercased and deduplicated |
| `phone_output.txt` | One phone number per line, deduplicated |
| `links_output.txt` | One URL per line, sorted alphabetically and deduplicated |

Files for categories with no results are not created.

---

## Scraping Details

### Email addresses

Matches the pattern `user@domain.tld` case-insensitively. The following TLDs are filtered out as false positives, since they appear in asset filenames rather than real addresses:

`png`, `jpg`, `jpeg`, `gif`, `svg`, `css`, `js`, `woff`, `ttf`

### Phone numbers

Matches these formats:

| Format | Example |
|---|---|
| `(NXX) NXX-XXXX` | `(800) 555-0199` |
| `NXX-NXX-XXXX` | `800-555-0199` |
| `NXX NXX XXXX` | `800 555 0199` |
| `XXXXXXXXXX` | `8005550199` |
| With country code | `+1-800-555-0199`, `+44 20 7946 0958` |

A digit-length gate (7–15 digits) is applied to reject false positives like version strings or numeric IDs.

### Links

Matches all `http://` and `https://` URLs found in the raw HTML. Trailing punctuation characters — `.` `,` `)` `]` — that commonly get captured from HTML attribute context are stripped from the end of each match.

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| [`reqwest`](https://crates.io/crates/reqwest) | 0.13 | Blocking HTTP client with timeout, redirects, and User-Agent |
| [`regex`](https://crates.io/crates/regex) | 1 | Email, phone, and link pattern matching |
| [`colored`](https://crates.io/crates/colored) | 3 | Terminal color output |
| [`once_cell`](https://crates.io/crates/once_cell) | 1 | Lazy static regex compilation (compiled once at startup) |

All dependencies are managed automatically by Cargo — no manual installs needed.

---

## Improvements over the Bash Version

| Area | Bash (`websift.sh`) | Rust (`websift`) |
|---|---|---|
| Dependency install | Manual `apt` / `pkg` per tool | Cargo fetches everything |
| Regex compilation | Rebuilt on every `grep` call | Compiled once at startup via `once_cell` |
| Internet check | `wget --spider google.com` | `reqwest` HEAD request with `1.1.1.1` fallback |
| HTTP client | Basic `curl -s` — no timeout | 15s timeout, User-Agent, follows up to 5 redirects |
| HTTP errors | Silently returns empty | Reports HTTP status code and decoding errors |
| Phone patterns | US formats only | US formats + international country code prefix |
| Email false positives | Not filtered | Asset file extensions filtered out |
| Phone false positives | Not filtered | Digit-length gate (7–15 digits) |
| Link trailing punctuation | Kept as-is | `.` `,` `)` `]` stripped from ends |
| Multi-URL session | Must restart binary | "Scrape another URL?" loop |
| Result counts | Not displayed | Shown in output and in save summary |
| Path traversal guard | None | Folder names with `/` or `\` are rejected |
| Code repetition | Verbose `echo` per message | `info!` / `ok!` / `err!` macros |

---

## Project Structure

```
websift/
├── Cargo.toml          # Package manifest and dependencies
├── README.md           # This file
└── src/
    └── main.rs         # All application logic
```

---

## Known Limitations

- **JavaScript-rendered content** — WebSift fetches raw HTML only. Emails, phones, or links injected by JavaScript after page load will not be found. For JS-heavy sites, consider pre-rendering with a headless browser before piping to this tool.
- **Phone number accuracy** — The regex is heuristic. It may catch numeric strings that look like phone numbers but aren't (e.g. order numbers, ZIP codes). Manual review of results is recommended.
- **Email TLD length** — The pattern matches TLDs up to unlimited length. Modern TLDs like `.photography` or `.international` are included, but so are some false positives in verbose HTML.
- **Single-page only** — WebSift scrapes the URL you provide and does not follow internal links or crawl the site.

---

## License

MIT — see [LICENSE](LICENSE) for details.
