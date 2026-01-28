fn main() {
    divan::main();
}

#[divan::bench]
fn serde_json_parse() {
    let json = r#"{"name":"rari","description":"Runtime Accelerated Rendering Infrastructure"}"#;
    let _: serde_json::Value = divan::black_box(serde_json::from_str(json).unwrap());
}

#[divan::bench]
fn serde_json_stringify() {
    let data = serde_json::json!({
        "name": "rari",
        "description": "Runtime Accelerated Rendering Infrastructure",
        "features": ["rsc", "ssr", "app-router"]
    });
    let _json = divan::black_box(serde_json::to_string(&data).unwrap());
}

#[divan::bench(args = [10, 100, 1000])]
fn string_concat(n: usize) {
    let mut result = String::new();
    for i in 0..n {
        result.push_str(&format!("item_{}", i));
    }
    divan::black_box(result);
}

#[divan::bench]
fn regex_matching() {
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    let dates = ["2024-01-01", "2024-12-31", "invalid", "2024-06-15"];
    for date in dates {
        divan::black_box(re.is_match(date));
    }
}
