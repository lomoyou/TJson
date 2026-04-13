use tjson::{parse, stringify, stringify_pretty, JsonValue};

fn main() {
    // ===== 解码 =====
    let input = 
    r#"{
            "name": "Rust JSON",
            "version": 1.0,
            "features": ["parse", "stringify", "pretty print"],
            "stable": true,
            "metadata": null
        }"#
    ;

    let value = parse(input).expect("parse failed");

    // 用 Index trait 访问
    println!("name: {}", value["name"]);           // "Rust JSON"
    println!("first feature: {}", value["features"][0]); // "parse"
    println!("stable: {}", value["stable"]);        // true
    println!("missing: {}", value["nonexist"]);     // null

    // 用类型方法访问
    if let Some(name) = value["name"].as_str() {
        println!("name is: {}", name);
    }

    // ===== 编码 =====
    let compact = stringify(&value);
    println!("compact: {}", compact);

    let pretty = stringify_pretty(&value, 2);
    println!("pretty:\n{}", pretty);

    // ===== 手动构建 =====
    use std::collections::BTreeMap;

    let mut obj = BTreeMap::new();
    obj.insert("x".to_string(), JsonValue::Number(1.0));
    obj.insert("y".to_string(), JsonValue::Number(2.0));
    let point = JsonValue::Object(obj);

    println!("point: {}", stringify(&point)); // {"x":1,"y":2}
}