use des_ndl::*;

const TEXT: &str = "
{
    ident,
    identb,
    identc,
    idente,
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = Delimited::<Punctuated<Ident, Comma>>::parse_from(Delimiter::Brace, &buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
