use constellations::asset::block::text::{Edit, EncodedText, Text};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use serde::Deserialize;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use ulid::Ulid;
use zenoh::config::WhatAmI;
use zenoh::pubsub::{Publisher, Subscriber};
use zenoh::{session, Session};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::str::from_utf8;
use std::sync::Arc;
use tui_textarea::{CursorMove, Input, Key, TextArea};
use rand::{thread_rng, Rng};

macro_rules! error {
    ($fmt: expr $(, $args:tt)*) => {{
        Err(io::Error::new(io::ErrorKind::Other, format!($fmt $(, $args)*)))
    }};
}

struct LlmBox<'a> {
    textarea: TextArea<'a>,
    open: bool,
}

impl<'a> Default for LlmBox<'a> {
    fn default() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().borders(Borders::ALL).title("Chat"));
        Self {
            textarea,
            open: false,
        }
    }
}

impl<'a> LlmBox<'a> {
    fn open(&mut self) {
        self.open = true;
    }

    fn close(&mut self) {
        self.open = false;
        // Remove input for next search. Do not recreate `self.textarea` instance to keep undo history so that users can
        // restore previous input easily.
        self.textarea.move_cursor(CursorMove::End);
        self.textarea.delete_line_by_head();
    }

    fn height(&self) -> u16 {
        if self.open {
            3
        } else {
            0
        }
    }

    fn input(&mut self, input: Input) -> Option<&'_ str> {
        match input {
            Input {
                key: Key::Enter, ..
            }
            | Input {
                key: Key::Char('m'),
                ctrl: true,
                ..
            } => None, // Disable shortcuts which inserts a newline. See `single_line` example
            input => {
                let modified = self.textarea.input(input);
                modified.then(|| self.textarea.lines()[0].as_str())
            }
        }
    }

    fn set_error(&mut self, err: Option<impl Display>) {
        let b = if let Some(err) = err {
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Chat: {}", err))
                .style(Style::default().fg(Color::Red))
        } else {
            Block::default().borders(Borders::ALL).title("Chat")
        };
        self.textarea.set_block(b);
    }
}

struct Buffer<'a> {
    id: Ulid,
    textarea: TextArea<'a>,
    text: Arc<Mutex<constellations::asset::block::text::Text>>,
    subscriber_update: Arc<Mutex<bool>>,
    modified: bool,
    realtime: bool,
    materialized: bool,
    publisher: Option<Publisher<'a>>,
    realtime_publisher: Option<Publisher<'a>>,
    task_tx: mpsc::Sender<JoinHandle<()>>,
}

impl<'a> Buffer<'a> {
    fn new(text: constellations::asset::block::text::Text, id: Ulid, task_tx: mpsc::Sender<JoinHandle<()>>) -> io::Result<Self> {
        let lines: Vec<String> = text.buffer.lines().map(|x| x.to_string()).collect();
        let mut textarea = TextArea::new(lines);
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        Ok(Self {
            id,
            textarea,
            text: Arc::new(Mutex::new(text)),
            subscriber_update: Arc::new(Mutex::new(false)),
            modified: false,
            realtime: false,
            materialized: false,
            publisher: None,
            realtime_publisher: None,
            task_tx,
        })
    }

    async fn save(&mut self, session: Session, user: String) -> io::Result<()> {
        if !self.modified {
            return Ok(());
        }

        let content = self.textarea.lines().join("\n");

        if self.publisher.is_none() {
            let pub_key_expression = user.clone() + "/holobank/block/text/" + self.id.to_string().as_str();
            
            let client = reqwest::Client::new();
            let url = "http://127.0.0.1:9999/v0/edittext?user=".to_string() + &user + "&id=" + self.id.to_string().as_str();
            let res = client.get(url.as_str()).send().await.unwrap();
            
            self.publisher = Some(session.declare_publisher(pub_key_expression).await.unwrap());
        }

        self.publisher.as_mut().unwrap().put(content).await.unwrap();
        self.modified = false;
        Ok(())
    }

