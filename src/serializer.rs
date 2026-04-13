use crate::value::JsonValue;

pub fn stringify(value: &JsonValue) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn write_value(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && 
                n.is_finite() && 
                n.abs() < (i64::MAX as f64) {
                    out.push_str(&(*n as i64).to_string());
            } else {
                out.push_str(&n.to_string());
            }
        }
        JsonValue::String(s) => {
            out.push('"');
            write_escaped_string(s, out);
            out.push('"');
        }
        JsonValue::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(v, out);
            }
            out.push(']');
        }
        JsonValue::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 { out.push(',');}
                out.push('"');
                write_escaped_string(k, out);
                out.push_str("\":");
                write_value(v, out);
            }
            out.push('}');
        }
    }
}

fn write_escaped_string(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if c.is_control() => {
                // 其他控制字符用\u00XX表示
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
            c => out.push(c)
        }
    }
}

pub fn stringify_pretty(value: &JsonValue, indent_size: usize) -> String {
    let mut output = String::new();
    write_value_pretty(value, &mut output, 0, indent_size);
    output.push('\n');
    output
}

fn write_value_pretty(value: &JsonValue, out: &mut String, depth: usize, indent_size: usize) {
    match value {
        JsonValue::Array(arr) if arr.is_empty() => out.push_str("[]"),
        JsonValue::Object(map) if map.is_empty() => out.push_str("{}"),
        JsonValue::Array(arr) => {
            out.push_str("[\n");
            for (i, v) in arr.iter().enumerate() {
                write_indent(out, depth+1, indent_size);
                write_value_pretty(v, out, depth +1, indent_size);
                if i < arr.len() - 1 {out.push(',');}
                out.push('\n');
            }
            write_indent(out, depth, indent_size);
            out.push(']');
        }
        JsonValue::Object(map) => {
            out.push_str("{\n");
            let entries: Vec<_> = map.iter().collect();
            for (i, (k, v)) in entries.iter().enumerate() {
                write_indent(out, depth+1, indent_size);
                out.push('"');
                write_escaped_string(k, out);
                out.push_str("\": ");
                write_value_pretty(v, out, depth +1, indent_size);
                if i < entries.len() -1 {out.push(',');}
                out.push('\n');
            }
            write_indent(out, depth, indent_size);
            out.push('}');
        }
        other => write_value(other, out), 
    }
}

fn write_indent(out: &mut String, depth: usize, indent_size: usize) {
    for _ in 0..(depth*indent_size) {
        out.push(' ');
    }
}