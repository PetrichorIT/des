use des_ndl::*;

fn main() -> Result<()> {
    let ctx = Context::load("des-ndl/src/bin/case-0/main.ndl")?;

    // println!("{:#?}", tbl);
    // println!("{:#?}", tbl2);
    println!("{:#?}", ctx.ir);
    Ok(())
}
