pub mod cases;
pub mod decision;
pub mod emotion;
pub mod identity;

pub use decision::{evaluate_decision, get_confidence_tier, score_decision};

pub fn py_round(x: f64, digits: u32) -> f64 {
    let s = format!("{:.*}", digits as usize, x);
    s.parse::<f64>().unwrap_or(x)
}

pub fn py_str(x: f64) -> String {
    let s = format!("{}", x);
    if x.fract() == 0.0 && x.is_finite() && !s.contains('.') && !s.contains('e') {
        format!("{}.0", s)
    } else {
        s
    }
}

pub fn py_list_repr(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|i| format!("'{}'", i)).collect();
    format!("[{}]", inner.join(", "))
}

pub fn py_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) > 0x7E => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn py_dumps(v: &serde_json::Value) -> String {
    use serde_json::Value;
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => py_json_string(s),
        Value::Array(a) => {
            let inner: Vec<String> = a.iter().map(py_dumps).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(o) => {
            let inner: Vec<String> = o
                .iter()
                .map(|(k, v)| format!("{}: {}", py_json_string(k), py_dumps(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}
