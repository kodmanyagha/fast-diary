use std::{fs, path::Path};

use anyhow::anyhow;
use chrono::{Local, NaiveDate, Timelike};
use druid::{
    keyboard_types::Key, tests::harness::Harness, widget::Controller, Env, Event, EventCtx,
    KeyEvent, Modifiers, MouseButton, MouseButtons, MouseEvent, Point, Selector, Vec2, Widget,
    WidgetExt,
};
use fast_diary::{
    consts::druid_selector,
    modal::{
        app_state::AppState,
        state::calendar_month::{CalendarMonth, DAYS_IN_WEEK},
    },
    view::{
        widget::calendar::{calendar_grid::CalendarGrid, grid_geometry::GridGeometry},
        window::main::{
            main_window_controller::MainWindowController,
            main_window_controller_utils::load_diary_items,
        },
    },
};

const GRID_WIDTH: f64 = 400.0;

fn write_diaries(folder: &Path, diaries: &[(&str, &str)]) -> anyhow::Result<()> {
    diaries
        .iter()
        .try_for_each(|(file_name, text)| fs::write(folder.join(file_name), text))?;
    Ok(())
}

fn app_state_for(folder: &Path, month_of: NaiveDate) -> anyhow::Result<AppState> {
    let mut app_state = AppState::new();
    app_state.diary_base_path = folder.to_str().map(str::to_string);
    let store = app_state
        .diary_store()
        .ok_or_else(|| anyhow!("The folder has no store."))?;
    app_state.diaries = load_diary_items(&store)?;
    app_state.calendar_month = CalendarMonth::containing(month_of);

    Ok(app_state)
}

fn cell_center(first_month: CalendarMonth, date: NaiveDate) -> anyhow::Result<Point> {
    let geometry = GridGeometry::new(GRID_WIDTH, first_month.window_rows());

    first_month
        .window()
        .iter()
        .enumerate()
        .find_map(|(month_index, month)| {
            (0..month.week_rows() * DAYS_IN_WEEK)
                .find(|cell| month.date_at_cell(*cell) == Some(date))
                .map(|cell| geometry.cell_rect(month_index, cell).center())
        })
        .ok_or_else(|| anyhow!("{date} is not shown by the calendar."))
}

fn mouse_event(pos: Point) -> MouseEvent {
    MouseEvent {
        pos,
        window_pos: pos,
        buttons: MouseButtons::new().with(MouseButton::Left),
        mods: Modifiers::empty(),
        count: 1,
        focus: false,
        button: MouseButton::Left,
        wheel_delta: Vec2::ZERO,
    }
}

fn open_window(harness: &mut Harness<AppState>) {
    harness.send_initial_events();
    harness.just_layout();
}

fn click(harness: &mut Harness<AppState>, pos: Point) {
    harness.event(Event::MouseMove(mouse_event(pos)));
    harness.event(Event::MouseDown(mouse_event(pos)));
    harness.event(Event::MouseUp(mouse_event(pos)));
}

fn press(harness: &mut Harness<AppState>, key: Key) {
    harness.event(Event::KeyDown(KeyEvent::for_test(Modifiers::empty(), key)));
}

struct CommandOrderRecorder;

impl<W: Widget<AppState>> Controller<AppState, W> for CommandOrderRecorder {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if cmd.is(druid_selector::DIARY_SAVE_CURRENT) {
                app_state.status_message.push_str("save,");
            } else if cmd.is(druid_selector::DIARY_SET_CURRENT) {
                app_state.status_message.push_str("open,");
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

const WRITE_TEXT: Selector<String> = Selector::new("test.write_text");

struct TextWriter;

impl<W: Widget<AppState>> Controller<AppState, W> for TextWriter {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if let Some(text) = cmd.get(WRITE_TEXT) {
                app_state.txt_diary = text.clone();
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

fn write_text(harness: &mut Harness<AppState>, text: &str) {
    harness.submit_command(WRITE_TEXT.with(text.to_string()));
}

fn save(harness: &mut Harness<AppState>) {
    harness.submit_command(druid_selector::DIARY_SAVE_CURRENT);
}

fn calendar_test(app_state: AppState, scenario: impl FnMut(&mut Harness<AppState>)) {
    let root = CalendarGrid::new()
        .controller(MainWindowController::new())
        .controller(TextWriter);
    Harness::create_simple(app_state, root, scenario);
}

fn date(year: i32, month: u32, day: u32) -> anyhow::Result<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Invalid date."))
}

fn opened_file_name(harness: &Harness<AppState>) -> String {
    harness.data().current_diary.diary.file_name.clone()
}

const DIARIES: [(&str, &str); 4] = [
    ("240115090000.md", "morning"),
    ("240115200000.md", "evening"),
    ("240116100000.md", "next day"),
    ("240210100000.md", "february"),
];

#[test]
fn clicking_a_day_opens_the_latest_diary_of_that_day() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_15th = date(2024, 1, 15)?;
    let app_state = app_state_for(folder.path(), january_15th)?;
    let day_center = cell_center(app_state.calendar_month, january_15th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);

        assert!(harness.data().current_diary.is_selected);
        assert_eq!(opened_file_name(harness), "240115200000.md");
        assert_eq!(harness.data().txt_diary, "evening");
    });
    Ok(())
}

