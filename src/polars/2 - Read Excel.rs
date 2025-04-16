use std::env;
use std::error::Error;
use std::path::Path;

use calamine::open_workbook;
use calamine::Reader;
use calamine::Xlsx;
use polars::prelude::*;

fn read_excel_to_dataframe<P: AsRef<Path>>(path: P) -> Result<DataFrame, Box<dyn Error>>
{
    // Abrir o arquivo Excel
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    // Ler a primeira planilha (ou você pode especificar pelo nome)
    let range = workbook.worksheet_range_at(0).ok_or("Planilha não encontrada")??;

    // Converter para DataFrame do Polars
    let mut df = DataFrame::default();

    // Se houver cabeçalhos (assumimos que a primeira linha contém os cabeçalhos)
    if range.rows().count() > 0
    {
        // Criar um builder para as colunas
        let mut columns: Vec<Series> = Vec::new();

        // Para cada coluna no Excel
        for (i, header) in range.rows().next().unwrap().iter().enumerate()
        {
            let header = header.to_string();
            let mut values: Vec<Option<String>> = Vec::new();

            // Coletar valores da coluna (pulando o cabeçalho)
            for row in range.rows().skip(1)
            {
                if i < row.len()
                {
                    values.push(Some(row[i].to_string()));
                }
                else
                {
                    values.push(None);
                }
            }

            // Criar a série do Polars
            let series = Series::new(header.into(), values);
            columns.push(series);
        }

        df = DataFrame::new(columns.into_iter().map(|s| s.into()).collect())?;
    }

    Ok(df)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>
{
    // Caminho para o arquivo Excel

    let home_path = env::var("HOME")?;
    let file_path =
        Path::new(&home_path).join("code/exclude-from-compact/rust-concepts/src/data/Cripple Detalhado por Chassi.xlsx");

    // Ler o Excel e converter para DataFrame
    let df = read_excel_to_dataframe(file_path)?;

    // Mostrar o DataFrame
    println!("DataFrame lido do Excel:");
    println!("{:?}", df);

    // Operações básicas com Polars
    println!("\nInformações sobre o DataFrame:");
    println!("Número de linhas: {}", df.height());
    println!("Número de colunas: {}", df.width());
    println!("Nomes das colunas: {:?}", df.get_column_names());

    // Filtrar dados (assumindo que temos uma coluna "Idade" do tipo numérico)
    if let Ok(idade) = df.column("Idade")
    {
        if let Ok(idade) = idade.i32()
        {
            let df_filtrado = df.filter(&idade.gt(28))?;
            println!("\nPessoas com mais de 28 anos:");
            println!("{:?}", df_filtrado);
        }
    }

    Ok(())
}
