use std::fs;
use std::env;
use std::{
    // error::Error,
    // io,
    time::{Duration, Instant},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    style::{Color,Style,Modifier},
    text::{Span, Spans},
    layout::{Alignment,Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, BorderType, Clear, ListState, ListItem, List, Paragraph, Wrap},
    Frame, Terminal,
};
use sqlx::sqlite::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;

    sqlx::migrate!("db/migrations")
        .run(&pool)
        .await?;

    // // Insert the task, then obtain the ID of this row
    // let id = sqlx::query_unchecked!(
    //     r#"
    //     BEGIN;
    //     CREATE TABLE sources (
    //         id INT,
    //         name VARCHAR,
    //         url VARCHAR
    //     );
    //
    //     CREATE TABLE articles (
    //         id INT,
    //         title VARCHAR,
    //         url VARCHAR
    //     );
    //
    //     CREATE TABLE source_articles (
    //         source_id INT,
    //         article_id INT
    //     );
    //
    //     CREATE UNIQUE INDEX unique_article_url_idx ON articles(url);
    //     CREATE UNIQUE INDEX unique_source_url_idx ON sources(url);
    //     CREATE UNIQUE INDEX unique_source_name_idx ON sources(name);
    //     CREATE UNIQUE INDEX unique_source_article_idx ON source_articles(source_id, article_id);
    //
    //     COMMIT;
    //     "#,
    // )
    // .execute(&mut conn);
    // .await?;

    // println!("foo 2...\n");

    // let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    // let mut conn = pool.acquire().await?;
    //
    // let id = sqlx::query_unchecked!(
    //     r#"
    //     CREATE UNIQUE INDEX unique_article_url_idx ON articles(url);
    //     CREATE UNIQUE INDEX unique_source_url_idx ON sources(url);
    //     CREATE UNIQUE INDEX unique_source_name_idx ON sources(name);
    //     CREATE UNIQUE INDEX unique_source_article_idx ON source_articles(source_id, article_id);
    //     "#,
    // )
    // .execute(&mut conn)
    // .await?;

    // println!("foo 3...\n");

    // let resp = reqwest::get("https://blog.cleancoder.com/atom.xml")
    //     .await?;
    // let body = resp.text().await?;
    // let articles = news::parse_atom(body.clone().as_ref())?;
    //
    // println!("------- PARSED --------");
    // println!("{:?}", articles);
    // println!("-----------------------");
    // let articles = articles.iter().map(|a| a).collect();

    // let sources = sqlx::query!(
    //     "select * from sources"
    // )
    // .fetch_one(&mut conn)
    // .await?;

    // let articles = sqlx::query!(
    //     "select * from articles"
    // )
    // .fetch_all(&mut conn)
    // .await?;

    // println!("ARTICLES: {:?}", articles);
    // println!("FOUND SOURCES: {:?}", sources);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = AppState::new(pool).await?;
    let app = App::new(&mut state);
    let tick_rate = Duration::from_millis(50);

    // create app and run it
    let res = run_app(&mut terminal, app, tick_rate).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // if let Err(err) = res {
    //     println!("{:?}", err)
    // }

    Ok(())
}

enum InputMode {
    Normal,
    Editing,
}

enum InputSelected {
    SourceName,
    SourceURL,
}

enum SelectedPane {
    Sources,
    Articles,
    ArticleText,
}

#[derive(Clone)]
struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T: Clone> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected(self) -> Option<T> {
        return match self.state.selected() {
            Some(i) => Some(self.items[i].clone()),
            _ => None::<T>,
        };
    }

    // fn unselect(&mut self) {
    //     self.state.select(None);
    // }
}

struct AppState<'a> {
    pool: sqlx::sqlite::SqlitePool,
    sources: Vec<news::Source>,
    articles: Vec<news::Article>,
    _phantom_data: std::marker::PhantomData<&'a ()>, // TODO: remove this
}

