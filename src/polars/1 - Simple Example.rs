use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Cria as Series do Polars
    let mut series_vec: Vec<Series> = Vec::with_capacity(5);

    for (_, col_name) in [
        "ID".to_string(),
        "CHASSI".to_string(),
        "LOCATION".to_string(),
    ]
    .iter()
    .enumerate()
    {
        let data = [1, 2, 3, 4, 5];
        let series = Series::new(col_name.into(), data);
        series_vec.push(series);
    }

    // Cria o DataFrame
    let df = DataFrame::new(series_vec.into_iter().map(|s| s.into_column()).collect())?;

    // Exibe o DataFrame
    println!("{}", df.head(Some(10)));

    Ok(())
}
