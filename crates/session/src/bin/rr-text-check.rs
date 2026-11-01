use rr_session::text::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: rr-text-check DATA_ROOT".into());
    }
    let data = std::path::Path::new(&args[0]);
    let inventory = language_inventory(data)?;
    let mut cases = 0;
    let mut codepoints = std::collections::BTreeSet::new();
    for entry in rr_session::ui::Catalog::built_in().entries {
        for label in entry.labels {
            codepoints.extend(label.chars().filter(|c| !c.is_control()).map(u32::from));
        }
    }
    for language in Language::ALL {
        let text = SourceText::load(data, language)?;
        for line in text.strings.iter().chain(text.credits.iter().flatten()) {
            codepoints.extend(line.chars().filter(|c| !c.is_control()).map(u32::from));
        }
        for level in 1..=5 {
            for character in 1..=8 {
                for partner in 0..8 {
                    for draw in 0..4 {
                        let (_, line) = text.spoken(level, partner, character, None, draw)?;
                        if line.trim().is_empty() {
                            return Err("Empty required source dialogue".into());
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"languages":inventory,"dialogue_cases":cases,"printable_codepoints":codepoints,"passed":true})
        )?
    );
    Ok(())
}