impl<'a> AppState<'a> {
    async fn new(pool: sqlx::sqlite::SqlitePool) -> Result<AppState<'a>, Box<dyn Error>> {
        let mut conn = pool.acquire().await?;
        let found_sources = sqlx::query!(
            "select * from sources"
        )
        .fetch_all(&mut conn)
        .await?;

        let mut sources = vec![];
        for s in found_sources.iter() {
            sources.push(news::Source{
                name: s.name.clone().unwrap_or("foo".to_string()),
                url: s.url.clone().unwrap_or("foo".to_string()),
            });
        }

        let contents = fs::read_to_string("example_rss.xml").expect("Should have been able to read the file");
        let articles = news::parse_rss(contents.as_ref()).unwrap_or_default();
        Ok(AppState {
            pool,
            sources,
            articles,
            _phantom_data: std::marker::PhantomData,
        })
    }

    fn set_articles(&mut self, articles: Vec<news::Article>) {
        self.articles = articles;
    }

    fn set_sources(&mut self, sources: Vec<news::Source>) {
        self.sources = sources;
    }
}

struct App<'a> {
    inputURL: String,
    inputSourceName: String,
    input_mode: InputMode,
    input_selected: InputSelected,
    state: &'a mut AppState<'a>,
    sources: StatefulList<news::Source>,
    articles: StatefulList<news::Article>,
    selected: SelectedPane,
    last_action: i8,
    scroll: u16,
    show_popup: bool,
}

