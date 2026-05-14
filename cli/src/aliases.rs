use itertools::Itertools;
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map},
};

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct Aliases(HashMap<String, String>);

impl Aliases {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains_key(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.0.get(k)
    }

    pub fn iter(&self) -> hash_map::Iter<'_, String, String> {
        self.0.iter()
    }

    pub fn insert(&mut self, k: String, v: String) -> Option<String> {
        self.0.insert(k, v)
    }

    pub fn insert_if_missing(&mut self, alias: &str, command: &str) {
        if !self.contains_key(alias) {
            self.insert(alias.into(), command.into());
        }
    }

    pub fn update(&mut self, alias_update: &str) {
        if alias_update.is_empty() {
            return;
        }
        if let Some((alias, command)) = alias_update.split_once('=') {
            if !command.is_empty() {
                self.insert(alias.into(), command.into());
            } else {
                self.remove(alias);
            }
        } else {
            self.remove(alias_update);
        }
    }

    pub fn remove(&mut self, k: &str) -> Option<String> {
        self.0.remove(k)
    }

    pub fn reverse_map(&self) -> HashMap<&str, Vec<&str>> {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for (alias, value) in self.iter() {
            map.entry(value.as_str()).or_default().push(alias.as_str());
        }
        map
    }

    pub fn expand<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if let Some(alias) = self.get(text) {
            return Cow::Owned(alias.clone());
        }
        if let Some(pos) = text.find([' ', ':'])
            && let Some(alias) = self.get(&text[..pos])
        {
            return Cow::Owned([alias, &text[pos..]].concat());
        }
        Cow::Borrowed(text)
    }

    pub fn print(&self) {
        for (alias, to) in self.iter().sorted() {
            println!("{alias}={to}");
        }
    }
}

impl Extend<(String, String)> for Aliases {
    fn extend<T: IntoIterator<Item = (String, String)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl IntoIterator for Aliases {
    type Item = (String, String);
    type IntoIter = hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_add_remove() {
        let mut aliases = Aliases::default();
        assert_eq!(aliases.len(), 0);
        aliases.update("a=b");
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("a").unwrap(), "b");

        aliases.update("c=d");
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases.get("c").unwrap(), "d");

        // No value removes the alias.
        aliases.update("c");
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("a").unwrap(), "b");

        // Removing non-existent alias is allowed.
        aliases.update("z");
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("a").unwrap(), "b");

        // Update existing alias.
        aliases.update("a=x");
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("a").unwrap(), "x");

        // Empty value also removes the alias.
        aliases.update("a=");
        assert_eq!(aliases.len(), 0);
    }

    // Empty string is allowed as a no-op.
    #[test]
    fn update_empty_str() {
        let mut aliases = Aliases::default();
        assert_eq!(aliases.len(), 0);
        aliases.update("");
        assert_eq!(aliases.len(), 0);
    }

    // The alias can contains the `=` character.
    #[test]
    fn update_eq_in_value() {
        let mut aliases = Aliases::default();
        assert_eq!(aliases.len(), 0);
        aliases.update("a=b=c");
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("a").unwrap(), "b=c");
    }

    #[test]
    fn deserialize_from_hashmap_json() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());
        let json = serde_json::to_string(&map).unwrap();
        let aliases: Aliases = serde_json::from_str(&json).unwrap();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases.get("key1").unwrap(), "value1");
        assert_eq!(aliases.get("key2").unwrap(), "value2");
    }

    #[test]
    fn expand() {
        let mut aliases = Aliases::default();
        aliases.insert("a".into(), "x".into());
        assert_eq!(aliases.expand("a"), "x");
        assert_eq!(aliases.expand("z"), "z");

        // Expands the first word.
        assert_eq!(aliases.expand("a b"), "x b");
        assert_eq!(aliases.expand("a:b"), "x:b");

        // The rests are retained, even if they are aliases.
        assert_eq!(aliases.expand("a a"), "x a");
        assert_eq!(aliases.expand("a:a"), "x:a");

        // Even if they have separators.
        assert_eq!(aliases.expand("a  b"), "x  b");
        assert_eq!(aliases.expand("a :b"), "x :b");
        assert_eq!(aliases.expand("a: b"), "x: b");
        assert_eq!(aliases.expand("a::b"), "x::b");
    }
}
