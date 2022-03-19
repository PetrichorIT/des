use ndl::*;

fn main() -> std::io::Result<()> {
    let workspaces: Vec<String> = if std::env::args().len() >= 2 {
        std::env::args().skip(1).collect()
    } else {
        vec![std::env::current_dir()?.to_str().unwrap().to_string()]
    };

    for workspace in workspaces {
        if matches!(&workspace[..], "help" | "h" | "-h" | "-help" | "--help") {
            println!("ndl [..]");
            println!("A parser and typechecker for ndl workspaces.\n");

            println!("USAGE:");
            println!("\tndl <path...>");
            println!();
            println!("\tIf no paths are given the current working directory ");
            println!("\twill be used as the workspace.");
            return Ok(());
        }

        println!("Workspace '{}'", workspace);
        println!();

        let mut resolver = NdlResolver::new(&workspace).unwrap();
        let (g, has_err, par_files) = resolver.run_cached().unwrap();

        println!(
            "> {} errors and found {} par files",
            if has_err { "has" } else { "free of" },
            par_files.len()
        );

        let g = g.to_owned();

        // Modules
        for module in g.modules {
            println!("{}", module)
        }

        for network in g.networks {
            println!("{}", network)
        }
    }

    Ok(())
}
