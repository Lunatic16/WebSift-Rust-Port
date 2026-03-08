use colored::Colorize;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

// ── Compile regexes once at startup ─────────────────────────────────────────

static RE_EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}").unwrap()
});

static RE_PHONE: Lazy<Regex> = Lazy::new(|| {
    // US formats + international (+1-800-555-1234, +44 20 7946 0958, etc.)
    Regex::new(
        r"(?x)
        (?:\+\d{1,3}[\s\-]?)?
        (?:
            \(\d{3}\)[\s\-]?\d{3}[\s\-]?\d{4}  |
            \d{3}[\s\-]\d{3}[\s\-]\d{4}         |
            \d{10}
        )",
    )
    .unwrap()
});

static RE_LINK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://(?:[a-zA-Z0-9\-]+\.)+[a-zA-Z]{2,}(?:/[^"' <>\s]*)?"#).unwrap()
});

static RE_URL_VALID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(https?|ftp|file)://[-A-Za-z0-9+&@#/%?=~_|!:,.;]*[-A-Za-z0-9+&@#/%=~_|]$",
    )
    .unwrap()
});

// ── Pretty-print helpers ─────────────────────────────────────────────────────

macro_rules! info {
    ($($arg:tt)*) => {
        println!(
            "{} {}",
            format!("[{}]", "*".bright_yellow()).white().bold(),
            format!($($arg)*).bright_yellow()
        )
    };
}

macro_rules! ok {
    ($($arg:tt)*) => {
        println!(
            "{} {}",
            format!("[{}]", "*".bright_green()).white().bold(),
            format!($($arg)*).bright_green()
        )
    };
}

macro_rules! err {
    ($($arg:tt)*) => {
        println!(
            "{} {}",
            format!("[{}]", "!".bright_red()).white().bold(),
            format!($($arg)*).bright_red()
        )
    };
}

// ── Banner ───────────────────────────────────────────────────────────────────

fn print_banner() {
    print!("\x1B[2J\x1B[1;1H");
    println!(
        "{}",
        r#"
   __          __  _        _____ _  __ _
   \ \        / / | |      / ____(_)/ _| |
    \ \  /\  / /__| |__   | (___  _| |_| |_
     \ \/  \/ / _ \ '_ \   \___ \| |  _| __|
      \  /\  /  __/ |_) |  ____) | | | | |_
       \/  \/ \___|_.__/  |_____/|_|_|  \__|

                     Port Developer: Lunatic16
"#
        .bright_green()
    );
    println!("{}", "  Email, Phone Number, and Link Scraper Tool".cyan());
    println!(
        "{}\n",
        "  GitHub: https://github.com/Lunatic16".bright_yellow()
    );
}

// ── I/O helpers ──────────────────────────────────────────────────────────────

