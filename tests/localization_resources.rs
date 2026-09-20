use std::{collections::BTreeSet, fs, path::PathBuf};

const LOCALES: [&str; 2] = ["en-US", "tr-TR"];
const TRANSLATED_PREFIXES: [&str; 8] = [
    "page-",
    "app-",
    "month-",
    "weekday-short-",
    "menu-",
    "about-",
    "view-mode-",
    "calendar-",
];
const SOURCES_WITH_TRANSLATED_TEXT: [&str; 9] = [
    "src/view/page/main/main_page.rs",
    "src/view/page/settings/settings_page.rs",
    "src/modal/state/diary_view_mode.rs",
    "src/utils/localization.rs",
    "src/view/page/diary/widgets/icon_toolbar.rs",
    "src/view/page/diary/widgets/btn_create_widget/build_ui.rs",
    "src/view/window/main/main_menu.rs",
    "src/view/window/about/about_window.rs",
    "src/view/widget/calendar/mod.rs",
];

fn project_file(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn has_translated_prefix(key: &str) -> bool {
    TRANSLATED_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

fn defined_keys(locale: &str) -> anyhow::Result<BTreeSet<String>> {
    let path = project_file(&format!("resources/i18n/{locale}/builtin.ftl"));

    Ok(fs::read_to_string(path)?
        .lines()
        .filter_map(|line| line.split_once(" = "))
        .map(|(key, _value)| key.trim().to_string())
        .filter(|key| has_translated_prefix(key))
        .collect())
}

fn keys_used_in_sources() -> anyhow::Result<BTreeSet<String>> {
    SOURCES_WITH_TRANSLATED_TEXT
        .iter()
        .map(|source| Ok(fs::read_to_string(project_file(source))?))
        .collect::<anyhow::Result<Vec<_>>>()
        .map(|sources| {
            sources
                .iter()
                .flat_map(|source| source.split('"').skip(1).step_by(2))
                .filter(|literal| has_translated_prefix(literal))
                .map(str::to_string)
                .collect()
        })
}

#[test]
fn every_language_defines_the_same_menu_calendar_and_about_texts() -> anyhow::Result<()> {
    let english = defined_keys("en-US")?;

    LOCALES.iter().try_for_each(|locale| {
        let missing = english
            .difference(&defined_keys(locale)?)
            .cloned()
            .collect::<Vec<_>>();
        assert!(missing.is_empty(), "{locale} lacks {missing:?}");
        anyhow::Ok(())
    })
}

#[test]
fn every_text_key_used_in_the_code_is_translated_in_every_language() -> anyhow::Result<()> {
    let used = keys_used_in_sources()?;

    assert!(used.len() >= 40, "only found {} keys", used.len());
    LOCALES.iter().try_for_each(|locale| {
        let defined = defined_keys(locale)?;
        let undefined = used.difference(&defined).cloned().collect::<Vec<_>>();
        assert!(undefined.is_empty(), "{locale} lacks {undefined:?}");
        anyhow::Ok(())
    })
}
