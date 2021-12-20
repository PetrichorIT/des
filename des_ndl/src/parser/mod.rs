use std::fmt::{Display};
use std::collections::VecDeque;

use des_core::ChannelMetrics;

use super::loc::Loc;
use super::error::Error;
use super::error::ErrorCode::*;
use super::error::LocalParsingErrorContext;
use super::lexer::{LiteralKind, Token, TokenKind};
use super::source::{SourceAsset, SourceAssetDescriptor};

mod tests;

///
/// Parses the given asset and its associated tokenstream
/// returning a parsing result that may or may not contain errors.
/// 
#[allow(unused)]
pub fn parse(asset: &SourceAsset, tokens: VecDeque<Token>) -> ParsingResult {

    let result = ParsingResult {
        asset: asset.descriptor.clone(),
        
        includes: Vec::new(),
        links: Vec::new(),
        modules: Vec::new(),
        networks: Vec::new(),

        errors: Vec::new(),
    };


    let mut parser = Parser { result, tokens, asset,  ectx: LocalParsingErrorContext::new(asset) };

    while !parser.is_done() {
        if let Some((token, raw_parts)) = parser.next_token() {
            match token.kind {
                TokenKind::Whitespace => continue,
                TokenKind::Ident => {
                    let ident = raw_parts;
                    match ident {
                        "include" => parser.parse_include(),
                        "module" => parser.parse_module(),
                        "link" => parser.parse_link(),
                        "network" => parser.parse_network(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    parser.finish()
}


///
/// The result of parsing an asset.
/// 
#[derive(Debug, Clone)]
pub struct ParsingResult {
    /// The descriptor of the asset that was parsed.
    pub asset: SourceAssetDescriptor,

    /// A collection of all unchecked includes.
    pub includes: Vec<IncludeDef>,
    /// A collection of all unchecked channel definitions.
    pub links: Vec<LinkDef>,
    /// A collection of all unchecked modules definitions.
    pub modules: Vec<ModuleDef>,
    /// A collection of all unchecked network definitions.
    pub networks: Vec<NetworkDef>,

    /// A list of all parsing errors that were encountered.
    pub errors: Vec<Error>,
}

struct Parser<'a> {
    result: ParsingResult,

    asset: &'a SourceAsset,
    tokens: VecDeque<Token>,
    ectx: LocalParsingErrorContext<'a>,
}

impl<'a> Parser<'a> {

    fn is_done(&self) -> bool {
        self.tokens.is_empty()
    }

    fn finish(mut self) -> ParsingResult {
        self.result.errors.append(&mut self.ectx.errors);
        self.result
    }

    fn eat_while(&mut self, mut predicate: impl FnMut(&Token) -> bool) {
        while self.tokens.front().is_some() && predicate(self.tokens.front().unwrap()) {
           self.tokens.pop_front();
        }
    }

    fn eat_whitespace(&mut self) {
        while self.tokens.front().is_some()
            && self.tokens.front().unwrap().kind == TokenKind::Whitespace
        {
             self.tokens.pop_front();
        }
    }
    
    fn next_token(&mut self) -> Option<(Token, &str)> {
        let token = self.tokens.pop_front()?;
        let raw_parts = token.loc.referenced_slice_in(&self.asset.data);
        Some((token, raw_parts))
    }
}

impl<'a> Parser<'a> {
    fn parse_include(&mut self) {
        self.ectx.reset_transient();
        self.eat_whitespace();

        let mut path_comps = Vec::new();
        let mut expects_comp = true;

        let start_line = self.tokens.front().map(|t| t.loc.line).unwrap_or(0);
        let start_pos = self.tokens.front().map(|t| t.loc.pos).unwrap_or_else(|| self.asset.data.len());

        let end_pos = (|| loop {
            if let Some((token, raw_parts)) = self.next_token() {
                match token.kind {
                    TokenKind::Ident if expects_comp => {
                        path_comps.push(String::from(raw_parts));
                        expects_comp = false;
                    }
                    TokenKind::Slash if !expects_comp => expects_comp = true,
                    _ => return token.loc.pos,
                }
            } else {
                return self.asset.data.len();
            }
        })();


        self.result.includes.push(IncludeDef {
            loc: Loc::new(start_pos, end_pos - start_pos, start_line),
            path: path_comps.join("/"),
        });

        self.eat_whitespace();
    }

    fn parse_module(&mut self) {
        self.ectx.reset_transient();
        self.eat_whitespace();

        let (id_token, id) = self.next_token().unwrap();
        let id = String::from(id);
        if id_token.kind != TokenKind::Ident {
            self.ectx.record(ParModuleMissingIdentifer, String::from("Invalid token. Expected module identfier."), id_token.loc);
            return;
        }


        self.eat_whitespace();
        let (token, _raw) = self.next_token().unwrap();
        if token.kind != TokenKind::OpenBrace {
            self.ectx.record(
                ParModuleMissingDefBlockOpen, 
                String::from("Invalid token. Expected module definition block (OpenBrace)"), 
                token.loc,
            );
            return;
        }

        // Contents reading

        let mut module_def = ModuleDef {
            loc: Loc::new(0, 0, 0),
            asset: self.asset.descriptor.clone(),

            name: id,
            gates: Vec::new(),
            submodule: Vec::new(),
            connections: Vec::new(),
        };

        loop {
            self.eat_whitespace();

            let (subsec_token, subsection_id) = self.next_token().unwrap();
            let subsection_id = String::from(subsection_id);
            if subsec_token.kind != TokenKind::Ident {

                if subsec_token.kind == TokenKind::CloseBrace {
                    self.ectx.reset_transient();
                    self.result.modules.push(module_def);
                    return;
                }

                self.ectx.record(
                    ParModuleMissingSectionIdentifier, 
                    String::from("Invalid token. Expected identifier for subsection (gates / submodules / connections)."), 
                    subsec_token.loc,
                );
                return;
            }

            if !(vec!["gates", "submodules", "connections"]).contains(&&subsection_id[..]) {
                self.ectx.record(
                    ParModuleInvalidSectionIdentifer,
                    format!("Invalid subsection identifier '{}'. Possibilities are gates / submodules / connections.", subsection_id),
                    subsec_token.loc,
                );
                return;
            }

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::Colon {
                self.ectx.record(
                    ParModuleInvalidSeperator,
                    String::from("Unexpected token. Expected colon ':'."),
                    token.loc,
                )
            };

            self.ectx.reset_transient();

            let done = match &subsection_id[..] {
                "gates" => self.parse_module_gates(&mut module_def),
                "submodules" => self.parse_module_submodules(&mut module_def),
                "connections" => self.parse_module_connections(&mut module_def),
                _ => unreachable!()
            };

            if done {
                break;
            }
        }

        let len = self.tokens.front().map(|t| t.loc.pos).unwrap_or_else(|| self.asset.data.len()) - id_token.loc.pos;

        module_def.loc = Loc::new(id_token.loc.pos, len, id_token.loc.line);

        self.result.modules.push(module_def);
    }

    fn parse_module_gates(&mut self, module_def: &mut ModuleDef) -> bool {

        'mloop: loop {
            self.eat_whitespace();

            let (name_token, name) = self.next_token().unwrap();
            let name = String::from(name);
            if name_token.kind != TokenKind::Ident {

                if name_token.kind == TokenKind::CloseBrace {
                    self.ectx.reset_transient();
                    return true;
                }

                self.ectx.record(
                    ParModuleInvalidKeyToken,
                    String::from("Invalid token. Expected gate identifier."),
                    name_token.loc,
                );
                
                continue 'mloop;
            }

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::OpenBracket {
                // Single size gate
                if token.kind == TokenKind::Whitespace {
                    module_def.gates.push(GateDef { loc: Loc::fromto(name_token.loc, token.loc), name, size: 1 })
                } else if token.kind == TokenKind::Colon {
                    // New identifer
                    self.tokens.push_front(token);
                    self.tokens.push_front(name_token);
                    self.ectx.reset_transient();
                    return false;
                } else {
                    self.ectx.record(
                        ParModuleGateInvalidIdentifierToken,
                        String::from("Unexpected token. Expected whitespace."),
                        token.loc,
                    );
                    
                    continue 'mloop
                }
                
            } else {
                // cluster gate

                let (token, literal) = self.next_token().unwrap();
                #[allow(clippy::collapsible_match)]
                match token.kind {
                    TokenKind::Literal { kind, ..} => {
                        if let LiteralKind::Int { base, .. } = kind {
                            match usize::from_str_radix(literal, base.radix()) {
                                Ok(value) => {
                                    self.eat_whitespace();
                                    let (token, _raw) = self.next_token().unwrap();
                                    if token.kind != TokenKind::CloseBracket {
                                        self.ectx.record(
                                            ParModuleGateMissingClosingBracket,
                                            String::from("Unexpected token. Expected closing bracket."),
                                            token.loc,
                                        );
                                        
                                        continue 'mloop;
                                    }

                                    module_def.gates.push(GateDef { loc: Loc::fromto(name_token.loc, token.loc), name, size: value }); 
                                },
                                Err(e) => {
                                    self.ectx.record(
                                        LiteralIntParseError, 
                                        format!("Failed to parse integer: {}", e), 
                                        token.loc,
                                    );
                                    
                                    self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                                    continue 'mloop;
                                }
                            }

                        } else {
                            self.ectx.record(
                                ParModuleGateInvalidGateSize,
                                String::from("Unexpected token. Expected gate size (Int)."),
                                token.loc,
                            );

                            self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                            continue 'mloop;
                        }
                    }
                    _ => {
                        self.ectx.record(
                            ParModuleGateInvalidGateSize,
                            String::from("Unexpected token. Expected gate size (Int)."),
                            token.loc,
                        );

                        self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                        continue 'mloop;
                    }
                }

            }
        }

    }

    fn parse_module_submodules(&mut self, module_def: &mut ModuleDef) -> bool {

        loop {
            self.eat_whitespace();
            let (ty_token, ty) = self.next_token().unwrap();
            let ty = String::from(ty);
            match ty_token.kind {
                TokenKind::CloseBrace => {
                    self.ectx.reset_transient();
                    return true;
                },
                TokenKind::Ident => {

                    let (token, _raw) = self.next_token().unwrap();
                    if token.kind != TokenKind::Whitespace {
                        if token.kind == TokenKind::Colon {
                            // new subsection
                            self.tokens.push_front(token);
                            self.tokens.push_front(ty_token);
                            self.ectx.reset_transient();
                        } else {
                            self.ectx.record(
                                ParModuleSubInvalidSeperator,
                                String::from("Unexpected token. Expected whitespace."),
                                token.loc,
                            );
                        }
                        return false;
                    }

                    let (token, defname) = self.next_token().unwrap();
                    let defname = String::from(defname);
                    if token.kind != TokenKind::Ident {
                        self.ectx.record(
                            ParModuleSubInvalidIdentiferToken, 
                            String::from("Unexpected token. Expected submodule identifier."),
                            token.loc,
                        );
                        return false;
                    }

                module_def.submodule.push(SubmoduleDef { loc: Loc::fromto(ty_token.loc, token.loc), ty, descriptor: defname });
                },
                _ => {
                    self.ectx.record(
                        ParModuleSubInvalidIdentiferToken,
                        String::from("Unexpected token. Expected submodule type."),
                        ty_token.loc,
                    );
                    return false;
                }
            }

        }

    }

    fn parse_module_connections(&mut self, module_def: &mut ModuleDef) -> bool {
        loop {
            let front_ident = match self.parse_connetion_identifer_token() {
                ConIdentiferResult::Result(ident) => ident,
                ConIdentiferResult::Error => return false,
                ConIdentiferResult::NewSubsection => return false,
                ConIdentiferResult::Done => return true,
            };

            self.eat_whitespace();

            let (t1, _raw) = self.next_token().unwrap();
            let (t2, _raw) = self.next_token().unwrap();
            let (t3, _raw) = self.next_token().unwrap();


            use TokenKind::*;
            let to_right = match (t1.kind, t2.kind, t3.kind) {
                (Minus, Minus, Gt) => true,
                (Lt, Minus, Minus) => false,
                _ => {
                    self.ectx.record(
                        ParModuleConInvaldiChannelSyntax,
                        String::from("Unexpected token. Expected arrow syntax."),
                        Loc::fromto(t1.loc, t3.loc),
                    );
                    return false;
                }
            };


            let mid_ident = match self.parse_connetion_identifer_token() {
                ConIdentiferResult::Result(ident) => ident,
                ConIdentiferResult::Error => return false,
                ConIdentiferResult::NewSubsection => return false,
                ConIdentiferResult::Done => return true,
            };

            if mid_ident.subident.is_some() {
                // Direct connection to stack frame
                if to_right {
                    module_def.connections.push(ConDef {
                        loc: Loc::fromto(front_ident.loc, t3.loc),

                        from: front_ident,
                        to: mid_ident,
                        channel: None,
                    })
                } else {
                    module_def.connections.push(ConDef {
                        loc: Loc::fromto(front_ident.loc, t3.loc),

                        from: mid_ident,
                        to: front_ident,
                        channel: None,
                    })
                }
            } else {

                self.eat_whitespace();

                // check for second arrow
                let (t1, _raw) = self.next_token().unwrap();

                if t1.kind == TokenKind::Ident {
                    self.tokens.push_front(t1);
                    continue;
                }
                
                let (t2, _raw) = self.next_token().unwrap();
                let (t3, _raw) = self.next_token().unwrap();

                let to_right2 = match (t1.kind, t2.kind, t3.kind) {
                    (Minus, Minus, Gt) => true,
                    (Lt, Minus, Minus) => false,
                    _ => {
                        self.ectx.record(
                            ParModuleConInvaldiChannelSyntax,
                            String::from("Unexpected token. Expected arrow syntax"),
                            Loc::fromto(t1.loc, t3.loc),
                        );
                        return false;
                    }
                };

                if (to_right && to_right2) || (!to_right && !to_right2) {

                    let last_ident = match self.parse_connetion_identifer_token() {
                        ConIdentiferResult::Result(ident) => ident,
                        ConIdentiferResult::Error => return false,
                        ConIdentiferResult::NewSubsection => return false,
                        ConIdentiferResult::Done => return true,
                    };

                    if to_right {
                        module_def.connections.push(ConDef {
                            loc: Loc::fromto(front_ident.loc, t3.loc),

                            from: front_ident,
                            to: last_ident,
                            channel: Some(mid_ident.ident),
                        })
                    } else {
                        module_def.connections.push(ConDef {
                            loc: Loc::fromto(front_ident.loc, t3.loc),

                            from: last_ident,
                            to: front_ident,
                            channel: Some(mid_ident.ident),
                        })
                    }

                } else {
                    self.ectx.record(
                        ParModuleConInvaldiChannelSyntax,
                        String::from("Invalid arrow syntax. Both arrows must match."),
                        Loc::fromto(t1.loc, t3.loc),
                    );
                    return false;
                }
            }
        }
    }

    fn parse_connetion_identifer_token(&mut self) -> ConIdentiferResult {
        use ConIdentiferResult::*;

        self.eat_whitespace();

        let (first_token, id) = self.next_token().unwrap();
        let id = String::from(id);

        if first_token.kind != TokenKind::Ident {
            
            if first_token.kind == TokenKind::CloseBrace {
                self.ectx.reset_transient();
                return Done
            }

            self.ectx.record(
                ParModuleConInvalidIdentiferToken,
                String::from("Unexpected token. Expected identifer."),
                first_token.loc,
            );
            return Error;
        }

        let (token, _raw) = self.next_token().unwrap();
        match token.kind {
            TokenKind::Slash => {
                let (token, id_second) = self.next_token().unwrap();
                let id_second = String::from(id_second);
                if token.kind != TokenKind::Ident {
                    self.ectx.record(
                        ParModuleConInvalidIdentiferToken,
                        String::from("Unexpected token. Expected second part identifer"),
                        token.loc,
                    );
                    return Error;
                }

                self.ectx.reset_transient();
                Result(ConNodeIdent { loc: Loc::fromto(first_token.loc, token.loc), ident: id, subident: Some(id_second) } )
            },
            TokenKind::Whitespace => {
                self.ectx.reset_transient();
                Result(ConNodeIdent { loc: Loc::fromto(first_token.loc, token.loc), ident: id, subident: None })
            },
            TokenKind::Colon => {
                self.ectx.reset_transient();

                self.tokens.push_front(token);
                self.tokens.push_front(first_token);

                NewSubsection
            },
            _ => {
                self.ectx.record(
                    ParModuleConInvalidIdentiferToken,
                    String::from("Unexpected token. Expected whitespace or slash."),
                    token.loc,
                );
                
                Error
            },
        }
    }

    fn parse_link(&mut self) {
        self.ectx.reset_transient();

        self.eat_whitespace();
        let (id_token, identifier) = self.next_token().unwrap();
        if id_token.kind != TokenKind::Ident {
            self.ectx.record(
                ParLinkMissingIdentifier,
                String::from("Unexpected token. Expected identifer for link definition"),
                id_token.loc,
            );
            return;
        }

        let identifier = String::from(identifier);
        
        self.eat_whitespace();
        let (paran_open, _raw) = self.next_token().unwrap();
        if paran_open.kind != TokenKind::OpenBrace {
            self.ectx.record(
                ParLinkMissingDefBlockOpen,
                String::from("Unexpected token. Expected block for link definition"),
                paran_open.loc,
            );
            return;
        }

        let mut bitrate: Option<usize> = None;
        let mut jitter: Option<f64> = None;
        let mut latency: Option<f64> = None;

        while bitrate.is_none() || jitter.is_none() || latency.is_none() {
            self.eat_whitespace();

            let (key_token, raw) = self.next_token().unwrap();
            if key_token.kind != TokenKind::Ident {
                self.ectx.record(
                    ParLinkInvalidKeyToken,
                    String::from("Unexpected token. Expected identifer for definition key."),
                    key_token.loc,
                );
                return;
            }
            let identifier = String::from(raw);

            self.eat_whitespace();

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::Colon {
                self.ectx.record(
                    ParLinkInvalidKvSeperator,
                    String::from(
                        "Unexpected token. Expected colon ':' between definition key and value",
                    ),
                    token.loc,
                );
                return;
            }

            self.eat_whitespace();
            let (token, raw) = self.next_token().unwrap();

            match token.kind {
                TokenKind::Literal { kind, .. } => match &identifier[..] {
                    "bitrate" => {
                        if let LiteralKind::Int { base, .. } = kind {
                            match usize::from_str_radix(raw, base.radix()) {
                                Ok(value) => bitrate = Some(value),
                                Err(e) => {
                                    self.ectx.record(
                                        LiteralIntParseError,
                                        format!("Int parsing error: {}", e), 
                                        token.loc,
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected integer."), 
                                token.loc,
                            );
                            return;
                        }
                    }

                    "latency" => {
                        use std::str::FromStr;

                        if let LiteralKind::Float { .. } = kind {
                            match f64::from_str(raw) {
                                Ok(value) => latency = Some(value),
                                Err(e) => {
                                    self.ectx.record(
                                        LiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.loc
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.loc,
                            );
                            return;
                        }
                    }
                    "jitter" => {
                        use std::str::FromStr;

                        if let LiteralKind::Float { .. } = kind {
                            match f64::from_str(raw) {
                                Ok(value) => jitter = Some(value),
                                Err(e) => {
                                    self.ectx.record(
                                        LiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.loc,
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.loc
                            );
                            return;
                        }
                    }
                    _ => {
                        self.ectx.record(
                            ParLinkInvalidKey, 
                            format!("Invlaid key '{}' in kv-pair. Valid keys are latency, bitrate or jitter.", identifier), 
                            key_token.loc,
                        );
                        return;
                    }
                },
                _ => {
                    self.ectx.record(
                        ParLinkInvalidValueToken,
                        String::from("Unexpected token. Expected literal"),
                        token.loc,
                    );
                    return;
                }
            }
        }

        self.eat_whitespace();

        let (token, _raw) = self.next_token().unwrap();
        if token.kind != TokenKind::CloseBrace {
            self.ectx.record(ParLinkMissingDefBlockClose, String::from("Unexpected token. Expected closing brace."), token.loc);
            return;
        }

        self.result.links.push(LinkDef {
            loc: Loc::fromto(id_token.loc, token.loc),
            asset: self.asset.descriptor.clone(),

            name: identifier,
            metrics: ChannelMetrics::new(
                bitrate.unwrap(),
                latency.unwrap().into(),
                jitter.unwrap().into(),
            ),
        });

        self.ectx.reset_transient()
    }

    fn parse_network(&mut self) {
        unimplemented!()
    }
}

impl Display for ParsingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ParsingResult {{")?;
        
        writeln!(f, "    includes:")?;
        for include in &self.includes {
            writeln!(f, "    - {}", include)?;
        }

        writeln!(f)?;
        writeln!(f, "    links:")?;
        for link in &self.links {
            writeln!(f, "    - {}", link)?;
        }

        writeln!(f)?;
        writeln!(f, "    modules:")?;
        for module in &self.modules {
            writeln!(f, "    - {} {{", module.name)?;

            writeln!(f, "      submodules:")?;
            for submodule in &module.submodule {
                writeln!(f, "        {} {}", submodule.ty, submodule.descriptor)?;
            }

            writeln!(f)?;
            writeln!(f, "      gates:")?;
            for gate in &module.gates {
                writeln!(f, "        {}", gate)?;
            }

            writeln!(f)?;
            writeln!(f, "      connections:")?;
            for con in &module.connections {
                writeln!(f, "        {}", con)?;
            }

            writeln!(f, "    }}")?;
        }

        write!(f, "}}")
    }
}

enum ConIdentiferResult {
    Error,
    Result(ConNodeIdent),
    NewSubsection,
    Done
}

///
/// A definition of a include statement.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDef {
    /// The token location of the include.
    pub loc: Loc,
    /// The imported modules alias.
    pub path: String,
}

impl Display for IncludeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

///
/// A definition of a channel.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkDef {
    /// The tokens location in the source asset.
    pub loc: Loc,
    /// The asset the channel was defined (used for import suggestions).
    pub asset: SourceAssetDescriptor,

    /// The identifier of the channel.
    pub name: String,
    /// The defining metric for the channel.
    pub metrics: ChannelMetrics,
}

impl Display for LinkDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f, 
            "{}(bitrate: {}, latency: {}, jitter: {})", 
            self.name, 
            self.metrics.bitrate, 
            self.metrics.latency, 
            self.metrics.jitter
        )
    }
}

///
/// A definition of a module.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDef {
    /// The tokens location in the source asset.
    pub loc: Loc,
    /// The asset the module was defined in (used for import suggestions).
    pub asset: SourceAssetDescriptor,

    /// The identifier of the module.
    pub name: String,
    /// The local submodules defined for this module.
    pub submodule: Vec<SubmoduleDef>,
    /// The gates exposed on this module.
    pub gates: Vec<GateDef>,
    /// The connections defined by this module.
    pub connections: Vec<ConDef>,
}

///
/// A definition of a local submodule, in a modules definition.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleDef {
    /// The location of the source tokens.
    pub loc: Loc,

    /// The type of the submodule.
    pub ty: String,
    /// A module internal descriptor for the created submodule.
    pub descriptor: String,
}

///
/// A definition of a Gate, in a modules definition.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the local gate cluster.
    pub name: String,
    /// The size of the local gate cluster.
    pub size: usize
}

impl Display for GateDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.name, self.size)
    }
}

///
/// A description of a connection, in a modules definition.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The origin gate cluster the connection starts from.
    pub from: ConNodeIdent,
    /// The channel that is used to creat delays on this connection.
    pub channel: Option<String>,
    /// The target gate cluster the connection points to.
    pub to: ConNodeIdent
}

impl Display for ConDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(channel) = &self.channel {
            write!(f, "{} --> {} --> {}", self.from, channel, self.to)
        } else {
            write!(f, "{} --> {}", self.from, self.to)
        }
        
    }
}

///
/// A gate cluster definition, that may reference a submodules gate cluster,
/// inside a modules connection definition.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConNodeIdent {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The primary identifier either being the a local gate or a submodule name.
    pub ident: String,
    /// The secondary identifier either being the submodules gate or None.
    pub subident: Option<String>
}

impl Display for ConNodeIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(subident) = &self.subident {
            write!(f, "{}/{}", self.ident, subident)
        } else {
            write!(f, "{}", self.ident)
        }
    }
}

///
/// A definition of a Network.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDef {}
