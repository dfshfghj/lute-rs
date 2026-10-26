use lute_core::options::ParseOptions;
use lute_core::render::json::render_json;
fn main() {
    let dom = std::env::args().nth(1).unwrap();
    let tree = lute_core::html2md::html2tree(&dom, ParseOptions::new());
    println!(
        "{}",
        render_json(&tree, &lute_core::options::RenderOptions::new())
    );
}
