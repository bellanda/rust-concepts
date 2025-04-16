use ::polars::prelude::*;
use utils::polars_df_to_json::dataframe_to_json;
mod utils;

fn main()
{
    // Criar um DataFrame de exemplo
    let df = df!(
        "name" => &["John", "Jane", "Jim", "Jill"],
        "age" => &[30, 25, 35, 28],
        "city" => &["New York", "Los Angeles", "Chicago", "Houston"]
    )
    .expect("Falha ao criar DataFrame");

    // Mostrar o DataFrame
    println!("DataFrame:");
    println!("{}", df);

    // Converter para JSON e imprimir
    match dataframe_to_json(&df)
    {
        Ok(json_value) =>
        {
            println!("\nJSON:");
            println!("{}", serde_json::to_string_pretty(&json_value).unwrap());
        },
        Err(e) =>
        {
            println!("Erro ao converter para JSON: {}", e);
        },
    }
}
