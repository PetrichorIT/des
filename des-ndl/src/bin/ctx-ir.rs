use des_ndl::error::Result;
use des_ndl::*;

fn main() -> Result<()> {
    let ctx = Context::load("des-ndl/src/bin/case-1/main.ndl")?;
    // println!("{:#?}", ctx.ir);
    println!("{:#?}", ctx.entry);
    Ok(())
}
