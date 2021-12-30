use syn::Attribute;
use syn::Lit;
use syn::Meta;
use syn::MetaNameValue;

#[derive(Debug, Clone)]
pub struct Attributes {
    pub workspace: Option<String>,
    pub ident: Option<String>,
}

impl Attributes {
    pub fn from_attr(attrs: Vec<Attribute>) -> Self {
        let mut obj = Attributes {
            workspace: None,
            ident: None,
        };

        for attr in attrs {
            match attr.parse_meta().unwrap() {
                Meta::NameValue(MetaNameValue {
                    ref path, ref lit, ..
                }) => match &path.segments.last().unwrap().ident.to_string()[..] {
                    "ndl_workspace" => {
                        obj.workspace = match lit {
                            Lit::Str(str) => Some(str.value()),
                            _ => None,
                        }
                    }
                    "ndl_ident" => {
                        obj.ident = match lit {
                            Lit::Str(str) => Some(str.value()),
                            _ => None,
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        obj
    }
}
