use crate::*;

use crate::error::*;
use crate::lexer::LiteralKind;

use std::fmt::Display;

mod defs;
pub use defs::*;

const MODULE_SUBSECTION_IDENT: [&str; 4] = ["gates", "submodules", "connections", "parameters"];
const NETWORK_SUBSECTION_IDENT: [&str; 3] = ["nodes", "connections", "parameters"];


///
/// Parses the given asset and its associated tokenstream
/// returning a parsing result that may or may not contain errors.
/// 
#[allow(unused)]
pub fn parse(asset: Asset<'_>, tokens: TokenStream) -> ParsingResult {

    let last_loc = asset.end_loc();

    let result = ParsingResult {
        asset: asset.descriptor(),
        loc: asset.start_loc(),

        includes: Vec::new(),
        links: Vec::new(),
        modules_and_prototypes: Vec::new(),
        aliases: Vec::new(),
        networks: Vec::new(),

        errors: Vec::new(),
    };

    let mut ectx = ParsingErrorContext::new(&asset);

    let mut parser = Parser { result, tokens, asset };

    let p_state: NdlResult<()> = (||{
        while !parser.is_done() {
            if let Ok((token, raw_parts)) = parser.next_token() {
                match token.kind {
                    TokenKind::Whitespace => continue,
                    TokenKind::Ident => {
                        let ident = raw_parts;
                        match ident {
                            "include" => parser.parse_include(&mut ectx)?,
                            "module" => parser.parse_module(false, &mut ectx)?,
                            "prototype" => parser.parse_module(true, &mut ectx)?,
                            "alias" => parser.parse_alias(&mut ectx)?,
                            "link" => parser.parse_link(&mut ectx)?,
                            "network" => parser.parse_network(&mut ectx)?,
                            _ => { 
                                ectx.record(
                                    ParUnexpectedKeyword, 
                                    format!("Unexpected keyword '{}'. Expected include / module / link or network", ident), 
                                    token.loc
                                );
                                ectx.reset_transient()
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    if let Err(e) = p_state {
        ectx.record(
            ParUnexpectedEOF, 
            e.into(), 
            last_loc
        );
    }

    parser.finish(ectx)
}

///
/// The result of parsing an asset.
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingResult {
    /// The descriptor of the asset that was parsed.
    pub asset: AssetDescriptor,

    /// The location of the referenced asset.
    pub loc: Loc,

    /// A collection of all unchecked includes.
    pub includes: Vec<IncludeDef>,
    /// A collection of all unchecked channel definitions.
    pub links: Vec<LinkDef>,
    /// A collection of all unchecked modules definitions.
    pub modules_and_prototypes: Vec<ModuleDef>,
    /// A collection of all aliases refering to prototypes.
    pub aliases: Vec<AliasDef>,
    /// A collection of all unchecked network definitions.
    pub networks: Vec<NetworkDef>,

    /// A list of all parsing errors that were encountered.
    pub errors: Vec<Error>,
}

struct Parser<'a> {
    result: ParsingResult,

    asset: Asset<'a>,
    tokens: TokenStream,
    //ectx: ParsingErrorContext<'a>,
}

impl<'a> Parser<'a> {

    fn is_done(&self) -> bool {
        self.tokens.is_empty()
    }

    fn finish(mut self, mut ectx: ParsingErrorContext<'_>) -> ParsingResult {
        self.result.errors.append(&mut ectx.errors);
        self.result
    }

    fn eat_optionally(&self, predicate: impl FnOnce(&Token) -> bool) {
        if self.tokens.peek().is_ok() && predicate(self.tokens.peek().unwrap()) {
            let _ = self.tokens.bump();
        }
    }
    
    fn eat_while(&self, mut predicate: impl FnMut(&Token) -> bool) {
        while self.tokens.peek().is_ok() && predicate(self.tokens.peek().unwrap()) {
           let _ = self.tokens.bump();
        }
    }

    fn eat_whitespace(&self) {
        self.eat_while(|t| t.kind == TokenKind::Whitespace)
    }
    
    fn next_token(&self) -> NdlResult<(&Token, &str)> {
        let token = self.tokens.bump()?;
        let raw_parts = self.asset.referenced_slice_for(token.loc);

        Ok((token, raw_parts))
    }
}

impl<'a> Parser<'a> {
    fn parse_include(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<()> {
        ectx.reset_transient();
        self.eat_whitespace();

        let mut path_comps = Vec::new();
        let mut expects_comp = true;

        let start_line = self.tokens.peek().map(|t| t.loc.line)?;
        let start_pos = self.tokens.peek().map(|t| t.loc.pos)?;

        let end_pos = (|| loop {
            if let Ok((token, raw_parts)) = self.next_token() {
                match token.kind {
                    TokenKind::Ident if expects_comp => {
                        path_comps.push(String::from(raw_parts));
                        expects_comp = false;
                    }
                    TokenKind::Slash if !expects_comp => expects_comp = true,
                    _ => return token.loc.pos,
                }
            } else {
                return self.asset.end_pos();
            }
        })();


        self.result.includes.push(IncludeDef {
            loc: Loc::new(start_pos, end_pos - start_pos, start_line),
            path: path_comps.join("/"),
        });

        self.eat_whitespace();

        Ok(())
    }

    fn parse_module(&mut self, is_prototype: bool,  ectx: &mut ParsingErrorContext<'_>) -> NdlResult<()> {
        ectx.reset_transient();
        self.eat_whitespace();

        let (id_token, id) = self.next_token()?;
        let id_token_loc = id_token.loc;
        let id = String::from(id);
        if id_token.kind != TokenKind::Ident {
            ectx.record(
                ParModuleMissingIdentifer, 
                String::from("Invalid token. Expected module identfier."), 
                id_token.loc
            )?;
            return Ok(());
        }


        self.eat_whitespace();
        let (token, _raw) = self.next_token()?;
        if token.kind != TokenKind::OpenBrace {
            ectx.record(
                ParModuleMissingDefBlockOpen, 
                String::from("Invalid token. Expected module definition block (OpenBrace)"), 
                token.loc,
            )?;
            return Ok(());
        }

        // Contents reading

        let mut module_def = ModuleDef {
            loc: Loc::new(0, 1, 1),
       
            name: id,
            gates: Vec::new(),
            submodules: Vec::new(),
            connections: Vec::new(),
            parameters: Vec::new(),
            
            is_prototype,
        };

        loop {
            self.eat_whitespace();

            let (subsec_token, subsection_id) = self.next_token()?;
            let subsection_id = String::from(subsection_id);
            if subsec_token.kind != TokenKind::Ident {

                if subsec_token.kind == TokenKind::CloseBrace {
                    ectx.reset_transient();

                    module_def.loc = Loc::fromto(id_token_loc, subsec_token.loc);
                    self.result.modules_and_prototypes.push(module_def);
                    return Ok(());
                }

                ectx.record(
                    ParModuleMissingSectionIdentifier, 
                    format!("Invalid token. Expected identifier for subsection are {}.", MODULE_SUBSECTION_IDENT.join(" / ")), 
                    subsec_token.loc,
                )?;
                return Ok(());
            }

            if !MODULE_SUBSECTION_IDENT.contains(&&subsection_id[..]) {
                ectx.record(
                    ParModuleInvalidSectionIdentifer,
                    format!("Invalid subsection identifier '{}'. Possibilities are {}.", subsection_id, MODULE_SUBSECTION_IDENT.join(" / ")),
                    subsec_token.loc,
                )?;
                return Ok(());
            }

            let (token, _raw) = self.next_token()?;
            if token.kind != TokenKind::Colon {
                ectx.record(
                    ParModuleInvalidSeperator,
                    String::from("Unexpected token. Expected colon ':'."),
                    token.loc,
                )?;
            };

            ectx.reset_transient();

            let done = match &subsection_id[..] {
                "gates" => self.parse_module_gates(&mut module_def.gates, ectx)?,
                "submodules" => self.parse_childmodule_def(&mut module_def.submodules, ectx, &MODULE_SUBSECTION_IDENT)?,
                "connections" => self.parse_node_connections(&mut module_def.connections, ectx, &MODULE_SUBSECTION_IDENT)?,
                "parameters" => self.parse_par(&mut module_def.parameters, ectx, &MODULE_SUBSECTION_IDENT)?,
                _ => unreachable!()
            };

            if done {
                break;
            }
        }

        let len = self.tokens.peek()
            .map(|t| t.loc.pos)
            .unwrap_or_else(|_| self.asset.end_pos()) - id_token_loc.pos;
        module_def.loc = Loc::new(id_token_loc.pos, len, id_token_loc.line);

        self.result.modules_and_prototypes.push(module_def);

        Ok(())
    }

    fn parse_par(&mut self, parameters: &mut Vec<ParamDef>, ectx: &mut ParsingErrorContext<'_>, escape_keywords: &[&str]) -> NdlResult<bool> {

        loop {
            self.eat_whitespace();
            let (first_token, ident) = self.next_token()?;
            let ident = String::from(ident);
            match first_token.kind {
                TokenKind::CloseBrace => {
                    ectx.reset_transient();
                    return Ok(true);
                },
                TokenKind::Ident => {

                    self.eat_whitespace();

                    let (token, _raw) = self.next_token()?;
                    if token.kind != TokenKind::Colon {
                        ectx.record(
                            ParModuleSubInvalidSeperator,
                            String::from("Unexpected token. Expected colon ':'."),
                            token.loc,
                        )?;
                        return Ok(false);
                    }

                    if escape_keywords.contains(&&ident[..]) {
                        // new subsection ident
                        self.tokens.bump_back(2);
                        ectx.reset_transient();
                        return Ok(false);
                    } else {
                        // new submodule def.
                        self.eat_whitespace();

                        let (second_token, ty) = self.next_token()?;
                        let ty = String::from(ty);
                        if second_token.kind != TokenKind::Ident {
                            ectx.record(
                                ParModuleSubInvalidIdentiferToken,
                                String::from("Unexpected token. Expected type identifer."),
                                second_token.loc
                            )?;
                            return Ok(false);
                        }

                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);
                        parameters.push(ParamDef { loc: Loc::fromto(first_token.loc, second_token.loc), ty, ident });
                    }
                },
                _ => {
                    ectx.record(
                        ParModuleSubInvalidIdentiferToken,
                        String::from("Unexpected token. Expected submodule type."),
                        first_token.loc,
                    )?;
                    return Ok(false);
                }
            }

        }
    }

    fn parse_module_gates(&mut self, gates: &mut Vec<GateDef>, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<bool> {
        'mloop: loop {
            self.eat_whitespace();

            let (name_token, name) = self.next_token()?;
            let name_token = name_token.clone();
            let name = String::from(name);
            if name_token.kind != TokenKind::Ident {

                if name_token.kind == TokenKind::CloseBrace {
                    ectx.reset_transient();
                    return Ok(true);
                }

                ectx.record(
                    ParModuleInvalidKeyToken,
                    String::from("Invalid token. Expected gate identifier."),
                    name_token.loc,
                )?;
                
                continue 'mloop;
            }

            let (token, _raw) = self.next_token()?;
            let token = token.clone();
            if token.kind != TokenKind::OpenBracket {
                // Single size gate
                if token.kind == TokenKind::Whitespace {
                    // Consume whitespace and comma optionally
                    self.eat_whitespace();
                    let annotation = self.parse_optional_gate_annotation(ectx)?;
                    self.eat_optionally(|t| t.kind == TokenKind::Comma);

                    // Push gate
                    gates.push(GateDef { loc: Loc::fromto(name_token.loc, token.loc), name, size: 1, annotation })
                } else if token.kind == TokenKind::Comma {
                    // Push gate
                    // instant comma == no annotation
                    let annotation = GateAnnotation::Unknown;
                    gates.push(GateDef { loc: Loc::fromto(name_token.loc, token.loc), name, size: 1, annotation })
                } else if token.kind == TokenKind::Colon {
                    // New identifer
                    self.tokens.bump_back(2);
                    ectx.reset_transient();
                    return Ok(false);
                } else {
                    ectx.record(
                        ParModuleGateInvalidIdentifierToken,
                        String::from("Unexpected token. Expected whitespace."),
                        token.loc,
                    )?;
                    
                    continue 'mloop
                }
                
            } else {
                // cluster gate

                let (token, literal) = self.next_token()?;
                #[allow(clippy::collapsible_match)]
                match token.kind {
                    TokenKind::Literal { kind, ..} => {
                        if let LiteralKind::Int { base, .. } = kind {
                            match usize::from_str_radix(literal, base.radix()) {
                                Ok(value) => {
                                    self.eat_whitespace();
                                    let (token, _raw) = self.next_token()?;
                                    let token = token.clone();
                                    if token.kind != TokenKind::CloseBracket {
                                        // The found token wasn't expected
                                        // could be relevant for next pass.
                                        self.tokens.bump_back(1);
                                        ectx.record_missing_token(
                                            ParModuleGateMissingClosingBracket,
                                            String::from("Unexpected token. Expected closing bracket."),
                                            self.tokens.prev_non_whitespace(0).unwrap(),
                                            "]"
                                        )?;

                                        
                                        continue 'mloop;
                                    }

                                    self.eat_whitespace();
                                    let annotation = self.parse_optional_gate_annotation(ectx)?;
                                    self.eat_optionally(|t| t.kind == TokenKind::Comma);

                                    gates.push(GateDef { loc: Loc::fromto(name_token.loc, token.loc), name, size: value, annotation }); 
                                },
                                Err(e) => {
                                    ectx.record(
                                        ParLiteralIntParseError, 
                                        format!("Failed to parse integer: {}", e), 
                                        token.loc,
                                    )?;
                                    
                                    self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                                    continue 'mloop;
                                }
                            }

                        } else {
                            ectx.record(
                                ParModuleGateInvalidGateSize,
                                String::from("Unexpected token. Expected gate size (Int)."),
                                token.loc,
                            )?;

                            self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                            continue 'mloop;
                        }
                    }
                    _ => {
                        ectx.record(
                            ParModuleGateInvalidGateSize,
                            String::from("Unexpected token. Expected gate size (Int)."),
                            token.loc,
                        )?;

                        self.eat_while(|t| matches!(t.kind, TokenKind::Whitespace | TokenKind::CloseBracket));
                        continue 'mloop;
                    }
                }

            }
        }

    }

    pub fn parse_optional_gate_annotation(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<GateAnnotation> {
        // only allowed if next token.
        let (token, _) = self.next_token()?;
        if token.kind != TokenKind::At {
            self.tokens.bump_back(1);
            return Ok(GateAnnotation::Unknown)
        }

        // annotation
        let (token, raw) = self.next_token()?;
        if token.kind != TokenKind::Ident {
            ectx.record(
                ParModuleGateInvalidServiceAnnotation,
                format!("Invalid token '{}', expected ident.", raw), 
                token.loc
            )?;
            return Ok(GateAnnotation::Unknown)
        }

        match raw {
            "input" => Ok(GateAnnotation::Input),
            "output" => Ok(GateAnnotation::Output),
            _ => {
                ectx.record_with_solution(
                    ParModuleGateInvalidServiceAnnotation, 
                    String::from("Invalid service annotation, expected '@input' or '@output'."),
                    token.loc, 
                    ErrorSolution::new(String::from("Remove or replace with 'input' or 'output'"), token.loc)
                )?;
                Ok(GateAnnotation::Unknown)
            }
        }
    }

    fn parse_childmodule_def(&mut self, child_modules: &mut Vec<ChildeModuleDef>, ectx: &mut ParsingErrorContext<'_>, escape_keywords: &[&str]) -> NdlResult<bool> {

        loop {
            self.eat_whitespace();
            let (first_token, ident) = self.next_token()?;
            let first_token_loc = first_token.loc;
            let ident = String::from(ident);
            match first_token.kind {
                TokenKind::CloseBrace => {
                    ectx.reset_transient();
                    return Ok(true);
                },
                TokenKind::Ident => {

                    self.eat_whitespace();

                    let mut desc = LocalDescriptorDef::new_non_cluster(ident, first_token_loc);

                    let (token, _raw) = self.next_token()?;
                    if token.kind != TokenKind::Colon {
                        // cluster def.
                        if token.kind == TokenKind::OpenBracket {

                            assert!(desc.cluster_bounds.is_none(),"Doesn not support nested implicite macros");

                            let from_int = match self.parse_literal_usize(ectx)? {
                                Some(value) => value,
                                None => {
                                    return Ok(false)
                                }
                            };

                            for _ in 0..3 {
                                let (token, raw) = self.next_token()?;
                                if token.kind != TokenKind::Dot {
                                    ectx.record(
                                        ParModuleSubInvalidClusterDotChain,
                                        format!("Unexpected token '{}'. Expected three dots.", raw),
                                        token.loc
                                    )?;
                                    return Ok(false)
                                }
                            }

                            let to_int = match self.parse_literal_usize(ectx)? {
                                Some(value) => value,
                                None => {
                                    return Ok(false)
                                }
                            };

                            desc.cluster_bounds = Some((from_int, to_int));


                            let (token, raw) = self.next_token()?;
                            if token.kind != TokenKind::CloseBracket {
                                ectx.record(
                                    ParModuleSubMissingClosingBracket,
                                    format!("Unexpected token '{}'. Expected closing bracket.", raw),
                                    token.loc,
                                )?;
                                return Ok(false);
                            }

                            let (token, raw) = self.next_token()?;
                            if token.kind != TokenKind::Colon {
                                ectx.record(
                                    ParModuleSubInvalidSeperator,
                                    format!("Unexpected token '{}'. Expected colon.", raw),
                                    token.loc,
                                )?;
                                return Ok(false);
                            }

                            desc.loc = Loc::fromto(first_token_loc, token.loc);
                        } else {
                            ectx.record(
                                ParModuleSubInvalidSeperator,
                                String::from("Unexpected token. Expected colon ':'."),
                                token.loc,
                            )?;
                            return Ok(false);
                        }
                    } else {
                        desc.loc = Loc::fromto(first_token_loc, token.loc);
                    }
                    

                    if escape_keywords.contains(&&desc.descriptor[..]) {
                        // new subsection ident
                        self.tokens.bump_back(2);
                        ectx.reset_transient();
                        return Ok(false);
                    } else {
                        // new submodule def.
                        self.eat_whitespace();

                        let (second_token, ty) = self.next_token()?;
                        let ty = String::from(ty);
                        if second_token.kind != TokenKind::Ident {
                            ectx.record(
                                ParModuleSubInvalidIdentiferToken,
                                String::from("Unexpected token. Expected type identifer."),
                                second_token.loc
                            )?;
                            return Ok(false);
                        }

                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);

                        child_modules.push(ChildeModuleDef { loc: Loc::fromto(first_token_loc, second_token.loc), ty, desc });
                    }
                },
                _ => {
                    ectx.record(
                        ParModuleSubInvalidIdentiferToken,
                        String::from("Unexpected token. Expected submodule type."),
                        first_token.loc,
                    )?;
                    return Ok(false);
                }
            }

        }

    }

    fn parse_node_connections(&mut self, connections: &mut Vec<ConDef>, ectx: &mut ParsingErrorContext<'_>, _escape_keywords: &[&str]) -> NdlResult<bool> {
        
        // Note that escape keywords are not needed here but will be provided anyway
        // since their usage is likley in the future.

        loop {
            let front_ident = match self.parse_connetion_identifer_token(ectx)? {
                ConIdentiferResult::Result(ident) => ident,
                ConIdentiferResult::Error => return Ok(false),
                ConIdentiferResult::NewSubsection => return Ok(false),
                ConIdentiferResult::Done => return Ok(true),
            };

            self.eat_whitespace();

            let (t1, _raw) = self.next_token()?;
            let (t2, _raw) = self.next_token()?;
            let (t3, _raw) = self.next_token()?;

            let t3_loc = t3.loc;

            use TokenKind::*;
            let to_right = match (t1.kind, t2.kind, t3.kind) {
                (Minus, Minus, Gt) => true,
                (Lt, Minus, Minus) => false,
                _ => {
                    ectx.record(
                        ParModuleConInvaldiChannelSyntax,
                        String::from("Unexpected token. Expected arrow syntax."),
                        Loc::fromto(t1.loc, t3.loc),
                    )?;
                    return Ok(false);
                }
            };


            let mid_ident = match self.parse_connetion_identifer_token(ectx)? {
                ConIdentiferResult::Result(ident) => ident,
                ConIdentiferResult::Error => return Ok(false),
                ConIdentiferResult::NewSubsection => return Ok(false),
                ConIdentiferResult::Done => return Ok(true),
            };

            match  mid_ident {
                ConNodeIdent::Child { .. } => {
                    // Since potential channel ident contains slash 
                    // this MUST be the right node identifer.

                    self.eat_whitespace();
                    self.eat_optionally(|t| t.kind == TokenKind::Comma);

                    if to_right {
                        connections.push(ConDef {
                            loc: Loc::fromto(front_ident.loc(), t3_loc),
    
                            from: front_ident,
                            to: mid_ident,
                            channel: None,
                        })
                    } else {
                        connections.push(ConDef {
                            loc: Loc::fromto(front_ident.loc(), t3_loc),
    
                            from: mid_ident,
                            to: front_ident,
                            channel: None,
                        })
                    }
                },
                ConNodeIdent::Local { ident, loc } => {
                    // This tokem could be either a channel identifer or
                    // node ident.

                    self.eat_whitespace();

                    // # Issue
                    // requesting 3 tokens from the token stream
                    // may not be possible since the token stream may end in < 3 tokens.
                    // The issue is that this may can occur on valid token stream
                    // 
                    // # Exampele
                    // <ident> --> <iden>}
                    // 
                    // t1 will be the first token after the last ident 
                    // -> so allways safe to call
                    // t2 and t3 could be None

                    // check for second arrow
                    let (t1, _raw) = self.next_token()?;
    
                    // Next line, expecting next conident or subident or closing delim
                    if matches!(t1.kind, TokenKind::Ident | TokenKind::CloseBrace | TokenKind::Comma) {
                        // Record valid result
                        if to_right {
                            connections.push(ConDef {
                                loc: Loc::fromto(front_ident.loc(), t3_loc),
        
                                from: front_ident,
                                to: ConNodeIdent::Local { ident, loc },
                                channel: None,
                            })
                        } else {
                            connections.push(ConDef {
                                loc: Loc::fromto(front_ident.loc(), t3_loc),
        
                                from: ConNodeIdent::Local { ident, loc },
                                to: front_ident,
                                channel: None,
                            })
                        }
                        ectx.reset_transient();
                        
                        match t1.kind {
                            TokenKind::Ident => {
                                // Prepare for subident or new con
                                self.tokens.bump_back(1);
                                continue;
                            },
                            TokenKind::Comma => {
                                // Prepare for subident or new con
                                continue;
                            },
                            TokenKind::CloseBrace => {
                                // terminate module / network parsing
                                return Ok(true);
                            },
                            _ => unsafe { std::hint::unreachable_unchecked() }
                        }
                    }

                    let (t2, _raw) = self.next_token()?;
                    let (t3, _raw) = self.next_token()?;
    
                    let t3_loc = t3.loc;
    
                    let to_right2 = match (t1.kind, t2.kind, t3.kind) {
                        (Minus, Minus, Gt) => true,
                        (Lt, Minus, Minus) => false,
                        _ => {
                            ectx.record(
                                ParModuleConInvaldiChannelSyntax,
                                String::from("Unexpected token. Expected arrow syntax"),
                                Loc::fromto(t1.loc, t3.loc),
                            )?;
                            return Ok(false);
                        }
                    };
    
                    if (to_right && to_right2) || (!to_right && !to_right2) {
    
                        let last_ident = match self.parse_connetion_identifer_token(ectx)? {
                            ConIdentiferResult::Result(ident) => ident,
                            ConIdentiferResult::Error => return Ok(false),
                            ConIdentiferResult::NewSubsection => return Ok(false),
                            ConIdentiferResult::Done => return Ok(true),
                        };

                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);
    
                        if to_right {
                            connections.push(ConDef {
                                loc: Loc::fromto(front_ident.loc(), t3_loc),
    
                                from: front_ident,
                                to: last_ident,
                                channel: Some(ident.unwrap_direct()),
                            })
                        } else {
                            connections.push(ConDef {
                                loc: Loc::fromto(front_ident.loc(), t3_loc),
    
                                from: last_ident,
                                to: front_ident,
                                channel: Some(ident.unwrap_direct()),
                            })
                        }
    
                    } else {
                        ectx.record(
                            ParModuleConInvaldiChannelSyntax,
                            String::from("Invalid arrow syntax. Both arrows must match."),
                            Loc::fromto(t1.loc, t3.loc),
                        )?;
                        return Ok(false);
                    }
                }
            }
        }
    }

    fn parse_connetion_identifer_token(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<ConIdentiferResult> {
        use ConIdentiferResult::*;

        self.eat_whitespace();

        let (first_token, id) = self.next_token()?;
        let first_token = first_token.clone();
        let mut id = Ident::Direct { ident: String::from(id) };

        if first_token.kind != TokenKind::Ident {
            
            if first_token.kind == TokenKind::CloseBrace {
                ectx.reset_transient();
                return Ok(Done)
            }

            ectx.record(
                ParModuleConInvalidIdentiferToken,
                String::from("Unexpected token. Expected identifer."),
                first_token.loc,
            )?;
            return Ok(Error);
        }

        let (token, _) = self.next_token()?;
        let mut token = token.clone();

        if token.kind == TokenKind::OpenBracket {
            // Cluster definition
            let num = self.parse_literal_usize(ectx)?;
            // reade closing bracket
            let (token_br, _) = self.next_token()?;

            if token_br.kind != TokenKind::CloseBracket {
                ectx.record_missing_token(ParModuleConMissingClosingBracketForCLusterIdent, String::from("Missing closing bracket for clustered ident."), token_br, "]")?;
            } else if let Some(num) = num{
                
                // Reset id
                if let Ident::Direct { ident } = id {
                    id = Ident::Clustered { ident, index: num }
                } else {
                    unreachable!("This is literallly impossible")
                }

                // reset given token and raw to match the next var
                token = self.next_token()?.0.clone();
            }
        }


        match token.kind {
            TokenKind::Slash => {
                let (token, id_second) = self.next_token()?;
                let id_second = String::from(id_second) ;
                if token.kind != TokenKind::Ident {
                    ectx.record(
                        ParModuleConInvalidIdentiferToken,
                        String::from("Unexpected token. Expected second part identifer"),
                        token.loc,
                    )?;
                    return Ok(Error);
                }

                // Check for clusterd defs.
                let token = self.tokens.peek()?;
                if token.kind == TokenKind::OpenBracket {
                    let _ = self.next_token()?;
                    let num = self.parse_literal_usize(ectx)?;
                    let (token, _raw) = self.next_token()?;

                    if token.kind != TokenKind::CloseBracket {
                        ectx.record_missing_token(ParModuleConMissingClosingBracketForCLusterIdent, String::from("Missing closing bracket for clustered ident."), token, "]")?;
                    } 

                    if let Some(num) = num {
                        ectx.reset_transient();
                        Ok(Result(ConNodeIdent::Child { loc: Loc::fromto(first_token.loc, token.loc), child: id, ident: Ident::Clustered { ident: id_second, index: num}}))
                    } else {
                        Ok(Error)
                    }
                } else {
                    ectx.reset_transient();
                    Ok(Result(ConNodeIdent::Child { loc: Loc::fromto(first_token.loc, token.loc), child: id, ident: Ident::Direct { ident: id_second } }))
                }
              },
            TokenKind::Whitespace => {
                ectx.reset_transient();
                Ok(Result(ConNodeIdent::Local { loc: Loc::fromto(first_token.loc, token.loc), ident: id }))
            },
            TokenKind::Colon => {
                self.tokens.bump_back(2);
                ectx.reset_transient();
                Ok(NewSubsection)
            },
            _ => {
                ectx.record(
                    ParModuleConInvalidIdentiferToken,
                    String::from("Unexpected token. Expected whitespace or slash."),
                    token.loc,
                )?;
                
                Ok(Error)
            },
        }
    }

    fn parse_alias(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<()> {
        let loc = {
            self.tokens.bump_back(1);
            self.next_token().unwrap().0.loc
        };
        // Consumed keyword already
        // expect next token to be a whitspace so eat it.
        self.eat_whitespace();

        let (token, name) = self.next_token()?;
        let name = name.to_string();
        if token.kind != TokenKind::Ident {
            ectx.record(ParAliasMissingIdent, format!("Unexpected token '{}'. Expected ident.", name), token.loc)?;
            return Ok(())
        }

        self.eat_whitespace();
        let (token_like, like) = self.next_token()?;
        if token_like.kind != TokenKind::Ident {
            ectx.record(ParAliasMissingLikeToken, format!("Unexpected token '{}'. Expected 'like'.", &like), token_like.loc)?;
            return Ok(())
        }

        if like != "like" {
            ectx.record(ParAliasMissingLikeKeyword, format!("Unexpected keyword '{}'. Expected 'like'.", like), token_like.loc)?;
            return Ok(())
        }

        self.eat_whitespace();
        let (token, prototype) = self.next_token()?;
        let prototype = prototype.to_string();
        if token.kind != TokenKind::Ident {
            ectx.record(ParAliasMissingPrototypeIdent, format!("Unexpected token '{}'. Expected ident.", name), token.loc)?;
            return Ok(())
        }

        self.result.aliases.push(AliasDef {
            loc,

            name,
            prototype
        });

        Ok(())
    }

    fn parse_network(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<()> {
        ectx.reset_transient();
        self.eat_whitespace();

        let (id_token, id) = self.next_token()?;
        let id_token_loc = id_token.loc;
        let id = String::from(id);
        if id_token.kind != TokenKind::Ident {
            ectx.record(
                ParNetworkMissingIdentifer, 
                String::from("Invalid token. Expected network identfier."), 
                id_token.loc
            )?;
            return Ok(());
        }


        self.eat_whitespace();
        let (token, _raw) = self.next_token()?;
        if token.kind != TokenKind::OpenBrace {
            ectx.record(
                ParNetworkMissingDefBlockOpen, 
                String::from("Invalid token. Expected network definition block (OpenBrace)"), 
                token.loc,
            )?;
            return Ok(());
        }

        // Contents reading

        let mut network_def = NetworkDef {
            loc: Loc::new(0, 1, 1),
       
            name: id,
            nodes: Vec::new(),
            connections: Vec::new(),
            parameters: Vec::new(),
        };

        loop {
            self.eat_whitespace();

            let (subsec_token, subsection_id) = self.next_token()?;
            let subsection_id = String::from(subsection_id);
            if subsec_token.kind != TokenKind::Ident {

                if subsec_token.kind == TokenKind::CloseBrace {
                    ectx.reset_transient();

                    network_def.loc = Loc::fromto(id_token_loc, subsec_token.loc);
                    self.result.networks.push(network_def);
                    return Ok(());
                }

                ectx.record(
                    ParNetworkMissingSectionIdentifier, 
                    format!("Invalid token. Expected identifier for subsection are {}.", NETWORK_SUBSECTION_IDENT.join(" / ")), 
                    subsec_token.loc,
                )?;
                return Ok(());
            }

            if !NETWORK_SUBSECTION_IDENT.contains(&&subsection_id[..]) {
                ectx.record(
                    ParNetworkInvalidSectionIdentifer,
                    format!("Invalid subsection identifier '{}'. Possibilities are {}.", subsection_id, NETWORK_SUBSECTION_IDENT.join(" / ")),
                    subsec_token.loc,
                )?;
                return Ok(());
            }

            let (token, _raw) = self.next_token()?;
            if token.kind != TokenKind::Colon {
                ectx.record(
                    ParNetworkInvalidSeperator,
                    String::from("Unexpected token. Expected colon ':'."),
                    token.loc,
                )?;
            };

            ectx.reset_transient();

            let done = match &subsection_id[..] {
                "nodes" => self.parse_childmodule_def(&mut network_def.nodes, ectx, &NETWORK_SUBSECTION_IDENT)?,
                "connections" => self.parse_node_connections(&mut network_def.connections, ectx, &NETWORK_SUBSECTION_IDENT)?,
                "parameters" => self.parse_par(&mut network_def.parameters, ectx, &NETWORK_SUBSECTION_IDENT)?,
                _ => unreachable!()
            };

            if done {
                break;
            }
        }

        let len = self.tokens.peek()
            .map(|t| t.loc.pos)
            .unwrap_or_else(|_| self.asset.end_pos()) - id_token_loc.pos;
        network_def.loc = Loc::new(id_token_loc.pos, len, id_token_loc.line);

        self.result.networks.push(network_def);

        Ok(())
    }

    fn parse_link(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<()> {
        ectx.reset_transient();

        self.eat_whitespace();
        let (id_token, identifier) = self.next_token()?;
        let id_token_loc = id_token.loc;
        if id_token.kind != TokenKind::Ident {
            ectx.record(
                ParLinkMissingIdentifier,
                String::from("Unexpected token. Expected identifer for link definition"),
                id_token.loc,
            )?;
            return Ok(());
        }

        let identifier = String::from(identifier);
        
        self.eat_whitespace();
        let (paran_open, _raw) = self.next_token()?;
        if paran_open.kind != TokenKind::OpenBrace {
            ectx.record(
                ParLinkMissingDefBlockOpen,
                String::from("Unexpected token. Expected block for link definition"),
                paran_open.loc,
            )?;
            return Ok(());
        }

        let mut bitrate: Option<usize> = None;
        let mut jitter: Option<f64> = None;
        let mut latency: Option<f64> = None;

        while bitrate.is_none() || jitter.is_none() || latency.is_none() {
            self.eat_whitespace();

            let (key_token, raw) = self.next_token()?;
            if key_token.kind != TokenKind::Ident {

                if key_token.kind == TokenKind::CloseBrace {
                    // Unfinished def. Add to stack anyway but print error
                    self.tokens.bump_back(1);
                    break;
                }

                ectx.record(
                    ParLinkInvalidKeyToken,
                    String::from("Unexpected token. Expected identifer for definition key."),
                    key_token.loc,
                )?;
                return Ok(());
            }
            let identifier = String::from(raw);

            self.eat_whitespace();

            let (token, _raw) = self.next_token()?;
            if token.kind != TokenKind::Colon {
                ectx.record(
                    ParLinkInvalidKvSeperator,
                    String::from(
                        "Unexpected token. Expected colon ':' between definition key and value",
                    ),
                    token.loc,
                )?;
                return Ok(());
            }

            self.eat_whitespace();
            let (token, raw) = self.next_token()?;

            match token.kind {
                TokenKind::Literal { kind, .. } => match &identifier[..] {
                    "bitrate" => {
                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);

                        if let LiteralKind::Int { base, .. } = kind {
                            match usize::from_str_radix(raw, base.radix()) {
                                Ok(value) => bitrate = Some(value),
                                Err(e) => {
                                    ectx.record(
                                        ParLiteralIntParseError,
                                        format!("Int parsing error: {}", e), 
                                        token.loc,
                                    )?;
                                    return Ok(());
                                }
                            }
                        } else {
                            ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected integer."), 
                                token.loc,
                            )?;
                            return Ok(());
                        }
                    }

                    "latency" => {
                        use std::str::FromStr;
                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);

                        if let LiteralKind::Float { .. } = kind {
                            match f64::from_str(raw) {
                                Ok(value) => latency = Some(value),
                                Err(e) => {
                                    ectx.record(
                                        ParLiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.loc
                                    )?;
                                    return Ok(());
                                }
                            }
                        } else {
                            ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.loc,
                            )?;
                            return Ok(());
                        }
                    }
                    "jitter" => {
                        use std::str::FromStr;
                        self.eat_whitespace();
                        self.eat_optionally(|t| t.kind == TokenKind::Comma);

                        if let LiteralKind::Float { .. } = kind {
                            match f64::from_str(raw) {
                                Ok(value) => jitter = Some(value),
                                Err(e) => {
                                    ectx.record(
                                        ParLiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.loc,
                                    )?;
                                    return Ok(());
                                }
                            }
                        } else {
                            ectx.record(
                                ParLinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.loc
                            )?;
                            return Ok(());
                        }
                    }
                    _ => {
                        ectx.record(
                            ParLinkInvalidKey, 
                            format!("Invalid key '{}' in kv-pair. Valid keys are latency, bitrate or jitter.", identifier), 
                            key_token.loc,
                        )?;
                        return Ok(());
                    }
                },
                _ => {
                    ectx.record(
                        ParLinkInvalidValueToken,
                        String::from("Unexpected token. Expected literal"),
                        token.loc,
                    )?;
                    return Ok(());
                }
            }
        }

        self.eat_whitespace();

        let (token, _raw) = self.next_token()?;
        if token.kind != TokenKind::CloseBrace {
            ectx.record(
                ParLinkMissingDefBlockClose, 
                String::from("Unexpected token. Expected closing brace."), 
                token.loc
            )?;
            return Ok(());
        }

        if bitrate.is_none() || latency.is_none() || jitter.is_none() {
            // Broke read loop with incomplete def.

            let missing_par = [(bitrate.is_some(), "bitrate"), (jitter.is_some(), "jitter"), (latency.is_some(), "latency")]
                .iter()
                .filter_map(|(v, n)| if *v { Some(*n) } else { None })
                .collect::<Vec<&str>>()
                .join(" + ");

            ectx.record_with_solution(
                ParLinkIncompleteDefinition,
                format!("Channel '{}' was missing some parameters.", identifier),
                Loc::fromto(id_token.loc, token.loc),
                ErrorSolution::new(
                    format!("Add parameters {}", missing_par),
                    id_token.loc,
                ),
            )?;
        }

        let token_loc = token.loc;

        self.result.links.push(LinkDef {
            loc: Loc::fromto(id_token_loc, token_loc),
            
            name: identifier,
            bitrate: bitrate.unwrap_or(1_000),
            latency: latency.unwrap_or(0.1),
            jitter: jitter.unwrap_or(0.1)
        });

        ectx.reset_transient();

        Ok(())
    }

    #[allow(clippy::collapsible_match)]
    fn parse_literal_usize(&mut self, ectx: &mut ParsingErrorContext<'_>) -> NdlResult<Option<usize>> {
        let (token, raw) = self.next_token()?;
        if let TokenKind::Literal { kind, .. } = token.kind {
            if let LiteralKind::Int { base, .. } = kind  {
                match usize::from_str_radix(raw, base.radix()) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => {
                        ectx.record(
                            ParLiteralIntParseError,
                            format!("Error parsing integer: {}", e),
                            token.loc
                        )?;
                        Ok(None)
                    }
                }
            } else {
                ectx.record(
                    ParExpectedIntLiteral,
                format!("Unexpected token '{}'. Expected integer literal", raw),
                    token.loc
                )?;
                Ok(None)
            }
        } else {
            ectx.record(
                ParExpectedIntLiteral,
                format!("Unexpected token '{}'. Expected integer literal", raw),
                token.loc
            )?;

            Ok(None)
        }
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
        for module in &self.modules_and_prototypes {
            writeln!(f, "    - {}{} {{", module.name, if module.is_prototype { " @prototype" } else { "" })?;

            writeln!(f, "      submodules:")?;
            for submodule in &module.submodules {
                writeln!(f, "        {} {}", submodule.ty, submodule.desc)?;
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

        writeln!(f)?;
        writeln!(f, "    aliases:")?;
        for alias in &self.aliases {
            writeln!(f, "    - alias {} like {}", alias.name, alias.prototype)?
        }

        writeln!(f)?;
        writeln!(f, "    networks:")?;
        for module in &self.networks {
            writeln!(f, "    - {} {{", module.name)?;

            writeln!(f, "      nodes:")?;
            for submodule in &module.nodes {
                writeln!(f, "        {} {}", submodule.ty, submodule.desc)?;
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

