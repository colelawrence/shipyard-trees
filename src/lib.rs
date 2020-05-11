//! # Tree
//!
//! Tree provides a very simple way to manipulate the ordering of elements in a CRDT fashion enabling
//! a minimal amount of effort to reorder entities and resolve multiple concurrent reorderings.
//!
//! This is intended to be used alongside an additional decorating system which can index the child positions
//! in the tree whenever a movement occurs and to keep everything in check.
//!
//! So, you would have a tree_reordering system (for managing reordering commands operating on the tree components) and
//! a tree_indexing system for updating the tree_index components.
//!
//! The Advantages to this approach are to
//!  - simplify the source of truth for more reliable ser & de
//!  - Reduce the size of the seriallized form
//!  - Less blocking systems (if something only cares that the ChildOf / Ordering has changed and the system does not
//!    look at the indexed outputs, then it can run concurrently with the tree_indexing system)
pub mod indexing;
pub mod node;
pub mod reordering;

pub use indexing::*;
pub use node::*;
pub use reordering::*;

type ID = shipyard::EntityId;

/// A spattering of tests to check things
#[cfg(test)]
mod tests {
    use super::*;
    use shipyard::*;

    #[test]
    fn test_indexing() {
        let world = World::new();
        world.add_unique(ReorderCommands(vec![]));
        world.run(|mut vm_child_of: ViewMut<ChildOf>| {
            vm_child_of.update_pack();
        });

        world
            .add_workload("tests")
            .with_system(system!(reordering::tree_reordering))
            .with_system(system!(indexing::tree_indexing))
            .build();

        let (a, a1, a2, a3, a6) = world.run(
            |mut entities: EntitiesViewMut, mut vm_child_of: ViewMut<ChildOf>| {
                let a = entities.add_entity((), ());
                let a6 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(6)));
                let a3 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(3)));
                let a1 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(1)));
                let a2 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(2)));
                (a, a1, a2, a3, a6)
            },
        );

        world.run_default();

        world.run(
            |v_parent_index: View<ParentIndex>, v_sibling_index: View<SiblingIndex>| {
                v_sibling_index
                    .try_get(a)
                    .expect_err("should not have sibling data");

                let c: &ParentIndex = v_parent_index.try_get(a).expect("has children");

                assert_eq!(
                    parent_children_ids(c),
                    vec![a1, a2, a3, a6],
                    "should be in order"
                );
            },
        );

        let (a1b, a1b1, a0, a4, a7) = world.run(
            |mut entities: EntitiesViewMut, mut vm_child_of: ViewMut<ChildOf>| {
                let a7 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(7)));
                let a0 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(0)));
                let a4 = entities.add_entity(&mut vm_child_of, ChildOf(a, Ordered::hinted(4)));
                let a1b = entities.add_entity(&mut vm_child_of, ChildOf(a1, Ordered::hinted(4)));
                let a1b1 = entities.add_entity(&mut vm_child_of, ChildOf(a1b, Ordered::hinted(1)));
                (a1b, a1b1, a0, a4, a7)
            },
        );

        world.run_default();

        world.run(
            |v_parent_index: View<ParentIndex>, v_sibling_index: View<SiblingIndex>| {
                v_sibling_index
                    .try_get(a)
                    .expect_err("should not have sibling data");

                let a1b_sib: &SiblingIndex = v_sibling_index
                    .try_get(a1b)
                    .expect("should have sibling data");

                assert_eq!(a1b_sib.next_sibling, None, "only child");
                assert_eq!(a1b_sib.prev_sibling, None, "only child");

                assert_eq!(
                    parent_children_ids(v_parent_index.try_get(a1b).expect("has children")),
                    vec![a1b1],
                    "should have the one child"
                );

                assert_eq!(
                    parent_children_ids(v_parent_index.try_get(a).expect("has children")),
                    vec![a0, a1, a2, a3, a4, a6, a7],
                    "should be in order"
                );
            },
        );

        // test removing and deleting ChildOf components

        world.run(|mut vm_child_of: ViewMut<ChildOf>| {
            // remove should not be used
            &mut vm_child_of.delete(a7);
            &mut vm_child_of.delete(a4);
            &mut vm_child_of.delete(a0);
            &mut vm_child_of.delete(a1b);
        });

        world.run_default();

        world.run(
            |v_parent_index: View<ParentIndex>, v_sibling_index: View<SiblingIndex>| {
                v_sibling_index
                    .try_get(a1b)
                    .expect_err("should not have sibling data");

                assert_eq!(
                    parent_children_ids(v_parent_index.try_get(a1b).expect("has children")),
                    vec![a1b1],
                    "should have the one child"
                );

                assert_eq!(
                    parent_children_ids(v_parent_index.try_get(a).expect("has children")),
                    vec![a1, a2, a3, a6],
                    "should be in order without deleted entities"
                );
            },
        );
    }

    fn parent_children_ids(pi: &ParentIndex) -> Vec<EntityId> {
        pi.children.iter().map(|c| c.1).collect()
    }
}
