use des_ndl::error::*;
use des_ndl::*;

fn main() -> RootResult<()> {
    let ctx = Context::load("des-ndl/src/bin/case-0/main.ndl")?;

    // println!("{:#?}", tbl);
    // println!("{:#?}", tbl2);
    println!("{:#?}", ctx.ir);
    Ok(())
}
