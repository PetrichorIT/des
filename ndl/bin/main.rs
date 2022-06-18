use std::{path::PathBuf, str::FromStr};

use ndl::*;
use structopt::StructOpt;

#[derive(Debug)]
enum CompilationTarget {
    Parse,
    TyChk,
}

impl FromStr for CompilationTarget {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "parse" | "Parse" => Ok(Self::Parse),
            "tychk" | "TyChk" => Ok(Self::TyChk),
            _ => Err("Invalid keyword"),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "ndl", about = "A parser for network description files.")]
struct Opt {
    #[structopt(short, long, help = "Prevents ndl from printing the result.")]
    quiet: bool,

    #[structopt(short, long, help = "Suppreses errors.")]
    suppres_errors: bool,

    #[structopt(
        short,
        long,
        help = "The target of the compilation process.",
        default_value = "TyChk"
    )]
    target: CompilationTarget,

    #[structopt(
        short = "v",
        long = "verbose",
        help = "Defines a output directory which stores incremental results."
    )]
    verbose_with_dir: Option<String>,

    #[structopt(
        short,
        long,
        help = "The number of times the process is repeated.",
        default_value = "1"
    )]
    repeat: usize,

    #[structopt(name = "Workspaces")]
    workspaces: Vec<String>,
}

fn main() -> std::io::Result<()> {
    let Opt {
        quiet,
        suppres_errors,
        target,
        verbose_with_dir,
        repeat: times,
        mut workspaces,
    } = Opt::from_args();

    if workspaces.is_empty() {
        workspaces.push(
            std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        );
    }

    // if workspaces.is_empty() {
    //     workspaces.push(std::env::current_dir()?.to_str().unwrap().to_string());
    // }

    let opts = NdlResolverOptions {
        silent: suppres_errors,
        verbose: verbose_with_dir.is_some(),
        verbose_output_dir: verbose_with_dir.map(PathBuf::from).unwrap_or_default(),
        desugar: matches!(target, CompilationTarget::TyChk),
        tychk: matches!(target, CompilationTarget::TyChk),
    };

    for workspace in workspaces {
        println!("Workspace '{}'", workspace);
        println!();

        // For debug
        for _ in 0..times {
            let mut resolver = NdlResolver::new_with(&workspace, opts.clone()).unwrap();
            let (g, errs, par_files) = resolver.run_cached().unwrap();
            let has_err = errs.count() != 0;

            let g = g.to_owned();

            // Modules
            if !quiet {
                println!(
                    "> {} errors and found {} par files",
                    if has_err { "has" } else { "free of" },
                    par_files.len()
                );

                for module in g.modules {
                    println!("{}", module)
                }

                for network in g.subsystems {
                    println!("{}", network)
                }
            }
        }
    }

    Ok(())
}