    async fn enable_realtime_edit(&mut self, session: Session, user: String) -> io::Result<()> {

        if self.realtime {
            self.realtime = false;
            self.realtime_publisher = None;
        }
        else {
            let key_expression = user.to_string() + "/editor/encodedtext?id=" + self.id.to_string().as_str();
            let replies = session.get(key_expression.as_str()).await.unwrap();
            while let Ok(reply) = replies.recv_async().await {
                let enc_text: EncodedText = serde_json::from_str(String::from_utf8(reply.result().unwrap().payload().to_bytes().to_vec()).unwrap().as_str()).unwrap();
                let text = Text::from(enc_text);
                *self.text.lock().await = text;
                break;
            }

            let text_arc = Arc::clone(&self.text);
            let sub_update_arc = Arc::clone(&self.subscriber_update);
            let id_clone = self.id.clone();
            let user_clone = user.clone();
            let subscriber_handle = tokio::spawn(async move {
                let key_expression = user_clone.clone() + "/realtime/block/text/" + id_clone.to_string().as_str();
                let subscriber = session.declare_subscriber(key_expression.as_str()).await.unwrap();
                while let Ok(sample) = subscriber.recv_async().await {
                    let update = String::from_utf8(sample.payload().to_bytes().to_vec()).unwrap();
                    // println!("{}", update);
                    let edit: Edit = serde_json::from_str(&update).unwrap();
                    match edit {
                        Edit::Inserted(ins) => {
                            text_arc.lock().await.integrate_insertion(ins);
                        }
                        Edit::Deleted(del) => {
                            text_arc.lock().await.integrate_deletion(del);
                        }
                    }
                    *sub_update_arc.lock().await = true;
                }
            });
            self.task_tx.send(subscriber_handle).await.unwrap();
            
            self.realtime = true;
        }
        Ok(())
    }

    pub async fn update(&mut self) {
        let cursor = self.textarea.cursor().clone();
        let lines: Vec<String> = self.text.lock().await.buffer.lines().map(|x| x.to_string()).collect();
        let new: TextArea<'a> = TextArea::new(lines);
        self.textarea = new;
        self.textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        self.textarea.move_cursor(CursorMove::Jump(u16::try_from(cursor.0).unwrap(), u16::try_from(cursor.1).unwrap()));
        self.textarea.input(Input { key: Key::Null, ctrl: false, alt: false, shift: false});
    }
}

struct Editor<'a> {
    user: String,
    current: usize,
    blocks: Vec<Buffer<'a>>,
    term: Terminal<CrosstermBackend<io::Stdout>>,
    message: Option<Cow<'static, str>>,
    chat: LlmBox<'a>,
    session: Session,
    ext_session: Session,
}

