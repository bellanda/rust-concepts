use polars::prelude::*;
use serde_json;

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    // Create a simple DataFrame
    let df = df! {
        "name" => ["John", "Jane", "Jim", "Jill"],
        "age" => [30, 25, 35, 28],
        "city" => ["New York", "Los Angeles", "Chicago", "Houston"]
    }?;

    println!("Original DataFrame:");
    println!("{}", df);

    // Method 1: Using the serde feature (requires "serde" feature in Cargo.toml)
    println!("\nMethod 1: Using serde_json::to_value:");
    let df_json_value = serde_json::to_value(&df)?;
    println!("{}", serde_json::to_string_pretty(&df_json_value)?);

    // Method 2: Using JsonWriter with JSON format
    println!("\nMethod 2: Using JsonWriter with JSON format:");
    let mut json_string = Vec::new();
    let mut df_clone = df.clone();
    JsonWriter::new(&mut json_string)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df_clone)?;
    println!("{}", String::from_utf8(json_string)?);

    // Method 3: Using JsonWriter with JsonLines format (NDJSON)
    println!("\nMethod 3: Using JsonWriter with JsonLines format (NDJSON):");
    let mut ndjson_string = Vec::new();
    let mut df_clone = df.clone();
    JsonWriter::new(&mut ndjson_string)
        .with_json_format(JsonFormat::JsonLines)
        .finish(&mut df_clone)?;
    println!("{}", String::from_utf8(ndjson_string)?);

    Ok(())
}
