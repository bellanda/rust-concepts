use ::polars::prelude::*;
use serde_json::json;
use serde_json::Value;

pub fn df_to_json_each_column(df: &DataFrame) -> Result<Value, PolarsError>
{
    let mut json_obj = serde_json::Map::new();

    for col in df.get_columns()
    {
        let col_name = col.name();

        // Convertemos o conteúdo de cada coluna para um formato adequado para JSON
        let values = match col.dtype()
        {
            DataType::Int32 =>
            {
                let s = col.i32()?;
                let vec: Vec<Option<i32>> = s.iter().collect();
                json!(vec)
            },
            DataType::Int64 =>
            {
                let s = col.i64()?;
                let vec: Vec<Option<i64>> = s.iter().collect();
                json!(vec)
            },
            DataType::Float32 =>
            {
                let s = col.f32()?;
                let vec: Vec<Option<f32>> = s.iter().collect();
                json!(vec)
            },
            DataType::Float64 =>
            {
                let s = col.f64()?;
                let vec: Vec<Option<f64>> = s.iter().collect();
                json!(vec)
            },
            DataType::String =>
            {
                let s = col.str()?;
                let vec: Vec<Option<&str>> = s.iter().collect();
                json!(vec)
            },
            DataType::Boolean =>
            {
                let s = col.bool()?;
                let vec: Vec<Option<bool>> = s.iter().collect();
                json!(vec)
            },
            _ =>
            {
                // Para outros tipos, convertemos para string
                let s = col.cast(&DataType::String)?;
                let string_vec: Vec<Option<&str>> = s.str()?.iter().collect();
                json!(string_vec)
            },
        };

        json_obj.insert(col_name.to_string(), values);
    }

    Ok(Value::Object(json_obj))
}

pub fn df_to_json_each_row(df: &mut DataFrame) -> Result<String, PolarsError>
{
    let mut json_obj = Vec::new();

    JsonWriter::new(&mut json_obj).with_json_format(JsonFormat::Json).finish(df)?;

    let json_str = String::from_utf8(json_obj).unwrap();

    Ok(json_str)
}
