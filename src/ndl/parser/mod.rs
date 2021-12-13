use std::fmt::{Display};
use std::io::Write;
use std::{collections::VecDeque};

use termcolor::Color;
use termcolor::ColorChoice;
use termcolor::ColorSpec;
use termcolor::StandardStream;
use termcolor::WriteColor;

use crate::{ChannelMetrics};

use self::error::ErrorCode::*;
use self::error::ErrorContext;

use super::lexer::{LiteralKind, Token, TokenKind, tokenize};

mod error;
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeDef {
    path: Vec<String>,
}

impl Display for IncludeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.join("/"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkDef {
    name: String,
    metrics: ChannelMetrics,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleDef {
    name: String,
    submodule: Vec<(String, String)>,
    gates: Vec<GateDef>,
    connections: Vec<ConDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateDef {
    name: String,
    size: usize
}

impl Display for GateDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.name, self.size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConDef {
    from: ConNodeIdent,
    channel: Option<String>,
    to: ConNodeIdent
}

impl Display for ConDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(channel) = &self.channel {
            write!(f, "{} -->{} --> {}", self.from, channel, self.to)
        } else {
            write!(f, "{} --> {}", self.from, self.to)
        }
        
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConNodeIdent {
    ident: String,
    subident: Option<String>
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct NetworkDef {}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    Loaded,
    Parsed,
    Validated,
}

#[derive(Debug)]
pub struct Parser {
    state: ParserState,

    filepath: String,
    raw: String,
    /// A mapping (idx == line_number) --> (pos == char idx.) of endl char
    raw_line_map: Vec<usize>,
    tokens: VecDeque<Token>,

    includes: Vec<IncludeDef>,
    links: Vec<LinkDef>,
    modules: Vec<ModuleDef>,
    networks: Vec<NetworkDef>,

    errors: ErrorContext,
}

impl Parser {
    pub fn new(filepath: String) -> Self {
        
        let raw = std::fs::read_to_string(&filepath)
            .expect("Failed to read file");

        let tokens = tokenize(&raw).collect();
        
        Self {
            state: ParserState::Loaded,

            filepath,
            raw,
            raw_line_map: Vec::new(),
            tokens ,

            includes: Vec::new(),
            links: Vec::new(),
            modules: Vec::new(),
            networks: Vec::new(),

            errors: ErrorContext::new(),
        }
    }

    pub fn parse(&mut self) -> bool {
        while self.tokens.len() > 0 {
            match self.next_token() {
                Some((token, raw_parts)) => {
                    match token.kind {
                        TokenKind::LineComment => continue,
                        TokenKind::Whitespace => continue,

                        TokenKind::Ident => {
                            let ident = raw_parts;
                            match ident {
                                "include" => self.parse_include(),
                                "module" => self.parse_module(),
                                "link" => self.parse_link(),
                                "network" => self.parse_network(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                None => {}
            }

            self.state = ParserState::Parsed;
        }

        self.errors.len() == 0
    }

    pub fn print_errors(&mut self) -> std::io::Result<()> {

        let mut stream = StandardStream::stdout(ColorChoice::Always);

        for e in self.errors.errors.clone() {
            let fragement = self.select_code_fragment(e.pos, e.len);

            stream.set_color(
                ColorSpec::new().set_fg(Some(Color::Red)),
            )?;
            write!(&mut stream, "error: ")?;
            
            stream.reset()?;
            write!(&mut stream, "{}\n", e.msg)?;
        
            stream.set_color(
                ColorSpec::new().set_fg(Some(Color::Blue))
            )?;
            write!(&mut stream, "   --> ")?;

            stream.reset()?;
            write!(&mut stream, "{}\n", self.filepath)?;
            
            write!(&mut stream, "{}\n", fragement)?;
        }

        Ok(())
    }

    fn select_code_fragment(&mut self, pos: usize, len: usize) -> String {
        if self.raw_line_map.len() == 0 {
            self.generate_line_map()
        }

        let base_line_start = self.get_line_for_pos(pos);
        let base_line_end = self.get_line_for_pos(pos + len);

        let fragement_start_line = base_line_start.checked_sub(2).unwrap_or(0);
        let fragement_end_line = (base_line_end + 1).min(self.raw_line_map.len() - 1);

        String::from(&self.raw
            [(self.raw_line_map[fragement_start_line] + 1)..self.raw_line_map[fragement_end_line]]
        )
    }

    fn get_line_for_pos(&self, pos: usize) -> usize {
        for (line_number, &end_pos) in self.raw_line_map.iter().enumerate() {
            if end_pos > pos {
                return line_number
            }    
        }
        return *self.raw_line_map.last().unwrap()
    }

    fn generate_line_map(&mut self) {
        self.raw_line_map.push(0);
        for (idx, c) in self.raw.chars().enumerate() {
            if c == '\n' {
                self.raw_line_map.push(idx)
            }
        }
    }

    fn parse_include(&mut self) {
        self.errors.reset_transient();
        self.eat_whitespace();

        let mut path_comps = Vec::new();
        let mut expects_comp = true;

        while let Some((token, raw_parts)) = self.next_token() {
            match token.kind {
                TokenKind::Ident if expects_comp => {
                    path_comps.push(String::from(raw_parts));
                    expects_comp = false;
                }
                TokenKind::Slash if !expects_comp => expects_comp = true,
                _ => break,
            }
        }

        self.includes.push(IncludeDef {
            path: path_comps,
        });

        self.eat_whitespace();
    }

    fn parse_module(&mut self) {
        self.errors.reset_transient();
        self.eat_whitespace();

        let (id_token, id) = self.next_token().unwrap();
        let id = String::from(id);
        if id_token.kind != TokenKind::Ident {
            self.errors.record(ModuleMissingIdentifer, String::from("Unexpected token. Expected module identfier."), id_token.pos, id_token.len);
            return;
        }


        self.eat_whitespace();
        let (token, _raw) = self.next_token().unwrap();
        if token.kind != TokenKind::OpenBrace {
            self.errors.record(
                ModuleMissingDefBlockOpen, 
                String::from("Unexpected token. Expected module definition block"), 
                token.pos, 
                token.len
            );
            return;
        }

        // Contents reading

        let mut module_def = ModuleDef {
            name: id,
            gates: Vec::new(),
            submodule: Vec::new(),
            connections: Vec::new(),
        };

        loop {
            self.eat_whitespace();

            let (token, subsection_id) = self.next_token().unwrap();
            let subsection_id = String::from(subsection_id);
            if token.kind != TokenKind::Ident {
                self.errors.record(
                    ModuleMissingSectionIdentifier, 
                    String::from("Unexpected token. Expected identifier for subsection"), 
                    token.pos, 
                    token.len
                );
                return;
            }

            if !(vec!["gates", "submodules", "connections"]).contains(&&subsection_id[..]) {
                self.errors.record(
                    ModuleInvalidSectionIdentifer,
                    String::from("Invalid subsection identifier. Valid are gates / submodules or connections"),
                    token.pos,
                    token.len
                );
                return;
            }

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::Colon {
                self.errors.record(
                    ModuleInvalidSeperator,
                    String::from("Unexpected token. Expected colon ':'"),
                    token.pos,
                    token.len,
                )
            };

            self.errors.reset_transient();

            let done = match &subsection_id[..] {
                "gates" => self.parse_module_gates(&mut module_def),
                "submodules" => self.parse_module_submodules(&mut module_def),
                "connections" => self.parse_module_connections(&mut module_def),
                _ => todo!()
            };

            if done {
                break;
            }
        }

        self.modules.push(module_def);
    }

    fn parse_module_gates(&mut self, module_def: &mut ModuleDef) -> bool {

        loop {
            self.eat_whitespace();

            let (name_token, name) = self.next_token().unwrap();
            let name = String::from(name);
            if name_token.kind != TokenKind::Ident {

                if name_token.kind == TokenKind::CloseBrace {
                    self.errors.reset_transient();
                    return true;
                }

                self.errors.record(
                    ModuleInvalidKeyToken,
                    String::from("Invalid key token. Expected gate identifer"),
                    name_token.pos,
                    name_token.len,
                );
                return false;
            }

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::OpenBracket {
                // Single size gate
                if token.kind == TokenKind::Whitespace {
                    module_def.gates.push(GateDef { name, size: 1 })
                } if token.kind == TokenKind::Colon {
                    // New identifer
                    self.tokens.push_front(token);
                    self.tokens.push_front(name_token);
                    self.errors.reset_transient();
                    return false;
                } else {
                    self.errors.record(
                        ModuleGateInvalidIdentifierToken,
                        String::from("Unexpected token. Expected whitespace."),
                        token.pos,
                        token.len
                    );
                    return false;
                }
                
            } else {
                // cluster gate

                let (token, literal) = self.next_token().unwrap();
                match token.kind {
                    TokenKind::Literal { kind, ..} => {
                        if let LiteralKind::Int { base, .. } = kind {
                            match usize::from_str_radix(literal, base.radix()) {
                                Ok(value) => { 
                                    let (token, _raw) = self.next_token().unwrap();
                                    if token.kind != TokenKind::CloseBracket {
                                        self.errors.record(
                                            ModuleGateMissingClosingBracket,
                                            String::from("Unexpected token. Expected closing bracket"),
                                            token.pos,
                                            token.len,
                                        );
                                        return false;
                                    }

                                    module_def.gates.push(GateDef { name, size: value }); 
                                },
                                Err(e) => {
                                    self.errors.record(
                                        LiteralIntParseError, 
                                        format!("Int parse error: {}", e), 
                                        token.pos, 
                                        token.len
                                    );
                                    return false;
                                }
                            }

                        } else {
                            self.errors.record(
                                ModuleGateInvalidGateSize,
                                String::from("Unexpected token. Expected literal gate size definition (Int)."),
                                token.pos,
                                token.len,
                            );
                            return false;
                        }
                    }
                    _ => {
                        self.errors.record(
                            ModuleGateInvalidGateSize,
                            String::from("Unexpected token. Expected literal gate size definition (Int)."),
                            token.pos,
                            token.len,
                        );
                        return false;
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
                    self.errors.reset_transient();
                    return true;
                },
                TokenKind::Ident => {

                    let (token, _raw) = self.next_token().unwrap();
                    if token.kind != TokenKind::Whitespace {
                        if token.kind == TokenKind::Colon {
                            // new subsection
                            self.tokens.push_front(token);
                            self.tokens.push_front(ty_token);
                            self.errors.reset_transient();
                        } else {
                            self.errors.record(
                                ModuleSubInvalidSeperator,
                                String::from("Unexpected token. Expected whitespace."),
                                token.pos,
                                token.len
                            );
                        }
                        return false;
                    }

                    let (token, defname) = self.next_token().unwrap();
                    let defname = String::from(defname);
                    if token.kind != TokenKind::Ident {
                        self.errors.record(
                            ModuleSubInvalidIdentiferToken, 
                            String::from("Unexpected token. Expected submodule identifer"),
                            token.pos,
                            token.len,
                        );
                        return false;
                    }

                   module_def.submodule.push((ty, defname));
                },
                _ => {
                    println!("{:?}", ty_token);
                    self.errors.record(
                        ModuleSubInvalidIdentiferToken,
                        String::from("Unexpected token. Expected submodule type"),
                        ty_token.pos,
                        ty_token.len,
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
                    self.errors.record(
                        ModuleConInvaldiChannelSyntax,
                        String::from("Unexpected token. Expected arrow syntax."),
                        t1.pos,
                        t1.len + t2.len + t3.len
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
                        from: front_ident,
                        to: mid_ident,
                        channel: None,
                    })
                } else {
                    module_def.connections.push(ConDef {
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

                let to_right2 = match (t1.kind, t1.kind, t3.kind) {
                    (Minus, Minus, Gt) => true,
                    (Lt, Minus, Minus) => false,
                    _ => {
                        self.errors.record(
                            ModuleConInvaldiChannelSyntax,
                            String::from("Unexpected token. Expected arrow syntax"),
                            t1.pos,
                            t1.len + t2.len + t3.len
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
                            from: front_ident,
                            to: last_ident,
                            channel: Some(mid_ident.ident),
                        })
                    } else {
                        module_def.connections.push(ConDef {
                            from: last_ident,
                            to: front_ident,
                            channel: Some(mid_ident.ident),
                        })
                    }

                } else {
                    self.errors.record(
                        ModuleConInvaldiChannelSyntax,
                        String::from("Invalid arrow syntax. Both arrows must match."),
                        t1.pos,
                        t1.len + t2.len + t3.len,
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
                self.errors.reset_transient();
                return Done
            }

            self.errors.record(
                ModuleConInvalidIdentiferToken,
                String::from("Unexpected token. Expected identifer."),
                first_token.pos,
                first_token.len,
            );
            return Error;
        }

        let (token, _raw) = self.next_token().unwrap();
        match token.kind {
            TokenKind::Slash => {
                let (token, id_second) = self.next_token().unwrap();
                let id_second = String::from(id_second);
                if token.kind != TokenKind::Ident {
                    self.errors.record(
                        ModuleConInvalidIdentiferToken,
                        String::from("Unexpected token. Expected second part identifer"),
                        token.pos,
                        token.len,
                    );
                    return Error;
                }

                self.errors.reset_transient();
                return Result(ConNodeIdent { ident: id, subident: Some(id_second) } )
            },
            TokenKind::Whitespace => {
                self.errors.reset_transient();
                return Result(ConNodeIdent { ident: id, subident: None })
            },
            TokenKind::Colon => {
                self.errors.reset_transient();

                self.tokens.push_front(token);
                self.tokens.push_front(first_token);

                return NewSubsection;
            },
            _ => {
                self.errors.record(
                    ModuleConInvalidIdentiferToken,
                    String::from("Unexpected token. Expected whitespace or slash."),
                    token.pos,
                    token.len,
                );
                return Error;
            },
        }
    }

    fn parse_link(&mut self) {
        self.errors.reset_transient();

        self.eat_whitespace();
        let (id_token, identifier) = self.next_token().unwrap();
        if id_token.kind != TokenKind::Ident {
            self.errors.record(
                LinkMissingIdentifier,
                String::from("Unexpected token. Expected identifer for link definition"),
                id_token.pos,
                id_token.len,
            );
            return;
        }

        let identifier = String::from(identifier);
        
        self.eat_whitespace();
        let (paran_open, _raw) = self.next_token().unwrap();
        if paran_open.kind != TokenKind::OpenBrace {
            self.errors.record(
                LinkMissingDefBlockOpen,
                String::from("Unexpected token. Expected block for link definition"),
                paran_open.pos,
                paran_open.len,
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
                self.errors.record(
                    LinkInvalidKeyToken,
                    String::from("Unexpected token. Expected identifer for definition key."),
                    key_token.pos,
                    key_token.len,
                );
                return;
            }
            let identifier = String::from(raw);

            self.eat_whitespace();

            let (token, _raw) = self.next_token().unwrap();
            if token.kind != TokenKind::Colon {
                self.errors.record(
                    LinkInvalidKvSeperator,
                    String::from(
                        "Unexpected token. Expected colon ':' between definition key and value",
                    ),
                    token.pos,
                    token.len,
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
                                    self.errors.record(
                                        LiteralIntParseError,
                                        format!("Int parsing error: {}", e), 
                                        token.pos, 
                                        token.len
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.errors.record(
                                LinkInvalidValueType, 
                                String::from("Invalid value type. Expected integer."), 
                                token.pos, 
                                token.len
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
                                    self.errors.record(
                                        LiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.pos, 
                                        token.len
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.errors.record(
                                LinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.pos, 
                                token.len
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
                                    self.errors.record(
                                        LiteralFloatParseError,
                                        format!("Float parsing error: {}", e), 
                                        token.pos, 
                                        token.len
                                    );
                                    return;
                                }
                            }
                        } else {
                            self.errors.record(
                                LinkInvalidValueType, 
                                String::from("Invalid value type. Expected float."), 
                                token.pos, 
                                token.len
                            );
                            return;
                        }
                    }
                    _ => {
                        self.errors.record(
                            LinkInvalidKey, 
                            format!("Invlaid key '{}' in kv-pair. Valid keys are latency, bitrate or jitter.", identifier), 
                            key_token.pos, 
                            key_token.len
                        );
                        return;
                    }
                },
                _ => {
                    self.errors.record(
                        LinkInvalidValueToken,
                        String::from("Unexpected token. Expected literal"),
                        token.pos,
                        token.len,
                    );
                    return;
                }
            }
        }

        self.eat_whitespace();

        let (token, _raw) = self.next_token().unwrap();
        if token.kind != TokenKind::CloseBrace {
            self.errors.record(LinkMissingDefBlockClose, String::from("Unexpected token. Expected closing brace."), token.pos, token.len);
            return;
        }

        self.links.push(LinkDef {
            name: identifier,
            metrics: ChannelMetrics::new(
                bitrate.unwrap(),
                latency.unwrap().into(),
                jitter.unwrap().into(),
            ),
        });

        self.errors.reset_transient()
    }

    fn parse_network(&mut self) {}

    fn eat_whitespace(&mut self) {
        while self.tokens.front().is_some()
            && self.tokens.front().unwrap().kind == TokenKind::Whitespace
        {
             self.tokens.pop_front().unwrap();
        }
    }

    fn next_token(&mut self) -> Option<(Token, &str)> {
        let token = self.tokens.pop_front()?;
        let raw_parts = &self.raw[token.pos..(token.pos + token.len)];
        Some((token, raw_parts))
    }
}

enum ConIdentiferResult {
    Error,
    Result(ConNodeIdent),
    NewSubsection,
    Done
}

impl Display for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parser({:?}) {{", self.state)?;
        
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
                writeln!(f, "        {} {}", submodule.0, submodule.1)?;
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