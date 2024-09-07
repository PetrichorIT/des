use fxhash::FxHashMap;
use serde::ser::Error;
use serde_yml::{Result, Value};

pub(super) fn yaml_to_par_map(s: &str) -> Result<FxHashMap<String, String>> {
    let map = serde_yml::from_str::<FxHashMap<String, Value>>(s)?;
    let mut result = FxHashMap::default();
    for (k, v) in map {
        fix_value_map(k, v, &mut result)?;
    }
    Ok(result)
}

fn fix_value_map(
    prefix: String,
    value: Value,
    target: &mut FxHashMap<String, String>,
) -> Result<()> {
    match value {
        Value::Mapping(map) => {
            for (k, v) in map {
                let new_prefix = format!("{prefix}.{}", k.as_str().unwrap());
                fix_value_map(new_prefix, v, target)?;
            }
        }
        Value::Bool(b) => {
            target.insert(prefix, b.to_string());
        }
        Value::Number(n) => {
            target.insert(prefix, n.to_string());
        }
        Value::String(s) => {
            target.insert(prefix, s.clone());
        }
        Value::Null => {
            target.insert(prefix, String::new());
        }
        s => return Err(serde_yml::Error::custom(format!("invalid symbol: {s:?}"))),
    };
    Ok(())
}
