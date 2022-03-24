use ndl::*;

fn main() -> std::io::Result<()> {
    let workspaces: Vec<String> = if std::env::args().len() >= 2 {
        std::env::args().skip(1).collect()
    } else {
        vec![std::env::current_dir()?.to_str().unwrap().to_string()]
    };

    let options: Vec<&String> = workspaces.iter().filter(|w| w.starts_with("-")).collect();
    let workspaces: Vec<&String> = workspaces.iter().filter(|w| !w.starts_with("-")).collect();

    let quiet = options.iter().any(|o| ["-q", "--quiet"].contains(&&o[..]));
    let help = options
        .iter()
        .any(|o| ["-h", "-help", "--help"].contains(&&o[..]));

    if help {
        println!("ndl [..]");
        println!("A parser and typechecker for ndl workspaces.\n");

        println!("USAGE:");
        println!("\tndl <path...>");
        println!();
        println!("\tIf no paths are given the current working directory ");
        println!("\twill be used as the workspace.");
        return Ok(());
    }

    for workspace in workspaces {
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
        if !quiet {
            for module in g.modules {
                println!("{}", module)
            }

            for network in g.networks {
                println!("{}", network)
            }
        }
    }

    Ok(())
}
