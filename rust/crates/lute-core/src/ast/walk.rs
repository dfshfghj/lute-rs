// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! 深度优先遍历。

use super::{Node, RawNode};

/// 对应 Go 版 ast.WalkStatus。数值必须与 Go 一致，JS 侧依赖它。
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WalkStatus {
    /// 不需要继续遍历。
    WalkStop = 0,
    /// 不要遍历子节点。
    WalkSkipChildren = 1,
    /// 继续遍历。
    WalkContinue = 2,
}

/// 使用深度优先算法遍历指定的树节点。
pub(crate) fn walk_raw<F>(node: RawNode, walker: &mut F) -> WalkStatus
where
    F: FnMut(RawNode, bool) -> WalkStatus,
{
    let ret = walker(node, true);
    if WalkStatus::WalkStop == ret {
        return ret;
    }

    if WalkStatus::WalkSkipChildren != ret {
        // 与 Go 版一致：每次循环体执行完再取 Next，
        // 因此遍历过程中 Unlink 当前节点会终止该层的后续遍历。
        let mut child = node.first_child();
        while let Some(c) = child {
            if WalkStatus::WalkStop == walk_raw(c, walker) {
                return WalkStatus::WalkStop;
            }
            child = c.next();
        }
    }

    walker(node, false)
}

/// Walk a tree, retaining every node passed to the callback.
pub fn walk<F>(node: &Node, walker: &mut F) -> WalkStatus
where
    F: FnMut(Node, bool) -> WalkStatus,
{
    walk_raw(node.raw(), &mut |raw, entering| {
        walker(Node::from_raw(raw), entering)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{NodeType, Tree};

    #[test]
    fn enters_and_leaves_in_order() {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        let p = tree.new_node_raw(NodeType::NodeParagraph);
        let t = tree.new_node_raw(NodeType::NodeText);
        root.append_child(p);
        p.append_child(t);

        let mut log = Vec::new();
        walk_raw(root, &mut |n, entering| {
            log.push((n.typ(), entering));
            WalkStatus::WalkContinue
        });

        assert_eq!(
            vec![
                (NodeType::NodeDocument, true),
                (NodeType::NodeParagraph, true),
                (NodeType::NodeText, true),
                (NodeType::NodeText, false),
                (NodeType::NodeParagraph, false),
                (NodeType::NodeDocument, false),
            ],
            log
        );
    }

    #[test]
    fn stop_terminates_walk() {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        for _ in 0..3 {
            let p = tree.new_node_raw(NodeType::NodeParagraph);
            root.append_child(p);
        }

        let mut visited = 0;
        walk_raw(root, &mut |_n, entering| {
            if entering {
                visited += 1;
                if 2 == visited {
                    return WalkStatus::WalkStop;
                }
            }
            WalkStatus::WalkContinue
        });
        assert_eq!(2, visited);
    }

    #[test]
    fn skip_children_still_leaves_node() {
        let mut tree = Tree::new();
        let root = tree.root_raw();
        let p = tree.new_node_raw(NodeType::NodeParagraph);
        let t = tree.new_node_raw(NodeType::NodeText);
        root.append_child(p);
        p.append_child(t);

        let mut seen_text = false;
        let mut left_paragraph = false;
        walk_raw(root, &mut |n, entering| {
            if NodeType::NodeText == n.typ() {
                seen_text = true;
            }
            if NodeType::NodeParagraph == n.typ() && !entering {
                left_paragraph = true;
            }
            if NodeType::NodeParagraph == n.typ() && entering {
                return WalkStatus::WalkSkipChildren;
            }
            WalkStatus::WalkContinue
        });
        assert!(!seen_text);
        assert!(left_paragraph);
    }
}
