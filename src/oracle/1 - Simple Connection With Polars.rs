use dotenv::dotenv;
use log::info;
use oracle::Connection;
use polars::prelude::*;
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Inicializa o log e carrega variáveis de ambiente do .env
    env_logger::init();
    dotenv().ok();

    // Recupera as variáveis de ambiente
    let username =
        env::var("ORACLE_USERNAME").expect("Variável de ambiente ORACLE_USERNAME não encontrada");
    let password =
        env::var("ORACLE_PASSWORD").expect("Variável de ambiente ORACLE_PASSWORD não encontrada");
    let host = env::var("ORACLE_HOST").expect("Variável de ambiente ORACLE_HOST não encontrada");
    let port = env::var("ORACLE_PORT").expect("Variável de ambiente ORACLE_PORT não encontrada");
    let service_name = env::var("ORACLE_SERVICE_NAME")
        .expect("Variável de ambiente ORACLE_SERVICE_NAME não encontrada");
    let client_path = env::var("CLIENT_PATH").ok();

    // Se CLIENT_PATH for fornecido, configuramos o LD_LIBRARY_PATH (para Unix-like)
    if let Some(ref path) = client_path {
        env::set_var("LD_LIBRARY_PATH", path);
        info!("Oracle Client configurado a partir de: {}", path);
    } else {
        info!("CLIENT_PATH não definido; usando a configuração padrão do sistema para o Oracle Client.");
    }

    // Constroi a string de conexão no formato esperado pelo rust-oracle: //host:port/service_name
    let connect_string = format!("//{}:{}/{}", host, port, service_name);
    info!("Conectando ao Oracle com: {}", connect_string);

    // Conecta-se ao Oracle
    let conn = Connection::connect(&username, &password, &connect_string)?;
    info!("Conexão estabelecida com sucesso!");

    // Define a consulta SQL - agora buscando todas as colunas
    let sql = "
        SELECT *
        FROM SYSADM.PS_MMC_CHASSI_LOC
        WHERE ROWNUM <= 30
    ";

    // Executa a consulta
    let mut stmt = conn.query(sql, &[])?;

    // Obtém informações sobre as colunas
    let column_info = stmt.column_info();
    let column_count = column_info.len();

    // Armazena os nomes das colunas
    let column_names: Vec<String> = column_info
        .iter()
        .map(|info| info.name().to_string())
        .collect();

    info!("Colunas encontradas: {:?}", column_names);

    // Armazena os dados como vetores de strings (uma abordagem genérica)
    let mut data: Vec<Vec<Option<String>>> = vec![Vec::new(); column_count];

    // Itera sobre os resultados
    while let Some(row_result) = stmt.next() {
        let row = row_result?;

        // Para cada coluna na linha
        for i in 0..column_count {
            // Tenta obter o valor como string
            let value: Option<String> = row.get(i)?;
            data[i].push(value);
        }
    }

    info!(
        "Consulta executada; {} linhas recuperadas.",
        if data.is_empty() { 0 } else { data[0].len() }
    );

    // Cria as Series do Polars
    let mut series_vec: Vec<Series> = Vec::with_capacity(column_count);

    for (i, col_name) in column_names.iter().enumerate() {
        let series = Series::new(col_name.into(), &data[i]);
        series_vec.push(series);
    }

    // Cria o DataFrame
    let df = DataFrame::new(series_vec.into_iter().map(|s| s.into_column()).collect())?;

    // Exibe o DataFrame
    println!("{}", df.head(Some(10)));

    Ok(())
}
