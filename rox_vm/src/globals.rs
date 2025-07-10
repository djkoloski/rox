use std::collections::HashMap;

use crate::{NativeFunction, Value};

macro_rules! define_globals {
    ($($name:ident: $value:expr),* $(,)?) => {
        pub fn global_names<'ast>() -> impl Iterator<Item = &'ast str> {
            [$(::core::stringify!($name),)*].into_iter()
        }

        pub fn global_values() -> HashMap<String, Value> {
            let mut result = HashMap::new();

            $(
                result.insert(::core::stringify!($name).to_string(), $value);
            )*

            result
        }
    };
}

define_globals! {
    clock: Value::NativeFunction(NativeFunction::Clock),
}
