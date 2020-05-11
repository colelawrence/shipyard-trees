use crate::*;
use shipyard::*;

#[derive(Clone)]
pub enum ReorderCmd {
    Move { entity: ID, between: (ID, ID) },
}

/// Unique storage of commands for reordering
pub struct ReorderCommands(pub Vec<ReorderCmd>);

pub fn tree_reordering(
    (mut commands, mut child_of): (UniqueViewMut<ReorderCommands>, ViewMut<ChildOf>),
) {
    for cmd in commands.0.drain(..) {
        match cmd {
            ReorderCmd::Move {
                entity: target,
                between: (a, b),
            } => {
                let (target_parent, target_after, target_before) = {
                    // check that a & b are both of the same parent
                    let ChildOf(a_of, a_ord) = (&child_of).get(a);
                    let ChildOf(b_of, mut b_ord) = (&child_of).get(b);

                    if a_of != b_of {
                        eprintln!("reorder between targets two elements of different parents target={:?}; {:?} vs {:?}", target, a_of, b_of);
                        // Future: take a_of and try to insert directly after
                        // we would have to look up all children a_of
                        b_ord = (&child_of)
                            .iter()
                            .filter(|ChildOf(e_of, e_ord)| e_of == a_of && a_ord < e_ord)
                            .fold(
                                // default to farthest away which will be immediately replaced
                                MAX_ORDERED,
                                |after, ChildOf(_, e_ord)| {
                                    if *e_ord < after {
                                        // e_ord is closer than previous after
                                        *e_ord
                                    } else {
                                        after
                                    }
                                },
                            );
                    }

                    (*a_of, a_ord.clone(), b_ord.clone())
                };

                // update position of the child
                let mut target_child_of: &mut ChildOf = (&mut child_of).get(target);
                target_child_of.0 = target_parent;
                target_child_of.1 = Ordered::between(&target_after, &target_before);
            }
        }
    }
}
