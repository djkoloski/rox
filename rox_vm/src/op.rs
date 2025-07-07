use core::fmt;

use crate::{Codec, DecodeError, codec::U24};

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
        Constant {
            index: u8,
        },
        ConstantLong {
            #[codec(U24)]
            index: u32,
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
    }
}
