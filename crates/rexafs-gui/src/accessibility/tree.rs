//! Linear-time accessibility diffs, independent of the native window adapter.

use std::collections::HashMap;

use accesskit::TreeUpdate;

pub(super) fn changes_since(
    current: &TreeUpdate,
    previous: Option<&TreeUpdate>,
) -> Option<TreeUpdate> {
    let Some(previous) = previous.filter(|previous| previous.tree_id == current.tree_id) else {
        return Some(current.clone());
    };
    // Most frames only move the plot pointer. Avoid allocating even an index
    // when the controls are unchanged and retain their usual traversal order.
    if current == previous {
        return None;
    }
    let old_nodes: HashMap<_, _> = previous
        .nodes
        .iter()
        .map(|(id, node)| (*id, node))
        .collect();
    let nodes: Vec<_> = current
        .nodes
        .iter()
        .filter(|(id, node)| old_nodes.get(id).copied() != Some(node))
        .cloned()
        .collect();
    let tree_changed = previous.tree != current.tree;
    if nodes.is_empty() && !tree_changed && previous.focus == current.focus {
        return None;
    }
    Some(TreeUpdate {
        nodes,
        tree: tree_changed.then(|| current.tree.clone()).flatten(),
        tree_id: current.tree_id,
        focus: current.focus,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Node, NodeId, Role, TreeId, TreeInfo};

    fn sample() -> TreeUpdate {
        let mut root = Node::new(Role::Window);
        root.set_children([NodeId(2), NodeId(3)]);
        let mut button = Node::new(Role::Button);
        button.set_label("Import");
        let mut input = Node::new(Role::TextInput);
        input.set_value("9000");
        TreeUpdate {
            nodes: vec![(NodeId(1), root), (NodeId(2), button), (NodeId(3), input)],
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: NodeId(1),
        }
    }

    #[test]
    fn activation_receives_the_complete_tree() {
        let current = sample();
        assert_eq!(changes_since(&current, None), Some(current));
    }

    #[test]
    fn unchanged_redraws_do_not_send_native_updates() {
        let previous = sample();
        let mut current = previous.clone();
        assert!(changes_since(&current, Some(&previous)).is_none());
        current.nodes.reverse(); // AccessKit node order has no meaning.
        assert!(changes_since(&current, Some(&previous)).is_none());
    }

    #[test]
    fn edited_controls_send_only_the_changed_node() {
        let previous = sample();
        let mut current = previous.clone();
        current.nodes[2].1.set_value("9010");
        let update = changes_since(&current, Some(&previous)).unwrap();
        assert_eq!(update.nodes, vec![current.nodes[2].clone()]);
        assert!(update.tree.is_none());
        assert_eq!(update.focus, current.focus);
    }

    #[test]
    fn focus_changes_are_delivered_without_node_changes() {
        let previous = sample();
        let mut current = previous.clone();
        current.focus = NodeId(3);
        let update = changes_since(&current, Some(&previous)).unwrap();
        assert!(update.nodes.is_empty());
        assert_eq!(update.focus, NodeId(3));
    }

    #[test]
    fn removing_a_control_updates_its_parent() {
        let previous = sample();
        let mut current = previous.clone();
        current.nodes.pop();
        current.nodes[0].1.set_children([NodeId(2)]);
        let update = changes_since(&current, Some(&previous)).unwrap();
        assert_eq!(update.nodes, vec![current.nodes[0].clone()]);
    }

    #[test]
    fn adding_a_control_updates_both_parent_and_child() {
        let previous = sample();
        let mut current = previous.clone();
        current.nodes[0]
            .1
            .set_children([NodeId(2), NodeId(3), NodeId(4)]);
        current.nodes.push((NodeId(4), Node::new(Role::Button)));
        let update = changes_since(&current, Some(&previous)).unwrap();
        assert_eq!(
            update.nodes,
            vec![current.nodes[0].clone(), current.nodes[3].clone()]
        );
    }

    #[test]
    fn tree_metadata_changes_are_delivered() {
        let previous = sample();
        let mut current = previous.clone();
        current.tree.as_mut().unwrap().toolkit_name = Some("GPUI".into());
        let update = changes_since(&current, Some(&previous)).unwrap();
        assert_eq!(update.tree, current.tree);
        assert!(update.nodes.is_empty());
    }
}
