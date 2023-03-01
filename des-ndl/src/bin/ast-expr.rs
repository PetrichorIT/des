use des_ndl::ast::*;
use des_ndl::*;

const TEXT: &str = "
module A {
    gates {
        in @input,
        out @output,
    }

    submodules {
        in: In,
        out[1]: Out,
    }

    connections {
        in --> LaLink --> out
    }
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = ModuleStmt::parse(&buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
