use druid::{
    tests::harness::Harness, widget::Flex, Event, KeyEvent, Modifiers, Selector, WidgetExt,
};
use fast_diary::view::widget::password_box::{focused_password_box, password_box};

const SUBMIT: Selector<()> = Selector::new("test.submit");

/// Types `text` one key at a time into whatever holds the keyboard focus.
fn type_text(harness: &mut Harness<(String, String)>, text: &str) {
    text.chars().for_each(|character| {
        harness.event(Event::KeyDown(KeyEvent::for_test(
            Modifiers::empty(),
            character.to_string().as_str(),
        )))
    });
}

#[test]
fn the_focused_password_box_receives_typing_as_soon_as_it_is_shown() {
    let root = Flex::column()
        .with_child(password_box(SUBMIT).lens(druid::lens!((String, String), 0)))
        .with_child(focused_password_box(SUBMIT).lens(druid::lens!((String, String), 1)));

    Harness::create_simple((String::new(), String::new()), root, |harness| {
        harness.send_initial_events();
        harness.just_layout();

        type_text(harness, "secret");

        assert_eq!(harness.data().0, "");
        assert_eq!(harness.data().1, "secret");
    });
}
