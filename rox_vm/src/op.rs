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
            constant_index: usize,
        },
        ConstantLong {
            #[codec(U24)]
            constant_index: usize,
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
            constant_index: usize,
        },
        DefineGlobalLong {
            #[codec(U24)]
            constant_index: usize,
        },
        GetGlobal {
            #[codec(U8)]
            constant_index: usize,
        },
        GetGlobalLong {
            #[codec(U24)]
            constant_index: usize,
        },
        SetGlobal {
            #[codec(U8)]
            constant_index: usize,
        },
        SetGlobalLong {
            #[codec(U24)]
            constant_index: usize,
        },
        GetLocal {
            #[codec(U8)]
            local_index: usize,
        },
        GetLocalLong {
            #[codec(U24)]
            local_index: usize,
        },
        SetLocal {
            #[codec(U8)]
            local_index: usize,
        },
        SetLocalLong {
            #[codec(U24)]
            local_index: usize,
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
        },
        PushFrame,
        Call {
            #[codec(U8)]
            arity: usize,
        },
        CloseFunction {
            #[codec(U8)]
            function_index: usize,
        },
        CloseFunctionLong {
            #[codec(U24)]
            function_index: usize,
        },
        CloseLocal,
        GetUpvalue {
            #[codec(U8)]
            upvalue_index: usize,
        },
        GetUpvalueLong {
            #[codec(U24)]
            upvalue_index: usize,
        },
        SetUpvalue {
            #[codec(U8)]
            upvalue_index: usize,
        },
        SetUpvalueLong {
            #[codec(U24)]
            upvalue_index: usize,
        },
        Class {
            #[codec(U8)]
            class_index: usize,
        },
        ClassLong {
            #[codec(U24)]
            class_index: usize,
        },
        GetField {
            #[codec(U8)]
            constant_index: usize,
        },
        GetFieldLong {
            #[codec(U24)]
            constant_index: usize,
        },
        SetField {
            #[codec(U8)]
            constant_index: usize,
        },
        SetFieldLong {
            #[codec(U24)]
            constant_index: usize,
        },
        Invoke {
            #[codec(U8)]
            constant_index: usize,
            #[codec(U8)]
            arity: usize,
        },
        InvokeLong {
            #[codec(U24)]
            constant_index: usize,
            #[codec(U8)]
            arity: usize,
        },
    }
}

macro_rules! long_ops {
    ($(
        $fn:ident: $field:ident $(, $rest:ident: $rest_ty:ty)* $(,)?
        => $short:ident, $long:ident
    );* $(;)?) => {
        impl Op {
            $(
                pub fn $fn($field: usize $(, $rest: $rest_ty)*) -> Self {
                    if $field <= U8::MAX {
                        Self::$short { $field $(, $rest)* }
                    } else if $field <= U24::MAX {
                        Self::$long { $field $(, $rest)* }
                    } else {
                        panic!(
                            ::core::concat!(
                                "attempted to encode a ",
                                ::core::stringify!($fn),
                                " op with an argument that was too large ({})",
                            ),
                            $field,
                        );
                    }
                }
            )*
        }
    };
}

long_ops! {
    constant: constant_index => Constant, ConstantLong;
    define_global: constant_index => DefineGlobal, DefineGlobalLong;
    get_global: constant_index => GetGlobal, GetGlobalLong;
    set_global: constant_index => SetGlobal, SetGlobalLong;
    get_local: local_index => GetLocal, GetLocalLong;
    set_local: local_index => SetLocal, SetLocalLong;
    close_function: function_index => CloseFunction, CloseFunctionLong;
    get_upvalue: upvalue_index => GetUpvalue, GetUpvalueLong;
    set_upvalue: upvalue_index => SetUpvalue, SetUpvalueLong;
    class: class_index => Class, ClassLong;
    get_field: constant_index => GetField, GetFieldLong;
    set_field: constant_index => SetField, SetFieldLong;
    invoke: constant_index, arity: usize => Invoke, InvokeLong;
}