impl<'a> Editor<'a> {
    async fn new(session: Session, ext_session: Session, user: &str) -> io::Result<(Self, mpsc::Receiver<JoinHandle<()>>)> {
        let (tx,rx) = mpsc::channel(100);
        let mut rng = thread_rng();
        let mut blocks: Vec<Buffer<'a>> = Vec::new();
        let client = reqwest::Client::new();
        let url = "http://127.0.0.1:9999/v0/text?name=".to_string() + user;
        let res = client.get(url.as_str()).send().await.unwrap();
        let block_ids = res.text_with_charset("utf-8").await.unwrap();
        let block_ids: Vec<Ulid> = if block_ids.len() > 4 {
            block_ids.trim().split(',').into_iter().map(|x| {
                Ulid::from_string(x).unwrap()
            }).collect()
        }
        else {
            Vec::new()
        };

        for block_id in block_ids {
            let key_expression = user.to_string() + "/holobank/block/text?id=" + block_id.to_string().as_str();
            let replies = session.get(key_expression.as_str()).await.unwrap();
            while let Ok(reply) = replies.recv_async().await {
                let content = String::from_utf8(reply.result().unwrap().payload().to_bytes().to_vec()).unwrap();
                let buf = Buffer::new(Text::new(content, rng.gen()), block_id, tx.clone()).unwrap();
                blocks.push(buf);
                break;
            }
        }

        if blocks.len() < 1 {
            let new_id = Ulid::new();
            let url = "http://127.0.0.1:9999/v0/newtext?user=".to_string() + user + "&id=" + new_id.to_string().as_str();
            client.get(url.as_str()).send().await.unwrap();
            let buf = Buffer::new(Text::new("New text block!", rng.gen()), new_id, tx.clone()).unwrap();
            blocks.push(buf)
        }

        let mut stdout = io::stdout();
        enable_raw_mode()?;
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let term = Terminal::new(backend)?;
        Ok((Self {
            user: user.to_string(),
            current: 0,
            blocks,
            term,
            message: None,
            chat: LlmBox::default(),
            session: session.clone(),
            ext_session: ext_session.clone(),
        },
        rx))
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<JoinHandle<()>>) -> io::Result<()> {
        let _ = tokio::spawn( async move {
            while let Some(handle) = rx.recv().await {
                match handle.await {
                    Ok(_) => {
                        println!("Task completed successfully");
                    }
                    Err(e) => {
                        eprintln!("Task failed: {:?}", e);
                    }
                }
            }
        });
        loop {
            let chat_height = self.chat.height();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(chat_height),
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                );
            
            let sub_update = *self.blocks[self.current].subscriber_update.lock().await;
            self.term.draw(|f| {
                let chunks = layout.split(f.area());

                if chat_height > 0 {
                    f.render_widget(&self.chat.textarea, chunks[0]);
                }

                let block = &self.blocks[self.current];
                let textarea = &block.textarea;
                f.render_widget(textarea, chunks[1]);

                // Render status line
                let modified = if block.modified { " [modified]" } else { "" };
                let realtime = if block.realtime { " [realtime]" } else { "" };
                let updating = if sub_update { " [updating]" } else { "" };
                let slot = format!("[{}/{}]", self.current + 1, self.blocks.len());
                let id = format!(" {}{}{}{} ", block.id.to_string(), modified, realtime, updating);
                let (row, col) = textarea.cursor();
                let cursor = format!("({},{})", row + 1, col + 1);
                let status_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(slot.len() as u16),
                            Constraint::Min(1),
                            Constraint::Length(cursor.len() as u16),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[2]);
                let status_style = Style::default().add_modifier(Modifier::REVERSED);
                f.render_widget(Paragraph::new(slot).style(status_style), status_chunks[0]);
                f.render_widget(Paragraph::new(id).style(status_style), status_chunks[1]);
                f.render_widget(Paragraph::new(cursor).style(status_style), status_chunks[2]);

                // Render message at bottom
                let message = if let Some(message) = self.message.take() {
                    Line::from(Span::raw(message))
                } else if chat_height > 0 {
                    Line::from(vec![
                        Span::raw("Press "),
                        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to send prompt, "),
                        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to close, ")
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("Press "),
                        Span::styled("^Q", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to quit, "),
                        Span::styled("^S", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to save, "),
                        Span::styled("^G", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to chat, "),
                        Span::styled("^T", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to switch blocks, "),
                        Span::styled("^R", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to toggle realtime edit"),
                    ])
                };
                f.render_widget(Paragraph::new(message), chunks[3]);
            })?;

            *self.blocks[self.current].subscriber_update.lock().await = false;

            if chat_height > 0 {
                match crossterm::event::read()?.into() {
                    Input {
                        key: Key::Enter, ..
                    } => {
                        self.chat.close();
                    }
                    Input { key: Key::Esc, .. } => {
                        self.chat.close();
                    }
                    input => {
                        if let Some(query) = self.chat.input(input) {

                        }
                    }
                }
            } else {
                match crossterm::event::read()?.into() {
                    Input {
                        key: Key::Char('q'),
                        ctrl: true,
                        ..
                    } => break,
                    Input {
                        key: Key::Char('t'),
                        ctrl: true,
                        ..
                    } => {
                        self.current = (self.current + 1) % self.blocks.len();
                        self.message =
                            Some(format!("Switched to block #{}", self.current + 1).into());
                    }
                    Input {
                        key: Key::Char('s'),
                        ctrl: true,
                        ..
                    } => {
                        self.blocks[self.current].save(self.session.clone(), self.user.clone()).await?;
                        self.message = Some("Saved to holobank!".into());
                    }
                    Input {
                        key: Key::Char('g'),
                        ctrl: true,
                        ..
                    } => {
                        self.chat.open();
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: true,
                        ..
                    } => {
                        self.blocks[self.current].enable_realtime_edit(self.ext_session.clone(), self.user.clone()).await?;
                        self.message = Some("Toggled realtime edit!".into());
                    }
                    input => {
                        let cursor = self.blocks[self.current].textarea.cursor();
                        // let rows = self.blocks[self.current].textarea.lines()[0..cursor.0].to_vec();
                        // let all_rows = self.blocks[self.current].textarea.lines()[0..].to_vec();
                        let rows: Vec<String> = self.blocks[self.current].text.lock().await.buffer.split_inclusive('\n').into_iter().map(|x| x.to_string()).collect();

                        let mut sum: usize = cursor.1;
                        for i in 0..cursor.0 {
                            sum = sum + rows[i].len();
                        }

                        let mut end: usize = 0;
                        for i in 0..rows.len() {
                            end = end + rows[i].len();
                        }
                        match input {
                            Input {
                                key: Key::Char('a'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "a");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    let payload = serde_json::to_string(&edit).unwrap();
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(payload).await.unwrap();
                                }
                            },
                            Input {
                                key: Key::Char('b'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "b");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    let payload = serde_json::to_string(&edit).unwrap();
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(payload).await.unwrap();
                                }
                            },
                            Input {
                                key: Key::Char('c'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "c");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('d'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "d");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('e'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "e");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('f'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "f");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('g'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "g");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('h'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "h");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('i'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "i");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('j'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "j");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('k'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "k");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('l'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "l");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('m'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "m");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('n'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "n");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('o'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "o");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('p'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "p");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('q'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "q");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('r'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "r");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('s'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "s");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('t'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "t");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('u'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "u");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('v'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "v");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('w'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "w");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('x'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "x");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('y'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "y");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('z'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "z");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('A'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "A");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    let payload = serde_json::to_string(&edit).unwrap();
                                    self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(payload).await.unwrap();
                                }
                            },
                            Input {
                                key: Key::Char('B'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "B");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    let payload = serde_json::to_string(&edit).unwrap();
                                    self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(payload).await.unwrap();
                                }
                            },
                            Input {
                                key: Key::Char('C'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "C");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('D'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "D");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('E'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "E");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('F'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "F");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('G'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "G");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('H'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "H");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('I'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "I");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('J'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "J");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('K'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "K");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('L'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "L");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('M'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "M");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('N'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "N");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('O'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "O");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('P'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "P");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('Q'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "Q");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('R'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "R");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('S'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "S");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('T'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "T");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('U'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "U");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('V'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "V");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('W'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "W");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('X'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "X");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('Y'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "Y");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('Z'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "Z");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char(' '),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, " ");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('!'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "!");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('@'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "@");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('#'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "#");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('$'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "$");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('%'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "%");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('^'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "^");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('&'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "&");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('*'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "*");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('('),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "(");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char(')'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ")");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('_'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "_");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('-'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "-");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('='),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "=");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('+'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "+");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char(':'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ":");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char(';'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ";");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('\''),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "\'");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('\"'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "\"");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('?'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "?");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('.'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ".");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char(','),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ",");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('<'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "<");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('>'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, ">");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('1'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "1");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('2'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "2");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('3'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "3");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('4'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "4");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('5'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "5");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('6'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "6");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('7'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "7");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('8'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "8");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('9'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "9");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Char('0'),
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "0");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Enter,
                                ..
                            } => {
                                let ins = self.blocks[self.current].text.lock().await.insert(sum, "\n");
                                if self.blocks[self.current].realtime {
                                    let edit = Edit::Inserted(ins);
                                    // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                }
                            },
                            Input {
                                key: Key::Backspace,
                                ..
                            } => {
                                if sum != 0 {
                                    let del = self.blocks[self.current].text.lock().await.delete((sum-1)..sum);
                                    if self.blocks[self.current].realtime {
                                        let edit = Edit::Deleted(del);
                                        // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                    }
                                }
                            },
                            Input {
                                key: Key::Delete,
                                ..
                            } => {
                                if sum != end {
                                    let del = self.blocks[self.current].text.lock().await.delete((sum+1)..(sum+2));
                                    if self.blocks[self.current].realtime {
                                        let edit = Edit::Deleted(del);
                                        // self.blocks[self.current].realtime_publisher.as_ref().unwrap().put(serde_json::to_string(&edit).unwrap()).await;
                                    }
                                }
                            },
                            _ => {}
                        }
                        let buffer = &mut self.blocks[self.current];
                        buffer.modified = buffer.textarea.input(input);
                    }
                }
            }
            self.blocks[self.current].update().await;
        }

        Ok(())
    }
}

