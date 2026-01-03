use std::sync::Arc;
use parking_lot::RwLock;
use crate::brain::model::SemanticBrain;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::io::Read;
use bzip2::read::MultiBzDecoder;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use regex::Regex;
use futures_util::StreamExt;

#[derive(Clone, Serialize, Deserialize)]
pub struct TrainerConfig {
    pub dump_url: String,
    pub learning_rate: f32,
    pub window_size: usize,
    pub negative_samples: usize,
    pub max_articles: Option<usize>,
    pub checkpoint_every_articles: usize,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct TrainerState {
    pub articles_processed: usize,
    pub tokens_processed: u64,
    pub last_title: Option<String>,
    pub running: bool,
    pub error: Option<String>,
}

pub struct WikipediaTrainer {
    pub config: TrainerConfig,
    pub state: Arc<RwLock<TrainerState>>,
    pub brain: Arc<RwLock<SemanticBrain>>,
}

impl WikipediaTrainer {
    pub fn new(brain: Arc<RwLock<SemanticBrain>>) -> Self {
        let state = if let Ok(data) = fs::read_to_string("data/wiki_progress.json") {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            TrainerState::default()
        };

        let mut initial_state = state;
        initial_state.running = false;

        Self {
            config: TrainerConfig {
                dump_url: "https://dumps.wikimedia.org/enwiki/latest/enwiki-latest-pages-articles-multistream.xml.bz2".to_string(),
                learning_rate: 0.025,
                window_size: 5,
                negative_samples: 5,
                max_articles: None,
                checkpoint_every_articles: 100,
            },
            state: Arc::new(RwLock::new(initial_state)),
            brain,
        }
    }

