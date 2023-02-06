use des_ndl::*;

const TEXT: &str = "
submodules {
    parent: P,
    child[6]: Child,
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = SubmodulesStmt::parse(&buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
