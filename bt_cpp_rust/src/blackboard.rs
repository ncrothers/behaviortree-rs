use std::{collections::HashMap, any::Any};

use crate::basic_types::{StringInto, BTToString};

pub trait BlackboardString {
    fn strip_bb_pointer(&self) -> Option<String>;
}

impl<'a, T> BlackboardString for T
where T: AsRef<str> + Clone
{
    fn strip_bb_pointer(&self) -> Option<String> {
        let str_ref = self.as_ref();

        // Is bb pointer
        if str_ref.starts_with("{") && str_ref.ends_with("}") {
            Some(str_ref.strip_prefix("{").unwrap().strip_suffix("}").unwrap().to_string())
        }
        else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Blackboard {
    map: HashMap<String, Box<dyn Any>>,
}

impl Blackboard {
    pub fn new() -> Blackboard {
        Self {
            map: HashMap::new(),
        }
    }
    
    pub fn read<T>(&self, key: impl AsRef<str>) -> Option<T>
    where 
        T: Any + Clone,
        String: StringInto<T>,
    {
        let key = key.as_ref().to_string();

        // Try to get the key
        if let Some(value) = self.map.get(&key) {
            // Try to downcast directly to T
            if let Some(value) = value.downcast_ref::<T>() {
                return Some(value.clone());
            }
            else {
                // If value is a String or &str, try to call `StringInto` to convert to T
                if let Some(value) = value.downcast_ref::<String>() {
                    if let Ok(value) = value.string_into() {
                        return Some(value);
                    }
                }
                else if let Some(value) = value.downcast_ref::<&str>() {
                    let value = value.to_string();
                    if let Ok(value) = value.string_into() {
                        return Some(value);
                    }
                }
            }
        }

        // No matches
        None
    }

    pub fn write<'a, T: Any + BTToString + 'static>(&mut self, key: impl AsRef<str>, value: T) -> Option<T> {
        let prev = self.map.insert(key.as_ref().to_string(), Box::new(value));

        match prev {
            Some(prev) => {
                match prev.downcast::<T>() {
                    Ok(prev) => Some(*prev),
                    Err(_) => None,
                }
            }
            None => None,
        }
    }
}