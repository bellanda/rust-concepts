use std::sync::Arc;

use oracle::connection::EngineOracle;
use polars::prelude::*;
mod oracle;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    // 1) .env + logger
    dotenv::dotenv().ok();

    // 2) Instancia o EngineOracle e embala num Arc
    let engine = Arc::new(EngineOracle::new().expect("falha ao conectar no Oracle"));

    let sql = r#"
        SELECT *
        FROM SYSADM.PS_MMC_CHASSI_LOC
        WHERE ROWNUM <= :1
    "#;

    let mut df: DataFrame = engine.query_to_polars_df(sql, &[&1000000]).unwrap();

    let path = std::env::current_dir().unwrap();
    let mut file = std::fs::File::create(path.join("src/data/test.csv")).unwrap();
    CsvWriter::new(&mut file).finish(&mut df).unwrap();

    println!("{}", df);
    Ok(())
}
