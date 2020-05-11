use super::*;
use shipyard::*;

// Ordered first in tuple so it takes ordering precedence
type SiblingID = (Ordered, ID);

/// Managed by the tree_indexing system to provide more concise info for walking the tree
#[derive(Debug)]
pub struct SiblingIndex {
    pub parent_node: ID,
    pub ordered_node: SiblingID,
    pub prev_sibling: Option<SiblingID>,
    pub next_sibling: Option<SiblingID>,
}

/// Managed by the tree_indexing system to provide more concise info for walking the tree
#[derive(Debug)]
pub struct ParentIndex {
    pub children: Vec<SiblingID>,
}

/// Indexes tree ChildOf and Ordering components into more helpful between nodes
pub fn tree_indexing(
    (v_entities, v_child_of, mut vm_sibling_index, mut vm_parent_index): (
        EntitiesView,
        View<ChildOf>,
        ViewMut<SiblingIndex>,
        ViewMut<ParentIndex>,
    ),
) {
    // iff ChildOf was completely deleted (does not include "removed")
    v_child_of
        .deleted()
        .into_iter()
        .map(|(id, _)| id)
        .for_each(|deleted_id: &ID| {
            unlink_child(&mut vm_sibling_index, &mut vm_parent_index, *deleted_id);
        });

    // iff ChildOf is completely new component
    v_child_of.inserted().iter().with_id().into_iter().for_each(
        |(inserted_id, ChildOf(parent_id, child_order))| {
            insert_child_of(
                &v_entities,
                &v_child_of,
                &mut vm_sibling_index,
                &mut vm_parent_index,
                inserted_id,
                &child_order,
                parent_id.clone(),
            );
        },
    );

    // iff ChildOf was modified
    v_child_of.modified().iter().with_id().into_iter().for_each(
        |(modified_id, ChildOf(parent_id, child_order))| {
            dbg!(modified_id);

            // remove from parent
            unlink_child(&mut vm_sibling_index, &mut vm_parent_index, modified_id);

            // reinsert child
            insert_child_of(
                &v_entities,
                &v_child_of,
                &mut vm_sibling_index,
                &mut vm_parent_index,
                modified_id,
                &child_order,
                parent_id.clone(),
            );
        },
    );
}

fn insert_child_of(
    v_entities: &EntitiesView,
    v_child_of: &View<ChildOf>, // needed for creating parent node indexes, since parents do not need a ChildOf component
    vm_sibling_index: &mut ViewMut<SiblingIndex>,
    vm_parent_index: &mut ViewMut<ParentIndex>,
    child_id: ID,
    child_order: &Ordered, // used to position between siblings
    parent_id: ID,         // insert to this parent
) {
    // parent: insert into list at correct location,
    // find next index and previous index and update their sibling references respectively
    let parent_index: &mut ParentIndex = {
        if let Ok(parent_index) = vm_parent_index.try_get(parent_id) {
            parent_index
        } else {
            let mut children = v_child_of
                .iter()
                .filter(|ChildOf(child_parent_id, _)| child_parent_id == &parent_id)
                .with_id()
                .into_iter()
                .map(|(id, ChildOf(_, ref ordered))| -> SiblingID { (ordered.clone(), id) })
                .collect::<Vec<SiblingID>>();

            children.sort();

            // we need to create their SiblingIndex components
            for (idx, child) in children.iter().enumerate() {
                // dbg!(child);
                v_entities.add_component(
                    &mut *vm_sibling_index,
                    SiblingIndex {
                        next_sibling: if idx < children.len() - 1 {
                            Some(children[idx + 1])
                        } else {
                            None
                        },
                        prev_sibling: if idx > 0 {
                            Some(children[idx - 1])
                        } else {
                            None
                        },
                        ordered_node: child.clone(),
                        parent_node: parent_id,
                    },
                    child.1,
                );
            }

            // Good debugging spot if needed
            // for sibling in vm_sibling_index.iter() {
            //     dbg!(sibling);
            // }

            // parent has no parent or siblings
            v_entities.add_component(&mut *vm_parent_index, ParentIndex { children }, parent_id);

            vm_parent_index
                .try_get(parent_id)
                .expect("parent should have a parent index now")
        }
    };

    let siblings = &mut parent_index.children;

    let to_insert: SiblingID = (child_order.clone(), child_id);
    if siblings.binary_search(&to_insert).is_err() {
        // didn't find the sibling_id (ord + id) combo in siblings,
        // this could mean that either the Ordered value changed, or
        // this could mean that the entity is not present in the sibling list
        // at all.

        // remove our id, just in case it was just an "Ordered" change
        siblings.retain(|(_, id)| id != &child_id);

        // "insert_at" points to the index of the element after
        // "insert_at - 1" points to the index of the previous element
        let insert_at = {
            siblings
                .binary_search(&to_insert)
                .expect_err("existing child")
        };

        let (prev_node_opt, next_node_opt) = {
            (
                if insert_at > 0 {
                    // we have an element before to update (which becomes our previous node)
                    Some((&siblings)[insert_at - 1].clone())
                } else {
                    None
                },
                if insert_at < siblings.len() {
                    // we have an element after to update (which becomes our next node)
                    Some((&siblings)[insert_at].clone())
                } else {
                    None
                },
            )
        };

        // insert node into children as final modification to siblings
        siblings.insert(insert_at, to_insert);

        // update references
        if let Some(prev_node) = prev_node_opt {
            // prev node should point at inserted node as next
            (vm_sibling_index.get(prev_node.1)).next_sibling = Some(to_insert);
        }

        if let Some(next_node) = next_node_opt {
            // next node should point at inserted node as prev
            (vm_sibling_index.get(next_node.1)).prev_sibling = Some(to_insert);
        }

        v_entities.add_component(
            vm_sibling_index,
            SiblingIndex {
                ordered_node: to_insert,
                next_sibling: next_node_opt,
                prev_sibling: prev_node_opt,
                parent_node: parent_id,
            },
            child_id,
        );
    }
}

fn unlink_child(
    vm_sibling_index: &mut ViewMut<SiblingIndex>,
    vm_parent_index: &mut ViewMut<ParentIndex>,
    child: ID,
) {
    let (parent_id, t_prev_sibling, t_next_sibling) = {
        let child_index = vm_sibling_index.get(child);
        (
            child_index.parent_node,
            child_index.prev_sibling.clone(),
            child_index.next_sibling.clone(),
        )
    };

    // parent: remove T from children
    let parent_index = vm_parent_index.get(parent_id);
    parent_index.children.retain(|(_, id)| id != &child);

    if let Some(prev_sibling_id) = t_prev_sibling {
        // prevsibling: set nextsibling to T's nextsibling
        let mut prev_sibling_index: &mut SiblingIndex = vm_sibling_index.get(prev_sibling_id.1);
        prev_sibling_index.next_sibling = t_next_sibling;
    }

    if let Some(next_sibling_id) = t_next_sibling {
        // nextsibling: set prevsibling to T's prevsibling
        let mut next_sibling_index: &mut SiblingIndex = vm_sibling_index.get(next_sibling_id.1);
        next_sibling_index.prev_sibling = t_prev_sibling;
    }

    vm_sibling_index.delete(child);
}
