use des_ndl::*;

const TEXT: &str = "
link FastLink {
    a: 123,
    b: 1.0,
    c: \"str\"
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = Link::parse(&buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
