use lute_core::lute::Lute;

fn dump(dom: &lute_core::dom::Dom, idx: usize, depth: usize) {
    let n = &dom.nodes[idx];
    println!(
        "{}{:?} {:?} {:?}",
        "  ".repeat(depth),
        n.typ,
        n.data,
        n.attrs
    );
    for &c in &n.children {
        dump(dom, c, depth + 1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let html = args.get(1).cloned().unwrap_or_default();
    let dom = lute_core::dom::parse(&html);
    dump(&dom, dom.root, 0);
    let lute = Lute::new();
    let md = lute.vditor_ir_dom2md(&html);
    println!("MD={:?}", md);
}
