use std::{
    error::Error,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::{
    log::{Level, LogPlugin},
    DefaultPlugins,
};
use chromadb::v2::ChromaClient;
use ollama_rs::Ollama;

#[derive(Debug, Default)]
struct CurrentState {
    jsons: bool,
    docs: bool,
    db: bool,
    ollama: bool,
}

impl CurrentState {
    async fn check() -> Self {
        let chroma: ChromaClient = ChromaClient::new(Default::default());
        let ollama = Ollama::default();

        Self {
            jsons: std::fs::read_dir("./jsons").is_ok(),
            docs: std::fs::read_dir("./docs").is_ok(),
            db: chroma.list_collections().await.is_ok(),
            ollama: ollama.list_local_models().await.is_ok(),
        }
    }

    async fn check_embedding_model(config: &Config) -> bool {
        let ollama = Ollama::default();
        let models = ollama.list_local_models().await.unwrap();

        for model in models {
            if model.name == config.embedding_model {
                return true;
            }
        }
        return false;
    }

    async fn check_vector_db(config: &Config) -> bool {
        let chroma: ChromaClient = ChromaClient::new(Default::default());

        let mut hash = DefaultHasher::new();
        config.hash(&mut hash);
        let hash = hash.finish().to_string();

        chroma.get_collection(&hash).await.is_ok()
    }
}

#[derive(Debug, Clone, Hash)]
enum Distance {
    SquaredL2,
    InnerProduct,
    Cosine,
}

#[derive(Resource, Debug, Clone, Hash)]
struct Config {
    target: String,
    embedding_model: String,
    distance: Distance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target: "bevy".to_string(),
            embedding_model: "nomic-embed-text:latest".to_string(),
            distance: Distance::SquaredL2,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                level: Level::ERROR,
                ..Default::default()
            }),
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            TuiPlugin,
            app::panel,
            status::panel,
            actions::panel,
            generate_jsons::panel,
            generate_docs::panel,
            download_model::panel,
            unimplemented::panel::<1>,
            unimplemented::panel::<4>,
            unimplemented::panel::<5>,
        ))
        .init_state::<CurrentAction>()
        .insert_resource(Config::default())
        .run();

    Ok(())
}
#[derive(Resource)]
struct RagState {
    services: CurrentState,
    model: bool,
    db: bool,
}

use ratatecs::prelude::*;

mod app {
    use bevy_tokio_tasks::TokioTasksRuntime;
    use ratatecs::prelude::*;
    use ratatui::widgets::Block;
    use symbols::border;

    use crate::{Config, CurrentState, RagState};

    pub fn panel(app: &mut App) {
        app.add_systems(Update, (exit_on_esc, update_state));

        app.add_systems(PostUpdate, render);
    }

    fn exit_on_esc(event: Res<BackendEvent>, mut exit: EventWriter<AppExit>) {
        if let Some(event) = &event.0 {
            if let event::Event::Key(key_event) = event {
                if key_event.code == event::KeyCode::Esc {
                    exit.send(AppExit::Success);
                }
            }
        }
    }

    fn update_state(runtime: ResMut<TokioTasksRuntime>, config: Res<Config>) {
        if !config.is_changed() {
            return;
        }
        runtime.spawn_background_task(|mut ctx| async move {
            let state = CurrentState::check().await;
            let config = Config::default();
            let model = CurrentState::check_embedding_model(&config).await;
            let db = CurrentState::check_vector_db(&config).await;
            let state = RagState {
                services: state,
                model,
                db,
            };

            ctx.run_on_main_thread(move |ctx| {
                let world: &mut World = ctx.world;
                world.insert_resource(state);
            })
            .await;
        });
    }

    fn render(mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let area = frame.area();

        let title = Line::from(" Doc Explorer ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<Esc> ".blue().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        drawer.push_widget(Box::new(block), area, 0);
    }
}

mod status {
    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, List};
    use symbols::border;

    use crate::{Config, RagState};

    pub fn panel(app: &mut App) {
        app.add_systems(PostUpdate, render);
    }