    pub fn start(&self) {
        let mut state = self.state.write();
        if state.running {
            return;
        }
        state.running = true;
        state.error = None;
        drop(state);

        let trainer_state = self.state.clone();
        let brain = self.brain.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_training(brain, trainer_state.clone(), config).await {
                let mut state = trainer_state.write();
                state.running = false;
                state.error = Some(e.to_string());
            } else {
                let mut state = trainer_state.write();
                state.running = false;
            }
        });
    }

    pub fn stop(&self) {
        let mut state = self.state.write();
        state.running = false;
    }

    async fn run_training(
        brain: Arc<RwLock<SemanticBrain>>,
        state: Arc<RwLock<TrainerState>>,
        config: TrainerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("> Starting Wikipedia training pipeline...");
        println!("> URL: {}", config.dump_url);

        let client = reqwest::Client::builder()
            .user_agent("VecorsTrainer/1.0 (https://github.com/zoren-games/vecors; contact@example.com) Vecors/0.1.0")
            .build()?;
            
        let response = client.get(&config.dump_url).send().await?;
        
        if !response.status().is_success() {
            let err_msg = format!("HTTP Error: {}", response.status());
            eprintln!("> {}", err_msg);
            return Err(err_msg.into());
        }

        let bytes_stream = response.bytes_stream();
        println!("> Download stream established. Waiting for chunks...");

        let (tx, rx) = std::sync::mpsc::sync_channel(128);
        
        let stream_state = state.clone();
        tokio::spawn(async move {
            let mut stream = bytes_stream;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        if tx.send(bytes.to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("Stream error: {}", e);
                        eprintln!("> {}", err_msg);
                        let mut s = stream_state.write();
                        s.error = Some(err_msg);
                        break;
                    }
                }
            }
        });

        let blocking_state = state.clone();
        tokio::task::spawn_blocking(move || {
            struct ChannelReader {
                rx: std::sync::mpsc::Receiver<Vec<u8>>,
                current: std::io::Cursor<Vec<u8>>,
            }

            impl Read for ChannelReader {
                fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                    if self.current.position() >= self.current.get_ref().len() as u64 {
                        match self.rx.recv() {
                            Ok(bytes) => self.current = std::io::Cursor::new(bytes),
                            Err(_) => return Ok(0),
                        }
                    }
                    self.current.read(buf)
                }
            }

            let reader = ChannelReader {
                rx,
                current: std::io::Cursor::new(Vec::new()),
            };

            let bz_decoder = MultiBzDecoder::new(reader);
            let mut xml_reader = Reader::from_reader(std::io::BufReader::new(bz_decoder));
            xml_reader.trim_text(true);

            let mut buf = Vec::new();
            let mut current_title = String::new();
            let mut current_text = String::new();
            let mut in_text = false;
            let mut in_title = false;
            let mut article_count = 0;

            let re_link = Regex::new(r"\[\[([^|\]]+\|)?([^\]]+)\]\]").unwrap();
            let re_template = Regex::new(r"\{\{[^}]+\}\}").unwrap();
            let re_clean = Regex::new(r"[^A-Z\s]").unwrap();

            println!("> Parsing XML and training (this may take a minute to start as bzip2 decompresses)...");

            loop {
                {
                    let s = blocking_state.read();
                    if !s.running {
                        println!("> Training stop requested.");
                        break;
                    }
                }

                match xml_reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        match e.name().as_ref() {
                            b"title" => {
                                in_title = true;
                                current_title.clear();
                            }
                            b"text" => {
                                in_text = true;
                                current_text.clear();
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::Text(ref e)) => {
                        if in_title {
                            current_title.push_str(&e.unescape().unwrap_or_default());
                        } else if in_text {
                            current_text.push_str(&e.unescape().unwrap_or_default());
                        }
                    }
                    Ok(Event::CData(ref e)) => {
                        let text = String::from_utf8_lossy(e.as_ref()).to_string();
                        if in_title {
                            current_title.push_str(&text);
                        } else if in_text {
                            current_text.push_str(&text);
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        match e.name().as_ref() {
                            b"title" => in_title = false,
                            b"text" => {
                                in_text = false;
                                if !current_title.contains(':') && !current_text.is_empty() {
                                    println!("> Training on article: {}", current_title);
                                    let cleaned = re_template.replace_all(&current_text, "");
                                    let cleaned = re_link.replace_all(&cleaned, "$2");
                                    let cleaned = cleaned.to_uppercase();
                                    let cleaned = re_clean.replace_all(&cleaned, " ");
                                    
                                    let tokens: Vec<String> = cleaned
                                        .split_whitespace()
                                        .filter(|t| t.len() >= 3)
                                        .map(|t| t.to_string())
                                        .take(2000)
                                        .collect();

                                    if !tokens.is_empty() {
                                        {
                                            let mut b = brain.write();
                                            let current_tokens = { state.read().tokens_processed };
                                            let alpha = (config.learning_rate * (1.0 - (current_tokens as f32 / 500_000_000.0))).max(0.0001);
                                            
                                            for i in 0..tokens.len() {
                                                let center = &tokens[i];
                                                let start = i.saturating_sub(config.window_size);
                                                let end = (i + config.window_size + 1).min(tokens.len());
                                                let context: Vec<String> = tokens[start..i]
                                                    .iter()
                                                    .chain(tokens[i + 1..end].iter())
                                                    .cloned()
                                                    .collect();
                                                
                                                b.train_step(center, &context, &[], alpha, config.negative_samples);
                                            }
                                        }
                                        
                                        article_count += 1;
                                        {
                                            let mut s = blocking_state.write();
                                            s.articles_processed += 1;
                                            s.tokens_processed += tokens.len() as u64;
                                            s.last_title = Some(current_title.clone());
                                        }

                                        if article_count % config.checkpoint_every_articles == 0 {
                                            Self::save_checkpoint_sync(&brain, &blocking_state)?;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::Eof) => {
                        println!("> XML end of file reached.");
                        break;
                    }
                    Err(e) => {
                        let err_msg = format!("XML error: {}", e);
                        eprintln!("> {}", err_msg);
                        let mut s = blocking_state.write();
                        s.error = Some(err_msg);
                        break;
                    }
                    _ => {}
                }
                buf.clear();

                if let Some(max) = config.max_articles {
                    if article_count >= max {
                        println!("> Max articles reached ({}).", max);
                        break;
                    }
                }
            }

            Self::save_checkpoint_sync(&brain, &blocking_state)?;
            println!("> Training pipeline finished.");
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }).await?
    }

    fn save_checkpoint_sync(
        brain: &Arc<RwLock<SemanticBrain>>,
        state: &Arc<RwLock<TrainerState>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let b = brain.read();
        let bytes = b.to_bytes()?;
        let temp_path = "data/model.bin.tmp";
        let final_path = "data/model.bin";
        
        if !Path::new("data").exists() {
            fs::create_dir_all("data")?;
        }
        
        fs::write(temp_path, bytes)?;
        fs::rename(temp_path, final_path)?;

        let s = state.read();
        let progress_json = serde_json::to_string(&*s)?;
        fs::write("data/wiki_progress.json", progress_json)?;
        
        println!("Checkpoint saved: {} articles processed", s.articles_processed);
        Ok(())
    }
}
