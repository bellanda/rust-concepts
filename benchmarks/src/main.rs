use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use chrono::Local;
use colored::*;
use console::Term;
use dotenv::dotenv;
use futures::stream;
use futures::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use rand::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

/// Configurações do benchmark, carregadas via `env::var`.
#[derive(Debug)]
struct Config
{
    concurrency: usize,
    requests: usize,
    duration: u64,
    no_warmup: bool,
    no_keepalive: bool,
    force_all: bool,
    base_url: String,
    endpoints: Vec<String>,
    results_dir: String,
    default_req_per_batch: usize,
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

#[tokio::main]
async fn main() -> Result<()>
{
    // Carrega .env, se existir
    dotenv().ok();

    // …e então lê cada variável, com default quando aplicável:
    let concurrency = env::var("CONCURRENCY")
        .unwrap_or_else(|_| "100".into())
        .parse::<usize>()
        .expect("CONCURRENCY deve ser um número");
    let requests = env::var("REQUESTS")
        .unwrap_or_else(|_| "10000000".into())
        .parse::<usize>()
        .expect("REQUESTS deve ser um número");
    let duration = env::var("DURATION")
        .unwrap_or_else(|_| "15".into())
        .parse::<u64>()
        .expect("DURATION deve ser um número");
    let no_warmup = env::var("NO_WARMUP")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()
        .expect("NO_WARMUP deve ser true ou false");
    let no_keepalive = env::var("NO_KEEPALIVE")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()
        .expect("NO_KEEPALIVE deve ser true ou false");
    let force_all = env::var("FORCE_ALL")
        .unwrap_or_else(|_| "false".into())
        .parse::<bool>()
        .expect("FORCE_ALL deve ser true ou false");
    let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let endpoints: Vec<String> = env::var("ENDPOINTS")
        .unwrap_or_else(|_| "/users-df".into())
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .collect();
    let results_dir = env::var("RESULTS_DIR").unwrap_or_else(|_| "results".into());
    let default_req_per_batch = env::var("DEFAULT_REQ_PER_BATCH")
        .unwrap_or_else(|_| "500".into())
        .parse::<usize>()
        .expect("DEFAULT_REQ_PER_BATCH deve ser um número");

    let config = Config {
        concurrency,
        requests,
        duration,
        no_warmup,
        no_keepalive,
        force_all,
        base_url,
        endpoints,
        results_dir,
        default_req_per_batch,
    };

    // Exibe configurações iniciais
    println!("{}", "Benchmark de API - Xitca Web".green().bold());
    println!("Concorrência: {}", config.concurrency);
    println!("Requisições máximas por endpoint: {}", config.requests);
    println!("Duração máxima por endpoint: {} segundos", config.duration);
    println!(
        "Keepalive: {}",
        if config.no_keepalive
        {
            "Desativado".red()
        }
        else
        {
            "Ativado".green()
        }
    );
    println!("Endpoints: {:?}", config.endpoints);
    println!(
        "Forçar todos: {}",
        if config.force_all { "Sim".yellow() } else { "Não".green() }
    );
    println!("Resultados serão salvos em: {}", config.results_dir);
    println!(
        "\n{}",
        "IMPORTANTE: Execute o servidor no modo RELEASE:\ncd src/xitca && cargo run --release --bin '2 - Json Response'"
            .yellow()
            .bold()
    );
    Term::stdout().read_key().context("Erro ao ler teclado")?;

    // Valida endpoints
    let available_endpoints = validate_endpoints(&config).await?;
    if available_endpoints.is_empty()
    {
        println!("{}", "Nenhum endpoint disponível!".red().bold());
        return Ok(());
    }

    // Warmup opcional
    if !config.no_warmup
    {
        warmup(&config).await?;
    }

    // Configura cliente HTTP
    let client = build_client(&config)?;
    let mut all_results = HashMap::new();

    // Executa benchmarks por endpoint
    for endpoint in &config.endpoints
    {
        let url = format!("{}{}", &config.base_url, endpoint);
        println!("\n{} {}", "Testando endpoint:".cyan().bold(), url);
        let result = run_benchmark(&client, &url, &config).await?;
        all_results.insert(endpoint.clone(), result);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n{}\n", "Benchmark concluído!".green().bold());
    print_results(&all_results, &config)?;

    Ok(())
}

async fn validate_endpoints(config: &Config) -> Result<HashMap<String, u16>>
{
    println!("{}", "Validando endpoints...".blue());
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let mut available = HashMap::new();

    for endpoint in &config.endpoints
    {
        let url = format!("{}{}", &config.base_url, endpoint);
        match client.get(&url).send().await
        {
            Ok(resp) =>
            {
                let status = resp.status().as_u16();
                let ok = status < 400;
                println!("  {} - {}", endpoint, status);
                if ok || config.force_all
                {
                    available.insert(endpoint.clone(), status);
                }
            },
            Err(_) if config.force_all =>
            {
                available.insert(endpoint.clone(), 0);
            },
            Err(e) => println!("  {} - erro: {}", endpoint, e),
        }
    }
    Ok(available)
}

fn build_client(config: &Config) -> Result<Client>
{
    let mut builder = Client::builder()
        .user_agent("rust-benchmark/0.1.0")
        .pool_max_idle_per_host(config.concurrency * if config.no_keepalive { 0 } else { 4 })
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(60));

    if !config.no_keepalive
    {
        builder = builder
            .pool_idle_timeout(Duration::from_secs(300))
            .http2_keep_alive_timeout(Duration::from_secs(300));
    }

    Ok(builder.build()?)
}

async fn warmup(config: &Config) -> Result<()>
{
    println!("{}", "Realizando warmup...".blue());
    let client = Client::builder()
        .user_agent("rust-benchmark/0.1.0")
        .pool_max_idle_per_host(config.concurrency)
        .tcp_nodelay(true)
        .pool_idle_timeout(Duration::from_secs(30))
        .build()?;

    for endpoint in &config.endpoints
    {
        let url = format!("{}{}", &config.base_url, endpoint);
        for wave in 0..3
        {
            let num = 100 * (wave + 1);
            let futures = (0..num).map(|_| {
                let c = client.clone();
                let u = url.clone();
                async move {
                    let _ = c.get(&u).header("Connection", "keep-alive").send().await;
                }
            });
            stream::iter(futures)
                .buffer_unordered(std::cmp::min(num, config.concurrency))
                .collect::<Vec<_>>()
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    println!("{}", "Warmup concluído.".blue());
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}

async fn run_benchmark(client: &Client, url: &str, config: &Config) -> Result<BenchmarkResult>
{
    let start = Instant::now();
    let end = start + Duration::from_secs(config.duration);
    let mut results = Vec::new();
    let pb = ProgressBar::new(config.requests as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} ({elapsed_precise}) {percent}% ({eta}) [RPS: {msg}]",
            )?
            .progress_chars("►■□"),
    );
    let mut current = 0;
    let mut batch_size = std::cmp::min(config.default_req_per_batch, config.concurrency);
    let (mut last_time, mut last_count) = (start, 0);
    let mut current_rps = 0.0;

    while Instant::now() < end && current < config.requests
    {
        if current > 1000
        {
            batch_size = std::cmp::min(config.concurrency, config.requests - current);
        }
        let batch = run_batch(client, url, batch_size).await;
        let cnt = batch.len();
        results.extend(batch);
        current += cnt;
        pb.set_position(current as u64);

        let now = Instant::now();
        let elapsed = now.duration_since(last_time);
        if elapsed.as_secs_f64() >= 1.0
        {
            let since = current - last_count;
            current_rps = since as f64 / elapsed.as_secs_f64();
            pb.set_message(format!("{:.2}", current_rps));
            last_time = now;
            last_count = current;
        }
    }

    pb.finish_with_message(format!("Completo: {} req, {:.2} req/s", current, current_rps));
    let total = start.elapsed().as_secs_f64();
    if results.is_empty()
    {
        return Ok(BenchmarkResult {
            requests: 0,
            successful: 0,
            total_time: total,
            rps: 0.0,
            success_rate: 0.0,
            latency_stats: None,
        });
    }

    let success = results.iter().filter(|r| r.success).count();
    let latencies: Vec<f64> = if results.len() > 10_000
    {
        let mut rng = rand::thread_rng();
        results.choose_multiple(&mut rng, 10_000).map(|r| r.duration_ms).collect()
    }
    else
    {
        results.iter().map(|r| r.duration_ms).collect()
    };

    let stats = LatencyStats {
        min: *latencies.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
        max: *latencies.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
        mean: latencies.iter().sum::<f64>() / latencies.len() as f64,
        median: percentile(&latencies, 50.0),
        p90: percentile(&latencies, 90.0),
        p99: percentile(&latencies, 99.0),
    };

    Ok(BenchmarkResult {
        requests: results.len(),
        successful: success,
        total_time: total,
        rps: results.len() as f64 / total,
        success_rate: success as f64 / results.len() as f64,
        latency_stats: Some(stats),
    })
}

async fn run_batch(client: &Client, url: &str, size: usize) -> Vec<RequestResult>
{
    let mut out = Vec::with_capacity(size);
    const MAX_BATCH: usize = 1_000;
    let chunks = (size + MAX_BATCH - 1) / MAX_BATCH;
    for _ in 0..chunks
    {
        let bs = std::cmp::min(MAX_BATCH, size - out.len());
        let futs = (0..bs).map(|_| {
            let cli = client.clone();
            let u = url.to_string();
            async move {
                let start = Instant::now();
                match cli.get(&u).header("Connection", "keep-alive").send().await
                {
                    Ok(r) =>
                    {
                        let status = r.status().as_u16();
                        let success = r.status().is_success();
                        let _ = r.bytes().await;
                        let d = start.elapsed().as_secs_f64() * 1000.0;
                        RequestResult {
                            duration_ms: d,
                            status,
                            success,
                            error: None,
                        }
                    },
                    Err(e) =>
                    {
                        let d = start.elapsed().as_secs_f64() * 1000.0;
                        RequestResult {
                            duration_ms: d,
                            status: 0,
                            success: false,
                            error: Some(e.to_string()),
                        }
                    },
                }
            }
        });
        let res: Vec<RequestResult> = stream::iter(futs).buffer_unordered(bs).collect().await;
        out.extend(res);
    }
    out
}

fn percentile(data: &[f64], p: f64) -> f64
{
    if data.is_empty()
    {
        return 0.0;
    }
    let mut s = data.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = (s.len() - 1) as f64 * p / 100.0;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi
    {
        s[lo]
    }
    else
    {
        s[lo] * (hi as f64 - idx) + s[hi] * (idx - lo as f64)
    }
}

fn print_results(all: &HashMap<String, BenchmarkResult>, config: &Config) -> Result<()>
{
    println!("{:=^80}", " Resultados do Benchmark ");
    println!(
        "{:<15} | {:>10} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8}",
        "Endpoint", "Requests", "RPS", "Sucesso", "Média", "Mediana", "p90", "p99"
    );
    println!("{:-<80}", "");
    for (ep, r) in all
    {
        if let Some(lat) = &r.latency_stats
        {
            println!(
                "{:<15} | {:>10} | {:>8.2} | {:>7.1}% | {:>7.2}ms | {:>7.2}ms | {:>7.2}ms | {:>7.2}ms",
                ep,
                r.requests,
                r.rps,
                r.success_rate * 100.0,
                lat.mean,
                lat.median,
                lat.p90,
                lat.p99
            );
        }
    }

    // Salva em arquivo
    let dir = Path::new(&config.results_dir);
    if !dir.exists()
    {
        fs::create_dir_all(dir)?;
    }
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let file_path = dir.join(format!("benchmark_{}.txt", ts));
    let mut f = File::create(&file_path)?;
    writeln!(f, "Resultados do Benchmark - {:?}", Local::now())?;
    for (ep, r) in all
    {
        writeln!(f, "Endpoint: {}", ep)?;
        writeln!(f, "Requests: {}", r.requests)?;
        writeln!(f, "RPS: {:.2}", r.rps)?;
        writeln!(f, "Sucesso: {:.1}%", r.success_rate * 100.0)?;
        if let Some(lat) = &r.latency_stats
        {
            writeln!(f, "Latência média: {:.2} ms", lat.mean)?;
            writeln!(f, "Mediana: {:.2} ms", lat.median)?;
            writeln!(f, "p90: {:.2} ms", lat.p90)?;
            writeln!(f, "p99: {:.2} ms", lat.p99)?;
        }
        writeln!(f, "---")?;
    }
    println!("\nResultados salvos em {:?}", file_path.display());
    Ok(())
}