#[test]
fn clicking_a_day_without_diaries_starts_an_empty_draft_at_the_current_time() -> anyhow::Result<()>
{
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_20th = date(2024, 1, 20)?;
    let app_state = app_state_for(folder.path(), january_20th)?;
    let day_center = cell_center(app_state.calendar_month, january_20th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);

        let current_diary = &harness.data().current_diary;
        assert!(current_diary.is_selected);
        assert!(current_diary.is_draft);
        assert_eq!(current_diary.diary.date.local_date(), january_20th);
        assert_eq!(harness.data().txt_diary, "");
        assert_eq!(harness.data().diaries.len(), DIARIES.len());
        assert!(!folder.path().join(&current_diary.diary.file_name).exists());
    });
    Ok(())
}

#[test]
fn a_draft_without_text_never_creates_a_file() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_20th = date(2024, 1, 20)?;
    let app_state = app_state_for(folder.path(), january_20th)?;
    let day_center = cell_center(app_state.calendar_month, january_20th)?;
    let draft_path = folder.path().join("240120000000.md");

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);

        save(harness);
        write_text(harness, "  \n ");
        save(harness);

        assert!(harness.data().current_diary.is_draft);
        assert!(!draft_path.exists());
    });
    Ok(())
}

#[test]
fn writing_into_a_draft_creates_the_diary_file_of_that_day() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_20th = date(2024, 1, 20)?;
    let app_state = app_state_for(folder.path(), january_20th)?;
    let day_center = cell_center(app_state.calendar_month, january_20th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);
        let file_name = harness.data().current_diary.diary.file_name.clone();
        let draft_path = folder.path().join(&file_name);

        write_text(harness, "hello diary");
        save(harness);

        assert_eq!(
            fs::read_to_string(&draft_path).ok().as_deref(),
            Some("hello diary")
        );
        assert!(!harness.data().current_diary.is_draft);
        assert_eq!(harness.data().diaries.len(), DIARIES.len() + 1);
        assert_eq!(
            harness
                .data()
                .diaries
                .iter()
                .find(|item| item.file_name == file_name)
                .map(|item| item.summary.as_str()),
            Some("hello diary")
        );

        write_text(harness, "hello again");
        save(harness);
        assert_eq!(
            fs::read_to_string(&draft_path).ok().as_deref(),
            Some("hello again")
        );

        click(harness, day_center);
        assert!(!harness.data().current_diary.is_draft);
        assert_eq!(harness.data().txt_diary, "hello again");
    });
    Ok(())
}

#[test]
fn switching_to_another_day_saves_the_written_draft_first() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_20th = date(2024, 1, 20)?;
    let january_22nd = date(2024, 1, 22)?;
    let app_state = app_state_for(folder.path(), january_20th)?;
    let first_day = cell_center(app_state.calendar_month, january_20th)?;
    let second_day = cell_center(app_state.calendar_month, january_22nd)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, first_day);
        let first_name = harness.data().current_diary.diary.file_name.clone();
        let first_path = folder.path().join(&first_name);
        write_text(harness, "note of the 20th");
        click(harness, second_day);
        let second_name = harness.data().current_diary.diary.file_name.clone();
        let second_path = folder.path().join(&second_name);

        assert_eq!(
            fs::read_to_string(&first_path).ok().as_deref(),
            Some("note of the 20th")
        );
        assert!(!second_path.exists());
        assert_eq!(opened_file_name(harness), second_name);
        assert!(harness.data().current_diary.is_draft);
        assert_eq!(harness.data().txt_diary, "");
    });
    Ok(())
}

#[test]
fn starting_a_draft_on_a_day_with_diaries_opens_the_latest_one() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_15th = date(2024, 1, 15)?;
    let app_state = app_state_for(folder.path(), january_15th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        harness.submit_command(druid_selector::DIARY_START_DRAFT.with(january_15th));

        assert!(!harness.data().current_diary.is_draft);
        assert_eq!(opened_file_name(harness), "240115200000.md");
    });
    Ok(())
}