impl<'a> App<'a> {
    fn new(state: &'a mut AppState<'a>) -> App<'a> {
        let mut articles: Vec<news::Article> = Vec::new();
        for article in state.articles.iter() {
            articles.push(article.clone());
        }
        let mut sources: Vec<news::Source> = Vec::new();
        for source in state.sources.iter() {
            sources.push(source.clone());
        }
        App {
            inputURL: String::new(),
            inputSourceName: String::new(),
            input_mode: InputMode::Normal,
            input_selected: InputSelected::SourceName,
            state: state,
            scroll: 0,
            last_action: 0,
            selected: SelectedPane::Sources,
            show_popup: false,
            sources: StatefulList::with_items(sources),
            articles: StatefulList::with_items(articles),
        }
    }

    // fn update_articles(&mut self) {
    //     let contents = fs::read_to_string("example_atom.xml").expect("Should have been able to read the file");
    //     let articles = news::parse_atom(contents.as_ref()).unwrap_or_default();
    //     self.state.set_articles(articles);
    //     self.articles = StatefulList::with_items(self.state.articles.clone());
    // }

    // fn update_sources(&mut self) {
    //     let source_1 = news::Source{ name: "foo 1".to_string(), url: "bar 2".to_string() };
    //     let source_2 = news::Source{ name: "foo 2".to_string(), url: "bar 2".to_string() };
    //     let source_3 = news::Source{ name: "foo 3".to_string(), url: "bar 2".to_string() };
    //     let source_4 = news::Source{ name: "foo 4".to_string(), url: "bar 2".to_string() };
    //     let sources = vec![source_1, source_2, source_3, source_4];
    //     self.state.set_sources(sources);
    //     self.sources = StatefulList::with_items(self.state.sources.clone());
    //     // self.sources.state.select(Some(1));
    // }

    async fn add_source(&mut self, source_name: String, source_url: String) -> Result<(), Box<dyn Error>> {
        let mut conn = self.state.pool.acquire().await?;
        let id = sqlx::query!(
            r#"
            INSERT INTO sources (name, url)
            VALUES (?1, ?2)
            "#,
            source_name, source_url,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        let mut sources = self.state.sources.clone();
        sources.push(news::Source{ name: source_name, url: source_url });
        self.state.set_sources(sources);
        self.sources = StatefulList::with_items(self.state.sources.clone());

        Ok(())
    }

    fn on_tick(&mut self) {
        if self.last_action == 1 {
            self.scroll += 3;
        } else if self.last_action == 2 {
            if (self.scroll as i16) - 3 >= 0 {
                self.scroll -= 3;
            }
        }
    }

    fn next_block(&mut self) {
        match self.selected {
            SelectedPane::Sources => self.selected = SelectedPane::Articles,
            SelectedPane::Articles => self.selected = SelectedPane::ArticleText,
            SelectedPane::ArticleText => self.selected = SelectedPane::Sources,
        }
    }

    async fn save_articles(&self, articles: Vec<news::Article>) -> Result<(), Box<dyn Error>> {
        let mut conn = self.state.pool.acquire().await?;
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "INSERT INTO articles(title, url) "
        );

        const BIND_LIMIT: usize = 65535;

        query_builder.push_values(articles.into_iter().take(BIND_LIMIT / 4), |mut b, article| {
            b.push_bind(article.title)
             .push_bind(article.url);
        });

        let mut query = query_builder.build();
        query.execute(&mut conn).await?;

        Ok(())
    }
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App<'_>,
    tick_rate: Duration,
) -> Result<(), Box<dyn Error>> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        app.last_action = 0;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    // KeyCode::Char('a') => {
                    //     app.update_articles();
                    // },
                    KeyCode::Char('d') => {
                        match app.selected {
                            SelectedPane::Sources  => {
                                match app.sources.clone().selected() {
                                    Some(source) => {
                                        let resp = reqwest::get(source.url).await?;
                                        let body = resp.text().await?;
                                        let articles = news::parse_rss(body.clone().as_ref())?;
                                        app.save_articles(articles).await?;
                                    },
                                    _ => {},
                                }
                            },
                            _ => {},
                        }
                    },
                    KeyCode::Char('p') => {
                        app.inputURL = "".to_string();
                        app.inputSourceName = "".to_string();
                        app.input_mode = InputMode::Editing;
                        app.input_selected = InputSelected::SourceName;
                        app.show_popup = !app.show_popup;
                    },
                    KeyCode::Down | KeyCode::Char('j') => {
                        match app.selected {
                            SelectedPane::Sources => app.sources.next(),
                            SelectedPane::Articles => {
                                app.scroll = 0;
                                app.articles.next();
                            },
                            _ => {
                                match app.selected {
                                    SelectedPane::ArticleText => app.last_action = 1,
                                    _ => {}
                                }
                            },
                        }
                    },
                    KeyCode::Up | KeyCode::Char('k') => {
                        match app.selected {
                            SelectedPane::Sources => app.sources.previous(),
                            SelectedPane::Articles => {
                                app.scroll = 0;
                                app.articles.previous();
                            }
                            _ => {
                                match app.selected {
                                    SelectedPane::ArticleText => app.last_action = 2,
                                    _ => {},
                                }
                            },
                        }
                    },
                    KeyCode::Tab => app.next_block(),
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        // TODO: handle add source and don't close the form in case of error
                        match app.add_source(app.inputSourceName.clone(), app.inputURL.clone()).await {
                            Ok(x) => {
                                // println!("X: {:?}", x);
                                // ..
                            },
                            Err(err) => {
                                // TODO: handle error - display error message
                                continue;
                            }
                        }
                        // app.add_source(app.inputSourceName.clone(), app.inputURL.clone()).await?;
                        app.input_mode = InputMode::Normal;
                        app.show_popup = !app.show_popup;
                    }
                    KeyCode::Tab => {
                        match app.input_selected {
                            InputSelected::SourceName => app.input_selected = InputSelected::SourceURL,
                            InputSelected::SourceURL => app.input_selected = InputSelected::SourceName,
                        }
                    }
                    KeyCode::Char(c) => {
                        match app.input_selected {
                            InputSelected::SourceName => app.inputSourceName.push(c),
                            InputSelected::SourceURL => app.inputURL.push(c),
                        }
                    }
                    KeyCode::Backspace => {
                        match app.input_selected {
                            InputSelected::SourceName => { app.inputSourceName.pop(); },
                            InputSelected::SourceURL => { app.inputURL.pop(); },
                        }
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.show_popup = !app.show_popup;
                    }
                    _ => {}
                }
            }
        }

        match app.selected  {
            SelectedPane::ArticleText => {
                if last_tick.elapsed() >= tick_rate {
                    app.on_tick();
                    last_tick = Instant::now();
                }
            },
            _ => {},
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Wrapping block for a group
    // Just draw the block and the group on the same area and build the group
    // with at least a margin of 1
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(1),
        ].as_ref())
        .split(f.size());

    let block = Block::default()
        .borders(Borders::NONE)
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let text = vec![
        Spans::from(vec![
            Span::styled(":: quit[q] | next[tab] | down/up[j/k] ::", Style::default().fg(Color::Blue)),
        ]),
    ];
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[1]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ].as_ref())
        .split(chunks[0]);

    let foo = chunks[0];

    // Surrounding block
    let sources_block = Block::default()
        .borders(Borders::ALL)
        .title("sources")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);

    let sources_block = match app.selected {
        SelectedPane::Sources => sources_block.border_style(Style::default().fg(Color::LightGreen)),
        _ => sources_block
    };
    f.render_widget(sources_block, chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(chunks[1]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("articles list")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let block = match app.selected {
        SelectedPane::Articles => block.border_style(Style::default().fg(Color::LightGreen)),
        _ => block
    };
    f.render_widget(block, chunks[0]);
    let bar = chunks[0];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("article")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let block = match app.selected {
        SelectedPane::ArticleText => block.border_style(Style::default().fg(Color::LightGreen)),
        _ => block
    };
    f.render_widget(block, chunks[1]);
    let baz = chunks[1];

    // Iterate through all elements in the `items` app and append some debug text to it.
    let sources: Vec<ListItem> = app
        .sources
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(i)];
            ListItem::new(lines) //.style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

    let style = match app.selected {
        SelectedPane::Sources => {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        },
        _ => {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        }
    };
    // Create a List from all list items and highlight the currently selected one
    let sources = List::new(sources)
        .block(Block::default().borders(Borders::ALL).title("Sources"))
        .highlight_style(style)
        .highlight_symbol("");

    // We can now render the item list
    f.render_stateful_widget(sources, foo, &mut app.sources.state);

    // Iterate through all elements in the `items` app and append some debug text to it.
    let articles: Vec<ListItem> = app
        .articles
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(i)];
            ListItem::new(lines)
        })
        .collect();

    let style = match app.selected {
        SelectedPane::Articles => {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        },
        _ => {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        }
    };
    // Create a List from all list items and highlight the currently selected one
    let articles = List::new(articles)
        .block(Block::default().borders(Borders::ALL).title("Articles"))
        .highlight_style(style)
        .highlight_symbol("");

    // We can now render the item list
    f.render_stateful_widget(articles, bar, &mut app.articles.state);

    let create_block = |title| {
        Block::default()
            .borders(Borders::ALL)
            // .style(Style::default().bg(Color::White).fg(Color::Black))
            .title(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))
    };

    match app.articles.state.selected() {
        Some(i) => {
            let paragraph = Paragraph::new(app.articles.items[i].content.clone())
                // .style(Style::default().bg(Color::White).fg(Color::Black))
                .block(create_block("Article"))
                // .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .scroll((app.scroll, 0));
            f.render_widget(paragraph, baz);
        }
        _ => {}
    }

    if app.show_popup {
        let block = Block::default().title("Add source").borders(Borders::ALL);
        let area = centered_rect(60, 17, size);
        let block_area = block.inner(area);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ].as_ref())
        .split(block_area);

        let inputSourceName = Paragraph::new(app.inputSourceName.as_ref())
            .style(match app.input_selected {
                InputSelected::SourceName => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .block(Block::default().borders(Borders::ALL).title("Source Name"));
        f.render_widget(inputSourceName, chunks[0]);

        let inputURL = Paragraph::new(app.inputURL.as_ref())
            .style(match app.input_selected {
                InputSelected::SourceURL => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .block(Block::default().borders(Borders::ALL).title("URL"));
        f.render_widget(inputURL, chunks[1]);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
