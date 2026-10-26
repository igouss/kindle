use serde::Serialize;

use crate::show::{Link, Shown};

const CHILD: &str = "parent-child";
const BLOCKS: &str = "blocks";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Member {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Epic {
    pub id: String,
    pub title: String,
    pub beads: Vec<Member>,
    pub blocks: Vec<[String; 2]>,
}

pub fn root(bead: &Shown) -> String {
    match &bead.parent {
        Some(parent) => parent.clone(),
        None => bead.id.clone(),
    }
}

pub fn children(epic: &Shown) -> Vec<Member> {
    epic.dependents
        .iter()
        .filter(|link: &&Link| link.dependency_type == CHILD)
        .map(|link: &Link| Member {
            id: link.id.clone(),
            title: link.title.clone(),
            status: link.status.clone(),
        })
        .collect()
}

pub fn assemble(epic: &Shown, children: &[Shown]) -> Epic {
    let mut beads: Vec<Member> = self::children(epic);
    if beads.is_empty() {
        beads.push(Member {
            id: epic.id.clone(),
            title: epic.title.clone(),
            status: epic.status.clone(),
        });
    }
    let blocks: Vec<[String; 2]> = children
        .iter()
        .flat_map(|child: &Shown| {
            child
                .dependencies
                .iter()
                .filter(|link: &&Link| {
                    link.dependency_type == BLOCKS && beads.iter().any(|m: &Member| m.id == link.id)
                })
                .map(|link: &Link| [link.id.clone(), child.id.clone()])
        })
        .collect();
    Epic {
        id: epic.id.clone(),
        title: epic.title.clone(),
        beads,
        blocks,
    }
}

#[cfg(test)]
mod tests;
