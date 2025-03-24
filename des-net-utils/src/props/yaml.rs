use serde_yml::{Mapping, Value};

use super::Props;

/// A collection of configuration parameters, which
/// can be used to assign properties to a component.
#[derive(Debug, Default)]
pub struct Cfg {
    parts: Vec<Value>,
}

impl Cfg {
    /// Adds a configuration parameter to the collection.
    pub fn add(&mut self, cfg: Value) {
        self.parts.push(compartmentalize(cfg));
    }

    /// Generates the preset properties for a component based on the configuration.
    pub fn capture_for(&self, path: &[&str]) -> Props {
        let mut props = Props::default();
        for part in &self.parts {
            extract_from_cfg(part, path, &mut props);
        }
        props
    }
}

const ANY: &str = "<any>";

fn compartmentalize(base: Value) -> Value {
    match base {
        Value::Mapping(mut map) => {
            compartmentalize_map(&mut map);
            Value::Mapping(map)
        }
        other => other,
    }
}

fn compartmentalize_map(map: &mut Mapping) {
    let keys = map
        .keys()
        .map(Value::as_str)
        .flatten()
        .filter(|k| k.contains(ANY))
        .map(str::to_string)
        .collect::<Vec<_>>();

    for key in keys {
        let Some((top, bot)) = key.as_str().split_once(ANY) else {
            continue;
        };
        let value = map.remove(&key).unwrap();

        let entry = if top.is_empty() {
            &mut *map
        } else {
            let top = Value::String(top.trim_end_matches('.').to_string());
            let entry = map.entry(top).or_insert(Value::Mapping(Mapping::new()));
            let Some(entry) = entry.as_mapping_mut() else {
                continue;
            };
            entry
        };

        let subentry = entry
            .entry(Value::String(ANY.to_string()))
            .or_insert(Value::Mapping(Mapping::new()));
        let Some(subentry) = subentry.as_mapping_mut() else {
            continue;
        };

        let bot = Value::String(bot.trim_start_matches('.').to_string());
        subentry.insert(bot, value);

        compartmentalize_map(subentry);
    }
}

fn extract_from_cfg(base: &Value, path: &[&str], props: &mut Props) {
    if path.is_empty() {
        if let Value::Mapping(map) = base {
            for (k, v) in map {
                let Value::String(k) = k else {
                    continue;
                };
                if k.contains(ANY) {
                    continue;
                }
                props.set(k.clone(), v.clone());
            }
        }
    } else {
        if let Value::Mapping(map) = base {
            if let Some(value) = map.get(ANY) {
                extract_from_cfg(value, &path[1..], props);
            }

            let mut key = String::new();
            for i in 0..path.len() {
                if i != 0 {
                    key.push('.');
                }
                key.push_str(path[i]);

                let Some(entry) = map.get(&key[..]) else {
                    continue;
                };
                extract_from_cfg(entry, &path[(i + 1)..], props);
            }

            // extract direct prefixes
            for matching_key in map
                .keys()
                .map(Value::as_str)
                .flatten()
                .filter(|k| k.starts_with(&key) && k.len() > key.len())
            {
                let Some(entry) = map.get(matching_key) else {
                    continue;
                };
                let remaining = &matching_key[(key.len() + 1)..];
                props.set(remaining.to_string(), entry.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_yml::from_str;

    use super::*;

    #[test]
    fn compartmentalized() -> serde_yml::Result<()> {
        const RAW: &str = "\
        lx.alice.tcp.sack: true\n\
        lx.<any>.log: trace\n\
        <any>.router.type: OSPF\n\
        ";

        const COMP: &str = "\
        lx.alice.tcp.sack: true\n\
        lx: { <any>: { log: trace } }\n\
        <any>: { router.type: OSPF }\n\
        ";

        assert_eq!(
            compartmentalize(from_str::<Value>(RAW)?),
            from_str::<Value>(COMP)?
        );
        Ok(())
    }

    #[test]
    fn compartmentalized_multi_any() -> serde_yml::Result<()> {
        const RAW: &str = "\
        lx.alice.tcp.sack: true\n\
        lx.<any>.node.<any>.log: trace\n\
        ";

        const COMP: &str = "\
        lx.alice.tcp.sack: true\n\
        lx: { <any>: { node: { <any>: { log: trace } } } }\n\
        ";

        assert_eq!(
            compartmentalize(from_str::<Value>(RAW)?),
            from_str::<Value>(COMP)?
        );
        Ok(())
    }

    #[test]
    fn compartmentalized_preexisting_mapping() -> serde_yml::Result<()> {
        const RAW: &str = "\
        lx: { alice.tcp.sack: true }\n\
        lx.<any>.node.<any>.log: trace\n\
        ";

        const COMP: &str = "\
        lx: {  alice.tcp.sack: true, <any>: { node: { <any>: { log: trace } } } }\n\
        ";

        assert_eq!(
            compartmentalize(from_str::<Value>(RAW)?),
            from_str::<Value>(COMP)?
        );
        Ok(())
    }

    #[test]
    fn capture_parameter_set() -> serde_yml::Result<()> {
        let mut cfg = Cfg::default();
        cfg.add(from_str::<Value>(
            "\
            alice.addr: 1.1.1.1\n\
            alice.tcp.sack: yes\n\
            alice.tcp.mss: 1500\n\
            alice.mac: 23:34:d2:ad:fd\n\
            bob.addr: 2.2.2.2\n\
            <any>.log: trace\n\
            ",
        )?);

        assert_eq!(
            cfg.capture_for(&["alice"]).keys(),
            ["addr", "tcp.sack", "log", "mac", "tcp.mss"]
        );

        assert_eq!(cfg.capture_for(&["alice", "tcp"]).keys(), ["sack", "mss"]);

        Ok(())
    }

    #[test]
    fn capture_parameter_set_does_not_contain_any() -> serde_yml::Result<()> {
        let mut cfg = Cfg::default();
        cfg.add(from_str::<Value>(
            "\
            alice.addr: 1.1.1.1\n\
            alice.<any>.component: yes\n\
            alice.tcp.mss: 1500\n\
            ",
        )?);

        assert_eq!(cfg.capture_for(&["alice"]).keys(), ["addr", "tcp.mss"]);
        assert_eq!(
            cfg.capture_for(&["alice", "tcp"]).keys(),
            ["component", "mss"]
        );
        assert_eq!(cfg.capture_for(&["alice", "udp"]).keys(), ["component"]);

        Ok(())
    }
}
