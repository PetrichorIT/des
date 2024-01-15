use std::fmt;

use crate::{
    ast::{
        parse::*, ClusterDefinition, Comma, ConnectionsToken, Delimited, Delimiter, Ident,
        LeftSingleArrow, Punctuated, RightSingleArrow, Slash, LeftRightSingleArrow, EitherOr,
    },
    error::Result,
    resource::Span,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionsStmt {
    pub keyword: ConnectionsToken,
    pub span: Span,
    pub items: Punctuated<ConnectionDefinition, Comma>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDefinition {
    pub lhs: ModuleGateReference,
    pub rhs: ModuleGateReference,
    pub first_arrow: EitherOr<LeftSingleArrow, LeftRightSingleArrow>,
    pub second_arrow: Option<RightSingleArrow>,
    pub link: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleGateReference {
    Local(LocalModuleGateReference),
    Nonlocal(NonlocalModuleGateReference),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalModuleGateReference {
    pub gate: Ident,
    pub gate_cluster: Option<ClusterDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NonlocalModuleGateReference {
    pub submodule: Ident,
    pub submodule_cluster: Option<ClusterDefinition>,
    pub slash: Slash,
    pub gate: LocalModuleGateReference,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionArrow {
    Double(LeftRightSingleArrow),
    Left(LeftSingleArrow),
    Right(RightSingleArrow),
}

// # Impl

impl ConnectionArrow {
    pub fn is_double(&self) -> bool {
        matches!(self, Self::Double(_))
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right(_))
    }
}

impl LocalModuleGateReference {
    pub fn pos(&self) -> usize {
        self.gate_cluster
            .as_ref()
            .map(|c| c.lit.as_integer() as usize)
            .unwrap_or(0)
    }
}

impl fmt::Display for ModuleGateReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(local) => local.fmt(f),
            Self::Nonlocal(nonlocal) => nonlocal.fmt(f),
        }
    }
}

impl fmt::Display for LocalModuleGateReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cluster) = &self.gate_cluster {
            write!(f, "{}{}", self.gate.raw, cluster)
        } else {
            write!(f, "{}", self.gate.raw)
        }
    }
}

impl fmt::Display for NonlocalModuleGateReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.submodule.raw)?;
        if let Some(cluster) = &self.submodule_cluster {
            write!(f, "{}", cluster)?;
        }
        write!(f, "/{}", self.gate)?;

        Ok(())
    }
}

// # Spanning

impl Spanned for ConnectionsStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ConnectionDefinition {
    fn span(&self) -> Span {
        Span::fromto(self.lhs.span(), self.rhs.span())
    }
}

impl Spanned for ModuleGateReference {
    fn span(&self) -> Span {
        match self {
            Self::Local(local) => local.span(),
            Self::Nonlocal(nonlocal) => nonlocal.span(),
        }
    }
}

impl Spanned for LocalModuleGateReference {
    fn span(&self) -> Span {
        Span::fromto(
            self.gate.span(),
            self.gate_cluster
                .as_ref()
                .map(|v| v.span())
                .unwrap_or(self.gate.span()),
        )
    }
}

impl Spanned for NonlocalModuleGateReference {
    fn span(&self) -> Span {
        Span::fromto(self.submodule.span(), self.gate.span())
    }
}

impl Spanned for ConnectionArrow {
    fn span(&self) -> Span {
        match self {
            Self::Double(d) => d.span(),
            Self::Left(left) => left.span(),
            Self::Right(right) => right.span(),
        }
    }
}

// # Parse

impl Parse for ConnectionsStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = ConnectionsToken::parse(input)?;
        let delim = Delimited::<Punctuated<ConnectionDefinition, Comma>>::parse_from(
            Delimiter::Brace,
            input,
        )?;
        let span = Span::fromto(delim.delim_span.open, delim.delim_span.close);
        Ok(ConnectionsStmt {
            keyword,
            span,
            items: delim.inner,
        })
    }
}

impl Parse for ConnectionDefinition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let first = ModuleGateReference::parse(input)?;
        let first_arrow = EitherOr::parse(input)?;

        if matches!(first_arrow, EitherOr::Either(_)){
            // Could be a delayed connections
            let link = Ident::parse(input)?;
            let second_arrow = RightSingleArrow::parse(input)?;
            let third = ModuleGateReference::parse(input)?;

            Ok(ConnectionDefinition {
                lhs: first,
                rhs: third,
                first_arrow,
                second_arrow: Some(second_arrow),
                link: Some(link),
            })
        } else {
            let second = ModuleGateReference::parse(input)?;

            Ok(ConnectionDefinition {
                lhs: first,
                first_arrow,
                rhs: second,
                second_arrow: None,
                link: None,
            })
        }
    }
}

impl Parse for ModuleGateReference {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let old_state = input.ts.state();
        match NonlocalModuleGateReference::parse(input) {
            Ok(v) => Ok(ModuleGateReference::Nonlocal(v)),
            Err(_e) => {
                input.ts.set_state(old_state);
                let local = LocalModuleGateReference::parse(input)?;
                Ok(ModuleGateReference::Local(local))
            }
        }
    }
}

impl Parse for LocalModuleGateReference {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let gate = Ident::parse(input)?;
        let gate_cluster = Option::<ClusterDefinition>::parse(input)?;
        Ok(LocalModuleGateReference { gate, gate_cluster })
    }
}

impl Parse for NonlocalModuleGateReference {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let submodule = Ident::parse(input)?;
        let submodule_cluster = Option::<ClusterDefinition>::parse(input)?;
        let slash = Slash::parse(input)?;
        let gate = Ident::parse(input)?;
        let gate_cluster = Option::<ClusterDefinition>::parse(input)?;

