use std::env;

use dotenv::dotenv;
use log::info;
use oracle::sql_type::ToSql;
use oracle::Connection;
use oracle::Row;
use polars::prelude::*;

/// EngineOracle encapsula a conexão e execução de queries no Oracle,
/// retornando resultados em um DataFrame do Polars.
pub struct EngineOracle
{
    conn: Connection,
}

impl EngineOracle
{
    /// Cria um novo EngineOracle, carregando configurações do ambiente e abrindo a conexão.
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    {
        // Inicializa variáveis de ambiente e logger
        dotenv().ok();
        env_logger::init();

        // Lê credenciais e detalhes de conexão
        let username = env::var("ORACLE_USERNAME")?;
        let password = env::var("ORACLE_PASSWORD")?;
        let host = env::var("ORACLE_HOST")?;
        let port = env::var("ORACLE_PORT")?;
        let service_name = env::var("ORACLE_SERVICE_NAME")?;

        // Caso fornecido, ajusta o caminho do client Oracle
        if let Ok(client_path) = env::var("CLIENT_PATH")
        {
            env::set_var("LD_LIBRARY_PATH", &client_path);
            info!("Oracle Client configurado a partir de: {}", client_path);
        }
        else
        {
            info!("CLIENT_PATH não definido; usando configuração padrão do sistema.");
        }

        // Monta a string de conexão: //host:port/service_name
        let connect_string = format!("//{}:{}/{}", host, port, service_name);
        info!("Conectando ao Oracle em: {}", connect_string);

        // Estabelece a conexão
        let conn = Connection::connect(&username, &password, &connect_string)?;
        info!("Conexão estabelecida com sucesso!");

        Ok(Self { conn })
    }

    /// Executa uma query segura no Oracle e converte o resultado em DataFrame.
    ///
    /// # Parâmetros
    /// - `sql`: instrução SQL com binds posicionais (`:1`, `:2`, ...)
    /// - `params`: slice de parâmetros que implementam `ToSql`
    pub fn query_to_polars_df(
        &self,
        sql: &str,
        params: &[&dyn ToSql],
    ) -> Result<DataFrame, Box<dyn std::error::Error + Send + Sync>>
    {
        // Executa a query e obtém um ResultSet<Row>
        let mut rows = self.conn.query(sql, params)?;

        // Metadata das colunas
        let column_info = rows.column_info();
        let column_count = column_info.len();
        let column_names: Vec<String> = column_info.iter().map(|ci| ci.name().to_string()).collect();

        info!("Colunas encontradas: {:?}", column_names);

        // Preparar armazenamento de dados por coluna
        let mut data: Vec<Vec<Option<String>>> = vec![Vec::new(); column_count];

        // Itera sobre cada linha retornada
        while let Some(row_res) = rows.next()
        {
            let row: Row = row_res?;
            for i in 0..column_count
            {
                let value: Option<String> = row.get(i)?;
                data[i].push(value);
            }
        }

        info!(
            "Consulta executada; {} linhas recuperadas.",
            if data.is_empty() { 0 } else { data[0].len() }
        );

        // Cria Series e DataFrame
        let series_vec: Vec<Series> = column_names
            .iter()
            .enumerate()
            .map(|(i, name)| Series::new(name.into(), &data[i]))
            .collect();

        let df = DataFrame::new(series_vec.into_iter().map(|s| s.into_column()).collect())?;
        Ok(df)
    }
}

// Para usar, adicione no Cargo.toml:
// oracle = { version = "0.6.3", features = ["stmt_without_lifetime"] }
// polars = "0.29"
