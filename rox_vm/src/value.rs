use core::fmt;

use crate::{Closure, Handle, RuntimeError, String};

#[derive(Debug, PartialEq)]
pub enum Constant {
    // TODO: can shrink constant size by
    // - switching String(String) to String(Box<str)
    // - and NaN-boxing other values
    Float(f64),
    String(std::string::String),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{value}"),
            Self::String(value) => write!(f, "{value}"),
        }
    }
}

macro_rules! define_native_functions {
    ($($fn:ident($arity:expr): $name:expr),* $(,)?) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum NativeFunction {
            $($fn,)*
        }

        impl NativeFunction {
            pub fn arity(&self) -> usize {
                match self {
                    $(Self::$fn => $arity,)*
                }
            }

            pub fn name(&self) -> &'static str {
                match self {
                    $(Self::$fn => $name,)*
                }
            }
        }

        #[derive(Debug)]
        pub struct InvalidNativeFunction {
            pub index: usize,
        }

        impl TryFrom<usize> for NativeFunction {
            type Error = InvalidNativeFunction;

            #[allow(non_upper_case_globals)]
            fn try_from(value: usize) -> Result<Self, Self::Error> {
                $(const $fn: usize = NativeFunction::$fn as usize;)*
                match value {
                    $($fn => Ok(Self::$fn),)*
                    index => Err(InvalidNativeFunction { index }),
                }
            }
        }
    };
}

define_native_functions! {
    Clock(0): "clock",
}

const NAN_BITS: u64 = 0x7f_fc_00_00_00_00_00_00;
const TAG_BIT0: u64 = 0x80_00_00_00_00_00_00_00;
const TAG_BIT1: u64 = 0x00_02_00_00_00_00_00_00;
const TAG_BIT2: u64 = 0x00_01_00_00_00_00_00_00;
const TAG_BITS: u64 = TAG_BIT0 | TAG_BIT1 | TAG_BIT2;
const DATA_BITS: u64 = 0x00_00_ff_ff_ff_ff_ff_ff;

const TAG_ENUMERATED: u64 = 0;
const TAG_STRING: u64 = TAG_BIT1;
const TAG_CLOSURE: u64 = TAG_BIT0 | TAG_BIT1;
const _TAG_UNUSED0: u64 = TAG_BIT2;
const _TAG_UNUSED1: u64 = TAG_BIT0 | TAG_BIT2;
const _TAG_UNUSED2: u64 = TAG_BIT1 | TAG_BIT2;
const _TAG_UNUSED3: u64 = TAG_BIT0 | TAG_BIT1 | TAG_BIT2;

const ENUM_INTERNAL: u64 = 0;
const ENUM_NIL: u64 = 1;
const ENUM_FALSE: u64 = 2;
const ENUM_TRUE: u64 = 3;
const ENUM_NATIVE_FUNCTION0: u64 = 4;

const INTERNAL_BITS: u64 = NAN_BITS | TAG_ENUMERATED | ENUM_INTERNAL;
const NIL_BITS: u64 = NAN_BITS | TAG_ENUMERATED | ENUM_NIL;
const FALSE_BITS: u64 = NAN_BITS | TAG_ENUMERATED | ENUM_FALSE;
const TRUE_BITS: u64 = NAN_BITS | TAG_ENUMERATED | ENUM_TRUE;
const NATIVE_FUNCTION0_BITS: u64 =
    NAN_BITS | TAG_ENUMERATED | ENUM_NATIVE_FUNCTION0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    bits: u64,
}

impl Value {
    pub const fn register(value: u64) -> Self {
        Self { bits: value }
    }

    pub const fn float(value: f64) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    pub const fn internal() -> Self {
        Self {
            bits: INTERNAL_BITS,
        }
    }

    pub const fn nil() -> Self {
        Self { bits: NIL_BITS }
    }

    pub const fn boolean(value: bool) -> Self {
        Self {
            bits: if value { TRUE_BITS } else { FALSE_BITS },
        }
    }

    pub const fn native_function(value: usize) -> Self {
        Self {
            bits: NATIVE_FUNCTION0_BITS + value as u64,
        }
    }

    pub fn string(handle: Handle<String>) -> Self {
        Self {
            bits: NAN_BITS | TAG_STRING | (handle.address() as u64 & DATA_BITS),
        }
    }

    pub fn closure(handle: Handle<Closure>) -> Self {
        Self {
            bits: NAN_BITS
                | TAG_CLOSURE
                | (handle.address() as u64 & DATA_BITS),
        }
    }

    pub fn truthiness(&self) -> bool {
        self.bits != NIL_BITS && self.bits != FALSE_BITS
    }

    pub fn as_register(self) -> u64 {
        self.bits
    }

    pub fn as_float(self) -> f64 {
        f64::from_bits(self.bits)
    }

    pub fn unpack(&self) -> Result<UnpackedValue, RuntimeError> {
        if self.bits & NAN_BITS != NAN_BITS {
            return Ok(UnpackedValue::Float(f64::from_bits(self.bits)));
        }

        let tag = self.bits & TAG_BITS;
        let data = self.bits & DATA_BITS;
        Ok(match tag {
            TAG_ENUMERATED => match data {
                ENUM_INTERNAL => return Err(RuntimeError::InternalValue),
                ENUM_NIL => UnpackedValue::Nil,
                ENUM_FALSE => UnpackedValue::False,
                ENUM_TRUE => UnpackedValue::True,
                f => UnpackedValue::NativeFunction(NativeFunction::try_from(
                    (f - ENUM_NATIVE_FUNCTION0) as usize,
                )?),
            },
            TAG_STRING => UnpackedValue::String(unsafe {
                Handle::from_address(data as usize)
            }),
            TAG_CLOSURE => UnpackedValue::Closure(unsafe {
                Handle::from_address(data as usize)
            }),
            _ => unreachable!(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnpackedValue {
    Float(f64),
    Nil,
    False,
    True,
    NativeFunction(NativeFunction),
    String(Handle<String>),
    Closure(Handle<Closure>),
}