        Ok(NonlocalModuleGateReference {
            submodule,
            submodule_cluster,
            slash,
            gate: LocalModuleGateReference { gate, gate_cluster },
        })
    }
}

impl Parse for ConnectionArrow {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        LeftSingleArrow::parse(input)
            .map(ConnectionArrow::Left)
            .or_else(|_| RightSingleArrow::parse(input).map(ConnectionArrow::Right))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::TokenStream, SourceMap};

    #[test]
    fn simple_noncluster_connections() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "connections { from <--> to, iden_t <--> from_dent }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].lhs.to_string(), "from");
        assert_eq!(items[0].rhs.to_string(), "to");
        assert_eq!(items[0].link, None);

        assert_eq!(items[1].lhs.to_string(), "iden_t");
        assert_eq!(items[1].rhs.to_string(), "from_dent");
        assert_eq!(items[1].link, None);

        // # Case 1
        let asset = smap.load_raw("raw:case1", "connections { from <--> 123 }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 2
        let asset = smap.load_raw("raw:case2", "connections { from + <--> ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections {  <--> ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections { from <--> ident,, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();
    }

    #[test]
    fn simple_cluster_connections() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "connections { from[1] <--> to, iden_t[10] <--> from_dent[12] }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();

        assert_eq!(items[0].lhs.to_string(), "from[1]");
        assert_eq!(items[0].rhs.to_string(), "to");
        assert_eq!(items[0].link, None);

        assert_eq!(items[1].lhs.to_string(), "iden_t[10]");
        assert_eq!(items[1].rhs.to_string(), "from_dent[12]");
        assert_eq!(items[1].link, None);

        // # Case 1
        let asset = smap.load_raw("raw:case1", "connections { from[ident] --> to }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 2
        let asset = smap.load_raw("raw:case2", "connections { from[] --> to }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections { from[213 --> to }");
        let _ts = TokenStream::new(asset).unwrap_err();
    }

    #[test]
    fn nonlocal_noncluster_connections() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "connections { child/from <--> child/to, child/iden_t <--> child/from_dent }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].lhs.to_string(), "child/from");
        assert_eq!(items[0].rhs.to_string(), "child/to");
        assert_eq!(items[0].link, None);

        assert_eq!(items[1].lhs.to_string(), "child/iden_t");
        assert_eq!(items[1].rhs.to_string(), "child/from_dent");
        assert_eq!(items[1].link, None);

        // # Case 1
        let asset = smap.load_raw("raw:case1", "connections { child/from --> child/123 }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 2
        let asset = smap.load_raw("raw:case2", "connections { child/from + --> child/ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections {  --> child/ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections { child/ --> ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();
    }

    #[test]
    fn nonlocal_cluster_connections() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "connections { child/from[1] <--> child/to, child/iden_t[10] <--> child/from_dent[12] }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();

        assert_eq!(items[0].lhs.to_string(), "child/from[1]");
        assert_eq!(items[0].rhs.to_string(), "child/to");
        assert_eq!(items[0].link, None);

        assert_eq!(items[1].lhs.to_string(), "child/iden_t[10]");
        assert_eq!(items[1].rhs.to_string(), "child/from_dent[12]");
        assert_eq!(items[1].link, None);

        // # Case 1
        let asset = smap.load_raw(
            "raw:case1",
            "connections { child/from[ident] <--> child/to }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 2
        let asset = smap.load_raw("raw:case2", "connections { child/from[] <--> child/to }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 3
        let asset = smap.load_raw("raw:case3", "connections { child/from[213 <--> child/to }");
        let _ts = TokenStream::new(asset).unwrap_err();

        // # Case 4
        let asset = smap.load_raw(
            "raw:case4",
            "connections { child[1]/from <--> child/to, child[10]/iden_t <--> child[12]/from_dent }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();

        assert_eq!(items[0].lhs.to_string(), "child[1]/from");
        assert_eq!(items[0].rhs.to_string(), "child/to");
        assert_eq!(items[0].link, None);

        assert_eq!(items[1].lhs.to_string(), "child[10]/iden_t");
        assert_eq!(items[1].rhs.to_string(), "child[12]/from_dent");
        assert_eq!(items[1].link, None);

        // # Case 5
        let asset = smap.load_raw(
            "raw:case5",
            "connections { child[ident]/from <--> child/to }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();

        // # Case 6
        let asset = smap.load_raw("raw:case6", "connections { child[]/from <--> child[1]/to }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ConnectionsStmt::parse(&buf).unwrap_err();
    }

    #[test]
    fn delayed_connections() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "connections { from <-- FastLink --> to, iden_t <-- L --> from_dent }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].lhs.to_string(), "from");
        assert_eq!(items[0].rhs.to_string(), "to");
        assert_eq!(items[0].link.as_ref().map(|v| &v.raw[..]), Some("FastLink"));

        assert_eq!(items[1].lhs.to_string(), "iden_t");
        assert_eq!(items[1].rhs.to_string(), "from_dent");
        assert_eq!(items[1].link.as_ref().map(|v| &v.raw[..]), Some("L"));

        // # Case 1
        let asset = smap.load_raw(
            "raw:case1",
            "connections { from[1] <-- FastLink --> to, iden_t[5] <--> from_dent }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ConnectionsStmt::parse(&buf).unwrap();
        let items = stmt.items.iter().cloned().collect::<Vec<_>>();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].lhs.to_string(), "from[1]");
        assert_eq!(items[0].rhs.to_string(), "to");
        assert_eq!(items[0].link.as_ref().map(|v| &v.raw[..]), Some("FastLink"));

        assert_eq!(items[1].lhs.to_string(), "iden_t[5]");
        assert_eq!(items[1].rhs.to_string(), "from_dent");
        assert_eq!(items[1].link.as_ref().map(|v| &v.raw[..]), None);
    }
}
