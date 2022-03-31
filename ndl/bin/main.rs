use std::path::PathBuf;

use ndl::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "ndl", about = "A parser for network description files.")]
struct Opt {
    #[structopt(short, long, help = "Prevents ndl from printing the result.")]
    quiet: bool,

    #[structopt(
        short = "v",
        long = "verbose",
        help = "Defines a output directory which stores incremental results."
    )]
    verbose_with_dir: Option<String>,

    #[structopt(name = "Workspaces")]
    workspaces: Vec<String>,
}

fn main() -> std::io::Result<()> {
    let Opt {
        quiet,
        verbose_with_dir,
        mut workspaces,
    } = Opt::from_args();

    if workspaces.is_empty() {
        workspaces.push(std::env::current_dir()?.to_str().unwrap().to_string());
    }

    let opts = NdlResolverOptions {
        silent: true,
        verbose: verbose_with_dir.is_some(),
        verbose_output_dir: verbose_with_dir.map(PathBuf::from).unwrap_or_default(),
    };

    for workspace in workspaces {
        println!("Workspace '{}'", workspace);
        println!();

        let mut resolver = NdlResolver::new_with(&workspace, opts.clone()).unwrap();
        let (g, errs, par_files) = resolver.run_cached().unwrap();
        let has_err = errs.count() == 0;

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
