macro_rules! ast_expect_single_token {
    (
        $vis:vis struct $type:ident {
            token: $token:expr,
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $type {
            $vis span: crate::Span
        }

        impl crate::ast::parse::Parse for $type {
            fn parse(input: crate::ast::parse::ParseStream)
                -> crate::error::Result<$type> {
                let Some(peek) = input.ts.peek() else {
                    return Err(
                        crate::error::Error::new(
                            crate::error::ErrorKind::UnexpectedToken,
                            format!("expected {}, found EOF", $token.token_kind_err_output())
                        )
                    );
                };

                if let crate::ast::token::TokenTree::Token(token, spacing) = peek {
                    if crate::ast::token::Spacing::Alone != *spacing {
                        return Err(
                            crate::error::Error::new(
                                crate::error::ErrorKind::ExpectedSingleFoundJoint,
                                format!("expected {}, found invalid spacing", $token.token_kind_err_output())
                            ).spanned(token.span)
                        );
                    }

                    if token.kind == $token {
                        let ret = Ok(Self { span: token.span });
                        input.ts.bump();
                        ret
                    } else {
                        Err(
                            crate::error::Error::new(
                                crate::error::ErrorKind::UnexpectedToken,
                                format!("expected {}, found {}", $token.token_kind_err_output(), token.kind.token_kind_err_output())
                            ).spanned(token.span)
                        )
                    }
                } else {
                    Err(
                        crate::error::Error::new(
                            crate::error::ErrorKind::UnexpectedDelim,
                            format!("expected {}, found delim", $token.token_kind_err_output())
                        ).spanned(peek.span())
                    )
                }
            }
        }

        impl crate::ast::parse::Spanned for $type {
            fn span(&self) -> crate::resource::Span {
                self.span
            }
        }
    };
}
