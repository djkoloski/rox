pub trait TokenKind {
    fn matches_token(token: &Token) -> bool;
    fn from_token(token: Token) -> Self;
}

macro_rules! define_tokens {
    (
        pub enum $name:ident {
            $($variant:ident $(($value:ty))?),* $(,)?
        }
    ) => {
        pub mod token_kind {
            $(
                #[derive(Clone, Debug)]
                pub struct $variant {
                    pub span: ::rox_diag::Span,
                    $(pub value: $value,)*
                }

                impl ::rox_diag::Spanned for $variant {
                    fn span_start(&self) -> usize {
                        self.span.start()
                    }

                    fn span_end(&self) -> usize {
                        self.span.end()
                    }
                }

                impl $crate::TokenKind for $variant {
                    fn matches_token(token: &$crate::Token) -> bool {
                        ::core::matches!(token, $crate::Token::$variant(_))
                    }

                    fn from_token(token: $crate::Token) -> Self {
                        let $crate::Token::$variant(this) = token else {
                            ::core::unreachable!()
                        };
                        this
                    }
                }

                define_tokens!(@impl_from_span $variant $($value)?);
            )*
        }

        #[derive(Clone, Debug)]
        pub enum $name {
            $($variant(token_kind::$variant),)*
        }

        $(
            impl From<token_kind::$variant> for $name {
                fn from(token: token_kind::$variant) -> Self {
                    Self::$variant(token)
                }
            }
        )*

        impl ::rox_diag::Spanned for $name {
            fn span_start(&self) -> usize {
                match self {
                    $(Self::$variant(token) => token.span_start(),)*
                }
            }

            fn span_end(&self) -> usize {
                match self {
                    $(Self::$variant(token) => token.span_end(),)*
                }
            }
        }
    };
    (@impl_from_span $variant:ident) => {
        impl From<::rox_diag::Span> for $variant {
            fn from(span: ::rox_diag::Span) -> Self {
                Self { span }
            }
        }
    };
    (@impl_from_span $variant:ident $($value:ty)?) => {};
}

define_tokens! {
    pub enum Token {
        LeftParen,
        RightParen,
        LeftBrace,
        RightBrace,
        Comma,
        Dot,
        Minus,
        Plus,
        Semicolon,
        Slash,
        Star,
        Bang,
        BangEqual,
        Equal,
        EqualEqual,
        Greater,
        GreaterEqual,
        Less,
        LessEqual,
        Ident(std::string::String),
        String(std::string::String),
        Number(f64),
        And,
        Class,
        Else,
        False,
        Fun,
        For,
        If,
        Nil,
        Or,
        Print,
        Return,
        Super,
        This,
        True,
        Var,
        While,
        Eof,
    }
}
