use std::fs;
use std::env;
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
    layout::{Alignment,Constraint, Direction, Layout, Rect, Corner},
    widgets::{Block, Borders, BorderType, ListState, ListItem, List, Paragraph, Wrap},
    Frame, Terminal,
};
use sqlx::sqlite::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    // let mut conn = pool.acquire().await?;
    //
    // // Insert the task, then obtain the ID of this row
    // let id = sqlx::query!(
    //     r#"
    //     CREATE TABLE IF NOT EXISTS sources (
    //         id INT,
    //         name VARCHAR
    //     );
    //     "#,
    // )
    // .execute(&mut conn)
    // .await?;
    //
    // let resp = reqwest::get("https://blog.cleancoder.com/atom.xml")
    //     .await?;
    // let body = resp.text().await?;
    // let articles = news::parse_atom(body.clone().as_ref())?;
    //
    // println!("------- PARSED --------");
    // println!("{:?}", articles);
    // println!("-----------------------");
    // let articles = articles.iter().map(|a| a).collect();

    // for local testing
    let contents = fs::read_to_string("example_atom.xml").expect("Should have been able to read the file");
    let articles = news::parse_atom(contents.as_ref())?;
    let articles = articles.iter().map(|a| a).collect();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let app = App::new(articles);

    // create app and run it
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
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

    // fn unselect(&mut self) {
    //     self.state.select(None);
    // }
}

struct App<'a> {
    items: StatefulList<(&'a str, usize)>,
    articles: StatefulList<&'a news::Article>,
    selected: i8,
}

impl<'a> App<'a> {
    fn new(articles: Vec<&'a news::Article>) -> App<'a> {
        App {
            selected: 0,
            items: StatefulList::with_items(vec![
                ("Item0", 1),
                ("Item1", 2),
                ("Item2", 1),
                ("Item3", 3),
                ("Item4", 1),
                ("Item5", 4),
            ]),
            articles: StatefulList::with_items(articles),
        }
    }

    fn next_block(&mut self) {
        match self.selected {
            2 => self.selected = 0,
            _ => self.selected += 1,
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                // KeyCode::Left => app.items.unselect(),
                KeyCode::Down | KeyCode::Char('j') => {
                    match app.selected {
                        0 => app.items.next(),
                        1 => app.articles.next(),
                        _ => {},
                    }
                },
                KeyCode::Up | KeyCode::Char('k') => {
                    match app.selected {
                        0 => app.items.previous(),
                        1 => app.articles.previous(),
                        _ => {},
                    }
                },
                KeyCode::Tab => app.next_block(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Wrapping block for a group
    // Just draw the block and the group on the same area and build the group
    // with at least a margin of 1
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ].as_ref())
        .split(f.size());

    let foo = chunks[0];

    // Surrounding block
    let sources_block = Block::default()
        .borders(Borders::ALL)
        .title("sources")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let sources_block = if app.selected == 0 {
        sources_block.border_style(Style::default().fg(Color::LightGreen))
    } else {
        sources_block
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
    let block = if app.selected == 1 {
        block.border_style(Style::default().fg(Color::LightGreen))
    } else {
        block
    };
    f.render_widget(block, chunks[0]);
    let bar = chunks[0];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("article")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let block = if app.selected == 2 {
        block.border_style(Style::default().fg(Color::LightGreen))
    } else {
        block
    };
    f.render_widget(block, chunks[1]);
    let baz = chunks[1];

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(i.0)];
            ListItem::new(lines) //.style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Sources"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    // We can now render the item list
    f.render_stateful_widget(items, foo, &mut app.items.state);

    // Iterate through all elements in the `items` app and append some debug text to it.
    let articles: Vec<ListItem> = app
        .articles
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(*i)];
            ListItem::new(lines)
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let articles = List::new(articles)
        .block(Block::default().borders(Borders::ALL).title("Articles"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    // We can now render the item list
    f.render_stateful_widget(articles, bar, &mut app.articles.state);

    let create_block = |title| {
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::White).fg(Color::Black))
            .title(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))
    };

    match app.articles.state.selected() {
        Some(i) => {
            let paragraph = Paragraph::new(app.articles.items[i].content.clone())
                .style(Style::default().bg(Color::White).fg(Color::Black))
                .block(create_block("Center, wrap"))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
                // .scroll((app.scroll, 0));
            f.render_widget(paragraph, baz);
        }
        _ => {}
    }
}
