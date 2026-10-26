use super::*;

fn link(id: &str, status: &str, kind: &str) -> Link {
    Link {
        id: id.to_owned(),
        title: format!("title {id}"),
        status: status.to_owned(),
        dependency_type: kind.to_owned(),
    }
}

fn shown(id: &str, parent: Option<&str>, dependencies: Vec<Link>, dependents: Vec<Link>) -> Shown {
    Shown {
        id: id.to_owned(),
        title: format!("title {id}"),
        status: String::from("open"),
        parent: parent.map(str::to_owned),
        description: None,
        dependencies,
        dependents,
    }
}

fn member(id: &str, status: &str) -> Member {
    Member {
        id: id.to_owned(),
        title: format!("title {id}"),
        status: status.to_owned(),
    }
}

#[test]
fn a_child_shows_its_parent_and_a_bead_without_one_shows_itself() {
    assert_eq!(
        (
            root(&shown("e.1", Some("e"), vec![], vec![])),
            root(&shown("e", None, vec![], vec![]))
        ),
        (String::from("e"), String::from("e"))
    );
}

#[test]
fn the_children_are_the_parent_child_dependents_only() {
    let head: Shown = shown(
        "e",
        None,
        vec![],
        vec![
            link("e.1", "closed", "parent-child"),
            link("x", "open", "blocks"),
            link("e.2", "in_progress", "parent-child"),
        ],
    );
    assert_eq!(
        children(&head),
        [member("e.1", "closed"), member("e.2", "in_progress")]
    );
}

#[test]
fn blocking_edges_run_from_blocker_to_blocked_among_siblings_only() {
    let head: Shown = shown(
        "e",
        None,
        vec![],
        vec![
            link("e.1", "closed", "parent-child"),
            link("e.2", "open", "parent-child"),
            link("e.3", "open", "parent-child"),
        ],
    );
    let kids: [Shown; 3] = [
        shown(
            "e.1",
            Some("e"),
            vec![link("e", "open", "parent-child")],
            vec![],
        ),
        shown(
            "e.2",
            Some("e"),
            vec![
                link("e.1", "closed", "blocks"),
                link("other", "open", "blocks"),
            ],
            vec![],
        ),
        shown(
            "e.3",
            Some("e"),
            vec![
                link("e.1", "closed", "blocks"),
                link("e.2", "open", "blocks"),
            ],
            vec![],
        ),
    ];
    assert_eq!(
        assemble(&head, &kids).blocks,
        [
            [String::from("e.1"), String::from("e.2")],
            [String::from("e.1"), String::from("e.3")],
            [String::from("e.2"), String::from("e.3")]
        ]
    );
}

#[test]
fn a_bead_with_no_children_is_an_epic_of_one() {
    let lone: Shown = Shown {
        status: String::from("in_progress"),
        ..shown("x", None, vec![], vec![])
    };
    assert_eq!(
        assemble(&lone, &[]),
        Epic {
            id: String::from("x"),
            title: String::from("title x"),
            beads: vec![member("x", "in_progress")],
            blocks: vec![],
        }
    );
}