fn prompt(label: &str) -> String {
    print!(
        "{} ",
        format!("[{}] {} :", "*".bright_green(), label)
            .white()
            .bold()
    );
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

fn ask_yes_no(label: &str) -> bool {
    prompt(label).to_lowercase().starts_with('y')
}

// ── Network ──────────────────────────────────────────────────────────────────

fn check_connection() {
    thread::sleep(Duration::from_millis(400));
    err!("Checking internet connection...");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .unwrap();

    let reachable = client
        .head("https://www.google.com")
        .send()
        .or_else(|_| client.head("https://1.1.1.1").send())
        .is_ok();

    if reachable {
        ok!("Connected to the internet.");
    } else {
        err!("No internet connection detected. Try again later.");
        std::process::exit(1);
    }
}

fn fetch_page(url: &str) -> Option<String> {
    info!("Fetching page...");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; WebSift/1.0)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap();

    match client.get(url).send() {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                err!("Server returned HTTP {}", status);
                return None;
            }
            match resp.text() {
                Ok(body) => {
                    ok!("Page fetched ({} bytes).", body.len());
                    Some(body)
                }
                Err(e) => {
                    err!("Failed to decode response body: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            err!("Failed to fetch URL: {}", e);
            None
        }
    }
}

// ── Scrapers ─────────────────────────────────────────────────────────────────

fn scrape_emails(content: &str) -> BTreeSet<String> {
    RE_EMAIL
        .find_iter(content)
        .map(|m| m.as_str().to_lowercase())
        // Filter common false-positives (image / asset extensions)
        .filter(|e| {
            let tld = e.rsplit('.').next().unwrap_or("");
            !matches!(tld, "png" | "jpg" | "jpeg" | "gif" | "svg" | "css" | "js" | "woff" | "ttf")
        })
        .collect()
}

fn scrape_phones(content: &str) -> BTreeSet<String> {
    RE_PHONE
        .find_iter(content)
        .map(|m| m.as_str().trim().to_string())
        // Skip strings that are obviously not phone numbers (too short / too long)
        .filter(|p| {
            let digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
            (7..=15).contains(&digits.len())
        })
        .collect()
}

fn scrape_links(content: &str) -> BTreeSet<String> {
    RE_LINK
        .find_iter(content)
        .map(|m| {
            // Strip trailing punctuation artefacts from HTML
            m.as_str()
                .trim_end_matches(|c| matches!(c, '.' | ',' | ')' | ']' | '>'))
                .to_string()
        })
        .collect()
}

// ── Save results ─────────────────────────────────────────────────────────────

fn write_set(folder: &str, filename: &str, data: &BTreeSet<String>) {
    if data.is_empty() {
        return;
    }
    let path = format!("{}/{}", folder, filename);
    let content = data.iter().cloned().collect::<Vec<_>>().join("\n") + "\n";
    fs::write(&path, content).unwrap_or_else(|e| err!("Could not write {}: {}", path, e));
}

fn save_data(emails: &BTreeSet<String>, phones: &BTreeSet<String>, links: &BTreeSet<String>) {
    loop {
        let folder = prompt("Enter folder name to save results");

        if folder.is_empty() {
            err!("Folder name cannot be empty.");
            continue;
        }

        // Reject names with path separators to prevent directory traversal
        if folder.contains('/') || folder.contains('\\') {
            err!("Folder name must not contain path separators.");
            continue;
        }

        if Path::new(&folder).exists() {
            err!("Folder '{}' already exists. Choose another name.", folder);
            continue;
        }

        fs::create_dir(&folder).unwrap_or_else(|e| {
            err!("Could not create folder: {}", e);
            std::process::exit(1);
        });

        write_set(&folder, "email_output.txt", emails);
        write_set(&folder, "phone_output.txt", phones);
        write_set(&folder, "links_output.txt", links);

        thread::sleep(Duration::from_millis(300));
        ok!("Output saved in '{}'.", folder);

        if !emails.is_empty() {
            info!("  {} email(s)  -> {}/email_output.txt", emails.len(), folder);
        }
        if !phones.is_empty() {
            info!("  {} phone(s)  -> {}/phone_output.txt", phones.len(), folder);
        }
        if !links.is_empty() {
            info!("  {} link(s)   -> {}/links_output.txt", links.len(), folder);
        }

        break;
    }
}

// ── Display results ───────────────────────────────────────────────────────────

fn display_results(label: &str, items: &BTreeSet<String>) {
    if items.is_empty() {
        err!("No {} found.", label);
    } else {
        info!("{} {} found:", items.len(), label);
        for item in items {
            println!("  {}", item.white());
        }
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    print_banner();
    check_connection();

    'outer: loop {
        thread::sleep(Duration::from_millis(500));

        // ── URL input with validation loop ──
        let target_url = loop {
            let url = prompt("Enter URL to scrape");
            if url.is_empty() {
                err!("URL cannot be empty.");
                continue;
            }
            if RE_URL_VALID.is_match(&url) {
                break url;
            }
            err!("Invalid URL — must start with http://, https://, ftp://, or file://");
        };

        // ── Options ──
        let do_emails = ask_yes_no("Scrape emails? (y/n)");
        let do_phones = ask_yes_no("Scrape phone numbers? (y/n)");
        let do_links  = ask_yes_no("Scrape links (social media & other)? (y/n)");

        if !do_emails && !do_phones && !do_links {
            err!("No options selected. Nothing to do.");
            break;
        }

        // ── Fetch ──
        err!("Scraping started...");
        let content = match fetch_page(&target_url) {
            Some(c) => c,
            None => break,
        };

        // ── Scrape ──
        let emails = if do_emails { scrape_emails(&content) } else { BTreeSet::new() };
        let phones = if do_phones { scrape_phones(&content) } else { BTreeSet::new() };
        let links  = if do_links  { scrape_links(&content)  } else { BTreeSet::new() };

        println!();

        if do_emails { display_results("email(s)",        &emails); }
        if do_phones { display_results("phone number(s)", &phones); }
        if do_links  { display_results("link(s)",         &links);  }

        // ── Save prompt ──
        let any_results = !emails.is_empty() || !phones.is_empty() || !links.is_empty();
        if any_results {
            thread::sleep(Duration::from_millis(400));
            if ask_yes_no("Save output to a folder? (y/n)") {
                save_data(&emails, &phones, &links);
            }
        }

        // ── Loop back ──
        thread::sleep(Duration::from_millis(300));
        if !ask_yes_no("Scrape another URL? (y/n)") {
            break 'outer;
        }
        print!("\x1B[2J\x1B[1;1H");
        print_banner();
    }

    thread::sleep(Duration::from_millis(400));
    err!("Exiting....\n");
}
