use anyhow::Result;

use crate::ansi::*;
use crate::napm::Napm;

pub fn run(search: &str, num_results: Option<u32>) -> Result<()> {
    let mut napm = Napm::new()?;

    match napm.sync(false) {
        Ok(_) => {}
        Err(e) => eprintln!("\x1b[33mWarning\x1b[0m: using local databases only ({e})"),
    }

    fn relevance_score(name: &str, desc: &str, search: &str) -> f64 {
        let search_lower = search.to_lowercase();
        let name_lower = name.to_lowercase();
        let desc_lower = desc.to_lowercase();

        let name_matches = name_lower.matches(&search_lower).count() as f64;
        let desc_matches = desc_lower.matches(&search_lower).count() as f64;

        let name_len = name.len() as f64;
        let desc_len = desc.len().max(1) as f64;

        (name_matches / name_len * 2.0) + (desc_matches / desc_len)
    }

    struct SearchResult {
        db_name: String,
        name: String,
        desc: String,
        version: String,

        score: f64,
    }

    let mut results = Vec::new();

    for pkg in napm.search(&[search])? {
        let db_name = pkg.db_name;
        let name = pkg.name;
        let desc = pkg.desc;
        let version = pkg.version;

        let score = relevance_score(&name, &desc, search);

        results.push(SearchResult {
            db_name,
            name,
            desc,
            version,

            score,
        });
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    fn highlight(text: &str, search: &str, color: &str) -> String {
        let lower = text.to_lowercase();
        let search_lower = search.to_lowercase();
        if let Some(idx) = lower.find(&search_lower) {
            let end = idx + search.chars().count();
            format!(
                "{}{}{}{}{}{}{}{}",
                color,
                &text[..idx],
                ANSI_UNDERLINE,
                &text[idx..end],
                ANSI_RESET,
                color,
                &text[end..],
                ANSI_RESET
            )
        } else {
            format!("{color}{text}{ANSI_RESET}")
        }
    }

    let results = if let Some(n) = num_results {
        results.iter().take(n as usize).collect::<Vec<_>>()
    } else {
        results.iter().collect::<Vec<_>>()
    };

    for (
        i,
        SearchResult {
            db_name,
            name,
            desc,
            version,
            ..
        },
    ) in results.iter().enumerate().rev()
    {
        let indicator = format!("{ANSI_RED}-{ANSI_RESET}");

        let name = highlight(name, search, ANSI_CYAN);
        let desc = highlight(desc, search, ANSI_WHITE);

        let n = i + 1;

        println!(
            " {indicator} {ANSI_YELLOW}[{ANSI_BOLD}{n}{ANSI_RESET}{ANSI_YELLOW}]{ANSI_RESET} {ANSI_CYAN}{db_name}{ANSI_WHITE}/{name} {ANSI_BLUE}{version}{ANSI_RESET} {desc}"
        );
    }

    Ok(())
}
