mod git;

use cursive::traits::*;
use cursive::views::DummyView;
use cursive::views::EditView;
use cursive::views::Panel;
use cursive::views::{CircularFocus, Dialog, LinearLayout, SelectView, TextView};
use std::cell::RefCell;
use std::env::current_dir;
use std::rc::Rc;

fn main() {
    run_tui();
}

fn run_tui() {
    //TODO organize this mess
    let current_dir = current_dir().unwrap();
    let repo = git::Repo::new(current_dir.as_os_str().to_str().unwrap()).unwrap();
    let stashes = repo.stashes().unwrap();
    let mut siv = cursive::default();
    let stash_selected: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

    let diff_view = Panel::new(LinearLayout::vertical().with_name("diff_view")).title("Diff");

    let select = select_stash(stashes, stash_selected.clone());

    siv.add_global_callback('q', |s| s.quit());
    apply_callback(&mut siv, repo.clone(), stash_selected.clone());
    stash_callback(&mut siv, repo.clone());

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

//list all stashes on the main screen
fn select_stash<'a>(
    stashes: Vec<git::StashDiff>,
    selected: Rc<RefCell<Option<usize>>>,
) -> SelectView<git::StashDiff> {
    let mut select = SelectView::new();
    for stash in stashes {
        select.add_item(stash.title().to_string(), stash);
    }
    select.set_on_submit(move |s: &mut cursive::Cursive, item: &git::StashDiff| {
        if let Some(mut diff_view) = s.find_name::<LinearLayout>("diff_view") {
            selected.replace(Some(item.index()));
            render_diff(&mut diff_view, &item);
        }
    });
    select
}

// handles the ok button when user is creating a new stash
fn stash_ok_button(siv: &mut cursive::Cursive, repo: git::Repo) {
    siv.add_layer(
        Dialog::around(EditView::new().with_name("stash_msg").fixed_width(10))
            .title("Enter message")
            .button("Ok", move |s| {
                let msg = s
                    .call_on_name("stash_msg", |view: &mut EditView| view.get_content())
                    .unwrap();
                match repo.stash(&msg) {
                    Ok(_) => {
                        s.pop_layer();
                        s.quit();
                        run_tui()
                    }
                    Err(e) => {
                        s.add_layer(
                            Dialog::around(TextView::new(format!("Failed to stash {:}!", e)))
                                .title("Stash")
                                .button("Ok", |s| {
                                    s.pop_layer();
                                })
                                .wrap_with(CircularFocus::new)
                                .wrap_tab(),
                        );
                    }
                }
            })
            .button("Cancel", |s| {
                s.pop_layer();
            }),
    );
}

// adds a callback on the 's' keypress
fn stash_callback(siv: &mut cursive::Cursive, repo: git::Repo) {
    siv.add_global_callback('s', move |s| {
        stash_ok_button(s, repo.clone());
    });
}

// adds a callback on the 'a' keypress
fn apply_callback(
    siv: &mut cursive::Cursive,
    repo: git::Repo,
    selected_stash: Rc<RefCell<Option<usize>>>,
) {
    siv.add_global_callback('a', move |s| {
        match selected_stash.replace(None) {
            Some(t) => {
                let result = repo.stash_apply(t);
                if let Err(e) = result {
                    s.add_layer(
                        Dialog::around(TextView::new(format!("Stash applied failed with {:}!", e)))
                            .title("Apply")
                            .button("Ok", |s| {
                                s.pop_layer();
                            })
                            .wrap_with(CircularFocus::new)
                            .wrap_tab(),
                    );
                } else {
                    s.add_layer(
                        Dialog::around(TextView::new("Stash applied!"))
                            .title("Apply")
                            .button("Ok", |s| {
                                s.pop_layer();
                                s.quit();
                                run_tui();
                            })
                            .wrap_with(CircularFocus::new)
                            .wrap_tab(),
                    );
                }
            }
            None => {
                s.add_layer(
                    Dialog::around(TextView::new(
                        "No stash selected! You forgot to press (ENTER) on the stash?",
                    ))
                    .title("Apply")
                    .button("Ok", |s| {
                        s.pop_layer();
                    })
                    .wrap_with(CircularFocus::new)
                    .wrap_tab(),
                );
            }
        };
    });
}

// renders the diff of a specific stash
fn render_diff(view: &mut LinearLayout, diff: &git::StashDiff) {
    for line in &diff.diffs {
        let text = match line {
            git::LineDiff::HunkHeader(c)
            | git::LineDiff::LineBinary(c)
            | git::LineDiff::FileHeader(c)
            | git::LineDiff::RemoveEndOfAFile(c)
            | git::LineDiff::AddEndOfAFile(c)
            | git::LineDiff::ContextEndOfAFile(c) => {
                TextView::new(c).style(cursive::theme::ColorStyle::primary())
            }
            git::LineDiff::Addition(c) => {
                TextView::new(format!("+++ {c}")).style(cursive::theme::ColorStyle::secondary())
            }
            git::LineDiff::Deletion(c) => {
                TextView::new(format!("--- {c}")).style(cursive::theme::ColorStyle::tertiary())
            }
            git::LineDiff::SameAsPrevious(c) => TextView::new(format!("{c}")),
        };
        view.add_child(text);
    }
}