impl<'a> Drop for Editor<'a> {
    fn drop(&mut self) {
        self.term.show_cursor().unwrap();
        disable_raw_mode().unwrap();
        crossterm::execute!(
            self.term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut config = zenoh::Config::default();
    config.set_mode(Some(WhatAmI::Client));
    config.scouting.multicast.set_enabled(Some(false));
    config.scouting.gossip.set_enabled(Some(false));
    config.transport.link.set_protocols(Some(vec!["unixsock-stream".to_string()]));
    config.connect.endpoints.set([
        "unixsock-stream//tmp/".to_string() +
        "test_spaceport" +
        ".sock"
    ].iter().map(|s|s.parse().unwrap()).collect()
    ).unwrap();

    let mut config_ext = zenoh::Config::default();
    config_ext.set_mode(Some(WhatAmI::Peer));
    config_ext.scouting.multicast.set_enabled(Some(true));
    config_ext.scouting.gossip.set_enabled(Some(true));
    config_ext.transport.link.set_protocols(Some(vec!["tcp".to_string()]));

    let zenoh_session = zenoh::open(config).await.unwrap();
    let ext_zenoh_session = zenoh::open(config_ext).await.unwrap();

    let (mut editor, task_rx) = Editor::new(zenoh_session.clone(), ext_zenoh_session.clone(), "test").await.unwrap();
    editor.run(task_rx).await
}