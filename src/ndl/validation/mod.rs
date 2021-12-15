use super::{
    error::Error,
    parser::{LinkDef, ModuleDef, NetworkDef, Parser},
    NdlResolver,
};

mod tests;

pub fn validate(resolver: &NdlResolver, unit: &Parser) -> Vec<Error> {
    let mut tyctx = TyContext::new();
    resolve_includes(resolver, unit, &mut tyctx);

    match tyctx.check_name_collision() {
        Ok(()) => {
            for module in &unit.modules {
                let self_ty = &module.name;
                // Check submodule namespaces and types
                let mut descriptors = Vec::new();

                for (ty, descriptor) in &module.submodule {
                    if descriptors.contains(&descriptor) {
                        todo!()
                    }
                    descriptors.push(descriptor);

                    let ty_valid = tyctx.modules.iter().any(|&m| m.name == *ty);
                    if !ty_valid || ty == self_ty {
                        todo!()
                    }
                }

                //

                // Check connection definition.

                for connection in &module.connections {
                    // check channel
                    if let Some(channel) = &connection.channel {
                        let ch_valid = tyctx.links.iter().any(|&l| l.name == *channel);
                        if !ch_valid {
                            todo!()
                        }
                    }

                    // check peers
                    for peer in [&connection.from, &connection.to] {
                        if let Some(subident) = &peer.subident {
                            // Referencing subvalue
                            let peer_ident_valid = descriptors.contains(&&peer.ident);
                            if !peer_ident_valid {
                                todo!()
                            }

                            let (ty, _n) = module
                                .submodule
                                .iter()
                                .find(|(_ty, str)| str == &peer.ident)
                                .unwrap();

                            let mod_def = tyctx.modules.iter().find(|m| m.name == *ty).unwrap();

                            let peer_subident_valid =
                                mod_def.gates.iter().any(|g| g.name == *subident);

                            if !peer_subident_valid {
                                println!("{} ", peer);
                                todo!()
                            }
                        } else {
                            // referencing direct value
                            let peer_valid = module.gates.iter().any(|g| g.name == peer.ident);
                            if !peer_valid {
                                todo!()
                            }
                        }
                    }
                }
            }
        }
        Err(_e) => {
            todo!()
        }
    }

    Vec::new()
}

fn resolve_includes<'a>(resolver: &'a NdlResolver, unit: &'a Parser, tyctx: &mut TyContext<'a>) {
    if tyctx.included.contains(&&unit.filepath) {
        return;
    }

    tyctx.included.push(&unit.filepath);

    // Add parsers own defs

    for link in &unit.links {
        tyctx.links.push(link)
    }

    for module in &unit.modules {
        tyctx.modules.push(module)
    }

    for network in &unit.networks {
        tyctx.networks.push(network)
    }

    // resolve meta imports.

    for include in &unit.includes {
        resolve_includes(resolver, resolver.units.get(&include.path).unwrap(), tyctx);
    }
}

#[derive(Debug)]
pub(crate) struct TyContext<'a> {
    pub included: Vec<&'a String>,

    pub links: Vec<&'a LinkDef>,
    pub modules: Vec<&'a ModuleDef>,
    pub networks: Vec<&'a NetworkDef>,
}

impl<'a> TyContext<'a> {
    fn new() -> Self {
        Self {
            included: Vec::new(),

            links: Vec::new(),
            modules: Vec::new(),
            networks: Vec::new(),
        }
    }

    fn check_name_collision(&self) -> Result<(), &'static str> {
        let dup_links = (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));
        let dup_modules =
            (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));
        let dup_networks =
            (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));

        if dup_links || dup_modules || dup_networks {
            Err("Found duplicated symbols")
        } else {
            Ok(())
        }
    }
}