    fn render(state: Res<RagState>, config: Res<Config>, mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 1;
        area.y += 1;
        area.height = 9;
        area.width -= 1;

        let title = Line::from(" Status ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let json_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "rustdoc jsons: ").into(),
            match state.services.jsons {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let docs_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "documents: ").into(),
            match state.services.docs {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let ollama_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "ollama: ").into(),
            match state.services.ollama {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let model_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "model in ollama: ").into(),
            match state.model {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let database_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "database: ").into(),
            match state.services.db {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let embeddings_state = Text::from(vec![Line::from(vec![
            format!("{:<30}", "embeddings available: ").into(),
            match state.db {
                true => "✅".green(),
                false => "❌".red(),
            },
        ])]);
        let config = Text::from(vec![Line::from(vec![format!("{:?}", *config).yellow()])]);

        drawer.push_widget(
            Box::new(
                List::new([
                    json_state,
                    docs_state,
                    ollama_state,
                    model_state,
                    database_state,
                    embeddings_state,
                    config,
                ])
                .block(block),
            ),
            area,
            1,
        );
    }
}

#[derive(States, Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
enum CurrentAction {
    #[default]
    Menu,
    ChangeConfig,
    GenerateJsons,
    GenerateDocs,
    StartDb,
    StartOllama,
    DownloadModel,
}

impl CurrentAction {
    fn from_u8(i: u8) -> Self {
        match i {
            0 => CurrentAction::Menu,
            1 => CurrentAction::ChangeConfig,
            2 => CurrentAction::GenerateJsons,
            3 => CurrentAction::GenerateDocs,
            4 => CurrentAction::StartDb,
            5 => CurrentAction::StartOllama,
            6 => CurrentAction::DownloadModel,
            _ => CurrentAction::Menu,
        }
    }
}

mod actions {
    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, List};
    use symbols::border;

    use crate::{Config, CurrentAction, RagState};

    #[derive(Resource)]
    struct ActionsListState {
        list: Vec<(String, CurrentAction)>,
        selected: usize,
    }

    pub fn panel(app: &mut App) {
        app.add_systems(OnEnter(CurrentAction::Menu), |mut commands: Commands| {
            commands.insert_resource(ActionsListState {
                list: vec![],
                selected: 0,
            });
        });
        app.add_systems(
            Update,
            (prepare_list, input).run_if(in_state(CurrentAction::Menu)),
        );

        app.add_systems(PostUpdate, render.run_if(in_state(CurrentAction::Menu)));
    }

    fn prepare_list(
        mut list_state: ResMut<ActionsListState>,
        state: Res<RagState>,
        config: Res<Config>,
    ) {
        list_state.list = vec![
            ("Change Config".to_string(), CurrentAction::ChangeConfig),
            (
                format!("Generate JSONs for {}", config.target),
                CurrentAction::GenerateJsons,
            ),
        ];

        if state.services.jsons {
            list_state.list.push((
                "Generate Documents from JSONs".to_string(),
                CurrentAction::GenerateDocs,
            ));
        }

        if state.services.ollama && !state.model {
            list_state.list.push((
                format!("Download Model {}", config.embedding_model),
                CurrentAction::DownloadModel,
            ));
        }

        if !state.services.ollama {
            list_state
                .list
                .push(("Start Ollama".to_string(), CurrentAction::StartOllama));
        }
        if !state.services.db {
            list_state
                .list
                .push(("Start Vector Database".to_string(), CurrentAction::StartDb));
        }
    }

    fn input(
        mut list_state: ResMut<ActionsListState>,
        event: Res<BackendEvent>,
        mut next_state: ResMut<NextState<CurrentAction>>,
    ) {
        if let Some(event) = &event.0 {
            if let event::Event::Key(key_event) = event {
                match key_event.code {
                    event::KeyCode::Up => {
                        list_state.selected = list_state.selected.saturating_sub(1)
                    }
                    event::KeyCode::Down => {
                        list_state.selected =
                            (list_state.selected + 1).min(list_state.list.len() - 1)
                    }
                    event::KeyCode::Char(' ') => {
                        if let Some(action) = list_state.list.get(list_state.selected) {
                            next_state.set(action.1);
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn render(list_state: Res<ActionsListState>, mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 15;
        area.y = (area.height / 2 - 10).max(10);
        area.height = 8;
        area.width -= 30;

        let title = Line::from(" Actions ".bold());
        let instructions = Line::from(vec![
            " Choose Action ".into(),
            "<Up> / <Down> ".blue().bold(),
            " Select ".into(),
            "<Space> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        let width = area.width as usize - 10;

        let actions = list_state.list.iter().enumerate().map(|(i, action)| {
            if i == list_state.selected {
                Line::from(format!(" >> {:^width$} <<", action.0, width = width))
                    .style(Style::new().italic())
                    .green()
            } else {
                Line::from(format!("    {:^width$}", action.0, width = width))
            }
        });

        drawer.push_widget(
            Box::new(List::new(actions).block(block).style(Style::new().white())),
            area,
            1,
        );
    }
}

mod generate_jsons {
    use doc_explorer::json_generator::generate_jsons;
    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, Clear, Paragraph};
    use symbols::border;

    use crate::{Config, CurrentAction};

    pub fn panel(app: &mut App) {
        app.add_systems(Update, exit.run_if(in_state(CurrentAction::GenerateJsons)));
        app.add_systems(OnExit(CurrentAction::GenerateJsons), work);
        app.add_systems(
            PostUpdate,
            render.run_if(in_state(CurrentAction::GenerateJsons)),
        );
    }

    fn exit(mut next_state: ResMut<NextState<CurrentAction>>) {
        next_state.set(CurrentAction::Menu);
    }

    fn work(mut config: ResMut<Config>) {
        config.set_changed();
        generate_jsons(config.target.clone());
    }

    fn render(mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 15;
        area.y = (area.height / 2 - 10).max(10);
        area.height = 8;
        area.width -= 30;

        let instructions = Line::from(vec![" Back to Menu ".into(), "<Space> ".blue().bold()]);

        let block = Block::bordered()
            .title(Line::from("Generate JSONs").bold().centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        drawer.push_widget(Box::new(Clear), area, 1);
        drawer.push_widget(
            Box::new(
                Paragraph::new(Line::from("Working... (can take a few minutes)").italic())
                    .centered()
                    .block(block),
            ),
            area,
            2,
        );
    }
}

mod generate_docs {
    use doc_explorer::document::generate_docs;
    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, Clear, Paragraph};
    use symbols::border;

    use crate::{Config, CurrentAction};

    pub fn panel(app: &mut App) {
        app.add_systems(Update, exit.run_if(in_state(CurrentAction::GenerateDocs)));
        app.add_systems(OnExit(CurrentAction::GenerateDocs), work);
        app.add_systems(
            PostUpdate,
            render.run_if(in_state(CurrentAction::GenerateDocs)),
        );
    }

    fn exit(mut next_state: ResMut<NextState<CurrentAction>>) {
        next_state.set(CurrentAction::Menu);
    }

    fn work(mut config: ResMut<Config>) {
        config.set_changed();
        generate_docs(config.target.clone());
    }

    fn render(mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 15;
        area.y = (area.height / 2 - 10).max(10);
        area.height = 8;
        area.width -= 30;

        let instructions = Line::from(vec![" Back to Menu ".into(), "<Space> ".blue().bold()]);

        let block = Block::bordered()
            .title(Line::from("Generate Documents").bold().centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        drawer.push_widget(Box::new(Clear), area, 1);
        drawer.push_widget(
            Box::new(
                Paragraph::new(Line::from("Working... (can take a few minutes)").italic())
                    .centered()
                    .block(block),
            ),
            area,
            2,
        );
    }
}

mod download_model {
    use bevy_tokio_tasks::TokioTasksRuntime;
    use doc_explorer::ollama::SimpleOllama;
    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, Clear, Paragraph};
    use symbols::border;

    use crate::{Config, CurrentAction};

    pub fn panel(app: &mut App) {
        app.add_systems(Update, exit.run_if(in_state(CurrentAction::DownloadModel)));
        app.add_systems(OnEnter(CurrentAction::DownloadModel), work);
        app.add_systems(
            PostUpdate,
            render.run_if(in_state(CurrentAction::DownloadModel)),
        );
    }

    #[derive(Resource)]
    struct Done;

    fn exit(
        _done: Res<Done>,
        mut commands: Commands,
        mut next_state: ResMut<NextState<CurrentAction>>,
    ) {
        commands.remove_resource::<Done>();
        next_state.set(CurrentAction::Menu);
    }

    fn work(runtime: ResMut<TokioTasksRuntime>, config: Res<Config>) {
        let ollama = SimpleOllama::new(config.embedding_model.clone());
        runtime.spawn_background_task(|mut ctx| async move {
            ollama.download_model().await.unwrap();

            ctx.run_on_main_thread(move |ctx| {
                let world: &mut World = ctx.world;
                world.insert_resource(Done);
                world.resource_mut::<Config>().set_changed();
            })
            .await;
        });
    }
    fn render(mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 15;
        area.y = (area.height / 2 - 10).max(10);
        area.height = 8;
        area.width -= 30;

        let instructions = Line::from(vec![" Back to Menu ".into(), "<Space> ".blue().bold()]);

        let block = Block::bordered()
            .title(Line::from("Generate JSONs").bold().centered())
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        drawer.push_widget(Box::new(Clear), area, 1);
        drawer.push_widget(
            Box::new(
                Paragraph::new(Line::from("Working... (can take a few minutes)").italic())
                    .centered()
                    .block(block),
            ),
            area,
            2,
        );
    }
}

mod unimplemented {

    use ratatecs::prelude::*;
    use ratatui::widgets::{Block, Paragraph};
    use symbols::border;

    use crate::CurrentAction;

    pub fn panel<const A: u8>(app: &mut App) {
        app.add_systems(Update, exit.run_if(in_state(CurrentAction::from_u8(A))));
        app.add_systems(
            PostUpdate,
            render::<A>.run_if(in_state(CurrentAction::from_u8(A))),
        );
    }

    fn exit(event: Res<BackendEvent>, mut next_state: ResMut<NextState<CurrentAction>>) {
        if let Some(event) = &event.0 {
            if let event::Event::Key(key_event) = event {
                if key_event.code == event::KeyCode::Char(' ') {
                    next_state.set(CurrentAction::Menu);
                }
            }
        }
    }

    fn render<const A: u8>(mut drawer: WidgetDrawer) {
        let frame = drawer.get_frame();
        let mut area = frame.area();
        area.x += 15;
        area.y = (area.height / 2 - 10).max(10);
        area.height = 8;
        area.width -= 30;

        let instructions = Line::from(vec![" Back to Menu ".into(), "<Space> ".blue().bold()]);

        let block = Block::bordered()
            .title(
                Line::from(format!("{:?}", CurrentAction::from_u8(A)))
                    .bold()
                    .centered(),
            )
            .title_bottom(instructions.right_aligned())
            .border_set(border::THICK);

        drawer.push_widget(
            Box::new(
                Paragraph::new(Line::from("unimplemented").italic())
                    .centered()
                    .block(block),
            ),
            area,
            1,
        );
    }
}