#[test]
fn arrow_keys_from_a_draft_go_to_the_nearest_older_diary() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_20th = date(2024, 1, 20)?;
    let app_state = app_state_for(folder.path(), january_20th)?;
    let day_center = cell_center(app_state.calendar_month, january_20th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);

        press(harness, Key::ArrowLeft);

        assert_eq!(opened_file_name(harness), "240116100000.md");
        assert!(!harness.data().current_diary.is_draft);
    });
    Ok(())
}

#[test]
fn arrow_keys_walk_through_the_diaries_and_the_calendar_follows() -> anyhow::Result<()> {
    let february = CalendarMonth::containing(date(2024, 2, 10)?);
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_15th = date(2024, 1, 15)?;
    let app_state = app_state_for(folder.path(), january_15th)?;
    let day_center = cell_center(app_state.calendar_month, january_15th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        click(harness, day_center);
        assert_eq!(opened_file_name(harness), "240115200000.md");

        press(harness, Key::ArrowLeft);
        assert_eq!(opened_file_name(harness), "240115090000.md");

        press(harness, Key::ArrowLeft);
        assert_eq!(opened_file_name(harness), "240115090000.md");

        press(harness, Key::ArrowRight);
        press(harness, Key::ArrowRight);
        assert_eq!(opened_file_name(harness), "240116100000.md");

        press(harness, Key::ArrowDown);
        assert_eq!(opened_file_name(harness), "240210100000.md");
        assert_eq!(harness.data().txt_diary, "february");
        assert!(harness.data().calendar_month.shows(february));

        press(harness, Key::ArrowRight);
        assert_eq!(opened_file_name(harness), "240210100000.md");
    });
    Ok(())
}

#[test]
fn the_opened_diary_is_saved_before_another_one_is_opened() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_16th = date(2024, 1, 16)?;
    let app_state = app_state_for(folder.path(), january_16th)?;
    let day_center = cell_center(app_state.calendar_month, january_16th)?;
    let root = CalendarGrid::new()
        .controller(MainWindowController::new())
        .controller(CommandOrderRecorder);

    Harness::create_simple(app_state, root, |harness| {
        open_window(harness);
        click(harness, day_center);

        assert_eq!(harness.data().status_message, "save,open,");
    });
    Ok(())
}

#[test]
fn opening_a_diary_of_a_month_that_is_not_shown_puts_that_month_in_the_middle() -> anyhow::Result<()>
{
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    write_diaries(folder.path(), &[("240610100000.md", "june")])?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;
    let june = CalendarMonth::containing(date(2024, 6, 10)?);

    calendar_test(app_state, |harness| {
        open_window(harness);
        let june_diary = harness
            .data()
            .diaries
            .iter()
            .find(|item| item.file_name == "240610100000.md")
            .cloned();
        if let Some(june_diary) = june_diary {
            harness.submit_command(druid_selector::DIARY_SET_CURRENT.with(june_diary));
        }

        assert_eq!(opened_file_name(harness), "240610100000.md");
        assert_eq!(harness.data().calendar_month, june.shifted(-1));
    });
    Ok(())
}

fn diary_files(folder: &Path) -> anyhow::Result<usize> {
    Ok(fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".md"))
        .count())
}

fn todays_file_of_hour(hour: u32) -> String {
    format!("{}{hour:02}0000.md", Local::now().format("%y%m%d"))
}

fn open_diary_named(harness: &mut Harness<AppState>, file_name: &str) {
    let diary = harness
        .data()
        .diaries
        .iter()
        .find(|item| item.file_name == file_name)
        .cloned();
    if let Some(diary) = diary {
        harness.submit_command(druid_selector::DIARY_SET_CURRENT.with(diary));
    }
}

#[test]
fn clicking_a_day_again_opens_the_next_diary_of_that_day() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &DIARIES)?;
    let january_15th = date(2024, 1, 15)?;
    let january_16th = date(2024, 1, 16)?;
    let app_state = app_state_for(folder.path(), january_15th)?;
    let day_15 = cell_center(app_state.calendar_month, january_15th)?;
    let day_16 = cell_center(app_state.calendar_month, january_16th)?;

    calendar_test(app_state, |harness| {
        open_window(harness);

        click(harness, day_16);
        assert_eq!(opened_file_name(harness), "240116100000.md");

        click(harness, day_15);
        assert_eq!(opened_file_name(harness), "240115200000.md");
        click(harness, day_15);
        assert_eq!(opened_file_name(harness), "240115090000.md");
        assert_eq!(harness.data().txt_diary, "morning");
        click(harness, day_15);
        assert_eq!(opened_file_name(harness), "240115200000.md");

        click(harness, day_16);
        click(harness, day_16);
        assert_eq!(opened_file_name(harness), "240116100000.md");
    });
    Ok(())
}

