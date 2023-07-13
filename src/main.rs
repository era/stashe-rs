mod git;

use cursive::traits::*;
use cursive::views::DummyView;
use cursive::views::EditView;
use cursive::views::Panel;
use cursive::views::{LinearLayout, SelectView, TextView};
use cursive::CursiveExt;
use std::env::current_dir;

fn main() {
    //TODO organize this mess
    let current_dir = current_dir().unwrap();
    let mut repo = git::Repo::new(current_dir.as_os_str().to_str().unwrap()).unwrap();
    let stashes = repo.stashes().unwrap();
    let mut siv = cursive::default();

    siv.add_global_callback('q', |s| s.quit());

    let diff_view = Panel::new(LinearLayout::vertical().with_name("diff_view")).title("Diff");

    let mut select = select_stash(stashes);

    let main_layout = LinearLayout::vertical()
        .child(Panel::new(TextView::new(
            "(q) to exit - (ENTER) to see diff - (s) to create a new stash - (a) to apply selected stash",
        )))
        .child(DummyView)
        .child(Panel::new(select).title("Stashes available"))
        .child(DummyView)
        .child(diff_view)
        .scrollable()
        .scroll_x(true)
        .scroll_y(true);

    siv.add_layer(main_layout);

    siv.run();
}

fn select_stash<'a>(stashes: Vec<git::StashDiff>) -> SelectView<git::StashDiff> {
    let mut select = SelectView::new(); //.h_align(cursive::align::HAlign::Center).v_align(cursive::align::VAlign::Center)
    for stash in stashes {
        select.add_item(stash.title().to_string(), stash);
    }
    // When an option is selected, update the text view with the selected option
    select.set_on_submit(move |s: &mut cursive::Cursive, item: &git::StashDiff| {
        if let Some(mut diff_view) = s.find_name::<LinearLayout>("diff_view") {
            // text_view.set_color(ColorStyle::new(Color::Dark(BaseColor::Green), Color::Reset)); //select_repo.diff(item)
            diff_view.clear();
            diff_view.add_child(TextView::new("for now").style(cursive::theme::ColorStyle::tertiary()));
            diff_view.add_child(TextView::new("for now").style(cursive::theme::ColorStyle::secondary()));
            diff_view.add_child(TextView::new("for now").style(cursive::theme::ColorStyle::primary()));
        }
    });
    select
}
