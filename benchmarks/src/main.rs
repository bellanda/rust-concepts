use std::collections::HashMap;
use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use chrono::Local;
use clap::Parser;
use colored::*;
use console::Term;
use futures::stream;
use futures::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use rand::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

#[derive(Parser, Debug)]
#[clap(name = "Rust Benchmark", about = "Benchmark para servidor Xitca Web em Rust")]
struct Args
{
    /// Número de requisições concorrentes
    #[clap(short, long, default_value = "100")]
    concurrency: usize,

    /// Número máximo de requisições por endpoint
    #[clap(short, long, default_value = "10000000")]
    requests: usize,

    /// Duração máxima do teste em segundos
    #[clap(short, long, default_value = "15")]
    duration: u64,

    /// Pular fase de warmup
    #[clap(long)]
    no_warmup: bool,

    /// Desativar conexões HTTP keep-alive
    #[clap(long)]
    no_keepalive: bool,

    /// Forçar teste em todos os endpoints, mesmo os que retornam erro
    #[clap(long)]
    force_all: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestResult
{
    duration_ms: f64,
    status: u16,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LatencyStats
{
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
    p90: f64,
    p99: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkResult
{
    requests: usize,
    successful: usize,
    total_time: f64,
    rps: f64,
    success_rate: f64,
    latency_stats: Option<LatencyStats>,
}

/// URLs para testar
const BASE_URL: &str = "http://localhost:8080";
const ENDPOINTS: &[&str] = &[
    "/",
    // "/user",
    "/users-df-json",
    // "/users"
];

// Diretório para salvar os resultados
const RESULTS_DIR: &str = "results";

// Constantes para melhorar o desempenho
const DEFAULT_REQ_PER_BATCH: usize = 500;

#[tokio::main]
async fn main() -> Result<()>
{
    let args = Args::parse();

    println!(
        "{}\n{}: {}\n{}: {}\n{}: {} {}\n{}: {}\n{}: {}\n{}: {}",
        "Benchmark de API - Xitca Web".green().bold(),
        "Concorrência".cyan(),
        args.concurrency,
        "Requisições máximas por endpoint".cyan(),
        args.requests,
        "Duração máxima por endpoint".cyan(),
        args.duration,
        "segundos".cyan(),
        "Keepalive".cyan(),
        if args.no_keepalive
        {
            "Desativado".red()
        }
        else
        {
            "Ativado".green()
        },
        "Endpoints".cyan(),
        ENDPOINTS.join(", "),
        "Testar todos".cyan(),
        if args.force_all { "Sim".yellow() } else { "Não".green() }
    );

    println!(
        "{}\n{}",
        "IMPORTANTE: Certifique-se de executar o servidor no modo RELEASE para máxima performance:"
            .yellow()
            .bold(),
        "cd src/xitca && cargo run --release --bin \"2 - Json Response\"".white()
    );

    println!("\nPressione Enter quando o servidor estiver pronto...");
    Term::stdout().read_key().context("Erro ao ler teclado")?;

    // Validar se o servidor está rodando e quais endpoints estão disponíveis
    let available_endpoints = validate_endpoints(args.force_all).await?;

    if available_endpoints.is_empty()
    {
        println!(
            "{}",
            "Nenhum endpoint disponível! Verifique se o servidor está rodando."
                .red()
                .bold()
        );
        return Ok(());
    }

    println!("\n{}", "Endpoints disponíveis:".green().bold());
    for (endpoint, status) in &available_endpoints
    {
        println!("  {} - Status: {}", endpoint, status);
    }

    // Realizar warmup para JIT e caches
    if !args.no_warmup
    {
        warmup(&available_endpoints.keys().map(|k| k.as_str()).collect::<Vec<_>>()).await?;
    }

    let mut all_results = HashMap::new();

    // Configuração do cliente HTTP
    let client = build_client(&args)?;

    for (endpoint, _) in &available_endpoints
    {
        let url = format!("{}{}", BASE_URL, endpoint);
        println!("\n{} {}", "Testando endpoint:".cyan().bold(), url.white());

        let result = run_benchmark(&client, &url, &args).await?;

        all_results.insert(endpoint.clone(), result);

        // Espera breve entre os testes
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n{}\n", "Benchmark concluído!".green().bold());
    print_results(&all_results)?;

    Ok(())
}

/// Valida quais endpoints estão disponíveis antes de iniciar o benchmark
async fn validate_endpoints(force_all: bool) -> Result<HashMap<String, u16>>
{
    println!("{}", "Validando endpoints disponíveis...".blue());
    println!("{}", "Por padrão, apenas endpoints com status 2xx/3xx serão testados.".blue());
    if force_all
    {
        println!(
            "{}",
            "Modo força: Todos os endpoints serão testados mesmo com erros.".yellow()
        );
    }

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    let mut available = HashMap::new();

    for &endpoint in ENDPOINTS
    {
        let url = format!("{}{}", BASE_URL, endpoint);
        match client.get(&url).send().await
        {
            Ok(response) =>
            {
                let status = response.status().as_u16();
                let status_ok = status < 400; // Considerar apenas códigos 2xx e 3xx como válidos
                println!(
                    "  {}: {} - {}",
                    endpoint.green(),
                    status,
                    if status_ok { "OK".green() } else { "Falha".red() }
                );

                // Incluir endpoint se estiver funcionando ou se force_all estiver ativado
                if status_ok || force_all
                {
                    available.insert(endpoint.to_string(), status);
                    if !status_ok && force_all
                    {
                        println!(
                            "    ↳ {} {}",
                            "Incluindo mesmo com erro:".yellow(),
                            "Modo força ativado".yellow()
                        );
                    }
                }
                else
                {
                    println!("    ↳ {} {}", "Pulando:".yellow(), "Endpoint respondeu com erro".yellow());
                }
            },
            Err(e) =>
            {
                println!("  {}: {} - {}", endpoint.red(), "Erro", e);
                // Incluir apenas se force_all estiver ativado
                if force_all
                {
                    println!(
                        "    ↳ {} {}",
                        "Incluindo mesmo com erro:".yellow(),
                        "Modo força ativado".yellow()
                    );
                    available.insert(endpoint.to_string(), 0);
                }
            },
        }
    }

    if available.is_empty()
    {
        println!("{}", "Aviso: Nenhum endpoint funcional encontrado!".red().bold());
        println!(
            "{}",
            "Execute com --force-all para testar todos os endpoints mesmo com erros.".yellow()
        );
    }

    Ok(available)
}

fn build_client(args: &Args) -> Result<Client>
{
    let mut client_builder = Client::builder()
        .user_agent("rust-benchmark/0.1.0")
        .pool_max_idle_per_host(args.concurrency)
        .pool_max_idle_per_host(args.concurrency * 4)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(60));

    if args.no_keepalive
    {
        client_builder = client_builder.pool_max_idle_per_host(0);
    }
    else
    {
        client_builder = client_builder
            .pool_idle_timeout(Duration::from_secs(300))
            .http2_keep_alive_timeout(Duration::from_secs(300));
    }

    Ok(client_builder.build()?)
}

async fn warmup(endpoints: &[&str]) -> Result<()>
{
    println!("{}", "Realizando warmup do servidor...".blue().bold());

    // Usar um cliente otimizado para o warmup
    let client = Client::builder()
        .user_agent("rust-benchmark/0.1.0")
        .pool_max_idle_per_host(500)
        .tcp_nodelay(true)
        .pool_idle_timeout(Duration::from_secs(30))
        .build()?;

    // Fazer mais requests de warmup para cada endpoint
    for &endpoint in endpoints
    {
        let url = format!("{}{}", BASE_URL, endpoint);
        println!("Warming up: {}", url);

        // Executar em várias ondas para melhor aquecimento
        for wave in 0..3
        {
            println!("  Onda de warmup {}/3...", wave + 1);

            // Aumentar gradualmente o número de requisições
            let num_requests = 100 * (wave + 1);

            let futures = (0..num_requests).map(|_| {
                let client = client.clone();
                let url = url.clone();
                async move {
                    match client.get(&url).header("Connection", "keep-alive").send().await
                    {
                        Ok(response) =>
                        {
                            // Consumir o corpo para liberar a conexão
                            let _ = response.bytes().await;
                        },
                        Err(_) =>
                        {},
                    }
                }
            });

            // Usar uma concorrência menor para não sobrecarregar durante o warmup
            let concurrency = std::cmp::min(num_requests, 100);

            // Executar as requisições de warmup
            stream::iter(futures).buffer_unordered(concurrency).collect::<Vec<_>>().await;

            // Pequena pausa entre as ondas
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    println!("{}\n", "Warmup concluído.".blue().bold());

    // Pausa para permitir que o servidor se estabilize
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}

async fn run_benchmark(client: &Client, url: &str, args: &Args) -> Result<BenchmarkResult>
{
    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(args.duration);

    // Pré-alocar com um tamanho razoável para evitar muitas realocações
    let expected_capacity = std::cmp::min(args.requests, 1_000_000);
    let mut results = Vec::with_capacity(expected_capacity);

    // Ajustar o estilo da barra de progresso
    let pb = ProgressBar::new(args.requests as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} ({elapsed_precise}) {percent}% ({eta}) [RPS: {msg}]")
            .unwrap()
            .progress_chars("►■□"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    let mut current_requests = 0;

    // Lotes menores no início para ajustar progressivamente
    let mut batch_size = std::cmp::min(DEFAULT_REQ_PER_BATCH, args.concurrency);

    // Para calcular a taxa real de requisições por segundo
    let mut last_report_time = start_time;
    let mut last_report_count = 0;
    let mut current_rps = 0.0;

    while Instant::now() < end_time && current_requests < args.requests
    {
        // Aumentar gradualmente o tamanho do lote para encontrar o ponto ótimo
        if current_requests > 1000
        {
            batch_size = std::cmp::min(args.concurrency, args.requests - current_requests);
        }

        let batch_results = run_batch(client, url, batch_size).await;
        let batch_count = batch_results.len();

        if batch_count == 0
        {
            // Se não recebemos resultados, algo está errado
            println!("Aviso: Lote retornou 0 resultados. Servidor pode estar sobrecarregado.");
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        results.extend(batch_results);
        current_requests += batch_count;

        // Atualizar progresso
        let position = std::cmp::min(current_requests, args.requests) as u64;
        pb.set_position(position);

        // Calcular RPS em intervalos regulares (a cada 1 segundo) para evitar o bug
        let now = Instant::now();
        let elapsed_since_last_report = now.duration_since(last_report_time).as_secs_f64();

        if elapsed_since_last_report >= 1.0
        {
            let requests_since_last_report = current_requests - last_report_count;
            current_rps = requests_since_last_report as f64 / elapsed_since_last_report;

            // Atualizar mensagem com RPS atual mais preciso
            pb.set_message(format!("{:.2}", current_rps));

            // Resetar contadores para próximo intervalo
            last_report_time = now;
            last_report_count = current_requests;
        }

        // Pequena pausa para permitir que o progresso seja atualizado
        if current_requests % 10000 == 0
        {
            tokio::task::yield_now().await;
        }
    }

    pb.finish_with_message(format!("Completo: {} requests, {:.2} req/s", current_requests, current_rps));

    let total_time = start_time.elapsed().as_secs_f64();

    if results.is_empty()
    {
        return Ok(BenchmarkResult {
            requests: 0,
            successful: 0,
            total_time,
            rps: 0.0,
            success_rate: 0.0,
            latency_stats: None,
        });
    }

    let successful = results.iter().filter(|r| r.success).count();

    // Para grandes volumes, usar amostragem para calcular as estatísticas de latência
    let latencies: Vec<f64> = if results.len() > 10000
    {
        // Usar amostragem aleatória para grandes volumes
        let mut rng = rand::thread_rng();
        results.choose_multiple(&mut rng, 10000).map(|r| r.duration_ms).collect()
    }
    else
    {
        results.iter().map(|r| r.duration_ms).collect()
    };

    let latency_stats = if !latencies.is_empty()
    {
        Some(LatencyStats {
            min: latencies.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            max: latencies.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            mean: latencies.iter().sum::<f64>() / latencies.len() as f64,
            median: percentile(&latencies, 50.0),
            p90: percentile(&latencies, 90.0),
            p99: percentile(&latencies, 99.0),
        })
    }
    else
    {
        None
    };

    // Calcular o RPS real baseado no número de requisições completas e tempo total
    let actual_rps = results.len() as f64 / total_time;

    Ok(BenchmarkResult {
        requests: results.len(),
        successful,
        total_time,
        rps: actual_rps, // Usar valor calculado real aqui, não o da barra de progresso
        success_rate: if results.is_empty()
        {
            0.0
        }
        else
        {
            successful as f64 / results.len() as f64
        },
        latency_stats,
    })
}

async fn run_batch(client: &Client, url: &str, size: usize) -> Vec<RequestResult>
{
    // Criar um vetor com capacidade pré-alocada para evitar realocações
    let mut results = Vec::with_capacity(size);

    // Criar lotes menores para melhor controle de recursos
    const MAX_BATCH_SIZE: usize = 1000;
    let batch_chunks = (size + MAX_BATCH_SIZE - 1) / MAX_BATCH_SIZE; // Arredondamento para cima

    for _ in 0..batch_chunks
    {
        let batch_size = std::cmp::min(MAX_BATCH_SIZE, size - results.len());
        if batch_size == 0
        {
            break;
        }

        let futures = (0..batch_size).map(|_| {
            let client = client.clone();
            let url = url.to_string();
            async move {
                let start = Instant::now();
                match client.get(&url).header("Connection", "keep-alive").send().await
                {
                    Ok(response) =>
                    {
                        let status = response.status();
                        let _ = response.bytes().await; // consumir o corpo
                        let duration = start.elapsed().as_secs_f64() * 1000.0;

                        RequestResult {
                            duration_ms: duration,
                            status: status.as_u16(),
                            success: status.is_success(),
                            error: None,
                        }
                    },
                    Err(e) =>
                    {
                        let duration = start.elapsed().as_secs_f64() * 1000.0;
                        RequestResult {
                            duration_ms: duration,
                            status: 0,
                            success: false,
                            error: Some(e.to_string()),
                        }
                    },
                }
            }
        });

        // Usar buffer_unordered para limitar a concorrência dentro do lote
        let batch_results: Vec<RequestResult> = stream::iter(futures).buffer_unordered(batch_size).collect().await;

        results.extend(batch_results);
    }

    results
}

fn percentile(data: &[f64], percentile: f64) -> f64
{
    if data.is_empty()
    {
        return 0.0;
    }

    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let index = (sorted_data.len() as f64 - 1.0) * (percentile / 100.0);
    let floor = index.floor() as usize;
    let ceil = index.ceil() as usize;

    if floor == ceil
    {
        return sorted_data[floor];
    }

    let weight = index - floor as f64;
    sorted_data[floor] * (1.0 - weight) + sorted_data[ceil] * weight
}

fn print_results(all_results: &HashMap<String, BenchmarkResult>) -> Result<()>
{
    // Imprimir tabela de resultados
    println!("{:=^80}", " Resultados do Benchmark ");
    println!(
        "{:<15} | {:>10} | {:>15} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8}",
        "Endpoint", "Requests", "RPS", "Sucesso", "Média", "Mediana", "p90", "p99"
    );
    println!("{:-<80}", "");

    for (endpoint, result) in all_results
    {
        if let Some(latency) = &result.latency_stats
        {
            println!(
                "{:<15} | {:>10} | {:>15.2} | {:>7.1}% | {:>7.2}ms | {:>7.2}ms | {:>7.2}ms | {:>7.2}ms",
                endpoint,
                result.requests,
                result.rps,
                result.success_rate * 100.0,
                latency.mean,
                latency.median,
                latency.p90,
                latency.p99
            );
        }
        else
        {
            println!(
                "{:<15} | {:>10} | {:>15.2} | {:>7.1}% | {:>8} | {:>8} | {:>8} | {:>8}",
                endpoint,
                result.requests,
                result.rps,
                result.success_rate * 100.0,
                "N/A",
                "N/A",
                "N/A",
                "N/A"
            );
        }
    }

    // Salvar resultados em arquivo
    let results_dir = Path::new(RESULTS_DIR);
    if !results_dir.exists()
    {
        fs::create_dir_all(results_dir)?;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}/benchmark_{}.txt", RESULTS_DIR, timestamp);
    let mut file = File::create(&filename)?;

    writeln!(file, "Resultados do Benchmark - {}", Local::now())?;
    writeln!(file, "{}", "=".repeat(50))?;
    writeln!(file)?;

    for (endpoint, result) in all_results
    {
        writeln!(file, "Endpoint: {}", endpoint)?;
        writeln!(file, "Requests: {}", result.requests)?;
        writeln!(file, "Requests por segundo: {:.2}", result.rps)?;
        writeln!(file, "Taxa de sucesso: {:.1}%", result.success_rate * 100.0)?;

        if let Some(latency) = &result.latency_stats
        {
            writeln!(file, "Latência mínima: {:.2} ms", latency.min)?;
            writeln!(file, "Latência média: {:.2} ms", latency.mean)?;
            writeln!(file, "Latência mediana: {:.2} ms", latency.median)?;
            writeln!(file, "Latência p90: {:.2} ms", latency.p90)?;
            writeln!(file, "Latência p99: {:.2} ms", latency.p99)?;
            writeln!(file, "Latência máxima: {:.2} ms", latency.max)?;
        }

        writeln!(file, "\n{}\n", "-".repeat(50))?;
    }

    println!("\nResultados salvos em {}", filename.green());

    Ok(())
}