#[test]
fn creating_a_diary_in_an_hour_that_has_one_opens_that_diary() -> anyhow::Result<()> {
    let hour = Local::now().hour();
    let file_of_this_hour = todays_file_of_hour(hour);
    let folder = tempfile::tempdir()?;
    write_diaries(
        folder.path(),
        &[
            ("240115090000.md", "old"),
            (&file_of_this_hour, "text of this hour"),
        ],
    )?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        open_diary_named(harness, "240115090000.md");

        harness.submit_command(druid_selector::CREATE_NEW_DIARY);

        assert_eq!(opened_file_name(harness), file_of_this_hour);
        assert_eq!(harness.data().txt_diary, "text of this hour");
        assert_eq!(harness.data().diaries.len(), 2);
        assert!(harness
            .data()
            .calendar_month
            .shows(CalendarMonth::containing(Local::now().date_naive())));
        assert_eq!(diary_files(folder.path()).ok(), Some(2));
    });
    Ok(())
}

#[test]
fn creating_a_diary_saves_the_diary_that_was_open() -> anyhow::Result<()> {
    let file_of_this_hour = todays_file_of_hour(Local::now().hour());
    let folder = tempfile::tempdir()?;
    write_diaries(
        folder.path(),
        &[
            ("240115090000.md", "old"),
            (&file_of_this_hour, "text of this hour"),
        ],
    )?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;
    let old_path = folder.path().join("240115090000.md");

    calendar_test(app_state, |harness| {
        open_window(harness);
        open_diary_named(harness, "240115090000.md");
        write_text(harness, "edited old");

        harness.submit_command(druid_selector::CREATE_NEW_DIARY);

        assert_eq!(opened_file_name(harness), file_of_this_hour);
        assert_eq!(
            fs::read_to_string(&old_path).ok().as_deref(),
            Some("edited old")
        );
    });
    Ok(())
}

#[test]
fn creating_a_diary_in_an_hour_without_one_makes_and_opens_a_new_one() -> anyhow::Result<()> {
    let now = Local::now();
    let other_hour_today = todays_file_of_hour((now.hour() + 12) % 24);
    let folder = tempfile::tempdir()?;
    write_diaries(
        folder.path(),
        &[
            ("240115090000.md", "old"),
            (&other_hour_today, "another hour today"),
        ],
    )?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;

    calendar_test(app_state, |harness| {
        open_window(harness);

        harness.submit_command(druid_selector::CREATE_NEW_DIARY);

        let opened = harness.data().current_diary.clone();
        assert!(opened.is_selected);
        assert!(!opened.is_draft);
        assert_ne!(opened.diary.file_name, other_hour_today);
        assert!(opened
            .diary
            .file_name
            .starts_with(&now.format("%y%m%d%H").to_string()));
        assert_eq!(harness.data().txt_diary, "");
        assert_eq!(harness.data().diaries.len(), 3);
        assert_eq!(diary_files(folder.path()).ok(), Some(3));
    });
    Ok(())
}

#[test]
fn creating_twice_in_an_hour_keeps_a_single_new_diary() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &[("240115090000.md", "old")])?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;

    calendar_test(app_state, |harness| {
        open_window(harness);

        harness.submit_command(druid_selector::CREATE_NEW_DIARY);
        let first_name = opened_file_name(harness);
        write_text(harness, "typed meanwhile");
        harness.submit_command(druid_selector::CREATE_NEW_DIARY);

        assert_eq!(opened_file_name(harness), first_name);
        assert_eq!(harness.data().txt_diary, "typed meanwhile");
        assert_eq!(harness.data().diaries.len(), 2);
        assert_eq!(diary_files(folder.path()).ok(), Some(2));
    });
    Ok(())
}

/// A draft of today is always started at the current time, so it already belongs to "this
/// hour" and [`druid_selector::CREATE_NEW_DIARY`] finds it open instead of filing it away.
#[test]
fn creating_a_diary_keeps_what_was_written_in_the_draft_of_today() -> anyhow::Result<()> {
    let today = Local::now().date_naive();
    let folder = tempfile::tempdir()?;
    write_diaries(folder.path(), &[("240115090000.md", "old")])?;
    let app_state = app_state_for(folder.path(), date(2024, 1, 15)?)?;

    calendar_test(app_state, |harness| {
        open_window(harness);
        harness.submit_command(druid_selector::DIARY_START_DRAFT.with(today));
        write_text(harness, "draft text");

        harness.submit_command(druid_selector::CREATE_NEW_DIARY);

        assert!(harness.data().current_diary.is_draft);
        assert_eq!(harness.data().txt_diary, "draft text");
    });
    Ok(())
}
