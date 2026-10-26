use lute_core::ast::{walk, Node, WalkStatus};
use lute_core::blockdom::blockdom2tree;
use lute_core::options::ParseOptions;
fn main() {
    let html = std::env::args().nth(1).unwrap_or_default();
    let tree = blockdom2tree(&html, ParseOptions::new());
    let mut out = String::new();
    walk(&tree.root(), &mut |n: Node, entering| {
        if entering {
            out.push_str(&format!(
                "{:?} data={:?} tokens={:?}\n",
                n.typ(),
                n.ial_attr("id"),
                n.tokens_str()
            ));
        }
        WalkStatus::WalkContinue
    });
    print!("{}", out);
}
