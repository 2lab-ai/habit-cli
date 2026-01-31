use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

fn stable_clone(v: &Value) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => Value::Number(n.clone()),
        Value::String(s) => Value::String(s.clone()),
        Value::Array(arr) => Value::Array(arr.iter().map(stable_clone).collect()),
        Value::Object(map) => {
            let mut out: BTreeMap<String, Value> = BTreeMap::new();
            for (k, vv) in map.iter() {
                out.insert(k.clone(), stable_clone(vv));
            }
            let mut m = serde_json::Map::new();
            for (k, vv) in out {
                m.insert(k, vv);
            }
            Value::Object(m)
        }
    }
}

pub fn stable_to_string_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let v = serde_json::to_value(value)?;
    let stable = stable_clone(&v);
    serde_json::to_string_pretty(&stable)
}
