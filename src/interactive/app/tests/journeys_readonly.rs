use anyhow::Result;
use pretty_assertions::assert_eq;
use std::ffi::OsString;

use crate::interactive::{
    app::tests::{
        utils::{
            fixture_str, index_by_name, initialized_app_and_terminal_from_fixture, into_keys,
            node_by_index, node_by_name,
        },
        FIXTURE_PATH,
    },
    SortMode,
};

#[test]
fn init_from_pdu_results() -> Result<()> {
    use crate::interactive::app::tests::utils::new_test_terminal;
    let _terminal = new_test_terminal()?;

    Ok(())
}

#[test]
fn simple_user_journey_read_only() -> Result<()> {
    let long_root = "sample-02/dir";
    let short_root = "sample-01";
    let (mut terminal, mut app) =
        initialized_app_and_terminal_from_fixture(&[short_root, long_root])?;

    // POST-INIT
    // after initialization, we expect that...
    {
        assert_eq!(
            app.state.sorting,
            SortMode::SizeDescending,
            "it will sort entries in descending order by size"
        );

        assert!(
            !app.state.is_scanning,
            "it will not think it is still scanning"
        );

        let first_selected_path = OsString::from(format!("{}/{}", FIXTURE_PATH, long_root));
        assert_eq!(
            node_by_name(&app, &first_selected_path).name,
            first_selected_path,
            "the roots are always listed with the given (possibly long) names",
        );

        assert_eq!(
            node_by_name(&app, fixture_str(short_root)),
            node_by_index(&app, *app.state.selected.as_ref().unwrap()),
            "it selects the first node in the list",
        );

        assert_eq!(
            app.traversal.root_index, app.state.root,
            "the root is the 'virtual' root",
        );
    }

    // SORTING
    {
        // when hitting the S key
        app.process_events(&mut terminal, into_keys(b"s".iter()))?;
        assert_eq!(
            app.state.sorting,
            SortMode::SizeAscending,
            "it sets the sort mode to ascending by size"
        );
        assert_eq!(
            node_by_index(&app, app.state.entries[0].index),
            node_by_name(&app, fixture_str(long_root)),
            "it recomputes the cached entries"
        );
        // when hitting the S key again
        app.process_events(&mut terminal, into_keys(b"s".iter()))?;
        assert_eq!(
            app.state.sorting,
            SortMode::SizeDescending,
            "it sets the sort mode to descending by size"
        );
        assert_eq!(
            node_by_index(&app, app.state.entries[0].index),
            node_by_name(&app, fixture_str(short_root)),
            "it recomputes the cached entries"
        );
    }

    // Entry-Navigation
    {
        // when hitting the j key
        app.process_events(&mut terminal, into_keys(b"j".iter()))?;
        assert_eq!(
            node_by_name(&app, fixture_str(long_root)),
            node_by_index(&app, *app.state.selected.as_ref().unwrap()),
            "it moves the cursor down and selects the next entry based on the current sort mode"
        );
        // when hitting it while there is nowhere to go
        app.process_events(&mut terminal, into_keys(b"j".iter()))?;
        assert_eq!(
            node_by_name(&app, fixture_str(long_root)),
            node_by_index(&app, *app.state.selected.as_ref().unwrap()),
            "it stays at the previous position"
        );
        // when hitting the k key
        app.process_events(&mut terminal, into_keys(b"k".iter()))?;
        assert_eq!(
            node_by_name(&app, fixture_str(short_root)),
            node_by_index(&app, *app.state.selected.as_ref().unwrap()),
            "it moves the cursor up and selects the next entry based on the current sort mode"
        );
        // when hitting the k key again
        app.process_events(&mut terminal, into_keys(b"k".iter()))?;
        assert_eq!(
            node_by_name(&app, fixture_str(short_root)),
            node_by_index(&app, *app.state.selected.as_ref().unwrap()),
            "it stays at the current cursor position as there is nowhere to go"
        );
        // when hitting the o key with a directory selected
        app.process_events(&mut terminal, into_keys(b"o".iter()))?;
        {
            let new_root_idx = index_by_name(&app, fixture_str(short_root));
            assert_eq!(
                new_root_idx, app.state.root,
                "it enters the entry if it is a directory, changing the root"
            );
            assert_eq!(
                index_by_name(&app, "dir"),
                *app.state.selected.as_ref().unwrap(),
                "it selects the first entry in the directory"
            );

            // when hitting the u key while inside a sub-directory
            app.process_events(&mut terminal, into_keys(b"u".iter()))?;
            {
                assert_eq!(
                    app.traversal.root_index, app.state.root,
                    "it sets the root to be the (roots) parent directory, being the virtual root"
                );
                assert_eq!(
                    node_by_name(&app, fixture_str(short_root)),
                    node_by_index(&app, *app.state.selected.as_ref().unwrap()),
                    "changes the selection to the first item in the list of entries"
                );
            }
        }
        // when hitting the u key while inside of the root directory
        // We are moving the cursor down just to have a non-default selection
        app.process_events(&mut terminal, into_keys(b"ju".iter()))?;
        {
            assert_eq!(
                app.traversal.root_index, app.state.root,
                "it keeps the root - it can't go further up"
            );
            assert_eq!(
                node_by_name(&app, fixture_str(long_root)),
                node_by_index(&app, *app.state.selected.as_ref().unwrap()),
                "keeps the previous selection"
            );
        }
    }

    // Deletion
    {
        // when hitting the 'd' key (also move cursor back to start)
        app.process_events(&mut terminal, into_keys(b"k".iter()))?;
        let previously_selected_index = *app.state.selected.as_ref().unwrap();
        app.process_events(&mut terminal, into_keys(b"d".iter()))?;
        {
            assert_eq!(
                Some(1),
                app.window.mark_pane.as_ref().map(|p| p.marked().len()),
                "it marks only a single node",
            );
            assert!(
                app.window.mark_pane.as_ref().map_or(false, |p| p
                    .marked()
                    .contains_key(&previously_selected_index)),
                "it marks the selected node"
            );
            assert_eq!(
                app.state.selected.as_ref().unwrap().index(),
                app.state.entries[1].index.index(),
                "moves the cursor down one level to facilitate many markings in a row"
            );
        }

        // when hitting the 'd' key again
        {
            app.process_events(&mut terminal, into_keys(b"d".iter()))?;

            assert_eq!(
                Some(2),
                app.window.mark_pane.as_ref().map(|p| p.marked().len()),
                "it marks the currently selected, second node",
            );

            assert_eq!(
                app.state.selected.as_ref().unwrap().index(),
                app.state.entries[1].index.index(),
                "it could not advance the cursor, thus the newly marked item is still selected"
            );
        }

        // when hitting the 'd' key once again
        {
            app.process_events(&mut terminal, into_keys(b"d".iter()))?;

            assert_eq!(
                Some(1),
                app.window.mark_pane.as_ref().map(|p| p.marked().len()),
                "it toggled the previous selected entry off",
            );

            assert!(
                app.window.mark_pane.as_ref().map_or(false, |p| p
                    .marked()
                    .contains_key(&previously_selected_index)),
                "it leaves the first selected entry marked"
            );
        }
        // when hitting the spacebar (after moving up to the first entry)
        {
            app.process_events(&mut terminal, into_keys(b"k ".iter()))?;

            assert_eq!(
                None,
                app.window.mark_pane.as_ref().map(|p| p.marked().len()),
                "it toggles the item off",
            );

            assert_eq!(
                node_by_index(&app, previously_selected_index),
                node_by_index(&app, *app.state.selected.as_ref().unwrap()),
                "it does not advance the selection"
            );
        }
    }

    // Marking
    {
        // select something
        app.process_events(&mut terminal, into_keys(b" j ".iter()))?;
        assert_eq!(
            Some(false),
            app.window.mark_pane.as_ref().map(|p| p.has_focus()),
            "the marker pane starts out without focus",
        );

        assert_eq!(
            Some(2),
            app.window.mark_pane.as_ref().map(|p| p.marked().len()),
            "it has two items marked",
        );

        // when advancing the selection to the marker pane
        app.process_events(&mut terminal, into_keys(b"\t".iter()))?;
        {
            assert_eq!(
                Some(true),
                app.window.mark_pane.as_ref().map(|p| p.has_focus()),
                "after tabbing into it, it has focus",
            );
        }

        // TODO: a bunch of additional tests are missing (handling of markers, deselecting them)
        // Yes, caught me, no TDD for these things, just because in Rust it's not needed as things
        // tend to just work when they compile, and while experimenting, tests can be in the way.
        // However, if Dua should be more widely used, we need CI and these tests written.
    }

    Ok(())
}
