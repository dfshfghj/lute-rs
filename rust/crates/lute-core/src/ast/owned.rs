//! Owning node handles for the public AST API.
use super::{NodeOwner, NodeType, RawNode};
use std::fmt;

/// A stable node handle retaining all linked node arenas.
/// Clones share the same node; field reads return independent snapshots.
///
/// ```
/// use lute_core::ast::Tree;
/// let node = { let tree = Tree::new(); tree.root() };
/// assert!(node.parent().is_none());
/// ```
///
/// Node handles retain storage and cannot be copied implicitly:
/// ```compile_fail
/// fn requires_copy<T: Copy>() {}
/// requires_copy::<lute_core::ast::Node>();
/// ```
///
/// The engine's non-owning handle is not part of the public API:
/// ```compile_fail
/// use lute_core::ast::RawNode;
/// ```
#[derive(Clone)]
pub struct Node {
    raw: RawNode,
    owner: NodeOwner,
}
impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Formatting invokes the caller's writer, which may mutate another
        // handle to this node. Do not keep references into its data alive.
        let typ = self.typ();
        let id = self.id();
        f.debug_struct("Node")
            .field("type", &typ)
            .field("id", &id)
            .finish()
    }
}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl Eq for Node {}
impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.raw, state);
    }
}
impl Node {
    pub(crate) fn from_raw(raw: RawNode) -> Self {
        Self {
            raw,
            owner: raw.owner(),
        }
    }
    pub(crate) fn raw(&self) -> RawNode {
        self.raw
    }
    pub fn typ(&self) -> NodeType {
        self.raw.typ()
    }
    pub fn close(&self) -> bool {
        self.raw.close()
    }
    pub fn parent_is(&self, types: &[NodeType]) -> bool {
        self.raw.parent_is(types)
    }
    pub fn accept_lines(&self) -> bool {
        self.raw.accept_lines()
    }
    pub fn can_contain(&self, node_type: NodeType) -> bool {
        self.raw.can_contain(node_type)
    }
    pub fn dump(&self) -> String {
        self.raw.dump()
    }
    pub fn revive_from_marker(&self, marker: u8) {
        self.raw.revive_from_marker(marker)
    }
    pub fn set_typ(&self, typ: NodeType) {
        self.raw.set_typ(typ)
    }
    pub fn id(&self) -> String {
        self.raw.id_owned()
    }
    pub fn id_owned(&self) -> String {
        self.raw.id_owned()
    }
    pub fn set_id(&self, id: impl Into<String>) {
        self.raw.set_id(id)
    }
    pub fn tokens(&self) -> Vec<u8> {
        self.raw.tokens_vec()
    }
    pub fn tokens_vec(&self) -> Vec<u8> {
        self.raw.tokens_vec()
    }
    pub fn tokens_str(&self) -> String {
        self.raw.tokens_str()
    }
    pub fn set_tokens(&self, tokens: impl Into<Vec<u8>>) {
        self.raw.set_tokens(tokens)
    }
    pub fn append_tokens(&self, tokens: &[u8]) {
        self.raw.append_tokens(tokens)
    }
    pub fn prepend_tokens(&self, tokens: &[u8]) {
        self.raw.prepend_tokens(tokens)
    }
    pub fn token_len(&self) -> usize {
        self.raw.token_len()
    }
    pub fn parent(&self) -> Option<Node> {
        self.raw.parent().map(Self::from_raw)
    }
    pub fn previous(&self) -> Option<Node> {
        self.raw.previous().map(Self::from_raw)
    }
    pub fn next(&self) -> Option<Node> {
        self.raw.next().map(Self::from_raw)
    }
    pub fn first_child(&self) -> Option<Node> {
        self.raw.first_child().map(Self::from_raw)
    }
    pub fn last_child(&self) -> Option<Node> {
        self.raw.last_child().map(Self::from_raw)
    }
    pub fn children(&self) -> Children {
        Children {
            next: self.raw.first_child(),
            owner: self.owner.clone(),
        }
    }
    pub fn append_child(&self, child: &Node) {
        self.raw.append_child(child.raw)
    }
    pub fn prepend_child(&self, child: &Node) {
        self.raw.prepend_child(child.raw)
    }
    pub fn insert_before(&self, sibling: &Node) {
        self.raw.insert_before(sibling.raw)
    }
    pub fn insert_after(&self, sibling: &Node) {
        self.raw.insert_after(sibling.raw)
    }
    pub fn unlink(&self) {
        self.raw.unlink()
    }
    pub fn child_by_type(&self, child_type: NodeType) -> Option<Node> {
        self.raw.child_by_type(child_type).map(Self::from_raw)
    }
    pub fn children_by_type(&self, child_type: NodeType) -> Vec<Node> {
        self.raw
            .children_by_type(child_type)
            .into_iter()
            .map(Self::from_raw)
            .collect()
    }
    pub fn list(&self) -> Vec<Node> {
        self.raw.list().into_iter().map(Self::from_raw).collect()
    }
    pub fn text(&self) -> String {
        self.raw.text()
    }
    pub fn text_len(&self) -> usize {
        self.raw.text_len()
    }
    pub fn escape_marker_content(&self) -> String {
        self.raw.escape_marker_content()
    }
    pub fn content(&self) -> String {
        self.raw.content()
    }
    pub fn is_block(&self) -> bool {
        self.raw.is_block()
    }
    pub fn is_container_block(&self) -> bool {
        self.raw.is_container_block()
    }
    pub fn is_marker(&self) -> bool {
        self.raw.is_marker()
    }
    pub fn ial_raw(&self) -> Vec<(String, String)> {
        self.raw.ial_raw()
    }
    pub fn set_ial_attr(&self, name: &str, value: &str) {
        self.raw.set_ial_attr(name, value)
    }
    pub fn remove_ial_attr(&self, name: &str) {
        self.raw.remove_ial_attr(name)
    }
    pub fn remove_ial_attrs_by_prefix(&self, prefix: &str) {
        self.raw.remove_ial_attrs_by_prefix(prefix)
    }
    pub fn is_text_mark_type(&self, typ: &str) -> bool {
        self.raw.is_text_mark_type(typ)
    }
    pub fn is_next_same_inline_memo(&self) -> bool {
        self.raw.is_next_same_inline_memo()
    }
    pub fn is_same_text_mark_type(&self, other: &Node) -> bool {
        self.raw.is_same_text_mark_type(other.raw)
    }
    pub fn sort_text_mark_data_types(&self) {
        self.raw.sort_text_mark_data_types()
    }
    pub fn ial_attr(&self, name: &str) -> String {
        self.raw.ial_attr(name)
    }
    pub fn effective_task_list_item_marker(&self) -> String {
        self.raw.effective_task_list_item_marker()
    }
}
/// Child iterator retaining its arena when the source node is dropped.
pub struct Children {
    next: Option<RawNode>,
    owner: NodeOwner,
}
impl Iterator for Children {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.next?;
        self.next = raw.next();
        Some(Node {
            raw,
            owner: self.owner.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{walk, Tree, WalkStatus};

    #[test]
    fn nodes_and_navigation_outlive_the_tree() {
        let (root, child) = {
            let mut tree = Tree::new();
            let root = tree.root();
            let child = tree.new_node(NodeType::NodeText);
            child.set_tokens(b"retained".to_vec());
            root.append_child(&child);
            (root, child)
        };
        assert_eq!("retained", child.text());
        assert_eq!(Some(root.clone()), child.parent());
        drop(root);
        let parent = child.parent().unwrap();
        drop(child);
        assert_eq!("retained", parent.first_child().unwrap().text());
    }

    #[test]
    fn child_iterators_and_saved_walk_nodes_retain_arenas() {
        let mut tree = Tree::new();
        let child = tree.new_node(NodeType::NodeText);
        child.set_tokens(b"saved".to_vec());
        tree.root().append_child(&child);
        let mut children = tree.root().children();
        let mut saved = None;
        walk(&tree.root(), &mut |node, entering| {
            if entering && node.typ() == NodeType::NodeText {
                saved = Some(node);
            }
            WalkStatus::WalkContinue
        });
        drop(child);
        drop(tree);
        assert_eq!("saved", children.next().unwrap().text());
        drop(children);
        assert_eq!("saved", saved.unwrap().text());
    }

    #[test]
    fn field_snapshots_survive_mutation_from_another_handle() {
        let mut tree = Tree::new();
        let node = tree.new_node(NodeType::NodeText);
        node.set_id("before");
        node.set_tokens(b"before".to_vec());
        let id = node.id();
        let tokens = node.tokens();
        let alias = node.clone();
        alias.set_id("after");
        alias.set_tokens(b"after".to_vec());
        drop(tree);
        drop(node);
        assert_eq!("before", id);
        assert_eq!(b"before", tokens.as_slice());
        assert_eq!("after", alias.id());
        assert_eq!(b"after", alias.tokens().as_slice());
    }

    #[test]
    fn cross_tree_link_keeps_both_sides_alive() {
        let child = {
            let mut source = Tree::new();
            let child = source.new_node(NodeType::NodeText);
            child.set_tokens(b"linked".to_vec());
            let target = Tree::new();
            target.root().append_child(&child);
            child
        };
        assert_eq!("linked", child.text());
        let parent = child.parent().unwrap();
        drop(child);
        assert_eq!("linked", parent.first_child().unwrap().text());
    }
}
