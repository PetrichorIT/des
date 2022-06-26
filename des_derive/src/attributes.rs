use crate::common::Result;
use proc_macro_error::Diagnostic;
use proc_macro_error::Level;
use syn::AttributeArgs;
use syn::Lit;
use syn::Meta;
use syn::MetaNameValue;
use syn::NestedMeta;

pub struct Attr {
    pub workspace: Option<String>,
    pub overwrite_ident: Option<String>,
}

// Valid calls:
// #[NdlModule("workspace")]
// #[NdlModule(workspace = "workspace")]
// #[NdlModule(workspace = "w", ndl_ident = "Ident")] //ORDER IRRElEvANT
// #[NdlModule("workspace", "Ident")]

impl Attr {
    pub fn from_args(args: AttributeArgs) -> Result<Self> {
        let mut workspace = None;
        let mut overwrite_ident = None;

        let mut lit_list = true;
        let mut i = 0;
        for arg in args {
            match arg {
                NestedMeta::Lit(lit) => {
                    if i == 0 && lit_list {
                        // Workspace in lit_list
                        let val = match lit {
                            Lit::Str(str) => str.value(),
                            _ => return Err(Diagnostic::new(
                                Level::Error,
                                "First argument in macro attributes is the workspace. The workspace must be a string."
                                    .to_string(),
                            ))
                        };

                        workspace = Some(val);
                        i += 1;
                        continue;
                    }

                    if i == 1 && lit_list {
                        let val = match lit {
                            Lit::Str(str) => str.value(),
                            _ => return Err(Diagnostic::new(
                                Level::Error,
                                "Second argument in macro attributes is the ndl ident. The ident must be a string."
                                    .to_string(),
                            ))
                        };

                        overwrite_ident = Some(val);
                        i += 1;
                        continue;
                    }

                    // WARNING
                }
                NestedMeta::Meta(meta) => match meta {
                    Meta::NameValue(MetaNameValue { path, lit, .. }) => {
                        lit_list = false;
                        let path = match path.get_ident().map(|i| format!("{}", i)) {
                            Some(v) => v,
                            None => {
                                return Err(Diagnostic::new(
                                    Level::Error,
                                    "[des_derive] attributes do not support paths as keys."
                                        .to_string(),
                                ))
                            }
                        };

                        let val = match lit {
                            Lit::Str(str) => str.value(),
                            _ => {
                                return Err(Diagnostic::new(
                                    Level::Error,
                                    "The value of an [des_derive] attribute must be a string."
                                        .to_string(),
                                ))
                            }
                        };

                        match &path[..] {
                            "workspace" | "ndl_workspace" => workspace = Some(val),
                            "ident" | "ndl_ident" | "overwrite_ident" => {
                                overwrite_ident = Some(val)
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    Level::Error,
                                    format!("Unknown keyword '{}'.", path),
                                ))
                            }
                        };

                        i += 1;
                    }
                    _ => panic!("Gotem"),
                },
            }
        }

        Ok(Self {
            workspace,
            overwrite_ident,
        })
    }
}
