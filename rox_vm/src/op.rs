use core::fmt;

use crate::{
    Codec, DecodeError,
    codec::{U8, U16, U24},
};

macro_rules! define_ops {
    (
        pub enum Op {
            $(
                $name:ident
                $({
                    $(
                        $(#[codec($codec:ty)])?
                        $field:ident: $ty:ty
                    ),* $(,)?
                })?
            ),* $(,)?
        }
    ) => {

        #[derive(Debug)]
        #[repr(u8)]
        pub enum Op {
            $($name $({ $($field: $ty,)* })?,)*
        }

        #[allow(non_upper_case_globals)]
        mod opcode {
            #[repr(u8)]
            enum Opcode {
                $($name,)*
            }

            $(pub const $name: u8 = Opcode::$name as u8;)*
        }

        impl Codec for Op {
            fn encode(self, bytes: &mut Vec<u8>) {
                match self {
                    $(
                        Self::$name $({ $($field,)* })? => {
                            opcode::$name.encode(bytes);
                            $($(
                                <
                                    define_ops!(@codec_ty $ty $(, $codec)?)
                                >::from($field).encode(bytes);
                            )*)?
                        }
                    )*
                }
            }

            fn decode(
                bytes: &[u8],
                offset: &mut usize,
            ) -> Result<Self, DecodeError> {
                match u8::decode(bytes, offset)? {
                    $(
                        opcode::$name => {
                            $($(
                                let $field = <$ty>::from(<
                                    define_ops!(@codec_ty $ty $(, $codec)?)
                                >::decode(bytes, offset)?);
                            )*)?
                            Ok(Self::$name $({ $($field,)* })?)
                        }
                    )*
                    byte => Err(DecodeError::InvalidOpcode { byte }),
                }
            }
        }

        impl fmt::Display for Op {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$name $({ $($field,)* })? => {
                        write!(f, "{}", stringify!($name))?;
                        $(
                            write!(f, " {{")?;
                            $(
                                write!(
                                    f,
                                    " {}={}",
                                    stringify!($field),
                                    $field,
                                )?;
                            )*
                            write!(f, " }}")?;
                        )?
                    })*
                }

                Ok(())
            }
        }
    };
    (@codec_ty $ty:ty, $codec:ty) => { $codec };
    (@codec_ty $ty:ty) => { $ty };
}

define_ops! {
    pub enum Op {
        Return,
        Nil,
        True,
        False,
        Constant {
            #[codec(U8)]
            index: usize,
        },
        ConstantLong {
            #[codec(U24)]
            index: usize,
        },
        Not,
        Negate,
        Add,
        Subtract,
        Multiply,
        Divide,
        Equal,
        Greater,
        Less,
        Print,
        Pop,
        DefineGlobal {
            #[codec(U8)]
            index: usize,
        },
        DefineGlobalLong {
            #[codec(U24)]
            index: usize,
        },
        GetGlobal {
            #[codec(U8)]
            index: usize,
        },
        GetGlobalLong {
            #[codec(U24)]
            index: usize,
        },
        SetGlobal {
            #[codec(U8)]
            index: usize,
        },
        SetGlobalLong {
            #[codec(U24)]
            index: usize,
        },
        GetLocal {
            #[codec(U8)]
            index: usize,
        },
        GetLocalLong {
            #[codec(U24)]
            index: usize,
        },
        SetLocal {
            #[codec(U8)]
            index: usize,
        },
        SetLocalLong {
            #[codec(U24)]
            index: usize,
        },
        JumpIfFalse {
            #[codec(U16)]
            distance: usize,
        },
        Jump {
            #[codec(U16)]
            distance: usize,
        },
        Loop {
            #[codec(U16)]
            distance: usize,
        }
    }
}

macro_rules! long_ops {
    ($($fn:ident: $short:ident $long:ident),* $(,)?) => {
        impl Op {
            $(
                pub fn $fn(index: usize) -> Self {
                    if index <= U8::MAX {
                        Self::$short { index }
                    } else if index <= U24::MAX {
                        Self::$long { index }
                    } else {
                        panic!(
                            ::core::concat!(
                                "attempted to encode a ",
                                ::core::stringify!($fn),
                                " op with an index that was too large ({})",
                            ),
                            index,
                        );
                    }
                }
            )*
        }
    };
}

long_ops! {
    constant: Constant ConstantLong,
    define_global: DefineGlobal DefineGlobalLong,
    get_global: GetGlobal GetGlobalLong,
    set_global: SetGlobal SetGlobalLong,
    get_local: GetLocal GetLocalLong,
    set_local: SetLocal SetLocalLong,
}
